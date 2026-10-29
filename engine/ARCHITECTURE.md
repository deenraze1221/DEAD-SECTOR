# Sector Engine + Dead Sector — architecture

Sector Engine is a reusable Rust + wgpu game engine. Dead Sector is its first
game. Everything lives in `engine/` for now; the original Godot project
(`godot/`) and HTML game (repository root) are not modified.

```
engine/crates/
  sector-assets   (engine)  scene data, images, colour helpers, seeded RNG, building modules, material packs
  sector-render   (engine)  renderer: lighting, shadows, AO, post chain
  sector-physics  (engine)  static box world, grid broadphase, kinematic box mover
  dead-sector     (game)    city.json, city/building/interior generation, player, climate, lamps, look, surface hook, executable
```

`dead-sector` depends on both engine crates. The engine crates never depend
on the game. Engine growth is driven by what Dead Sector actually needs.

## What is reusable today

| System | Crate / module | Reusable how |
|---|---|---|
| Scene data | `sector-assets::scene` | `SceneBuilder`: add meshes, materials, instances and instanced draws; any game converts its formats into this |
| Materials | `sector-assets::MaterialParams` | Albedo, emission, roughness, metallic, unlit, alpha test, texture, pass (opaque / double-sided / transparent) and a game **shading id** |
| Images | `sector-assets::load_image` | Any PNG; the game decides paths |
| Building modules | `sector-assets::modular` | Box-based kit: `wall` (door and window openings, banded: one strip below and above the window band, piers, a pane per window, a frame inside each door), `facade` (identical storeys stacked as full-height piers plus spandrels per window column), `slab` (rectangle minus holes), `stairs` (`Flight` of solid steps), `parapet`. No two boxes share volume, so no faces fight. Each box has a material `Slot` and a `solid` flag, for drawing as unit-box instances and for collision; `WallSpec::outward` puts sills on the outdoor side |
| Random numbers | `sector-assets::rng::Rng` | PCG32: same seed, same sequence on every platform (integer ranges, floats, shuffles); for anything every co-op client must build identically |
| GPU setup | `sector-render::gpu` | Vulkan first, DX12 fallback on Windows, `WGPU_BACKEND` override |
| Camera | `sector-render::view::Camera` | Position, yaw/pitch, FOV, near/far, reversed-Z projection |
| Lighting | `sector-render::Lighting`, `PointLight` | Sun, ambient, exponential fog, sky gradient; up to 16 point lights per frame with Godot's omni falloff |
| Shading | `scene.wgsl` | Burley diffuse, GGX specular, sky reflections, fog, HDR output |
| Texture synthesis | `sector-assets::texgen` | Periodic value noise, fBm and Voronoi, wrap-around blur, normals from height (DirectX), cavity AO, a seam metric and 3 × 3 tiling previews: anything built from them tiles seamlessly |
| Material packs | `sector-assets::materials` | `.smat` files: colour (BC1 sRGB), tangent normal XY (BC5, DirectX +Y = +v) and AO + roughness (BC5), each with a full mip chain; encode (mips in linear light, normals renormalised) and decode (fallback) |
| Surface maps | `RendererConfig::surface_maps`, `MaterialParams::{surface_map, tiling, surface_faces}` | Any number of packs as three shared texture arrays (one layer each; packs of different sizes share the smallest size, larger ones contributing from their matching mip), 16× anisotropic trilinear sampler; world-space box projection on the face's dominant axis (no stretching on scaled instances), tangent frame from the projection, AO on indirect light only, face mask (up / down / sides). BC when the adapter has it, else decoded to ≤ 1024². `max_map_size` drops top mips. Shadows use the face normal |
| Glass | `MaterialParams::glass` (transparent pass) | Front faces only (one layer per pane), Schlick-Fresnel opacity, reflections not dimmed by the blend, no screen-space occlusion from what is behind; like all transparent draws, not a shadow caster and not in the depth prepass |
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
| `buildings.rs` | Seeded modular buildings: storeys, facades, door, slabs, switchback stairwell with guard rails, roof |
| `surfaces.rs` | Loads `engine/assets/materials/library.json` and the packs it names, and assigns them to city materials by colour or generated name |
| `examples/import_materials.rs` | Converts PBR texture folders (colour, normal, ambient, roughness or specular) into `.smat` packs |
| `examples/generate_materials.rs` | The 16 stylized materials, procedurally (recipes per material), with seam checks and previews |
| `emit.rs` | Adds generated instances to the existing batch with the same geometry and materials, and colliders |
| `interiors.rs` | Seeded floor plans and furniture (`interior_layout.gd`), on every storey |
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

## Modular buildings (Stage 5)

