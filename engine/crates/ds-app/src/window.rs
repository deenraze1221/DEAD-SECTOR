//! Windowed mode: winit event loop, surface presentation and a free-fly camera.

use crate::options::Options;
use crate::{LoadedCity, environment, initial_camera};
use anyhow::{Context, Result};
use ds_render::{Camera, Environment, FrameInputs, Quality, Renderer, connect};
use glam::Vec3;
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
const WALK_SPEED: f32 = 8.0;
const FAST_MULTIPLIER: f32 = 4.0;
const MOUSE_RADIANS_PER_PIXEL: f32 = 0.0022;
const TITLE_INTERVAL: Duration = Duration::from_millis(500);

pub fn run(options: Options, loaded: LoadedCity) -> Result<()> {
    let event_loop = EventLoop::new().context("creating event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let camera = initial_camera(&options, &loaded.city);
    let mut app = App { options, loaded, camera, gpu: None, error: None };
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
    fixed_time: Option<f32>,
    started: Instant,
    held: HashSet<KeyCode>,
    captured: bool,
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
        let gpu = connect(Some(window.clone()))?;
        let (adapter, device, queue) = (gpu.adapter, gpu.device, gpu.queue);
        let surface = gpu.surface.context("window surface missing")?;
        let info = adapter.get_info();
        log::info!("adapter: {} ({:?}, {:?})", info.name, info.backend, info.device_type);

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by the adapter")?;
        // Shaders output linear colour, so present through an sRGB format.
        let capabilities = surface.get_capabilities(&adapter);
        config.format = capabilities.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(config.format);
        config.present_mode = if options.vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        config.desired_maximum_frame_latency = 2;
        surface.configure(&device, &config);
        log::info!("surface: {:?}, {:?}", config.format, config.present_mode);

        let started = Instant::now();
        let renderer = Renderer::new(&device, &queue, config.format, &loaded.city, &loaded.scene, &loaded.assets)?;
        log::info!("uploaded scene in {:.0} ms", started.elapsed().as_secs_f64() * 1e3);
        Ok(Gpu {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            adapter_name: info.name,
            environment: environment(options, &loaded.city),
            quality: options.quality,
            fixed_time: options.time,
            started: Instant::now(),
            held: HashSet::new(),
            captured: false,
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
            camera.position += direction * WALK_SPEED * if fast { FAST_MULTIPLIER } else { 1.0 } * dt;
        }
    }

    fn redraw(&mut self, camera: &mut Camera) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.fly(camera, dt);

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
        let inputs = FrameInputs {
            camera,
            environment: &self.environment,
            quality: self.quality,
            time: self.fixed_time.unwrap_or_else(|| self.started.elapsed().as_secs_f32()),
            hurt: 0.0,
            low_health: false,
        };
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
                "{TITLE} | {fps:.0} fps ({:.2} ms) | {} draws, {:.0}k tris | {} | pos {:.1} {:.1} {:.1}",
                1e3 / fps,
                stats.draws,
                stats.triangles as f64 / 1e3,
                self.adapter_name,
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
            WindowEvent::RedrawRequested => gpu.redraw(&mut self.camera),
            WindowEvent::Focused(false) => {
                gpu.held.clear();
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
            self.camera.yaw -= delta.0 as f32 * MOUSE_RADIANS_PER_PIXEL;
            let limit = 89f32.to_radians();
            self.camera.pitch = (self.camera.pitch - delta.1 as f32 * MOUSE_RADIANS_PER_PIXEL).clamp(-limit, limit);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }
}
