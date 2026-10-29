# Sector Engine + DEAD SECTOR

Sector Engine is a reusable Rust + wgpu engine (Vulkan first, DX12 fallback on
Windows); Dead Sector is its first game, migrated in stages from the Godot
version. The HTML and Godot versions are not modified. See
[ARCHITECTURE.md](ARCHITECTURE.md) for what is engine and what is game.

**Stage 5 (current):** the existing city with the Godot version's look,
cascaded sun shadows, local-light shadows, contact shadows and optional SSAO,
a first-person player that walks, sprints and jumps with collision, and
seeded city generation (`--seed N`): new plot layout, modular multi-storey
buildings with windows, doors, stairs and roofs, and furnished floors.
Without `--seed` the exported city is shown unchanged. No combat yet.

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
mouse look, Shift sprint, Space jump, F11 fullscreen. V toggles a free-fly
debug camera (Space/Ctrl up/down, Shift fast); `--camera` starts in it. The
title bar shows FPS, frame time, draw calls, triangles, GPU, walk/fly and
camera position.

Options (`dead-sector --help`):

| Option | Meaning |
|---|---|
| `--size 1920x1080` | Window / capture size |
| `--no-vsync` | Uncapped FPS (use this to measure performance) |
| `--camera x,y,z,yaw,pitch` | Start pose (metres, degrees; yaw 0 looks along −Z) |
| `--seed <n>` | Generate the city layout, modular buildings and interiors from a seed (default: the exported seed-42 city, unchanged) |
| `--quality low\|medium\|high` | Graphics quality (default medium, as in Godot) |
| `--weather overcast\|rain\|storm\|clear` | Weather (default: the seeded wave 1 roll) |
| `--fog-level light\|medium\|heavy` | Fog level (default: the seeded wave 1 roll) |
| `--night` | Night lighting |
| `--blackout 0..3` | Cut power to one city quadrant |
| `--no-fog` | Debug view without fog or mist |
| `--time 3.5` | Fixed effect time (mist, lens rain, grain) for repeatable screenshots |
| `--sun-shadows on\|off` | Cascaded sun shadows (off on low) |
| `--shadow-res 1024\|2048` | Sun shadow map size (also 512, 4096) |
| `--cascades 1..4` | Sun shadow cascades |
| `--shadow-distance 90` | Sun shadow distance in metres |
| `--shadow-filter hard\|pcf\|soft` | Shadow filtering |
| `--local-shadows 0..4` | Shadowed point lights (nearest first) |
| `--local-shadow-res 512` | Point-light shadow face size |
| `--contact-shadows on\|off` | Screen-space contact shadows |
| `--ssao on\|off` | Ambient occlusion (off in every preset) |
| `--screenshot out.png` | Render one frame offscreen and exit |
| `--bench 300` | Render N offscreen frames; prints frame time and per-pass GPU time |

`WGPU_BACKEND=dx12` (or `vulkan`) forces a backend instead of the default
Vulkan → DX12 order.

## Layout

| Crate | Purpose |
|---|---|
| `crates/sector-assets` | Engine: scene builder and buffers, materials, images, colour helpers, seeded RNG, building modules |
| `crates/sector-render` | Engine: renderer, lighting, shadows, SSAO, post chain, profiling, offscreen capture |
| `crates/sector-physics` | Engine: static box collision world and kinematic character mover |
| `crates/dead-sector` | Game: city data, seeded city/building/interior generation, player, climate, lamps, look, surface hook; `dead-sector` executable |

Presets (`--quality`):

| | Low | Medium (default) | High |
|---|---|---|---|
| Sun shadows | off | 4 cascades, 1024², 90 m, PCF | 4 cascades, 2048², 120 m, soft |
| Local shadows | off | 1 light, 512² faces | 4 lights, 1024² faces |
| Contact shadows | off | on | on |
| SSAO | off | off | off |
| Street lamp lights | 0 | 4 | 8 |

## Tests

```
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo test --workspace -- --ignored   # GPU tests: needs a GPU or software Vulkan
```

Without a GPU (Linux CI), point `VK_ICD_FILENAMES` at Mesa lavapipe
(`/usr/share/vulkan/icd.d/lvp_icd.json`) or SwiftShader.

Cross-compiling the Windows `.exe` from Linux:

```
rustup target add x86_64-pc-windows-gnu    # plus the mingw-w64 package
cargo build --release --target x86_64-pc-windows-gnu -p dead-sector
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the frame structure, reusable
systems and current limitations.