With `--seed`, after the layout step every building except the store is
rebuilt from modules (the exported shell and its decorations are removed; the
shop sign stays). Without `--seed` nothing changes.

- **Storeys:** 3.3 m each (3.12 m clear + 0.18 m slab), as many as the
  exported height suggests, ±1 by seed for buildings of two or more, 1–12.
  `Building::h` becomes the new height.
- **Walls:** 0.22 m, in the building's exported wall material and tint. The
  ground floor is one `wall` per side with the front door (1.6 × 2.3 m, wood
  frame) centred where the interior plans expect it; upper storeys are one
  `facade` per side. Window bay (1.8–2.4 m), width (0.8–1.15 m) and sill
  height vary by seed. The back wall and side walls stay blind beside the
  stairs.
- **Slabs:** inside the walls, one per level, with a hole over that storey's
  flight; the top one is the roof (dark material) with a 0.5–1.0 m parapet.
- **Stairs:** two 1 m strips along the back wall; flights alternate strip and
  direction each storey (a switchback), 15 steps of 0.22 × 0.28 m (under the
  player's 0.3 m step-up), 1.1 m landings at both ends, mirrored by seed.
  Guard rails (1 m) line every open side of a stair hole except the top end.
- **Interiors:** the ground floor keeps the Stage 3 floor plans; partitions
  are cut around the stairwell and the doorway is kept clear. Each upper
  storey is one open room with two items of a per-type program (plus chairs).
  Furniture never enters the stairwell.
- **Draw calls:** modules, partitions and furniture join the exported batch
  with the same geometry and materials, so the seeded city has the same 313
  batches (Stage 3 had 323) and fewer non-empty draws than the export.

Seed 7: 99 rebuilt buildings, 366 storeys, 13,491 module boxes, 1,819
furniture items, 77,675 instances (Stage 4: 65,708). Generation takes
50–100 ms.

Tests: kit unit tests (wall volume and openings, clear doorway, facade face
coverage sampled on a grid, slab area minus holes, step heights, parapet);
`tests/buildings.rs` on seeded cities: every flight of every building climbs
exactly one storey with the player controller; a player climbs every
building of four or more storeys to the top using movement input only;
walking out from the middle of any upper floor stays inside the building
and never falls through a floor; the front door leads out; structural boxes
do not overlap; every collider is a drawn box; every level's slab covers the
interior minus the stair hole; building heights match their storeys;
stairwells are free of partitions and furniture; seeds vary storeys; draw
count does not grow.

## Clean modules and glass (Stage 5.1)

- **No overlapping boxes.** Every box drawn for a building (walls, facades,
  door frames, sills, panes, slabs, stairs, rails, roof, partitions) is
  disjoint from every other; a test checks all buildings of two seeds and
  synthetic 1-, 4-, 8- and 12-storey buildings (both stair mirrorings).
  Fixes, all by removing shared volume rather than offsetting faces:
  - facade spandrels now fill only the window columns between full-height
    piers (they used to span the whole wall through the piers);
  - one pane per window, exactly filling the opening (was a strip through
    the piers);
  - door frames sit inside the opening (were overlapping the wall);
  - sill ledges stick out of the outer face only, on the outdoor side (on
    front and right walls they used to point indoors);
  - stair-hole rails sit inside the hole (used to share volume with the next
    flight);
  - partitions run to the exterior walls' inner faces (they stopped 8 cm
    short) and the back-room and washroom stubs end at the walls they meet.
- **Glass.** Windows use a generated material (`#8cb8b0`, opacity 0.3,
  roughness 0.12) marked `glass`, drawn in one instanced transparent draw.
  Interiors and furniture show through; sunlight now reaches interiors
  through the windows. A GPU test renders a red wall behind a pane, behind no
  pane and behind an opaque pane and checks the wall stays visible, tinted.
- The export has no glass material, so the default city is unchanged
  (pixel-identical to the Stage 2b baseline).

Seed 7: 81,023 instances (Stage 5: 77,675), one more batch (the glass).
Software renderer frame time unchanged within noise (~502 → ~509 ms).

## Textures and materials (Stage 6)

The user's 12 PBR materials (2048², colour / normal / ambient / displacement /
specular PNGs, 270 MB) were imported with `examples/import_materials.rs` into
`engine/assets/materials/*.smat`: colour and normal at 2048², AO + roughness
at 1024² (roughness = 0.95 − 0.45 × specular), full mip chains,
block-compressed: 113 MB on disk, about 117 MB of VRAM. Displacement is not
used. Normal maps are DirectX (Y−), measured from the data: the green channel
correlates with the height gradient the same way as red. The shader keeps that
convention (green follows +v); a GPU test checks that normals tilted towards
+v darken a wall lit from above and brighten it lit from below.

