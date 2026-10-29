//! Converts PBR texture folders into Sector Engine material packs (`.smat`).
//!
//!   cargo run --release -p dead-sector --example import_materials -- <source> <output> [orm-size]
//!
//! `<source>` holds one folder per material, `Name/Name.png` (colour) plus
//! `Name_normal.png` (DirectX convention), `Name_ambient.png` (occlusion) and
//! `Name_roughness.png` or `Name_specular.png` (converted to roughness).
//! Height and displacement maps are not packed. Each folder becomes
//! `<output>/Name.smat`; add it to `library.json` to use it.

use anyhow::{Context, Result, bail};
use image::imageops::FilterType;
use sector_assets::materials::{MaterialPack, SourceMaps};
use std::path::Path;

/// Specular (0 = matte .. 1 = shiny) to roughness.
fn roughness(specular: u8) -> u8 {
    let s = specular as f32 / 255.0;
    ((0.95 - 0.45 * s) * 255.0).round() as u8
}

fn load(path: &Path) -> Result<image::RgbaImage> {
    Ok(image::open(path).with_context(|| format!("reading {}", path.display()))?.to_rgba8())
}

fn convert(dir: &Path, name: &str, out: &Path, orm_size: u32) -> Result<()> {
    let albedo = load(&dir.join(format!("{name}.png")))?;
    let normal = load(&dir.join(format!("{name}_normal.png")))?;
    let size = albedo.width();
    if albedo.height() != size || !size.is_power_of_two() || normal.dimensions() != albedo.dimensions() {
        bail!("{name}: colour and normal maps must be the same power-of-two square");
    }
    let ambient = image::imageops::resize(
        &load(&dir.join(format!("{name}_ambient.png")))?,
        orm_size,
        orm_size,
        FilterType::Triangle,
    );
    // Roughness directly when given, else converted from specular.
    let rough_path = dir.join(format!("{name}_roughness.png"));
    let (gloss_path, convert) =
        if rough_path.is_file() { (rough_path, false) } else { (dir.join(format!("{name}_specular.png")), true) };
    let gloss = image::imageops::resize(&load(&gloss_path)?, orm_size, orm_size, FilterType::Triangle);
    let orm: Vec<u8> = ambient
        .pixels()
        .zip(gloss.pixels())
        .flat_map(|(a, g)| [a[0], if convert { roughness(g[0]) } else { g[0] }, 0, 255])
        .collect();
    let maps = SourceMaps { size, albedo: albedo.as_raw(), normal: normal.as_raw(), orm: &orm, orm_size };
    let pack = MaterialPack::encode(name, &maps)?;
    let path = out.join(format!("{name}.smat"));
    std::fs::write(&path, pack.to_bytes()).with_context(|| format!("writing {}", path.display()))?;
    println!("{name}: {size}² colour/normal, {orm_size}² AO/roughness → {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, out, rest @ ..] = args.as_slice() else {
        bail!("usage: import_materials <source> <output> [orm-size]");
    };
    let orm_size: u32 = rest.first().map(|s| s.parse()).transpose()?.unwrap_or(1024);
    std::fs::create_dir_all(out)?;
    let mut names: Vec<String> = std::fs::read_dir(source)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        convert(&Path::new(source).join(&name), &name, Path::new(out), orm_size)?;
    }
    Ok(())
}
