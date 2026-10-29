//! Procedural texture synthesis for tileable materials. Every primitive
//! wraps at the image edges (periodic lattices, wrap-around filters), so
//! anything built from them tiles seamlessly in both directions.

/// A square scalar field that wraps at its edges.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub size: usize,
    pub data: Vec<f32>,
}

impl Field {
    pub fn new(size: usize, value: f32) -> Field {
        Field { size, data: vec![value; size * size] }
    }

    pub fn from_fn(size: usize, f: impl Fn(usize, usize) -> f32) -> Field {
        let mut data = Vec::with_capacity(size * size);
        for y in 0..size {
            for x in 0..size {
                data.push(f(x, y));
            }
        }
        Field { size, data }
    }

    /// Value at integer coordinates, wrapping.
    pub fn at(&self, x: isize, y: isize) -> f32 {
        let s = self.size as isize;
        self.data[(y.rem_euclid(s) * s + x.rem_euclid(s)) as usize]
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.size + x]
    }

    pub fn map(&self, f: impl Fn(f32) -> f32) -> Field {
        Field { size: self.size, data: self.data.iter().map(|&v| f(v)).collect() }
    }

    pub fn zip(&self, other: &Field, f: impl Fn(f32, f32) -> f32) -> Field {
        assert_eq!(self.size, other.size);
        Field { size: self.size, data: self.data.iter().zip(&other.data).map(|(&a, &b)| f(a, b)).collect() }
    }

    /// Rescales to 0..1.
    pub fn normalized(&self) -> Field {
        let (lo, hi) = self.data.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        let span = (hi - lo).max(1e-6);
        self.map(|v| (v - lo) / span)
    }

    /// Wrap-around box blur of `radius` pixels, three passes (≈ Gaussian).
    pub fn blurred(&self, radius: usize) -> Field {
        let mut f = self.clone();
        for _ in 0..3 {
            f = f.box_pass(radius, true).box_pass(radius, false);
        }
        f
    }

    fn box_pass(&self, radius: usize, horizontal: bool) -> Field {
        let s = self.size as isize;
        let r = radius as isize;
        let norm = 1.0 / (2 * r + 1) as f32;
        let mut out = Field::new(self.size, 0.0);
        for line in 0..s {
            let sample = |i: isize| if horizontal { self.at(i, line) } else { self.at(line, i) };
            let mut sum: f32 = (-r..=r).map(sample).sum();
            for i in 0..s {
                let idx = if horizontal { line * s + i } else { i * s + line } as usize;
                out.data[idx] = sum * norm;
                sum += sample(i + r + 1) - sample(i - r);
            }
        }
        out
    }
}

