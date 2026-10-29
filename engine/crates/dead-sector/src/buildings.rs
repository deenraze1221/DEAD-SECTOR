//! Seeded modular buildings. Every building except the store is rebuilt from
//! `sector-assets::modular` pieces: one storey per 3.3 m, exterior walls with
//! windows (and the front door on the ground floor), floor slabs, a
//! switchback stairwell along the back wall, and a roof with a parapet. Each
//! piece is a unit-box instance in the building's own wall material (or the
//! city's floor, glass, wood and concrete materials), and each solid piece is
//! also a collider.

use crate::city::{City, Owner};
use crate::citygen::{Aabb, matrix};
use crate::emit::Emitter;
use glam::{Mat4, Vec3};
use sector_assets::color::parse_hex_color;
use sector_assets::modular::{self, Axis, Opening, Part, Rect, Slot, WallSpec};
use sector_assets::rng::Rng;
use std::collections::HashSet;

pub const STOREY: f32 = 3.3;
pub const WALL_HEIGHT: f32 = 3.12;
pub const THICKNESS: f32 = 0.22;
pub const MAX_FLOORS: u32 = 12;
/// 15 steps of 0.22 m: below the player's 0.3 m step-up.
pub const STEPS: usize = 15;
pub const RISE: f32 = STOREY / STEPS as f32;
pub const TREAD: f32 = 0.28;
pub const FLIGHT_WIDTH: f32 = 1.0;
pub const LANDING: f32 = 1.1;
pub const DOOR_WIDTH: f32 = 1.6;
pub const WINDOW_TOP: f32 = 2.3;
const RAIL: f32 = 0.05;
const RAIL_HEIGHT: f32 = 1.0;
/// The store keeps its exported building.
const STORE: &str = "grocery";
/// Exported shell colliders that the modules replace.
const SHELL_KINDS: [&str; 4] = ["wall", "lintel", "ceiling", "upper"];

/// The layout of one rebuilt building, for interiors and tests.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub building: i64,
    pub floors: u32,
    pub footprint: Rect,
    /// Stairs, holes and landings (empty for one storey); keep furniture and
    /// partitions out of it on every floor.
    pub stairwell: Option<Rect>,
    /// The two stair strips (flights alternate between them floor by floor).
    pub strips: [(f32, f32); 2],
    /// Low end of each flight, by floor: (x, direction).
    pub flights: Vec<(f32, f32)>,
}

