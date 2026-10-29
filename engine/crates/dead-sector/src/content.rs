//! Turns the exported city into Sector Engine scene data, following the
//! material rules of `godot/scripts/bridge.gd`.

use crate::city::{City, Side};
use anyhow::{Result, bail};
use sector_assets::color::srgb_to_linear;
use sector_assets::{Image, Instance, MaterialParams, PassKind, SceneBuffers, SceneBuilder, Vertex};
use std::path::{Path, PathBuf};

/// Shading id for unlit lights and signs that go dark in a blackout
/// (`powered.gdshader`); 1..=5 are `surface.gdshader` kinds. See `surface.wgsl`.
pub const SHADING_POWERED: u32 = 16;

/// Dead Sector's surface hook for the engine's scene shader.
pub const SURFACE_HOOK: &str = include_str!("surface.wgsl");

/// File that marks a usable asset directory.
pub const CITY_FILE: &str = "city.json";

/// Finds the asset directory. Order: explicit path, `DS_ASSETS`, `assets/` next
/// to the executable, then `godot/assets` in any ancestor of the executable or
/// working directory (development checkout).
pub fn locate_assets(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        if dir.join(CITY_FILE).is_file() {
            return Ok(dir.to_path_buf());
        }
        bail!("{} does not contain {CITY_FILE}", dir.display());
    }
    if let Some(dir) = std::env::var_os("DS_ASSETS").map(PathBuf::from) {
        if dir.join(CITY_FILE).is_file() {
            return Ok(dir);
        }
        bail!("DS_ASSETS={} does not contain {CITY_FILE}", dir.display());
    }
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    if let Some(dir) = exe_dir.as_ref().map(|d| d.join("assets")).filter(|d| d.join(CITY_FILE).is_file()) {
        return Ok(dir);
    }
    let starts = exe_dir.into_iter().chain(std::env::current_dir().ok());
    for start in starts {
        for ancestor in start.ancestors() {
            let dir = ancestor.join("godot").join("assets");
            if dir.join(CITY_FILE).is_file() {
                return Ok(dir);
            }
        }
    }
    bail!("could not find {CITY_FILE}; pass --assets <dir> or set DS_ASSETS")
}

fn material(m: &crate::city::Material) -> MaterialParams {
    let linear = m.color.map(srgb_to_linear);
    let pass = if m.transparent && m.texture.is_none() {
        PassKind::Transparent
    } else if m.side == Side::Double {
        PassKind::OpaqueDoubleSided
    } else {
        PassKind::Opaque
    };
    // Godot only gives emission to plain lit materials; procedural surfaces
    // and unlit "powered" lights ignore it.
    let emits = m.surface == 0 && !m.unlit && m.energy > 0.0;
    let emission = if emits { m.emissive.map(|c| srgb_to_linear(c) * m.energy) } else { [0.0; 3] };
    let opacity = if pass == PassKind::Transparent { m.opacity } else { 1.0 };
    // bridge.gd swaps unlit opaque materials for the powered shader in the world.
    let powered = m.unlit && !m.transparent && m.surface == 0;
    MaterialParams {
        albedo: [linear[0], linear[1], linear[2], opacity],
        emission,
        unlit: m.unlit,
        alpha_test: m.texture.is_some(),
        texture: m.texture,
        pass,
        // surface.gdshader fixes roughness at 0.92 and ignores metalness.
        roughness: if m.surface > 0 { 0.92 } else { m.roughness },
        metallic: if m.surface > 0 { 0.0 } else { m.metalness },
        shading: if powered { SHADING_POWERED } else { m.surface },
        glass: m.glass,
    }
}

/// Builds the engine scene: one mesh per geometry, one instance run per batch.
pub fn build_scene(city: &City) -> SceneBuffers {
    let mut builder = SceneBuilder::new();
    for m in &city.materials {
        builder.add_material(material(m));
    }
    let meshes: Vec<_> = city
        .geometries
        .iter()
        .map(|g| {
            let vertices: Vec<Vertex> = (0..g.vertex_count())
                .map(|v| Vertex {
                    position: [g.position[v * 3], g.position[v * 3 + 1], g.position[v * 3 + 2]],
                    normal: [g.normal[v * 3], g.normal[v * 3 + 1], g.normal[v * 3 + 2]],
                    // three.js textures are flipped on upload (flipY), so v=0 is the image bottom.
                    uv: if g.uv.is_empty() { [0.0; 2] } else { [g.uv[v * 2], 1.0 - g.uv[v * 2 + 1]] },
                    color: if g.color.is_empty() {
                        [1.0; 3]
                    } else {
                        [g.color[v * 3], g.color[v * 3 + 1], g.color[v * 3 + 2]]
                    },
                })
                .collect();
            // three.js and wgpu both treat counter-clockwise triangles as front
            // faces, so unlike bridge.gd the winding is kept as authored.
            builder.add_mesh(&vertices, (!g.index.is_empty()).then_some(g.index.as_slice()))
        })
        .collect();
    for (batch_index, b) in city.batches.iter().enumerate() {
        let instances: Vec<Instance> = b
            .transforms
            .iter()
            .enumerate()
            .map(|(i, model)| Instance {
                model: *model,
                color: if b.colors.is_empty() {
                    [1.0; 4]
                } else {
                    [b.colors[i * 3], b.colors[i * 3 + 1], b.colors[i * 3 + 2], 1.0]
                },
            })
            .collect();
        let range = builder.add_instances(&instances);
        let g = &city.geometries[b.geometry];
        let mesh = meshes[b.geometry];
        // Same rule as bridge.gd: material groups only apply to multi-material batches.
        if b.materials.len() > 1 && !g.groups.is_empty() {
            for group in &g.groups {
                let indices = group.start..group.start + group.count;
                builder.add_draw(
                    batch_index,
                    mesh,
                    Some(indices),
                    b.materials[group.material_index as usize],
                    range.clone(),
                );
            }
        } else {
            builder.add_draw(batch_index, mesh, None, b.materials[0], range);
        }
    }
    builder.finish()
}

/// Loads `label-<n>.png` for every texture index the materials use, in index order.
pub fn load_textures(city: &City, assets: &Path) -> Result<Vec<Image>> {
    let count = city.materials.iter().filter_map(|m| m.texture).max().map_or(0, |max| max + 1);
    (0..count).map(|i| sector_assets::load_image(&assets.join(format!("label-{i}.png")))).collect()
}
