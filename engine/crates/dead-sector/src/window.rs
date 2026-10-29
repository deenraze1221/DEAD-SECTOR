//! Windowed mode: winit event loop, surface presentation, the first-person
//! player (fixed 60 Hz steps, interpolated) and a free-fly debug camera.

use crate::options::Options;
use crate::{LoadedCity, environment, initial_camera};
use anyhow::{Context, Result};
use dead_sector::climate::{Environment, Quality};
use dead_sector::frame::{CityLights, FrameData};
use dead_sector::player::{self, Player, TICK};
use glam::Vec3;
use sector_physics::World;
use sector_render::{Camera, RenderSettings, Renderer, connect};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

const TITLE: &str = "DEAD SECTOR";
const FLY_SPEED: f32 = 8.0;
const FAST_MULTIPLIER: f32 = 4.0;
const MOUSE_SENSITIVITY: f32 = 1.0;
/// Simulation steps per frame at most (avoids a spiral after a long stall).
const MAX_STEPS_PER_FRAME: u32 = 8;
const TITLE_INTERVAL: Duration = Duration::from_millis(500);

pub fn run(options: Options, loaded: LoadedCity) -> Result<()> {
    let event_loop = EventLoop::new().context("creating event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let camera = initial_camera(&options, &loaded.city);
    let started = Instant::now();
    let world = player::collision_world(&loaded.city);
    log::info!("collision: {} boxes in {:.1} ms", world.len(), started.elapsed().as_secs_f64() * 1e3);
    let spawn = Vec3::new(loaded.city.spawn.x, loaded.city.spawn.y, loaded.city.spawn.z);
    let mut walker = Player::new(spawn);
    walker.yaw = camera.yaw;
    walker.pitch = camera.pitch;
    // An explicit --camera pose is a debug view: start in fly mode there.
    let fly = options.camera.is_some();
    let mut app = App { options, loaded, camera, player: walker, world, fly, gpu: None, error: None };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct App {
    options: Options,
    loaded: LoadedCity,
    camera: Camera,
    player: Player,
    world: World,
    /// Free-fly debug camera instead of the player (toggle with V).
    fly: bool,
    gpu: Option<Gpu>,
    error: Option<anyhow::Error>,
}

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    adapter_name: String,
    environment: Environment,
    quality: Quality,
    settings: RenderSettings,
    fixed_time: Option<f32>,
    started: Instant,
    held: HashSet<KeyCode>,
    captured: bool,
    /// Jump pressed since the last simulation step.
    jump_queued: bool,
    /// Unsimulated time, in seconds.
    accumulator: f32,
    last_frame: Instant,
    title_frames: u32,
    title_since: Instant,
}

impl Gpu {
    fn new(event_loop: &ActiveEventLoop, options: &Options, loaded: &LoadedCity) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(PhysicalSize::new(options.size.0, options.size.1));
        let window = Arc::new(event_loop.create_window(attributes).context("creating window")?);
        let mut gpu = connect(Some(window.clone()))?;
        let surface = gpu.surface.take().context("window surface missing")?;
        let (adapter, device, queue) = (&gpu.adapter, gpu.device.clone(), gpu.queue.clone());
        let info = adapter.get_info();
        log::info!("adapter: {} ({:?}, {:?})", info.name, info.backend, info.device_type);

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by the adapter")?;
        // Shaders output linear colour, so present through an sRGB format.
        let capabilities = surface.get_capabilities(adapter);
        config.format = capabilities.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(config.format);
        config.present_mode = if options.vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        config.desired_maximum_frame_latency = 2;
        surface.configure(&device, &config);
        log::info!("surface: {:?}, {:?}", config.format, config.present_mode);

        let started = Instant::now();
        let mut renderer = loaded.renderer(&gpu, config.format)?;
        let environment = environment(options, &loaded.city);
        renderer.set_decals(&loaded.lights.pool_decals(&environment));
        log::info!("uploaded scene in {:.0} ms", started.elapsed().as_secs_f64() * 1e3);
        Ok(Gpu {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            adapter_name: info.name,
            environment,
            quality: options.quality,
            settings: options.render_settings(),
            fixed_time: options.time,
            started: Instant::now(),
            held: HashSet::new(),
            captured: false,
            jump_queued: false,
            accumulator: 0.0,
            last_frame: Instant::now(),
            title_frames: 0,
            title_since: Instant::now(),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn set_captured(&mut self, captured: bool) {
        if captured {
            let grabbed = self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            if let Err(error) = grabbed {
                log::warn!("cannot capture mouse: {error}");
                return;
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        }
        self.window.set_cursor_visible(!captured);
        self.captured = captured;
    }

    fn fly(&mut self, camera: &mut Camera, dt: f32) {
        let key = |code| self.held.contains(&code);
        let axis = |positive, negative| (key(positive) as i32 - key(negative) as i32) as f32;
        let forward = camera.forward();
        let mut direction = forward * axis(KeyCode::KeyW, KeyCode::KeyS)
            + camera.right() * axis(KeyCode::KeyD, KeyCode::KeyA)
            + Vec3::Y * (key(KeyCode::Space) as i32 - (key(KeyCode::ControlLeft) || key(KeyCode::KeyC)) as i32) as f32;
        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
            let fast = key(KeyCode::ShiftLeft) || key(KeyCode::ShiftRight);
            camera.position += direction * FLY_SPEED * if fast { FAST_MULTIPLIER } else { 1.0 } * dt;
        }
    }

    /// Runs whole simulation steps for the elapsed time; returns how far the
    /// next step has progressed (0..1), for interpolation.
    fn walk(&mut self, walker: &mut Player, world: &World, dt: f32) -> f32 {
        let key = |code| self.held.contains(&code);
        let axis = |positive, negative| (key(positive) as i32 - key(negative) as i32) as f32;
        let mut input = player::Input {
            forward: axis(KeyCode::KeyW, KeyCode::KeyS),
            right: axis(KeyCode::KeyD, KeyCode::KeyA),
            sprint: key(KeyCode::ShiftLeft) || key(KeyCode::ShiftRight),
            jump: false,
        };
        self.accumulator += dt;
        let mut steps = 0;
        while self.accumulator >= TICK && steps < MAX_STEPS_PER_FRAME {
            input.jump = std::mem::take(&mut self.jump_queued);
            walker.step(world, input);
            self.accumulator -= TICK;
            steps += 1;
        }
        self.accumulator = self.accumulator.min(TICK);
        self.accumulator / TICK
    }

    fn redraw(&mut self, camera: &mut Camera, walker: &mut Player, world: &World, fly: bool, lights: &CityLights) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if fly {
            self.fly(camera, dt);
        } else {
            let alpha = self.walk(walker, world, dt);
            *camera = walker.camera(alpha, *camera);
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("surface validation error while acquiring a frame");
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let frame_data =
            FrameData::new(lights, camera.position, &self.environment, self.quality, self.settings, 0.0, false);
        let time = self.fixed_time.unwrap_or_else(|| self.started.elapsed().as_secs_f32());
        let inputs = frame_data.inputs(camera, time);
        self.renderer.render(&mut encoder, &view, (self.config.width, self.config.height), &inputs);
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);

        self.title_frames += 1;
        let elapsed = self.title_since.elapsed();
        if elapsed >= TITLE_INTERVAL {
            let fps = self.title_frames as f64 / elapsed.as_secs_f64();
            let stats = self.renderer.stats();
            let p = camera.position;
            self.window.set_title(&format!(
                "{TITLE} | {fps:.0} fps ({:.2} ms) | {} draws, {:.0}k tris | {} | {} {:.1} {:.1} {:.1}",
                1e3 / fps,
                stats.draws,
                stats.triangles as f64 / 1e3,
                self.adapter_name,
                if fly { "fly" } else { "walk" },
                p.x,
                p.y,
                p.z
            ));
            self.title_frames = 0;
            self.title_since = Instant::now();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        match Gpu::new(event_loop, &self.options, &self.loaded) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gpu) = self.gpu.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => gpu.resize(size),
            WindowEvent::RedrawRequested => {
                gpu.redraw(&mut self.camera, &mut self.player, &self.world, self.fly, &self.loaded.lights)
            }
            WindowEvent::Focused(false) => {
                gpu.held.clear();
                gpu.jump_queued = false;
                gpu.set_captured(false);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. }
                if !gpu.captured =>
            {
                gpu.set_captured(true)
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key: PhysicalKey::Code(code), state, repeat, .. },
                ..
            } => match state {
                ElementState::Pressed => {
                    gpu.held.insert(code);
                    match code {
                        KeyCode::Escape if !repeat => gpu.set_captured(false),
                        KeyCode::Space if !repeat => gpu.jump_queued = true,
                        KeyCode::KeyV if !repeat => {
                            self.fly = !self.fly;
                            if self.fly {
                                self.camera = self.player.camera(1.0, self.camera);
                            } else {
                                // Drop the player in at the fly camera's spot.
                                let feet = self.camera.position - Vec3::Y * sector_render::view::EYE_HEIGHT;
                                self.player = Player::new(feet);
                                self.player.yaw = self.camera.yaw;
                                self.player.pitch = self.camera.pitch;
                                gpu.accumulator = 0.0;
                                gpu.jump_queued = false;
                            }
                        }
                        KeyCode::F11 if !repeat => {
                            let full = gpu.window.fullscreen().is_none().then_some(Fullscreen::Borderless(None));
                            gpu.window.set_fullscreen(full);
                        }
                        _ => {}
                    }
                }
                ElementState::Released => {
                    gpu.held.remove(&code);
                }
            },
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let Some(gpu) = self.gpu.as_ref() else { return };
        if let DeviceEvent::MouseMotion { delta } = event
            && gpu.captured
        {
            let (dx, dy) = (delta.0 as f32, delta.1 as f32);
            if self.fly {
                self.camera.yaw -= dx * player::LOOK_RADIANS_PER_PIXEL * MOUSE_SENSITIVITY;
                let limit = 89f32.to_radians();
                self.camera.pitch =
                    (self.camera.pitch - dy * player::LOOK_RADIANS_PER_PIXEL * MOUSE_SENSITIVITY).clamp(-limit, limit);
            } else {
                self.player.look(dx, dy, MOUSE_SENSITIVITY);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }
}
