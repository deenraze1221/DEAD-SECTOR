//! Window glass on a tiny synthetic scene: a red wall seen through a pane.
//! Needs a Vulkan/DX12 adapter (or software Vulkan), so ignored by default:
//!   cargo test -p sector-render --test glass_gpu -- --include-ignored

mod common;

use common::{boxed, cube};
use glam::Vec3;
use sector_assets::{MaterialParams, PassKind, SceneBuffers, SceneBuilder};
use sector_render::{
    Atmosphere, CAPTURE_FORMAT, Camera, FrameInputs, Lighting, PointLight, RenderSettings, Renderer, RendererConfig,
    RetroSettings, connect,
};

const SIZE: (u32, u32) = (160, 90);

#[derive(Clone, Copy, PartialEq)]
enum Pane {
    None,
    Glass,
    /// The same box, opaque: what the windows used to be.
    Opaque,
}

/// A red wall at x = 4 and a pane at x = 2 between it and the camera.
fn scene(pane: Pane) -> SceneBuffers {
    let mut builder = SceneBuilder::new();
    let red = builder.add_material(MaterialParams { albedo: [0.8, 0.05, 0.05, 1.0], ..Default::default() });
    let tint = [0.27, 0.48, 0.43];
    let glass = builder.add_material(MaterialParams {
        albedo: [tint[0], tint[1], tint[2], 0.3],
        pass: PassKind::Transparent,
        roughness: 0.12,
        glass: true,
        ..Default::default()
    });
    let opaque =
        builder.add_material(MaterialParams { albedo: [tint[0], tint[1], tint[2], 1.0], ..Default::default() });
    let mesh = builder.add_mesh(&cube(), None);
    let wall = builder.add_instances(&[boxed(Vec3::new(4.0, 1.5, 0.0), Vec3::new(0.2, 3.0, 6.0))]);
    builder.add_draw(0, mesh, None, red, wall);
    let pane_box = boxed(Vec3::new(2.0, 1.5, 0.0), Vec3::new(0.02, 3.0, 3.0));
    match pane {
        Pane::None => {}
        Pane::Glass => {
            let i = builder.add_instances(&[pane_box]);
            builder.add_draw(1, mesh, None, glass, i);
        }
        Pane::Opaque => {
            let i = builder.add_instances(&[pane_box]);
            builder.add_draw(1, mesh, None, opaque, i);
        }
    }
    builder.finish()
}

/// Mean RGB of the frame centre (the pane covers it).
fn render(pane: Pane) -> [f32; 3] {
    let gpu = connect(None).expect("GPU adapter");
    let scene = scene(pane);
    let mut renderer =
        Renderer::new(&gpu.device, &gpu.queue, CAPTURE_FORMAT, &scene, &[], &RendererConfig::default()).unwrap();
    let lighting = Lighting {
        sun_direction: Vec3::Y,
        sun: Vec3::ZERO,
        ambient: Vec3::splat(0.3),
        fog_color: Vec3::ZERO,
        fog_density: 0.0,
        sky_top: Vec3::new(0.3, 0.35, 0.4),
        sky_horizon: Vec3::new(0.3, 0.35, 0.4),
        ground_horizon: Vec3::splat(0.1),
        ground_bottom: Vec3::splat(0.1),
    };
    let lights =
        [PointLight { position: Vec3::new(0.0, 1.5, 0.0), range: 10.0, color: Vec3::splat(3.0), cast_shadows: false }];
    let mut camera = Camera::player_view(Vec3::new(-1.0, -0.15, 0.0));
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
    for y in 35..55 {
        for x in 70..90 {
            let i = ((y * SIZE.0 + x) * 4) as usize;
            for c in 0..3 {
                sum[c] += pixels[i + c] as f32;
            }
            n += 1.0;
        }
    }
    sum.map(|v| v / n)
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn wall_shows_through_tinted_glass() {
    let (clear, glass, opaque) = (render(Pane::None), render(Pane::Glass), render(Pane::Opaque));
    eprintln!("centre rgb: no pane {clear:?}, glass {glass:?}, opaque pane {opaque:?}");
    // How much the red wall shows: red above the other channels.
    let red = |c: [f32; 3]| c[0] - (c[1] + c[2]) / 2.0;
    assert!(red(clear) > 60.0, "wall not lit");
    // Through glass the wall is still clearly visible, but partly veiled...
    assert!(red(glass) > red(clear) * 0.2 && red(glass) < red(clear) * 0.8, "glass not translucent");
    // ...and shifted towards the blue-green tint.
    let ratio = |c: [f32; 3]| (c[1] + c[2]) / c[0].max(1.0);
    assert!(ratio(glass) > ratio(clear) * 1.2, "no tint");
    // An opaque pane (the old windows) hides the wall entirely.
    assert!(red(opaque) < red(clear) * 0.1);
}
