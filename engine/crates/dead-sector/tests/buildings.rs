//! Modular buildings in seeded cities: storeys connect, stairs can be climbed,
//! walls enclose every floor, modules have matching colliders, and generated
//! content adds no draw calls.

use dead_sector::buildings::{FLIGHT_WIDTH, Plan, STEPS, STOREY, TREAD, WALL_HEIGHT};
use dead_sector::city::{City, Owner};
use dead_sector::citygen::{self, Aabb};
use dead_sector::content::{build_scene, locate_assets};
use dead_sector::player::{self, Input, Player};
use glam::Vec3;
use std::collections::BTreeMap;
use std::f32::consts::{FRAC_PI_2, PI};
use std::path::PathBuf;
use std::sync::OnceLock;

fn exported() -> &'static City {
    static CITY: OnceLock<City> = OnceLock::new();
    CITY.get_or_init(|| {
        let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets");
        City::load(&locate_assets(Some(&checkout)).unwrap().join("city.json")).unwrap()
    })
}

fn seeded(seed: u64) -> &'static (City, citygen::Report) {
    static CITIES: OnceLock<std::sync::Mutex<BTreeMap<u64, &'static (City, citygen::Report)>>> = OnceLock::new();
    let cache = CITIES.get_or_init(Default::default);
    let mut cache = cache.lock().unwrap();
    cache.entry(seed).or_insert_with(|| {
        let mut city = exported().clone();
        let report = citygen::generate(&mut city, seed);
        Box::leak(Box::new((city, report)))
    })
}

/// Yaw that faces +X (dir > 0) or −X.
fn yaw_x(dir: f32) -> f32 {
    if dir > 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 }
}

fn walk(p: &mut Player, world: &sector_physics::World, yaw: f32, ticks: usize) {
    p.yaw = yaw;
    for _ in 0..ticks {
        p.step(world, Input { forward: 1.0, ..Default::default() });
        assert!(!world.overlaps(&p.body()), "inside a collider at {:?}", p.feet);
    }
}

#[test]
fn every_flight_climbs_one_storey() {
    for seed in [7, 1234] {
        let (city, report) = seeded(seed);
        let world = player::collision_world(city);
        let mut flights = 0;
        for plan in &report.buildings.plans {
            for (floor, &(start, dir)) in plan.flights.iter().enumerate() {
                let strip = plan.strips[floor % 2];
                let base = floor as f32 * STOREY;
                // Stand on the landing just before the first step, facing up the flight.
                let feet = Vec3::new(start - dir * 0.6, base + 0.01, (strip.0 + strip.1) / 2.0);
                let mut p = Player::new(feet);
                walk(&mut p, &world, yaw_x(dir), 150);
                let top = base + STOREY;
                assert!(
                    (p.feet.y - top).abs() < 0.05 && p.grounded,
                    "seed {seed} building {} floor {floor}: ended at {:?}, top {top}",
                    plan.building,
                    p.feet
                );
                flights += 1;
            }
        }
        assert!(flights > 150, "{flights} flights");
    }
}

#[test]
fn walls_enclose_upper_floors_and_the_door_leads_out() {
    let (city, report) = seeded(7);
    let world = player::collision_world(city);
    for plan in &report.buildings.plans {
        let f = plan.footprint;
        let center = |y: f32| Vec3::new((f.min_x + f.max_x) / 2.0, y, (f.min_z + f.max_z) / 2.0 + 1.5);
        for floor in 1..plan.floors {
            for yaw in [0.0, FRAC_PI_2, PI, -FRAC_PI_2] {
                let mut p = Player::new(center(floor as f32 * STOREY + 0.01));
                if world.overlaps(&p.body()) {
                    continue; // furniture at the centre
                }
                walk(&mut p, &world, yaw, 240);
                let inside = p.feet.x > f.min_x && p.feet.x < f.max_x && p.feet.z > f.min_z && p.feet.z < f.max_z;
                assert!(inside, "building {} floor {floor}: walked out to {:?}", plan.building, p.feet);
                // At most a step down onto the top of the stairs below, never a fall.
                assert!(p.feet.y > floor as f32 * STOREY - 0.5, "fell through floor {floor}: {:?}", p.feet);
            }
        }
        // Ground floor: straight out of the front door (the doorway is kept clear).
        let x = (f.min_x + f.max_x) / 2.0;
        let mut p = Player::new(Vec3::new(x, 0.01, f.max_z - 1.2));
        walk(&mut p, &world, PI, 120);
        assert!(p.feet.z > f.max_z + 1.0, "building {}: could not leave by the door, at {:?}", plan.building, p.feet);
    }
}

