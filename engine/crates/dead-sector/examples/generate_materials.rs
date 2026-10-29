//! Generates Dead Sector's stylized PBR materials procedurally (no image
//! generation model is involved): seamless 1024² albedo, height, normal,
//! roughness and AO maps per material, plus 3 × 3 tiled previews.
//!
//!   cargo run --release -p dead-sector --example generate_materials -- <source-out> [preview-out]
//!
//! Then import the folders with `import_materials` like any other material.
//! Every map is checked for seams (wrap-around jump vs. ordinary pixel steps)
//! and generation fails if one shows. Art direction (Valorant-inspired, all
//! original): one shared palette; large deliberate colour shapes; every
//! element (brick, slab, board, rib, sheet, clump, pebble) painted with a lit
//! top-left bevel, shaded bottom-right and a soft gradient; colour blocking
//! per element instead of pixel noise; wear only at element edges as clean
//! shapes; detail grouped on purpose and sized to read at gameplay distance.
//! Buildings and ground assume a 3 m tile (≈ 341 px/m), props 2 m.

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
}

fn scale(c: [f32; 3], k: f32) -> [f32; 3] {
    c.map(|v| v * k)
}

fn pick(palette: &[u32], key: i64, seed: u32) -> [f32; 3] {
    hex(palette[(hash01(key, 0, seed) * palette.len() as f32) as usize % palette.len()])
}

// --- Art direction -------------------------------------------------------
//
// One world palette, shared by every material (sRGB hex): warm creams and
// sands, terracotta, sage and teal accents, slate blues, warm woods, and
// blue-grey asphalt. Values stay in a controlled mid range so materials sit
// together under the same light.

const CREAM: u32 = 0xe6dbc3;
const CREAM_SHADE: u32 = 0xd6c9ae;
const SAND: [u32; 4] = [0xc6bca8, 0xbdb3a0, 0xcdc4b1, 0xb5ac9a];
const STONE_JOINT: u32 = 0x857e72;
const TERRACOTTA: [u32; 4] = [0xb9583d, 0xab4f37, 0xc5684a, 0xb35d41];
const TERRACOTTA_DARK: u32 = 0x8a4131;
const TERRACOTTA_LIGHT: u32 = 0xcf7c58;
const MORTAR: u32 = 0xd6cab1;
const TEAL: [u32; 2] = [0x5d918a, 0x67998f];
const SAGE: [u32; 2] = [0x667a71, 0x71857b];
const SLATE: [u32; 2] = [0x6c7f8e, 0x75889a];
const BARE_METAL: u32 = 0xa9b0b3;
const WOOD: [u32; 4] = [0x9b6a3f, 0xa97546, 0x8d5f37, 0xb38050];
const OAK: [u32; 3] = [0xd2a877, 0xc89c6a, 0xdbb383];
const RUST: [u32; 3] = [0x6d3523, 0xa2522b, 0xcf7a3c];
const ASPHALT: [u32; 2] = [0x30353d, 0x3a4049];
const GRASS: [u32; 3] = [0x4f7d2f, 0x5b8a35, 0x47752b];
const GRASS_TIP: u32 = 0x9cc25a;
const GRASS_GAP: u32 = 0x34561f;
const EARTH: [u32; 2] = [0x7a5a3d, 0x8a6847];
const PEBBLES: [u32; 3] = [0xa39686, 0x8e8273, 0xb3a796];
const ROOFING: [u32; 3] = [0x4c5057, 0x545860, 0x484c53];

/// Rounded bevel: 0 in a joint, rising to 1 over `band` texels from the edge.
fn bevel(edge: f32, band: f32) -> f32 {
    let t = (edge / band).clamp(0.0, 1.0);
    (1.0 - (1.0 - t).powi(2)).sqrt()
}

/// Painted light on a rectangular element (light from the upper left):
/// a bright band along the top and left edges, shadow along the bottom and
/// right, and a soft top-to-bottom gradient. `lx`, `ly` are texels from the
/// element's top-left corner; `w`, `h` its size.
fn paint_light(lx: f32, ly: f32, w: f32, h: f32, band: f32, strength: f32) -> f32 {
    let t = smoothstep(band, 0.0, ly);
    let l = smoothstep(band, 0.0, lx);
    let b = smoothstep(band, 0.0, h - ly);
    let r = smoothstep(band, 0.0, w - lx);
    let gradient = 0.09 * (0.5 - ly / h);
    1.0 + strength * (0.32 * t + 0.14 * l - 0.28 * b - 0.12 * r) + gradient
}

