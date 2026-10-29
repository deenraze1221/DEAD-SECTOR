//! Shadow behaviour on a tiny synthetic scene. Needs a Vulkan/DX12 adapter
//! (or a software Vulkan driver via VK_ICD_FILENAMES), so ignored by default:
//!   cargo test -p sector-render --test shadows_gpu -- --include-ignored

use glam::Vec3;
use sector_assets::{MaterialParams, SceneBuffers, SceneBuilder};
use sector_render::{
    Atmosphere, CAPTURE_FORMAT, Camera, FrameInputs, Lighting, LocalShadows, PointLight, RenderSettings, Renderer,
    RendererConfig, RetroSettings, ShadowFilter, connect,
};

const SIZE: (u32, u32) = (160, 90);

mod common;

use common::{boxed, cube};

/// Floor, a wall at x = 4 and optionally a pillar between the light and the wall.
fn scene(pillar: bool) -> SceneBuffers {
    let mut builder = SceneBuilder::new();
    let material = builder.add_material(MaterialParams { albedo: [0.8, 0.8, 0.8, 1.0], ..Default::default() });
    let mesh = builder.add_mesh(&cube(), None);
    let mut boxes = vec![
        boxed(Vec3::new(0.0, -0.05, 0.0), Vec3::new(20.0, 0.1, 20.0)),
        boxed(Vec3::new(4.0, 2.5, 0.0), Vec3::new(0.2, 5.0, 10.0)),
    ];
    if pillar {
        boxes.push(boxed(Vec3::new(2.0, 2.0, 0.0), Vec3::new(0.5, 4.0, 0.5)));
    }
    let instances = builder.add_instances(&boxes);
    builder.add_draw(0, mesh, None, material, instances);
    builder.finish()
}

fn render(scene: &SceneBuffers, budget: u32, camera: &Camera, name: &str) -> Vec<u8> {
    let gpu = connect(None).expect("GPU adapter");
    let mut renderer =
        Renderer::new(&gpu.device, &gpu.queue, CAPTURE_FORMAT, scene, &[], &RendererConfig::default()).unwrap();
    let lighting = Lighting {
        sun_direction: Vec3::Y,
        sun: Vec3::ZERO,
        ambient: Vec3::splat(0.02),
        fog_color: Vec3::ZERO,
        fog_density: 0.0,
        sky_top: Vec3::ZERO,
        sky_horizon: Vec3::ZERO,
        ground_horizon: Vec3::ZERO,
        ground_bottom: Vec3::ZERO,
    };
    let lights =
        [PointLight { position: Vec3::new(0.0, 2.0, 0.0), range: 12.0, color: Vec3::splat(4.0), cast_shadows: true }];
    let settings = RenderSettings {
        sun_shadows: None,
        contact_shadows: None,
        local_shadows: LocalShadows { budget, resolution: 512, filter: ShadowFilter::Pcf },
        ..Default::default()
    };
    let inputs = FrameInputs {
        camera,
        lighting: &lighting,
        lights: &lights,
        settings: &settings,
        atmosphere: &Atmosphere::NONE,
        retro: &RetroSettings::NEUTRAL,
        game: [0.0; 4],
        time: 0.0,
    };
    let pixels = renderer.capture(SIZE, &inputs).unwrap();
    // SHADOW_TEST_DUMP=<dir> saves the frames for inspection.
    if let Some(dir) = std::env::var_os("SHADOW_TEST_DUMP") {
        let path = std::path::Path::new(&dir).join(format!("{name}-{budget}.png"));
        image::save_buffer(path, &pixels, SIZE.0, SIZE.1, image::ColorType::Rgba8).unwrap();
    }
    pixels
}

/// Looking along +X straight at the wall.
fn facing_wall() -> Camera {
    let mut camera = Camera::player_view(Vec3::new(-3.0, 0.35, 0.0));
    camera.yaw = -std::f32::consts::FRAC_PI_2;
    camera
}

/// Beside the pillar, looking at the part of the wall behind it.
fn beside_pillar() -> Camera {
    let mut camera = Camera::player_view(Vec3::new(1.0, 0.0, 6.0));
    let to_wall = Vec3::new(3.0, 0.0, -6.0).normalize();
    camera.yaw = (-to_wall.x).atan2(-to_wall.z);
    camera
}

/// Mean luma of a column band of the image (the wall fills the frame centre).
fn band(pixels: &[u8], x0: u32, x1: u32) -> f32 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for y in 20..70 {
        for x in x0..x1 {
            let i = ((y * SIZE.0 + x) * 4) as usize;
            sum += 0.2126 * pixels[i] as f32 + 0.7152 * pixels[i + 1] as f32 + 0.0722 * pixels[i + 2] as f32;
            n += 1.0;
        }
    }
    sum / n
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn unoccluded_wall_is_not_darkened_by_its_shadow_map() {
    let open = scene(false);
    let unshadowed = render(&open, 0, &facing_wall(), "open");
    let shadowed = render(&open, 1, &facing_wall(), "open");
    let (a, b) = (band(&unshadowed, 20, 140), band(&shadowed, 20, 140));
    eprintln!("open wall luma: no shadow map {a:.1}, with shadow map {b:.1}");
    assert!(a > 20.0, "wall should be lit ({a})");
    assert!((a - b).abs() < a * 0.05, "shadow map darkened an open wall: {a} vs {b}");
}

/// Mean luma of a 3×3 pixel patch around where `world` appears on screen.
fn at(pixels: &[u8], camera: &Camera, world: Vec3) -> f32 {
    let ndc = camera.view_proj(SIZE.0 as f32 / SIZE.1 as f32).project_point3(world);
    let (cx, cy) = (((ndc.x * 0.5 + 0.5) * SIZE.0 as f32) as i32, ((0.5 - ndc.y * 0.5) * SIZE.1 as f32) as i32);
    let mut sum = 0.0;
    for y in cy - 1..=cy + 1 {
        for x in cx - 1..=cx + 1 {
            let i = ((y as u32 * SIZE.0 + x as u32) * 4) as usize;
            sum += 0.2126 * pixels[i] as f32 + 0.7152 * pixels[i + 1] as f32 + 0.0722 * pixels[i + 2] as f32;
        }
    }
    sum / 9.0
}

#[test]
#[ignore = "needs a Vulkan/DX12 adapter"]
fn pillar_casts_a_shadow_on_the_wall() {
    let blocked = scene(true);
    let camera = beside_pillar();
    let unshadowed = render(&blocked, 0, &camera, "pillar");
    let shadowed = render(&blocked, 1, &camera, "pillar");
    // Light (0, 2, 0) → pillar at x = 2 → its shadow centre on the wall plane x = 3.9 is z = 0.
    let in_shadow = Vec3::new(3.89, 2.0, 0.0);
    // 2.5 m to the side the wall sees the light directly.
    let open = Vec3::new(3.89, 2.0, -2.5);
    let (free, dark) = (at(&unshadowed, &camera, in_shadow), at(&shadowed, &camera, in_shadow));
    let (control_free, control) = (at(&unshadowed, &camera, open), at(&shadowed, &camera, open));
    eprintln!("shadow point {free:.1} → {dark:.1}, open point {control_free:.1} → {control:.1}");
    assert!(dark < free * 0.3, "no pillar shadow");
    assert!((control - control_free).abs() < control_free * 0.05, "open wall darkened");
}