`library.json` (data, not code) lists the packs with their tile size in
metres and assigns them to city materials by exported colour or generated
material name, with optional `tint` (how much the city's palette colour tints
the texture) and `faces`. Current assignment: roads Asphalt (4 m),
sidewalks TilesFloorSquare (2 m), yards Grass (3 m), industrial walls
GrayBricks, market walls and brick RedBricks, residential walls WhiteBricks
(2 m), civic walls ConcreteWall, concrete Concrete, plaster ConcreteWhite
(3 m), interior floors TilesFloor (tops only: their undersides are ceilings),
modular roofs ConcreteGray (tops only), modular stairs TilesFloorGray. The
procedural brick, tile and wood patterns are skipped on textured surfaces;
grime and wet asphalt still apply.

Adding a material: put `Name/Name.png`, `Name_normal.png`,
`Name_ambient.png`, `Name_specular.png` in a folder, run
`cargo run --release -p dead-sector --example import_materials -- <folder> assets/materials`,
then add it to `library.json` and assign it. No code changes.

No draw calls are added: textured materials are the same materials. Seeded
cities add two batches (named stairs and roof materials). `--no-textures`
renders exactly as Stage 5.1 (pixel-identical to the baseline);
`--texture-size 512|1024` uploads smaller mips.

## Stylized material library (Stage 6B)

No image-generation model was available, so the new materials are
procedural: `examples/generate_materials.rs` builds each from a height
field and painted colour rules with `sector-assets::texgen`, then derives
the normal (DirectX), AO and roughness maps, and writes albedo, normal,
roughness, AO and height PNGs at 1024² plus 3 × 3 tiled previews (plain and
lit). Generation fails if any map has a seam: the step across the wrap must
be no stronger than the strongest ordinary line in the map, which passes
structural lines that fall on the wrap (mortar, board edges) and fails a real
seam; a test also checks the shipped, compressed data.

Art direction (Valorant-like, not photoreal, not PSX): readable shapes
(bricks, pavers, planks, boards, ribs, rolls, pebbles, grass clumps),
bevelled edges with a painted top light per element, value bands instead of
pixel noise, clean chip shapes with lighter rims, subtle baked occlusion
(AO never below 0.55, since the city is lit mostly by ambient light), and
no distinctive one-off marks on large surfaces (they make the tiling
readable; wear comes from the game's world-space grime).

16 materials: Asphalt, Sidewalk, Grass, Dirt, Concrete, BrickRed,
BrickPainted, Plaster, PaintedSiding, MetalPanel, PaintedMetal, Rust,
WoodPlanks, InteriorFloor, Roof, FloorTiles. Texel density: 1024 px per
3 m (341 px/m) for buildings and ground, per 2 m (512 px/m) for small
props. 3.5 MB each, 56 MB in all (the 12 photoreal packs: 117 MB).

`library.json` (the default set) assigns them: roads, sidewalks, lawns (tops
only), rubble, industrial walls (metal panels), market walls (red brick),
brick (painted brick), residential walls (siding), civic walls and concrete,
plaster and interior walls, ground floors (tiles, tops only), wood, rust and
steel props, and the generated roof (tops only), stairs and `upper_floor`
(wood, tops only) materials. The original 12 photoreal packs are untouched
and used by `library-photoreal.json` (`--materials photoreal`).

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

## Limitations (Stage 6B)

- Procedural, not hand-painted or AI-generated: shapes are clean but simpler
  than an artist's.
- One material per surface: no per-building variation or decals yet, so large
  areas of one material can still look uniform from the air.
- Height maps are generated and used for normals and AO, but not packed
  (no parallax).

## Limitations (Stage 6)

- Box projection picks one axis per pixel: rounded props (cylinders,
  spheres) show seams where the axis changes, and textures do not follow
  rotated surfaces' own orientation (fine for the city's axis-aligned boxes).
- Displacement maps are ignored (no parallax or tessellation).
- Without BC support the maps are decoded to at most 1024² (more memory).
- 113 MB of packs are committed to the repository.

## Limitations (Stage 5.1)

- Modular buildings exist only in seeded cities (`--seed`); the default city
  is the exported one, unchanged.
- No roof access; upper floors are single open rooms.
- Glass panes are not sorted against each other (one instanced draw); with
  identical tint the blend order barely matters, but looking through two
  panes at a grazing angle can differ slightly.
- Glass casts no shadow and does not refract; its own diffuse term lightens
  it near bright lights.
- Hidden interiors still cost shadow and depth work: there is no occlusion
  culling.

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
