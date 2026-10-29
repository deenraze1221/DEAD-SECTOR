//! Sector Engine collision: a static world of axis-aligned boxes with a grid
//! broadphase, and a kinematic box mover (sweep, slide, step up) for
//! characters. Pure `f32` arithmetic in a fixed order, so the same inputs give
//! the same result on every machine.

use glam::Vec3;
use std::collections::HashMap;

/// Gap kept between a moving body and what it touches.
pub const SKIN: f32 = 1e-3;
/// A box counts as "ahead" when the body overlaps it by less than this; deeper
/// overlaps are ignored so a body that starts inside something can walk out.
const AHEAD_TOLERANCE: f32 = 0.02;
/// Boxes spanning more cells than this go in an always-checked list.
const MAX_CELLS_PER_BOX: i64 = 256;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Aabb { min, max }
    }

    /// A box standing on `feet`: `half_width` around it on X and Z, `height` up.
    pub fn standing(feet: Vec3, half_width: f32, height: f32) -> Self {
        Aabb::new(feet - Vec3::new(half_width, 0.0, half_width), feet + Vec3::new(half_width, height, half_width))
    }

    pub fn is_valid(&self) -> bool {
        self.min.cmplt(self.max).all() && self.min.is_finite() && self.max.is_finite()
    }

    /// Overlap with positive volume (touching faces do not count).
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.cmplt(other.max).all() && self.max.cmpgt(other.min).all()
    }

    pub fn translated(&self, offset: Vec3) -> Aabb {
        Aabb::new(self.min + offset, self.max + offset)
    }
}

/// Static collision boxes with an XZ grid for queries.
#[derive(Debug, Clone)]
pub struct World {
    boxes: Vec<Aabb>,
    cell: f32,
    grid: HashMap<(i32, i32), Vec<u32>>,
    /// Very large boxes (ground, city walls), checked by every query.
    large: Vec<u32>,
}

/// Result of [`World::move_box`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moved {
    /// How far the body actually moved.
    pub offset: Vec3,
    /// Something stopped downward movement.
    pub grounded: bool,
    /// Something stopped upward movement.
    pub ceiling: bool,
    /// Something stopped horizontal movement (after any step up).
    pub blocked: bool,
}

impl World {
    /// Builds the world; boxes without volume are skipped.
    pub fn new(boxes: impl IntoIterator<Item = Aabb>, cell: f32) -> World {
        let boxes: Vec<Aabb> = boxes.into_iter().filter(Aabb::is_valid).collect();
        let mut world = World { boxes: Vec::new(), cell, grid: HashMap::new(), large: Vec::new() };
        for b in boxes {
            let index = world.boxes.len() as u32;
            world.boxes.push(b);
            let (x0, z0, x1, z1) = world.cells(&b);
            if (x1 - x0 + 1) as i64 * (z1 - z0 + 1) as i64 > MAX_CELLS_PER_BOX {
                world.large.push(index);
                continue;
            }
            for z in z0..=z1 {
                for x in x0..=x1 {
                    world.grid.entry((x, z)).or_default().push(index);
                }
            }
        }
        world
    }

    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    fn cells(&self, b: &Aabb) -> (i32, i32, i32, i32) {
        let cell = |v: f32| (v / self.cell).floor() as i32;
        (cell(b.min.x), cell(b.min.z), cell(b.max.x), cell(b.max.z))
    }

