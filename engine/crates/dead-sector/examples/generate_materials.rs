//! Generates Dead Sector's stylized PBR materials procedurally (no image
//! generation model is involved): seamless 1024² albedo, height, normal,
//! roughness and AO maps per material, plus 3 × 3 tiled previews.
//!
//!   cargo run --release -p dead-sector --example generate_materials -- <source-out> [preview-out]
//!
//! Then import the folders with `import_materials` like any other material.
//! Every map is checked for seams (wrap-around jump vs. ordinary pixel steps)
//! and generation fails if one shows. Art direction: clean readable shapes,
//! bevelled edges with painted highlights, soft baked occlusion, controlled
//! palettes and light wear. All materials assume a 3 m tile (≈ 341 px/m).

use anyhow::{Context, Result, bail};
use sector_assets::texgen::{
    Field, cavity_ao, fbm, hash01, hex, lerp3, noise, normal_map, seam_ratio, smoothstep, tile3x3, to_srgb8, voronoi,
};
use std::path::Path;

const SIZE: usize = 1024;
/// Worst acceptable seam ratio: the wrap may be at most as strong as the
/// strongest ordinary line in the map (plus rounding).
const MAX_SEAM: f32 = 1.05;

/// A material before its derived maps are made.
struct Recipe {
    albedo: Vec<[f32; 3]>,
    height: Field,
    roughness: Field,
    /// Relief in texels across the height range (normal strength).
    relief: f32,
    /// Extra occlusion multiplied into the cavity AO (1 = none).
    occlusion: Option<Field>,
}

fn field(f: impl Fn(usize, usize) -> f32) -> Field {
    Field::from_fn(SIZE, f)
}

/// Rounded-bevel profile: 0 in a gap, rising to 1 over `bevel` texels from the edge.
fn bevel(edge_distance: f32, bevel: f32) -> f32 {
    let t = (edge_distance / bevel).clamp(0.0, 1.0);
    (1.0 - (1.0 - t).powi(2)).sqrt()
}

/// A running-bond or grid layout of rectangles: per pixel, distance to the
/// nearest edge (texels), the element's id, and the position inside it (0..1).
struct Layout {
    edge: Field,
    id: Vec<u32>,
    local: Vec<(f32, f32)>,
}

fn layout(cols: usize, rows: usize, stagger: bool, gap: f32, seed: u32) -> Layout {
    let (w, h) = (SIZE as f32 / cols as f32, SIZE as f32 / rows as f32);
    let wobble = fbm(SIZE, 16, 3, 0.5, seed ^ 0x51);
    let mut edge = Field::new(SIZE, 0.0);
    let mut id = vec![0; SIZE * SIZE];
    let mut local = vec![(0.0, 0.0); SIZE * SIZE];
    for y in 0..SIZE {
        let row = (y as f32 / h) as usize;
        let shift = if stagger && row % 2 == 1 { w / 2.0 } else { 0.0 };
        for x in 0..SIZE {
            let xs = (x as f32 + shift).rem_euclid(SIZE as f32);
            let col = (xs / w) as usize % cols;
            let (lx, ly) = (xs - col as f32 * w, y as f32 - row as f32 * h);
            let d = lx.min(w - lx).min(ly).min(h - ly);
            // Hand-chipped edges: the gap wobbles a little.
            let i = y * SIZE + x;
            edge.data[i] = d - gap / 2.0 - wobble.data[i] * gap * 0.8;
            id[i] = (row * cols + col) as u32;
            local[i] = (lx / w, ly / h);
        }
    }
    Layout { edge, id, local }
}

fn pick(palette: &[u32], id: u32, seed: u32) -> [f32; 3] {
    hex(palette[(hash01(id as i64, 0, seed) * palette.len() as f32) as usize % palette.len()])
}

fn scale(c: [f32; 3], k: f32) -> [f32; 3] {
    c.map(|v| v * k)
}

// --- Style helpers -------------------------------------------------------

/// Hand-painted value bands: `v` (0..1) quantised to `steps` tones with soft
/// transitions, instead of continuous noise.
fn poster(v: f32, steps: f32) -> f32 {
    let x = v.clamp(0.0, 0.9999) * steps;
    (x.floor() + smoothstep(0.4, 0.6, x.fract())) / steps
}

/// Painted top light across an element: brighter at the top (`ly` = 0).
fn top_light(ly: f32, amount: f32) -> f32 {
    1.0 + amount * (0.5 - ly)
}

