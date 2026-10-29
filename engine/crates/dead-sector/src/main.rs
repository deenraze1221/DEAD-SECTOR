//! Dead Sector — Rust engine, stage 5: walk the exported city, or a seeded one with modular buildings.
//!
//! Window mode: free-fly camera over the static city.
//! `--screenshot <png>` / `--bench <frames>`: offscreen, no window.

mod options;
mod window;

use anyhow::{Context, Result};
use dead_sector::city::City;
use dead_sector::citygen;
use dead_sector::climate::{Environment, Weather};
use dead_sector::content::{self, CITY_FILE, locate_assets};
use dead_sector::frame::{CityLights, FrameData};
use dead_sector::surfaces::{self, SurfaceLibrary};
use glam::Vec3;
use options::Options;
use sector_assets::{Image, SceneBuffers};
use sector_render::{CAPTURE_FORMAT, Camera, Renderer, RendererConfig, connect};
use std::path::PathBuf;
use std::time::Instant;

/// Everything loaded from disk before the GPU is involved.
pub struct LoadedCity {
    pub assets: PathBuf,
    pub city: City,
    pub scene: SceneBuffers,
    pub textures: Vec<Image>,
    pub lights: CityLights,
    /// Tiling surface materials (empty with `--no-textures` or when none are found).
    pub surface_maps: std::sync::Arc<Vec<sector_assets::materials::MaterialPack>>,
    pub texture_size: u32,
}

impl LoadedCity {
    /// A renderer for this city with Dead Sector's surface hook.
    pub fn renderer(&self, gpu: &sector_render::Connection, format: wgpu::TextureFormat) -> Result<Renderer> {
        let config = RendererConfig {
            surface_hook: content::SURFACE_HOOK.to_string(),
            surface_maps: self.surface_maps.clone(),
            max_map_size: self.texture_size,
            ..Default::default()
        };
        Renderer::new(&gpu.device, &gpu.queue, format, &self.scene, &self.textures, &config)
    }
}

fn load(options: &Options) -> Result<LoadedCity> {
    let assets = locate_assets(options.assets.as_deref())?;
    let started = Instant::now();
    let mut city = City::load(&assets.join(CITY_FILE))?;
    if let Some(seed) = options.seed {
        let report = citygen::generate(&mut city, seed);
        log::info!(
            "seed {seed}: {} plots moved, {} pinned, {} storeys from {} module pieces, {} rooms, {} furniture items in {:.0} ms",
            report.layout.moved_plots(),
            report.layout.pinned.iter().filter(|p| **p).count(),
            report.buildings.plans.iter().map(|p| p.floors).sum::<u32>(),
            report.buildings.pieces,
            report.interiors.rooms.len(),
            report.interiors.furniture,
            report.millis
        );
        city.validate()?;
    }
    let parsed = started.elapsed();
    let mut scene = content::build_scene(&city);
    log::info!(
        "loaded {} in {:.0} ms (+{:.0} ms buffers): {} batches, {} instances, {} triangles/frame",
        assets.join(CITY_FILE).display(),
        parsed.as_secs_f64() * 1e3,
        (started.elapsed() - parsed).as_secs_f64() * 1e3,
        city.batches.len(),
        scene.instances.len(),
        scene.triangles_per_frame(),
    );
    let mut surface_maps = Vec::new();
    if options.textures {
        match surfaces::locate(&assets) {
            Some(dir) => {
                let started = Instant::now();
                let library = SurfaceLibrary::load(&dir, options.material_set.as_deref())?;
                let assigned = library.apply(&city, &mut scene);
                log::info!(
                    "surface materials: {} packs from {} on {assigned} city materials in {:.0} ms",
                    library.packs.len(),
                    dir.display(),
                    started.elapsed().as_secs_f64() * 1e3
                );
                surface_maps = library.packs;
            }
            None => log::warn!("no material library found (engine/assets/materials); using flat surfaces"),
        }
    }
    let textures = content::load_textures(&city, &assets)?;
    let lights = CityLights::new(&city);
    Ok(LoadedCity {
        assets,
        city,
        scene,
        textures,
        lights,
        surface_maps: std::sync::Arc::new(surface_maps),
        texture_size: options.texture_size,
    })
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
    let mut renderer = loaded.renderer(&gpu, CAPTURE_FORMAT)?;
    let camera = initial_camera(options, &loaded.city);
    let environment = environment(options, &loaded.city);
    renderer.set_decals(&loaded.lights.pool_decals(&environment));
    let settings = options.render_settings();
    log::info!("render settings: {settings:?}");
    let frame = FrameData::new(&loaded.lights, camera.position, &environment, options.quality, settings, 0.0, false);
    let inputs = frame.inputs(&camera, options.time.unwrap_or(0.0));
    let size = options.size;

    if let Some(frames) = options.bench {
        renderer.bench(size, &inputs, 2)?; // warm-up
        let result = renderer.bench(size, &inputs, frames)?;
        let ms = result.frame_ms;
        let kind = if info.device_type == wgpu::DeviceType::Cpu { "SOFTWARE renderer" } else { "GPU" };
        println!(
            "bench: {frames} frames at {}x{} on {} [{kind}, {:?}] — {ms:.2} ms/frame ({:.0} fps, GPU-synchronised, no present)",
            size.0,
            size.1,
            info.name,
            info.backend,
            1e3 / ms
        );
        if result.gpu_passes.is_empty() {
            println!("  (per-pass GPU timings unavailable: no timestamp query support)");
        }
        for (label, pass_ms) in &result.gpu_passes {
            println!("  {label:<16} {pass_ms:7.3} ms");
        }
        if !result.cascade_casters.is_empty() {
            let counts: Vec<String> = result.cascade_casters.iter().map(u32::to_string).collect();
            println!("  sun cascade casters: {} of {} instances", counts.join(" / "), renderer.stats().instances);
        }
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
