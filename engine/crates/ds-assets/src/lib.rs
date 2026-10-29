//! Asset loading for the Dead Sector Rust engine.

pub mod city;
pub mod scene;

pub use city::City;
pub use scene::SceneBuffers;

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

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

/// Decoded RGBA8 image (sRGB).
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Loads `label-<index>.png`, the sign/label textures baked by the exporter.
pub fn load_label_texture(assets: &Path, index: u32) -> Result<Image> {
    let path = assets.join(format!("label-{index}.png"));
    let image = image::open(&path).with_context(|| format!("loading {}", path.display()))?.to_rgba8();
    Ok(Image { width: image.width(), height: image.height(), rgba: image.into_raw() })
}
