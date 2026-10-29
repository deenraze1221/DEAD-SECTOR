//! Sector Engine asset types: GPU-ready scene data, images and colour helpers.
//! Game-specific file formats live in the game crates and convert into these.

pub mod color;
pub mod materials;
pub mod modular;
pub mod rng;
pub mod scene;

pub use scene::{Instance, MaterialParams, Mesh, PassKind, SceneBuffers, SceneBuilder, Vertex};

use anyhow::{Context, Result};
use std::path::Path;

/// Decoded RGBA8 image (sRGB).
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Loads any image the `image` crate can decode (PNG is enabled).
pub fn load_image(path: &Path) -> Result<Image> {
    let image = image::open(path).with_context(|| format!("loading {}", path.display()))?.to_rgba8();
    Ok(Image { width: image.width(), height: image.height(), rgba: image.into_raw() })
}
