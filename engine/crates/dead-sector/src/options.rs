//! Command-line options.

use anyhow::{Context, Result, bail};
use dead_sector::climate::{FogLevel, Quality, WeatherKind};
use sector_render::{ContactShadows, RenderSettings, ShadowFilter, Ssao, SunShadows};
use std::path::PathBuf;

pub const USAGE: &str = "\
DEAD SECTOR (Sector Engine, stage 3)

USAGE: dead-sector [options]

  --assets <dir>          Folder containing city.json (default: auto-detect)
  --size <W>x<H>          Window / capture size (default 1280x720)
  --camera <x,y,z,yaw,pitch>
                          Start camera: position in metres, angles in degrees
  --no-vsync              Present without vsync (uncapped FPS)
  --seed <n>              Generate a new city layout and interiors from this seed
                          (default: the exported seed-42 city, unchanged)
  --quality <q>           Graphics quality: low, medium (default), high
  --weather <w>           overcast, rain, storm or clear (default: seeded wave 1 roll)
  --fog-level <f>         light, medium or heavy (default: seeded wave 1 roll)
  --night                 Night lighting (default: day, as in wave 1)
  --blackout <0-3>        Cut power to one city quadrant
  --no-fog                Disable fog and mist (debug view of the whole city)
  --time <seconds>        Fixed effect time for screenshots (mist, rain, grain)

Shadow and occlusion overrides (on top of --quality):
  --sun-shadows on|off    Cascaded sun shadows
  --shadow-res <px>       Sun shadow map size: 512, 1024, 2048 or 4096
  --cascades <1-4>        Sun shadow cascades
  --shadow-distance <m>   Sun shadow distance
  --shadow-filter <f>     hard, pcf or soft (sun and local)
  --local-shadows <0-4>   Shadowed point lights (nearest first)
  --local-shadow-res <px> Point-light shadow face size
  --contact-shadows on|off
  --ssao on|off           Screen-space ambient occlusion (off in every preset)
  --screenshot <file.png> Render one frame offscreen, save it and exit
  --bench <frames>        Render frames offscreen and report the average time
  -h, --help              Show this help

Window controls: click to capture the mouse, Esc to release it,
WASD move, Space/Ctrl up/down, Shift fast, F11 fullscreen.";

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub assets: Option<PathBuf>,
    pub size: (u32, u32),
    pub camera: Option<[f32; 5]>,
    pub vsync: bool,
    pub fog: bool,
    pub quality: Quality,
    pub seed: Option<u64>,
    pub weather: Option<WeatherKind>,
    pub fog_level: Option<FogLevel>,
    pub night: bool,
    pub blackout: Option<u8>,
    pub time: Option<f32>,
    pub sun_shadows: Option<bool>,
    pub shadow_res: Option<u32>,
    pub cascades: Option<u32>,
    pub shadow_distance: Option<f32>,
    pub shadow_filter: Option<ShadowFilter>,
    pub local_shadows: Option<u32>,
    pub local_shadow_res: Option<u32>,
    pub contact_shadows: Option<bool>,
    pub ssao: Option<bool>,
    pub screenshot: Option<PathBuf>,
    pub bench: Option<u32>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            assets: None,
            size: (1280, 720),
            camera: None,
            vsync: true,
            fog: true,
            quality: Quality::Medium,
            seed: None,
            weather: None,
            fog_level: None,
            night: false,
            blackout: None,
            time: None,
            sun_shadows: None,
            shadow_res: None,
            cascades: None,
            shadow_distance: None,
            shadow_filter: None,
            local_shadows: None,
            local_shadow_res: None,
            contact_shadows: None,
            ssao: None,
            screenshot: None,
            bench: None,
        }
    }
}

