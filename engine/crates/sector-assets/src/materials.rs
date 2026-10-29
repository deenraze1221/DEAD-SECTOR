//! Surface material packs: PBR maps for one tiling material (colour, tangent
//! normal, ambient occlusion + roughness), each with a full mip chain in a GPU
//! block-compressed format, stored in a small `.smat` file. Any number of
//! packs can be loaded; nothing in the engine knows their names.
//!
//! Normal maps keep the source's DirectX convention: +X along increasing u,
//! +Y along increasing v (down the image), +Z out of the surface.

use anyhow::{Context, Result, bail, ensure};
use texpresso::{Format, Params};

/// Block-compressed map formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapFormat {
    /// RGB colour, sRGB-encoded (BC1, 8 bytes per 4×4 block).
    Bc1Srgb,
    /// Two linear channels (BC5, 16 bytes per 4×4 block).
    Bc5,
}

impl MapFormat {
    fn codec(self) -> Format {
        match self {
            MapFormat::Bc1Srgb => Format::Bc1,
            MapFormat::Bc5 => Format::Bc5,
        }
    }

    fn id(self) -> u8 {
        match self {
            MapFormat::Bc1Srgb => 1,
            MapFormat::Bc5 => 2,
        }
    }

    fn from_id(id: u8) -> Result<Self> {
        Ok(match id {
            1 => MapFormat::Bc1Srgb,
            2 => MapFormat::Bc5,
            other => bail!("unknown map format {other}"),
        })
    }

    pub fn block_bytes(self) -> u32 {
        match self {
            MapFormat::Bc1Srgb => 8,
            MapFormat::Bc5 => 16,
        }
    }
}

/// One square map with its mip chain (level 0 = `size` × `size`).
#[derive(Clone, PartialEq)]
pub struct Map {
    pub format: MapFormat,
    pub size: u32,
    pub levels: Vec<Vec<u8>>,
}

impl std::fmt::Debug for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Map({:?}, {}², {} levels)", self.format, self.size, self.levels.len())
    }
}

impl Map {
    pub fn level_size(&self, level: usize) -> u32 {
        (self.size >> level).max(1)
    }

    /// Compresses an RGBA8 image and its mips (built by `downsample`).
    fn encode(format: MapFormat, size: u32, mut rgba: Vec<u8>, downsample: fn(&[u8], u32) -> Vec<u8>) -> Map {
        let mut levels = Vec::new();
        let mut s = size;
        loop {
            let codec = format.codec();
            let mut out = vec![0u8; codec.compressed_size(s as usize, s as usize)];
            codec.compress(&rgba, s as usize, s as usize, Params::default(), &mut out);
            levels.push(out);
            if s == 1 {
                break;
            }
            rgba = downsample(&rgba, s);
            s /= 2;
        }
        Map { format, size, levels }
    }

    /// Decodes one level to RGBA8 (for adapters without block compression).
    pub fn decode(&self, level: usize) -> Vec<u8> {
        let s = self.level_size(level) as usize;
        // texpresso decodes whole 4×4 blocks; small levels are padded.
        let padded = s.div_ceil(4) * 4;
        let mut out = vec![0u8; padded * padded * 4];
        self.format.codec().decompress(&self.levels[level], padded, padded, &mut out);
        if padded == s {
            return out;
        }
        out.chunks(padded * 4).take(s).flat_map(|row| row[..s * 4].to_vec()).collect()
    }
}

/// One tiling material: colour, normal and AO/roughness maps.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialPack {
    pub name: String,
    /// Average linear colour of the colour map, for tinting.
    pub mean_albedo: [f32; 3],
    /// sRGB colour (BC1).
    pub albedo: Map,
    /// Tangent normal X, Y (BC5); Z is rebuilt in the shader.
    pub normal: Map,
    /// R = ambient occlusion, G = roughness (BC5).
    pub orm: Map,
}

const MAGIC: &[u8; 4] = b"SMAT";
const VERSION: u32 = 1;

/// Source maps for [`MaterialPack::encode`], all square RGBA8.
pub struct SourceMaps<'a> {
    pub size: u32,
    pub albedo: &'a [u8],
    /// Tangent normal, 0..255 encoding −1..1, DirectX convention.
    pub normal: &'a [u8],
    /// R = ambient occlusion, G = roughness.
    pub orm: &'a [u8],
    /// Size of the AO/roughness map (it may be smaller than the others).
    pub orm_size: u32,
}

