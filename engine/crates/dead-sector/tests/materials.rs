//! The shipped surface material library: every pack loads with full mip
//! chains, and the library assigns maps to the right city surfaces.

use dead_sector::city::City;
use dead_sector::citygen;
use dead_sector::content::{build_scene, locate_assets};
use dead_sector::surfaces::{self, SurfaceLibrary};
use sector_assets::color::parse_hex_color;
use sector_assets::scene::{FACES_ALL, FACES_UP};
use std::path::PathBuf;
use std::sync::OnceLock;

fn assets() -> PathBuf {
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets");
    locate_assets(Some(&checkout)).unwrap()
}

fn city() -> &'static City {
    static CITY: OnceLock<City> = OnceLock::new();
    CITY.get_or_init(|| City::load(&assets().join("city.json")).unwrap())
}

fn library() -> &'static SurfaceLibrary {
    static LIBRARY: OnceLock<SurfaceLibrary> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let dir = surfaces::locate(&assets()).expect("engine/assets/materials/library.json in the checkout");
        SurfaceLibrary::load(&dir).unwrap()
    })
}

#[test]
fn every_pack_has_full_mip_chains_and_upward_normals() {
    let packs = &library().packs;
    assert!(packs.len() >= 12, "{} packs", packs.len());
    for p in packs {
        assert_eq!((p.albedo.size, p.albedo.levels.len()), (2048, 12), "{}", p.name);
        assert_eq!((p.normal.size, p.normal.levels.len()), (2048, 12), "{}", p.name);
        assert_eq!((p.orm.size, p.orm.levels.len()), (1024, 11), "{}", p.name);
        // A 32² mip of a tangent normal map points mostly out of the surface.
        let level = p.normal.decode(6);
        let mean_tilt: f32 = level
            .chunks(4)
            .map(|px| {
                let (x, y) = (px[0] as f32 / 127.5 - 1.0, px[1] as f32 / 127.5 - 1.0);
                (x * x + y * y).sqrt()
            })
            .sum::<f32>()
            / (level.len() / 4) as f32;
        assert!(mean_tilt < 0.5, "{}: mean tilt {mean_tilt}", p.name);
        assert!(p.mean_albedo.iter().all(|c| (0.005..1.0).contains(c)), "{}: {:?}", p.name, p.mean_albedo);
    }
}

fn textured(city: &City) -> Vec<(usize, u32, u32)> {
    let mut scene = build_scene(city);
    library().apply(city, &mut scene);
    scene.materials.iter().enumerate().filter_map(|(i, m)| m.surface_map.map(|l| (i, l, m.surface_faces))).collect()
}

#[test]
fn library_textures_the_city_surfaces() {
    let city = city();
    let maps = textured(city);
    let by_color = |hex: &str| {
        let c = parse_hex_color(hex).unwrap();
        city.materials.iter().position(|m| m.color == c && m.texture.is_none() && !m.unlit).unwrap()
    };
    let name_of = |i: usize| maps.iter().find(|m| m.0 == i).map(|m| library().packs[m.1 as usize].name.as_str());
    assert_eq!(name_of(by_color("#343c37")), Some("Asphalt"));
    assert_eq!(name_of(by_color("#808574")), Some("TilesFloorSquare"));
    assert_eq!(name_of(by_color("#727e62")), Some("Grass"));
    assert_eq!(name_of(by_color("#9b725c")), Some("RedBricks"));
    assert_eq!(name_of(by_color("#786f60")), Some("GrayBricks"));
    // Floors are textured on top only (their undersides are ceilings).
    let floor = maps.iter().find(|m| m.0 == by_color("#9a9380")).unwrap();
    assert_eq!(floor.2, FACES_UP);
    assert_eq!(maps.iter().find(|m| m.0 == by_color("#343c37")).unwrap().2, FACES_ALL);
    // Signs, glass-like and unlit materials keep their own look.
    for (i, _, _) in &maps {
        let m = &city.materials[*i];
        assert!(m.texture.is_none() && !m.transparent && !m.unlit);
    }
    assert_eq!(maps.len(), 11);
}

#[test]
fn seeded_buildings_texture_roofs_and_stairs_but_not_glass() {
    let mut city = city().clone();
    citygen::generate(&mut city, 7);
    let maps = textured(&city);
    let named = |n: &str| city.materials.iter().position(|m| m.name.as_deref() == Some(n)).unwrap();
    let pack_of = |i: usize| maps.iter().find(|m| m.0 == i).map(|m| library().packs[m.1 as usize].name.clone());
    assert_eq!(pack_of(named("roof")).as_deref(), Some("ConcreteGray"));
    assert_eq!(pack_of(named("stairs")).as_deref(), Some("TilesFloorGray"));
    assert_eq!(pack_of(named("glass")), None);
    // Every one of the shipped packs is used somewhere.
    let used: std::collections::BTreeSet<u32> = maps.iter().map(|m| m.1).collect();
    assert_eq!(used.len(), library().packs.len());
}