impl Plan {
    /// Where furniture must not go on `floor` (stairwell, and the doorway on the ground floor).
    pub fn keepouts(&self, floor: u32) -> Vec<Aabb> {
        let base = floor as f32 * STOREY;
        let boxed = |r: Rect| Aabb::new(Vec3::new(r.min_x, base, r.min_z), Vec3::new(r.max_x, base + 3.0, r.max_z));
        let mut out: Vec<Aabb> = self.stairwell.into_iter().map(boxed).collect();
        if floor == 0 {
            let x = (self.footprint.min_x + self.footprint.max_x) / 2.0;
            let front = self.footprint.max_z;
            out.push(boxed(Rect { min_x: x - 0.9, max_x: x + 0.9, min_z: front - 1.6, max_z: front }));
        }
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub plans: Vec<Plan>,
    /// Module boxes emitted (instances).
    pub pieces: usize,
    pub colliders: usize,
}

struct Materials {
    unit_box: usize,
    floor: usize,
    glass: usize,
    wood: usize,
    concrete: usize,
    roof: usize,
}

impl Materials {
    fn find(city: &City) -> Materials {
        let by_color = |hex: &str| {
            let color = parse_hex_color(hex).unwrap();
            city.materials.iter().position(|m| m.color == color && m.texture.is_none()).unwrap_or(0)
        };
        Materials {
            unit_box: city.geometries.iter().position(|g| g.position.len() == 72 && g.index.len() == 36).unwrap_or(0),
            floor: by_color("#9a9380"),
            glass: by_color("#354d50"),
            wood: by_color("#806448"),
            concrete: by_color("#8b8e7e"),
            roof: by_color("#303c35"),
        }
    }
}

/// Rebuilds every building except the store from modules.
pub fn rebuild(city: &mut City, emit: &mut Emitter, seed: u64) -> Report {
    let materials = Materials::find(city);
    let targets: Vec<usize> = (0..city.buildings.len()).filter(|&i| city.buildings[i].kind != STORE).collect();
    let ids: HashSet<i64> = targets.iter().map(|&i| city.buildings[i].id).collect();
    // Each building keeps its exterior material and tint.
    let looks: Vec<(usize, [f32; 3])> = targets
        .iter()
        .map(|&i| wall_look(city, city.buildings[i].id, materials.unit_box, materials.concrete))
        .collect();
    remove_shells(city, &ids);

    let mut rng = Rng::new(seed ^ 0x424c_4447);
    let mut report = Report::default();
    for (&i, &(wall_material, tint)) in targets.iter().zip(&looks) {
        let b = city.buildings[i].clone();
        let base_floors = (b.h / STOREY).round().max(1.0) as i32;
        let floors = if base_floors >= 2 { base_floors + rng.range_i32(-1, 1) } else { base_floors };
        let floors = floors.clamp(1, MAX_FLOORS as i32) as u32;
        let bay = rng.range_f32(1.8, 2.4);
        let window = rng.range_f32(0.8, 1.15);
        let sill = rng.range_f32(0.85, 1.05);
        let mirror = rng.range_i32(0, 1) == 1;
        let parapet = rng.range_f32(0.5, 1.0);

        let (parts, plan) = building_parts(&b, floors, bay, window, sill, mirror, parapet);
        let owner = Owner::Str(format!("module-{}", b.id));
        for part in &parts {
            let (material, color) = match part.slot {
                Slot::Wall => (wall_material, tint),
                Slot::Glass => (materials.glass, [1.0; 3]),
                Slot::Frame => (materials.wood, [1.0; 3]),
                Slot::Floor => (materials.floor, [1.0; 3]),
                Slot::Stairs => (materials.concrete, [1.0; 3]),
                Slot::Roof => (materials.roof, [1.0; 3]),
            };
            let t = Mat4::from_translation(part.center()) * Mat4::from_scale(part.size());
            emit.instance(city, (materials.unit_box, &[material]), t, color, &owner, "module");
            if part.solid {
                let kind = match part.slot {
                    Slot::Glass => "window",
                    Slot::Stairs => "stairs",
                    Slot::Floor | Slot::Roof => "floor",
                    _ => "wall",
                };
                emit.collider(city, Aabb::new(part.min, part.max), kind, Owner::Int(b.id));
                report.colliders += 1;
            }
        }
        report.pieces += parts.len();
        city.buildings[i].h = floors as f32 * STOREY;
        report.plans.push(plan);
    }
    report
}

/// The exported ground-floor wall's material and tint.
fn wall_look(city: &City, id: i64, unit_box: usize, fallback: usize) -> (usize, [f32; 3]) {
    for batch in city.batches.iter().filter(|b| b.geometry == unit_box && b.materials.len() == 1) {
        for (i, owner) in batch.owners.iter().enumerate() {
            if *owner != Some(Owner::Int(id)) {
                continue;
            }
            let m = matrix(&batch.transforms[i]);
            let size = Vec3::new(m.x_axis.length(), m.y_axis.length(), m.z_axis.length());
            if (2.9..3.2).contains(&size.y)
                && size.x.max(size.z) > 2.5
                && city.materials[batch.materials[0]].texture.is_none()
            {
                let color = if batch.colors.len() > i * 3 + 2 {
                    [batch.colors[i * 3], batch.colors[i * 3 + 1], batch.colors[i * 3 + 2]]
                } else {
                    [1.0; 3]
                };
                return (batch.materials[0], color);
            }
        }
    }
    (fallback, [1.0; 3])
}

/// Drops the exported shells (everything a building owns except its sign).
fn remove_shells(city: &mut City, ids: &HashSet<i64>) {
    let textured: Vec<bool> = city.materials.iter().map(|m| m.texture.is_some()).collect();
    for batch in &mut city.batches {
        let sign = batch.materials.iter().any(|&m| textured[m]);
        if sign {
            continue;
        }
        let count = batch.transforms.len();
        let keep: Vec<bool> = (0..count)
            .map(|i| !matches!(batch.owners.get(i), Some(Some(Owner::Int(id))) if ids.contains(id)))
            .collect();
        if keep.iter().all(|k| *k) {
            continue;
        }
        let mut n = 0;
        for (i, &kept) in keep.iter().enumerate() {
            if !kept {
                continue;
            }
            batch.transforms[n] = batch.transforms[i];
            if !batch.colors.is_empty() {
                batch.colors.copy_within(i * 3..i * 3 + 3, n * 3);
            }
            if i < batch.owners.len() {
                batch.owners.swap(n, i);
            }
            if i < batch.roles.len() {
                batch.roles.swap(n, i);
            }
            n += 1;
        }
        batch.transforms.truncate(n);
        if !batch.colors.is_empty() {
            batch.colors.truncate(n * 3);
        }
        batch.owners.truncate(n);
        batch.roles.truncate(n);
    }
    city.colliders.retain(|c| {
        !(SHELL_KINDS.contains(&c.kind.as_str()) && matches!(&c.owner, Some(Owner::Int(id)) if ids.contains(id)))
    });
}

/// All module pieces of one building, and its plan.
pub fn building_parts(
    b: &crate::city::Building,
    floors: u32,
    bay: f32,
    window: f32,
    sill: f32,
    mirror: bool,
    parapet: f32,
) -> (Vec<Part>, Plan) {
    let t = THICKNESS;
    let footprint =
        Rect { min_x: b.x - b.w / 2.0, max_x: b.x + b.w / 2.0, min_z: b.z - b.d / 2.0, max_z: b.z + b.d / 2.0 };
    let f = footprint;
    // Stairs: two strips along the back wall; flights alternate strips and directions.
    let strip_z = f.min_z + t;
    let strips = [(strip_z, strip_z + FLIGHT_WIDTH), (strip_z + FLIGHT_WIDTH, strip_z + 2.0 * FLIGHT_WIDTH)];
    let x_low = f.min_x + t + LANDING;
    let x_high = x_low + STEPS as f32 * TREAD;
    let flights: Vec<(f32, f32)> = (0..floors.saturating_sub(1))
        .map(|floor| if (floor % 2 == 0) != mirror { (x_low, 1.0) } else { (x_high, -1.0) })
        .collect();
    let stairwell = (floors > 1).then_some(Rect {
        min_x: x_low - LANDING,
        max_x: x_high + LANDING,
        min_z: strip_z,
        max_z: strip_z + 2.0 * FLIGHT_WIDTH + 0.3,
    });

    let mut parts = Vec::new();
    // Walls run the full storey height; slabs sit inside them.
    let spec = |axis, from, to, plane, base| WallSpec {
        axis,
        from,
        to,
        plane,
        thickness: t,
        base,
        height: STOREY,
        sill,
        top: WINDOW_TOP,
    };
    let centers = |from: f32, to: f32, avoid| modular::window_centers(from, to, bay, window, avoid, 0.35);
    let door = (b.x - DOOR_WIDTH / 2.0, b.x + DOOR_WIDTH / 2.0);
    // The back wall is blind where the stairwell runs; side windows never open onto the stairs.
    let back = centers(f.min_x + t, f.max_x - t, stairwell.map(|s| (s.min_x, s.max_x)));
    let side: Vec<f32> = centers(f.min_z + t, f.max_z - t, None)
        .into_iter()
        .filter(|c| stairwell.is_none_or(|s| c - window / 2.0 > s.max_z))
        .collect();
    let front_upper = centers(f.min_x + t, f.max_x - t, None);
    // Front (+Z) and back run the full width; the sides fit between them.
    let walls = [
        (Axis::X, f.min_x, f.max_x, f.max_z - t, &front_upper),
        (Axis::X, f.min_x, f.max_x, f.min_z, &back),
        (Axis::Z, f.min_z + t, f.max_z - t, f.min_x, &side),
        (Axis::Z, f.min_z + t, f.max_z - t, f.max_x - t, &side),
    ];
    for (i, (axis, from, to, plane, upper)) in walls.into_iter().enumerate() {
        // Ground floor: its own wall, with the front door.
        let mut openings: Vec<Opening> = if i == 0 {
            centers(f.min_x + t, f.max_x - t, Some(door))
                .into_iter()
                .map(|center| Opening::Window { center, width: window })
                .collect()
        } else {
            upper.iter().map(|&center| Opening::Window { center, width: window }).collect()
        };
        if i == 0 {
            openings.push(Opening::Door { center: b.x, width: DOOR_WIDTH });
        }
        modular::wall(&spec(axis, from, to, plane, 0.0), &openings, &mut parts);
        // Upper storeys: one stacked facade.
        modular::facade(&spec(axis, from, to, plane, STOREY), STOREY, floors - 1, upper, window, &mut parts);
    }
    let inner = Rect { min_x: f.min_x + t, max_x: f.max_x - t, min_z: f.min_z + t, max_z: f.max_z - t };
    for floor in 0..floors {
        let base = floor as f32 * STOREY;
        // Slab above this floor, open over this floor's flight.
        let top = (floor + 1) as f32 * STOREY;
        let mut holes = Vec::new();
        if let Some(&(start, dir)) = flights.get(floor as usize) {
            let strip = strips[floor as usize % 2];
            let end = start + dir * STEPS as f32 * TREAD;
            holes.push(Rect { min_x: start.min(end), max_x: start.max(end), min_z: strip.0, max_z: strip.1 });
            let flight = modular::Flight {
                axis: Axis::X,
                start,
                dir,
                across: strip,
                base,
                steps: STEPS,
                rise: RISE,
                tread: TREAD,
            };
            modular::stairs(&flight, &mut parts);
            // Guard rails round the hole above: along each long side that is
            // not the back wall, and across the end the flight starts from
            // (the top end stays open).
            let rail = |min: Vec3, max: Vec3| Part::new(min, max, Slot::Frame, true);
            for edge in [strip.0 - RAIL, strip.1] {
                if edge < strip_z {
                    continue;
                }
                parts.extend(rail(
                    Vec3::new(start.min(end), top, edge),
                    Vec3::new(start.max(end), top + RAIL_HEIGHT, edge + RAIL),
                ));
            }
            let x = if dir > 0.0 { start - RAIL } else { start };
            parts.extend(rail(Vec3::new(x, top, strip.0), Vec3::new(x + RAIL, top + RAIL_HEIGHT, strip.1)));
        }
        let slot = if floor + 1 == floors { Slot::Roof } else { Slot::Floor };
        modular::slab(inner, top - (STOREY - WALL_HEIGHT), top, &holes, slot, &mut parts);
    }
    let roof = floors as f32 * STOREY;
    modular::parapet(f, roof, parapet, t, Slot::Wall, &mut parts);
    let plan = Plan { building: b.id, floors, footprint, stairwell, strips, flights };
    (parts, plan)
}
