//! Data the game passes to the renderer: scene lighting, quality settings and
//! the look of the post chain. Nothing here knows about a particular game.

use glam::Vec3;

/// Global lighting for a frame. Colours are linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lighting {
    /// Unit vector pointing towards the sun.
    pub sun_direction: Vec3,
    /// Colour × energy.
    pub sun: Vec3,
    /// Colour × energy.
    pub ambient: Vec3,
    pub fog_color: Vec3,
    /// Exponential fog density per metre (0 = off).
    pub fog_density: f32,
    /// Procedural sky gradient used for reflections; the background is the fog colour.
    pub sky_top: Vec3,
    pub sky_horizon: Vec3,
    pub ground_horizon: Vec3,
    pub ground_bottom: Vec3,
}

/// Quality-dependent renderer settings. Games map their presets onto this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSettings {
    /// Render the 3D scene at most this many lines tall and upscale with the
    /// retro pass (`None` = native resolution).
    pub max_render_lines: Option<u32>,
    /// Strength of the retro pass sharpening and grain (0..1).
    pub post_detail: f32,
    /// Draw ground decals and fog sheets.
    pub atmosphere: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings { max_render_lines: None, post_detail: 1.0, atmosphere: true }
    }
}

/// Glow, tonemap and colour adjustment. Defaults follow Godot's Environment
/// with the values Dead Sector uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostSettings {
    pub exposure: f32,
    /// Filmic tonemap white point.
    pub white: f32,
    pub glow_intensity: f32,
    pub glow_strength: f32,
    pub glow_bloom: f32,
    pub glow_hdr_threshold: f32,
    pub glow_hdr_scale: f32,
    pub glow_luminance_cap: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
}

impl Default for PostSettings {
    fn default() -> Self {
        PostSettings {
            exposure: 1.0,
            white: 5.0,
            glow_intensity: 0.55,
            glow_strength: 0.9,
            glow_bloom: 0.04,
            glow_hdr_threshold: 1.05,
            glow_hdr_scale: 2.0,
            glow_luminance_cap: 12.0,
            brightness: 1.0,
            contrast: 1.1,
            saturation: 0.8,
        }
    }
}

/// A coloured vignette that fades in from `inner` to `outer` screen-centre distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeTint {
    pub color: Vec3,
    pub inner: f32,
    pub outer: f32,
    /// 0 = invisible.
    pub strength: f32,
}

impl EdgeTint {
    pub const NONE: EdgeTint = EdgeTint { color: Vec3::ZERO, inner: 0.0, outer: 1.0, strength: 0.0 };
}

/// Final retro pass: grade, vignette, scanlines, grain, lens rain and edge tints.
/// Colours here work on display (sRGB) values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetroSettings {
    pub tint: Vec3,
    /// 1 = unchanged, 0 = greyscale.
    pub saturation: f32,
    pub shadow_tint: Vec3,
    pub highlight_tint: Vec3,
    /// Luminance at which highlights fully replace shadows.
    pub highlight_start: f32,
    pub gamma: f32,
    /// Strength of the smoothstep contrast curve.
    pub s_curve: f32,
    pub vignette: f32,
    /// Darkening of every other output line.
    pub scanlines: f32,
    pub grain: f32,
    /// Raindrops on the lens, 0..1.
    pub lens_rain: f32,
    pub edges: [EdgeTint; 2],
}

impl RetroSettings {
    /// No grading: only the pixel grid and sharpening remain.
    pub const NEUTRAL: RetroSettings = RetroSettings {
        tint: Vec3::ONE,
        saturation: 1.0,
        shadow_tint: Vec3::ONE,
        highlight_tint: Vec3::ONE,
        highlight_start: 0.55,
        gamma: 1.0,
        s_curve: 0.0,
        vignette: 0.0,
        scanlines: 0.0,
        grain: 0.0,
        lens_rain: 0.0,
        edges: [EdgeTint::NONE; 2],
    };
}

/// Look of the ground decals and fog sheets for this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Atmosphere {
    /// Linear colour of the additive ground decals.
    pub decal_color: Vec3,
    pub decal_opacity: f32,
    pub fog_sheet_color: Vec3,
    /// 0 hides the sheets.
    pub fog_sheet_density: f32,
    pub fog_sheet_heights: [f32; 2],
    pub fog_sheet_half_size: f32,
    /// Sheets fade out between these distances from the camera.
    pub fog_sheet_reach: [f32; 2],
}

impl Atmosphere {
    pub const NONE: Atmosphere = Atmosphere {
        decal_color: Vec3::ZERO,
        decal_opacity: 0.0,
        fog_sheet_color: Vec3::ZERO,
        fog_sheet_density: 0.0,
        fog_sheet_heights: [0.0, 0.0],
        fog_sheet_half_size: 0.0,
        fog_sheet_reach: [0.0, 1.0],
    };
}
