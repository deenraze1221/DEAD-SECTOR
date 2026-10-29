//! Command-line options.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub const USAGE: &str = "\
DEAD SECTOR (Rust engine, stage 1)

USAGE: dead-sector [options]

  --assets <dir>          Folder containing city.json (default: auto-detect)
  --size <W>x<H>          Window / capture size (default 1280x720)
  --camera <x,y,z,yaw,pitch>
                          Start camera: position in metres, angles in degrees
  --no-vsync              Present without vsync (uncapped FPS)
  --no-fog                Disable fog (debug view of the whole city)
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
    pub screenshot: Option<PathBuf>,
    pub bench: Option<u32>,
}

impl Default for Options {
    fn default() -> Self {
        Options { assets: None, size: (1280, 720), camera: None, vsync: true, fog: true, screenshot: None, bench: None }
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
    }
}