// --- Recipes -------------------------------------------------------------

fn bricks(painted: bool) -> Recipe {
    let l = layout(10, 30, true, 9.0, 11);
    let stain = fbm(SIZE, 3, 3, 0.5, 12);
    let chips = fbm(SIZE, 10, 4, 0.5, 14);
    let height = field(|x, y| {
        let i = y * SIZE + x;
        let jitter = hash01(l.id[i] as i64, 1, 15) * 0.12;
        bevel(l.edge.data[i], 7.0) * (0.86 + jitter)
    });
    let reds = [0xa84c36, 0xb65c40, 0x98452f, 0xbd6a48];
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let mortar = l.edge.data[i] <= 0.0;
        let ly = l.local[i].1;
        let tone = poster(stain.data[i], 3.0);
        let mut c = if mortar {
            hex(0xcbc1ac)
        } else {
            scale(pick(&reds, l.id[i], 16), top_light(ly, 0.16) * (0.94 + tone * 0.1))
        };
        let mut r = if mortar { 0.92 } else { 0.8 };
        if painted {
            // Cream paint over the wall; clean chips show the brick, with a
            // lighter rim of lifted paint around each one.
            let chip = chips.data[i];
            if mortar || chip < 0.68 {
                let rim = smoothstep(0.62, 0.68, chip);
                c = scale(hex(0xe8ddc4), (if mortar { 0.9 } else { top_light(ly, 0.08) }) * (1.0 + rim * 0.08));
                r = 0.6;
            }
        }
        albedo.push(c);
        roughness.data[i] = r;
    }
    Recipe { albedo, height, roughness, relief: 10.0, occlusion: None }
}

#[allow(clippy::too_many_arguments)]
fn tiles(
    cols: usize,
    gap: f32,
    palette: &[u32],
    grout: u32,
    checker: bool,
    gloss: f32,
    cracks: bool,
    seed: u32,
) -> Recipe {
    let l = layout(cols, cols, false, gap, seed);
    let stain = fbm(SIZE, 3, 3, 0.5, seed + 1);
    let crack = voronoi(SIZE, 4, 0.9, seed + 3);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let id = l.id[i];
        let (row, col) = (id as usize / cols, id as usize % cols);
        let grouted = l.edge.data[i] <= 0.0;
        let base = if checker { hex(palette[(row + col) % palette.len()]) } else { pick(palette, id, seed + 5) };
        let (_, ly) = l.local[i];
        let mut c = scale(base, top_light(ly, 0.1) * (0.95 + hash01(id as i64, 2, seed) * 0.08));
        let mut h = bevel(l.edge.data[i], 6.0) * 0.9;
        // A few cracked slabs: one clean crack line through the tile.
        if cracks && hash01(id as i64, 3, seed) > 0.85 && (crack.f2.data[i] - crack.f1.data[i]) < 0.012 {
            h -= 0.3;
            c = scale(c, 0.62);
        }
        if grouted {
            c = hex(grout);
        }
        albedo.push(scale(c, 0.95 + poster(stain.data[i], 3.0) * 0.08));
        roughness.data[i] = if grouted { 0.92 } else { gloss };
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 8.0, occlusion: None }
}

fn planks(boards: usize, palette: &[u32], gloss: f32, seed: u32) -> Recipe {
    let w = SIZE as f32 / boards as f32;
    let flow = fbm(SIZE, 2, 2, 0.5, seed + 1);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let col = (x as f32 / w) as usize;
            // Each board column has 1–3 butt joints at its own offsets (periodic in y).
            let pieces = 1 + (hash01(col as i64, 7, seed) * 3.0) as usize;
            let offset = hash01(col as i64, 8, seed) * SIZE as f32;
            let len = SIZE as f32 / pieces as f32;
            let along = (y as f32 + offset).rem_euclid(SIZE as f32);
            let piece = (along / len) as usize;
            let lx = x as f32 - col as f32 * w;
            let ly = along - piece as f32 * len;
            let edge = lx.min(w - lx).min(ly).min(len - ly) - 2.5;
            let id = (col * 4 + piece) as u32;
            // Grain: smooth flowing lines along the board, a whole number of waves per tile.
            let phase = hash01(id as i64, 10, seed) * std::f32::consts::TAU;
            let wave = (y as f32 / SIZE as f32 * std::f32::consts::TAU * 3.0 + phase).sin() * 0.18 + flow.data[i] * 0.5;
            let line = ((lx / w * 7.0 + wave).fract() - 0.5).abs();
            let grain = smoothstep(0.42, 0.5, line);
            let mut c = scale(pick(palette, id, seed + 3), top_light(lx / w, 0.12) * (1.0 - grain * 0.18));
            if edge <= 0.0 {
                c = scale(c, 0.45);
            }
            albedo.push(c);
            height.data[i] = bevel(edge, 4.0) * (0.9 + hash01(id as i64, 9, seed) * 0.1) - grain * 0.04;
            roughness.data[i] = gloss + grain * 0.1;
        }
    }
    Recipe { albedo, height, roughness, relief: 7.0, occlusion: None }
}

