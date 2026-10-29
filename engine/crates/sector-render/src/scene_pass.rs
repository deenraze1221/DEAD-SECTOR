//! Every pass that draws scene geometry: shadow casters, the depth + normal
//! prepass, the main lit pass, then ground decals and fog sheets.

use crate::gpu_util::{
    bind_group, comparison_sampler_entry, depth_texture_entry, fragment_uniform_entry, sampler_entry, texture_entry,
    unfiltered_texture_entry, uniform_entry, upload_texture,
};
use crate::lights::{MAX_LIGHTS, PointLight};
use crate::settings::{Atmosphere, Lighting};
use crate::shadows::{Cascade, CascadeCasters, InstanceBounds, MAX_CASCADES, MAX_SHADOWED_LIGHTS};
use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use sector_assets::{Image, Instance, PassKind, SceneBuffers, Vertex};
use wgpu::util::DeviceExt;

pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const FLAG_UNLIT: u32 = 1;
const FLAG_ALPHA_TEST: u32 = 2;
const FLAG_GLASS: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    position_range: [f32; 4],
    color: [f32; 4],
    shadow: [f32; 4],
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
    game: [f32; 4],
    lights: [GpuLight; MAX_LIGHTS],
}

impl FrameUniform {
    /// `shadow_slots[i]` is the shadow-map slot of `lights[i]`, if it has one.
    pub(crate) fn new(
        view_proj: Mat4,
        eye: Vec3,
        time: f32,
        env: &Lighting,
        lights: &[PointLight],
        shadow_slots: &[Option<u32>],
        game: [f32; 4],
    ) -> Self {
        let v4 = |v: Vec3, w: f32| v.extend(w).to_array();
        let mut gpu_lights = [GpuLight::zeroed(); MAX_LIGHTS];
        for (i, (slot, light)) in gpu_lights.iter_mut().zip(lights).enumerate() {
            let shadow = shadow_slots.get(i).copied().flatten().map_or(-1.0, |s| s as f32);
            *slot = GpuLight {
                position_range: v4(light.position, light.range),
                color: v4(light.color, 0.0),
                shadow: [shadow, 0.0, 0.0, 0.0],
            };
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
            params: [0.0, 0.0, lights.len().min(MAX_LIGHTS) as f32, 0.0],
            game,
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
    reach: [f32; 4],
}

struct GpuDraw {
    pass: PassKind,
    material: usize,
    alpha_test: bool,
    indices: std::ops::Range<u32>,
    base_vertex: i32,
    instances: std::ops::Range<u32>,
}

/// Which atmosphere layers to draw this frame.
pub(crate) struct AtmosphereLayers {
    pub decals: bool,
    pub fog_sheets: bool,
}

/// Main lit pipelines; one set clears depth itself, the other reuses the prepass depth.
struct MainPipelines {
    opaque: wgpu::RenderPipeline,
    double_sided: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
}

/// Casters inside one shadowed point light's range, as a compact instance buffer.
struct LocalCasters {
    key: Option<(Vec3, f32)>,
    buffer: wgpu::Buffer,
    /// (draw index, first instance in `buffer`, count)
    runs: Vec<(usize, u32, u32)>,
}

pub(crate) struct ScenePass {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub(crate) shadow_layout: wgpu::BindGroupLayout,
    pub(crate) depth_view_layout: wgpu::BindGroupLayout,
    main: [MainPipelines; 2],
    shadow_depth: wgpu::RenderPipeline,
    shadow_alpha: wgpu::RenderPipeline,
    culled_depth: wgpu::RenderPipeline,
    culled_alpha: wgpu::RenderPipeline,
    instance_storage: wgpu::BindGroup,
    /// Per cascade: buffer of instance indices and the draws they belong to.
    cascades: Vec<(wgpu::Buffer, CascadeCasters)>,
    prepass_back: wgpu::RenderPipeline,
    prepass_double: wgpu::RenderPipeline,
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
    instance_data: Vec<Instance>,
    bounds: InstanceBounds,
    local: Vec<LocalCasters>,
    decal_buffer: wgpu::Buffer,
    decal_count: u32,
    draws: Vec<GpuDraw>,
}

fn decal_buffer(device: &wgpu::Device, count: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("decals"),
        size: (count.max(1) * 16) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn index_buffer(device: &wgpu::Device, count: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cascade-casters"),
        size: (count.max(1) * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn instance_buffer(device: &wgpu::Device, count: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("local-casters"),
        size: (count.max(1) * std::mem::size_of::<Instance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[derive(Clone, Copy)]
struct Spec<'a> {
    label: &'a str,
    layout: &'a wgpu::PipelineLayout,
    shader: &'a wgpu::ShaderModule,
    vertex: &'a str,
    fragment: Option<&'a str>,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    cull: Option<wgpu::Face>,
    color: Option<(wgpu::TextureFormat, wgpu::BlendState)>,
    depth_format: wgpu::TextureFormat,
    depth_write: bool,
    compare: wgpu::CompareFunction,
    bias: wgpu::DepthBiasState,
}

fn pipeline(device: &wgpu::Device, spec: Spec) -> wgpu::RenderPipeline {
    let targets = [spec.color.map(|(format, blend)| wgpu::ColorTargetState {
        format,
        blend: Some(blend),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(spec.layout),
        vertex: wgpu::VertexState {
            module: spec.shader,
            entry_point: Some(spec.vertex),
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
            format: spec.depth_format,
            depth_write_enabled: Some(spec.depth_write),
            depth_compare: Some(spec.compare),
            stencil: Default::default(),
            bias: spec.bias,
        }),
        multisample: Default::default(),
        fragment: spec.fragment.map(|entry| wgpu::FragmentState {
            module: spec.shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: if spec.color.is_some() { &targets } else { &[] },
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl ScenePass {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &SceneBuffers,
        textures: &[Image],
        surface_hook: &str,
    ) -> Result<Self> {
        let layout = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(label), entries })
        };
        let frame_layout = layout("frame", &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT, false)]);
        let material_layout = layout("material", &[fragment_uniform_entry(0), texture_entry(1), sampler_entry(2)]);
        let effects_layout = layout(
            "effects",
            &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT, false), texture_entry(1), sampler_entry(2)],
        );
        let shadow_layout = layout(
            "shadows",
            &[
                fragment_uniform_entry(0),
                depth_texture_entry(1, wgpu::TextureViewDimension::D2Array),
                depth_texture_entry(2, wgpu::TextureViewDimension::D2Array),
                comparison_sampler_entry(3),
                unfiltered_texture_entry(4),
            ],
        );
        let depth_view_layout = layout("depth-view", &[uniform_entry(0, wgpu::ShaderStages::VERTEX, true)]);
        let storage_layout = layout(
            "instance-storage",
            &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );

        let shader = |label, source: String| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let scene_shader = shader("scene.wgsl", crate::SCENE_SHADER.replace("// SURFACE_HOOK", surface_hook));
        let depth_shader = shader("depth.wgsl", include_str!("depth.wgsl").to_string());
        let effects_shader = shader("effects.wgsl", include_str!("effects.wgsl").to_string());
        let pipeline_layout = |label, groups: &[&wgpu::BindGroupLayout]| {
            let groups: Vec<_> = groups.iter().map(|g| Some(*g)).collect();
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &groups,
                immediate_size: 0,
            })
        };
        let scene_layout = pipeline_layout("scene", &[&frame_layout, &material_layout, &shadow_layout]);
        let depth_layout = pipeline_layout("depth", &[&depth_view_layout, &material_layout]);
        let culled_layout = pipeline_layout("depth-culled", &[&depth_view_layout, &material_layout, &storage_layout]);
        let effects_pipeline_layout = pipeline_layout("effects", &[&frame_layout, &effects_layout]);

        let vertex_attributes =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x3];
        let instance_attributes =
            wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4];
        let geometry = [
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
        let index_attributes = wgpu::vertex_attr_array![9 => Uint32];
        let culled_geometry = [
            geometry[0].clone(),
            Some(wgpu::VertexBufferLayout {
                array_stride: 4,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &index_attributes,
            }),
        ];
        let decal_attributes = wgpu::vertex_attr_array![0 => Float32x4];
        let decal_buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &decal_attributes,
        })];

        let base = Spec {
            label: "",
            layout: &scene_layout,
            shader: &scene_shader,
            vertex: "vs_main",
            fragment: Some("fs_main"),
            buffers: &geometry,
            cull: Some(wgpu::Face::Back),
            color: Some((HDR_FORMAT, wgpu::BlendState::REPLACE)),
            depth_format: DEPTH_FORMAT,
            depth_write: true,
            compare: wgpu::CompareFunction::Greater,
            bias: Default::default(),
        };
        let main = |prepassed: bool| {
            // After the prepass, opaque depth is already final: test for equality, skip writes.
            let (compare, write) = if prepassed {
                (wgpu::CompareFunction::GreaterEqual, false)
            } else {
                (wgpu::CompareFunction::Greater, true)
            };
            MainPipelines {
                opaque: pipeline(device, Spec { label: "opaque", compare, depth_write: write, ..base }),
                double_sided: pipeline(
                    device,
                    Spec { label: "double-sided", cull: None, compare, depth_write: write, ..base },
                ),
                transparent: pipeline(
                    device,
                    Spec {
                        label: "transparent",
                        cull: None,
                        color: Some((HDR_FORMAT, wgpu::BlendState::ALPHA_BLENDING)),
                        depth_write: false,
                        ..base
                    },
                ),
            }
        };
        let main = [main(false), main(true)];

        // Casters render both faces; slope-scaled bias keeps surfaces from shadowing themselves.
        let shadow_bias = wgpu::DepthBiasState { constant: 2, slope_scale: 2.5, clamp: 0.0 };
        let caster = Spec {
            label: "shadow-casters",
            layout: &depth_layout,
            shader: &depth_shader,
            vertex: "vs_depth",
            fragment: None,
            cull: None,
            color: None,
            depth_format: SHADOW_FORMAT,
            compare: wgpu::CompareFunction::LessEqual,
            bias: shadow_bias,
            ..base
        };
        let shadow_depth = pipeline(device, caster);
        let shadow_alpha =
            pipeline(device, Spec { label: "shadow-casters-alpha", fragment: Some("fs_alpha_test"), ..caster });
        let culled = Spec {
            label: "sun-casters",
            layout: &culled_layout,
            vertex: "vs_shadow_culled",
            buffers: &culled_geometry,
            ..caster
        };
        let culled_depth = pipeline(device, culled);
        let culled_alpha =
            pipeline(device, Spec { label: "sun-casters-alpha", fragment: Some("fs_alpha_test"), ..culled });
        let prepass = Spec {
            label: "prepass",
            layout: &depth_layout,
            shader: &depth_shader,
            vertex: "vs_depth",
            fragment: Some("fs_prepass"),
            color: Some((NORMAL_FORMAT, wgpu::BlendState::REPLACE)),
            ..base
        };
        let prepass_back = pipeline(device, prepass);
        let prepass_double = pipeline(device, Spec { label: "prepass-double-sided", cull: None, ..prepass });

        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let effect =
            Spec { layout: &effects_pipeline_layout, shader: &effects_shader, cull: None, depth_write: false, ..base };
        let pools = pipeline(
            device,
            Spec {
                label: "ground-decals",
                vertex: "vs_pool",
                fragment: Some("fs_pool"),
                buffers: &decal_buffers,
                color: Some((HDR_FORMAT, additive)),
                ..effect
            },
        );
        let mist = pipeline(
            device,
            Spec {
                label: "fog-sheets",
                vertex: "vs_mist",
                fragment: Some("fs_mist"),
                buffers: &[],
                color: Some((HDR_FORMAT, wgpu::BlendState::ALPHA_BLENDING)),
                ..effect
            },
        );

        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_group = bind_group(device, "frame", &frame_layout, &[frame_buffer.as_entire_binding()]);

        // Textures use nearest filtering (crisp labels); untextured materials sample white.
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let srgb = wgpu::TextureFormat::Rgba8UnormSrgb;
        let white = upload_texture(device, queue, "white", (1, 1), srgb, &[255; 4]);
        let mut views = std::collections::HashMap::new();
        for index in scene.materials.iter().filter_map(|m| m.texture) {
            if let std::collections::hash_map::Entry::Vacant(slot) = views.entry(index) {
                let image = textures
                    .get(index as usize)
                    .with_context(|| format!("material uses texture {index}, only {} given", textures.len()))?;
                let label = format!("texture-{index}");
                slot.insert(upload_texture(device, queue, &label, (image.width, image.height), srgb, &image.rgba));
            }
        }
        let material_groups = scene
            .materials
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let flag = |set: bool, bit: u32| if set { bit } else { 0 };
                let uniform = MaterialUniform {
                    albedo: m.albedo,
                    emission: [m.emission[0], m.emission[1], m.emission[2], 0.0],
                    pbr: [m.roughness, m.metallic, 0.0, 0.0],
                    flags: [
                        flag(m.unlit, FLAG_UNLIT) | flag(m.alpha_test, FLAG_ALPHA_TEST) | flag(m.glass, FLAG_GLASS),
                        m.shading,
                        0,
                        0,
                    ],
                };
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("material-{i}")),
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let texture = m.texture.map_or(&white, |t| &views[&t]);
                bind_group(
                    device,
                    &format!("material-{i}"),
                    &material_layout,
                    &[
                        buffer.as_entire_binding(),
                        wgpu::BindingResource::TextureView(texture),
                        wgpu::BindingResource::Sampler(&nearest),
                    ],
                )
            })
            .collect();

        let mist_size = 256;
        let noise = crate::noise::mist_texture(mist_size, 3, 3, 0x6d15);
        let noise_view =
            upload_texture(device, queue, "mist-noise", (mist_size, mist_size), wgpu::TextureFormat::R8Unorm, &noise);
        let repeat = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fog-sheets"),
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
        let effects_group = bind_group(
            device,
            "effects",
            &effects_layout,
            &[
                effects_buffer.as_entire_binding(),
                wgpu::BindingResource::TextureView(&noise_view),
                wgpu::BindingResource::Sampler(&repeat),
            ],
        );

        let buffer = |label: &str, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage })
        };
        let instances = buffer(
            "instances",
            bytemuck::cast_slice(&scene.instances),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        );
        let instance_storage =
            bind_group(device, "instance-storage", &storage_layout, &[instances.as_entire_binding()]);
        let draws = scene
            .draws
            .iter()
            .map(|d| GpuDraw {
                pass: scene.materials[d.material].pass,
                material: d.material,
                alpha_test: scene.materials[d.material].alpha_test,
                indices: d.first_index..d.first_index + d.index_count,
                base_vertex: d.base_vertex,
                instances: d.first_instance..d.first_instance + d.instance_count,
            })
            .collect();
        Ok(ScenePass {
            device: device.clone(),
            queue: queue.clone(),
            shadow_layout,
            depth_view_layout,
            main,
            shadow_depth,
            shadow_alpha,
            prepass_back,
            prepass_double,
            pools,
            mist,
            frame_buffer,
            frame_group,
            material_groups,
            effects_buffer,
            effects_group,
            vertices: buffer("vertices", bytemuck::cast_slice(&scene.vertices), wgpu::BufferUsages::VERTEX),
            indices: buffer("indices", bytemuck::cast_slice(&scene.indices), wgpu::BufferUsages::INDEX),
            instances,
            instance_storage,
            cascades: (0..MAX_CASCADES).map(|_| (index_buffer(device, 1), CascadeCasters::default())).collect(),
            culled_depth,
            culled_alpha,
            instance_data: scene.instances.clone(),
            bounds: InstanceBounds::new(scene),
            local: (0..MAX_SHADOWED_LIGHTS)
                .map(|_| LocalCasters { key: None, buffer: instance_buffer(device, 1), runs: Vec::new() })
                .collect(),
            decal_buffer: decal_buffer(device, 1),
            decal_count: 0,
            draws,
        })
    }

    /// Replaces the ground decals: xyz = centre, w = radius (0 hides one).
    pub(crate) fn set_decals(&mut self, decals: &[[f32; 4]]) {
        if decals.len() as u64 * 16 > self.decal_buffer.size() {
            self.decal_buffer = decal_buffer(&self.device, decals.len());
        }
        self.queue.write_buffer(&self.decal_buffer, 0, bytemuck::cast_slice(decals));
        self.decal_count = decals.len() as u32;
    }

    /// Uploads per-frame data.
    pub(crate) fn prepare(&mut self, frame: &FrameUniform, atmosphere: &Atmosphere, time: f32) {
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(frame));
        let a = atmosphere;
        let effects = EffectsUniform {
            pool: a.decal_color.extend(a.decal_opacity).to_array(),
            mist: a.fog_sheet_color.extend(a.fog_sheet_density).to_array(),
            params: [time, a.fog_sheet_heights[0], a.fog_sheet_heights[1], a.fog_sheet_half_size],
            reach: [a.fog_sheet_reach[0], a.fog_sheet_reach[1], 0.0, 0.0],
        };
        self.queue.write_buffer(&self.effects_buffer, 0, bytemuck::bytes_of(&effects));
    }

    /// Collects the casters inside a shadowed light's range into slot `slot`.
    /// Only redone when the light moves or changes range.
    pub(crate) fn update_local_casters(&mut self, slot: usize, position: Vec3, range: f32) {
        let casters = &mut self.local[slot];
        if casters.key == Some((position, range)) {
            return;
        }
        let mut data = Vec::new();
        casters.runs.clear();
        for (index, offsets) in self.bounds.casters_for_light(position, range).into_iter().enumerate() {
            let draw = &self.draws[index];
            if draw.pass == PassKind::Transparent || offsets.is_empty() {
                continue;
            }
            casters.runs.push((index, data.len() as u32, offsets.len() as u32));
            data.extend(offsets.iter().map(|&o| self.instance_data[(draw.instances.start + o) as usize]));
        }
        if (data.len() * std::mem::size_of::<Instance>()) as u64 > casters.buffer.size() {
            casters.buffer = instance_buffer(&self.device, data.len().next_power_of_two());
        }
        self.queue.write_buffer(&casters.buffer, 0, bytemuck::cast_slice(&data));
        casters.key = Some((position, range));
    }

    fn bind_geometry(&self, pass: &mut wgpu::RenderPass<'_>, instances: &wgpu::Buffer) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, instances.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
    }

    /// Culls the opaque instances against each cascade and uploads the index
    /// lists. Returns the caster count per cascade.
    pub(crate) fn update_cascade_casters(&mut self, cascades: &[Cascade]) -> Vec<u32> {
        let draws = &self.draws;
        let casters = self.bounds.cascade_casters(cascades, |d| draws[d].pass != PassKind::Transparent);
        let mut counts = Vec::with_capacity(casters.len());
        for (slot, casters) in self.cascades.iter_mut().zip(casters) {
            if (casters.indices.len() * 4) as u64 > slot.0.size() {
                slot.0 = index_buffer(&self.device, casters.indices.len().next_power_of_two());
            }
            self.queue.write_buffer(&slot.0, 0, bytemuck::cast_slice(&casters.indices));
            counts.push(casters.indices.len() as u32);
            slot.1 = casters;
        }
        counts
    }

    /// Draws the culled casters of `cascade` into its shadow map.
    pub(crate) fn record_shadow(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        cascade: usize,
        view: &wgpu::BindGroup,
        offset: u32,
    ) {
        let (indices, casters) = &self.cascades[cascade];
        pass.set_bind_group(0, view, &[offset]);
        pass.set_bind_group(2, &self.instance_storage, &[]);
        self.bind_geometry(pass, indices);
        let mut alpha = None;
        for &(index, first, count) in &casters.runs {
            let draw = &self.draws[index];
            if alpha != Some(draw.alpha_test) {
                pass.set_pipeline(if draw.alpha_test { &self.culled_alpha } else { &self.culled_depth });
                alpha = Some(draw.alpha_test);
            }
            pass.set_bind_group(1, &self.material_groups[draw.material], &[]);
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, first..first + count);
        }
    }

    /// Draws the casters collected for a shadowed light into one cube face.
    pub(crate) fn record_local_shadow(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        slot: usize,
        view: &wgpu::BindGroup,
        offset: u32,
    ) {
        let casters = &self.local[slot];
        pass.set_bind_group(0, view, &[offset]);
        self.bind_geometry(pass, &casters.buffer);
        let mut alpha = None;
        for &(index, first, count) in &casters.runs {
            let draw = &self.draws[index];
            if alpha != Some(draw.alpha_test) {
                pass.set_pipeline(if draw.alpha_test { &self.shadow_alpha } else { &self.shadow_depth });
                alpha = Some(draw.alpha_test);
            }
            pass.set_bind_group(1, &self.material_groups[draw.material], &[]);
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, first..first + count);
        }
    }

    /// Camera depth + world normals for the screen-space passes.
    pub(crate) fn record_prepass(&self, pass: &mut wgpu::RenderPass<'_>, view: &wgpu::BindGroup, offset: u32) {
        pass.set_bind_group(0, view, &[offset]);
        self.bind_geometry(pass, &self.instances);
        let mut current = None;
        for draw in self.draws.iter().filter(|d| d.pass != PassKind::Transparent) {
            if current != Some(draw.pass) {
                let double = draw.pass == PassKind::OpaqueDoubleSided;
                pass.set_pipeline(if double { &self.prepass_double } else { &self.prepass_back });
                current = Some(draw.pass);
            }
            pass.set_bind_group(1, &self.material_groups[draw.material], &[]);
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, draw.instances.clone());
        }
    }

    /// The lit scene, then the atmosphere layers. `prepassed` selects the
    /// pipelines that reuse the prepass depth.
    pub(crate) fn record_main(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        shadows: &wgpu::BindGroup,
        prepassed: bool,
        layers: &AtmosphereLayers,
    ) {
        let pipelines = &self.main[prepassed as usize];
        pass.set_bind_group(0, &self.frame_group, &[]);
        pass.set_bind_group(2, shadows, &[]);
        self.bind_geometry(pass, &self.instances);
        let mut current: Option<(PassKind, usize)> = None;
        for draw in &self.draws {
            if current.map(|c| c.0) != Some(draw.pass) {
                pass.set_pipeline(match draw.pass {
                    PassKind::Opaque => &pipelines.opaque,
                    PassKind::OpaqueDoubleSided => &pipelines.double_sided,
                    PassKind::Transparent => &pipelines.transparent,
                });
            }
            if current.map(|c| c.1) != Some(draw.material) {
                pass.set_bind_group(1, &self.material_groups[draw.material], &[]);
            }
            current = Some((draw.pass, draw.material));
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, draw.instances.clone());
        }

        pass.set_bind_group(1, &self.effects_group, &[]);
        if layers.decals && self.decal_count > 0 {
            pass.set_pipeline(&self.pools);
            pass.set_vertex_buffer(0, self.decal_buffer.slice(..));
            pass.draw(0..6, 0..self.decal_count);
        }
        if layers.fog_sheets {
            pass.set_pipeline(&self.mist);
            pass.draw(0..6, 0..2);
        }
    }

    pub(crate) fn draw_count(&self) -> usize {
        self.draws.len()
    }
}
