# Sector Engine + Dead Sector — architecture

Sector Engine is a reusable Rust + wgpu game engine. Dead Sector is its first
game. Everything lives in `engine/` for now; the original Godot project
(`godot/`) and HTML game (repository root) are not modified.

```
engine/crates/
  sector-assets   (engine)  scene data, images, colour helpers, seeded RNG
  sector-render   (engine)  renderer: lighting, shadows, AO, post chain
  sector-physics  (engine)  static box world, grid broadphase, kinematic box mover
  dead-sector     (game)    city.json, city/interior generation, player, climate, lamps, look, surface hook, executable
```

`dead-sector` depends on both engine crates. The engine crates never depend
on the game. Engine growth is driven by what Dead Sector actually needs.

## What is reusable today

| System | Crate / module | Reusable how |
|---|---|---|
| Scene data | `sector-assets::scene` | `SceneBuilder`: add meshes, materials, instances and instanced draws; any game converts its formats into this |
| Materials | `sector-assets::MaterialParams` | Albedo, emission, roughness, metallic, unlit, alpha test, texture, pass (opaque / double-sided / transparent) and a game **shading id** |
| Images | `sector-assets::load_image` | Any PNG; the game decides paths |
| Random numbers | `sector-assets::rng::Rng` | PCG32: same seed, same sequence on every platform (integer ranges, floats, shuffles); for anything every co-op client must build identically |
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
| Collision | `sector-physics::World` | Static AABBs in a 4 m XZ grid (huge boxes checked always); `overlaps`, and `move_box`: vertical then X then Z swept moves with sliding, a skin gap, step-up onto low ledges, grounded/ceiling/blocked flags. Deterministic `f32` maths, no tunnelling at any speed |

## What stays in Dead Sector

| Module | Content |
|---|---|
| `city.rs` | `godot/assets/city.json` format and validation, including owners, roles, collider kinds and asset bounds |
| `citygen.rs` | Seeded plot layout (`city_generator.gd`) |
| `interiors.rs` | Seeded floor plans and furniture (`interior_layout.gd`) |
| `player.rs` | First-person player with `player.gd`'s tuning on a fixed 60 Hz step; the city's collision world |
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

## Seeded city (Stage 3)

`--seed N` runs `citygen::generate` on the loaded city before the scene is
built; without it nothing is generated and the exported seed-42 city renders
pixel-identically to Stage 2b. Generation is pure CPU work on the city data
(about 40–110 ms), so the renderer is unchanged.

1. **Prefab capture.** Each exported `interior-*` item becomes a prefab: its
   visible parts and colliders relative to the item's anchor (position, yaw).
   Up to five prefabs per furniture type.
2. **Layout.** The 10 × 10 plots keep their positions. The corners and the
   four store-block plots are fixed (plus any plot a shared object reaches
   into; none in the current export). Every other plot gets a building from
   its own district: each type at least once, extras chosen with replacement.
   All of a source plot's instances, colliders and assets are copied into the
   target plot, shifted by the plot-centre difference; owner ids are renamed
   per plot, so copies stay distinct.
3. **Interiors.** Exported furniture and partitions are removed except in the
   store. Each other building gets one of three floor plans (side rooms off a
   corridor, or a front/back split with an optional washroom), plaster
   partitions with colliders, and furniture from a per-building-type program.
   An item goes against a room wall only if its padded footprint misses every
   placed item and every reserved doorway/corridor box; desks, tables,
   workbenches and counters get a chair in front when it fits.

The layout and interior RNG streams are seeded with `seed` and
`seed ^ 0x524f4f4d`, as in Godot. The algorithms follow the Godot scripts, but
the random sequence is PCG32, not Godot's, so a seed gives a different city
than the Godot build does. The weather roll also uses the chosen seed.

Tests (`tests/citygen.rs`, real city data, four seeds): same seed → identical
output; different seeds → different layouts; a pinned seed-7 layout; fixed
plots and the store unchanged; every district keeps all its building types;
the result validates and builds; no two furniture items or an item and a
partition overlap; doorways and corridors stay clear of furniture and walls;
furniture stays inside its rooms; each furniture collider lies within its
drawn geometry and each partition collider equals its drawn box. The Windows
build renders the same seeded city pixel-identically under Wine.

## Player and physics (Stage 4)

`player::collision_world` turns the city's colliders (walls, partitions,
furniture, props, structures; the current or seeded city) plus Godot's ground
box (450 × 450 m, top at y = 0) into a `sector-physics::World`: about 7,300
boxes, built in ~1 ms.

The player is a 0.64 × 1.8 × 0.64 m box (Godot's capsule: radius 0.32, height
1.8), eyes at 1.65 m. Each 1/60 s step follows `player.gd`: walk 4.5 m/s,
sprint 7.5 m/s for 5 s of stamina (recharging, with an exhausted lock-out),
acceleration 31.3 / 47.8 m/s² (moving / stopping), gravity 20 m/s², jump
4.95 m/s (about 0.61 m), pitch limits −60° / +88°, 0.002 rad per mouse pixel.
It can step onto ledges up to 0.3 m (Godot's capsule rides over curbs). Below
y = −20 it is put back at the spawn. The window runs whole steps for the
elapsed time (at most 8 per frame) and interpolates the eye between the last
two steps, so movement is identical at any frame rate.

Controls: click to capture the mouse, Esc releases it; WASD, mouse look,
Shift sprint, Space jump; V toggles the free-fly debug camera. `--camera`
starts in fly mode; `--screenshot` and `--bench` are unchanged.

Tests: physics unit tests (landing, sliding, no tunnelling, step heights,
ceilings, escaping overlaps, grid vs brute force); player unit tests (speeds,
yaw-relative strafing, sprint and stamina, jump height, walls, respawn, pitch
clamp, interpolation); real-city tests (`tests/player_city.rs`): spawn is
clear; one minute of random play from the street and inside buildings in the
exported and seed-7 cities never overlaps a collider or goes below ground;
walking at every building is stopped; furniture blocks; identical inputs give
bit-identical positions. A step costs about 1 µs.

## Planned engine systems (not built yet)

These have no user in the code yet, so they will be added when a Dead Sector
stage needs them, not before:

- **Input** (action map, rebindable keys, controller, sensitivity setting)
  and **config files**: when a settings menu exists. Stage 4 reads winit keys
  directly in `window.rs`.
- **World/entities**: transforms plus handle-based storage, no full ECS unless
  it becomes justified: zombies stage. **Ray casts** against `World`:
  weapons stage. Zombie AI and movement rules stay in the game.
- **Network-ready simulation**: the player already steps at a fixed 60 Hz
  from an `Input` value, so the same steps can run on a server and be
  predicted on clients in the co-op stage.
- **Audio** (sound effects, music, 3D sound, volume buses): audio stage.
- **Mesh and animation import (glTF)**: weapons stage.

## Limitations (Stage 4)

- The player is a box, not a capsule: it does not round corners and stands
  on ledge edges up to its full half-width.
- No slopes: the city is all boxes. Ledges above 0.3 m need a jump.
- Under Wine on a virtual X display only the first key press reaches the
  window (the Stage 3 build behaves the same); this could not be checked on
  real Windows here.

## Limitations (Stage 3)

- Seeded cities differ from the Godot build's for the same seed (different
  RNG). Co-op clients must all run this engine's generator.
- Godot's plan-0 corridor box started 1 cm inside the back-room walls; here
  it starts at the wall face.
- `source_bounds` is remapped with the assets; nothing renders from it yet.

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