/// Per pixel: which rectangle it is in and where, for brick, slab, tile and
/// board layouts. Rows can be staggered by half an element.
struct Elements {
    id: Vec<i64>,
    /// Texels from the element's top-left corner.
    local: Vec<(f32, f32)>,
    /// Texels to the nearest element edge (joint half-width already removed).
    edge: Field,
    w: f32,
    h: f32,
}

fn elements(cols: usize, rows: usize, stagger: bool, joint: f32) -> Elements {
    let (w, h) = (SIZE as f32 / cols as f32, SIZE as f32 / rows as f32);
    let mut id = vec![0; SIZE * SIZE];
    let mut local = vec![(0.0, 0.0); SIZE * SIZE];
    let mut edge = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        let row = (y as f32 / h) as usize;
        let shift = if stagger && row % 2 == 1 { w / 2.0 } else { 0.0 };
        for x in 0..SIZE {
            let xs = (x as f32 + shift).rem_euclid(SIZE as f32);
            let col = (xs / w) as usize % cols;
            let (lx, ly) = (xs - col as f32 * w, y as f32 - row as f32 * h);
            let i = y * SIZE + x;
            id[i] = (row * cols + col) as i64;
            local[i] = (lx, ly);
            edge.data[i] = lx.min(w - lx).min(ly).min(h - ly) - joint / 2.0;
        }
    }
    Elements { id, local, edge, w, h }
}

/// Clean edge wear: true where an element's edge is chipped. Only elements
/// picked by `share`, only within `depth` texels of an edge, in broad
/// patches (never scattered spots).
fn edge_chipped(e: &Elements, patches: &Field, i: usize, depth: f32, share: f32, seed: u32) -> bool {
    hash01(e.id[i], 21, seed) < share && e.edge.data[i] < depth && e.edge.data[i] > 0.0 && patches.data[i] > 0.55
}

// --- Recipes -------------------------------------------------------------

fn bricks(painted: bool) -> Recipe {
    // 9 × 28 per 3 m tile: 33 × 11 cm bricks, staggered, 8-texel mortar.
    let e = elements(9, 28, true, 8.0);
    let patches = fbm(SIZE, 6, 3, 0.5, 13);
    let broad = fbm(SIZE, 2, 2, 0.5, 12);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let (lx, ly) = e.local[i];
        let joint = e.edge.data[i] <= 0.0;
        let tone = hash01(e.id[i], 1, 16);
        // Deliberate variation: mostly the four terracottas, a few burnt and a few pale bricks.
        let brick = if tone < 0.07 {
            hex(TERRACOTTA_DARK)
        } else if tone > 0.95 {
            hex(TERRACOTTA_LIGHT)
        } else {
            pick(&TERRACOTTA, e.id[i], 17)
        };
        let light = paint_light(lx, ly, e.w, e.h, 9.0, 1.0);
        let chipped = edge_chipped(&e, &patches, i, 9.0, 0.18, 18);
        let (mut c, mut r, mut h) = if joint {
            // Mortar sits in the shadow of the brick above.
            let under = smoothstep(0.0, 6.0, ly.min(e.h - ly));
            (scale(hex(MORTAR), 0.86 + 0.14 * under), 0.92, 0.0)
        } else {
            (scale(brick, light), 0.78, bevel(e.edge.data[i], 9.0) * (0.9 + tone * 0.1))
        };
        if chipped && !painted {
            c = scale(hex(TERRACOTTA_DARK), 0.95);
            h *= 0.7;
        }
        if painted {
            // Cream paint everywhere; worn through only at some brick edges,
            // with a lighter rim of lifted paint around each worn patch.
            if chipped {
                c = scale(brick, light * 0.95);
                h *= 0.85;
                r = 0.78;
            } else {
                let rim = !joint && hash01(e.id[i], 21, 18) < 0.18 && e.edge.data[i] < 13.0 && patches.data[i] > 0.5;
                let base = if joint { scale(hex(CREAM_SHADE), 0.92) } else { scale(hex(CREAM), light) };
                c = if rim { scale(base, 1.06) } else { base };
                r = 0.6;
            }
        }
        albedo.push(scale(c, 0.97 + broad.data[i] * 0.06));
        roughness.data[i] = r;
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 12.0 }
}

