//! Assembles the engine's per-frame inputs from Dead Sector state.

use crate::city::{Building, City};
use crate::climate::{Environment, Quality};
use crate::{lamps, look};
use glam::Vec3;
use sector_render::{Atmosphere, Camera, FrameInputs, Lighting, PointLight, RenderSettings, RetroSettings};

/// Static city data the frame needs every time.
pub struct CityLights {
    pub lamps: Vec<Vec3>,
    pub store: PointLight,
    pub buildings: Vec<Building>,
}

impl CityLights {
    pub fn new(city: &City) -> Self {
        CityLights {
            lamps: city.lamps().map(Vec3::from).collect(),
            store: lamps::store_light(city.terminal.x, city.terminal.z),
            buildings: city.buildings.clone(),
        }
    }

    /// Ground light pools for the renderer (`Renderer::set_decals`).
    pub fn pool_decals(&self, env: &Environment) -> Vec<[f32; 4]> {
        lamps::pool_decals(&self.lamps, env.power_cut)
    }
}

/// Everything except the camera and time that the renderer needs for a frame.
pub struct FrameData {
    pub lighting: Lighting,
    pub lights: Vec<PointLight>,
    pub settings: RenderSettings,
    pub atmosphere: Atmosphere,
    pub retro: RetroSettings,
    pub game: [f32; 4],
}

impl FrameData {
    pub fn new(city: &CityLights, eye: Vec3, env: &Environment, quality: Quality, hurt: f32, low_health: bool) -> Self {
        let mut lights = vec![city.store];
        lights.extend(lamps::street_lights(&city.lamps, eye, quality.lamp_lights(), env.power_cut, env.lamp_energy));
        let outdoors = !lamps::sheltered(&city.buildings, eye);
        let lens_rain = if quality.atmosphere() && outdoors { env.lens_rain } else { 0.0 };
        FrameData {
            lighting: env.lighting(),
            lights,
            settings: quality.render_settings(),
            atmosphere: look::atmosphere(env),
            retro: look::retro(lens_rain, hurt, low_health),
            game: env.game_params(),
        }
    }

    pub fn inputs<'a>(&'a self, camera: &'a Camera, time: f32) -> FrameInputs<'a> {
        FrameInputs {
            camera,
            lighting: &self.lighting,
            lights: &self.lights,
            settings: &self.settings,
            atmosphere: &self.atmosphere,
            retro: &self.retro,
            game: self.game,
            time,
        }
    }
}
