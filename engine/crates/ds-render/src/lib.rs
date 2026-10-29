//! wgpu renderer for the Dead Sector city.
//!
//! Frame structure: city + atmosphere into an HDR target at the PSX pixel-grid
//! resolution (≤ 540 lines), Godot-style glow and filmic tonemap, then the
//! pixels pass up to the output size.

pub mod environment;
pub mod lights;
pub mod noise;
mod post;
mod scene_pass;
pub mod view;

pub use environment::{Environment, FogLevel, Quality, Weather, WeatherKind};
pub use post::grid_size;
pub use view::Camera;

use anyhow::{Context, Result};
use ds_assets::{City, SceneBuffers};
use glam::Vec3;
use lights::PointLight;
use post::{PixelsUniform, PostPipelines, Targets};
use scene_pass::{Atmosphere, FrameUniform, ScenePass};
use std::path::Path;

/// Format used for offscreen captures.
pub const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Per-frame counters shown in the window title and logs.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws: usize,
    pub instances: usize,
    pub triangles: u64,
}

/// Everything that varies from frame to frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameInputs<'a> {
    pub camera: &'a Camera,
    pub environment: &'a Environment,
    pub quality: Quality,
    /// Seconds since start; drives mist scroll, lens rain and grain.
    pub time: f32,
    /// 0..1 red damage flash.
    pub hurt: f32,
    pub low_health: bool,
}

/// Backends tried in order when `WGPU_BACKEND` is not set: Vulkan first, then DX12
/// on Windows for GPUs without a working Vulkan driver (Metal on macOS).
pub fn backend_order() -> Vec<wgpu::Backends> {
    let mut order = vec![wgpu::Backends::VULKAN];
    if cfg!(windows) {
        order.push(wgpu::Backends::DX12);
    }
    if cfg!(target_os = "macos") {
        order.push(wgpu::Backends::METAL);
    }
    order
}

/// A working GPU device, plus the window surface when one was requested.
pub struct Connection {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface<'static>>,
}

/// Connects to the first backend in [`backend_order`] that yields a working device
/// (presenting to `window`, if given). `WGPU_BACKEND` replaces the order with a
/// single explicit choice.
pub fn connect(window: Option<std::sync::Arc<dyn wgpu::DisplayAndWindowHandle>>) -> Result<Connection> {
    let candidates = if std::env::var_os("WGPU_BACKEND").is_some() {
        vec![wgpu::Backends::all().with_env()]
    } else {
        backend_order()
    };
    let mut failures = Vec::new();
    for backends in candidates {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle().with_env();
        descriptor.backends = backends;
        let instance = wgpu::Instance::new(descriptor);
        let surface = match window.clone().map(|w| instance.create_surface(w)).transpose() {
            Ok(surface) => surface,
            Err(error) => {
                failures.push(format!("{backends:?}: {error}"));
                continue;
            }
        };
        match pollster::block_on(request_device(&instance, surface.as_ref())) {
            Ok((adapter, device, queue)) => return Ok(Connection { instance, adapter, device, queue, surface }),
            Err(error) => {
                log::warn!("{backends:?} unavailable: {error:#}");
                failures.push(format!("{backends:?}: {error:#}"));
            }
        }
    }
    anyhow::bail!("no usable GPU backend:\n  {}", failures.join("\n  "))
}

/// Requests an adapter and device. `surface` makes the adapter compatible with a window.
pub async fn request_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        })
        .await
        .context("no compatible GPU adapter found")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("dead-sector"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .context("creating GPU device")?;
    Ok((adapter, device, queue))
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    color_format: wgpu::TextureFormat,
    scene: ScenePass,
    post: PostPipelines,
    targets: Option<Targets>,
    lamps: Vec<Vec3>,
    store_light: PointLight,
    buildings: Vec<ds_assets::city::Building>,
    stats: SceneStats,
}

