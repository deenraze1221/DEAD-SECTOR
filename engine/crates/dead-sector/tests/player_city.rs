//! Player movement against the real city's colliders: no walking through
//! buildings, walls or furniture, no falling through the map, determinism.

use dead_sector::city::City;
use dead_sector::citygen;
use dead_sector::content::locate_assets;
use dead_sector::player::{self, HALF_WIDTH, Input, Player};
use glam::Vec3;
use sector_assets::rng::Rng;
use sector_physics::{Aabb, World};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

fn exported() -> &'static City {
    static CITY: OnceLock<City> = OnceLock::new();
    CITY.get_or_init(|| {
        let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets");
        City::load(&locate_assets(Some(&checkout)).unwrap().join("city.json")).unwrap()
    })
}

fn seeded() -> &'static City {
    static CITY: OnceLock<City> = OnceLock::new();
    CITY.get_or_init(|| {
        let mut city = exported().clone();
        citygen::generate(&mut city, 7);
        city
    })
}

fn spawn(city: &City) -> Vec3 {
    Vec3::new(city.spawn.x, city.spawn.y, city.spawn.z)
}

/// Random but reproducible play: turning, walking, sprinting and jumping.
fn wander(world: &World, start: Vec3, seed: u64, ticks: usize, check: &mut dyn FnMut(&Player)) -> Player {
    let mut rng = Rng::new(seed);
    let mut p = Player::new(start);
    let mut input = Input::default();
    for tick in 0..ticks {
        if tick % 30 == 0 {
            p.yaw += rng.range_f32(-1.2, 1.2);
            input = Input {
                forward: rng.range_f32(-0.3, 1.0),
                right: rng.range_f32(-1.0, 1.0),
                sprint: rng.range_i32(0, 2) == 0,
                jump: false,
            };
        }
        input.jump = tick % 45 == 0 && rng.range_i32(0, 1) == 0;
        p.step(world, input);
        check(&p);
    }
    p
}

#[test]
fn spawn_is_clear_and_the_player_stands_on_the_ground() {
    for city in [exported(), seeded()] {
        let world = player::collision_world(city);
        let mut p = Player::new(spawn(city));
        assert!(!world.overlaps(&p.body()));
        for _ in 0..60 {
            p.step(&world, Input::default());
        }
        assert!(p.grounded);
        assert!(p.feet.y.abs() < 0.01, "{:?}", p.feet);
    }
}

#[test]
fn never_passes_through_colliders_or_the_ground() {
    for (name, city) in [("exported", exported()), ("seed 7", seeded())] {
        let world = player::collision_world(city);
        // Start in the street and inside a few buildings, so interiors are covered too.
        let mut starts = vec![spawn(city)];
        starts.extend(city.buildings.iter().step_by(11).map(|b| Vec3::new(b.x, 0.5, b.z)));
        for (i, start) in starts.into_iter().enumerate() {
            if world.overlaps(&Aabb::standing(start, HALF_WIDTH, player::HEIGHT)) {
                continue;
            }
            let mut ticks = 0;
            let end = wander(&world, start, i as u64, 3600, &mut |p| {
                ticks += 1;
                assert!(!world.overlaps(&p.body()), "{name} start {i}: overlap at {:?} tick {ticks}", p.feet);
                assert!(p.feet.y > -1e-3, "{name} start {i}: below ground at {:?}", p.feet);
            });
            assert!(end.feet.distance(start) > 1.0, "{name} start {i}: did not move");
        }
    }
}

#[test]
fn walls_stop_the_player() {
    // Walk straight at every building from its plot centre's south side; the
    // player must stop at a wall or doorway frame, never end inside a wall box.
    let city = exported();
    let world = player::collision_world(city);
    let mut stopped = 0;
    for b in &city.buildings {
        let start = Vec3::new(b.x, 0.0, b.z + b.d / 2.0 + 2.0);
        if world.overlaps(&Aabb::standing(start, HALF_WIDTH, player::HEIGHT)) {
            continue;
        }
        let mut p = Player::new(start);
        for _ in 0..240 {
            p.step(&world, Input { forward: 1.0, ..Default::default() });
            assert!(!world.overlaps(&p.body()));
        }
        // Without collision the player would be ~17 m further north, through the building.
        if p.feet.z > b.z - b.d / 2.0 {
            stopped += 1;
        }
    }
    assert!(stopped > 80, "only {stopped} buildings stopped the player");
}

#[test]
fn furniture_blocks_movement() {
    let city = seeded();
    let world = player::collision_world(city);
    let mut checked = 0;
    for c in city.colliders.iter().filter(|c| c.kind == "furniture" && c.max_y > 0.6).take(200) {
        // Approach from 1.5 m south of the item's centre, walking north.
        let cx = (c.min_x + c.max_x) / 2.0;
        let start = Vec3::new(cx, 0.0, c.max_z + 1.5);
        if world.overlaps(&Aabb::standing(start, HALF_WIDTH, player::HEIGHT)) {
            continue;
        }
        let mut p = Player::new(start);
        for _ in 0..90 {
            p.step(&world, Input { forward: 1.0, ..Default::default() });
        }
        assert!(!world.overlaps(&p.body()));
        let item = Aabb::new(Vec3::new(c.min_x, c.min_y, c.min_z), Vec3::new(c.max_x, c.max_y, c.max_z));
        assert!(!item.intersects(&p.body()));
        checked += 1;
    }
    assert!(checked > 50, "{checked}");
}

#[test]
fn movement_is_deterministic() {
    let world = player::collision_world(seeded());
    let start = spawn(seeded());
    let a = wander(&world, start, 99, 1800, &mut |_| {});
    let b = wander(&world, start, 99, 1800, &mut |_| {});
    assert_eq!(a, b);
    assert_eq!(a.feet.to_array().map(f32::to_bits), b.feet.to_array().map(f32::to_bits));
    // A different input sequence ends elsewhere.
    let c = wander(&world, start, 100, 1800, &mut |_| {});
    assert_ne!(a.feet, c.feet);
}

#[test]
fn step_cost_is_small() {
    let world = player::collision_world(exported());
    let started = Instant::now();
    wander(&world, spawn(exported()), 5, 6000, &mut |_| {});
    let micros = started.elapsed().as_secs_f64() * 1e6 / 6000.0;
    eprintln!("player step: {micros:.2} µs ({} boxes)", world.len());
    assert!(micros < 1000.0, "{micros} µs per step");
}