fn module_parts(city: &City, id: i64) -> Vec<(Aabb, usize)> {
    let owner = Some(Owner::Str(format!("module-{id}")));
    let mut parts = Vec::new();
    for batch in &city.batches {
        for (i, o) in batch.owners.iter().enumerate() {
            if *o == owner {
                let m = citygen::matrix(&batch.transforms[i]);
                let half = Vec3::new(m.x_axis.length(), m.y_axis.length(), m.z_axis.length()) / 2.0;
                let c = m.w_axis.truncate();
                parts.push((Aabb::new(c - half, c + half), batch.materials[0]));
            }
        }
    }
    parts
}

#[test]
fn modules_connect_without_overlap_and_collide() {
    let (city, report) = seeded(7);
    let glass = city
        .materials
        .iter()
        .position(|m| m.color == sector_assets::color::parse_hex_color("#354d50").unwrap())
        .unwrap();
    let mut solid_colliders = 0;
    for plan in &report.buildings.plans {
        let colliders: Vec<Aabb> = city
            .colliders
            .iter()
            .filter(|c| {
                c.owner == Some(Owner::Int(plan.building))
                    && ["wall", "window", "floor", "stairs"].contains(&c.kind.as_str())
            })
            .map(|c| Aabb::new(Vec3::new(c.min_x, c.min_y, c.min_z), Vec3::new(c.max_x, c.max_y, c.max_z)))
            .collect();
        solid_colliders += colliders.len();
        // Structural boxes never overlap each other, except inside a wall
        // (glass and spandrel strips cross the piers of stacked facades).
        let f = plan.footprint;
        let t = dead_sector::buildings::THICKNESS;
        let in_wall = |c: &Aabb| {
            c.max.x <= f.min_x + t + 1e-4
                || c.min.x >= f.max_x - t - 1e-4
                || c.max.z <= f.min_z + t + 1e-4
                || c.min.z >= f.max_z - t - 1e-4
        };
        let structure: Vec<&Aabb> = colliders.iter().filter(|c| c.size().min_element() > 0.07).collect();
        for (i, a) in structure.iter().enumerate() {
            for b in &structure[i + 1..] {
                let overlap =
                    a.min.cmplt(b.max - 1e-4).all() && a.max.cmpgt(b.min + 1e-4).all() && !(in_wall(a) && in_wall(b));
                assert!(!overlap, "building {}: {a:?} overlaps {b:?}", plan.building);
            }
        }
        // Every collider is one of the drawn module boxes.
        let drawn = module_parts(city, plan.building);
        for c in &colliders {
            assert!(
                drawn
                    .iter()
                    .any(|(d, _)| (d.min - c.min).abs().max_element() < 1e-3
                        && (d.max - c.max).abs().max_element() < 1e-3),
                "building {}: collider {c:?} has no drawn box",
                plan.building
            );
        }
        assert!(drawn.iter().any(|(_, m)| *m == glass), "building {} has no windows", plan.building);
        // Each storey has a floor (slab) at its top, covering the footprint except the stair holes.
        let f = plan.footprint;
        for floor in 1..=plan.floors {
            let top = floor as f32 * STOREY;
            let area: f32 = colliders
                .iter()
                .filter(|c| (c.max.y - top).abs() < 1e-3 && (c.min.y - (top - (STOREY - WALL_HEIGHT))).abs() < 1e-3)
                .map(|c| c.size().x * c.size().z)
                .sum();
            let t = dead_sector::buildings::THICKNESS;
            let full = (f.max_x - f.min_x - 2.0 * t) * (f.max_z - f.min_z - 2.0 * t);
            let hole = if floor < plan.floors { STEPS as f32 * TREAD * FLIGHT_WIDTH } else { 0.0 };
            assert!(
                (area - (full - hole)).abs() < 0.05,
                "building {} level {floor}: slab {area} vs {}",
                plan.building,
                full - hole
            );
        }
        // The building is as tall as its storeys.
        let b = city.buildings.iter().find(|b| b.id == plan.building).unwrap();
        assert_eq!(b.h, plan.floors as f32 * STOREY);
    }
    assert_eq!(solid_colliders, report.buildings.colliders);
}

