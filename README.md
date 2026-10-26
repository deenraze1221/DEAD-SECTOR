# DEAD SECTOR

Browser wave-survival FPS with seeded 200 × 200 m towns, English/Russian UI and private four-player lobbies. All environment models are original procedural geometry. Weapon models come from the user's updated `gunsv2.fbx`, with the original pistol retained.

## Play

Download ZIP, extract it and open `index.html`. Play first opens a four-slot lobby (one occupied, three empty). The lobby Play button starts a five-second countdown. Offline solo play uses the same lobby and does not need a server. Single player needs no installation, server or Internet connection. A new run generates a town and starts beside SECTOR MART with 100 HP, $0, a pistol, 8 loaded rounds and 84 spare rounds. Starter ammunition makes the first cash earnings possible; there are no free ammunition or medkit boxes in the world.

WASD move; mouse look; Shift sprint; Space jump; LMB fire; R reload; E open the terminal or revive a nearby partner. **1 selects the primary weapon; 2 selects the pistol.** There are only these two weapon slots. Buying a different primary replaces the previous primary, returns its remaining magazine rounds to the universal reserve and equips the new empty gun. The pistol, its magazine and its condition remain. Reload the new weapon with R. H uses a small medkit; J uses a large medkit. Esc pauses single player. Resuming preserves the run; Restart starts over.

Sprinting lasts five seconds from full stamina. Stamina recovers in five seconds at walking speed or while stationary; exhaustion requires full recovery before sprinting resumes. The HUD shows health, stamina, cash, the two slots, weapon condition and ammunition. Static control instructions on the right have been removed. The terminal opens a scrollable shop with owned slot buttons, prices and purchase feedback. Close it using its button or Esc.

## Money, shop and condition

A dead enemy leaves a cash bundle on the ground. Walking within 0.85 m collects it automatically with a cash sound. Rewards derive from maximum HP, clamped to $5–100: runner 125 HP → $5; zombie 250 HP → $30; eye 300 HP → $40; brute 600 HP → $100. Cash remains until collected, and a bundle can be collected once. In co-op, the collector receives the money in their individual wallet.

SECTOR MART is a unique grocery building in the central quarter. Its large terminal opens within 2.1 m. The shop contains ten prop families: stocked shelves, produce stand, refrigerated case, freezer, checkout with cash register, shopping cart, basket stand, display rack, stock boxes and the terminal. Entrances and the route to the terminal have physical player-clearance checks.

| Weapon | Magazine | Damage | Fire interval | Reload | Mode | Price |
| --- | --- | --- | --- | --- | --- | --- |
| Pistol | 8 | 45 | 0.28 s | 1.15 s | single press | starter |
| Helsing | 40 | 30 | 0.075 s | 4 s | automatic | $6,000 |
| Tikhar | 1 | 55 | 0.45 s | 3 s | single press | $3,500 |
| Shambler | 5 | 50 | 0.50 s | 3 s | single press | $3,000 |
| Shotgun | 2 | 80 | 0.60 s | 2 s | single press | $2,000 |
| Kalash | 30 | 40 | 0.14 s | 3 s | automatic | $4,000 |
| VSV | 12 | 70 | 0.24 s | 6 s | automatic | $5,500 |
| Scorpion | 35 | 35 | 0.10 s | 4 s | automatic | $4,500 |
| Vector | 20 | 35 | 0.09 s | 3 s | automatic | $2,500 |
| G36 | 30 | 45 | 0.14 s | 4 s | automatic | $5,000 |

Damage includes the latest +15 to every weapon. Pistol also retains the preceding +20 increase (10 + 20 + 15 = 45). Fire intervals resolve qualitative rate requests; Shambler/Shotgun damage is total per shot. There is no aiming mode. Each weapon has a distinct synthesized report.

- Universal ammunition: **30 rounds for $50**. The **500-round limit includes reserve and both loaded magazines**. A full pack is refused if it would exceed the limit; no money is deducted.
- Small medkit: **$100**, immediately restores **25 HP** when purchased while injured; stored at full health for H.
- Large medkit: **$250**, immediately restores **75 HP** when purchased while injured; stored at full health for J. Health caps at 100; using a kit at full health consumes nothing.
- Every weapon has **100 condition**, losing **5 condition after each 100 shots actually fired**. Empty clicks do not cause wear. At zero condition it cannot shoot.
- Repair: **$500 for +50 condition** on the equipped weapon, capped at 100. Repairing an already intact weapon is refused without charging.

