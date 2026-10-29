//! Weather, time of day and graphics quality → the values the renderer uses.
//! Port of the visual targets in `godot/scripts/climate.gd`, `atmosphere.gd`
//! and `main.gd::apply_quality()`. Godot eases towards these targets over a few
//! seconds; this module returns the settled values.

use ds_assets::city::{parse_hex_color, srgb_to_linear};
use glam::{EulerRot, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherKind {
    Overcast,
    Rain,
    Storm,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogLevel {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Low,
    Medium,
    High,
}

impl WeatherKind {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "overcast" => Self::Overcast,
            "rain" => Self::Rain,
            "storm" => Self::Storm,
            "clear" => Self::Clear,
            _ => return None,
        })
    }

    pub fn is_wet(self) -> bool {
        matches!(self, Self::Rain | Self::Storm)
    }
}

impl FogLevel {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "light" => Self::Light,
            "medium" => Self::Medium,
            "heavy" => Self::Heavy,
            _ => return None,
        })
    }
}

impl Quality {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            _ => return None,
        })
    }

    fn index(self) -> usize {
        self as usize
    }

    /// Strength of the post pass sharpening and grain.
    pub fn post_detail(self) -> f32 {
        [0.0, 0.65, 1.0][self.index()]
    }

    /// Street lamps that get a real light around the player.
    pub fn lamp_lights(self) -> usize {
        [0, 4, 8][self.index()]
    }

    /// Camera far plane in metres.
    pub fn view_distance(self) -> f32 {
        [350.0, 600.0, 650.0][self.index()]
    }

    /// Light pools, ground mist and lens rain are disabled on low.
    pub fn atmosphere(self) -> bool {
        self != Self::Low
    }
}

/// `climate.gd::selection()`: the seeded weather and fog roll for a wave.
pub fn roll_weather(seed: i64, wave: i64) -> (WeatherKind, FogLevel) {
    const MASK: i64 = 0xffff_ffff;
    let next = |state: i64| (state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)) & MASK;
    let mut state = (seed ^ (wave.wrapping_mul(0x9e37_79b1) & MASK)) & MASK;
    state = next(state);
    let a = state as f64 / 4_294_967_296.0;
    state = next(state);
    let b = state as f64 / 4_294_967_296.0;
    let kind = if a < 0.55 {
        WeatherKind::Overcast
    } else if a < 0.75 {
        WeatherKind::Rain
    } else if a < 0.9 {
        WeatherKind::Storm
    } else {
        WeatherKind::Clear
    };
    let fog = if b < 0.6 {
        FogLevel::Light
    } else if b < 0.9 {
        FogLevel::Medium
    } else {
        FogLevel::Heavy
    };
    (kind, fog)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    pub kind: WeatherKind,
    pub fog: FogLevel,
    pub night: bool,
    /// 0..24; only decides the warm late-afternoon sun.
    pub hour: f32,
    /// Blacked-out district 0..3 (city quadrant), or `None`.
    pub power_cut: Option<u8>,
}

impl Weather {
    /// The weather Godot shows at the start of `wave` (preparation phase).
    /// Four waves form a day: two day waves, then two night waves.
    pub fn at_wave_start(seed: i64, wave: i64) -> Self {
        let (kind, fog) = roll_weather(seed, wave);
        let phase = (wave - 1).rem_euclid(4);
        Weather { kind, fog, night: phase >= 2, hour: (6 + phase * 6) as f32, power_cut: None }
    }
}

/// sRGB colour as Godot stores it before conversion.
type Srgb = [f32; 3];

fn hex(text: &str) -> Srgb {
    parse_hex_color(text).expect("valid built-in colour")
}

fn lerp(a: Srgb, b: Srgb, t: f32) -> Srgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn linear(c: Srgb) -> Vec3 {
    Vec3::from(c.map(srgb_to_linear))
}

/// Everything the renderer needs to light and colour a frame. Colours are linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// Unit vector pointing towards the sun.
    pub sun_direction: Vec3,
    /// Colour × energy.
    pub sun: Vec3,
    /// Colour × energy.
    pub ambient: Vec3,
    pub fog_color: Vec3,
    pub fog_density: f32,
    /// Procedural sky gradient, used for reflections (the visible sky is fully fogged).
    pub sky_top: Vec3,
    pub sky_horizon: Vec3,
    pub ground_horizon: Vec3,
    pub ground_bottom: Vec3,
    /// 0 dry … 1 soaked asphalt.
    pub wetness: f32,
    pub power_cut: Option<u8>,
    /// Street lamp light energy and light-pool opacity.
    pub lamp_energy: f32,
    pub pool_alpha: f32,
    /// Ground mist opacity and colour.
    pub mist_density: f32,
    pub mist_color: Vec3,
    /// Raindrops on the lens while outdoors (0 = none).
    pub lens_rain: f32,
}

