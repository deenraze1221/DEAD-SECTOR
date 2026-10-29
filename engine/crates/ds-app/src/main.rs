//! Dead Sector — Rust engine, stage 2: the exported city with Godot's look.
//!
//! Window mode: free-fly camera over the static city.
//! `--screenshot <png>` / `--bench <frames>`: offscreen, no window.

mod options;
mod window;

use anyhow::{Context, Result};
use ds_assets::{City, SceneBuffers, locate_assets};
use ds_render::{CAPTURE_FORMAT, Camera, Environment, FrameInputs, Renderer, Weather, connect};
use glam::Vec3;
use options::Options;
use std::path::PathBuf;
use std::time::Instant;

/// Everything loaded from disk before the GPU is involved.
pub struct LoadedCity {
    pub assets: PathBuf,
    pub city: City,
    pub scene: SceneBuffers,
}

fn load(options: &Options) -> Result<LoadedCity> {
    let assets = locate_assets(options.assets.as_deref())?;
    let started = Instant::now();
    let city = City::load(&assets.join(ds_assets::CITY_FILE))?;
    let parsed = started.elapsed();
    let scene = SceneBuffers::build(&city);
    log::info!(
        "loaded {} in {:.0} ms (+{:.0} ms buffers): {} batches, {} instances, {} triangles/frame",
        assets.join(ds_assets::CITY_FILE).display(),
        parsed.as_secs_f64() * 1e3,
        (started.elapsed() - parsed).as_secs_f64() * 1e3,
        city.batches.len(),
        scene.instances.len(),
        scene.triangles_per_frame(),
    );
    Ok(LoadedCity { assets, city, scene })
}

/// Starting camera: the player's spawn point and eye height, or `--camera`.
pub fn initial_camera(options: &Options, city: &City) -> Camera {
    let spawn = Vec3::new(city.spawn.x, city.spawn.y, city.spawn.z);
    let mut camera = Camera::player_view(spawn);
    camera.far = options.quality.view_distance();
    if let Some(pose) = options.camera {
        camera.position = Vec3::new(pose[0], pose[1], pose[2]);
        camera.yaw = pose[3].to_radians();
        camera.pitch = pose[4].to_radians();
    }
    camera
}

/// The look Godot shows at the start of wave 1 for this city's seed, with any
/// command-line overrides applied.
pub fn environment(options: &Options, city: &City) -> Environment {
    let mut weather = Weather::at_wave_start(city.seed as i64, 1);
    weather.kind = options.weather.unwrap_or(weather.kind);
    weather.fog = options.fog_level.unwrap_or(weather.fog);
    weather.night = options.night;
    if options.night {
        weather.hour = 18.0;
    }
    weather.power_cut = options.blackout;
    log::info!("weather: {:?}, {:?} fog, {}", weather.kind, weather.fog, if weather.night { "night" } else { "day" });
    let environment = Environment::new(&weather);
    if options.fog { environment } else { environment.without_fog() }
}

fn run_offscreen(options: &Options, loaded: &LoadedCity) -> Result<()> {
    let gpu = connect(None)?;
    let info = gpu.adapter.get_info();
    log::info!("adapter: {} ({:?}, {:?})", info.name, info.backend, info.device_type);
    let mut renderer =
        Renderer::new(&gpu.device, &gpu.queue, CAPTURE_FORMAT, &loaded.city, &loaded.scene, &loaded.assets)?;
    let camera = initial_camera(options, &loaded.city);
    let environment = environment(options, &loaded.city);
    let inputs = FrameInputs {
        camera: &camera,
        environment: &environment,
        quality: options.quality,
        time: options.time.unwrap_or(0.0),
        hurt: 0.0,
        low_health: false,
    };
    let size = options.size;

    if let Some(frames) = options.bench {
        renderer.bench(size, &inputs, 1)?; // warm-up
        let ms = renderer.bench(size, &inputs, frames)?.as_secs_f64() * 1e3 / frames as f64;
        println!(
            "bench: {frames} frames at {}x{} on {} — {ms:.2} ms/frame ({:.0} fps, GPU-synchronised, no present)",
            size.0,
            size.1,
            info.name,
            1e3 / ms
        );
    }
    if let Some(path) = &options.screenshot {
        let pixels = renderer.capture(size, &inputs)?;
        image::save_buffer(path, &pixels, size.0, size.1, image::ColorType::Rgba8)
            .with_context(|| format!("writing {}", path.display()))?;
        println!("screenshot: {} ({}x{})", path.display(), size.0, size.1);
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .init();
    let options = match Options::parse(std::env::args().skip(1))? {
        Some(options) => options,
        None => {
            println!("{}", options::USAGE);
            return Ok(());
        }
    };
    let loaded = load(&options)?;
    if options.screenshot.is_some() || options.bench.is_some() {
        run_offscreen(&options, &loaded)
    } else {
        window::run(options, loaded)
    }
}