fn asphalt() -> Recipe {
    let patches = fbm(SIZE, 2, 3, 0.5, 21);
    let flakes = voronoi(SIZE, 40, 0.9, 23);
    let cracks = voronoi(SIZE, 2, 0.9, 24);
    let crack_mask = fbm(SIZE, 2, 2, 0.5, 25);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            // Three painted tones in broad patches.
            let mut c = lerp3(hex(0x33373c), hex(0x40444a), poster(patches.data[i], 3.0));
            let mut h = 0.3;
            let mut r = 0.84;
            // Sparse light aggregate flakes, painted with a highlight.
            let f = hash01(flakes.id[i] as i64, 3, 27);
            if f > 0.93 && flakes.f1.data[i] < 0.14 {
                c = lerp3(c, hex(0x7a7d82), 0.45);
                h += 0.2;
            }
            // Sealed cracks: smooth dark tar lines.
            if crack_mask.data[i] > 0.7 && cracks.f2.data[i] - cracks.f1.data[i] < 0.008 {
                c = scale(c, 0.72);
                h += 0.05;
                r = 0.55;
            }
            albedo.push(c);
            height.data[i] = h;
            roughness.data[i] = r;
        }
    }
    Recipe { albedo, height, roughness, relief: 3.0, occlusion: None }
}

fn grass() -> Recipe {
    let clumps = voronoi(SIZE, 16, 0.85, 31);
    let tone = fbm(SIZE, 2, 2, 0.5, 32);
    let blades = noise(SIZE, 160, 18, 33);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let dome = 1.0 - smoothstep(0.0, 0.8, clumps.f1.data[i]);
        let lift = hash01(clumps.id[i] as i64, 4, 35);
        // Blade tips in the upper part of each clump, two tones only.
        let tip = smoothstep(0.55, 0.62, blades.data[i]) * smoothstep(0.35, 0.7, dome);
        let shade = poster(dome * 0.75 + lift * 0.25, 4.0);
        let mut c = lerp3(hex(0x3c6a25), hex(0x7aa23e), shade);
        c = lerp3(c, hex(0xa9c45c), tip * 0.7);
        c = scale(c, 0.94 + poster(tone.data[i], 2.0) * 0.1);
        albedo.push(c);
        height.data[i] = dome * 0.7 + tip * 0.3;
    }
    let roughness = Field::new(SIZE, 0.9);
    Recipe { albedo, height, roughness, relief: 6.0, occlusion: None }
}

fn dirt() -> Recipe {
    let tone = fbm(SIZE, 3, 3, 0.5, 41);
    let pebbles = voronoi(SIZE, 14, 0.8, 43);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let mut c = lerp3(hex(0x6b4f36), hex(0x8b6b49), poster(tone.data[i], 3.0));
            let mut h = poster(tone.data[i], 3.0) * 0.3;
            let p = hash01(pebbles.id[i] as i64, 5, 44);
            let d = pebbles.f1.data[i];
            let radius = 0.18 + p * 0.1;
            if p > 0.62 {
                if d < radius {
                    // Pebble: flat colour, painted highlight towards the top.
                    let dome = (1.0 - d / radius).sqrt();
                    let v = (y as f32 * 14.0 / SIZE as f32).fract() - 0.5;
                    c = scale(lerp3(hex(0x8f8577), hex(0xa79d8e), p), 0.9 + dome * 0.12 - v * 0.15);
                    h += dome * 0.55;
                } else if d < radius + 0.06 {
                    c = scale(c, 0.75);
                }
            }
            albedo.push(c);
            height.data[i] = h;
        }
    }
    let roughness = Field::new(SIZE, 0.95);
    Recipe { albedo, height, roughness, relief: 6.0, occlusion: None }
}

