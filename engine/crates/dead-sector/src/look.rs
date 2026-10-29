//! Dead Sector's look on top of the engine: the grim retro grade, damage
//! vignettes, lamp light pools and ground mist (`pixels.gdshader`,
//! `atmosphere.gd`, `mist.gdshader`).

use crate::climate::Environment;
use glam::Vec3;
use sector_assets::color::srgb_to_linear;
use sector_render::{Atmosphere, EdgeTint, RetroSettings};

/// `pixels.gdshader`: cold green-teal shadows, warmer highlights, crushed
/// blacks, vignette, scanlines and grain.
pub fn retro(lens_rain: f32, hurt: f32, low_health: bool) -> RetroSettings {
    RetroSettings {
        tint: Vec3::new(0.97, 1.0, 1.02),
        saturation: 0.9,
        shadow_tint: Vec3::new(0.86, 1.0, 0.99),
        highlight_tint: Vec3::new(1.03, 1.0, 0.96),
        highlight_start: 0.55,
        gamma: 1.04,
        s_curve: 0.2,
        vignette: 0.36,
        scanlines: 0.025,
        grain: 0.011,
        lens_rain,
        edges: [
            EdgeTint { color: Vec3::new(0.6, 0.025, 0.015), inner: 0.28, outer: 0.7, strength: hurt * 0.5 },
            EdgeTint {
                color: Vec3::new(0.5, 0.025, 0.015),
                inner: 0.26,
                outer: 0.68,
                strength: if low_health { 0.55 } else { 0.0 },
            },
        ],
    }
}

/// Warm additive pools under street lamps and the two low mist sheets.
pub fn atmosphere(env: &Environment) -> Atmosphere {
    Atmosphere {
        decal_color: Vec3::new(1.0, 0.85, 0.6).to_array().map(srgb_to_linear).into(),
        decal_opacity: env.pool_alpha,
        fog_sheet_color: env.mist_color,
        fog_sheet_density: env.mist_density,
        fog_sheet_heights: [0.3, 1.15],
        fog_sheet_half_size: 45.0,
        fog_sheet_reach: [9.0, 38.0],
    }
}
