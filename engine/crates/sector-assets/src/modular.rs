//! Box-based building modules: walls with door and window openings, floor
//! slabs with holes, stair flights and roof parapets. Each module emits
//! axis-aligned boxes tagged with a material slot and whether they collide, so
//! a game can draw them as instances of one unit-box mesh (one draw per
//! material) and use the same boxes for collision.

use glam::Vec3;

/// What a box is made of; the game maps slots to its materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Wall,
    Glass,
    Frame,
    Floor,
    Stairs,
    Roof,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    pub min: Vec3,
    pub max: Vec3,
    pub slot: Slot,
    /// Blocks movement (frames and trims do not).
    pub solid: bool,
}

impl Part {
    /// A box from `min` to `max`, or `None` when it has no volume.
    pub fn new(min: Vec3, max: Vec3, slot: Slot, solid: bool) -> Option<Part> {
        (min.cmplt(max).all() && (max - min).min_element() > 1e-4).then_some(Part { min, max, slot, solid })
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Z,
}

/// A straight wall running along `axis` from `from` to `to`, occupying
/// `plane..plane + thickness` across it and `base..base + height` vertically.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallSpec {
    pub axis: Axis,
    pub from: f32,
    pub to: f32,
    pub plane: f32,
    pub thickness: f32,
    pub base: f32,
    pub height: f32,
    /// Window band, relative to `base`: openings span `sill..top`; doors span `0..top`.
    pub sill: f32,
    pub top: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opening {
    Window { center: f32, width: f32 },
    Door { center: f32, width: f32 },
}

impl Opening {
    fn span(&self) -> (f32, f32) {
        let (Opening::Window { center, width } | Opening::Door { center, width }) = *self;
        (center - width / 2.0, center + width / 2.0)
    }
}

impl WallSpec {
    /// A box spanning `a..b` along the wall, `y0..y1` above the base, and
    /// `c0..c1` across (fractions of the thickness).
    fn piece(
        &self,
        (a, b): (f32, f32),
        (y0, y1): (f32, f32),
        (c0, c1): (f32, f32),
        slot: Slot,
        solid: bool,
    ) -> Option<Part> {
        let across = (self.plane + self.thickness * c0, self.plane + self.thickness * c1);
        let (y0, y1) = (self.base + y0, self.base + y1);
        let (min, max) = match self.axis {
            Axis::X => (Vec3::new(a, y0, across.0), Vec3::new(b, y1, across.1)),
            Axis::Z => (Vec3::new(across.0, y0, a), Vec3::new(across.1, y1, b)),
        };
        Part::new(min, max, slot, solid)
    }
}

/// Emits a wall with openings, banded to keep the box count low: one strip
/// below the window band (split only at doors), one strip above it, piers
/// between openings, and one glass strip per run of windows. Openings must not
/// overlap and must lie inside the wall.
pub fn wall(spec: &WallSpec, openings: &[Opening], out: &mut Vec<Part>) {
    let mut openings = openings.to_vec();
    openings.sort_by(|a, b| a.span().0.total_cmp(&b.span().0));
    let (sill, top, height) = (spec.sill, spec.top, spec.height);
    // Above the band.
    out.extend(spec.piece((spec.from, spec.to), (top, height), (0.0, 1.0), Slot::Wall, true));
    // Below the band, interrupted by doors.
    let mut cursor = spec.from;
    for o in &openings {
        if let Opening::Door { .. } = o {
            let (a, b) = o.span();
            out.extend(spec.piece((cursor, a), (0.0, sill), (0.0, 1.0), Slot::Wall, true));
            out.extend(spec.piece((a - 0.06, a), (0.0, top), (-0.25, 1.25), Slot::Frame, false));
            out.extend(spec.piece((b, b + 0.06), (0.0, top), (-0.25, 1.25), Slot::Frame, false));
            out.extend(spec.piece((a - 0.06, b + 0.06), (top, top + 0.06), (-0.25, 1.25), Slot::Frame, false));
            cursor = b;
        }
    }
    out.extend(spec.piece((cursor, spec.to), (0.0, sill), (0.0, 1.0), Slot::Wall, true));
    // Piers in the band, and glass behind each run of consecutive windows.
    let mut cursor = spec.from;
    let mut glass_from: Option<f32> = None;
    for o in &openings {
        let (a, b) = o.span();
        out.extend(spec.piece((cursor, a), (sill, top), (0.0, 1.0), Slot::Wall, true));
        match o {
            Opening::Window { .. } => {
                glass_from.get_or_insert(a);
                // Sill ledge, outside.
                out.extend(spec.piece((a - 0.05, b + 0.05), (sill - 0.06, sill), (-0.3, 0.5), Slot::Frame, false));
            }
            Opening::Door { .. } => {
                if let Some(g) = glass_from.take() {
                    out.extend(spec.piece((g, cursor), (sill, top), (0.35, 0.65), Slot::Glass, true));
                }
            }
        }
        cursor = b;
    }
    if let Some(g) = glass_from {
        out.extend(spec.piece((g, cursor), (sill, top), (0.35, 0.65), Slot::Glass, true));
    }
    out.extend(spec.piece((cursor, spec.to), (sill, top), (0.0, 1.0), Slot::Wall, true));
}

/// Several identical storeys of one wall, stacked from `spec.base`, each
/// `storey` high with windows at `windows` (centres, all `width` wide) in the
/// `spec.sill..spec.top` band of every storey. Emitted as full-height piers
/// between the window columns, one spandrel strip per floor line, and one
/// glass strip and sill ledge per storey, so the box count barely grows with
/// height. Spandrels overlap the piers inside the wall.
pub fn facade(spec: &WallSpec, storey: f32, storeys: u32, windows: &[f32], width: f32, out: &mut Vec<Part>) {
    if storeys == 0 {
        return;
    }
    let total = storey * storeys as f32;
    let mut centers = windows.to_vec();
    centers.sort_by(f32::total_cmp);
    // Piers: full height, between window columns.
    let mut cursor = spec.from;
    for c in &centers {
        out.extend(spec.piece((cursor, c - width / 2.0), (0.0, total), (0.0, 1.0), Slot::Wall, true));
        cursor = c + width / 2.0;
    }
    out.extend(spec.piece((cursor, spec.to), (0.0, total), (0.0, 1.0), Slot::Wall, true));
    if centers.is_empty() {
        return;
    }
    let run = (centers[0] - width / 2.0, centers[centers.len() - 1] + width / 2.0);
    // Spandrels: below the first band, between bands, above the last band.
    let mut below = 0.0;
    for floor in 0..storeys {
        let y = floor as f32 * storey;
        out.extend(spec.piece((spec.from, spec.to), (below, y + spec.sill), (0.0, 1.0), Slot::Wall, true));
        out.extend(spec.piece(run, (y + spec.sill, y + spec.top), (0.35, 0.65), Slot::Glass, true));
        out.extend(spec.piece(
            (run.0 - 0.05, run.1 + 0.05),
            (y + spec.sill - 0.06, y + spec.sill),
            (-0.3, 0.5),
            Slot::Frame,
            false,
        ));
        below = y + spec.top;
    }
    out.extend(spec.piece((spec.from, spec.to), (below, total), (0.0, 1.0), Slot::Wall, true));
}

/// Evenly spaced window centres along `from..to`, about `bay` apart, skipping
/// any window that would come within `margin` of `avoid` (a door span).
pub fn window_centers(from: f32, to: f32, bay: f32, width: f32, avoid: Option<(f32, f32)>, margin: f32) -> Vec<f32> {
    let length = to - from;
    let count = ((length - 0.6) / bay).floor().max(0.0) as usize;
    let start = from + (length - count as f32 * bay) / 2.0 + bay / 2.0;
    (0..count)
        .map(|i| start + i as f32 * bay)
        .filter(|c| avoid.is_none_or(|(a, b)| c + width / 2.0 + margin <= a || c - width / 2.0 - margin >= b))
        .collect()
}

/// A horizontal rectangle on the XZ plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Rect {
    pub fn area(&self) -> f32 {
        (self.max_x - self.min_x).max(0.0) * (self.max_z - self.min_z).max(0.0)
    }
}

