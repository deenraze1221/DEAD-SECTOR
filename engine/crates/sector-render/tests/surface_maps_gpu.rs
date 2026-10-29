//! Surface maps on a tiny synthetic scene: colour shows up, and the normal-map
//! convention (DirectX: +Y = +v, which runs down a wall) shades the right way.
//! Needs a Vulkan/DX12 adapter (or software Vulkan), so ignored by default:
//!   cargo test -p sector-render --test surface_maps_gpu -- --include-ignored

mod common;

use common::{boxed, cube};
use glam::Vec3;
use sector_assets::materials::{MaterialPack, SourceMaps};
use sector_assets::{MaterialParams, SceneBuffers, SceneBuilder};
use sector_render::{
    Atmosphere, CAPTURE_FORMAT, Camera, FrameInputs, Lighting, PointLight, RenderSettings, Renderer, RendererConfig,
    RetroSettings, connect,
};
use std::sync::Arc;

const SIZE: (u32, u32) = (160, 90);

/// A uniform 16² pack: one colour, one normal (DirectX encoding).
fn pack(albedo: [u8; 3], normal: [f32; 3]) -> MaterialPack {
    let e = |x: f32| ((x * 0.5 + 0.5) * 255.0).round() as u8;
    let fill = |px: [u8; 4], size: u32| (0..size * size).flat_map(|_| px).collect::<Vec<u8>>();
    let colour = fill([albedo[0], albedo[1], albedo[2], 255], 16);
    let normals = fill([e(normal[0]), e(normal[1]), e(normal[2]), 255], 16);
    let orm = fill([255, 200, 0, 255], 16);
    MaterialPack::encode("test", &SourceMaps { size: 16, albedo: &colour, normal: &normals, orm: &orm, orm_size: 16 })
        .unwrap()
}

/// A wall facing −X at x = 4, textured with surface map 0 when `mapped`.
fn scene(mapped: bool) -> SceneBuffers {
    let mut builder = SceneBuilder::new();
    let material = builder.add_material(MaterialParams {
        surface_map: mapped.then_some(0),
        tiling: 2.0,
        roughness: 1.0,
        ..Default::default()
    });
    let mesh = builder.add_mesh(&cube(), None);
    let wall = builder.add_instances(&[boxed(Vec3::new(4.0, 1.5, 0.0), Vec3::new(0.2, 3.0, 6.0))]);
    builder.add_draw(0, mesh, None, material, wall);
    builder.finish()
}

/// Mean RGB of the frame centre with one light above or below the view line.
fn render(maps: Vec<MaterialPack>, mapped: bool, light_y: f32) -> [f32; 3] {
    let gpu = connect(None).expect("GPU adapter");
    let config = RendererConfig { surface_maps: Arc::new(maps), ..Default::default() };
    let mut renderer = Renderer::new(&gpu.device, &gpu.queue, CAPTURE_FORMAT, &scene(mapped), &[], &config).unwrap();
    let lighting = Lighting {
        sun_direction: Vec3::Y,
        sun: Vec3::ZERO,
        ambient: Vec3::ZERO,
        fog_color: Vec3::ZERO,
        fog_density: 0.0,
        sky_top: Vec3::ZERO,
        sky_horizon: Vec3::ZERO,
        ground_horizon: Vec3::ZERO,
        ground_bottom: Vec3::ZERO,
    };
    // Close to the wall and far above (or below) the viewed spot: grazing light.
    let lights = [PointLight {
        position: Vec3::new(3.0, light_y, 0.0),
        range: 20.0,
        color: Vec3::splat(6.0),
        cast_shadows: false,
    }];
    let mut camera = Camera::player_view(Vec3::new(0.0, -0.15, 0.0));
    camera.yaw = -std::f32::consts::FRAC_PI_2;
    let settings = RenderSettings { sun_shadows: None, contact_shadows: None, ..Default::default() };
    let inputs = FrameInputs {
        camera: &camera,
        lighting: &lighting,
        lights: &lights,
        settings: &settings,
        atmosphere: &Atmosphere::NONE,
        retro: &RetroSettings::NEUTRAL,
        game: [0.0; 4],
        time: 0.0,
    };
    let pixels = renderer.capture(SIZE, &inputs).unwrap();
    let mut sum = [0.0f32; 3];
    let mut n = 0.0;
    for y in 40..50 {
        for x in 75..85 {
            let i = ((y * SIZE.0 + x) * 4) as usize;
            for c in 0..3 {
                sum[c] += pixels[i + c] as f32;
            }
            n += 1.0;
        }
    }
    sum.map(|v| v / n)
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn surface_map_colour_replaces_flat_white() {
    let flat = render(vec![pack([200, 30, 30], [0.0, 0.0, 1.0])], false, 1.5);
    let red = render(vec![pack([200, 30, 30], [0.0, 0.0, 1.0])], true, 1.5);
    eprintln!("flat {flat:?}, red map {red:?}");
    // Flat material is grey; the red map makes red clearly dominant (the bright light saturates).
    assert!((flat[0] - flat[1]).abs() < 5.0 && red[0] - red[1] > 60.0, "map colour not applied");
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn normal_maps_follow_the_directx_convention() {
    // Every texel tilts towards +v (DirectX +Y): on a wall, v runs down, so
    // the surface faces slightly downwards.
    let tilted = [0.0, 0.6, 0.8];
    let flat = [0.0, 0.0, 1.0];
    let above = |n| luma(render(vec![pack([180, 180, 180], n)], true, 4.0));
    let below = |n| luma(render(vec![pack([180, 180, 180], n)], true, -1.0));
    let (flat_above, tilted_above) = (above(flat), above(tilted));
    let (flat_below, tilted_below) = (below(flat), below(tilted));
    eprintln!(
        "lit from above: flat {flat_above:.1}, tilted {tilted_above:.1}; from below: flat {flat_below:.1}, tilted {tilted_below:.1}"
    );
    assert!(tilted_above < flat_above * 0.8, "downward-tilted normals should darken light from above");
    assert!(tilted_below > flat_below * 1.2, "downward-tilted normals should catch light from below");
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn packs_of_different_sizes_share_the_arrays() {
    // A 16² and a 32² pack: the larger one contributes from its 16² mip.
    let e = |x: f32| ((x * 0.5 + 0.5) * 255.0).round() as u8;
    let fill = |px: [u8; 4], size: u32| (0..size * size).flat_map(|_| px).collect::<Vec<u8>>();
    let big = MaterialPack::encode(
        "big",
        &SourceMaps {
            size: 32,
            albedo: &fill([30, 200, 30, 255], 32),
            normal: &fill([e(0.0), e(0.0), 255, 255], 32),
            orm: &fill([255, 200, 0, 255], 32),
            orm_size: 32,
        },
    )
    .unwrap();
    let small = pack([200, 30, 30], [0.0, 0.0, 1.0]);
    let green = render(vec![big.clone(), small.clone()], true, 1.5);
    let red = render(vec![small, big], true, 1.5);
    eprintln!("layer 0 = big {green:?}, layer 0 = small {red:?}");
    assert!(green[1] > green[0] + 60.0 && red[0] > red[1] + 60.0);
}