/// Square slabs or tiles with joints, painted bevel light and colour blocking.
#[allow(clippy::too_many_arguments)]
fn slabs(
    n: usize,
    joint: f32,
    band: f32,
    palette: &[u32],
    joint_colour: u32,
    checker: bool,
    gloss: f32,
    seed: u32,
) -> Recipe {
    let e = elements(n, n, false, joint);
    let patches = fbm(SIZE, 4, 3, 0.5, seed + 1);
    let broad = fbm(SIZE, 2, 2, 0.5, seed + 2);
    let sheen = noise(SIZE, 4, 48, seed + 3);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let (lx, ly) = e.local[i];
        let joint_px = e.edge.data[i] <= 0.0;
        let (row, col) = (e.id[i] / n as i64, e.id[i] % n as i64);
        let base = if checker {
            hex(palette[((row + col) % palette.len() as i64) as usize])
        } else {
            pick(palette, e.id[i], seed)
        };
        let mut c = scale(base, paint_light(lx, ly, e.w, e.h, band, 1.0));
        let mut h = bevel(e.edge.data[i], band);
        let mut r = gloss;
        // Glazed tiles: a soft painted gloss stroke across the upper-left corner.
        if checker {
            let stroke = smoothstep(0.18, 0.05, ((lx + ly) / (e.w + e.h) - 0.22).abs());
            c = scale(c, 1.0 + stroke * 0.07);
        } else {
            // Subtle broom-finish direction, alternating per slab.
            let s = if (row + col) % 2 == 0 { sheen.data[i] } else { sheen.get(((i % SIZE) + 512) % SIZE, i / SIZE) };
            c = scale(c, 0.985 + s * 0.03);
        }
        // A few corners knocked off, as clean shapes.
        if edge_chipped(&e, &patches, i, band * 1.1, 0.12, seed + 4) {
            c = scale(c, 0.82);
            h *= 0.6;
            r = 0.9;
        }
        if joint_px {
            c = hex(joint_colour);
            r = 0.92;
        }
        albedo.push(scale(c, 0.97 + broad.data[i] * 0.06));
        roughness.data[i] = r;
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 10.0 }
}

fn concrete() -> Recipe {
    // 2 × 2 cast panels (1.5 m) per tile, each its own tone; formwork bands,
    // four cone tie holes, and exposed aggregate only along the splash zone.
    let e = elements(2, 2, false, 10.0);
    let broad = fbm(SIZE, 2, 2, 0.5, 52);
    let stones = voronoi(SIZE, 64, 0.85, 53);
    let palette = [0xb3ada1, 0xa9a397, 0xbdb7ab, 0xa49e93];
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let roughness = Field::new(SIZE, 0.84);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let (lx, ly) = e.local[i];
        let mut c = scale(pick(&palette, e.id[i], 54), paint_light(lx, ly, e.w, e.h, 14.0, 0.9));
        let mut h = bevel(e.edge.data[i], 14.0);
        // Formwork: four faint horizontal pour bands per panel.
        let band = (ly / e.h * 4.0).fract();
        c = scale(c, 1.0 - smoothstep(0.92, 1.0, band) * 0.05 + smoothstep(0.08, 0.0, band) * 0.03);
        // Tie holes at the panel's quarter points: dark cone, lit lower rim.
        for (fx, fy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
            let (dx, dy) = (lx - fx * e.w, ly - fy * e.h);
            let d = (dx * dx + dy * dy).sqrt();
            if d < 11.0 {
                c = scale(c, 0.55 + 0.35 * (d / 11.0) + if dy > 4.0 { 0.15 } else { 0.0 });
                h -= 0.35 * (1.0 - d / 11.0);
            }
        }
        // Splash zone: aggregate in the bottom fifth of each panel.
        let splash = smoothstep(0.78, 0.9, ly / e.h);
        let stone = hash01(stones.id[i] as i64, 3, 55);
        if splash > 0.0 && stone > 0.45 && stones.f1.data[i] < 0.3 {
            let (ox, oy) = stones.offset[i];
            c = scale(pick(&PEBBLES, stones.id[i] as i64, 56), 1.0 - (ox + oy) * 0.4);
            h += 0.15 * splash;
        }
        if e.edge.data[i] <= 0.0 {
            c = scale(hex(0xa39d91), 0.7);
        }
        albedo.push(scale(c, 0.97 + broad.data[i] * 0.06));
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 8.0 }
}

