# Sector Engine + Dead Sector — architecture

Sector Engine is a reusable Rust + wgpu game engine. Dead Sector is its first
game. Everything lives in `engine/` for now; the original Godot project
(`godot/`) and HTML game (repository root) are not modified.

```
engine/crates/
  sector-assets   (engine)  scene data, images, colour helpers
  sector-render   (engine)  renderer: lighting, shadows, AO, post chain
  dead-sector     (game)    city.json, climate, lamps, look, surface hook, executable
```

`dead-sector` depends on both engine crates. The engine crates never depend
on the game. Engine growth is driven by what Dead Sector actually needs.

## What is reusable today

| System | Crate / module | Reusable how |
|---|---|---|
| Scene data | `sector-assets::scene` | `SceneBuilder`: add meshes, materials, instances and instanced draws; any game converts its formats into this |
| Materials | `sector-assets::MaterialParams` | Albedo, emission, roughness, metallic, unlit, alpha test, texture, pass (opaque / double-sided / transparent) and a game **shading id** |
| Images | `sector-assets::load_image` | Any PNG; the game decides paths |
| GPU setup | `sector-render::gpu` | Vulkan first, DX12 fallback on Windows, `WGPU_BACKEND` override |
| Camera | `sector-render::view::Camera` | Position, yaw/pitch, FOV, near/far, reversed-Z projection |
| Lighting | `sector-render::Lighting`, `PointLight` | Sun, ambient, exponential fog, sky gradient; up to 16 point lights per frame with Godot's omni falloff |
| Shading | `scene.wgsl` | Burley diffuse, GGX specular, sky reflections, fog, HDR output |
| Surface hook | `RendererConfig::surface_hook` | Game WGSL `surface_hook(shading, input)` changes albedo/roughness per material, with per-frame `frame.game` values |
| Sun shadows | `shadows.rs`, `shadow_maps.rs` | Cascaded shadow maps, 1–4 cascades, 1024–4096 maps, distance, hard / PCF / soft filtering, texel-snapped stable cascades, per-cascade caster culling (instances fetched from a storage buffer by index) |
| Local shadows | same | Six-face shadow maps for the nearest `cast_shadows` point lights within a budget (0–4); casters culled to each light's range; a light's own fixture is ignored |
| Contact shadows | `screen_space.rs` | Screen-space ray march towards the sun from the depth prepass |
| SSAO | `screen_space.rs` | Normal-oriented hemisphere SSAO with depth-aware blur; ambient light only |
| Atmosphere layers | `Atmosphere`, `Renderer::set_decals` | Additive ground decals and scrolling noise fog sheets |
| Post chain | `PostSettings`, `RetroSettings` | Glow mip chain, filmic tonemap, contrast/saturation; optional retro pass (render-line cap, sharpening, split-tone grade, vignette, scanlines, grain, lens rain, two edge tints) |
| Quality | `RenderSettings` | Plain values (render lines, shadows, local budget, contact, SSAO, atmosphere, post detail); games map their presets onto it |
| Profiling | `Renderer::bench` | Wall time per frame and per-pass GPU time (timestamp queries when supported) |
| Offscreen | `Renderer::capture` | Screenshots and image-based tests without a window |

## What stays in Dead Sector

| Module | Content |
|---|---|
| `city.rs` | `godot/assets/city.json` format and validation |
| `content.rs` | city → `SceneBuilder` conversion with `bridge.gd`'s material rules; label textures; asset folder lookup |
| `climate.rs` | Weather roll, day/night, wetness, fog levels (`climate.gd`), quality presets |
| `lamps.rs` | Store light, nearest street lamps, blackout districts, lens-rain shelter |
| `look.rs` | The grim grade and damage tints, lamp pools and mist |
| `surface.wgsl` | Surface hook: procedural brick, tile, wood, grime, wet asphalt, blackout dimming |
| `frame.rs` | Builds `FrameInputs` from game state each frame |
| `main.rs`, `window.rs`, `options.rs` | Executable, window loop, fly camera, command line |

Zombies, waves, weapons, inventory, missions and UI will also live in the game
crate. None of them are dependencies of the engine.

## Frame

```
sun cascades (depth)  ─┐
local light faces     ─┤  shadow maps
depth+normal prepass  ─┤  (only when SSAO or contact shadows are on)
SSAO + contact, blur  ─┘
scene: opaque → double-sided → transparent → decals → fog sheets   (HDR, ≤ render-line cap)
glow chain → tonemap + grade → retro pass → window / capture
```

With shadows, contact shadows and SSAO off, the frame is identical to the
Stage 2 renderer. Screenshots match the Stage 2 baseline within one 8-bit
level.

## Planned engine systems (not built yet)

These have no user in the code yet, so they will be added when a Dead Sector
stage needs them, not before:

- **Input** (action map, key/mouse/controller bindings, sensitivity) and
  **config files**: player controller stage.
- **World/entities**: transforms plus handle-based storage, no full ECS unless
  it becomes justified; **collision/physics** (AABB world, ray casts, capsule
  sweeps): player controller stage. Zombie AI and movement rules stay in the
  game.
- **Fixed-step simulation** separated from rendering and input, ready for
  server authority, interpolation and prediction: player controller stage.
- **Audio** (sound effects, music, 3D sound, volume buses): audio stage.
- **Mesh and animation import (glTF)**: weapons stage.

## Limitations (Stage 2b)

- Sun cascades are culled on the CPU each frame: an instance is drawn into a
  cascade only if its bounding sphere overlaps the cascade's frustum slice in
  light space (plus filter margin and everything towards the sun). Shadow maps
  are bit-identical to unculled rendering. At the High preset from the spawn
  point this drops cascade draws from 4 × 63,969 to about 47,000 instances. The
  far cascade still draws about half the city; GPU-driven culling and caching
  static cascades are the next options.
- Local shadow maps use 32-bit depth: 4 lights × 6 faces × 1024² ≈ 96 MB on
  high.
- SSAO and contact shadows run at the 3D resolution. SSAO only darkens ambient
  light, as in Godot, so it is subtle in sun- or lamp-lit scenes.
- Shadows are single-sample per filter tap; there is no shadow-map caching
  for static lights.
- Not yet: the flashlight's shadow (player stage), lightning, particles.