impl MaterialPack {
    /// Builds mip chains and compresses them.
    pub fn encode(name: &str, maps: &SourceMaps) -> Result<MaterialPack> {
        ensure!(maps.size.is_power_of_two() && maps.orm_size.is_power_of_two(), "map sizes must be powers of two");
        let n = (maps.size * maps.size * 4) as usize;
        ensure!(maps.albedo.len() == n && maps.normal.len() == n, "colour/normal must be {0}×{0} RGBA", maps.size);
        ensure!(maps.orm.len() == (maps.orm_size * maps.orm_size * 4) as usize, "bad AO/roughness size");
        let mut sum = [0.0f64; 3];
        for px in maps.albedo.chunks_exact(4) {
            for c in 0..3 {
                sum[c] += srgb_to_linear(px[c]) as f64;
            }
        }
        let pixels = (maps.size * maps.size) as f64;
        Ok(MaterialPack {
            name: name.to_string(),
            mean_albedo: sum.map(|s| (s / pixels) as f32),
            albedo: Map::encode(MapFormat::Bc1Srgb, maps.size, maps.albedo.to_vec(), downsample_srgb),
            normal: Map::encode(MapFormat::Bc5, maps.size, maps.normal.to_vec(), downsample_normal),
            orm: Map::encode(MapFormat::Bc5, maps.orm_size, maps.orm.to_vec(), downsample_linear),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.name.len() as u32).to_le_bytes());
        out.extend_from_slice(self.name.as_bytes());
        for c in self.mean_albedo {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for map in [&self.albedo, &self.normal, &self.orm] {
            out.push(map.format.id());
            out.extend_from_slice(&map.size.to_le_bytes());
            out.extend_from_slice(&(map.levels.len() as u32).to_le_bytes());
            for level in &map.levels {
                out.extend_from_slice(&(level.len() as u32).to_le_bytes());
                out.extend_from_slice(level);
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<MaterialPack> {
        let mut r = Reader { bytes, at: 0 };
        ensure!(r.take(4)? == MAGIC, "not a material pack");
        ensure!(r.u32()? == VERSION, "unsupported material pack version");
        let len = r.u32()? as usize;
        let name = String::from_utf8(r.take(len)?.to_vec()).context("pack name")?;
        let mean_albedo = [r.f32()?, r.f32()?, r.f32()?];
        let mut map = || -> Result<Map> {
            let format = MapFormat::from_id(r.take(1)?[0])?;
            let size = r.u32()?;
            let count = r.u32()? as usize;
            ensure!(size.is_power_of_two() && count == size.ilog2() as usize + 1, "bad mip chain");
            let mut levels = Vec::with_capacity(count);
            for level in 0..count {
                let s = (size >> level).max(1) as usize;
                let expected = format.codec().compressed_size(s, s);
                let len = r.u32()? as usize;
                ensure!(len == expected, "level {level}: {len} bytes, expected {expected}");
                levels.push(r.take(len)?.to_vec());
            }
            Ok(Map { format, size, levels })
        };
        let (albedo, normal, orm) = (map()?, map()?, map()?);
        ensure!(albedo.format == MapFormat::Bc1Srgb && normal.format == MapFormat::Bc5 && orm.format == MapFormat::Bc5);
        ensure!(albedo.size == normal.size, "colour and normal maps differ in size");
        Ok(MaterialPack { name, mean_albedo, albedo, normal, orm })
    }

    pub fn load(path: &std::path::Path) -> Result<MaterialPack> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_bytes(&bytes).with_context(|| format!("parsing {}", path.display()))
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len()).context("truncated material pack")?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    crate::color::srgb_to_linear(c as f32 / 255.0)
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

/// 2×2 box filter of an RGBA8 image of side `size`, through `f` / `g` per channel.
fn downsample_with(rgba: &[u8], size: u32, combine: impl Fn([[u8; 4]; 4]) -> [u8; 4]) -> Vec<u8> {
    let (s, h) = (size as usize, (size / 2) as usize);
    let mut out = vec![0u8; h * h * 4];
    for y in 0..h {
        for x in 0..h {
            let at = |dx: usize, dy: usize| {
                let i = ((2 * y + dy) * s + 2 * x + dx) * 4;
                [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
            };
            let px = combine([at(0, 0), at(1, 0), at(0, 1), at(1, 1)]);
            out[(y * h + x) * 4..(y * h + x) * 4 + 4].copy_from_slice(&px);
        }
    }
    out
}

/// Colour averaged in linear light.
fn downsample_srgb(rgba: &[u8], size: u32) -> Vec<u8> {
    downsample_with(rgba, size, |p| {
        let mut out = [255u8; 4];
        for c in 0..3 {
            out[c] = linear_to_srgb(p.iter().map(|q| srgb_to_linear(q[c])).sum::<f32>() / 4.0);
        }
        out
    })
}

fn downsample_linear(rgba: &[u8], size: u32) -> Vec<u8> {
    downsample_with(rgba, size, |p| {
        let mut out = [0u8; 4];
        for c in 0..4 {
            out[c] = ((p.iter().map(|q| q[c] as u32).sum::<u32>() + 2) / 4) as u8;
        }
        out
    })
}

/// Normals averaged as vectors and renormalised.
fn downsample_normal(rgba: &[u8], size: u32) -> Vec<u8> {
    downsample_with(rgba, size, |p| {
        let mut v = [0.0f32; 3];
        for q in &p {
            let n = decode_normal(q[0], q[1], q[2]);
            for c in 0..3 {
                v[c] += n[c];
            }
        }
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        let e = |x: f32| ((x / len * 0.5 + 0.5) * 255.0).round() as u8;
        [e(v[0]), e(v[1]), e(v[2]), 255]
    })
}

fn decode_normal(r: u8, g: u8, b: u8) -> [f32; 3] {
    let d = |c: u8| c as f32 / 255.0 * 2.0 - 1.0;
    [d(r), d(g), d(b)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker(size: u32) -> Vec<u8> {
        (0..size * size)
            .flat_map(|i| if (i / size + i % size).is_multiple_of(2) { [200, 40, 40, 255] } else { [40, 40, 200, 255] })
            .collect()
    }

    fn flat_normal(size: u32) -> Vec<u8> {
        (0..size * size).flat_map(|_| [128u8, 128, 255, 255]).collect()
    }

    fn pack() -> MaterialPack {
        let orm: Vec<u8> = (0..8 * 8).flat_map(|_| [255u8, 180, 0, 255]).collect();
        let maps = SourceMaps { size: 16, albedo: &checker(16), normal: &flat_normal(16), orm: &orm, orm_size: 8 };
        MaterialPack::encode("Test", &maps).unwrap()
    }

    #[test]
    fn encodes_full_mip_chains() {
        let p = pack();
        assert_eq!(p.albedo.levels.len(), 5);
        assert_eq!(p.orm.levels.len(), 4);
        assert_eq!(p.albedo.levels[0].len(), 4 * 4 * 8);
        assert_eq!(p.normal.levels[0].len(), 4 * 4 * 16);
        // Checker of two colours averages to their mean in linear light.
        let expected = (srgb_to_linear(200) + srgb_to_linear(40)) / 2.0;
        assert!((p.mean_albedo[0] - expected).abs() < 0.01, "{:?}", p.mean_albedo);
    }

    #[test]
    fn round_trips_through_bytes() {
        let p = pack();
        assert_eq!(MaterialPack::from_bytes(&p.to_bytes()).unwrap(), p);
        let mut bad = p.to_bytes();
        bad.truncate(bad.len() - 3);
        assert!(MaterialPack::from_bytes(&bad).is_err());
        assert!(MaterialPack::from_bytes(b"nope").is_err());
    }

    #[test]
    fn decodes_for_fallback() {
        let p = pack();
        let top = p.normal.decode(0);
        assert_eq!(top.len(), 16 * 16 * 4);
        // Flat normal stays (0, 0) in X/Y after compression.
        assert!(top.chunks(4).all(|px| (px[0] as i32 - 128).abs() <= 2 && (px[1] as i32 - 128).abs() <= 2));
        let orm = p.orm.decode(0);
        assert!(orm.chunks(4).all(|px| px[0] >= 250 && (px[1] as i32 - 180).abs() <= 2));
        // The 1×1 level decodes despite being smaller than a block.
        assert_eq!(p.albedo.decode(4).len(), 4);
    }

    #[test]
    fn normal_mips_stay_unit_length() {
        // A 45° bump to the right and one to the left average to straight up.
        let mut normal = Vec::new();
        for i in 0..4 {
            normal.extend_from_slice(if i % 2 == 0 { &[218u8, 128, 218, 255] } else { &[37u8, 128, 218, 255] });
        }
        let half = downsample_normal(&normal, 2);
        assert!((half[0] as i32 - 128).abs() <= 1 && half[2] >= 254, "{half:?}");
    }
}
