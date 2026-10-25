# DEAD SECTOR

Browser wave-survival FPS with seeded 200 × 200 m towns, English/Russian UI and private two-player rooms. All environment models are original procedural geometry. Weapon models come from the user's updated `gunsv2.fbx`, with the original pistol retained.

## Play

Download ZIP, extract it and open `index.html`. Single player needs no installation, server or Internet connection. A new run generates a town and starts beside SECTOR MART with 100 HP, $0, a pistol, 8 loaded rounds and 64 spare rounds. Starter ammunition makes the first cash earnings possible; there are no free ammunition or medkit boxes in the world.

WASD move; mouse look; Shift sprint; Space jump; LMB fire; R reload; E collect cash, open the terminal or revive a nearby partner. **1 selects the primary weapon; 2 selects the pistol.** There are only these two weapon slots. Buying a different primary replaces the previous primary, returns its remaining magazine rounds to the universal reserve and equips the new empty gun. The pistol, its magazine and its condition remain. Reload the new weapon with R. H uses a small medkit; J uses a large medkit. Esc pauses single player. Resuming preserves the run; Restart starts over.

Sprinting lasts five seconds from full stamina. Stamina recovers in five seconds at walking speed or while stationary; exhaustion requires full recovery before sprinting resumes. The HUD shows health, stamina, cash, the two slots, weapon condition and ammunition. Static control instructions on the right have been removed. The terminal opens a scrollable shop with owned slot buttons, prices and purchase feedback. Close it using its button or Esc.

## Money, shop and condition

A dead enemy leaves a cash bundle on the ground. E collects it within 1.6 m. Rewards derive from maximum HP, clamped to $5–100: runner 125 HP → $5; zombie 250 HP → $30; eye 300 HP → $40; brute 600 HP → $100. Cash remains until collected, and a bundle can be collected once. In co-op, the collector receives the money in their individual wallet.

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
- Small medkit: **$100**, restores **25 HP** when used with H.
- Large medkit: **$250**, restores **75 HP** when used with J. Health caps at 100; using a kit at full health consumes nothing.
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

The first wave spawns 30 enemies gradually over 120 seconds from four corner hives. Each later wave adds five. After spawning finishes, **every enemy must die before the 10-second break starts**. The HUD shows CLEAR REMAINING / ДОБЕЙТЕ ОСТАВШИХСЯ while waiting. Projectiles are blocked by cover and damage only on body contact. The perimeter is a concrete wall 5.8 m high.

## Network play

Node.js >=22.12 is needed to host rooms:

```sh
npm ci
npm run build
npm run server
```

The server serves the built client and WebSocket rooms on port 3000. Host: Open to network → reachable server URL → Create room. Friend: same URL and six-character code → Join. Both press Play. A room code does not expose a local server to the Internet; public play requires a deployed server and HTTPS/WSS where applicable. No public server is automatically published.

The host simulates enemies, shooting and waves. The room server tracks cash availability, individual wallets, owned slots, medkits and market transactions. It checks pickup/terminal range and rejects duplicate cash collection or invalid purchases. Transaction revisions prevent delayed combat snapshots from undoing purchases/repairs. As in the existing private-room design, clients are trusted for combat movement/ammunition; this is not an anti-cheat service.

At zero HP, a partner within 2.5 m presses E to restore 50 HP. Both downed ends the run. Menu/shop use does not stop an active network world. Unexpected disconnect reserves the seat, account and world snapshot for 120 seconds; Reconnect can restore the same room after reload. Explicit host Disconnect closes it. Rooms are held in memory and disappear on server restart. Public Internet gameplay requires separate deployment; local two-client and real-socket checks cover the provided implementation.

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

Browser checks need a running dev server. Override BASE_URL, CHROMIUM_PATH, ARTIFACT_DIR (screenshots), and WS_URL (external server for network/revive scenarios) as needed. `test:economy:browser` starts an isolated local room server and funds its test account server-side to verify purchases; the playable client has no money-grant debug API. `test:weapons:browser` aliases that market/equipment scenario. `test:network:browser` and `test:revive:browser` use a running room server. Tests cover exact stats, slots, wear/breakage, cash, caps, medkits, repairs, stamina, transactions/reconnect, wave completion, collision routes and model bounds. Software headless rendering is not a measurement of desktop GPU FPS.

`npm run build` writes self-contained `index.html` and `dist/index.html`, including models and styles. Source edits require rebuilding. The committed root file supports double-click offline play.
