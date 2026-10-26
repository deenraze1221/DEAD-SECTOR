# DEAD SECTOR — Godot version (stage 1)

A separate native port, alongside the unchanged HTML game. Requires Godot 4.6 Standard (Compatibility renderer; no .NET, Node.js or plugins required to play).

1. Download and unpack the repository ZIP.
2. In Godot choose Import and select godot/project.godot.
3. Allow the initial asset import to finish, then press F5 / Run Project.
4. Click PLAY. Escape releases the mouse and opens the pause/test menu.

This is an actual runnable Godot project, not a Windows executable. For an executable, install matching export templates from Editor → Manage Export Templates and create a Windows Desktop export preset.

## Included

- Original 250 × 250 m city, furnished interiors, store, perimeter and skyline from browser seed 42.
- Original city geometry transferred into MultiMeshes; collision volumes preserved.
- All ten original weapon models, materials and configurations: Pistol, Helsing, Tikhar, Shambler, Shotgun, Kalash, VSV, Scorpion, Vector, G36.
- Test armory in pause menu (free selection for validation; not the final shop).
- CharacterBody movement at 4.5 / 7.5 m/s, inertia, five-second sprint/recovery, jumping.
- 73° world FOV; maximum 60° downward view; captured relative mouse input.
- Shooting / automatic modes, exact magazines, reserve, reload durations and damage; three destructible 250HP training targets near initial spawn.
- Synthesized gunshot placeholder, recoil, muzzle light, test flashlight, dark sky/fog, world-space material detail and mandatory pixel effect.
- Pause/focus-loss handling, fullscreen toggle and reset to store spawn.

## Next stages (not implemented yet)

Zombie AI, waves, economy / actual purchases, player body and full skeletal hand animations, quests, weather transitions, full sound/music library, multiplayer/Steam and full UI/localization parity. Current gun sound is a shared placeholder. City is an exported seed-42 snapshot, not a native procedural generator yet. Browser shaders are adapted; graphics and physics are not guaranteed 1:1. This build does not connect to Render or the HTML lobby.

## Asset bridge (developer only)

From the repository root with npm dependencies and Chromium installed run: node scripts/export-godot.mjs

This regenerates city.json and label textures using current browser models. Coordinates/materials are converted by scripts/bridge.gd; triangle winding is reversed for Godot. The bridge does not modify the HTML build.

## Visual/handling revision

Transparent browser lamp decals are excluded from the static mesh bridge, preventing opaque white pavement patches. Per-instance colors are preserved. Materials use filtered world-space brick/sidewalk detail, softer overcast lighting, denser distance fog and restrained color grading. Weapons render in a separate transparent viewport, with grip-aligned glove/sleeve meshes for each gun and their own fill light; walls no longer cut through the viewmodel. HUD has bottom-corner vitals/ammo, a top-center bearing and FPS at the top right. Resuming capture has a short mouse guard. Hands remain procedural, not a finished animated soldier rig.