The updated FBX uses painted metal, brown stock/grip and blue pneumatic accents on appropriate parts, stored as vertex colors rather than stretched image textures. Authored normals are retained, and each model has individual trigger/support grip anchors and forearm endpoints. New guns are single-piece meshes; reload moves the gun and hand without detaching an invented magazine. Pistol retains its separate original magazine. `node scripts/import-weapons.js` rebuilds normalized geometry, colors and grip metadata.

## City generation

The town has 64 buildings and 20 repeatable architecture families, including the new cinema, fire station, motel and a tower with a 10 m structural height, plus the unique grocery. Ground floors use three floor plans: central spine, front lounge with rear rooms, and open floor with rear service band. Furniture, wall orientation and room use vary deterministically. Wide entrances have clear vestibules. Upper stories are closed architectural volumes; stairs and upper-floor access are not implemented.

Street population is increased by 15% after the baseline placement pass. Added families are traffic cones, street clocks, newspaper stands, public telephones and bus route boards. Reservations reject collisions with buildings, cars and existing props. Benches face pedestrian areas, bins pair with seating/shelters, lamps follow uniform pavement lines, cars fit lanes or marked bays, and fences form connected boundary runs of at least 14.6 m. The five extra prop families and all repeatable architectural families appear in each tested town.

`town.js`/`interiors.js` generate placement and colliders. `city.js` uses instanced primitive construction. Terrain is tiled once; room finishes replace yard patches. Markings/labels have depth bias, wall joints meet at boundaries, and shadow bias and ambient fill improve interior lighting. Solid instanced facades do not receive their own shadow map, eliminating the striped self-shadow acne; cast shadows remain on the ground. Furniture uses compound colliders at actual seat/table heights. Navigation accounts for the height/width of normal, flying and giant enemies. Tests validate overlap/support rules on 24 seeds and physically walk all room routes and the terminal approach on three cities; these checks do not guarantee every possible visual defect on every device.

World pixel rendering remains mandatory at 540 pixels vertically. There is no vertex snapping. Flat colors and original low-poly environment models are retained; no uploaded environment FBX/texture is used.

## Enemies and waves

| Enemy | HP | Speed | Damage | Chance |
| --- | --- | --- | --- | --- |
| Zombie | 250 | 2.2 m/s | 25 | 55% |
| Runner | 125 | 3.6 m/s | 15 | 20% |
| Flying flesh eye | 300 | 2.3 m/s | projectile 30 | 20% |
| Brute | 600 | 1.1 m/s | 40 | 5% |

Ordinary zombies have five procedural outfits. Jointed shoulders/elbows/hips/knees/ankles drive opposite strides, planted stance feet and attack motions. Brute arms stay outside the belly. Death immediately hides the model and emits red particles from one bounded pool. There are no falling corpses. Death/outfit/animation state replicates without replaying particles on duplicate packets.

The first wave spawns 15 enemies gradually over 120 seconds from four corner hives. Each later wave adds five. After spawning finishes, **every enemy must die before the 30-second break starts**. The HUD shows CLEAR REMAINING / ДОБЕЙТЕ ОСТАВШИХСЯ while waiting. Projectiles are blocked by cover and damage only on body contact. The perimeter is a concrete wall 5.8 m high.

## Network play

Node.js >=22.12 is needed to host rooms:

```sh
npm ci
npm run build
npm run server
```

The main menu has Play, Join by code and Settings. Play opens your local four-slot lobby. Open to network inside that lobby creates a real server room and displays an eight-digit numeric invitation code with a copy button. Friends use Join by code in their main menu; they must connect to the same server. The host presses Play to start a synchronized five-second countdown. Guests can press Ready in the lobby to prepare mouse capture; a browser that has released capture may require Resume when the match begins. Disconnecting during the countdown cancels it and returns the remaining players to the lobby.

The default endpoint is the current site's WSS/WS service (the address is blank for an offline file). A different shared endpoint can be set in Settings → Server. Reconnect is shown only when a previous room is saved. Codes identify lobbies on that server; they do not publish a machine to the Internet. No public server is automatically deployed. The existing 60-second in-world preparation starts after the five-second lobby countdown.

