//! Sector Engine renderer (wgpu).
//!
//! Frame structure: scene geometry and atmosphere layers into an HDR target
//! (optionally capped to a retro line count), glow and filmic tonemap, then
//! the retro pass up to the output size. Games supply the scene, lighting,
//! point lights, settings and an optional WGSL surface hook; nothing here is
//! specific to one game.

mod gpu;
mod gpu_util;
pub mod lights;
pub mod noise;
mod post;
mod profiler;
mod scene_pass;
mod screen_space;
pub mod settings;
mod shadow_maps;
mod shadows;
mod surface_maps;
pub mod view;

pub use gpu::{Connection, backend_order, connect, request_device};
pub use lights::PointLight;
pub use post::grid_size;
pub use settings::{
    Atmosphere, ContactShadows, EdgeTint, Lighting, LocalShadows, PostSettings, RenderSettings, RetroSettings,
    ShadowFilter, Ssao, SunShadows,
};
pub use view::Camera;

use anyhow::{Context, Result};
use post::{PostPipelines, RetroUniform, Targets};
use profiler::GpuProfiler;
use scene_pass::{AtmosphereLayers, FrameUniform, ScenePass};
use screen_space::{ScreenCamera, ScreenSpace, ScreenTargets};
use sector_assets::{Image, SceneBuffers};
use shadow_maps::{MapConfig, ShadowMaps, ShadowUniform};

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
    /// Tiling material maps; `MaterialParams::surface_map` indexes this list.
    pub surface_maps: std::sync::Arc<Vec<sector_assets::materials::MaterialPack>>,
    /// Largest colour/normal map side to upload (smaller mips are used above it).
    pub max_map_size: u32,
}