fn plaster() -> Recipe {
    // Clean two-tone painted plaster: broad soft tonal shapes and wide, faint
    // trowel sweeps. No marks: on large walls any mark repeats every tile.
    let broad = fbm(SIZE, 2, 2, 0.5, 61);
    let sweeps = noise(SIZE, 6, 3, 62);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let tone = smoothstep(0.35, 0.65, broad.data[i]);
            let sweep = sweeps.get((x + y) % SIZE, y);
            albedo.push(scale(lerp3(hex(CREAM_SHADE), hex(CREAM), tone), 0.985 + sweep * 0.03));
            height.data[i] = tone * 0.3 + sweep * 0.4;
        }
    }
    let roughness = Field::new(SIZE, 0.74);
    Recipe { albedo, height, roughness, relief: 1.5 }
}

fn siding() -> Recipe {
    // 12 lap boards per tile (25 cm): a strong shadow under each lip, a lit
    // top edge, two teal tones, wear only along a few lower lips.
    let boards = 12;
    let bh = SIZE as f32 / boards as f32;
    let grain = noise(SIZE, 3, 96, 71);
    let patches = fbm(SIZE, 6, 3, 0.5, 72);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        let board = (y as f32 / bh) as i64;
        let t = (y as f32 % bh) / bh;
        let profile = smoothstep(0.0, 0.06, t) * (0.55 + 0.45 * t);
        let light = 1.0 + 0.22 * smoothstep(0.12, 0.02, t) - 0.3 * smoothstep(0.86, 1.0, t) + 0.08 * (0.5 - t);
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let mut c = scale(pick(&TEAL, board, 73), light * (0.985 + grain.data[i] * 0.03));
            let mut r = 0.55;
            let worn = hash01(board, 2, 74) < 0.3 && t > 0.88 && patches.data[i] > 0.6;
            if worn {
                c = scale(hex(0xa0784e), light);
                r = 0.8;
            }
            if t < 0.035 {
                c = scale(c, 0.45);
            }
            albedo.push(c);
            height.data[i] = profile;
            roughness.data[i] = r;
        }
    }
    Recipe { albedo, height, roughness, relief: 12.0 }
}

fn metal_panel() -> Recipe {
    // Trapezoid standing ribs, 12 per tile (25 cm), two panels per tile with
    // an overlap seam and a rivet on every rib; the lit and shaded rib faces
    // are painted in, and short streaks run down from the rivets.
    let ribs = 12.0;
    let pitch = SIZE as f32 / ribs;
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.45);
    let mut height = Field::new(SIZE, 0.0);
    let half = SIZE as f32 / 2.0;
    for y in 0..SIZE {
        let panel = (y as f32 / half) as i64;
        let v = (y as f32 % half) / half;
        let seam_d = (y as f32 % half).min(half - y as f32 % half);
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let u = (x as f32 % pitch) / pitch;
            // Profile: valley, rising face, flat crest, falling face.
            let (h, face) = if u < 0.25 {
                (0.0, 0.0)
            } else if u < 0.4 {
                ((u - 0.25) / 0.15, 0.3)
            } else if u < 0.8 {
                (1.0, 0.08)
            } else if u < 0.95 {
                (1.0 - (u - 0.8) / 0.15, -0.26)
            } else {
                (0.0, 0.0)
            };
            // Same tone for both panels, so the seam inside the tile and the one
            // at its edge read alike; broad tone shifts come from macro variation.
            let mut c = scale(hex(SLATE[0]), 1.0 + face + 0.08 * (0.5 - v));
            let mut hh = h * 0.8;
            if seam_d < 9.0 {
                hh = 0.9;
                c = scale(c, if y as f32 % half < half / 2.0 { 1.12 } else { 0.78 });
            }
            // Rivet on each crest, just below the seam, with a streak under it.
            let (rx, ry) = ((u - 0.6) * pitch, y as f32 % half - 16.0);
            let rivet = (rx * rx + ry * ry).sqrt();
            if rivet < 5.0 {
                hh += 0.25;
                c = scale(c, 1.0 + 0.25 * (-(rx + ry) / 5.0).max(-0.6));
            } else if rx.abs() < 2.5 && ry > 5.0 && ry < 90.0 && hash01((x as f32 / pitch) as i64, panel, 82) < 0.4 {
                c = scale(c, 0.9 + 0.1 * (ry / 90.0));
            }
            albedo.push(c);
            height.data[i] = hh;
            if seam_d < 9.0 {
                roughness.data[i] = 0.5;
            }
        }
    }
    Recipe { albedo, height, roughness, relief: 9.0 }
}

