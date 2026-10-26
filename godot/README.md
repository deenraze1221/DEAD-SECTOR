# DEAD SECTOR — native Godot version

Separate from the HTML game in the repository root. Requires Godot 4.6 Standard, Compatibility renderer. No .NET, Node.js or browser needed to run this project.

## Open

Download/extract the repository, import godot/project.godot, wait for asset import, press F5. Select PLAY. Escape pauses/releases the mouse. Settings open on the right. NEW MATCH resets the run.

WASD / mouse: movement/look; Shift: sprint; Space: jump; LMB: shoot; R: reload; 1: owned primary; 2: pistol; E: store terminal; F: owned flashlight; B: battery; H/J: stored small/large medkits; 3 then LMB: owned trap.

## Implemented

- Original 250m city/interiors/colliders and all ten original weapon models/materials.
- Native movement, stamina, camera, grip-positioned procedural hands, recoil/reload motion, 1.5s weapon switch and original per-weapon sounds.
- Normal/runner/eye/Vomiter/brute/Warden models; nine normal zombie skins and five boss skins. Native grid routing for ground, large and flying bodies, physical separation, flanking, hopping, hit reaction, melee/projectiles, boss phases/attack warning.
- Starter pistol,92 total rounds, no cash. First wave waits60s;15 enemies +5 per wave,120s spawning, all survivors must die,30s intermission.
- Eye200HP after wave5; Vomiter300HP after wave5 with hard15% cap,8m range,.25s windup,5m/s projectile,1–2s cooldown,10 direct+5×5 poison. Brute800HP after wave20. Warden every fifth wave1800HP +600 each next appearance.
- Death particles (harmless green cloud for Vomiter), automatic cash pickup $15–100, persistent red edge below25HP.
- Store at original central terminal: E within3m. Price-sorted weapon/supplies tabs, original silhouettes, stats, wave locks; purchased primary replaces previous primary. Universal ammo50/$50,500 total cap; medkits, repair, flashlight120s, batteries and traps. Weapons lose5 condition per100 actual shots, repair$500 restores50.
- Monospaced terminal UI, left menu, HUD, bearing, mandatory540-line pixelation, green grading/scanlines/vignette. Original calm/tense playlists crossfade with wave state.

## Scope / remaining work

This is the first complete single-player combat/economy pass, not a promise of pixel-perfect HTML parity. Native block generation, weather, day/night, original audio and settings are described below. Missions/airdrops, skeletal hand animations and special-wave variants remain separate work. Navigation and native physics require further playtesting across later waves and crowded interiors.

Multiplayer and Steam are intentionally not implemented yet. HTML and Godot cannot join each other's matches. HTML version remains separately playable.

## Developer checks

godot --headless --path godot --script tests/smoke.gd
godot --headless --path godot --script tests/gameplay.gd

With a display/OpenGL renderer: godot --path godot --script tests/gameplay_render.gd

Re-export source assets from repository root: node scripts/export-godot.mjs (npm dependencies and Chromium required only for asset generation). The icon SVGs are derived from weapon-icons.js. Font license is included in assets/FONT-LICENSE.txt.

Windows export preset is supplied; install matching Godot export templates first.

## World / presentation expansion (build 0.3)

Every NEW MATCH assembles the city natively from furnished HTML block templates, using a random seed or the seed saved in Settings → World. Roads, the central store and border/hive approach blocks stay fixed; district-compatible blocks are permuted with their geometry, furniture, ground finishes and collision volumes together. This is procedural prefab assembly, not a fresh native rewrite of the HTML room-layout algorithm. Tested seeds moved88 blocks with no newly intersecting solid collider pairs.

Weather uses the exact HTML seeded55/20/15/10 overcast/rain/storm/clear selection and an independent60/30/10 fog roll. Four waves form one day (two day/two night); the hour advances during spawning and freezes during preparation/clearance/intermission. Rain stays in world space, is hidden under building roofs, and leaves gradually drying roughness/color on asphalt. Storms flash before delayed thunder. Low/medium/high quality controls rain counts and shadows.

The local soldier waist, belt and legs are exported from the HTML body, with camouflage, straight-knee distance-based gait and gradual look-down fade. Full soldier geometry is included for future third-person/network work; there is still no new skeletal animation system.

57 WAV assets are rendered from the original HTML Web Audio definitions, including ten original music scores, ten distinct gunshots, all enemy voices/deaths, weather, UI and interaction effects. Calm/assault playlists advance and crossfade with match state. Aircraft/mission audio is included for future events, which are not implemented yet.

The UI is English-only. Courier-compatible Liberation Mono and Arial-compatible Liberation Sans Bold are bundled with their license. Audio, Controls, Graphics and World settings persist in user://settings.cfg. PSX processing stays mandatory.

Additional checks: tests/expansion.gd and tests/expansion_render.gd. Audio can be regenerated with node scripts/export-godot-audio.mjs. No browser or Node.js is required for players.

## UI reference pass

The supplied UI and Visuals screenshots now guide the native presentation: orbiting city panorama behind a transparent left menu, a two-color Arial logo, compact dark blue-gray HUD panels, right-hand settings and a centered three-column store. Locked cards have an actual lock over a dimmed silhouette; weapon statistics are available on hover/focus. Supplies reuse the actual HTML SVG paths, not substitute icons.

SystemFont requests Arial for branding and Courier New for terminal UI on Windows, matching the browser's font families; bundled Liberation fonts provide compatible fallbacks elsewhere. Rendering/hinting can still differ between browsers and Godot.

Weapon view FOV/zoom/position now use the HTML formula, and the Pistol's metal/green/gold material palette matches weapons.js. Fog/light colors and the subtle color grade/sharpen/grain were aligned with weather.js and psx.js. Pixelation remains mandatory. Pause retains the player view; only the initial main menu uses the cinematic camera.

Join by code is visibly disabled with an explanation: native multiplayer remains a later task. No fake lobby or invitation code is generated.

UI checks: godot --headless --path godot --script tests/ui_geometry.gd
Visual checks: godot --path godot --script tests/reference_render.gd
Supply icons can be regenerated with node scripts/export-godot-icons.mjs.

Pistol framing adjustment: its viewmodel and attached hands use65% model scale to keep more of the lower-right view clear. Other weapon framing remains unchanged.
