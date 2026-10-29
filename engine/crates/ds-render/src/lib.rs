//! wgpu renderer for the Dead Sector city.

pub mod view;

pub use view::{Camera, Lighting};

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use ds_assets::SceneBuffers;
use ds_assets::scene::{Instance, PassKind, Vertex};
use std::path::Path;
use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Format used for offscreen captures.
pub const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const FLAG_UNLIT: u32 = 1;
const FLAG_ALPHA_TEST: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameUniform {
    view_proj: [f32; 16],
    camera_pos: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient: [f32; 4],
    fog: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    albedo: [f32; 4],
    emission: [f32; 4],
    flags: [u32; 4],
}

/// Per-frame counters shown in the window title and logs.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws: usize,
    pub instances: usize,
    pub triangles: u64,
}

struct GpuDraw {
    pass: PassKind,
    material: usize,
    indices: std::ops::Range<u32>,
    base_vertex: i32,
    instances: std::ops::Range<u32>,
}

struct DepthTarget {
    size: (u32, u32),
    view: wgpu::TextureView,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    color_format: wgpu::TextureFormat,
    opaque: wgpu::RenderPipeline,
    double_sided: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    frame_buffer: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    material_groups: Vec<wgpu::BindGroup>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    draws: Vec<GpuDraw>,
    depth: Option<DepthTarget>,
    stats: SceneStats,
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

impl Renderer {
    /// Uploads the scene and builds pipelines. `assets` is used to load label textures.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        scene: &SceneBuffers,
        assets: &Path,
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("city.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("city.wgsl").into()),
        });

        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("city"),
            bind_group_layouts: &[Some(&frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });

        let vertex_attributes =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x3];
        let instance_attributes =
            wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4];
        let buffers = [
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
        let pipeline = |label: &str, cull: Option<wgpu::Face>, blend: wgpu::BlendState, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: cull,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::Greater),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let opaque = pipeline("opaque", Some(wgpu::Face::Back), wgpu::BlendState::REPLACE, true);
        let double_sided = pipeline("double-sided", None, wgpu::BlendState::REPLACE, true);
        let transparent = pipeline("transparent", None, wgpu::BlendState::ALPHA_BLENDING, false);

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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("label"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let upload = |label: &str, width: u32, height: u32, rgba: &[u8]| {
            device
                .create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    rgba,
                )
                .create_view(&Default::default())
        };
        let white = upload("white", 1, 1, &[255; 4]);
        let mut labels = std::collections::HashMap::new();
        for index in scene.materials.iter().filter_map(|m| m.texture) {
            if let std::collections::hash_map::Entry::Vacant(slot) = labels.entry(index) {
                let image = ds_assets::load_label_texture(assets, index)?;
                slot.insert(upload(&format!("label-{index}"), image.width, image.height, &image.rgba));
            }
        }

        let material_groups = scene
            .materials
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let flags = if m.unlit { FLAG_UNLIT } else { 0 } | if m.alpha_test { FLAG_ALPHA_TEST } else { 0 };
                let uniform = MaterialUniform {
                    albedo: m.albedo,
                    emission: [m.emission[0], m.emission[1], m.emission[2], 0.0],
                    flags: [flags, 0, 0, 0],
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
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                    ],
                })
            })
            .collect();

        let buffer = |label: &str, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage })
        };
        let vertices = buffer("vertices", bytemuck::cast_slice(&scene.vertices), wgpu::BufferUsages::VERTEX);
        let indices = buffer("indices", bytemuck::cast_slice(&scene.indices), wgpu::BufferUsages::INDEX);
        let instances = buffer("instances", bytemuck::cast_slice(&scene.instances), wgpu::BufferUsages::VERTEX);

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
        let stats = SceneStats {
            draws: scene.draws.len(),
            instances: scene.instances.len(),
            triangles: scene.triangles_per_frame(),
        };

        Ok(Renderer {
            device: device.clone(),
            queue: queue.clone(),
            color_format,
            opaque,
            double_sided,
            transparent,
            frame_buffer,
            frame_group,
            material_groups,
            vertices,
            indices,
            instances,
            draws,
            depth: None,
            stats,
        })
    }

    pub fn stats(&self) -> SceneStats {
        self.stats
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    fn depth_view(&mut self, size: (u32, u32)) -> wgpu::TextureView {
        if self.depth.as_ref().is_none_or(|d| d.size != size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some(DepthTarget { size, view: texture.create_view(&Default::default()) });
        }
        self.depth.as_ref().expect("depth target just created").view.clone()
    }

    /// Records the city into `target` (which must use the renderer's colour format).
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: &Camera,
        lighting: &Lighting,
    ) {
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        let frame = FrameUniform {
            view_proj: camera.view_proj(aspect).to_cols_array(),
            camera_pos: camera.position.extend(1.0).to_array(),
            sun_dir: lighting.sun_direction.normalize().extend(0.0).to_array(),
            sun_color: lighting.sun.extend(0.0).to_array(),
            ambient: lighting.ambient.extend(0.0).to_array(),
            fog: lighting.fog_color.extend(lighting.fog_density).to_array(),
        };
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&frame));
        let depth = self.depth_view(size);
        let fog = lighting.fog_color;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("city"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                // Godot fogs the sky completely (fog_sky_affect = 1), so the background is the fog colour.
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
                view: &depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
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
    }

    /// Renders `frames` offscreen frames, waiting for the GPU after each one, and
    /// returns the total time. No readback, so it measures render cost only.
    pub fn bench(
        &mut self,
        size: (u32, u32),
        camera: &Camera,
        lighting: &Lighting,
        frames: u32,
    ) -> Result<std::time::Duration> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.color_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let started = std::time::Instant::now();
        for _ in 0..frames {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            self.render(&mut encoder, &view, size, camera, lighting);
            self.queue.submit([encoder.finish()]);
            self.device.poll(wgpu::PollType::wait_indefinitely())?;
        }
        Ok(started.elapsed())
    }

    /// Renders one frame offscreen and returns tightly packed sRGB RGBA8 pixels.
    /// The renderer must have been created with [`CAPTURE_FORMAT`].
    pub fn capture(&mut self, size: (u32, u32), camera: &Camera, lighting: &Lighting) -> Result<Vec<u8>> {
        anyhow::ensure!(self.color_format == CAPTURE_FORMAT, "capture needs a renderer created with CAPTURE_FORMAT");
        let (width, height) = size;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: CAPTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let row = width * 4;
        let padded_row = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture-readback"),
            size: padded_row as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.render(&mut encoder, &texture.create_view(&Default::default()), size, camera, lighting);
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