fn concrete() -> Recipe {
    // 2 × 2 cast panels (1.5 m) with shallow joints and 3 × 3 tie holes each.
    let l = layout(2, 2, false, 6.0, 51);
    let stain = fbm(SIZE, 3, 3, 0.5, 52);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let (lx, ly) = l.local[i];
        let hole_x = ((lx * 3.0).fract() - 0.5) * SIZE as f32 / 6.0;
        let hole_y = ((ly * 3.0).fract() - 0.5) * SIZE as f32 / 6.0;
        let hole = (hole_x * hole_x + hole_y * hole_y).sqrt();
        let mut h = bevel(l.edge.data[i], 5.0) * 0.8;
        let panel = hash01(l.id[i] as i64, 2, 53) * 0.06;
        let mut c = scale(lerp3(hex(0x9d9c95), hex(0xb0aea6), poster(stain.data[i], 3.0)), top_light(ly, 0.08) + panel);
        if hole < 7.0 {
            h -= 0.4 * (1.0 - hole / 7.0);
            c = scale(c, 0.62);
        }
        if l.edge.data[i] <= 0.0 {
            c = scale(c, 0.72);
        }
        albedo.push(c);
        height.data[i] = h;
        roughness.data[i] = 0.84;
    }
    Recipe { albedo, height, roughness, relief: 5.0, occlusion: None }
}

fn plaster() -> Recipe {
    // Clean painted plaster: two soft tones and a faint trowel texture. No chips
    // or cracks: on large walls any distinct mark repeats every tile and makes
    // the tiling readable; wear comes from the game's world-space grime.
    let tone = fbm(SIZE, 2, 3, 0.5, 61);
    let trowel = fbm(SIZE, 24, 2, 0.5, 62);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let t = poster(tone.data[i], 2.0);
        albedo.push(scale(lerp3(hex(0xd8d0c1), hex(0xe0d9ca), t), 0.99 + trowel.data[i] * 0.02));
        height.data[i] = t * 0.2 + trowel.data[i] * 0.3;
    }
    let roughness = Field::new(SIZE, 0.72);
    Recipe { albedo, height, roughness, relief: 1.5, occlusion: None }
}

fn siding() -> Recipe {
    // Horizontal lap boards: 16 per tile, each thicker at its lower edge.
    let boards = 16;
    let h_board = SIZE as f32 / boards as f32;
    let tone = fbm(SIZE, 2, 3, 0.5, 72);
    let chips = fbm(SIZE, 12, 4, 0.5, 73);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        let t = (y as f32 % h_board) / h_board;
        let profile = smoothstep(0.0, 0.08, t) * (0.6 + 0.4 * t);
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let mut c = scale(lerp3(hex(0x5a8b85), hex(0x679890), poster(tone.data[i], 2.0)), top_light(t, 0.14));
            let mut r = 0.55;
            if chips.data[i] > 0.86 {
                c = hex(0x8f7558);
                r = 0.8;
            } else if chips.data[i] > 0.84 {
                c = scale(c, 1.08);
            }
            if t < 0.05 {
                c = scale(c, 0.5);
            }
            albedo.push(c);
            height.data[i] = profile;
            roughness.data[i] = r;
        }
    }
    Recipe { albedo, height, roughness, relief: 9.0, occlusion: None }
}

fn metal_panel() -> Recipe {
    // Vertical ribs every 1/32 of the tile, panel seams every half, rivets along seams.
    let ribs = 32.0;
    let streaks = noise(SIZE, 48, 2, 81);
    let tone = fbm(SIZE, 2, 3, 0.5, 82);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let u = x as f32 / SIZE as f32 * ribs;
            let rib = (u.fract() - 0.5).abs() * 2.0;
            let profile = smoothstep(0.55, 0.35, rib);
            let v = (y as f32 / (SIZE as f32 / 2.0)).fract();
            let seam_d = v.min(1.0 - v) * SIZE as f32 / 2.0;
            let mut h = profile * 0.7;
            // Ribs catch light on their crest: painted two-tone.
            let mut c = scale(
                lerp3(hex(0x74818b), hex(0x84919c), poster(tone.data[i], 2.0)),
                0.9 + poster(profile, 2.0) * 0.14,
            );
            if seam_d < 10.0 {
                h = h * 0.4 + 0.5;
                c = scale(c, 0.82);
                let (rx, ry) = ((u.fract() - 0.5) * SIZE as f32 / ribs, seam_d - 5.0);
                if (u as usize).is_multiple_of(2) && (rx * rx + ry * ry).sqrt() < 4.0 {
                    h += 0.3;
                    c = scale(c, 1.2);
                }
            }
            // Clean vertical drip streaks.
            c = scale(c, 1.0 - smoothstep(0.7, 0.74, streaks.data[i]) * 0.12);
            albedo.push(c);
            height.data[i] = h;
            roughness.data[i] = 0.45;
        }
    }
    Recipe { albedo, height, roughness, relief: 6.0, occlusion: None }
}