#[test]
fn stairwells_stay_clear_of_partitions_and_furniture() {
    let (city, report) = seeded(7);
    for plan in report.buildings.plans.iter().filter(|p| p.stairwell.is_some()) {
        let well = plan.stairwell.unwrap();
        let blocked = |c: &&dead_sector::city::Collider| {
            c.min_x < well.max_x && c.max_x > well.min_x && c.min_z < well.max_z && c.max_z > well.min_z
        };
        let partitions =
            city.colliders.iter().filter(|c| c.kind == "partition" && c.owner == Some(Owner::Int(plan.building)));
        assert_eq!(partitions.filter(blocked).count(), 0, "building {}", plan.building);
        let prefix = format!("fresh-{}-", plan.building);
        let furniture = city
            .colliders
            .iter()
            .filter(|c| c.kind == "furniture" && c.owner.as_ref().is_some_and(|o| o.key().starts_with(&prefix)));
        assert_eq!(furniture.filter(blocked).count(), 0, "building {}", plan.building);
    }
}

#[test]
fn seeds_vary_storeys_and_add_no_draw_calls() {
    let floors = |seed| -> Vec<u32> { seeded(seed).1.buildings.plans.iter().map(|p: &Plan| p.floors).collect() };
    assert_ne!(floors(7), floors(1234));
    let (city, report) = seeded(7);
    assert_eq!(report.buildings.plans.len(), 99, "every building but the store");
    assert!(
        report.buildings.plans.iter().any(|p| p.floors >= 8) && report.buildings.plans.iter().any(|p| p.floors == 1)
    );
    // Modules and furniture join existing batches: the draw list does not grow.
    assert_eq!(city.batches.len(), exported().batches.len());
    let (draws, exported_draws) = (build_scene(city).draws.len(), build_scene(exported()).draws.len());
    assert!(draws <= exported_draws, "{draws} draws vs {exported_draws}");
}

/// Walks straight towards `target` (XZ) until within 5 cm; returns whether it got there.
fn go_to(p: &mut Player, world: &sector_physics::World, x: f32, z: f32) -> bool {
    for _ in 0..600 {
        let (dx, dz) = (x - p.feet.x, z - p.feet.z);
        if (dx * dx + dz * dz).sqrt() < 0.05 {
            return true;
        }
        // Forward is (−sin yaw, −cos yaw).
        p.yaw = (-dx).atan2(-dz);
        let slow = (dx * dx + dz * dz).sqrt() < 0.5;
        p.step(world, Input { forward: if slow { 0.3 } else { 1.0 }, ..Default::default() });
        assert!(!world.overlaps(&p.body()));
    }
    false
}

#[test]
fn players_can_climb_to_the_top_floor() {
    let (city, report) = seeded(7);
    let world = player::collision_world(city);
    let mut climbed = 0;
    for plan in report.buildings.plans.iter().filter(|p| p.floors >= 4) {
        let mid = |floor: usize| (plan.strips[floor % 2].0 + plan.strips[floor % 2].1) / 2.0;
        let (start, dir) = plan.flights[0];
        let mut p = Player::new(Vec3::new(start - dir * 0.6, 0.01, mid(0)));
        for (floor, &(start, dir)) in plan.flights.iter().enumerate() {
            // To the foot of this flight, then up it onto the landing.
            assert!(go_to(&mut p, &world, start - dir * 0.6, mid(floor)), "building {} floor {floor}", plan.building);
            let end = start + dir * STEPS as f32 * TREAD;
            assert!(go_to(&mut p, &world, end + dir * 0.6, mid(floor)), "building {} flight {floor}", plan.building);
            assert!((p.feet.y - (floor + 1) as f32 * STOREY).abs() < 0.05, "building {}: {:?}", plan.building, p.feet);
        }
        climbed += 1;
    }
    assert!(climbed >= 10, "{climbed} tall buildings");
}