fn painted_metal() -> Recipe {
    // Smooth sage paint for props: a broad two-tone gradient, faint brushing,
    // and a few horizontal scuffs where the paint is rubbed to bare metal.
    let broad = fbm(SIZE, 2, 2, 0.5, 91);
    let brush = noise(SIZE, 2, 128, 92);
    let scuffs = fbm(SIZE, 4, 3, 0.5, 93);
    let band = noise(SIZE, 2, 16, 94);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let tone = smoothstep(0.35, 0.65, broad.data[i]);
        let mut c = scale(lerp3(hex(SAGE[0]), hex(SAGE[1]), tone), 0.985 + brush.data[i] * 0.03);
        let mut r = 0.46;
        let mut h = 0.6;
        let scuff = scuffs.data[i] > 0.66 && band.data[i] > 0.7;
        if scuff {
            c = hex(BARE_METAL);
            r = 0.3;
            h = 0.5;
        } else if scuffs.data[i] > 0.64 && band.data[i] > 0.67 {
            c = scale(c, 1.1);
        }
        albedo.push(c);
        roughness.data[i] = r;
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 3.0 }
}

fn rust() -> Recipe {
    // Rust blooms running down old sage paint: elongated vertical shapes in
    // three tones, with a bright orange rim where rust meets paint.
    let drip = noise(SIZE, 10, 2, 101);
    let bloom = fbm(SIZE, 3, 3, 0.5, 102);
    let core = fbm(SIZE, 6, 2, 0.5, 103);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let amount = drip.data[i] * 0.55 + bloom.data[i] * 0.45;
        let (c, r, h) = if amount > 0.46 {
            let deep = smoothstep(0.55, 0.62, amount + core.data[i] * 0.1);
            let colour = if deep > 0.5 { hex(RUST[0]) } else { hex(RUST[1]) };
            (scale(colour, 0.95 + core.data[i] * 0.1), 0.86, 0.4 - deep * 0.1)
        } else if amount > 0.43 {
            (hex(RUST[2]), 0.7, 0.55)
        } else {
            (scale(hex(SAGE[0]), 0.9), 0.5, 0.6)
        };
        albedo.push(c);
        roughness.data[i] = r;
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 5.0 }
}