fn painted_metal() -> Recipe {
    let tone = fbm(SIZE, 2, 3, 0.5, 91);
    let chips = fbm(SIZE, 12, 4, 0.5, 93);
    let scratches = noise(SIZE, 3, 192, 94);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let mut c = lerp3(hex(0x5d6e67), hex(0x6a7b73), poster(tone.data[i], 2.0));
        let mut r = 0.48;
        let mut h = 0.6;
        // Chipped paint as clean shapes with a lighter rim; a few long scratches.
        if chips.data[i] > 0.77 || scratches.data[i] > 0.965 {
            c = hex(0xa6adb0);
            r = 0.3;
            h = 0.45;
        } else if chips.data[i] > 0.74 {
            c = scale(c, 1.12);
        }
        albedo.push(c);
        height.data[i] = h;
        roughness.data[i] = r;
    }
    Recipe { albedo, height, roughness, relief: 3.0, occlusion: None }
}

fn rust() -> Recipe {
    let blotch = fbm(SIZE, 3, 4, 0.55, 101);
    let paint = fbm(SIZE, 5, 3, 0.5, 103);
    let pits = voronoi(SIZE, 40, 0.9, 104);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        // Three rust tones in clean bands.
        let band = poster(blotch.data[i], 3.0);
        let mut c = if band < 0.34 {
            hex(0x9a5530)
        } else if band < 0.67 {
            hex(0x7d4127)
        } else {
            hex(0x58301f)
        };
        let mut h = 0.4 + band * 0.2;
        let mut r = 0.88;
        if paint.data[i] > 0.74 {
            c = hex(0x5d6e67);
            h = 0.75;
            r = 0.55;
        } else if paint.data[i] > 0.71 {
            c = hex(0xb06a3e);
        }
        let pit = hash01(pits.id[i] as i64, 6, 105);
        if pit > 0.8 && pits.f1.data[i] < 0.2 {
            h -= 0.25 * (1.0 - pits.f1.data[i] / 0.2);
            c = scale(c, 0.7);
        }
        albedo.push(c);
        height.data[i] = h;
        roughness.data[i] = r;
    }
    Recipe { albedo, height, roughness, relief: 5.0, occlusion: None }
}

fn roof() -> Recipe {
    // Bitumen membrane in 4 rolls (0.75 m) with raised overlap seams and fine
    // granules; 4 keeps every seam on the same compression-block phase.
    let rolls = 4.0;
    let granules = fbm(SIZE, 128, 2, 0.6, 111);
    let tone = fbm(SIZE, 2, 3, 0.5, 112);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        let v = (y as f32 / SIZE as f32 * rolls).fract();
        let seam = smoothstep(0.035, 0.0, v.min(1.0 - v));
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let c = scale(
                lerp3(hex(0x45484d), hex(0x505358), poster(tone.data[i], 2.0)),
                top_light(v, 0.06) * (0.95 + granules.data[i] * 0.08) - seam * 0.12,
            );
            albedo.push(c);
            height.data[i] = seam * 0.6 + granules.data[i] * 0.15;
        }
    }
    let roughness = Field::new(SIZE, 0.9);
    Recipe { albedo, height, roughness, relief: 6.0, occlusion: None }
}

// --- Output --------------------------------------------------------------

fn gray8(f: &Field) -> Vec<u8> {
    f.data
        .iter()
        .flat_map(|&v| {
            let b = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            [b, b, b, 255]
        })
        .collect()
}

fn save(path: &Path, rgba: &[u8], size: usize) -> Result<()> {
    image::save_buffer(path, rgba, size as u32, size as u32, image::ColorType::Rgba8)
        .with_context(|| format!("writing {}", path.display()))
}

