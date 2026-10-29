//! Renders the real city offscreen. Needs a Vulkan/DX12 adapter, so it is
//! ignored by default. Run with:
//!   cargo test -p ds-render --test offscreen -- --include-ignored
//! On a machine without a GPU, point VK_ICD_FILENAMES at a software driver
//! (Mesa lavapipe's lvp_icd.json or SwiftShader's vk_swiftshader_icd.json).

use ds_assets::{City, SceneBuffers, locate_assets};
use ds_render::{CAPTURE_FORMAT, Camera, Environment, FrameInputs, Quality, Renderer, Weather, WeatherKind, connect};
use glam::Vec3;
use std::path::PathBuf;

const SIZE: (u32, u32) = (320, 180);
const SPAWN: Vec3 = Vec3::new(10.35, 0.0, 17.1);

fn renderer() -> Renderer {
    let assets = locate_assets(Some(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../godot/assets"))).unwrap();
    let city = City::load(&assets.join("city.json")).unwrap();
    let scene = SceneBuffers::build(&city);
    let gpu = connect(None).expect("GPU adapter");
    eprintln!("adapter: {:?}", gpu.adapter.get_info());
    Renderer::new(&gpu.device, &gpu.queue, CAPTURE_FORMAT, &city, &scene, &assets).unwrap()
}

fn capture(renderer: &mut Renderer, camera: &Camera, environment: &Environment, quality: Quality) -> Vec<u8> {
    let inputs = FrameInputs { camera, environment, quality, time: 0.0, hurt: 0.0, low_health: false };
    renderer.capture(SIZE, &inputs).unwrap()
}

/// Fraction of pixels differing by more than `tolerance` levels in any channel.
fn difference(a: &[u8], b: &[u8], tolerance: u8) -> f32 {
    let differs = a
        .chunks(4)
        .zip(b.chunks(4))
        .filter(|(p, q)| p[..3].iter().zip(&q[..3]).any(|(x, y)| x.abs_diff(*y) > tolerance))
        .count();
    differs as f32 / (a.len() / 4) as f32
}

fn mean_luma(pixels: &[u8]) -> f32 {
    let sum: f32 = pixels.chunks(4).map(|p| 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32).sum();
    sum / (pixels.len() / 4) as f32
}

fn looking_away() -> Camera {
    let mut camera = Camera::player_view(Vec3::new(0.0, 0.0, 2000.0));
    camera.yaw = std::f32::consts::PI;
    camera
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn city_renders_and_background_is_fog_only() {
    let mut renderer = renderer();
    let day = Environment::new(&Weather::at_wave_start(42, 1));
    let quality = Quality::Medium;

    let street = capture(&mut renderer, &Camera::player_view(SPAWN), &day, quality);
    assert_eq!(street.len(), (SIZE.0 * SIZE.1 * 4) as usize);
    // Far outside the city looking away: only fog, vignette, scanlines and grain.
    let empty = capture(&mut renderer, &looking_away(), &day, quality);
    let covered = difference(&street, &empty, 6);
    eprintln!("street view: {:.1}% differs from an empty view", covered * 100.0);
    assert!(covered > 0.5, "spawn view should be mostly city, got {covered}");

    // Straight up from 200 m, above every building, matches the empty view.
    let mut up = Camera::player_view(Vec3::new(0.0, 200.0, 0.0));
    up.pitch = 89f32.to_radians();
    let sky = capture(&mut renderer, &up, &day, quality);
    assert!(difference(&sky, &empty, 6) < 0.02, "sky view should only show fog");
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn night_is_darker_and_quality_changes_the_frame() {
    let mut renderer = renderer();
    let camera = Camera::player_view(SPAWN);
    let day_weather = Weather::at_wave_start(42, 1);
    let day = Environment::new(&day_weather);
    let night = Environment::new(&Weather { night: true, hour: 18.0, ..day_weather });
    let day_luma = mean_luma(&capture(&mut renderer, &camera, &day, Quality::Medium));
    let night_luma = mean_luma(&capture(&mut renderer, &camera, &night, Quality::Medium));
    eprintln!("mean luma: day {day_luma:.1}, night {night_luma:.1}");
    assert!(night_luma < day_luma * 0.8, "night {night_luma} vs day {day_luma}");

    // Low quality drops street lights, pools, mist and post detail.
    let low = capture(&mut renderer, &camera, &night, Quality::Low);
    let high = capture(&mut renderer, &camera, &night, Quality::High);
    assert!(difference(&low, &high, 2) > 0.05);
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn rain_wets_the_street_and_hits_the_lens() {
    let mut renderer = renderer();
    let mut camera = Camera::player_view(SPAWN);
    camera.yaw = std::f32::consts::PI;
    let overcast = Weather::at_wave_start(42, 1);
    let dry = Environment::new(&overcast);
    let wet = Environment::new(&Weather { kind: WeatherKind::Rain, ..overcast });
    let dry_frame = capture(&mut renderer, &camera, &dry, Quality::High);
    let wet_frame = capture(&mut renderer, &camera, &wet, Quality::High);
    let changed = difference(&dry_frame, &wet_frame, 3);
    eprintln!("rain changes {:.1}% of the street view", changed * 100.0);
    assert!(changed > 0.2);
}