The host simulates enemies, shooting and waves for up to four people. Each client sees up to three peers, enemies target any living player, damage is addressed to its recipient, and E revives a nearby downed teammate. The room server tracks cash availability, individual wallets, owned slots, medkits and market transactions. It checks pickup/terminal range and rejects duplicate cash collection or invalid purchases. Transaction revisions prevent delayed combat snapshots from undoing purchases/repairs. As in the existing private-room design, clients are trusted for combat movement/ammunition; this is not an anti-cheat service.

At zero HP, a partner within 2.5 m presses E to restore 50 HP. The run ends when every connected player is downed. Menu/shop use does not stop an active network world. Unexpected disconnect reserves the seat, account and world snapshot for 120 seconds; Reconnect can restore the same room after reload. Explicit host Disconnect closes it. Rooms are held in memory and disappear on server restart. Public Internet gameplay requires separate deployment; local four-client and real-socket checks cover the provided implementation.

For Docker, first run `npm ci && npm run build`, then `docker build -t dead-sector .` and `docker run --rm -p 3000:3000 dead-sector`. The image runs as an unprivileged user; a public HTTPS site needs a TLS proxy with WebSocket upgrades.

## Development and checks

```sh
npm ci
npm run dev -- --port 5173
# Open /app.html for source development; root index.html is the generated standalone build.
npm test
npm run build
CHROMIUM_PATH=/usr/bin/chromium npm run test:geometry:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:mutants:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:economy:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:combat:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:city:browser
```

Browser checks need a running dev server. Override BASE_URL, CHROMIUM_PATH, ARTIFACT_DIR (screenshots), and WS_URL (external server for network/revive scenarios) as needed. `test:economy:browser` starts an isolated local room server and funds its test account server-side to verify purchases; the playable client has no money-grant debug API. `test:weapons:browser` aliases that market/equipment scenario. `test:network:browser` creates its own isolated server and verifies four browser clients; `test:revive:browser` uses a running room server. Tests cover exact stats, slots, wear/breakage, cash, caps, medkits, repairs, stamina, transactions/reconnect, wave completion, collision routes and model bounds. Software headless rendering is not a measurement of desktop GPU FPS.

`npm run build` writes self-contained `index.html` and `dist/index.html`, including models and styles. Source edits require rebuilding. The committed root file supports double-click offline play.

## Atmosphere and encounter update

The town now uses a cool overcast evening palette and lighter-distance fog, warm grocery lighting, cold street lights and an occasional flickering circuit. Activating the generator stabilizes and strengthens the street circuit for the rest of that run. Instanced boarded windows, facade cracks, litter beside dumpsters and weathered vehicle panels mark the abandoned environment without obstructing tested entrances. This is stylized procedural scenery, not a destructible-building simulation.

Each gun has its own recoil parameters and reload motion profile, with animated action/bolt, spent cases and support-hand operation. The original FBX guns remain single meshes: these are procedural first-person mechanisms rather than imported skeletal animations. Hits emit small red particles and cause a body recoil. Wounded enemies limp and slow to 80% speed below 35% health. Normal/runner melee has a short preparation; brutes wind up for 0.6 s and can miss if the player moves away or takes cover. Flying eyes swell before firing. Brute steps have a separate low-frequency sound.

Wave 1 remains standard. Waves congruent to 2 modulo 3 concentrate on one hive; waves congruent to 0 modulo 3 favor runners (at least70%, depending on unlocked types); every fifth wave overrides these modes and finishes with one mini-boss. It has 1,800 HP, 55 melee damage, speed 1.3 m/s and a separate armored Warden body, 2.2 m height and 0.7 m navigation radius, plus visible red chest marks and a HUD health counter. It must die before the normal clear-and-break transition.

One optional event activates 20 seconds into each assault, rotating through:

- Start the roadside generator near (-3.65, 12), using E within 2.4 m: $150 and stable street lighting.
- Hold the crossroads at (0, 24) for 25 seconds: $250. A living player must stay within 5 m, with no living enemy inside 10 m; progress decays outside those conditions.
- Destroy the fleshy growth at (24, 0), 180 HP: $350. Gunfire obeys cover and nearer enemies block shots.

Unfinished events expire at intermission. The HUD shows task, distance, progress/HP and payout; a world marker identifies the location. Co-op uses the host's event state and the room server grants the reward once per event to each connected account. Clearing a wave pays $75 + $25 × wave number; fifth waves pay $500 instead. Single player uses the same payouts. Cash drops remain capped at $100 per enemy; these bonuses are separate.

See [the reproducible resource model](docs/ECONOMY_BALANCE.md), generated by `node scripts/economy-report.js`. It includes ammunition and amortized repair costs at 65% accuracy, but excludes travel and damage taken. Prices and requested weapon damage are unchanged; the report is not a substitute for playtesting.