fn write(name: &str, recipe: Recipe, out: &Path, previews: Option<&Path>) -> Result<()> {
    let height = recipe.height.normalized();
    // Stylized occlusion stays subtle (never below 0.55): the city is lit
    // mostly by ambient light, which AO multiplies.
    let mut ao = cavity_ao(&height, 6, 1.2).zip(&cavity_ao(&height, 24, 0.8), |a, b| 0.55 + 0.45 * a * b);
    if let Some(extra) = &recipe.occlusion {
        ao = ao.zip(extra, |a, b| a * b);
    }
    // Hand-painted look: soft occlusion and convex-edge highlights baked into the colour.
    let curvature = height.zip(&height.blurred(2), |h, b| (h - b).max(0.0));
    let albedo: Vec<u8> = recipe
        .albedo
        .iter()
        .enumerate()
        .flat_map(|(i, &c)| {
            let lit = scale(c, 0.8 + 0.2 * ao.data[i] + (curvature.data[i] * 2.5).min(0.12));
            let [r, g, b] = to_srgb8(lit);
            [r, g, b, 255]
        })
        .collect();
    let normal = normal_map(&height, recipe.relief);
    let maps = [
        (format!("{name}.png"), albedo),
        (format!("{name}_normal.png"), normal),
        (format!("{name}_roughness.png"), gray8(&recipe.roughness)),
        (format!("{name}_ambient.png"), gray8(&ao)),
        (format!("{name}_height.png"), gray8(&height)),
    ];
    let dir = out.join(name);
    std::fs::create_dir_all(&dir)?;
    let mut worst: f32 = 0.0;
    for (file, rgba) in &maps {
        let seam = seam_ratio(rgba, SIZE);
        worst = worst.max(seam);
        if seam > MAX_SEAM && std::env::var_os("GEN_ALLOW_SEAMS").is_none() {
            bail!("{file}: visible seam (ratio {seam:.2})");
        }
        save(&dir.join(file), rgba, SIZE)?;
    }
    if let Some(previews) = previews {
        // Colour and a simple lit view (light from the upper left), tiled 3 × 3.
        let light = [-0.5f32, -0.5, 0.707];
        let lit: Vec<u8> = maps[0]
            .1
            .chunks(4)
            .zip(maps[1].1.chunks(4))
            .flat_map(|(c, n)| {
                let d = |v: u8| v as f32 / 127.5 - 1.0;
                let shade = (d(n[0]) * light[0] + d(n[1]) * light[1] + d(n[2]) * light[2]).max(0.0) * 0.8 + 0.25;
                [
                    (c[0] as f32 * shade).min(255.0) as u8,
                    (c[1] as f32 * shade).min(255.0) as u8,
                    (c[2] as f32 * shade).min(255.0) as u8,
                    255,
                ]
            })
            .collect();
        std::fs::create_dir_all(previews)?;
        save(&previews.join(format!("{name}_3x3.png")), &tile3x3(&maps[0].1, SIZE), SIZE * 3)?;
        save(&previews.join(format!("{name}_lit_3x3.png")), &tile3x3(&lit, SIZE), SIZE * 3)?;
    }
    println!("{name}: 5 maps, worst seam ratio {worst:.2}");
    Ok(())
}

type RecipeFn = fn() -> Recipe;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(out) = args.first() else { bail!("usage: generate_materials <source-out> [preview-out]") };
    let previews = args.get(1).map(Path::new);
    let out = Path::new(out);
    let recipes: Vec<(&str, RecipeFn)> = vec![
        ("StylizedAsphalt", asphalt),
        ("StylizedSidewalk", || tiles(6, 9.0, &[0xb4aea2, 0xbab4a8, 0xaea89c], 0x8a857b, false, 0.8, true, 201)),
        ("StylizedGrass", grass),
        ("StylizedDirt", dirt),
        ("StylizedConcrete", concrete),
        ("StylizedBrickRed", || bricks(false)),
        ("StylizedBrickPainted", || bricks(true)),
        ("StylizedPlaster", plaster),
        ("StylizedPaintedSiding", siding),
        ("StylizedMetalPanel", metal_panel),
        ("StylizedPaintedMetal", painted_metal),
        ("StylizedRust", rust),
        ("StylizedWoodPlanks", || planks(12, &[0x8c5d36, 0x9a6a3f, 0x845733, 0xa57447], 0.62, 301)),
        ("StylizedInteriorFloor", || planks(8, &[0xc39a68, 0xcfa774, 0xb98f5f], 0.42, 311)),
        ("StylizedRoof", roof),
        ("StylizedFloorTiles", || tiles(10, 6.0, &[0xd9d4c7, 0xa7b6b6], 0x9a958b, true, 0.3, false, 211)),
    ];
    for (name, recipe) in recipes {
        write(name, recipe(), out, previews)?;
    }
    Ok(())
}
