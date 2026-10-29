//! Seamless fractal gradient noise for the ground mist. Godot builds the mist
//! texture with FastNoiseLite (simplex smooth, frequency 0.012, 3 octaves,
//! 256², seamless, normalised); this produces the same kind of texture, not the
//! same bytes.

/// Small integer hash (lowbias32) for lattice gradients.
fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn gradient(ix: i32, iy: i32, period: i32, seed: u32) -> (f32, f32) {
    let (x, y) = (ix.rem_euclid(period) as u32, iy.rem_euclid(period) as u32);
    let angle = hash(x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b) ^ seed) as f32 / u32::MAX as f32
        * std::f32::consts::TAU;
    (angle.cos(), angle.sin())
}

/// Perlin noise in roughly −0.7..0.7, repeating every `period` lattice cells.
fn perlin(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let dot = |ix: i32, iy: i32| {
        let (gx, gy) = gradient(ix, iy, period, seed);
        gx * (x - ix as f32) + gy * (y - iy as f32)
    };
    let (u, v) = (fade(fx), fade(fy));
    let top = dot(x0, y0) + (dot(x0 + 1, y0) - dot(x0, y0)) * u;
    let bottom = dot(x0, y0 + 1) + (dot(x0 + 1, y0 + 1) - dot(x0, y0 + 1)) * u;
    top + (bottom - top) * v
}

/// `size`² seamless noise, `octaves` layers starting at `cells` lattice cells
/// across the texture, normalised to the full 0..=255 range.
pub fn mist_texture(size: u32, cells: i32, octaves: u32, seed: u32) -> Vec<u8> {
    let mut values = Vec::with_capacity((size * size) as usize);
    for py in 0..size {
        for px in 0..size {
            let (mut sum, mut amplitude, mut frequency) = (0.0, 1.0, cells);
            for octave in 0..octaves {
                let scale = frequency as f32 / size as f32;
                sum += perlin(px as f32 * scale, py as f32 * scale, frequency, seed.wrapping_add(octave)) * amplitude;
                amplitude *= 0.5;
                frequency *= 2;
            }
            values.push(sum);
        }
    }
    let (min, max) = values.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    values.iter().map(|v| ((v - min) / (max - min) * 255.0).round() as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_is_normalised_seamless_and_varied() {
        let size = 64;
        let texture = mist_texture(size, 3, 3, 7);
        assert_eq!(texture.len(), 64 * 64);
        assert_eq!(*texture.iter().min().unwrap(), 0);
        assert_eq!(*texture.iter().max().unwrap(), 255);
        // Wrapping across the edges is as smooth as stepping inside the texture.
        let at = |x: u32, y: u32| texture[((y % size) * size + x % size) as usize] as i32;
        let inner: i32 = (0..size).map(|y| (at(31, y) - at(32, y)).abs()).max().unwrap();
        let seam: i32 = (0..size).map(|y| (at(size - 1, y) - at(0, y)).abs()).max().unwrap();
        assert!(seam <= inner * 2 + 8, "seam jump {seam} vs inner {inner}");
        let seam_y: i32 = (0..size).map(|x| (at(x, size - 1) - at(x, 0)).abs()).max().unwrap();
        assert!(seam_y <= inner * 2 + 8, "vertical seam jump {seam_y}");
    }

    #[test]
    fn deterministic() {
        assert_eq!(mist_texture(32, 3, 3, 1), mist_texture(32, 3, 3, 1));
        assert_ne!(mist_texture(32, 3, 3, 1), mist_texture(32, 3, 3, 2));
    }
}