impl Renderer {
    /// Uploads the scene and builds pipelines. `color_format` is the format of
    /// the final output (window surface or [`CAPTURE_FORMAT`]).
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        city: &City,
        scene: &SceneBuffers,
        assets: &Path,
    ) -> Result<Self> {
        let lamps: Vec<Vec3> = city.lamps().map(Vec3::from).collect();
        Ok(Renderer {
            device: device.clone(),
            queue: queue.clone(),
            color_format,
            scene: ScenePass::new(device, queue, scene, lamps.clone(), assets)?,
            post: PostPipelines::new(device, color_format),
            targets: None,
            lamps,
            store_light: lights::store_light(city.terminal.x, city.terminal.z),
            buildings: city.buildings.clone(),
            stats: SceneStats {
                draws: scene.draws.len(),
                instances: scene.instances.len(),
                triangles: scene.triangles_per_frame(),
            },
        })
    }

    pub fn stats(&self) -> SceneStats {
        SceneStats { draws: self.scene.draw_count(), ..self.stats }
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    /// Point lights for this frame: the store light plus the nearest street lamps.
    fn frame_lights(&self, eye: Vec3, env: &Environment, quality: Quality) -> Vec<PointLight> {
        let mut lights = vec![self.store_light];
        lights.extend(lights::street_lights(&self.lamps, eye, quality.lamp_lights(), env.power_cut, env.lamp_energy));
        lights
    }

    /// Records a full frame into `output` (which must use the renderer's colour format).
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        output_size: (u32, u32),
        inputs: &FrameInputs,
    ) {
        if self.targets.as_ref().is_none_or(|t| t.output != output_size) {
            self.targets = Some(Targets::new(&self.device, &self.post, output_size));
        }
        let targets = self.targets.as_ref().expect("targets just created");
        let (camera, env) = (inputs.camera, inputs.environment);
        let aspect = targets.grid.0 as f32 / targets.grid.1 as f32;
        let lights = self.frame_lights(camera.position, env, inputs.quality);
        let frame = FrameUniform::new(camera.view_proj(aspect), camera.position, inputs.time, env, &lights);
        self.scene.prepare(&frame, env, inputs.time);

        let atmosphere = Atmosphere {
            pools: inputs.quality.atmosphere() && env.pool_alpha > 0.0,
            mist: inputs.quality.atmosphere() && env.mist_density > 0.0,
        };
        {
            let fog = env.fog_color;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.hdr,
                    depth_slice: None,
                    resolve_target: None,
                    // Godot fogs the sky completely (fog_sky_affect = 1): the background is the fog colour.
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: fog.x as f64,
                            g: fog.y as f64,
                            b: fog.z as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.scene.record(&mut pass, &atmosphere);
        }

        let outdoors = !lights::sheltered(&self.buildings, camera.position);
        let lens_rain = if inputs.quality.atmosphere() && outdoors { env.lens_rain } else { 0.0 };
        let pixels = PixelsUniform {
            output: [output_size.0 as f32, output_size.1 as f32, inputs.time, inputs.quality.post_detail()],
            effects: [
                lens_rain,
                inputs.hurt,
                if inputs.low_health { 1.0 } else { 0.0 },
                if self.color_format.is_srgb() { 1.0 } else { 0.0 },
            ],
        };
        targets.record(&self.queue, &self.post, encoder, output, &pixels);
    }

    fn output_texture(&self, size: (u32, u32), usage: wgpu::TextureUsages) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.color_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | usage,
            view_formats: &[],
        })
    }

    /// Renders `frames` offscreen frames, waiting for the GPU after each one, and
    /// returns the total time. No readback, so it measures render cost only.
    pub fn bench(&mut self, size: (u32, u32), inputs: &FrameInputs, frames: u32) -> Result<std::time::Duration> {
        let view = self.output_texture(size, wgpu::TextureUsages::empty()).create_view(&Default::default());
        let started = std::time::Instant::now();
        for _ in 0..frames {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            self.render(&mut encoder, &view, size, inputs);
            self.queue.submit([encoder.finish()]);
            self.device.poll(wgpu::PollType::wait_indefinitely())?;
        }
        Ok(started.elapsed())
    }

    /// Renders one frame offscreen and returns tightly packed sRGB RGBA8 pixels.
    /// The renderer must have been created with [`CAPTURE_FORMAT`].
    pub fn capture(&mut self, size: (u32, u32), inputs: &FrameInputs) -> Result<Vec<u8>> {
        anyhow::ensure!(self.color_format == CAPTURE_FORMAT, "capture needs a renderer created with CAPTURE_FORMAT");
        let (width, height) = size;
        let texture = self.output_texture(size, wgpu::TextureUsages::COPY_SRC);
        let row = width * 4;
        let padded_row = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture-readback"),
            size: padded_row as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.render(&mut encoder, &texture.create_view(&Default::default()), size, inputs);
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()?.context("mapping capture buffer")?;
        let mapped = slice.get_mapped_range()?;
        let mut pixels = Vec::with_capacity((row * height) as usize);
        for line in mapped.chunks(padded_row as usize) {
            pixels.extend_from_slice(&line[..row as usize]);
        }
        drop(mapped);
        readback.unmap();
        Ok(pixels)
    }
}