impl Options {
    /// Returns `None` when help was requested.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>> {
        let mut options = Options::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "-h" | "--help" => return Ok(None),
                "--assets" => options.assets = Some(PathBuf::from(value()?)),
                "--size" => options.size = parse_size(&value()?)?,
                "--camera" => options.camera = Some(parse_camera(&value()?)?),
                "--no-vsync" => options.vsync = false,
                "--no-fog" => options.fog = false,
                "--quality" => {
                    let text = value()?;
                    options.quality = Quality::parse(&text).with_context(|| format!("unknown quality {text:?}"))?;
                }
                "--weather" => {
                    let text = value()?;
                    options.weather =
                        Some(WeatherKind::parse(&text).with_context(|| format!("unknown weather {text:?}"))?);
                }
                "--fog-level" => {
                    let text = value()?;
                    options.fog_level =
                        Some(FogLevel::parse(&text).with_context(|| format!("unknown fog level {text:?}"))?);
                }
                "--seed" => options.seed = Some(value()?.parse().context("--seed expects a whole number >= 0")?),
                "--night" => options.night = true,
                "--blackout" => {
                    let quadrant: u8 = value()?.parse().context("--blackout expects 0..3")?;
                    if quadrant > 3 {
                        bail!("--blackout expects 0..3");
                    }
                    options.blackout = Some(quadrant);
                }
                "--time" => {
                    let time: f32 = value()?.parse().context("--time expects seconds")?;
                    if !time.is_finite() || time < 0.0 {
                        bail!("--time must be a non-negative number");
                    }
                    options.time = Some(time);
                }
                "--sun-shadows" => options.sun_shadows = Some(parse_switch(&value()?)?),
                "--shadow-res" => options.shadow_res = Some(parse_resolution(&value()?)?),
                "--cascades" => {
                    let count: u32 = value()?.parse().context("--cascades expects 1..4")?;
                    if !(1..=4).contains(&count) {
                        bail!("--cascades expects 1..4");
                    }
                    options.cascades = Some(count);
                }
                "--shadow-distance" => {
                    let metres: f32 = value()?.parse().context("--shadow-distance expects metres")?;
                    if !(1.0..=1000.0).contains(&metres) {
                        bail!("--shadow-distance must be 1..1000 m");
                    }
                    options.shadow_distance = Some(metres);
                }
                "--shadow-filter" => {
                    options.shadow_filter = Some(match value()?.as_str() {
                        "hard" => ShadowFilter::Hard,
                        "pcf" => ShadowFilter::Pcf,
                        "soft" => ShadowFilter::Soft,
                        other => bail!("unknown shadow filter {other:?}"),
                    });
                }
                "--local-shadows" => {
                    let budget: u32 = value()?.parse().context("--local-shadows expects 0..4")?;
                    if budget > 4 {
                        bail!("--local-shadows expects 0..4");
                    }
                    options.local_shadows = Some(budget);
                }
                "--local-shadow-res" => options.local_shadow_res = Some(parse_resolution(&value()?)?),
                "--contact-shadows" => options.contact_shadows = Some(parse_switch(&value()?)?),
                "--ssao" => options.ssao = Some(parse_switch(&value()?)?),
                "--screenshot" => options.screenshot = Some(PathBuf::from(value()?)),
                "--bench" => {
                    let frames: u32 = value()?.parse().context("--bench expects a frame count")?;
                    if frames == 0 {
                        bail!("--bench needs at least one frame");
                    }
                    options.bench = Some(frames);
                }
                other => bail!("unknown option {other:?}\n\n{USAGE}"),
            }
        }
        Ok(Some(options))
    }
}

impl Options {
    /// The quality preset with any shadow/occlusion overrides applied.
    pub fn render_settings(&self) -> RenderSettings {
        let mut settings = self.quality.render_settings();
        let preset_sun = settings.sun_shadows.unwrap_or(SunShadows {
            resolution: 1024,
            cascades: 4,
            distance: 90.0,
            filter: ShadowFilter::Pcf,
        });
        let sun_on = self.sun_shadows.unwrap_or(settings.sun_shadows.is_some());
        settings.sun_shadows = sun_on.then(|| SunShadows {
            resolution: self.shadow_res.unwrap_or(preset_sun.resolution),
            cascades: self.cascades.unwrap_or(preset_sun.cascades),
            distance: self.shadow_distance.unwrap_or(preset_sun.distance),
            filter: self.shadow_filter.unwrap_or(preset_sun.filter),
        });
        let local = &mut settings.local_shadows;
        local.budget = self.local_shadows.unwrap_or(local.budget);
        local.resolution = self.local_shadow_res.unwrap_or(local.resolution);
        local.filter = self.shadow_filter.unwrap_or(local.filter);
        if let Some(on) = self.contact_shadows {
            settings.contact_shadows = on.then(ContactShadows::default);
        }
        if let Some(on) = self.ssao {
            settings.ssao = on.then(Ssao::default);
        }
        settings
    }
}

fn parse_switch(text: &str) -> Result<bool> {
    match text {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => bail!("expected on or off, got {text:?}"),
    }
}

fn parse_resolution(text: &str) -> Result<u32> {
    let size: u32 = text.parse().context("shadow resolution expects a number")?;
    if ![256, 512, 1024, 2048, 4096].contains(&size) {
        bail!("shadow resolution must be 256, 512, 1024, 2048 or 4096");
    }
    Ok(size)
}