/// A slab covering `rect` from `y0` to `y1`, minus `holes`, as few boxes as a
/// column split allows.
pub fn slab(rect: Rect, y0: f32, y1: f32, holes: &[Rect], slot: Slot, out: &mut Vec<Part>) {
    let mut xs = vec![rect.min_x, rect.max_x];
    for h in holes {
        xs.extend([h.min_x.clamp(rect.min_x, rect.max_x), h.max_x.clamp(rect.min_x, rect.max_x)]);
    }
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    for column in xs.windows(2) {
        let (x0, x1) = (column[0], column[1]);
        let mid = (x0 + x1) / 2.0;
        let mut cuts: Vec<(f32, f32)> = holes
            .iter()
            .filter(|h| h.min_x < mid && h.max_x > mid)
            .map(|h| (h.min_z.max(rect.min_z), h.max_z.min(rect.max_z)))
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut z = rect.min_z;
        for (a, b) in cuts {
            out.extend(Part::new(Vec3::new(x0, y0, z), Vec3::new(x1, y1, a), slot, true));
            z = z.max(b);
        }
        out.extend(Part::new(Vec3::new(x0, y0, z), Vec3::new(x1, y1, rect.max_z), slot, true));
    }
}

/// A straight flight of `steps` solid steps rising `rise` each, `tread` deep,
/// starting at `start` (the low end) and climbing along `axis` in direction
/// `dir` (+1 or −1). The flight spans `across.0..across.1` sideways. Each step
/// is a block from `base` up to its tread, so it can be walked on with a
/// step-up of `rise`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flight {
    pub axis: Axis,
    pub start: f32,
    pub dir: f32,
    pub across: (f32, f32),
    pub base: f32,
    pub steps: usize,
    pub rise: f32,
    pub tread: f32,
}

