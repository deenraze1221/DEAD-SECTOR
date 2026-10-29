//! Seeded city generation against the real exported city: determinism, fixed
//! plots, furniture placement, clear doorways and collider/geometry agreement.

use dead_sector::city::{City, Collider, Owner};
use dead_sector::citygen::{self, Aabb, FIXED_PLOTS, PLOTS, geometry_bounds, matrix, plot_at};
use dead_sector::content::{build_scene, locate_assets};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const SEEDS: [u64; 4] = [1, 7, 1234, 2_147_483_647];

fn original() -> &'static City {
    static CITY: OnceLock<City> = OnceLock::new();
    CITY.get_or_init(|| {
        let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets");
        let assets = locate_assets(Some(&checkout)).expect("godot/assets/city.json must exist in the checkout");
        City::load(&assets.join("city.json")).expect("city.json loads")
    })
}

fn generated(seed: u64) -> (City, citygen::Report) {
    static CACHE: OnceLock<Mutex<BTreeMap<u64, (City, citygen::Report)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().unwrap().get(&seed) {
        return hit.clone();
    }
    let mut city = original().clone();
    let report = citygen::generate(&mut city, seed);
    cache.lock().unwrap().insert(seed, (city.clone(), report.clone()));
    (city, report)
}

fn collider_box(c: &Collider) -> Aabb {
    Aabb::new(Vec3::new(c.min_x, c.min_y, c.min_z), Vec3::new(c.max_x, c.max_y, c.max_z))
}

fn key(owner: &Option<Owner>) -> String {
    owner.as_ref().map(Owner::key).unwrap_or_default()
}

/// Colliders of generated furniture, grouped by item id.
fn fresh_items(city: &City) -> BTreeMap<String, Vec<Aabb>> {
    let mut items: BTreeMap<String, Vec<Aabb>> = BTreeMap::new();
    for c in city.colliders.iter().filter(|c| c.kind == "furniture") {
        let id = key(&c.owner);
        if id.starts_with("fresh-") {
            items.entry(id).or_default().push(collider_box(c));
        }
    }
    items
}

fn building_of(item: &str) -> i64 {
    item.split('-').nth(1).unwrap().parse().unwrap()
}

#[test]
fn same_seed_gives_identical_city() {
    for seed in [7, 1234] {
        let (a, ra) = generated(seed);
        let mut b = original().clone();
        let rb = citygen::generate(&mut b, seed);
        assert_eq!(ra.layout.source_for, rb.layout.source_for);
        assert_eq!(a.buildings, b.buildings);
        assert_eq!(a.batches, b.batches);
        assert_eq!(a.colliders, b.colliders);
        assert_eq!(a.source_assets, b.source_assets);
        assert_eq!(a.seed, seed);
    }
}

#[test]
fn layout_is_pinned_for_a_known_seed() {
    // Any change to the RNG or the generator changes every seeded city; this
    // makes such a change deliberate (and co-op clients must agree on it).
    let (city, report) = generated(7);
    let kinds: Vec<&str> = city.buildings.iter().map(|b| b.kind.as_str()).collect();
    let fingerprint = kinds.join(",");
    let expected = include_str!("citygen_seed7.txt").trim();
    assert_eq!(fingerprint, expected, "seed 7 layout changed");
    assert_eq!((report.interiors.furniture, city.instance_count()), (1819, 77_675));
    let pinned: Vec<usize> = (0..PLOTS).filter(|&p| report.layout.pinned[p]).collect();
    // No shared object reaches into a plot's core in the exported city, so only the fixed plots stay.
    assert_eq!(pinned, [0, 9, 44, 45, 54, 55, 90, 99]);
}

#[test]
fn different_seeds_give_different_cities() {
    let layouts: Vec<Vec<String>> =
        SEEDS.iter().map(|&s| generated(s).0.buildings.iter().map(|b| b.kind.clone()).collect()).collect();
    for i in 0..layouts.len() {
        for j in i + 1..layouts.len() {
            assert_ne!(layouts[i], layouts[j], "seeds {} and {}", SEEDS[i], SEEDS[j]);
        }
    }
    let exported: Vec<String> = original().buildings.iter().map(|b| b.kind.clone()).collect();
    assert!(layouts.iter().all(|l| *l != exported));
}

