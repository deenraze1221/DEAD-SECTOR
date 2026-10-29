//! The 3D pass: instanced city geometry, then light pools and ground mist,
//! rendered into the HDR target at pixel-grid resolution.

use crate::environment::Environment;
use crate::lights::{MAX_LIGHTS, PointLight, district_of};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use ds_assets::SceneBuffers;
use ds_assets::scene::{Instance, PassKind, Vertex};
use glam::{Mat4, Vec3, Vec4};
use std::path::Path;
use wgpu::util::DeviceExt;

pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const FLAG_UNLIT: u32 = 1;
const FLAG_ALPHA_TEST: u32 = 2;
const FLAG_POWERED: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    position_range: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct FrameUniform {
    view_proj: [f32; 16],
    camera_pos: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient: [f32; 4],
    fog: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    ground_horizon: [f32; 4],
    ground_bottom: [f32; 4],
    params: [f32; 4],
    lights: [GpuLight; MAX_LIGHTS],
}

impl FrameUniform {
    pub(crate) fn new(view_proj: Mat4, eye: Vec3, time: f32, env: &Environment, lights: &[PointLight]) -> Self {
        let v4 = |v: Vec3, w: f32| v.extend(w).to_array();
        let mut gpu_lights = [GpuLight::zeroed(); MAX_LIGHTS];
        for (slot, light) in gpu_lights.iter_mut().zip(lights) {
            *slot = GpuLight { position_range: v4(light.position, light.range), color: v4(light.color, 0.0) };
        }
        FrameUniform {
            view_proj: view_proj.to_cols_array(),
            camera_pos: v4(eye, time),
            sun_dir: v4(env.sun_direction.normalize(), 0.0),
            sun_color: v4(env.sun, 0.0),
            ambient: v4(env.ambient, 0.0),
            fog: v4(env.fog_color, env.fog_density),
            sky_top: v4(env.sky_top, 0.0),
            sky_horizon: v4(env.sky_horizon, 0.0),
            ground_horizon: v4(env.ground_horizon, 0.0),
            ground_bottom: v4(env.ground_bottom, 0.0),
            params: [env.wetness, env.power_cut.map_or(-1.0, f32::from), lights.len().min(MAX_LIGHTS) as f32, 0.0],
            lights: gpu_lights,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    albedo: [f32; 4],
    emission: [f32; 4],
    pbr: [f32; 4],
    flags: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EffectsUniform {
    pool: [f32; 4],
    mist: [f32; 4],
    params: [f32; 4],
}

struct GpuDraw {
    pass: PassKind,
    material: usize,
    indices: std::ops::Range<u32>,
    base_vertex: i32,
    instances: std::ops::Range<u32>,
}

/// What the atmosphere layers should draw this frame.
pub(crate) struct Atmosphere {
    pub pools: bool,
    pub mist: bool,
}

pub(crate) struct ScenePass {
    queue: wgpu::Queue,
    opaque: wgpu::RenderPipeline,
    double_sided: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    pools: wgpu::RenderPipeline,
    mist: wgpu::RenderPipeline,
    frame_buffer: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    material_groups: Vec<wgpu::BindGroup>,
    effects_buffer: wgpu::Buffer,
    effects_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    lamp_buffer: wgpu::Buffer,
    lamps: Vec<Vec3>,
    applied_cut: Option<Option<u8>>,
    draws: Vec<GpuDraw>,
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(crate) fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

pub(crate) fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

pub(crate) fn fragment_uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    uniform_entry(binding, wgpu::ShaderStages::FRAGMENT)
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    data: &[u8],
) -> wgpu::TextureView {
    device
        .create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        )
        .create_view(&Default::default())
}

impl ScenePass {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &SceneBuffers,
        lamps: Vec<Vec3>,
        assets: &Path,
    ) -> Result<Self> {
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[fragment_uniform_entry(0), texture_entry(1), sampler_entry(2)],
        });
        let effects_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effects"),
            entries: &[fragment_uniform_entry(0), texture_entry(1), sampler_entry(2)],
        });

        let city_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("city.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("city.wgsl").into()),
        });
        let effects_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("effects.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("effects.wgsl").into()),
        });
        let city_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("city"),
            bind_group_layouts: &[Some(&frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let effects_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effects"),
            bind_group_layouts: &[Some(&frame_layout), Some(&effects_layout)],
            immediate_size: 0,
        });

        let vertex_attributes =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x3];
        let instance_attributes =
            wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4];
        let city_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attributes,
            }),
        ];
        let lamp_attributes = wgpu::vertex_attr_array![0 => Float32x4];
        let lamp_buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &lamp_attributes,
        })];

        struct Spec<'a> {
            label: &'a str,
            layout: &'a wgpu::PipelineLayout,
            shader: &'a wgpu::ShaderModule,
            entry: (&'a str, &'a str),
            buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
            cull: Option<wgpu::Face>,
            blend: wgpu::BlendState,
            depth_write: bool,
        }
        let pipeline = |spec: Spec| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(spec.label),
                layout: Some(spec.layout),
                vertex: wgpu::VertexState {
                    module: spec.shader,
                    entry_point: Some(spec.entry.0),
                    compilation_options: Default::default(),
                    buffers: spec.buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: spec.cull,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(spec.depth_write),
                    depth_compare: Some(wgpu::CompareFunction::Greater),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: spec.shader,
                    entry_point: Some(spec.entry.1),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: HDR_FORMAT,
                        blend: Some(spec.blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let city = |label, cull, blend, depth_write| Spec {
            label,
            layout: &city_layout,
            shader: &city_shader,
            entry: ("vs_main", "fs_main"),
            buffers: &city_buffers,
            cull,
            blend,
            depth_write,
        };
        let opaque = pipeline(city("opaque", Some(wgpu::Face::Back), wgpu::BlendState::REPLACE, true));
        let double_sided = pipeline(city("double-sided", None, wgpu::BlendState::REPLACE, true));
        let transparent = pipeline(city("transparent", None, wgpu::BlendState::ALPHA_BLENDING, false));
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let pools = pipeline(Spec {
            label: "light-pools",
            layout: &effects_pipeline_layout,
            shader: &effects_shader,
            entry: ("vs_pool", "fs_pool"),
            buffers: &lamp_buffers,
            cull: None,
            blend: additive,
            depth_write: false,
        });
        let mist = pipeline(Spec {
            label: "mist",
            layout: &effects_pipeline_layout,
            shader: &effects_shader,
            entry: ("vs_mist", "fs_mist"),
            buffers: &[],
            cull: None,
            blend: wgpu::BlendState::ALPHA_BLENDING,
            depth_write: false,
        });

        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: frame_buffer.as_entire_binding() }],
        });

        // Label textures use nearest filtering, as in bridge.gd; untextured materials sample white.
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("label"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let srgb = wgpu::TextureFormat::Rgba8UnormSrgb;
        let white = upload_texture(device, queue, "white", (1, 1), srgb, &[255; 4]);
        let mut labels = std::collections::HashMap::new();
        for index in scene.materials.iter().filter_map(|m| m.texture) {
            if let std::collections::hash_map::Entry::Vacant(slot) = labels.entry(index) {
                let image = ds_assets::load_label_texture(assets, index)?;
                let view = upload_texture(
                    device,
                    queue,
                    &format!("label-{index}"),
                    (image.width, image.height),
                    srgb,
                    &image.rgba,
                );
                slot.insert(view);
            }
        }
        let material_groups = scene
            .materials
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let flag = |set: bool, bit: u32| if set { bit } else { 0 };
                let flags =
                    flag(m.unlit, FLAG_UNLIT) | flag(m.alpha_test, FLAG_ALPHA_TEST) | flag(m.powered, FLAG_POWERED);
                let uniform = MaterialUniform {
                    albedo: m.albedo,
                    emission: [m.emission[0], m.emission[1], m.emission[2], 0.0],
                    pbr: [m.roughness, m.metallic, 0.0, 0.0],
                    flags: [flags, m.surface, 0, 0],
                };
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("material-{i}")),
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let texture = m.texture.map_or(&white, |t| &labels[&t]);
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("material-{i}")),
                    layout: &material_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(texture) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&nearest) },
                    ],
                })
            })
            .collect();

        let mist_size = 256;
        let noise = crate::noise::mist_texture(mist_size, 3, 3, 0x6d15);
        let noise_view =
            upload_texture(device, queue, "mist-noise", (mist_size, mist_size), wgpu::TextureFormat::R8Unorm, &noise);
        let repeat = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mist"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let effects_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effects"),
            size: std::mem::size_of::<EffectsUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let effects_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("effects"),
            layout: &effects_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: effects_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&noise_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&repeat) },
            ],
        });

        let buffer = |label: &str, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage })
        };
        let vertices = buffer("vertices", bytemuck::cast_slice(&scene.vertices), wgpu::BufferUsages::VERTEX);
        let indices = buffer("indices", bytemuck::cast_slice(&scene.indices), wgpu::BufferUsages::INDEX);
        let instances = buffer("instances", bytemuck::cast_slice(&scene.instances), wgpu::BufferUsages::VERTEX);
        let lamp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lamps"),
            size: (lamps.len().max(1) * 16) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let draws = scene
            .draws
            .iter()
            .map(|d| GpuDraw {
                pass: scene.materials[d.material].pass,
                material: d.material,
                indices: d.first_index..d.first_index + d.index_count,
                base_vertex: d.base_vertex,
                instances: d.first_instance..d.first_instance + d.instance_count,
            })
            .collect();

        Ok(ScenePass {
            queue: queue.clone(),
            opaque,
            double_sided,
            transparent,
            pools,
            mist,
            frame_buffer,
            frame_group,
            material_groups,
            effects_buffer,
            effects_group,
            vertices,
            indices,
            instances,
            lamp_buffer,
            lamps,
            applied_cut: None,
            draws,
        })
    }

    /// Uploads per-frame data. Lamp pools are rewritten only when the blackout changes.
    pub(crate) fn prepare(&mut self, frame: &FrameUniform, env: &Environment, time: f32) {
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(frame));
        let pool_color = Vec3::new(1.0, 0.85, 0.6).to_array().map(ds_assets::city::srgb_to_linear);
        let effects = EffectsUniform {
            pool: [pool_color[0], pool_color[1], pool_color[2], env.pool_alpha],
            mist: env.mist_color.extend(env.mist_density).to_array(),
            params: [time, 0.0, 0.0, 0.0],
        };
        self.queue.write_buffer(&self.effects_buffer, 0, bytemuck::bytes_of(&effects));
        if self.applied_cut != Some(env.power_cut) {
            let data: Vec<[f32; 4]> = self
                .lamps
                .iter()
                .map(|l| {
                    let live = Some(district_of(l.x, l.z)) != env.power_cut;
                    Vec4::new(l.x, l.y, l.z, if live { 1.0 } else { 0.0 }).to_array()
                })
                .collect();
            self.queue.write_buffer(&self.lamp_buffer, 0, bytemuck::cast_slice(&data));
            self.applied_cut = Some(env.power_cut);
        }
    }

    pub(crate) fn record(&self, pass: &mut wgpu::RenderPass<'_>, atmosphere: &Atmosphere) {
        pass.set_bind_group(0, &self.frame_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut current: Option<(PassKind, usize)> = None;
        for draw in &self.draws {
            if current.map(|c| c.0) != Some(draw.pass) {
                pass.set_pipeline(match draw.pass {
                    PassKind::Opaque => &self.opaque,
                    PassKind::OpaqueDoubleSided => &self.double_sided,
                    PassKind::Transparent => &self.transparent,
                });
            }
            if current.map(|c| c.1) != Some(draw.material) {
                pass.set_bind_group(1, &self.material_groups[draw.material], &[]);
            }
            current = Some((draw.pass, draw.material));
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, draw.instances.clone());
        }

        pass.set_bind_group(1, &self.effects_group, &[]);
        if atmosphere.pools && !self.lamps.is_empty() {
            pass.set_pipeline(&self.pools);
            pass.set_vertex_buffer(0, self.lamp_buffer.slice(..));
            pass.draw(0..6, 0..self.lamps.len() as u32);
        }
        if atmosphere.mist {
            pass.set_pipeline(&self.mist);
            pass.draw(0..6, 0..2);
        }
    }

    pub(crate) fn draw_count(&self) -> usize {
        self.draws.len()
    }
}
