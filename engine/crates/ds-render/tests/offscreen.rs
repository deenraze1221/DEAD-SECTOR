//! Renders the real city offscreen. Needs a Vulkan/DX12 adapter, so it is
//! ignored by default. Run with:
//!   cargo test -p ds-render --test offscreen -- --include-ignored
//! On a machine without a GPU, point VK_ICD_FILENAMES at a software driver
//! (e.g. SwiftShader's vk_swiftshader_icd.json).

use ds_assets::{City, SceneBuffers, locate_assets};
use ds_render::{CAPTURE_FORMAT, Camera, Lighting, Renderer, connect};
use glam::Vec3;
use std::path::PathBuf;

const SIZE: (u32, u32) = (320, 180);

fn renderer() -> Renderer {
    let assets = locate_assets(Some(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets"))).unwrap();
    let city = City::load(&assets.join("city.json")).unwrap();
    let scene = SceneBuffers::build(&city);
    let gpu = connect(None).expect("GPU adapter");
    eprintln!("adapter: {:?}", gpu.adapter.get_info());
    Renderer::new(&gpu.device, &gpu.queue, CAPTURE_FORMAT, &scene, &assets).unwrap()
}

fn fog_pixel(lighting: &Lighting) -> [u8; 3] {
    // Linear → sRGB, as the Rgba8UnormSrgb target stores it.
    lighting.fog_color.to_array().map(|c| {
        let s = if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    })
}

/// Fraction of pixels that differ from `background` by more than 2 levels.
fn coverage(pixels: &[u8], background: [u8; 3]) -> f32 {
    let differs = pixels.chunks(4).filter(|p| p[..3].iter().zip(background).any(|(a, b)| a.abs_diff(b) > 2)).count();
    differs as f32 / (pixels.len() / 4) as f32
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn city_renders_from_spawn_and_sky_is_fog() {
    let mut renderer = renderer();
    let lighting = Lighting::godot_default();
    let fog = fog_pixel(&lighting);
    let spawn = Vec3::new(10.35, 0.0, 17.1);

    let street = renderer.capture(SIZE, &Camera::player_view(spawn), &lighting).unwrap();
    assert_eq!(street.len(), (SIZE.0 * SIZE.1 * 4) as usize);
    let covered = coverage(&street, fog);
    let mut colors: Vec<[u8; 3]> = street.chunks(4).map(|p| [p[0], p[1], p[2]]).collect();
    colors.sort_unstable();
    colors.dedup();
    eprintln!("street view: {:.1}% geometry, {} distinct colours", covered * 100.0, colors.len());
    assert!(covered > 0.5, "spawn view should be mostly city, got {covered}");
    assert!(colors.len() > 100, "spawn view looks flat: {} colours", colors.len());

    // Looking straight up from 200 m, above every building, there is only sky.
    let mut up = Camera::player_view(Vec3::new(0.0, 200.0, 0.0));
    up.pitch = 89f32.to_radians();
    let sky = renderer.capture(SIZE, &up, &lighting).unwrap();
    assert!(coverage(&sky, fog) < 0.02, "sky view should be clear colour");

    // Far outside the 250 m city looking away, nothing is drawn.
    let mut away = Camera::player_view(Vec3::new(0.0, 0.0, 2000.0));
    away.yaw = std::f32::consts::PI;
    let empty = renderer.capture(SIZE, &away, &lighting).unwrap();
    assert!(coverage(&empty, fog) < 0.001, "empty view drew geometry");
}
