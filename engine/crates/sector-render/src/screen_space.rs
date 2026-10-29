//! Screen-space ambient occlusion and sun contact shadows, computed from the
//! depth + normal prepass at the 3D resolution.

use crate::gpu_util::{depth_texture_entry, fragment_uniform_entry, render_texture, unfiltered_texture_entry};
use crate::profiler::GpuProfiler;
use crate::settings::RenderSettings;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// R = ambient occlusion, G = contact shadow.
pub(crate) const OCCLUSION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OcclusionUniform {
    proj: [f32; 16],
    inv_proj: [f32; 16],
    view: [f32; 16],
    sun_view: [f32; 4],
    params: [f32; 4],
    params2: [f32; 4],
}

pub(crate) struct ScreenSpace {
    occlusion: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    occlusion_layout: wgpu::BindGroupLayout,
    blur_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
}

/// Camera matrices and sun direction for the screen-space passes.
pub(crate) struct ScreenCamera {
    pub proj: Mat4,
    pub view: Mat4,
    /// World-space unit vector towards the sun.
    pub to_sun: Vec3,
}

/// Size-dependent textures and bind groups.
pub(crate) struct ScreenTargets {
    pub normal: wgpu::TextureView,
    raw: wgpu::TextureView,
    pub occlusion: wgpu::TextureView,
    occlusion_group: wgpu::BindGroup,
    blur_group: wgpu::BindGroup,
}

impl ScreenSpace {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("occlusion.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("occlusion.wgsl").into()),
        });
        let layout = |label, last: u32| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    fragment_uniform_entry(0),
                    depth_texture_entry(1, wgpu::TextureViewDimension::D2),
                    unfiltered_texture_entry(last),
                ],
            })
        };
        let occlusion_layout = layout("occlusion", 2);
        let blur_layout = layout("occlusion-blur", 3);
        let pipeline = |label, group: &wgpu::BindGroupLayout, entry| {
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
                        format: OCCLUSION_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        ScreenSpace {
            occlusion: pipeline("occlusion", &occlusion_layout, "fs_occlusion"),
            blur: pipeline("occlusion-blur", &blur_layout, "fs_blur"),
            occlusion_layout,
            blur_layout,
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("occlusion"),
                size: std::mem::size_of::<OcclusionUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    pub(crate) fn targets(&self, device: &wgpu::Device, size: (u32, u32), depth: &wgpu::TextureView) -> ScreenTargets {
        let view = |label, format| render_texture(device, label, size, 1, format).create_view(&Default::default());
        let normal = view("prepass-normals", crate::scene_pass::NORMAL_FORMAT);
        let raw = view("occlusion-raw", OCCLUSION_FORMAT);
        let occlusion = view("occlusion", OCCLUSION_FORMAT);
        let group = |label, layout, binding: u32, input: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniform.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depth) },
                    wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(input) },
                ],
            })
        };
        ScreenTargets {
            occlusion_group: group("occlusion", &self.occlusion_layout, 2, &normal),
            blur_group: group("occlusion-blur", &self.blur_layout, 3, &raw),
            normal,
            raw,
            occlusion,
        }
    }

    /// Computes occlusion into `targets.occlusion`.
    pub(crate) fn record(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: &ScreenTargets,
        settings: &RenderSettings,
        camera: &ScreenCamera,
        profiler: &mut Option<GpuProfiler>,
    ) {
        let (ssao, contact) = (settings.ssao, settings.contact_shadows);
        let uniform = OcclusionUniform {
            proj: camera.proj.to_cols_array(),
            inv_proj: camera.proj.inverse().to_cols_array(),
            view: camera.view.to_cols_array(),
            sun_view: camera.view.transform_vector3(camera.to_sun).normalize().extend(0.0).to_array(),
            params: [
                ssao.map_or(0.0, |s| s.radius),
                ssao.map_or(0.0, |s| s.intensity),
                ssao.map_or(0.0, |s| s.samples.clamp(1, 64) as f32),
                contact.map_or(0.0, |c| c.length),
            ],
            params2: [contact.map_or(0.0, |c| c.steps.clamp(1, 64) as f32), 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
        for (label, target, pipeline, group) in [
            ("occlusion", &targets.raw, &self.occlusion, &targets.occlusion_group),
            ("occlusion-blur", &targets.occlusion, &self.blur, &targets.blur_group),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: profiler.as_mut().and_then(|p| p.pass("ssao + contact")),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
