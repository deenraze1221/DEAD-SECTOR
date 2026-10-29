//! Render targets and the post chain: glow mip chain → tonemap → retro pass.

use crate::gpu_util::{fragment_uniform_entry, sampler_entry, texture_entry};
use crate::profiler::GpuProfiler;
use crate::scene_pass::{DEPTH_FORMAT, HDR_FORMAT};
use crate::settings::{PostSettings, RetroSettings};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Tonemapped, sRGB-encoded scene at grid resolution, read by the pixels pass.
const LDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Godot's default glow levels 1..=7 are [0, 0, 1, 0, 1, 0, 0]: levels 3 and 5 contribute.
const GLOW_LEVELS: usize = 5;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlowUniform {
    params: [f32; 4],
    threshold: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TonemapUniform {
    params: [f32; 4],
    bcs: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct RetroUniform {
    output: [f32; 4],
    effects: [f32; 4],
    tint: [f32; 4],
    shadow_tint: [f32; 4],
    highlight_tint: [f32; 4],
    finish: [f32; 4],
    edges: [[f32; 8]; 2],
}

impl RetroUniform {
    pub(crate) fn new(r: &RetroSettings, output: (u32, u32), time: f32, detail: f32, srgb_target: bool) -> Self {
        let edge = |e: &crate::settings::EdgeTint| {
            let c = e.color.extend(e.strength).to_array();
            [c[0], c[1], c[2], c[3], e.inner, e.outer, 0.0, 0.0]
        };
        RetroUniform {
            output: [output.0 as f32, output.1 as f32, time, detail],
            effects: [r.lens_rain, 0.0, 0.0, if srgb_target { 1.0 } else { 0.0 }],
            tint: r.tint.extend(r.saturation).to_array(),
            shadow_tint: r.shadow_tint.extend(r.highlight_start).to_array(),
            highlight_tint: r.highlight_tint.extend(r.gamma).to_array(),
            finish: [r.s_curve, r.vignette, r.scanlines, r.grain],
            edges: [edge(&r.edges[0]), edge(&r.edges[1])],
        }
    }
}

/// The 3D resolution for an output size: at most `max_lines` tall (never upscaled).
pub fn grid_size(output: (u32, u32), max_lines: Option<u32>) -> (u32, u32) {
    let Some(lines) = max_lines else { return (output.0.max(1), output.1.max(1)) };
    let scale = (lines as f32 / output.1.max(1) as f32).min(1.0);
    let scaled = |v: u32| ((v as f32 * scale).round() as u32).max(1);
    (scaled(output.0), scaled(output.1))
}

/// Size of glow level `k` (1 = half of the grid).
fn level_size(grid: (u32, u32), k: usize) -> (u32, u32) {
    ((grid.0 >> k).max(1), (grid.1 >> k).max(1))
}

pub(crate) struct PostPipelines {
    glow_layout: wgpu::BindGroupLayout,
    tonemap_layout: wgpu::BindGroupLayout,
    pixels_layout: wgpu::BindGroupLayout,
    glow_horizontal: wgpu::RenderPipeline,
    glow_vertical: wgpu::RenderPipeline,
    tonemap: wgpu::RenderPipeline,
    pixels: wgpu::RenderPipeline,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
}

impl PostPipelines {
    pub(crate) fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });
        let layout = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(label), entries })
        };
        let glow_layout = layout("glow", &[fragment_uniform_entry(0), texture_entry(1), sampler_entry(2)]);
        let tonemap_layout = layout(
            "tonemap",
            &[fragment_uniform_entry(0), texture_entry(1), texture_entry(2), texture_entry(3), sampler_entry(4)],
        );
        let pixels_layout = layout("pixels", &[fragment_uniform_entry(0), texture_entry(1), sampler_entry(2)]);
        let pipeline = |label, group: &wgpu::BindGroupLayout, entry, format| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(group)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_fullscreen"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let sampler = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        PostPipelines {
            glow_horizontal: pipeline("glow-horizontal", &glow_layout, "fs_glow_horizontal", HDR_FORMAT),
            glow_vertical: pipeline("glow-vertical", &glow_layout, "fs_glow_vertical", HDR_FORMAT),
            tonemap: pipeline("tonemap", &tonemap_layout, "fs_tonemap", LDR_FORMAT),
            pixels: pipeline("pixels", &pixels_layout, "fs_pixels", output_format),
            glow_layout,
            tonemap_layout,
            pixels_layout,
            linear: sampler("post-linear", wgpu::FilterMode::Linear),
            nearest: sampler("post-nearest", wgpu::FilterMode::Nearest),
        }
    }
}

struct GlowStep {
    target: wgpu::TextureView,
    group: wgpu::BindGroup,
    pipeline_horizontal: bool,
}

/// Every texture and bind group that depends on the output size.
pub(crate) struct Targets {
    /// Unique per allocation, so dependent bind groups know when to rebuild.
    pub id: u64,
    pub output: (u32, u32),
    pub grid: (u32, u32),
    pub max_lines: Option<u32>,
    pub hdr: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    glow_steps: Vec<GlowStep>,
    ldr: wgpu::TextureView,
    tonemap_group: wgpu::BindGroup,
    pixels_group: wgpu::BindGroup,
    pixels_buffer: wgpu::Buffer,
}