fn hash(x: i64, y: i64, seed: u32) -> u32 {
    let mut h =
        (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

/// Deterministic 0..1 value for a lattice point.
pub fn hash01(x: i64, y: i64, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Value noise with `px` × `py` lattice cells across the image (periodic).
pub fn noise(size: usize, px: usize, py: usize, seed: u32) -> Field {
    let (px, py) = (px.max(1) as i64, py.max(1) as i64);
    Field::from_fn(size, |x, y| {
        let u = x as f32 * px as f32 / size as f32;
        let v = y as f32 * py as f32 / size as f32;
        let (i, j) = (u.floor() as i64, v.floor() as i64);
        let (fu, fv) = (smooth(u - i as f32), smooth(v - j as f32));
        let corner = |a: i64, b: i64| hash01(a.rem_euclid(px), b.rem_euclid(py), seed);
        let top = corner(i, j) + (corner(i + 1, j) - corner(i, j)) * fu;
        let bottom = corner(i, j + 1) + (corner(i + 1, j + 1) - corner(i, j + 1)) * fu;
        top + (bottom - top) * fv
    })
}

/// Fractal value noise: `octaves` layers from `period` cells, each twice as
/// fine and `gain` times as strong. Result roughly 0..1.
pub fn fbm(size: usize, period: usize, octaves: u32, gain: f32, seed: u32) -> Field {
    let mut total = Field::new(size, 0.0);
    let mut amplitude = 1.0;
    let mut sum = 0.0;
    for o in 0..octaves {
        let p = period << o;
        if p > size {
            break;
        }
        let n = noise(size, p, p, seed.wrapping_add(o * 7919));
        total = total.zip(&n, |t, v| t + v * amplitude);
        sum += amplitude;
        amplitude *= gain;
    }
    total.map(|v| v / sum)
}

/// Periodic Worley noise on a `cells` × `cells` grid: per pixel, the
/// distance to the nearest and second-nearest feature point (in cell units)
/// and the nearest point's cell id.
pub struct Voronoi {
    pub f1: Field,
    pub f2: Field,
    pub id: Vec<u32>,
    /// Pixel minus its nearest feature point (cell units), for radial shapes and lighting.
    pub offset: Vec<(f32, f32)>,
}

pub fn voronoi(size: usize, cells: usize, jitter: f32, seed: u32) -> Voronoi {
    let c = cells as i64;
    let point = |i: i64, j: i64| {
        let (wi, wj) = (i.rem_euclid(c), j.rem_euclid(c));
        let ox = 0.5 + (hash01(wi, wj, seed) - 0.5) * jitter;
        let oy = 0.5 + (hash01(wi, wj, seed ^ 0x9e37) - 0.5) * jitter;
        (i as f32 + ox, j as f32 + oy, (wj * c + wi) as u32)
    };
    let mut f1 = Field::new(size, 0.0);
    let mut f2 = Field::new(size, 0.0);
    let mut id = vec![0; size * size];
    let mut offset = vec![(0.0, 0.0); size * size];
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 * cells as f32 / size as f32;
            let v = y as f32 * cells as f32 / size as f32;
            let (ci, cj) = (u.floor() as i64, v.floor() as i64);
            let (mut d1, mut d2, mut best, mut off) = (f32::MAX, f32::MAX, 0, (0.0, 0.0));
            for dj in -1..=1 {
                for di in -1..=1 {
                    let (px, py, pid) = point(ci + di, cj + dj);
                    let d = ((px - u).powi(2) + (py - v).powi(2)).sqrt();
                    if d < d1 {
                        d2 = d1;
                        d1 = d;
                        best = pid;
                        off = (u - px, v - py);
                    } else if d < d2 {
                        d2 = d;
                    }
                }
            }
            f1.data[y * size + x] = d1;
            f2.data[y * size + x] = d2;
            id[y * size + x] = best;
            offset[y * size + x] = off;
        }
    }
    Voronoi { f1, f2, id, offset }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Tangent-space normals from a height field (0..1 over `depth` texels of
/// relief), DirectX convention: +X along +x, +Y along +y (down the image).
/// Central differences wrap, so the normals tile with the height.
pub fn normal_map(height: &Field, depth: f32) -> Vec<u8> {
    let mut out = Vec::with_capacity(height.size * height.size * 4);
    for y in 0..height.size as isize {
        for x in 0..height.size as isize {
            let dx = (height.at(x + 1, y) - height.at(x - 1, y)) * 0.5 * depth;
            let dy = (height.at(x, y + 1) - height.at(x, y - 1)) * 0.5 * depth;
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let e = |v: f32| ((v / len * 0.5 + 0.5) * 255.0).round() as u8;
            out.extend_from_slice(&[e(-dx), e(-dy), e(1.0), 255]);
        }
    }
    out
}

/// Ambient occlusion from height: how far a texel sits below its blurred
/// surroundings (`radius` texels), scaled by `strength`. 1 = open.
pub fn cavity_ao(height: &Field, radius: usize, strength: f32) -> Field {
    let around = height.blurred(radius);
    height.zip(&around, |h, a| (1.0 - (a - h).max(0.0) * strength).clamp(0.0, 1.0))
}

/// Linear 0..1 colour to sRGB bytes.
pub fn to_srgb8(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    })
}

/// sRGB hex (`0xrrggbb`) to linear colour.
pub fn hex(rgb: u32) -> [f32; 3] {
    [(rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255].map(|c| crate::color::srgb_to_linear(c as f32 / 255.0))
}

/// How much stronger the jump across the wrap seam is than the strongest
/// ordinary line-to-line step anywhere in the image (per channel, worst of the
/// two directions). A seamless image scores at most about 1, even when a
/// structural line (mortar, a board edge) happens to lie on the wrap; a real
/// seam is an outlier no other line matches and scores well above 1.
pub fn seam_ratio(rgba: &[u8], size: usize) -> f32 {
    let px = |x: usize, y: usize, c: usize| rgba[(y * size + x) * 4 + c] as f32;
    let mut worst: f32 = 0.0;
    for c in 0..3 {
        for vertical in [false, true] {
            // Mean step between line k and line k + 1 (wrapping), for every k.
            let step = |k: usize| {
                let next = (k + 1) % size;
                (0..size)
                    .map(|i| {
                        if vertical {
                            (px(i, k, c) - px(i, next, c)).abs()
                        } else {
                            (px(k, i, c) - px(next, i, c)).abs()
                        }
                    })
                    .sum::<f32>()
                    / size as f32
            };
            let seam = step(size - 1);
            let strongest = (0..size - 1).map(step).fold(0.0, f32::max);
            worst = worst.max((seam + 0.5) / (strongest + 0.5));
        }
    }
    worst
}

/// The image tiled 3 × 3, for checking seams by eye.
pub fn tile3x3(rgba: &[u8], size: usize) -> Vec<u8> {
    let big = size * 3;
    let mut out = vec![0u8; big * big * 4];
    for y in 0..big {
        for x in 0..big {
            let (sx, sy) = (x % size, y % size);
            out[(y * big + x) * 4..(y * big + x) * 4 + 4]
                .copy_from_slice(&rgba[(sy * size + sx) * 4..(sy * size + sx) * 4 + 4]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(f: &Field) -> Vec<u8> {
        f.data
            .iter()
            .flat_map(|&v| {
                let b = (v.clamp(0.0, 1.0) * 255.0) as u8;
                [b, b, b, 255]
            })
            .collect()
    }

    #[test]
    fn noise_and_voronoi_tile_seamlessly() {
        let size = 128;
        let n = fbm(size, 4, 5, 0.5, 3);
        assert!(seam_ratio(&gray(&n), size) < 1.5, "{}", seam_ratio(&gray(&n), size));
        let v = voronoi(size, 6, 0.9, 5);
        let f1 = v.f1.map(|d| d.min(1.0));
        assert!(seam_ratio(&gray(&f1), size) < 1.5);
        // A deliberately non-periodic gradient has a hard seam.
        let ramp = Field::from_fn(size, |x, _| x as f32 / size as f32);
        assert!(seam_ratio(&gray(&ramp), size) > 20.0);
    }

    #[test]
    fn blur_and_normals_wrap() {
        let size = 64;
        let mut spot = Field::new(size, 0.0);
        spot.data[0] = 1.0;
        let b = spot.blurred(2);
        // The blurred spot at the corner spreads to the opposite edges.
        assert!(b.get(size - 1, 0) > 0.0 && b.get(0, size - 1) > 0.0);
        assert!((b.data.iter().sum::<f32>() - 1.0).abs() < 1e-3);
        // A height ramp rising towards +x tilts normals towards −x (red < 128).
        let ramp = Field::from_fn(size, |x, _| (x as f32 / size as f32 * std::f32::consts::TAU).sin());
        let n = normal_map(&ramp, 8.0);
        let red_at = |x: usize| n[x * 4];
        assert!(red_at(1) < 128 && red_at(size / 2) > 128);
    }

    #[test]
    fn normals_follow_directx_down_the_image() {
        // Height rising down the image (+y) means the surface faces up the image: green < 128.
        let size = 32;
        let h = Field::from_fn(size, |_, y| (y as f32 / size as f32 * std::f32::consts::TAU).sin());
        let n = normal_map(&h, 8.0);
        assert!(n[(size + 1) * 4 + 1] < 128);
    }

    #[test]
    fn cavities_are_darker() {
        let size = 64;
        let h = Field::from_fn(size, |x, _| if (x / 8) % 2 == 0 { 1.0 } else { 0.0 });
        let ao = cavity_ao(&h, 4, 2.0);
        assert!(ao.get(12, 5) < ao.get(4, 5));
    }
}