    /// Indices of boxes that may touch `region`, sorted and without duplicates.
    fn candidates(&self, region: &Aabb, out: &mut Vec<u32>) {
        out.clear();
        out.extend_from_slice(&self.large);
        let (x0, z0, x1, z1) = self.cells(region);
        for z in z0..=z1 {
            for x in x0..=x1 {
                if let Some(list) = self.grid.get(&(x, z)) {
                    out.extend_from_slice(list);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    /// Whether `body` overlaps any box.
    pub fn overlaps(&self, body: &Aabb) -> bool {
        let mut scratch = Vec::new();
        self.candidates(body, &mut scratch);
        scratch.iter().any(|&i| self.boxes[i as usize].intersects(body))
    }

    /// How far `body` can move by `distance` along `axis` before touching a box.
    fn sweep_axis(&self, body: &Aabb, axis: usize, distance: f32, scratch: &mut Vec<u32>) -> f32 {
        if distance == 0.0 {
            return 0.0;
        }
        let mut region = *body;
        if distance > 0.0 {
            region.max[axis] += distance;
        } else {
            region.min[axis] += distance;
        }
        self.candidates(&region, scratch);
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        let mut allowed = distance;
        for &i in scratch.iter() {
            let c = &self.boxes[i as usize];
            let beside =
                body.min[a] < c.max[a] && body.max[a] > c.min[a] && body.min[b] < c.max[b] && body.max[b] > c.min[b];
            if !beside {
                continue;
            }
            if distance > 0.0 {
                if body.max[axis] <= c.min[axis] + AHEAD_TOLERANCE && c.min[axis] < region.max[axis] {
                    allowed = allowed.min((c.min[axis] - body.max[axis] - SKIN).max(0.0));
                }
            } else if body.min[axis] >= c.max[axis] - AHEAD_TOLERANCE && c.max[axis] > region.min[axis] {
                allowed = allowed.max((c.max[axis] - body.min[axis] + SKIN).min(0.0));
            }
        }
        allowed
    }

    /// Moves on X then Z, sliding along whatever blocks one axis.
    fn slide_horizontal(&self, body: &mut Aabb, delta: Vec3, scratch: &mut Vec<u32>) -> bool {
        let mut blocked = false;
        for axis in [0, 2] {
            let moved = self.sweep_axis(body, axis, delta[axis], scratch);
            blocked |= moved != delta[axis];
            let mut offset = Vec3::ZERO;
            offset[axis] = moved;
            *body = body.translated(offset);
        }
        blocked
    }

    /// Kinematic move: vertical first, then horizontal with sliding. When the
    /// body stands on something and is blocked horizontally, it also tries to
    /// climb onto ledges up to `step_height` high and keeps whichever attempt
    /// got further.
    pub fn move_box(&self, body: Aabb, delta: Vec3, step_height: f32) -> Moved {
        let mut scratch = Vec::new();
        let vertical = self.sweep_axis(&body, 1, delta.y, &mut scratch);
        let grounded = delta.y < 0.0 && vertical > delta.y;
        let ceiling = delta.y > 0.0 && vertical < delta.y;
        let start = body.translated(Vec3::Y * vertical);

        let mut flat = start;
        let mut blocked = self.slide_horizontal(&mut flat, delta, &mut scratch);
        let mut end = flat;
        if blocked && grounded && step_height > 0.0 {
            let up = self.sweep_axis(&start, 1, step_height, &mut scratch);
            let mut raised = start.translated(Vec3::Y * up);
            let raised_blocked = self.slide_horizontal(&mut raised, delta, &mut scratch);
            let down = self.sweep_axis(&raised, 1, -(up + SKIN * 2.0), &mut scratch);
            let landed = down > -(up + SKIN * 2.0);
            raised = raised.translated(Vec3::Y * down);
            let progress = |b: &Aabb| {
                let d = b.min - start.min;
                d.x * d.x + d.z * d.z
            };
            if landed && progress(&raised) > progress(&flat) + 1e-8 {
                end = raised;
                blocked = raised_blocked;
            }
        }
        Moved { offset: end.min - body.min, grounded, ceiling, blocked }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground() -> Aabb {
        Aabb::new(Vec3::new(-100.0, -0.2, -100.0), Vec3::new(100.0, 0.0, 100.0))
    }

    fn body_at(x: f32, y: f32, z: f32) -> Aabb {
        Aabb::standing(Vec3::new(x, y, z), 0.32, 1.8)
    }

    #[test]
    fn lands_on_the_ground() {
        let world = World::new([ground()], 4.0);
        let moved = world.move_box(body_at(0.0, 1.0, 0.0), Vec3::new(0.0, -5.0, 0.0), 0.0);
        assert!(moved.grounded);
        assert!((moved.offset.y + 1.0 - SKIN).abs() < 1e-5, "{moved:?}");
    }

    #[test]
    fn wall_stops_and_slides() {
        let wall = Aabb::new(Vec3::new(1.0, 0.0, -10.0), Vec3::new(1.2, 3.0, 10.0));
        let world = World::new([ground(), wall], 4.0);
        let body = body_at(0.0, SKIN, 0.0);
        let moved = world.move_box(body, Vec3::new(2.0, 0.0, 1.0), 0.3);
        assert!(moved.blocked);
        assert!((body.max.x + moved.offset.x - (1.0 - SKIN)).abs() < 1e-5);
        assert_eq!(moved.offset.z, 1.0, "slides along the wall");
        assert!(!world.overlaps(&body.translated(moved.offset)));
    }

    #[test]
    fn fast_moves_do_not_tunnel() {
        let thin = Aabb::new(Vec3::new(5.0, 0.0, -1.0), Vec3::new(5.05, 3.0, 1.0));
        let world = World::new([ground(), thin], 4.0);
        let moved = world.move_box(body_at(0.0, SKIN, 0.0), Vec3::new(50.0, 0.0, 0.0), 0.0);
        assert!(moved.offset.x < 5.0);
    }

    #[test]
    fn steps_onto_low_ledges_only() {
        let curb = Aabb::new(Vec3::new(1.0, 0.0, -5.0), Vec3::new(3.0, 0.2, 5.0));
        let ledge = Aabb::new(Vec3::new(1.0, 0.0, -5.0), Vec3::new(3.0, 0.6, 5.0));
        for (step, climbs) in [(curb, true), (ledge, false)] {
            let world = World::new([ground(), step], 4.0);
            let body = body_at(0.0, SKIN, 0.0);
            // Standing: a small downward probe touches the ground.
            let moved = world.move_box(body, Vec3::new(1.0, -0.01, 0.0), 0.3);
            assert_eq!(moved.offset.x > 0.9, climbs, "{moved:?}");
            assert!(!world.overlaps(&body.translated(moved.offset)));
        }
    }

    #[test]
    fn ceiling_stops_jumps() {
        let roof = Aabb::new(Vec3::new(-2.0, 2.0, -2.0), Vec3::new(2.0, 2.2, 2.0));
        let world = World::new([ground(), roof], 4.0);
        let moved = world.move_box(body_at(0.0, SKIN, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.0);
        assert!(moved.ceiling && moved.offset.y < 0.2);
    }

    #[test]
    fn starting_inside_a_box_can_walk_out() {
        let crate_box = Aabb::new(Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5));
        let world = World::new([ground(), crate_box], 4.0);
        let moved = world.move_box(body_at(0.0, SKIN, 0.0), Vec3::new(2.0, 0.0, 0.0), 0.0);
        assert_eq!(moved.offset.x, 2.0);
    }

    #[test]
    fn grid_and_large_boxes_agree_with_brute_force() {
        let mut boxes = vec![ground()];
        for i in 0..200 {
            let x = (i % 20) as f32 * 3.1 - 30.0;
            let z = (i / 20) as f32 * 2.7 - 15.0;
            boxes.push(Aabb::new(Vec3::new(x, 0.0, z), Vec3::new(x + 1.3, 1.0 + (i % 3) as f32, z + 0.9)));
        }
        let world = World::new(boxes.clone(), 4.0);
        for i in 0..400 {
            let probe = body_at((i % 40) as f32 * 1.7 - 34.0, 0.5, (i / 40) as f32 * 3.3 - 16.0);
            assert_eq!(world.overlaps(&probe), boxes.iter().any(|b| b.intersects(&probe)), "probe {i}");
        }
    }

    #[test]
    fn skips_boxes_without_volume() {
        let flat = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 1.0));
        assert_eq!(World::new([flat, ground()], 4.0).len(), 1);
    }
}
