# DEAD SECTOR — Rust engine

Native Rust + wgpu (Vulkan first, DX12 fallback on Windows) rewrite of Dead Sector.
It is developed in stages next to the existing HTML and Godot versions, which are
not modified.

**Stage 2 (current):** the existing city from `godot/assets/city.json` with the
Godot version's look (lighting, weather, surfaces, lamps, mist, PSX post
pass) and a free-fly camera. No gameplay yet.

## Build and run (Windows)

1. Install Rust from <https://rustup.rs> (default MSVC toolchain; it asks for the
   Visual Studio C++ Build Tools if they are missing).
2. From the repository's `engine/` folder:

```
cargo run --release
```

The executable is `engine\target\release\dead-sector.exe`. Started anywhere inside
the repository checkout it finds `godot\assets` by itself; otherwise pass
`--assets <folder>` or set `DS_ASSETS`. A future packaging stage will place the
assets next to the `.exe`.

Controls: click the window to capture the mouse, Esc releases it. WASD move,
mouse look, Space/Ctrl up/down, Shift fast, F11 fullscreen. The title bar shows
FPS, frame time, draw calls, triangles, GPU and camera position.

Options (`dead-sector --help`):

| Option | Meaning |
|---|---|
| `--size 1920x1080` | Window / capture size |
| `--no-vsync` | Uncapped FPS (use this to measure performance) |
| `--camera x,y,z,yaw,pitch` | Start pose (metres, degrees; yaw 0 looks along −Z) |
| `--quality low\|medium\|high` | Graphics quality (default medium, as in Godot) |
| `--weather overcast\|rain\|storm\|clear` | Weather (default: the seeded wave 1 roll) |
| `--fog-level light\|medium\|heavy` | Fog level (default: the seeded wave 1 roll) |
| `--night` | Night lighting |
| `--blackout 0..3` | Cut power to one city quadrant |
| `--no-fog` | Debug view without fog or mist |
| `--time 3.5` | Fixed effect time (mist, lens rain, grain) for repeatable screenshots |
| `--screenshot out.png` | Render one frame offscreen and exit |
| `--bench 300` | Render N offscreen frames and print the average time |

`WGPU_BACKEND=dx12` (or `vulkan`) forces a backend instead of the default
Vulkan → DX12 order.

## Layout

| Crate | Purpose |
|---|---|
| `crates/ds-assets` | Typed `city.json` loader, validation, GPU-ready scene buffers (no GPU code) |
| `crates/ds-render` | wgpu renderer: instanced city, Godot lighting, weather/quality tables, glow, tonemap, PSX pass, offscreen capture |
| `crates/ds-app` | `dead-sector` executable: window, input, command line |

## Tests

```
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo test -p ds-render --test offscreen -- --include-ignored   # needs a GPU or software Vulkan
```

Without a GPU (Linux CI), point `VK_ICD_FILENAMES` at Mesa lavapipe
(`/usr/share/vulkan/icd.d/lvp_icd.json`) or SwiftShader.

Cross-compiling the Windows `.exe` from Linux:

```
rustup target add x86_64-pc-windows-gnu    # plus the mingw-w64 package
cargo build --release --target x86_64-pc-windows-gnu -p ds-app
```

## Rendering

Each frame: the city and atmosphere layers render into an HDR target at the
PSX pixel grid (at most 540 lines, like `pixels.gdshader`), then Godot-style
glow, filmic tonemap and colour adjustments, then the pixels pass scales up to
the window. The 3D cost does not grow with window resolution.

Ported from the Godot version (`godot/scripts`, `godot/shaders`):

- Lighting: Burley diffuse, GGX specular, ambient colour, sky reflections,
  exponential fog, sun and environment from `climate.gd` (weather roll, day and
  night, wet asphalt).
- Surfaces: `surface.gdshader` (brick, tile, wood, grime, rain streaks);
  `powered.gdshader` lights and signs with blackouts.
- Lights: store light and the 4 (medium) or 8 (high) nearest street lamps,
  with Godot's omni falloff; additive light pools; ground mist sheets.
- Post: glow levels 3 and 5, filmic tonemap (white 5), contrast 1.1,
  saturation 0.8, and `pixels.gdshader` (grid, lens rain, sharpening, grade,
  vignette, scanlines, grain).

Not yet: sun shadows, the player flashlight, lightning, rain/ash/hive
particles, per-seed city regeneration, player, weapons, enemies, UI, audio,
networking.

Known approximations: the mist noise is generated in Rust (FastNoiseLite is
not reproduced bit for bit); sky reflections blur by roughness instead of a
filtered radiance map; Godot eases weather changes over a few seconds, the
viewer shows the settled state.
