# DEAD SECTOR — Rust engine

Native Rust + wgpu (Vulkan first, DX12 fallback on Windows) rewrite of Dead Sector.
It is developed in stages next to the existing HTML and Godot versions, which are
not modified.

**Stage 1 (current):** a window that renders the existing city from
`godot/assets/city.json` with a free-fly camera. No gameplay yet.

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
| `--no-fog` | Debug view without fog |
| `--screenshot out.png` | Render one frame offscreen and exit |
| `--bench 300` | Render N offscreen frames and print the average time |

`WGPU_BACKEND=dx12` (or `vulkan`) forces a backend instead of the default
Vulkan → DX12 order.

## Layout

| Crate | Purpose |
|---|---|
| `crates/ds-assets` | Typed `city.json` loader, validation, GPU-ready scene buffers (no GPU code) |
| `crates/ds-render` | wgpu renderer: instanced draws, materials, camera, lighting, offscreen capture |
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

## Stage 1 scope

Included: all 313 batches / 63,969 instances / ~865k triangles of the exported
city in one instanced draw per batch, per-instance and per-vertex colours,
label textures with alpha cutout, double-sided and transparent materials, the
default sun/ambient/fog of `main.gd`, reversed-Z depth.

Not yet: procedural surface detail (`surface.gdshader`), the PSX post pass,
tonemapping/glow, shadows, street and store lights, per-seed city regeneration,
weather and day cycle, player, weapons, enemies, UI, audio, networking.
