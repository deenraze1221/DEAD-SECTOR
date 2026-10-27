# DEAD SECTOR — native Godot version

Separate project alongside the HTML game. Requires **Godot 4.6 Standard**, Compatibility renderer. Import `godot/project.godot`, wait for assets, press F5. No .NET, browser or Node.js is required to play solo or host a native LAN match.

PLAY opens a four-slot lobby. Start solo, host LAN, or use an internet WebSocket relay. JOIN BY CODE opens server connection settings. The lobby host starts a five-second countdown. [LAN / Render instructions in Russian](NETWORK_RU.md).

## Controls

WASD / mouse: move/look; Shift: sprint; Space: jump; LMB: shoot; R: reload; 1: primary; 2: pistol; E: store, mission interaction or revive; F: owned flashlight; B: replace battery; H/J: stored small/large medkits; 3 then LMB: owned trap. Escape releases the mouse and opens the menu. Solo pauses; the cooperative world continues while a player opens a menu/store.

## World

- 250 × 250 m city assembled natively from the HTML architecture catalogue. District-compatible selection changes buildings per seed, rather than only permuting the same set. The central store, perimeter and hive approach blocks remain fixed.
- Fresh GDScript room plans reserve doors/corridors and place collision-checked themed furniture. Residential, office and service programmes use distinct layouts; tables/desks get chairs where space permits. Washrooms receive sanitary fixtures. This uses the original models, but is not a byte-identical port of JavaScript's generation/random stream.
- Weather uses HTML's seeded55/20/15/10 overcast/rain/storm/clear roll and independent60/30/10 light/medium/heavy fog. Four waves form one day: two day/two night. Rain stays in world space, hides under roofs and leaves gradually drying asphalt. Lightning precedes thunder.

## Combat / economy

- Original pistol and nine primary models/materials, two weapon slots, grip-based hands, recoil, weapon-specific reload actions, moving magazines/support hands, casings and1.5s weapon switching. Pistol viewmodel remains at65% scale.
- Sprint stamina lasts5s and restores in5s. Inertia, lowered jump,73° world FOV and60° downward look limit. Camouflaged local waist/legs fade smoothly into view with straight-knee distance-based gait.
- Normal/runner/eye/Vomiter/brute/Warden models; nine normal and five boss skins. Three navigation grids, physical crowd separation, flanks, hops, stagnation replans, eye firing-position search and boss phases with slam warnings. Ground enemy speed is5% above the HTML values.
- Eye200HP fromwave6; Vomiter300HP fromwave6, hard15% cap,8m range,.25s preparation,5m/s projectile,1–2s reload,10 direct + five poison ticks of5. Brute800HP fromwave21. Warden every fifth wave1800HP +600 per subsequent appearance.
- Starter pistol,92 total rounds,$0. Preparation60s; first assault15 enemies, +5 per wave, spawning over120s. All survivors must die before30s intermission.
- Actual-hit blood, directional impulse/stagger, hit marker/hurt edge. Death removes the model immediately, with fading red fragments or harmless green Vomiter spores lasting2.5s. Enemy voices/deaths, distinct shots, footsteps, interaction and weather audio use the original source sounds.
- Cash pickup$15–100 by walking over it. Sorted weapon/gear shop with original silhouettes, statistics, completed-wave locks, repairs, medkits,120s flashlight/battery and enemy-only traps. Ammo50/$50,500 total cap. Medkits heal immediately when injured, otherwise enter inventory. Weapons lose5 condition per100 shots; repair$500 restores50.

## Missions / supplies

No missions during waves1–2; thereafter a seeded15% roll per wave. Package collection,30s zone hold in6m radius, generator and destructible growth reward$300–600. Generator requires uninterrupted E15s within2.4m; release, leaving or changing the operator resets it. Growth starts at500HP and gains500 every10waves.

Every480s, the original aircraft flies for6s with propellers/audio, then drops a parachuted crate. Descent8s; uninterrupted E10s opens it for$100–500. Aircraft altitude clears the tallest generated building.

## Native cooperative play

- Up to four players through direct LAN ENet (UDP3001, no external service), or WebSocket/WSS via the existing Node deployment at `/godot`. Numeric8-digit invitation codes, slot list,5s launch and shared city seed.
- Host-authoritative enemies, shots, damage, poison, traps, waves, missions and supplies. Guests render interpolated snapshots instead of running duplicate enemy simulations. Remote soldiers include the neck, all weapon grips, aim/straight-legged gait/flashlight and squad markers on the compass.
- Cash is removed once and rewards everyone. Missions can be completed by any living squad member and reward everyone. Crate reward goes to its opener. Credits are cumulative/deduplicated so reconnect snapshots cannot pay twice. Personal purchases/ammo/condition stay with each player.
- E near a fallen squad member revives them with50HP. WebSocket connection interruptions automatically retry/resume a reserved seat for up to120s; this is not persistence after restarting the application. The host must remain in the match; host migration is not implemented.
- HTML and Godot can use the same server process/address, but their rooms and simulations are separate. No cross-play or Steam integration.

## Presentation

English-only interface. User-supplied **Turnpike** logo and **PixelSplitter** HUD fonts, with bundled/system fallbacks. Transparent continuous compass with5°ticks/15°labels, azimuth, store, nearby enemies (all survivors during clearance), squad and objective/supply markers. Upper-center banners and separate lower-center hold-E prompts. Compact blue-gray HUD/store panels, right settings, silhouettes/locks and responsive bounds.

Mandatory540-line pixelation, scanlines, subtle grain/sharpening/vignette, weather grading, wet surface materials, nearby street lighting, soft hive fog, world-space dust, rain impacts, tracers and projectile trails. Low/medium/high quality controls shadows, particle density and local lighting. Postprocessing preserves the retro look without vertex wobble.

Original calm/assault playlists crossfade with wave state. Audio/Controls/Graphics/World settings persist in `user://settings.cfg`; the last server address is stored separately. Fifty-seven source WAV assets include five calm/five tense tracks, ten gunshots and enemy/weather/UI effects.

## Limits / verification

Native generation/physics/rendering differ from HTML; visual parity is an ongoing port rather than a claim of pixel-perfect equality. Weapon motion is procedural, not a new skeletal animation rig. The HTML district-blackout/special-wave variants are not yet ported. Very late crowded waves still need broader playtesting. Internet transport was exercised against the local deployment server; no public Render instance was provisioned in this workspace.

From repository root:

```
godot --headless --path godot --script tests/smoke.gd
godot --headless --path godot --script tests/gameplay.gd
godot --headless --path godot --script tests/combat.gd
godot --headless --path godot --script tests/next_port.gd
godot --headless --path godot --script tests/expansion.gd
godot --headless --path godot --script tests/ui_geometry.gd
python3 scripts/test-godot-coop.py all
npm test
```

With an OpenGL display: `godot --path godot --script tests/coop_render.gd` or `tests/next_visual.gd`. These save screenshots under `user://`.

Re-export assets with `node scripts/export-godot.mjs`, audio with `node scripts/export-godot-audio.mjs`, icons with `node scripts/export-godot-icons.mjs` (npm dependencies/Chromium needed only for export). Bundled fallback font license: `assets/FONT-LICENSE.txt`. Windows export preset is supplied; install matching Godot export templates first.
