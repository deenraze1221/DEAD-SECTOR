//! Typed view of `city.json`, the scene exported from the three.js build by
//! `scripts/export-godot.mjs`. Only the parts the renderer needs are read;
//! everything else in the file (creatures, shop items, …) is ignored for now.

use anyhow::{Context, Result, bail};
use sector_assets::color::parse_hex_color;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct City {
    pub version: u32,
    pub seed: u64,
    pub spawn: Spawn,
    pub geometries: Vec<Geometry>,
    pub materials: Vec<Material>,
    pub batches: Vec<Batch>,
    pub colliders: Vec<Collider>,
    /// One main building per 24 m plot (10 × 10 grid).
    pub buildings: Vec<Building>,
    /// Every placed asset (buildings, props, furniture) with its type name.
    pub source_assets: Vec<SourceAsset>,
    /// World bounds of each source asset, keyed by its id.
    #[serde(default)]
    pub source_bounds: BTreeMap<String, Bounds>,
    /// The SECTOR MART store terminal.
    pub terminal: Terminal,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Building {
    #[serde(default)]
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    /// Footprint centre and size.
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub d: f32,
    pub h: f32,
    /// Plot centre.
    pub cx: f32,
    pub cz: f32,
    pub district: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SourceAsset {
    pub id: Owner,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    pub z: f32,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub w: f32,
    #[serde(default)]
    pub h: f32,
    #[serde(default)]
    pub d: f32,
    /// The building an interior item belongs to.
    #[serde(default)]
    pub building: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Id of the asset an instance or collider belongs to: a building number or a
/// string such as `interior-3-1` or `asset-12`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(untagged)]
pub enum Owner {
    Int(i64),
    Str(String),
}

impl Owner {
    /// The id as Godot's `str()` writes it, which is also the `source_bounds` key.
    pub fn key(&self) -> String {
        match self {
            Owner::Int(n) => n.to_string(),
            Owner::Str(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Terminal {
    pub x: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Spawn {
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    pub z: f32,
}

/// three.js `BufferGeometry`: flat attribute arrays, optional index.
#[derive(Debug, Clone, Deserialize)]
pub struct Geometry {
    pub position: Vec<f32>,
    #[serde(default)]
    pub normal: Vec<f32>,
    #[serde(default)]
    pub uv: Vec<f32>,
    /// Linear RGB vertex colours (three.js working colour space).
    #[serde(default)]
    pub color: Vec<f32>,
    #[serde(default)]
    pub index: Vec<u32>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl Geometry {
    pub fn vertex_count(&self) -> usize {
        self.position.len() / 3
    }

    /// Number of indices drawn: the index buffer, or every vertex when non-indexed.
    pub fn index_count(&self) -> usize {
        if self.index.is_empty() { self.vertex_count() } else { self.index.len() }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Group {
    pub start: u32,
    pub count: u32,
    #[serde(rename = "materialIndex")]
    pub material_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Front,
    Back,
    Double,
}

/// three.js `MeshStandardMaterial` / `MeshBasicMaterial` parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct Material {
    pub opacity: f32,
    pub transparent: bool,
    #[serde(deserialize_with = "side")]
    pub side: Side,
    /// sRGB colour as exported by `Color.getHexString()`.
    #[serde(deserialize_with = "hex_color")]
    pub color: [f32; 3],
    #[serde(deserialize_with = "hex_color")]
    pub emissive: [f32; 3],
    pub energy: f32,
    pub roughness: f32,
    pub metalness: f32,
    /// `MeshBasicMaterial`: no lighting.
    pub unlit: bool,
    /// Index of `label-<n>.png`, if any.
    #[serde(deserialize_with = "texture_index")]
    pub texture: Option<u32>,
    /// Procedural city surface kind (0 = plain, 1 brick, 3 tile, 4 asphalt, 5 wood …).
    pub surface: u32,
    /// Window glass (generated cities only; never in the export).
    #[serde(default)]
    pub glass: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Batch {
    pub name: String,
    pub geometry: usize,
    pub materials: Vec<usize>,
    /// Column-major 4×4 world matrices (three.js `Matrix4.elements`).
    pub transforms: Vec<[f32; 16]>,
    /// Optional linear RGB colour per instance.
    #[serde(default)]
    pub colors: Vec<f32>,
    /// Owning asset per instance (may be shorter than `transforms`).
    #[serde(default)]
    pub owners: Vec<Option<Owner>>,
    /// `model`, `partition` or `fresh` per instance (may be shorter than `transforms`).
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Collider {
    #[serde(rename = "minX")]
    pub min_x: f32,
    #[serde(rename = "maxX")]
    pub max_x: f32,
    #[serde(rename = "minY")]
    pub min_y: f32,
    #[serde(rename = "maxY")]
    pub max_y: f32,
    #[serde(rename = "minZ")]
    pub min_z: f32,
    #[serde(rename = "maxZ")]
    pub max_z: f32,
    /// `structure`, `wall`, `prop`, `furniture`, `partition`, …
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub owner: Option<Owner>,
}

impl City {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let city = Self::parse(&bytes).with_context(|| format!("parsing {}", path.display()))?;
        Ok(city)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let city: City = serde_json::from_slice(bytes)?;
        city.validate()?;
        Ok(city)
    }

    /// Ground positions of all street lamps.
    pub fn lamps(&self) -> impl Iterator<Item = [f32; 3]> + '_ {
        self.source_assets.iter().filter(|a| a.kind == "lamp").map(|a| [a.x, a.y, a.z])
    }

    pub fn instance_count(&self) -> usize {
        self.batches.iter().map(|b| b.transforms.len()).sum()
    }

    /// Rejects data the renderer cannot draw correctly, listing the first problems found.
    pub fn validate(&self) -> Result<()> {
        let mut problems = Vec::new();
        for (i, g) in self.geometries.iter().enumerate() {
            let n = g.vertex_count();
            if g.position.len() % 3 != 0 || n == 0 {
                problems.push(format!("geometry {i}: position length {}", g.position.len()));
            }
            if g.normal.len() != n * 3 {
                problems.push(format!("geometry {i}: {} normal floats for {n} vertices", g.normal.len()));
            }
            if !g.uv.is_empty() && g.uv.len() != n * 2 {
                problems.push(format!("geometry {i}: {} uv floats for {n} vertices", g.uv.len()));
            }
            if !g.color.is_empty() && g.color.len() != n * 3 {
                problems.push(format!("geometry {i}: {} colour floats for {n} vertices", g.color.len()));
            }
            if g.index_count() % 3 != 0 {
                problems.push(format!("geometry {i}: {} indices is not whole triangles", g.index_count()));
            }
            if let Some(bad) = g.index.iter().find(|&&v| v as usize >= n) {
                problems.push(format!("geometry {i}: index {bad} out of range ({n} vertices)"));
            }
            for group in &g.groups {
                if (group.start + group.count) as usize > g.index_count() {
                    problems.push(format!("geometry {i}: group {group:?} exceeds index range"));
                }
            }
        }
        for (i, b) in self.batches.iter().enumerate() {
            let Some(g) = self.geometries.get(b.geometry) else {
                problems.push(format!("batch {i} ({}): geometry {} missing", b.name, b.geometry));
                continue;
            };
            if b.materials.is_empty() {
                problems.push(format!("batch {i} ({}): no materials", b.name));
            }
            if let Some(m) = b.materials.iter().find(|&&m| m >= self.materials.len()) {
                problems.push(format!("batch {i} ({}): material {m} missing", b.name));
            }
            if b.materials.len() > 1
                && let Some(group) = g.groups.iter().find(|gr| gr.material_index as usize >= b.materials.len())
            {
                problems.push(format!("batch {i} ({}): group material {} missing", b.name, group.material_index));
            }
            if !b.colors.is_empty() && b.colors.len() != b.transforms.len() * 3 {
                problems.push(format!(
                    "batch {i} ({}): {} colour floats for {} instances",
                    b.name,
                    b.colors.len(),
                    b.transforms.len()
                ));
            }
        }
        for (i, m) in self.materials.iter().enumerate() {
            if m.side == Side::Back {
                problems.push(format!("material {i}: back-side-only materials are not supported"));
            }
        }
        if problems.is_empty() {
            return Ok(());
        }
        let shown = problems.iter().take(10).cloned().collect::<Vec<_>>().join("\n  ");
        bail!("city data has {} problem(s):\n  {shown}", problems.len())
    }
}

fn hex_color<'de, D: Deserializer<'de>>(d: D) -> Result<[f32; 3], D::Error> {
    let text = String::deserialize(d)?;
    parse_hex_color(&text).ok_or_else(|| serde::de::Error::custom(format!("bad colour {text:?}")))
}

fn side<'de, D: Deserializer<'de>>(d: D) -> Result<Side, D::Error> {
    match u8::deserialize(d)? {
        0 => Ok(Side::Front),
        1 => Ok(Side::Back),
        2 => Ok(Side::Double),
        other => Err(serde::de::Error::custom(format!("bad material side {other}"))),
    }
}

fn texture_index<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    let value = i64::deserialize(d)?;
    Ok(u32::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{SHADING_POWERED, build_scene};
    use sector_assets::PassKind;

    const TINY: &str = r##"{
      "version": 1, "seed": 7, "spawn": {"x": 1, "z": 2},
      "geometries": [{
        "position": [0,0,0, 1,0,0, 0,1,0, 1,1,0], "normal": [0,0,1, 0,0,1, 0,0,1, 0,0,1],
        "uv": [0,0, 1,0, 0,1, 1,1], "color": [], "index": [0,1,2, 2,1,3],
        "groups": [{"start": 0, "count": 3, "materialIndex": 0}, {"start": 3, "count": 3, "materialIndex": 1}]
      }],
      "materials": [
        {"opacity": 1, "transparent": false, "side": 0, "color": "#ffffff", "emissive": "#000000", "energy": 0,
         "roughness": 1, "metalness": 0, "unlit": false, "texture": -1, "surface": 0},
        {"opacity": 0.3, "transparent": true, "side": 2, "color": "#808080", "emissive": "#ff0000", "energy": 2,
         "roughness": 1, "metalness": 0, "unlit": true, "texture": -1, "surface": 0}
      ],
      "batches": [
        {"name": "single", "geometry": 0, "materials": [0], "transforms": [[1,0,0,0, 0,1,0,0, 0,0,1,0, 5,0,0,1]], "colors": []},
        {"name": "grouped", "geometry": 0, "materials": [1, 0],
         "transforms": [[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1], [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,3,0,1]], "colors": [1,0,0, 0,1,0]}
      ],
      "colliders": [{"minX": 0, "maxX": 1, "minY": 0, "maxY": 1, "minZ": 0, "maxZ": 1, "kind": "structure"}],
      "buildings": [{"id": 0, "type": "shop", "x": 1, "z": 2, "w": 3, "d": 4, "h": 5, "cx": 0, "cz": 0, "district": "market"}],
      "source_assets": [{"id": "asset-1", "type": "lamp", "x": 4, "y": 0, "z": 5, "yaw": 0, "w": 0.3, "h": 5, "d": 0.3},
                        {"id": 2, "type": "bench", "x": 6, "z": 7, "yaw": 0, "w": 1, "h": 1, "d": 1}],
      "terminal": {"x": 7.25, "z": 10.9},
      "creatures": {}
    }"##;

    #[test]
    fn tiny_city_builds_grouped_draws() {
        let city = City::parse(TINY.as_bytes()).unwrap();
        assert_eq!(city.spawn.y, 0.0);
        assert_eq!(city.lamps().collect::<Vec<_>>(), vec![[4.0, 0.0, 5.0]]);
        assert_eq!(city.buildings[0].district, "market");
        let scene = build_scene(&city);
        assert_eq!(scene.vertices.len(), 4);
        // v is flipped for texture sampling.
        assert_eq!(scene.vertices[2].uv, [0.0, 0.0]);
        assert_eq!(scene.vertices[0].uv, [0.0, 1.0]);
        assert_eq!(scene.vertices[0].color, [1.0; 3]);
        // One plain draw + two group draws (materials remapped through the batch list).
        assert_eq!(scene.draws.len(), 3);
        let grouped: Vec<_> =
            scene.draws.iter().filter(|d| d.tag == 1).map(|d| (d.material, d.first_index, d.index_count)).collect();
        assert!(grouped.contains(&(1, 0, 3)) && grouped.contains(&(0, 3, 3)));
        // Opaque draws sort before the transparent one.
        assert_eq!(scene.materials[scene.draws.last().unwrap().material].pass, PassKind::Transparent);
        assert_eq!(scene.instances[2].color, [0.0, 1.0, 0.0, 1.0]);
        // Unlit materials get no emission, matching bridge.gd's powered/unshaded split.
        assert_eq!(scene.materials[1].emission, [0.0; 3]);
        assert_eq!(scene.materials[1].albedo[3], 0.3);
        // Transparent unlit materials stay plain unshaded; only opaque unlit ones are powered.
        assert!(scene.materials.iter().all(|m| m.shading != SHADING_POWERED));
        assert_eq!((scene.materials[0].roughness, scene.materials[0].metallic), (1.0, 0.0));
        assert_eq!(scene.triangles_per_frame(), 2 + 2 * 2);
    }

    #[test]
    fn rejects_out_of_range_index() {
        let broken = TINY.replace("[0,1,2, 2,1,3]", "[0,1,2, 2,1,9]");
        let error = City::parse(broken.as_bytes()).unwrap_err().to_string();
        assert!(error.contains("index 9 out of range"), "{error}");
    }

    #[test]
    fn rejects_missing_material() {
        let broken = TINY.replace(r#""materials": [0]"#, r#""materials": [5]"#);
        let error = City::parse(broken.as_bytes()).unwrap_err().to_string();
        assert!(error.contains("material 5 missing"), "{error}");
    }
}