impl Flight {
    /// Where the flight ends along its axis.
    pub fn end(&self) -> f32 {
        self.start + self.dir * self.tread * self.steps as f32
    }
}

pub fn stairs(flight: &Flight, out: &mut Vec<Part>) {
    let f = flight;
    for i in 0..f.steps {
        let a = f.start + f.dir * f.tread * i as f32;
        let b = a + f.dir * f.tread;
        let (lo, hi) = (a.min(b), a.max(b));
        let y1 = f.base + f.rise * (i + 1) as f32;
        let (min, max) = match f.axis {
            Axis::X => (Vec3::new(lo, f.base, f.across.0), Vec3::new(hi, y1, f.across.1)),
            Axis::Z => (Vec3::new(f.across.0, f.base, lo), Vec3::new(f.across.1, y1, hi)),
        };
        out.extend(Part::new(min, max, Slot::Stairs, true));
    }
}

/// A low wall around the edge of a roof `rect`, from `base` up `height`.
pub fn parapet(rect: Rect, base: f32, height: f32, thickness: f32, slot: Slot, out: &mut Vec<Part>) {
    let (y0, y1) = (base, base + height);
    let t = thickness;
    let r = rect;
    for (min, max) in [
        (Vec3::new(r.min_x, y0, r.min_z), Vec3::new(r.max_x, y1, r.min_z + t)),
        (Vec3::new(r.min_x, y0, r.max_z - t), Vec3::new(r.max_x, y1, r.max_z)),
        (Vec3::new(r.min_x, y0, r.min_z + t), Vec3::new(r.min_x + t, y1, r.max_z - t)),
        (Vec3::new(r.max_x - t, y0, r.min_z + t), Vec3::new(r.max_x, y1, r.max_z - t)),
    ] {
        out.extend(Part::new(min, max, slot, true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(parts: &[Part], slot: Slot) -> f32 {
        parts.iter().filter(|p| p.slot == slot).map(|p| p.size().x * p.size().y * p.size().z).sum()
    }

    fn overlap(a: &Part, b: &Part) -> bool {
        a.min.cmplt(b.max).all() && a.max.cmpgt(b.min).all()
    }

    fn spec() -> WallSpec {
        WallSpec {
            axis: Axis::X,
            from: 0.0,
            to: 10.0,
            plane: 0.0,
            thickness: 0.2,
            base: 3.3,
            height: 3.12,
            sill: 0.9,
            top: 2.3,
        }
    }

    #[test]
    fn wall_without_openings_is_solid() {
        let mut parts = Vec::new();
        wall(&spec(), &[], &mut parts);
        assert!((volume(&parts, Slot::Wall) - 10.0 * 3.12 * 0.2).abs() < 1e-3);
        assert!(parts.iter().all(|p| p.min.y >= 3.3 - 1e-5 && p.max.y <= 6.42 + 1e-5));
    }

    #[test]
    fn openings_are_cut_and_walls_do_not_overlap() {
        let mut parts = Vec::new();
        let openings = [
            Opening::Window { center: 2.0, width: 1.0 },
            Opening::Door { center: 5.0, width: 1.6 },
            Opening::Window { center: 8.0, width: 1.0 },
        ];
        wall(&spec(), &openings, &mut parts);
        let walls: Vec<&Part> = parts.iter().filter(|p| p.slot == Slot::Wall).collect();
        for (i, a) in walls.iter().enumerate() {
            for b in &walls[i + 1..] {
                assert!(!overlap(a, b), "{a:?} {b:?}");
            }
        }
        // Wall volume = full wall minus window band holes minus the door hole.
        let expected = 10.0 * 3.12 * 0.2 - 2.0 * (1.0 * 1.4 * 0.2) - 1.6 * 2.3 * 0.2;
        assert!((volume(&parts, Slot::Wall) - expected).abs() < 1e-3, "{}", volume(&parts, Slot::Wall));
        // The doorway is clear of solid boxes; windows are glazed.
        let door =
            Part { min: Vec3::new(4.21, 3.31, -0.5), max: Vec3::new(5.79, 5.59, 0.7), slot: Slot::Wall, solid: true };
        assert!(parts.iter().filter(|p| p.solid).all(|p| !overlap(p, &door)));
        assert_eq!(parts.iter().filter(|p| p.slot == Slot::Glass).count(), 2);
    }

    #[test]
    fn facade_leaves_only_the_windows_open() {
        let spec = WallSpec { height: 3.3, base: 3.3, ..spec() };
        let windows = [2.0, 5.0, 8.0];
        let mut parts = Vec::new();
        facade(&spec, 3.3, 4, &windows, 1.0, &mut parts);
        // Far fewer boxes than four separate storeys of `wall`.
        let mut separate = Vec::new();
        for floor in 0..4 {
            let s = WallSpec { base: 3.3 + floor as f32 * 3.3, ..spec };
            wall(&s, &windows.map(|center| Opening::Window { center, width: 1.0 }), &mut separate);
        }
        assert!(parts.len() * 2 < separate.len(), "{} vs {}", parts.len(), separate.len());
        // Sample the wall face: solid wall everywhere except inside window openings.
        for i in 0..100 {
            for j in 0..132 {
                let (a, y) = (0.05 + i as f32 * 0.1, 3.3 + 0.05 + j as f32 * 0.1);
                let floor_y = (y - 3.3) % 3.3;
                let in_window = windows.iter().any(|c| (a - c).abs() < 0.5) && floor_y > 0.9 && floor_y < 2.3;
                let p = Vec3::new(a, y, 0.1);
                let walled =
                    parts.iter().filter(|p| p.slot == Slot::Wall).any(|w| w.min.cmple(p).all() && w.max.cmpge(p).all());
                assert_eq!(walled, !in_window, "at {a} {y}");
            }
        }
        assert_eq!(parts.iter().filter(|p| p.slot == Slot::Glass).count(), 4);
        assert!(parts.iter().all(|p| p.min.y >= 3.3 - 1e-4 && p.max.y <= 3.3 + 13.2 + 1e-4));
    }

    #[test]
    fn window_centers_skip_the_door() {
        let centers = window_centers(0.0, 9.0, 2.0, 1.0, Some((3.7, 5.3)), 0.3);
        assert!(centers.iter().all(|c| c + 0.8 <= 3.7 || c - 0.8 >= 5.3), "{centers:?}");
        assert_eq!(window_centers(0.0, 9.0, 2.0, 1.0, None, 0.3).len(), 4);
    }

    #[test]
    fn slab_area_excludes_holes() {
        let rect = Rect { min_x: 0.0, max_x: 9.0, min_z: 0.0, max_z: 8.0 };
        let holes = [
            Rect { min_x: 1.0, max_x: 5.0, min_z: 0.0, max_z: 1.0 },
            Rect { min_x: 4.0, max_x: 6.0, min_z: 3.0, max_z: 4.0 },
        ];
        let mut parts = Vec::new();
        slab(rect, 3.12, 3.3, &holes, Slot::Floor, &mut parts);
        let area: f32 = parts.iter().map(|p| p.size().x * p.size().z).sum();
        assert!((area - (72.0 - 4.0 - 2.0)).abs() < 1e-3, "{area}");
        for p in &parts {
            for h in &holes {
                assert!(!(p.min.x < h.max_x && p.max.x > h.min_x && p.min.z < h.max_z && p.max.z > h.min_z));
            }
        }
    }

    #[test]
    fn stairs_climb_one_storey() {
        let mut parts = Vec::new();
        let flight = Flight {
            axis: Axis::X,
            start: 5.0,
            dir: -1.0,
            across: (0.0, 1.0),
            base: 3.3,
            steps: 15,
            rise: 0.22,
            tread: 0.28,
        };
        stairs(&flight, &mut parts);
        assert!((flight.end() - (5.0 - 15.0 * 0.28)).abs() < 1e-4);
        assert_eq!(parts.len(), 15);
        assert!((parts.last().unwrap().max.y - 6.6).abs() < 1e-4);
        assert!((parts.last().unwrap().min.x - (5.0 - 15.0 * 0.28)).abs() < 1e-4);
        for pair in parts.windows(2) {
            assert!((pair[1].max.y - pair[0].max.y - 0.22).abs() < 1e-5);
        }
    }

    #[test]
    fn parapet_rings_the_roof() {
        let mut parts = Vec::new();
        parapet(Rect { min_x: 0.0, max_x: 9.0, min_z: 0.0, max_z: 8.0 }, 10.0, 0.8, 0.2, Slot::Wall, &mut parts);
        assert_eq!(parts.len(), 4);
        for (i, a) in parts.iter().enumerate() {
            for b in &parts[i + 1..] {
                assert!(!overlap(a, b));
            }
        }
    }
}
