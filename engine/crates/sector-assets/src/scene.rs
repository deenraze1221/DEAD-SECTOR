//! GPU-ready scene data: one shared vertex/index buffer, one instance buffer,
//! a material table and a list of instanced draws. Games fill it through
//! [`SceneBuilder`]; the renderer uploads it as is. Pure CPU code, so it can be
//! tested without a graphics device.

use bytemuck::{Pod, Zeroable};
use std::ops::Range;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Texture coordinates with v = 0 at the top of the image.
    pub uv: [f32; 2],
    /// Linear RGB vertex colour (white when the mesh has none).
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

/// How a material is rasterised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PassKind {
    /// Opaque, back faces culled.
    Opaque,
    /// Opaque, both faces drawn.
    OpaqueDoubleSided,
    /// Alpha blended, drawn after all opaque geometry; casts no shadow.
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialParams {
    /// Linear RGB albedo, alpha = opacity (used by [`PassKind::Transparent`]).
    pub albedo: [f32; 4],
    /// Linear RGB emission already multiplied by its energy.
    pub emission: [f32; 3],
    /// No lighting: the albedo is the final colour.
    pub unlit: bool,
    /// Discard fragments whose texture alpha is below 0.5.
    pub alpha_test: bool,
    /// Index into the texture list given to the renderer.
    pub texture: Option<u32>,
    pub pass: PassKind,
    pub roughness: f32,
    pub metallic: f32,
    /// Game-defined shading id passed to the surface hook; 0 means standard.
    pub shading: u32,
    /// Layer of the renderer's tiling material maps (see
    /// `sector_assets::materials`), projected in world space; the albedo
    /// above multiplies the map's colour.
    pub surface_map: Option<u32>,
    /// World metres covered by one repeat of the surface map.
    pub tiling: f32,
    /// Which faces get the surface map: [`FACES_UP`], [`FACES_DOWN`] and/or
    /// [`FACES_SIDE`] by the face's dominant axis (e.g. floors but not ceilings).
    pub surface_faces: u32,
    /// Window glass (with [`PassKind::Transparent`]): front faces only,
    /// Fresnel-weighted opacity, reflections not dimmed by the blend, and no
    /// screen-space occlusion from the surfaces behind it.
    pub glass: bool,
}

impl Default for MaterialParams {
    fn default() -> Self {
        MaterialParams {
            albedo: [1.0; 4],
            emission: [0.0; 3],
            unlit: false,
            alpha_test: false,
            texture: None,
            pass: PassKind::Opaque,
            roughness: 1.0,
            metallic: 0.0,
            shading: 0,
            surface_map: None,
            tiling: 1.0,
            surface_faces: FACES_ALL,
            glass: false,
        }
    }
}

/// Faces whose normal points mostly up (+Y).
pub const FACES_UP: u32 = 1;
/// Faces whose normal points mostly down (−Y).
pub const FACES_DOWN: u32 = 2;
/// Faces whose normal is mostly horizontal.
pub const FACES_SIDE: u32 = 4;
pub const FACES_ALL: u32 = FACES_UP | FACES_DOWN | FACES_SIDE;

/// A mesh stored in the shared buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mesh {
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
}

/// One instanced draw: an index range of the shared buffers, a material and
/// a contiguous range of the instance buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    /// Caller-chosen id (e.g. the source batch), useful in tests and debugging.
    pub tag: usize,
    pub material: usize,
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
    pub instance_count: u32,
}

#[derive(Debug, Default)]
pub struct SceneBuffers {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub instances: Vec<Instance>,
    pub materials: Vec<MaterialParams>,
    /// Sorted by pass, then material, to minimise state changes.
    pub draws: Vec<Draw>,
}

impl SceneBuffers {
    /// Triangles submitted per frame (before any culling).
    pub fn triangles_per_frame(&self) -> u64 {
        self.draws.iter().map(|d| d.index_count as u64 / 3 * d.instance_count as u64).sum()
    }
}

#[derive(Debug, Default)]
pub struct SceneBuilder {
    scene: SceneBuffers,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_material(&mut self, material: MaterialParams) -> usize {
        self.scene.materials.push(material);
        self.scene.materials.len() - 1
    }