fn parse_size(text: &str) -> Result<(u32, u32)> {
    let (w, h) = text.split_once('x').with_context(|| format!("size {text:?} must look like 1280x720"))?;
    let size = (w.parse::<u32>()?, h.parse::<u32>()?);
    if size.0 == 0 || size.1 == 0 || size.0 > 8192 || size.1 > 8192 {
        bail!("size {text:?} out of range");
    }
    Ok(size)
}

fn parse_camera(text: &str) -> Result<[f32; 5]> {
    let values: Vec<f32> = text.split(',').map(|v| v.trim().parse::<f32>()).collect::<Result<_, _>>()?;
    let pose: [f32; 5] = values.try_into().map_err(|_| anyhow::anyhow!("--camera needs x,y,z,yaw,pitch"))?;
    if pose.iter().any(|v| !v.is_finite()) {
        bail!("--camera values must be finite");
    }
    Ok(pose)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Options>> {
        Options::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_all_options() {
        let options = parse(&[
            "--assets",
            "dir",
            "--size",
            "640x360",
            "--camera",
            "1,2,3,90,-10",
            "--no-vsync",
            "--no-fog",
            "--quality",
            "high",
            "--seed",
            "1234",
            "--weather",
            "storm",
            "--fog-level",
            "heavy",
            "--night",
            "--blackout",
            "2",
            "--time",
            "4.5",
            "--screenshot",
            "a.png",
            "--bench",
            "5",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(options.assets, Some(PathBuf::from("dir")));
        assert_eq!(options.size, (640, 360));
        assert_eq!(options.camera, Some([1.0, 2.0, 3.0, 90.0, -10.0]));
        assert!(!options.vsync);
        assert!(!options.fog);
        assert_eq!(options.quality, Quality::High);
        assert_eq!(options.seed, Some(1234));
        assert_eq!(options.weather, Some(WeatherKind::Storm));
        assert_eq!(options.fog_level, Some(FogLevel::Heavy));
        assert!(options.night);
        assert_eq!(options.blackout, Some(2));
        assert_eq!(options.time, Some(4.5));
        assert_eq!(options.screenshot, Some(PathBuf::from("a.png")));
        assert_eq!(options.bench, Some(5));
    }

    #[test]
    fn defaults_and_help() {
        assert_eq!(parse(&[]).unwrap(), Some(Options::default()));
        assert_eq!(parse(&["--help"]).unwrap(), None);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(parse(&["--size", "640"]).is_err());
        assert!(parse(&["--size", "0x10"]).is_err());
        assert!(parse(&["--camera", "1,2,3"]).is_err());
        assert!(parse(&["--camera", "1,2,3,nan,0"]).is_err());
        assert!(parse(&["--bench", "0"]).is_err());
        assert!(parse(&["--assets"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--seed", "-1"]).is_err());
        assert!(parse(&["--seed", "abc"]).is_err());
        assert!(parse(&["--weather", "snow"]).is_err());
        assert!(parse(&["--quality", "ultra"]).is_err());
        assert!(parse(&["--blackout", "4"]).is_err());
        assert!(parse(&["--time", "-1"]).is_err());
        assert!(parse(&["--shadow-res", "1000"]).is_err());
        assert!(parse(&["--cascades", "5"]).is_err());
        assert!(parse(&["--ssao", "yes"]).is_err());
        assert!(parse(&["--local-shadows", "5"]).is_err());
    }

    #[test]
    fn shadow_overrides_apply_on_top_of_the_preset() {
        let options = parse(&["--quality", "low", "--sun-shadows", "on", "--shadow-res", "2048", "--cascades", "2"])
            .unwrap()
            .unwrap();
        let sun = options.render_settings().sun_shadows.unwrap();
        assert_eq!((sun.resolution, sun.cascades, sun.distance), (2048, 2, 90.0));
        let options = parse(&["--quality", "high", "--ssao", "on", "--contact-shadows", "off", "--local-shadows", "0"])
            .unwrap()
            .unwrap();
        let settings = options.render_settings();
        assert!(settings.ssao.is_some() && settings.contact_shadows.is_none());
        assert_eq!(settings.local_shadows.budget, 0);
        assert_eq!(settings.sun_shadows.unwrap().resolution, 2048);
        let medium = parse(&[]).unwrap().unwrap().render_settings();
        assert_eq!(medium, Quality::Medium.render_settings());
    }
}