fn planks(boards: usize, palette: &[u32], gloss: f32, seed: u32) -> Recipe {
    // Boards along y with butt joints; each board its own tone, a lit top
    // edge, two or three long flowing grain lines and the odd knot.
    let w = SIZE as f32 / boards as f32;
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let col = (x as f32 / w) as i64;
            let pieces = 1 + (hash01(col, 7, seed) * 2.0) as usize;
            let offset = hash01(col, 8, seed) * SIZE as f32;
            let len = SIZE as f32 / pieces as f32;
            let along = (y as f32 + offset).rem_euclid(SIZE as f32);
            let piece = (along / len) as i64;
            let (lx, ly) = (x as f32 - col as f32 * w, along - piece as f32 * len);
            let edge = lx.min(w - lx).min(ly).min(len - ly) - 3.0;
            let id = col * 4 + piece;
            let mut c = scale(pick(palette, id, seed + 3), paint_light(lx, ly, w, len, 8.0, 0.9));
            // Grain lines: smooth waves, a whole number per tile so they wrap.
            let mut grain: f32 = 0.0;
            for k in 0..3 {
                let phase = hash01(id, 10 + k, seed) * std::f32::consts::TAU;
                let centre = w * (0.25 + 0.25 * k as f32)
                    + (along / SIZE as f32 * std::f32::consts::TAU * 2.0 + phase).sin() * w * 0.08;
                grain = grain.max(smoothstep(2.5, 0.5, (lx - centre).abs()));
            }
            c = scale(c, 1.0 - grain * 0.14);
            // A knot on about one board in three: dark eye with a ring.
            if hash01(id, 20, seed) < 0.33 {
                let (kx, ky) = (w * 0.5, len * (0.2 + 0.6 * hash01(id, 21, seed)));
                let d = (((lx - kx) / 1.0).powi(2) + ((ly - ky) / 2.2).powi(2)).sqrt();
                if d < 7.0 {
                    c = scale(c, 0.62);
                } else if d < 10.0 {
                    c = scale(c, 0.85);
                }
            }
            if edge <= 0.0 {
                c = scale(c, 0.42);
            }
            albedo.push(c);
            height.data[i] = bevel(edge, 6.0) * (0.9 + hash01(id, 9, seed) * 0.1) - grain * 0.05;
            roughness.data[i] = gloss + grain * 0.1;
        }
    }
    Recipe { albedo, height, roughness, relief: 9.0 }
}

fn roof() -> Recipe {
    // Membrane sheets, four per tile (75 cm), each overlapping the next with
    // a lit lip and a shadow below it; per-sheet tone, fine granule sheen.
    let sheets = 4.0;
    let granules = fbm(SIZE, 128, 2, 0.6, 111);
    let broad = fbm(SIZE, 2, 2, 0.5, 112);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for y in 0..SIZE {
        let v = (y as f32 / SIZE as f32 * sheets).fract();
        let sheet = (y as f32 / SIZE as f32 * sheets) as i64;
        let lip = smoothstep(0.05, 0.0, v);
        let shadow = smoothstep(0.05, 0.12, v) * smoothstep(0.2, 0.12, v);
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let c = scale(
                pick(&ROOFING, sheet, 113),
                (1.0 + lip * 0.25 - shadow * 0.12) * (0.97 + granules.data[i] * 0.05) * (0.97 + broad.data[i] * 0.06),
            );
            albedo.push(c);
            height.data[i] = lip * 0.7 + (1.0 - shadow) * 0.2 + granules.data[i] * 0.08;
        }
    }
    let roughness = Field::new(SIZE, 0.88);
    Recipe { albedo, height, roughness, relief: 6.0 }
}

fn asphalt() -> Recipe {
    // Controlled dark blue-grey with broad soft variation; light aggregate
    // only in clustered zones, each stone lit from the upper left.
    let broad = fbm(SIZE, 2, 3, 0.5, 21);
    let zones = fbm(SIZE, 4, 2, 0.5, 22);
    let stones = voronoi(SIZE, 56, 0.85, 23);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut roughness = Field::new(SIZE, 0.0);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let mut c = lerp3(hex(ASPHALT[0]), hex(ASPHALT[1]), smoothstep(0.3, 0.7, broad.data[i]));
        let mut h = 0.3;
        let mut r = 0.84;
        let stone = hash01(stones.id[i] as i64, 3, 24);
        if zones.data[i] > 0.55 && stone > 0.6 && stones.f1.data[i] < 0.24 {
            let (ox, oy) = stones.offset[i];
            c = scale(hex(0x5c6169), 1.0 - (ox + oy) * 0.9);
            h += 0.3 * (1.0 - stones.f1.data[i] / 0.24);
            r = 0.72;
        }
        albedo.push(c);
        roughness.data[i] = r;
        height.data[i] = h;
    }
    Recipe { albedo, height, roughness, relief: 4.0 }
}

