//! Checks the loader against the real exported city. Expected numbers were
//! counted independently from `godot/assets/city.json` with Python.

use ds_assets::scene::PassKind;
use ds_assets::{City, SceneBuffers, load_label_texture, locate_assets};
use std::path::PathBuf;
use std::sync::OnceLock;

fn assets_dir() -> PathBuf {
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets");
    locate_assets(Some(&checkout)).expect("godot/assets/city.json must exist in the checkout")
}

fn city() -> &'static City {
    static CITY: OnceLock<City> = OnceLock::new();
    CITY.get_or_init(|| City::load(&assets_dir().join("city.json")).expect("city.json loads and validates"))
}

#[test]
fn counts_match_export() {
    let city = city();
    assert_eq!(city.version, 1);
    assert_eq!(city.seed, 42);
    assert_eq!(city.geometries.len(), 318);
    assert_eq!(city.materials.len(), 189);
    assert_eq!(city.batches.len(), 313);
    assert_eq!(city.instance_count(), 63_969);
    assert_eq!(city.colliders.len(), 7_268);
    assert_eq!((city.spawn.x, city.spawn.y, city.spawn.z), (10.35, 0.0, 17.1));
}

#[test]
fn scene_buffers_cover_every_instance_and_triangle() {
    let scene = SceneBuffers::build(city());
    assert_eq!(scene.vertices.len(), 104_312);
    assert_eq!(scene.indices.len(), 114_945);
    assert_eq!(scene.instances.len(), 63_969);
    // No batch in this export uses material groups, so there is one draw per batch.
    assert_eq!(scene.draws.len(), 313);
    assert_eq!(scene.triangles_per_frame(), 864_760);
    let drawn: u32 = scene.draws.iter().map(|d| d.instance_count).sum();
    assert_eq!(drawn, 63_969);

    let passes = |kind| scene.materials.iter().filter(|m| m.pass == kind).count();
    assert_eq!(passes(PassKind::Transparent), 1);
    assert_eq!(passes(PassKind::OpaqueDoubleSided), 1);

    // Every draw stays inside the shared buffers.
    for d in &scene.draws {
        let end = (d.first_index + d.index_count) as usize;
        assert!(end <= scene.indices.len());
        let max = scene.indices[d.first_index as usize..end].iter().max().unwrap();
        assert!((d.base_vertex as usize + *max as usize) < scene.vertices.len());
        assert!((d.first_instance + d.instance_count) as usize <= scene.instances.len());
    }
}

#[test]
fn first_batch_keeps_transform_colour_and_material() {
    let city = city();
    let scene = SceneBuffers::build(city);
    let batch = &city.batches[0];
    assert_eq!(batch.name, "city/box/industrialWall");
    let draw = scene.draws.iter().find(|d| d.batch == 0).unwrap();
    let first = scene.instances[draw.first_instance as usize];
    assert_eq!(&first.model[12..15], &[-109.65, 1.56, -114.515]);
    assert_eq!(first.color, [0.9, 0.87, 0.864, 1.0]);
    // #786f60 in linear space.
    let albedo = scene.materials[draw.material].albedo;
    for (got, want) in albedo.iter().zip([0.18782, 0.15896, 0.11697, 1.0]) {
        assert!((got - want).abs() < 1e-4, "{albedo:?}");
    }
}

#[test]
fn every_label_texture_loads() {
    let dir = assets_dir();
    let mut textures: Vec<u32> = city().materials.iter().filter_map(|m| m.texture).collect();
    textures.sort_unstable();
    textures.dedup();
    assert_eq!(textures, (0..33).collect::<Vec<_>>());
    for t in textures {
        let image = load_label_texture(&dir, t).unwrap();
        assert_eq!(image.rgba.len(), (image.width * image.height * 4) as usize);
        assert!(image.rgba.chunks(4).any(|p| p[3] > 0), "label {t} is fully transparent");
    }
}