impl Default for RendererConfig {
    fn default() -> Self {
        RendererConfig {
            surface_hook: DEFAULT_SURFACE_HOOK.to_string(),
            post: PostSettings::default(),
            surface_maps: Default::default(),
            max_map_size: 2048,
        }
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
    screen: ScreenSpace,
    screen_targets: Option<(u64, ScreenTargets)>,
    shadow_maps: ShadowMaps,
    /// Stand-in occlusion texture (no occlusion) when the prepass is off.
    no_occlusion: wgpu::TextureView,
    profiler: Option<GpuProfiler>,
    profiling: bool,
    /// Instances drawn into each sun cascade last frame.
    cascade_casters: Vec<u32>,
    stats: SceneStats,
}

/// A depth-only pass into one shadow-map layer, cleared to the far plane.
fn depth_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    profiler: &mut Option<GpuProfiler>,
    label: &'static str,
    view: &wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
            stencil_ops: None,
        }),
        timestamp_writes: profiler.as_mut().and_then(|p| p.pass(label)),
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// Result of [`Renderer::bench`].
#[derive(Debug, Clone)]
pub struct BenchResult {
    /// Wall time per frame, including waiting for the GPU.
    pub frame_ms: f64,
    /// Average GPU time per pass group, when the adapter supports timestamps.
    pub gpu_passes: Vec<(&'static str, f64)>,
    /// Instances drawn into each sun cascade (of the scene's total).
    pub cascade_casters: Vec<u32>,
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
        let maps = surface_maps::SurfaceMaps::new(device, queue, &config.surface_maps, config.max_map_size)?;
        if !config.surface_maps.is_empty() {
            log::info!(
                "surface maps: {} materials at {}²{}",
                config.surface_maps.len(),
                maps.size,
                if maps.compressed { " (BC)" } else { " (decoded, no BC support)" }
            );
        }
        let used = scene.materials.iter().filter_map(|m| m.surface_map).max();
        anyhow::ensure!(
            used.is_none_or(|l| (l as usize) < config.surface_maps.len()),
            "a material uses surface map {used:?}, only {} given",
            config.surface_maps.len()
        );
        let scene_pass = ScenePass::new(device, queue, scene, textures, &config.surface_hook, &maps)?;
        let shadow_maps =
            ShadowMaps::new(device, &scene_pass.depth_view_layout, MapConfig::new(&RenderSettings::default()));
        let no_occlusion =
            gpu_util::upload_texture(device, queue, "no-occlusion", (1, 1), screen_space::OCCLUSION_FORMAT, &[255; 4]);
        Ok(Renderer {
            device: device.clone(),
            queue: queue.clone(),
            color_format,
            scene: scene_pass,
            post: PostPipelines::new(device, color_format),
            post_settings: config.post,
            targets: None,
            screen: ScreenSpace::new(device),
            screen_targets: None,
            shadow_maps,
            no_occlusion,
            profiler: GpuProfiler::new(device, queue),
            profiling: false,
            cascade_casters: Vec::new(),
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
        let settings = inputs.settings;
        let max_lines = settings.max_render_lines;
        if self.targets.as_ref().is_none_or(|t| t.output != output_size || t.max_lines != max_lines) {
            self.targets = Some(Targets::new(&self.device, &self.post, output_size, max_lines, &self.post_settings));
        }
        let map_config = MapConfig::new(settings);
        if self.shadow_maps.config != map_config {
            self.shadow_maps = ShadowMaps::new(&self.device, &self.scene.depth_view_layout, map_config);
        }
        let targets = self.targets.as_ref().expect("targets just created");
        let (camera, lighting) = (inputs.camera, inputs.lighting);
        let aspect = targets.grid.0 as f32 / targets.grid.1 as f32;
        let (view, projection) = (camera.view(), camera.projection(aspect));

        // Shadow views: sun cascades, then six faces for each shadowed light.
        let cascades = match settings.sun_shadows {
            Some(sun) if lighting.sun.length_squared() > 0.0 => {
                shadows::fit_cascades(camera, aspect, lighting.sun_direction, &sun)
            }
            _ => Vec::new(),
        };
        self.cascade_casters = self.scene.update_cascade_casters(&cascades);
        let shadowed = shadows::pick_shadowed(inputs.lights, camera.position, settings.local_shadows.budget);
        let mut slots = vec![None; inputs.lights.len()];
        let mut faces = Vec::with_capacity(shadowed.len());
        for (slot, &index) in shadowed.iter().enumerate() {
            let light = &inputs.lights[index];
            slots[index] = Some(slot as u32);
            faces.push(shadows::cube_faces(light.position, light.range));
            self.scene.update_local_casters(slot, light.position, light.range);
        }
        for (i, cascade) in cascades.iter().enumerate() {
            self.shadow_maps.write_view(&self.queue, shadow_maps::cascade_view(i), cascade.view_proj);
        }
        for (slot, matrices) in faces.iter().enumerate() {
            for (face, m) in matrices.iter().enumerate() {
                self.shadow_maps.write_view(&self.queue, shadow_maps::face_view(slot, face), *m);
            }
        }
        let prepass = settings.ssao.is_some() || settings.contact_shadows.is_some();
        let prepass_view = shadow_maps::PREPASS_VIEW * shadow_maps::VIEW_STRIDE;
        self.shadow_maps.write_view(&self.queue, prepass_view, projection * view);
        self.shadow_maps
            .write_uniform(&self.queue, &ShadowUniform::new(settings, &cascades, &faces, camera.forward(), prepass));

        let frame = FrameUniform::new(
            projection * view,
            camera.position,
            inputs.time,
            lighting,
            inputs.lights,
            &slots,
            inputs.game,
        );
        self.scene.prepare(&frame, inputs.atmosphere, inputs.time);

        let profiler = if self.profiling { &mut self.profiler } else { &mut None };
        for (i, layer) in self.shadow_maps.sun_layers.iter().enumerate().take(cascades.len()) {
            let mut pass = depth_pass(encoder, profiler, "sun shadows", layer);
            self.scene.record_shadow(&mut pass, i, &self.shadow_maps.view_group, shadow_maps::cascade_view(i));
        }
        for slot in 0..shadowed.len() {
            for face in 0..6 {
                let layer = &self.shadow_maps.local_layers[slot * 6 + face];
                let mut pass = depth_pass(encoder, profiler, "local shadows", layer);
                let offset = shadow_maps::face_view(slot, face);
                self.scene.record_local_shadow(&mut pass, slot, &self.shadow_maps.view_group, offset);
            }
        }

        if prepass && self.screen_targets.as_ref().is_none_or(|(id, _)| *id != targets.id) {
            self.screen_targets = Some((targets.id, self.screen.targets(&self.device, targets.grid, &targets.depth)));
        }
        let screen_targets = self.screen_targets.as_ref().filter(|_| prepass).map(|(_, t)| t);
        if let Some(screen_targets) = screen_targets {
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("prepass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &screen_targets.normal,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &targets.depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: profiler.as_mut().and_then(|p| p.pass("prepass")),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                self.scene.record_prepass(&mut pass, &self.shadow_maps.view_group, prepass_view);
            }
            let screen_camera = ScreenCamera { proj: projection, view, to_sun: lighting.sun_direction };
            self.screen.record(&self.queue, encoder, screen_targets, settings, &screen_camera, profiler);
        }
        let (key, occlusion) = match screen_targets {
            Some(t) => (targets.id, &t.occlusion),
            None => (u64::MAX, &self.no_occlusion),
        };
        let shadow_group = self.shadow_maps.scene_group(&self.device, &self.scene.shadow_layout, key, occlusion);

        let layers = AtmosphereLayers {
            decals: settings.atmosphere && inputs.atmosphere.decal_opacity > 0.0,
            fog_sheets: settings.atmosphere && inputs.atmosphere.fog_sheet_density > 0.0,
        };
        {
            let fog = lighting.fog_color;
            let depth_load = if prepass { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(0.0) };
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
                    depth_ops: Some(wgpu::Operations { load: depth_load, store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: profiler.as_mut().and_then(|p| p.pass("scene")),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.scene.record_main(&mut pass, shadow_group, prepass, &layers);
        }

        let retro = RetroUniform::new(
            inputs.retro,
            output_size,
            inputs.time,
            settings.post_detail,
            self.color_format.is_srgb(),
        );
        targets.record(&self.queue, &self.post, encoder, output, &retro, profiler);
        if let Some(profiler) = profiler {
            profiler.resolve(encoder);
        }
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

    /// Renders `frames` offscreen frames, waiting for the GPU after each one.
    /// No readback, so it measures render cost only.
    pub fn bench(&mut self, size: (u32, u32), inputs: &FrameInputs, frames: u32) -> Result<BenchResult> {
        let view = self.output_texture(size, wgpu::TextureUsages::empty()).create_view(&Default::default());
        let frames = frames.max(1);
        let mut gpu_passes: Vec<(&'static str, f64)> = Vec::new();
        self.profiling = self.profiler.is_some();
        let started = std::time::Instant::now();
        for _ in 0..frames {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            self.render(&mut encoder, &view, size, inputs);
            self.queue.submit([encoder.finish()]);
            self.device.poll(wgpu::PollType::wait_indefinitely())?;
            if let Some(profiler) = self.profiler.as_mut().filter(|_| self.profiling) {
                for (label, ms) in profiler.read(&self.device)? {
                    match gpu_passes.iter_mut().find(|(l, _)| *l == label) {
                        Some((_, total)) => *total += ms,
                        None => gpu_passes.push((label, ms)),
                    }
                }
            }
        }
        let frame_ms = started.elapsed().as_secs_f64() * 1e3 / frames as f64;
        self.profiling = false;
        for (_, total) in &mut gpu_passes {
            *total /= frames as f64;
        }
        Ok(BenchResult { frame_ms, gpu_passes, cascade_casters: self.cascade_casters.clone() })
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