fn grass() -> Recipe {
    // Painted grass clumps (~30 cm): fans of blades radiating from each
    // clump centre, lit on the upper left, darker gaps between clumps.
    let clumps = voronoi(SIZE, 10, 0.8, 31);
    let broad = fbm(SIZE, 2, 2, 0.5, 32);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let id = clumps.id[i] as i64;
        let (ox, oy) = clumps.offset[i];
        let d = clumps.f1.data[i];
        let angle = oy.atan2(ox);
        let blades = 7.0 + (hash01(id, 1, 33) * 4.0).floor();
        let fan =
            smoothstep(0.55, 0.85, (angle * blades + hash01(id, 2, 33) * std::f32::consts::TAU).cos() * 0.5 + 0.5);
        let dome = 1.0 - smoothstep(0.1, 0.75, d);
        let lit = (-(ox + oy) * 0.9).clamp(-0.4, 0.5);
        let base = if hash01(id, 3, 33) > 0.9 { hex(0x86a245) } else { pick(&GRASS, id, 34) };
        let mut c = scale(base, 1.0 + lit * 0.35);
        c = lerp3(c, hex(GRASS_TIP), fan * dome * 0.55);
        c = lerp3(hex(GRASS_GAP), c, smoothstep(0.0, 0.35, dome + fan * 0.2));
        albedo.push(scale(c, 0.97 + broad.data[i] * 0.06));
        height.data[i] = dome * (0.6 + 0.4 * fan);
    }
    let roughness = Field::new(SIZE, 0.9);
    Recipe { albedo, height, roughness, relief: 8.0 }
}

fn dirt() -> Recipe {
    // Packed earth in two broad tones; pebbles gathered in a few clusters,
    // each with a painted highlight and a contact shadow.
    let broad = fbm(SIZE, 2, 3, 0.5, 41);
    let zones = fbm(SIZE, 3, 2, 0.5, 42);
    let pebbles = voronoi(SIZE, 18, 0.8, 43);
    let mut albedo = Vec::with_capacity(SIZE * SIZE);
    let mut height = Field::new(SIZE, 0.0);
    for i in 0..SIZE * SIZE {
        let tone = smoothstep(0.35, 0.65, broad.data[i]);
        let mut c = lerp3(hex(EARTH[0]), hex(EARTH[1]), tone);
        let mut h = tone * 0.25;
        let id = pebbles.id[i] as i64;
        let radius = 0.24 + hash01(id, 2, 44) * 0.12;
        let d = pebbles.f1.data[i];
        if zones.data[i] > 0.5 && hash01(id, 1, 44) > 0.35 {
            let (ox, oy) = pebbles.offset[i];
            if d < radius {
                let dome = (1.0 - d / radius).sqrt();
                c = scale(pick(&PEBBLES, id, 45), 1.0 - (ox + oy) / radius * 0.22);
                h += dome * 0.6;
            } else if d < radius + 0.07 && oy > 0.0 {
                c = scale(c, 0.72);
            }
        }
        albedo.push(c);
        height.data[i] = h;
    }
    let roughness = Field::new(SIZE, 0.95);
    Recipe { albedo, height, roughness, relief: 7.0 }
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
    let ao = cavity_ao(&height, 6, 1.2).zip(&cavity_ao(&height, 24, 0.8), |a, b| 0.55 + 0.45 * a * b);
    // Hand-painted look: soft occlusion and convex-edge highlights baked into the colour.
    let curvature = height.zip(&height.blurred(2), |h, b| (h - b).max(0.0));
    let albedo: Vec<u8> = recipe
        .albedo
        .iter()
        .enumerate()
        .flat_map(|(i, &c)| {
            let lit = scale(c, 0.85 + 0.15 * ao.data[i] + (curvature.data[i] * 2.0).min(0.08));
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
        ("StylizedSidewalk", || slabs(4, 10.0, 14.0, &SAND, STONE_JOINT, false, 0.82, 201)),
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
        ("StylizedWoodPlanks", || planks(8, &WOOD, 0.62, 301)),
        ("StylizedInteriorFloor", || planks(10, &OAK, 0.38, 311)),
        ("StylizedRoof", roof),
        ("StylizedFloorTiles", || slabs(8, 7.0, 9.0, &[0xe3dccb, 0x9bb3ae], 0xa49d90, true, 0.25, 211)),
    ];

    for (name, recipe) in recipes {
        write(name, recipe(), out, previews)?;
    }
    Ok(())
}
