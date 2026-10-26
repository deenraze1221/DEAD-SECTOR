# DEAD SECTOR — native Godot version

Separate from the HTML game in the repository root. Requires Godot 4.6 Standard, Compatibility renderer. No .NET, Node.js or browser needed to run this project.

## Open

Download/extract the repository, import godot/project.godot, wait for asset import, press F5. Select PLAY. Escape pauses/releases the mouse. Settings open on the right. NEW MATCH resets the run.

WASD / mouse: movement/look; Shift: sprint; Space: jump; LMB: shoot; R: reload; 1: owned primary; 2: pistol; E: store terminal; F: owned flashlight; B: battery; H/J: stored small/large medkits; 3 then LMB: owned trap.

## Implemented

- Original 250m city/interiors/colliders and all ten original weapon models/materials.
- Native movement, stamina, camera, grip-positioned procedural hands, recoil/reload motion, 1.5s weapon switch and per-weapon sound pitch.
- Normal/runner/eye/Vomiter/brute/Warden models; nine normal zombie skins and five boss skins. Native grid routing for ground, large and flying bodies, physical separation, flanking, hopping, hit reaction, melee/projectiles, boss phases/attack warning.
- Starter pistol,92 total rounds, no cash. First wave waits60s;15 enemies +5 per wave,120s spawning, all survivors must die,30s intermission.
- Eye200HP after wave5; Vomiter300HP after wave5 with hard15% cap,8m range,.25s windup,5m/s projectile,1–2s cooldown,10 direct+5×5 poison. Brute800HP after wave20. Warden every fifth wave1800HP +600 each next appearance.
- Death particles (harmless green cloud for Vomiter), automatic cash pickup $15–100, persistent red edge below25HP.
- Store at original central terminal: E within3m. Price-sorted weapon/supplies tabs, original silhouettes, stats, wave locks; purchased primary replaces previous primary. Universal ammo50/$50,500 total cap; medkits, repair, flashlight120s, batteries and traps. Weapons lose5 condition per100 actual shots, repair$500 restores50.
- Monospaced terminal UI, left menu, HUD, bearing, mandatory540-line pixelation, green grading/scanlines/vignette. Generated calm/tense musical beds crossfade with wave state.

## Scope / remaining work

This is the first complete single-player combat/economy pass, not a promise of pixel-perfect HTML parity. City remains an exported seed42 snapshot. Day/night, weather transitions, missions/airdrops, full HTML audio library, skeletal soldier body/hand animations, graphical settings/localization parity and special-wave variants remain separate work. Navigation and native physics require further playtesting across later waves and crowded interiors.

Multiplayer and Steam are intentionally not implemented yet. HTML and Godot cannot join each other's matches. HTML version remains separately playable.

## Developer checks

godot --headless --path godot --script tests/smoke.gd
godot --headless --path godot --script tests/gameplay.gd

With a display/OpenGL renderer: godot --path godot --script tests/gameplay_render.gd

Re-export source assets from repository root: node scripts/export-godot.mjs (npm dependencies and Chromium required only for asset generation). The icon SVGs are derived from weapon-icons.js. Font license is included in assets/FONT-LICENSE.txt.

Windows export preset is supplied; install matching Godot export templates first.
