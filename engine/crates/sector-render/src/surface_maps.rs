//! Tiling PBR material maps on the GPU: three 2D texture arrays (colour,
//! normal, AO/roughness) with one layer per material pack, shared by every
//! material bind group, plus an anisotropic repeating sampler. Packs stay
//! block-compressed when the adapter supports BC formats; otherwise they are
//! decoded on the CPU at a capped size.

use anyhow::{Result, ensure};
use sector_assets::materials::{Map, MapFormat, MaterialPack};

/// Largest map side used when maps must be decoded (no BC support).
const DECODED_MAX: u32 = 1024;

pub(crate) struct SurfaceMaps {
    pub albedo: wgpu::TextureView,
    pub normal: wgpu::TextureView,
    pub orm: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    /// Map side actually uploaded (colour/normal) and whether it is compressed.
    pub size: u32,
    pub compressed: bool,
}

impl SurfaceMaps {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, packs: &[MaterialPack], max_size: u32) -> Result<Self> {
        let compressed = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        let cap = if compressed { max_size } else { max_size.min(DECODED_MAX) }.max(4);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("surface-maps"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        if packs.is_empty() {
            // One neutral layer so the bindings always exist.
            let one = |label, rgba: [u8; 4], srgb| {
                let format = if srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
                array(device, queue, label, format, 1, 1, &[vec![rgba.to_vec()]])
            };
            return Ok(SurfaceMaps {
                albedo: one("surface-albedo", [255; 4], true),
                normal: one("surface-normal", [128, 128, 255, 255], false),
                orm: one("surface-orm", [255, 255, 0, 255], false),
                sampler,
                size: 1,
                compressed: false,
            });
        }
        let upload = |label: &str, pick: fn(&MaterialPack) -> &Map| -> Result<(wgpu::TextureView, u32)> {
            let first = pick(&packs[0]);
            for p in packs {
                let m = pick(p);
                ensure!(
                    m.size == first.size && m.format == first.format,
                    "{}: {label} differs from {}",
                    p.name,
                    packs[0].name
                );
            }
            // Drop top mips until the map fits the cap.
            let skip = (0..first.levels.len()).find(|&l| first.level_size(l) <= cap).unwrap_or(first.levels.len() - 1);
            let levels = first.levels.len() - skip;
            let size = first.level_size(skip);
            let format = match (first.format, compressed) {
                (MapFormat::Bc1Srgb, true) => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
                (MapFormat::Bc5, true) => wgpu::TextureFormat::Bc5RgUnorm,
                (MapFormat::Bc1Srgb, false) => wgpu::TextureFormat::Rgba8UnormSrgb,
                (MapFormat::Bc5, false) => wgpu::TextureFormat::Rgba8Unorm,
            };
            let data: Vec<Vec<Vec<u8>>> = packs
                .iter()
                .map(|p| {
                    let m = pick(p);
                    (skip..m.levels.len()).map(|l| if compressed { m.levels[l].clone() } else { m.decode(l) }).collect()
                })
                .collect();
            Ok((array(device, queue, label, format, size, levels as u32, &data), size))
        };
        let (albedo, size) = upload("surface-albedo", |p| &p.albedo)?;
        let (normal, _) = upload("surface-normal", |p| &p.normal)?;
        let (orm, _) = upload("surface-orm", |p| &p.orm)?;
        Ok(SurfaceMaps { albedo, normal, orm, sampler, size, compressed })
    }
}

/// A 2D array texture with `data[layer][level]` already in `format`'s layout.
fn array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    format: wgpu::TextureFormat,
    size: u32,
    levels: u32,
    data: &[Vec<Vec<u8>>],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: data.len() as u32 },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (block, block_bytes) = match format.block_dimensions() {
        (1, 1) => (1, 4),
        (b, _) => (b, format.block_copy_size(None).unwrap()),
    };
    for (layer, mips) in data.iter().enumerate() {
        for (level, bytes) in mips.iter().enumerate() {
            let s = (size >> level).max(1);
            // Compressed levels are copied as whole blocks (physical size).
            let physical = s.div_ceil(block) * block;
            let blocks = physical / block;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(blocks * block_bytes),
                    rows_per_image: Some(blocks),
                },
                wgpu::Extent3d { width: physical, height: physical, depth_or_array_layers: 1 },
            );
        }
    }
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}
