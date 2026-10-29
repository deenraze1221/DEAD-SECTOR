//! Sector Engine renderer (wgpu).
//!
//! Frame structure: scene geometry and atmosphere layers into an HDR target
//! (optionally capped to a retro line count), glow and filmic tonemap, then
//! the retro pass up to the output size. Games supply the scene, lighting,
//! point lights, settings and an optional WGSL surface hook; nothing here is
//! specific to one game.

mod gpu;
pub mod lights;
pub mod noise;
mod post;
mod scene_pass;
pub mod settings;
pub mod view;

pub use gpu::{Connection, backend_order, connect, request_device};
pub use lights::PointLight;
pub use post::grid_size;
pub use settings::{Atmosphere, EdgeTint, Lighting, PostSettings, RenderSettings, RetroSettings};
pub use view::Camera;

use anyhow::{Context, Result};
use post::{PostPipelines, RetroUniform, Targets};
use scene_pass::{AtmosphereLayers, FrameUniform, ScenePass};
use sector_assets::{Image, SceneBuffers};

/// Format used for offscreen captures.
pub const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The scene shader source before the surface hook is spliced in at `// SURFACE_HOOK`.
pub const SCENE_SHADER: &str = include_str!("scene.wgsl");

/// The surface hook used when a game supplies none.
pub const DEFAULT_SURFACE_HOOK: &str = include_str!("default_surface.wgsl");

/// Per-frame counters shown in the window title and logs.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws: usize,
    pub instances: usize,
    pub triangles: u64,
}

/// Fixed renderer configuration chosen by the game.
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// WGSL defining `fn surface_hook(shading: u32, input: SurfaceInput) -> SurfaceOutput`.
    pub surface_hook: String,
    pub post: PostSettings,
}

impl Default for RendererConfig {
    fn default() -> Self {
        RendererConfig { surface_hook: DEFAULT_SURFACE_HOOK.to_string(), post: PostSettings::default() }
    }
}

/// Everything that varies from frame to frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameInputs<'a> {
    pub camera: &'a Camera,
    pub lighting: &'a Lighting,
    /// Point lights for this frame (at most [`lights::MAX_LIGHTS`] are used).
    pub lights: &'a [PointLight],
    pub settings: &'a RenderSettings,
    pub atmosphere: &'a Atmosphere,
    pub retro: &'a RetroSettings,
    /// Game-defined values readable by the surface hook as `frame.game`.
    pub game: [f32; 4],
    /// Seconds since start; drives fog-sheet scroll, lens rain and grain.
    pub time: f32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    color_format: wgpu::TextureFormat,
    scene: ScenePass,
    post: PostPipelines,
    post_settings: PostSettings,
    targets: Option<Targets>,
    stats: SceneStats,
}

impl Renderer {
    /// Uploads the scene and builds pipelines. `textures` are indexed by
    /// [`sector_assets::MaterialParams::texture`]; `color_format` is the format
    /// of the final output (window surface or [`CAPTURE_FORMAT`]).
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        scene: &SceneBuffers,
        textures: &[Image],
        config: &RendererConfig,
    ) -> Result<Self> {
        Ok(Renderer {
            device: device.clone(),
            queue: queue.clone(),
            color_format,
            scene: ScenePass::new(device, queue, scene, textures, &config.surface_hook)?,
            post: PostPipelines::new(device, color_format),
            post_settings: config.post,
            targets: None,
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

    /// Replaces the additive ground decals: xyz = centre, w = radius (0 hides one).
    pub fn set_decals(&mut self, decals: &[[f32; 4]]) {
        self.scene.set_decals(decals);
    }

    /// Records a full frame into `output` (which must use the renderer's colour format).
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        output_size: (u32, u32),
        inputs: &FrameInputs,
    ) {
        let max_lines = inputs.settings.max_render_lines;
        if self.targets.as_ref().is_none_or(|t| t.output != output_size || t.max_lines != max_lines) {
            self.targets = Some(Targets::new(&self.device, &self.post, output_size, max_lines, &self.post_settings));
        }
        let targets = self.targets.as_ref().expect("targets just created");
        let (camera, lighting) = (inputs.camera, inputs.lighting);
        let aspect = targets.grid.0 as f32 / targets.grid.1 as f32;
        let frame = FrameUniform::new(
            camera.view_proj(aspect),
            camera.position,
            inputs.time,
            lighting,
            inputs.lights,
            inputs.game,
        );
        self.scene.prepare(&frame, inputs.atmosphere, inputs.time);

        let layers = AtmosphereLayers {
            decals: inputs.settings.atmosphere && inputs.atmosphere.decal_opacity > 0.0,
            fog_sheets: inputs.settings.atmosphere && inputs.atmosphere.fog_sheet_density > 0.0,
        };
        {
            let fog = lighting.fog_color;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.hdr,
                    depth_slice: None,
                    resolve_target: None,
                    // The background is the fog colour (Godot's fog_sky_affect = 1).
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
            self.scene.record(&mut pass, &layers);
        }

        let retro = RetroUniform::new(
            inputs.retro,
            output_size,
            inputs.time,
            inputs.settings.post_detail,
            self.color_format.is_srgb(),
        );
        targets.record(&self.queue, &self.post, encoder, output, &retro);
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