#[test]
fn fixed_plots_keep_their_buildings_and_the_store() {
    let source = original();
    for seed in SEEDS {
        let (city, report) = generated(seed);
        assert_eq!(city.buildings.len(), PLOTS);
        for plot in FIXED_PLOTS {
            assert!(report.layout.pinned[plot]);
        }
        for (plot, pinned) in report.layout.pinned.iter().enumerate() {
            let b = &city.buildings[plot];
            assert_eq!(plot_at(b.cx, b.cz), Some(plot));
            if *pinned {
                assert_eq!(report.layout.source_for[plot], plot);
                let original = source.buildings.iter().find(|o| plot_at(o.cx, o.cz) == Some(plot)).unwrap();
                // Same building; only the height follows its modular storeys (the store is untouched).
                let floors = report.buildings.plans.iter().find(|p| p.building == b.id).map(|p| p.floors);
                let h = floors.map_or(original.h, |f| f as f32 * dead_sector::buildings::STOREY);
                assert_eq!(*b, dead_sector::city::Building { h, ..original.clone() }, "seed {seed} plot {plot}");
                assert_eq!(floors.is_none(), b.kind == "grocery");
            }
        }
        // The store keeps its exported interior and partitions untouched.
        let store = |c: &&Collider| {
            let k = key(&c.owner);
            k.starts_with("interior-55-") || (c.kind == "partition" && k == "55")
        };
        let before: Vec<&Collider> = source.colliders.iter().filter(store).collect();
        let after: Vec<&Collider> = city.colliders.iter().filter(store).collect();
        assert!(!before.is_empty());
        assert_eq!(before, after, "seed {seed}");
        assert_eq!(city.terminal.x, source.terminal.x);
        assert_eq!(city.spawn.x, source.spawn.x);
    }
}

#[test]
fn districts_keep_every_building_type() {
    let source = original();
    for seed in SEEDS {
        let (city, report) = generated(seed);
        let types = |c: &City, d: &str| -> BTreeSet<String> {
            c.buildings.iter().filter(|b| b.district == d).map(|b| b.kind.clone()).collect()
        };
        for district in ["industrial", "residential", "medical", "market"] {
            assert_eq!(types(&city, district), types(source, district), "seed {seed} {district}");
        }
        assert!(report.layout.moved_plots() > 30, "seed {seed}: {} moved", report.layout.moved_plots());
        // Each plot holds a building of its source plot's type, at the same place within the plot.
        for (target, &src) in report.layout.source_for.iter().enumerate() {
            let from = source.buildings.iter().find(|o| plot_at(o.cx, o.cz) == Some(src)).unwrap();
            let to = &city.buildings[target];
            assert_eq!(to.kind, from.kind);
            assert!(((to.x - to.cx) - (from.x - from.cx)).abs() < 1e-3);
            assert!(((to.z - to.cz) - (from.z - from.cz)).abs() < 1e-3);
        }
    }
}

#[test]
fn generated_city_is_valid_and_builds() {
    for seed in SEEDS {
        let (city, report) = generated(seed);
        city.validate().unwrap();
        for b in &city.batches {
            assert_eq!(b.owners.len(), b.transforms.len());
            assert_eq!(b.roles.len(), b.transforms.len());
        }
        // Exported furniture is gone except in the store.
        let leftovers = city
            .batches
            .iter()
            .flat_map(|b| &b.owners)
            .chain(city.colliders.iter().map(|c| &c.owner))
            .map(key)
            .filter(|k| k.starts_with("interior-") && !k.starts_with("interior-55-"))
            .count();
        assert_eq!(leftovers, 0, "seed {seed}");
        let scene = build_scene(&city);
        assert_eq!(scene.instances.len(), city.instance_count());
        // 99 furnished buildings: at least two rooms each.
        assert!(report.interiors.rooms.len() >= 99 * 2);
        eprintln!(
            "seed {seed}: {} moved, {} rooms, {} items, {} instances, {:.0} ms",
            report.layout.moved_plots(),
            report.interiors.rooms.len(),
            report.interiors.furniture,
            city.instance_count(),
            report.millis
        );
    }
}