fn render_texture(
    device: &wgpu::Device,
    label: &str,
    size: (u32, u32),
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

impl Targets {
    pub(crate) fn new(
        device: &wgpu::Device,
        post: &PostPipelines,
        output: (u32, u32),
        max_lines: Option<u32>,
        settings: &PostSettings,
    ) -> Self {
        let grid = grid_size(output, max_lines);
        let hdr = render_texture(device, "hdr", grid, HDR_FORMAT);
        let depth = render_texture(device, "depth", grid, DEPTH_FORMAT);
        let ldr = render_texture(device, "ldr", grid, LDR_FORMAT);

        let bind = |layout: &wgpu::BindGroupLayout,
                    label: &str,
                    buffer: &wgpu::Buffer,
                    textures: &[&wgpu::TextureView],
                    sampler: &wgpu::Sampler| {
            let mut entries = vec![wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }];
            for (i, view) in textures.iter().enumerate() {
                entries.push(wgpu::BindGroupEntry {
                    binding: i as u32 + 1,
                    resource: wgpu::BindingResource::TextureView(view),
                });
            }
            entries.push(wgpu::BindGroupEntry {
                binding: textures.len() as u32 + 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(label), layout, entries: &entries })
        };
        let uniform = |label: &str, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage })
        };

        // Each level: horizontal blur from the previous level into a scratch
        // texture, then vertical blur into the level itself.
        let mut glow_steps = Vec::with_capacity(GLOW_LEVELS * 2);
        let mut levels = Vec::with_capacity(GLOW_LEVELS);
        for k in 1..=GLOW_LEVELS {
            let size = level_size(grid, k);
            let texel = [1.0 / size.0 as f32, 1.0 / size.1 as f32];
            let threshold = [
                settings.glow_hdr_threshold,
                settings.glow_hdr_scale,
                settings.glow_bloom,
                settings.glow_luminance_cap,
            ];
            let first = if k == 1 { 1.0 } else { 0.0 };
            let scratch = render_texture(device, &format!("glow-scratch-{k}"), size, HDR_FORMAT);
            let level = render_texture(device, &format!("glow-{k}"), size, HDR_FORMAT);
            let source = if k == 1 { &hdr } else { &levels[k - 2] };
            let horizontal = GlowUniform { params: [texel[0], texel[1], settings.glow_strength, first], threshold };
            let buffer = uniform("glow-h", bytemuck::bytes_of(&horizontal), wgpu::BufferUsages::UNIFORM);
            glow_steps.push(GlowStep {
                group: bind(&post.glow_layout, "glow-h", &buffer, &[source], &post.linear),
                target: scratch.clone(),
                pipeline_horizontal: true,
            });
            let vertical = GlowUniform { params: [texel[0], texel[1], settings.glow_strength, 0.0], threshold };
            let buffer = uniform("glow-v", bytemuck::bytes_of(&vertical), wgpu::BufferUsages::UNIFORM);
            glow_steps.push(GlowStep {
                group: bind(&post.glow_layout, "glow-v", &buffer, &[&scratch], &post.linear),
                target: level.clone(),
                pipeline_horizontal: false,
            });
            levels.push(level);
        }

        let tonemap_uniform = TonemapUniform {
            params: [settings.exposure, settings.white, settings.glow_intensity, 0.0],
            bcs: [settings.brightness, settings.contrast, settings.saturation, 0.0],
        };
        let tonemap_buffer = uniform("tonemap", bytemuck::bytes_of(&tonemap_uniform), wgpu::BufferUsages::UNIFORM);
        let tonemap_group =
            bind(&post.tonemap_layout, "tonemap", &tonemap_buffer, &[&hdr, &levels[2], &levels[4]], &post.linear);
        let pixels_buffer = uniform(
            "pixels",
            bytemuck::bytes_of(&RetroUniform::zeroed()),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let pixels_group = bind(&post.pixels_layout, "pixels", &pixels_buffer, &[&ldr], &post.nearest);
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Targets { id, output, grid, max_lines, hdr, depth, glow_steps, ldr, tonemap_group, pixels_group, pixels_buffer }
    }

    pub(crate) fn record(
        &self,
        queue: &wgpu::Queue,
        post: &PostPipelines,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        pixels: &RetroUniform,
        profiler: &mut Option<GpuProfiler>,
    ) {
        queue.write_buffer(&self.pixels_buffer, 0, bytemuck::bytes_of(pixels));
        let mut fullscreen =
            |label: &str, target: &wgpu::TextureView, pipeline: &wgpu::RenderPipeline, group: &wgpu::BindGroup| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: profiler.as_mut().and_then(|p| p.pass("post")),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.draw(0..3, 0..1);
            };
        for step in &self.glow_steps {
            let pipeline = if step.pipeline_horizontal { &post.glow_horizontal } else { &post.glow_vertical };
            fullscreen("glow", &step.target, pipeline, &step.group);
        }
        fullscreen("tonemap", &self.ldr, &post.tonemap, &self.tonemap_group);
        fullscreen("pixels", output, &post.pixels, &self.pixels_group);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_matches_pixels_shader() {
        let psx = Some(540);
        assert_eq!(grid_size((1280, 720), psx), (960, 540));
        assert_eq!(grid_size((1920, 1080), psx), (960, 540));
        assert_eq!(grid_size((2560, 1440), psx), (960, 540));
        // Windows shorter than the limit are not upscaled.
        assert_eq!(grid_size((640, 360), psx), (640, 360));
        assert_eq!(grid_size((3440, 1440), psx), (1290, 540));
        assert_eq!(grid_size((2560, 1440), None), (2560, 1440));
    }

    #[test]
    fn glow_levels_halve() {
        assert_eq!(level_size((960, 540), 1), (480, 270));
        assert_eq!(level_size((960, 540), 3), (120, 67));
        assert_eq!(level_size((960, 540), 5), (30, 16));
        assert_eq!(level_size((20, 10), 5), (1, 1));
    }
}