Shop update: two tabs separate weapons from gear (ammo, medkits and repair). Weapons are ordered by price. Each card shows a side silhouette projected from its actual model; hovering or keyboard focus reveals damage, magazine capacity, cyclic rounds/minute and reload time. Touch screens show those details directly. The starter reserve is now84, for92 total rounds. Cash auto-picks up underfoot with a separate sound. Flying-eye projectiles now travel at20 m/s, twice their previous speed; enemy types have separate growl/snarl/chirp sounds.


A fresh match has a60-second preparation countdown with no enemy spawns or events. The first assault begins after that countdown; enemies then emerge gradually on its normal120-second schedule. Subsequent intermissions last30 seconds after every enemy is dead. Each assault starts with a five-second on-screen notification and an original synthesized horn. Original procedural ambient music has calm and tense stems: preparation/intermission uses calm chords, active assault/clearing uses the tense stem. Changes crossfade over2.5 seconds; paused single player fades music out. Browser audio starts after the player's Play gesture. No external songs or audio downloads are used.


Enemy progression: waves1–5 contain only zombies/runners (weights55:20 normalized over available types). Waves6–10 add eyes (55:20:20 normalized). Wave11 onward enables all four types at55/20/20/5. The separate Warden mini-boss appears every fifth wave starting at wave5; it is not a brute and does not unlock the regular brute early. Fifth-wave completion rewards remain $500 even before bosses unlock. Purchased medkits immediately heal an injured player; full-health purchases go into reserve for H/J.

The fifth enemy family is the armored Warden: distinct mask, shoulder armor and horns, slimmer body and separate navigation clearance. Regular brute unlock remains wave11.


Latest visual/control changes: the first assault contains15 enemies, then20,25 and +5 thereafter. A10m lattice landmark rises above the grocery roof with four SECTOR MART signs and a beacon. The terminal has a proper display, keypad, payment slot, trim and stereo beeps attenuated to silence at8m. The soundtrack now has four original calm and four original tense procedural compositions with different tempo, harmony, melody and timbre, rotating by wave while preserving the2.5s mood crossfade. Pointer-lock re-entry resets buffered input; non-finite/warp deltas are discarded and bounded direct input prevents sudden180° spikes. A top compass shows heading, azimuth ticks, localized N/E/S/W, and distance markers for the shop and up to three teammates. Targets behind the field of view are pinned to an edge.


Per-wave weather uses independent seeded rolls: overcast55%, rain20%, storm15%, clear10%. Fog is rolled separately: light60%, medium30%, heavy10%, and can overlay any weather. Rain is masked under building roofs; storms add brief lightning and delayed thunder. The four-wave day/night cycle is day for waves1–2 and night for3–4, repeating; each wave advances six in-game hours. Clock starts at06:00, freezes during preparation/clearance waits/intermissions and does not rewind during breaks. Weather, fog and day phase derive from the shared seed/wave and match across a lobby.


Settings now use Audio/Controls/Graphics tabs. Audio sliders persist master, music and weather volume independently, including mute at0. Weather has looping rain/wind beds, delayed storm thunder and fades between conditions; weapon/terminal sounds remain under master. Controls contains sensitivity and the key map. Low/Medium/High graphics adjust shadows, lamp lighting and rain density; required540p pixel rendering is unchanged. Graphics settings are reapplied when a new city or network room rebuilds the scene.

Menu settings open in a separate right-hand drawer with a close button. The menu plays the calm soundtrack after the first user gesture; buttons have hover/click tones routed through master volume. Gear cards include ammunition, medkit and repair silhouettes. Mouse input is consumed once per frame without a smoothing tail. All ten guns now use individually reviewed grip anchors aligned to the pistol grip and fore-end. A dedicated 45-degree weapon camera makes guns approximately54% larger without changing world FOV. HUD elements are scaled to125%; the compass has a transparent background.

Soldiers replace the old network capsules: helmet, goggles, face covering, plate carrier, pouches, backpack, shoulder protection, articulated legs and boots. Peers display the actual equipped model using the same grip anchors; shoulder/elbow segments connect to glove wrists. Aim pitch is replicated, poses interpolate, and hips/knees/feet animate each frame between pose packets. Looking down in first person reveals the local torso and legs without rendering a duplicate head or weapon. These are original procedural stylized models and animations.