impl Environment {
    pub fn new(weather: &Weather) -> Self {
        let night = weather.night;
        let base = hex(match weather.kind {
            WeatherKind::Overcast => "#1b262b",
            WeatherKind::Rain => "#16232b",
            WeatherKind::Storm => "#101a24",
            WeatherKind::Clear => "#34423f",
        });
        let color = if night { lerp(base, hex("#060a12"), 0.75) } else { lerp(base, hex("#404b48"), 0.42) };
        let sun_color = if night {
            hex("#6f8cba")
        } else if weather.hour > 16.0 {
            hex("#c9a074")
        } else {
            hex("#a9bcc8")
        };
        let sun_energy = if night {
            0.09
        } else {
            match weather.kind {
                WeatherKind::Clear => 0.95,
                WeatherKind::Overcast => 0.72,
                _ => 0.55,
            }
        };
        // DirectionalLight3D rotation_degrees (-57, -30, 0); it shines along its −Z.
        let basis = Quat::from_euler(EulerRot::YXZ, (-30f32).to_radians(), (-57f32).to_radians(), 0.0);
        let mut mist_density = match weather.fog {
            FogLevel::Light => 0.10,
            FogLevel::Medium => 0.17,
            FogLevel::Heavy => 0.30,
        };
        if weather.kind == WeatherKind::Clear {
            mist_density *= 0.6;
        }
        if night {
            mist_density *= 0.6;
        }
        Environment {
            sun_direction: basis * Vec3::Z,
            sun: linear(sun_color) * sun_energy,
            ambient: linear(hex("#74828a")) * if night { 0.11 } else { 0.46 },
            fog_color: linear(color),
            fog_density: match weather.fog {
                FogLevel::Light => 0.012,
                FogLevel::Medium => 0.021,
                FogLevel::Heavy => 0.062,
            },
            sky_top: linear(color.map(|c| c * 0.75)),
            sky_horizon: linear(color),
            ground_horizon: linear(color),
            ground_bottom: linear(hex("#0c1210")),
            wetness: if weather.kind.is_wet() { 1.0 } else { 0.0 },
            power_cut: weather.power_cut,
            lamp_energy: if night { 2.6 } else { 0.9 },
            pool_alpha: if night { 0.36 } else { 0.1 },
            mist_density,
            // Color.lightened(0.12) works on the stored sRGB values.
            mist_color: linear(lerp(color, [1.0; 3], 0.12)),
            lens_rain: match weather.kind {
                WeatherKind::Storm => 1.0,
                WeatherKind::Rain => 0.6,
                _ => 0.0,
            },
        }
    }

    /// Debug view: no distance fog or ground mist.
    pub fn without_fog(mut self) -> Self {
        self.fog_density = 0.0;
        self.mist_density = 0.0;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_roll_matches_climate_gd() {
        // Expected values computed from the GDScript formula in Python.
        let cases = [
            (42, 1, WeatherKind::Overcast, FogLevel::Light),
            (42, 2, WeatherKind::Overcast, FogLevel::Medium),
            (1, 3, WeatherKind::Overcast, FogLevel::Heavy),
            (7920, 2, WeatherKind::Rain, FogLevel::Light),
            (7920, 3, WeatherKind::Clear, FogLevel::Light),
            (15839, 8, WeatherKind::Storm, FogLevel::Medium),
            (2_147_483_646, 20, WeatherKind::Overcast, FogLevel::Light),
        ];
        for (seed, wave, kind, fog) in cases {
            assert_eq!(roll_weather(seed, wave), (kind, fog), "seed {seed} wave {wave}");
        }
    }

    #[test]
    fn day_cycle_follows_waves() {
        let night = |wave| Weather::at_wave_start(42, wave).night;
        assert_eq!([1, 2, 3, 4, 5].map(night), [false, false, true, true, false]);
        assert_eq!(Weather::at_wave_start(42, 2).hour, 12.0);
    }

    #[test]
    fn overcast_day_values() {
        let env = Environment::new(&Weather::at_wave_start(42, 1));
        // 38° → 57° sun elevation from climate.gd.
        assert!((env.sun_direction.y - 57f32.to_radians().sin()).abs() < 1e-5);
        assert!((env.sun.x - srgb_to_linear(0xa9 as f32 / 255.0) * 0.72).abs() < 1e-5);
        assert_eq!(env.fog_density, 0.012);
        assert_eq!(env.wetness, 0.0);
        // fog = lerp(#1b262b, #404b48, 0.42), red channel.
        let red = (0x1b as f32 + (0x40 - 0x1b) as f32 * 0.42) / 255.0;
        assert!((env.fog_color.x - srgb_to_linear(red)).abs() < 1e-6);
    }

    #[test]
    fn night_storm_is_dark_and_wet() {
        let weather =
            Weather { kind: WeatherKind::Storm, fog: FogLevel::Heavy, night: true, hour: 18.0, power_cut: None };
        let day = Environment::new(&Weather { night: false, ..weather });
        let env = Environment::new(&weather);
        assert!(env.sun.length() < day.sun.length() * 0.3);
        assert!(env.ambient.length() < day.ambient.length());
        assert_eq!((env.wetness, env.lens_rain, env.fog_density, env.lamp_energy), (1.0, 1.0, 0.062, 2.6));
        assert!((env.mist_density - 0.18).abs() < 1e-6);
    }

    #[test]
    fn quality_tables() {
        assert_eq!(Quality::parse("medium").map(Quality::lamp_lights), Some(4));
        assert_eq!(Quality::High.view_distance(), 650.0);
        assert!(!Quality::Low.atmosphere());
        assert_eq!(Quality::parse("ultra"), None);
    }
}
