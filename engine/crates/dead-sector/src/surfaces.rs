//! Dead Sector's surface material library: which tiling material packs exist
//! (`library.json` in the materials folder) and which city materials use them.
//! Everything is data: new packs are added by importing them and editing the
//! library, without code changes.

use crate::city::City;
use anyhow::{Context, Result, bail};
use sector_assets::SceneBuffers;
use sector_assets::color::{parse_hex_color, srgb_to_linear};
use sector_assets::materials::MaterialPack;
use sector_assets::scene::{FACES_ALL, FACES_DOWN, FACES_SIDE, FACES_UP};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const LIBRARY_FILE: &str = "library.json";
/// How strongly the city's palette colour tints a texture unless the library says otherwise.
const DEFAULT_TINT: f32 = 0.35;

#[derive(Debug, Deserialize)]
struct LibraryFile {
    materials: BTreeMap<String, PackEntry>,
    surfaces: Vec<Assignment>,
}

#[derive(Debug, Deserialize)]
struct PackEntry {
    file: String,
    /// Metres per repeat.
    tile: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct Assignment {
    /// Exported material colour (`#rrggbb`).
    #[serde(default)]
    color: Option<String>,
    /// Generated material name.
    #[serde(default)]
    name: Option<String>,
    material: String,
    /// Overrides the pack's tile size.
    #[serde(default)]
    tile: Option<f32>,
    #[serde(default)]
    tint: Option<f32>,
    /// Faces that get the map: any of "up", "down", "sides" (default: all).
    #[serde(default)]
    faces: Option<Vec<String>>,
}

/// Loaded packs plus the rules that assign them.
#[derive(Debug, Default)]
pub struct SurfaceLibrary {
    /// Packs in layer order (only those some surface uses).
    pub packs: Vec<MaterialPack>,
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    color: Option<[f32; 3]>,
    name: Option<String>,
    layer: u32,
    tile: f32,
    tint: f32,
    faces: u32,
}

impl SurfaceLibrary {
    /// Reads a library file (`library.json`, or `library-<set>.json` for a
    /// named set) and the packs it assigns from `dir`.
    pub fn load(dir: &Path, set: Option<&str>) -> Result<SurfaceLibrary> {
        let path = dir.join(set.map_or(LIBRARY_FILE.to_string(), |s| format!("library-{s}.json")));
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let file: LibraryFile = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let mut library = SurfaceLibrary::default();
        let mut layers: BTreeMap<String, (u32, f32)> = BTreeMap::new();
        for a in &file.surfaces {
            let Some(entry) = file.materials.get(&a.material) else {
                bail!("{}: surface uses unknown material {:?}", path.display(), a.material);
            };
            if !layers.contains_key(&a.material) {
                let pack = MaterialPack::load(&dir.join(&entry.file))?;
                layers.insert(a.material.clone(), (library.packs.len() as u32, entry.tile));
                library.packs.push(pack);
            }
            let (layer, tile) = layers[&a.material];
            let color = match &a.color {
                Some(hex) => Some(parse_hex_color(hex).with_context(|| format!("bad colour {hex:?}"))?),
                None => None,
            };
            if color.is_none() && a.name.is_none() {
                bail!("{}: a surface for {:?} has neither colour nor name", path.display(), a.material);
            }
            let faces = match &a.faces {
                None => FACES_ALL,
                Some(list) => list.iter().try_fold(0, |mask, f| {
                    Ok(mask
                        | match f.as_str() {
                            "up" => FACES_UP,
                            "down" => FACES_DOWN,
                            "sides" => FACES_SIDE,
                            other => bail!("{}: unknown face {other:?}", path.display()),
                        })
                })?,
            };
            library.rules.push(Rule {
                faces,
                color,
                name: a.name.clone(),
                layer,
                tile: a.tile.unwrap_or(tile),
                tint: a.tint.unwrap_or(DEFAULT_TINT).clamp(0.0, 1.0),
            });
        }
        Ok(library)
    }

    /// Gives every matching scene material its surface map, tile size and
    /// tint. `scene.materials[i]` must be built from `city.materials[i]`.
    /// Returns how many materials were assigned.
    pub fn apply(&self, city: &City, scene: &mut SceneBuffers) -> usize {
        let mut assigned = 0;
        for (i, m) in city.materials.iter().enumerate() {
            if m.texture.is_some() || m.transparent {
                continue;
            }
            let rule = self.rules.iter().find(|r| match (&r.name, &m.name) {
                (Some(want), Some(have)) => want == have,
                (Some(_), None) => false,
                (None, _) => m.name.is_none() && r.color == Some(m.color),
            });
            let Some(rule) = rule else { continue };
            let pack = &self.packs[rule.layer as usize];
            let params = &mut scene.materials[i];
            // Multiplier on the texture: 1 keeps it as authored; `tint` pulls
            // its average towards the city's colour for this surface.
            let linear = m.color.map(srgb_to_linear);
            for ((out, colour), mean) in params.albedo.iter_mut().zip(linear).zip(pack.mean_albedo) {
                let towards = colour / mean.max(1e-3);
                *out = 1.0 + (towards - 1.0) * rule.tint;
            }
            params.surface_map = Some(rule.layer);
            params.tiling = rule.tile;
            params.surface_faces = rule.faces;
            assigned += 1;
        }
        assigned
    }
}

/// The materials folder: `DS_MATERIALS`, `materials/` next to the executable,
/// or `engine/assets/materials` above the city assets or the executable.
pub fn locate(assets: &Path) -> Option<PathBuf> {
    let has = |d: &Path| d.join(LIBRARY_FILE).is_file();
    if let Some(dir) = std::env::var_os("DS_MATERIALS").map(PathBuf::from) {
        return has(&dir).then_some(dir);
    }
    let exe = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    if let Some(dir) = exe.as_ref().map(|e| e.join("materials")).filter(|d| has(d)) {
        return Some(dir);
    }
    let starts = [Some(assets.to_path_buf()), exe, std::env::current_dir().ok()];
    starts.into_iter().flatten().flat_map(|s| s.ancestors().map(Path::to_path_buf).collect::<Vec<_>>()).find_map(|a| {
        let dir = a.join("engine").join("assets").join("materials");
        has(&dir).then_some(dir)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_file_parses_and_rejects_unknown_materials() {
        let ok: LibraryFile = serde_json::from_str(
            r##"{"materials": {"A": {"file": "A.smat", "tile": 2}}, "surfaces": [{"color": "#112233", "material": "A"}]}"##,
        )
        .unwrap();
        assert_eq!(ok.materials["A"].tile, 2.0);
        let dir = std::env::temp_dir().join(format!("ds-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(LIBRARY_FILE), r#"{"materials": {}, "surfaces": [{"name": "x", "material": "Nope"}]}"#)
            .unwrap();
        let error = SurfaceLibrary::load(&dir, None).unwrap_err().to_string();
        assert!(error.contains("unknown material"), "{error}");
        std::fs::remove_dir_all(dir).ok();
    }
}