#[test]
fn furniture_does_not_overlap() {
    for seed in SEEDS {
        let (city, report) = generated(seed);
        let items = fresh_items(&city);
        assert_eq!(items.len(), report.interiors.furniture);
        let walls: Vec<(i64, Aabb)> = city
            .colliders
            .iter()
            .filter(|c| c.kind == "partition")
            .map(|c| (key(&c.owner).parse().unwrap(), collider_box(c)))
            .collect();
        let list: Vec<(&String, &Vec<Aabb>)> = items.iter().collect();
        for (i, (a, boxes_a)) in list.iter().enumerate() {
            for (b, boxes_b) in &list[i + 1..] {
                if building_of(a) != building_of(b) {
                    continue;
                }
                for x in *boxes_a {
                    for y in *boxes_b {
                        assert!(!x.intersects(y), "seed {seed}: {a} overlaps {b}");
                    }
                }
            }
            for (owner, wall) in &walls {
                if *owner == building_of(a) {
                    assert!(boxes_a.iter().all(|x| !x.intersects(wall)), "seed {seed}: {a} inside a partition");
                }
            }
        }
        // Padded footprints are disjoint too.
        let prints = &report.interiors.footprints;
        for (i, (ba, a)) in prints.iter().enumerate() {
            for (bb, b) in &prints[i + 1..] {
                assert!(ba != bb || !a.intersects(b), "seed {seed}: footprints overlap in building {ba}");
            }
        }
    }
}

#[test]
fn doorways_and_corridors_stay_clear() {
    for seed in SEEDS {
        let (city, report) = generated(seed);
        assert!(!report.interiors.reserved.is_empty());
        let fresh: Vec<(i64, Aabb)> = fresh_items(&city)
            .into_iter()
            .flat_map(|(id, boxes)| boxes.into_iter().map(move |b| (building_of(&id), b)))
            .collect();
        let walls: Vec<(i64, Aabb)> = city
            .colliders
            .iter()
            .filter(|c| c.kind == "partition")
            .map(|c| (key(&c.owner).parse().unwrap(), collider_box(c)))
            .collect();
        for (building, clear) in &report.interiors.reserved {
            for (owner, b) in fresh.iter().chain(&walls) {
                if owner == building {
                    assert!(
                        !clear.intersects(b),
                        "seed {seed}: building {building} doorway/corridor {clear:?} blocked by {b:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn furniture_stays_inside_its_room() {
    for seed in SEEDS {
        let (city, report) = generated(seed);
        for (id, boxes) in fresh_items(&city) {
            let building = building_of(&id);
            let union = boxes.iter().skip(1).fold(boxes[0], |u, b| u.union(b));
            let inside = report.interiors.rooms.iter().filter(|r| r.building == building).any(|r| {
                union.min.y >= r.floor_y - 1e-3
                    && union.max.y <= r.floor_y + 3.2
                    && union.min.x >= r.min_x - 1e-3
                    && union.max.x <= r.max_x + 1e-3
                    && union.min.z >= r.min_z - 1e-3
                    && union.max.z <= r.max_z + 1e-3
            });
            assert!(inside, "seed {seed}: {id} outside its rooms");
            let b = city.buildings.iter().find(|b| b.id == building).unwrap();
            assert_ne!(b.kind, "grocery");
        }
    }
}

#[test]
fn colliders_match_visible_geometry() {
    let bounds = geometry_bounds(original());
    for seed in [7, 1234] {
        let (city, _) = generated(seed);
        // Visible bounds of each generated item and partition.
        let mut visible: BTreeMap<String, Aabb> = BTreeMap::new();
        let mut partitions: Vec<Aabb> = Vec::new();
        for batch in &city.batches {
            for (i, owner) in batch.owners.iter().enumerate() {
                let k = key(owner);
                let world = bounds[batch.geometry].transformed(&matrix(&batch.transforms[i]));
                if k.starts_with("fresh-") {
                    visible.entry(k).and_modify(|u| *u = u.union(&world)).or_insert(world);
                } else if k.starts_with("partition-") {
                    partitions.push(world);
                }
            }
        }
        let items = fresh_items(&city);
        // Every collider has a visible item, and lies within what is drawn.
        for (id, boxes) in &items {
            let seen = visible.get(id).unwrap_or_else(|| panic!("{id} has colliders but no geometry"));
            for b in boxes {
                assert!(
                    b.min.cmpge(seen.min - 0.02).all() && b.max.cmple(seen.max + 0.02).all(),
                    "seed {seed}: {id} collider {b:?} outside visible {seen:?}"
                );
            }
        }
        // Each partition wall's collider is exactly its drawn box.
        let walls: Vec<Aabb> = city
            .colliders
            .iter()
            .filter(|c| c.kind == "partition" && key(&c.owner) != "55")
            .map(collider_box)
            .collect();
        assert_eq!(walls.len(), partitions.len());
        for (wall, drawn) in walls.iter().zip(&partitions) {
            assert!((wall.min - drawn.min).abs().max_element() < 1e-3, "{wall:?} vs {drawn:?}");
            assert!((wall.max - drawn.max).abs().max_element() < 1e-3, "{wall:?} vs {drawn:?}");
        }
    }
}