    /// Appends a triangle mesh. Without `indices` every three vertices form a
    /// triangle. Front faces are counter-clockwise.
    pub fn add_mesh(&mut self, vertices: &[Vertex], indices: Option<&[u32]>) -> Mesh {
        let mesh = Mesh {
            first_index: self.scene.indices.len() as u32,
            index_count: indices.map_or(vertices.len(), <[u32]>::len) as u32,
            base_vertex: self.scene.vertices.len() as i32,
        };
        match indices {
            Some(indices) => {
                assert!(indices.iter().all(|&i| (i as usize) < vertices.len()), "mesh index out of range");
                self.scene.indices.extend_from_slice(indices);
            }
            None => self.scene.indices.extend(0..vertices.len() as u32),
        }
        self.scene.vertices.extend_from_slice(vertices);
        mesh
    }

    /// Appends instances and returns their range for [`Self::add_draw`].
    pub fn add_instances(&mut self, instances: &[Instance]) -> Range<u32> {
        let start = self.scene.instances.len() as u32;
        self.scene.instances.extend_from_slice(instances);
        start..self.scene.instances.len() as u32
    }

    /// Draws `indices` (relative to the mesh, defaults to all of it) of `mesh`
    /// with `material` for every instance in `instances`.
    pub fn add_draw(
        &mut self,
        tag: usize,
        mesh: Mesh,
        indices: Option<Range<u32>>,
        material: usize,
        instances: Range<u32>,
    ) {
        let indices = indices.unwrap_or(0..mesh.index_count);
        assert!(indices.end <= mesh.index_count, "draw index range outside the mesh");
        assert!(material < self.scene.materials.len(), "draw material missing");
        assert!(instances.end as usize <= self.scene.instances.len(), "draw instances missing");
        if indices.is_empty() || instances.is_empty() {
            return;
        }
        self.scene.draws.push(Draw {
            tag,
            material,
            first_index: mesh.first_index + indices.start,
            index_count: indices.len() as u32,
            base_vertex: mesh.base_vertex,
            first_instance: instances.start,
            instance_count: instances.len() as u32,
        });
    }

    pub fn finish(mut self) -> SceneBuffers {
        let materials = &self.scene.materials;
        self.scene.draws.sort_by_key(|d| (materials[d.material].pass, d.material, d.tag));
        self.scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32) -> Vertex {
        Vertex { position: [x, 0.0, 0.0], normal: [0.0, 1.0, 0.0], uv: [0.0; 2], color: [1.0; 3] }
    }

    #[test]
    fn builds_shared_buffers_and_sorted_draws() {
        let mut builder = SceneBuilder::new();
        let glass = builder.add_material(MaterialParams { pass: PassKind::Transparent, ..Default::default() });
        let wall = builder.add_material(MaterialParams::default());
        let quad = builder.add_mesh(&[vertex(0.0), vertex(1.0), vertex(2.0), vertex(3.0)], Some(&[0, 1, 2, 2, 1, 3]));
        let tri = builder.add_mesh(&[vertex(4.0), vertex(5.0), vertex(6.0)], None);
        let identity = Instance { model: glam_identity(), color: [1.0; 4] };
        let two = builder.add_instances(&[identity, identity]);
        let one = builder.add_instances(&[identity]);
        builder.add_draw(0, quad, Some(0..3), glass, two.clone());
        builder.add_draw(1, quad, Some(3..6), wall, two);
        builder.add_draw(2, tri, None, wall, one.clone());
        builder.add_draw(3, tri, Some(0..0), wall, one);
        let scene = builder.finish();

        assert_eq!(scene.indices, vec![0, 1, 2, 2, 1, 3, 0, 1, 2]);
        assert_eq!(tri, Mesh { first_index: 6, index_count: 3, base_vertex: 4 });
        // Empty draws are dropped; opaque before transparent.
        assert_eq!(scene.draws.iter().map(|d| d.tag).collect::<Vec<_>>(), vec![1, 2, 0]);
        assert_eq!(scene.draws[0].first_index, 3);
        assert_eq!(scene.triangles_per_frame(), 2 + 1 + 2);
    }

    #[test]
    #[should_panic(expected = "mesh index out of range")]
    fn rejects_bad_indices() {
        SceneBuilder::new().add_mesh(&[vertex(0.0)], Some(&[0, 1, 2]));
    }

    fn glam_identity() -> [f32; 16] {
        [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
    }
}
