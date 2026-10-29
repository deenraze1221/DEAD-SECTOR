//! Flattens a [`City`] into GPU-ready arrays: one shared vertex/index buffer,
//! one instance buffer and a list of instanced draws. Pure CPU code so it can
//! be unit-tested without a graphics device.

use crate::city::{City, Side, srgb_to_linear};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Instance {
    /// Column-major world matrix.
    pub model: [f32; 16],
    /// Linear RGB tint, alpha unused.
    pub color: [f32; 4],
}

/// How a material is rasterised; mirrors the material split in `godot/scripts/bridge.gd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PassKind {
    /// Opaque, back faces culled.
    Opaque,
    /// Opaque, both faces drawn (`side: DoubleSide`).
    OpaqueDoubleSided,
    /// Alpha blended, drawn after all opaque geometry.
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialParams {
    /// Linear RGB albedo, alpha = opacity.
    pub albedo: [f32; 4],
    /// Linear RGB emission already multiplied by its energy.
    pub emission: [f32; 3],
    pub unlit: bool,
    /// Discard fragments whose texture alpha is below 0.5 (Godot alpha scissor).
    pub alpha_test: bool,
    pub texture: Option<u32>,
    pub pass: PassKind,
}

impl MaterialParams {
    pub fn from_city(city: &City) -> Vec<Self> {
        city.materials
            .iter()
            .map(|m| {
                let linear = m.color.map(srgb_to_linear);
                let pass = if m.transparent && m.texture.is_none() {
                    PassKind::Transparent
                } else if m.side == Side::Double {
                    PassKind::OpaqueDoubleSided
                } else {
                    PassKind::Opaque
                };
                // Godot only gives emission to plain lit materials; procedural
                // surfaces and unlit "powered" lights ignore it.
                let emits = m.surface == 0 && !m.unlit && m.energy > 0.0;
                let emission = if emits { m.emissive.map(|c| srgb_to_linear(c) * m.energy) } else { [0.0; 3] };
                let opacity = if pass == PassKind::Transparent { m.opacity } else { 1.0 };
                MaterialParams {
                    albedo: [linear[0], linear[1], linear[2], opacity],
                    emission,
                    unlit: m.unlit,
                    alpha_test: m.texture.is_some(),
                    texture: m.texture,
                    pass,
                }
            })
            .collect()
    }
}

/// One instanced draw: an index range of the shared buffers, a material and
/// a contiguous range of the instance buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    pub batch: usize,
    pub material: usize,
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
    pub instance_count: u32,
}

pub struct SceneBuffers {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub instances: Vec<Instance>,
    pub materials: Vec<MaterialParams>,
    /// Sorted by pass, then material, to minimise state changes.
    pub draws: Vec<Draw>,
}

impl SceneBuffers {
    pub fn build(city: &City) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        // (first_index, base_vertex) per geometry.
        let mut ranges = Vec::with_capacity(city.geometries.len());
        for g in &city.geometries {
            let n = g.vertex_count();
            ranges.push((indices.len() as u32, vertices.len() as i32));
            for v in 0..n {
                let color = if g.color.is_empty() {
                    [1.0; 3]
                } else {
                    [g.color[v * 3], g.color[v * 3 + 1], g.color[v * 3 + 2]]
                };
                // three.js textures are flipped on upload (flipY), so v=0 is the image bottom.
                let uv = if g.uv.is_empty() { [0.0; 2] } else { [g.uv[v * 2], 1.0 - g.uv[v * 2 + 1]] };
                vertices.push(Vertex {
                    position: [g.position[v * 3], g.position[v * 3 + 1], g.position[v * 3 + 2]],
                    normal: [g.normal[v * 3], g.normal[v * 3 + 1], g.normal[v * 3 + 2]],
                    uv,
                    color,
                });
            }
            // three.js and wgpu both treat counter-clockwise triangles as front
            // faces, so unlike bridge.gd the winding is kept as authored.
            if g.index.is_empty() {
                indices.extend(0..n as u32);
            } else {
                indices.extend_from_slice(&g.index);
            }
        }

        let mut instances = Vec::with_capacity(city.instance_count());
        let mut draws = Vec::new();
        for (batch_index, b) in city.batches.iter().enumerate() {
            let first_instance = instances.len() as u32;
            for (i, model) in b.transforms.iter().enumerate() {
                let color = if b.colors.is_empty() {
                    [1.0; 4]
                } else {
                    [b.colors[i * 3], b.colors[i * 3 + 1], b.colors[i * 3 + 2], 1.0]
                };
                instances.push(Instance { model: *model, color });
            }
            let g = &city.geometries[b.geometry];
            let (first_index, base_vertex) = ranges[b.geometry];
            let instance_count = b.transforms.len() as u32;
            // Same rule as bridge.gd: material groups only apply to multi-material batches.
            if b.materials.len() > 1 && !g.groups.is_empty() {
                for group in &g.groups {
                    draws.push(Draw {
                        batch: batch_index,
                        material: b.materials[group.material_index as usize],
                        first_index: first_index + group.start,
                        index_count: group.count,
                        base_vertex,
                        first_instance,
                        instance_count,
                    });
                }
            } else {
                draws.push(Draw {
                    batch: batch_index,
                    material: b.materials[0],
                    first_index,
                    index_count: g.index_count() as u32,
                    base_vertex,
                    first_instance,
                    instance_count,
                });
            }
        }
        let materials = MaterialParams::from_city(city);
        draws.retain(|d| d.index_count > 0 && d.instance_count > 0);
        draws.sort_by_key(|d| (materials[d.material].pass, d.material, d.batch));
        SceneBuffers { vertices, indices, instances, materials, draws }
    }

    /// Triangles submitted per frame (before any culling).
    pub fn triangles_per_frame(&self) -> u64 {
        self.draws.iter().map(|d| d.index_count as u64 / 3 * d.instance_count as u64).sum()
    }
}
