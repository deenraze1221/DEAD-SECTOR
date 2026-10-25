# DEAD SECTOR

Single-player browser FPS with randomly generated 200 × 200 m towns and two-player shared rooms. Four user-provided weapons are retained; all environment models are now original procedural geometry. English by default; Russian in Settings.

## Play offline

Download ZIP, extract and open `index.html`. No installation or Internet needed for single player. Click Play to capture the mouse. Restart / New run generates a new city and rolls one of four weapons. Resume preserves the current city and loadout.

WASD move; Shift sprint; Space jump; LMB fire; R reload; Esc menu. Survive endless waves from four corner hives. E interacts with supplies and revives a nearby partner; there is no exit victory. Player starts at (0, 85), with 100 HP. Buildings and props have solid colliders; doorways are accessible.

## Original city models

The city now uses semantic placement rather than random prop scattering. There are 16 building families and 24 placed outdoor prop/vegetation families, plus furniture and appliances. `asset-catalog.js` describes 1,200 procedural family/variant recipes; these are parameterized designs, not 1,200 separately hand-authored meshes. A connected street grid separates 64 lots (four times the former building count). Every category appears in each town; seeded variation selects buildings and prop arrangements. Main roads remain clear; tests verify routes to every building, enemy spawn and exit across several seeds.

`town.js` is the deterministic layout and collision source; `city.js` constructs meshes. No uploaded environment FBX/PNG/JPG is used or retained. `assets/imported/Guns/` and `assets/models/weapons.json` contain the four user-supplied weapons and matching magazines. Re-import with `node scripts/import-weapons.js`, then build.

Pixel rendering is always enabled at 540 pixels vertically; vertex snapping and graphic toggles are removed. Models use original low-poly geometry and flat materials. Mutants have articulated limbs, ribs, jaws and skin detail.

## Open to network

A running, reachable Node.js server is required for real rooms:

```sh
npm ci
npm run build
npm run server
```

The server serves the built game on port 3000 and accepts WebSocket rooms. On the host: Open to network → enter server URL (e.g. `ws://192.168.1.10:3000`) → Create room. Give your friend the same server address and the six-character code. The friend uses Join. Both then click Play. A code alone does not make a local server accessible over the Internet. For Internet play deploy the server on an accessible host; HTTPS pages require a `wss://` endpoint. No automatic hosting or port forwarding is performed.

Implemented: two-player limit, matching city seed, visible partner, movement replication, guest shots processed by the host, shared enemy health/deaths, damage to the nearest player and common completion. The host simulates the world; menu does not pause an active network game. At zero HP a network player is downed. A living partner within 2.5 m presses E to revive them with 50 HP. Both downed ends the run. Anti-cheat is not implemented. Starting a new run closes the current session. Unexpected disconnect reserves the seat and world snapshot for two minutes. The client retries automatically; after refreshing the page use Reconnect in the network panel. Explicit Disconnect by the host closes the room; explicit guest Disconnect frees the slot. These are private casual rooms, not a production multiplayer service.

## Development and validation

Node.js >=22.12:

```sh
npm ci
npm run dev -- --port 5173
# Open /app.html on the Vite server for source development.
npm test
npm run build
CHROMIUM_PATH=/usr/bin/chromium npm run test:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:combat:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:weapons:browser
CHROMIUM_PATH=/usr/bin/chromium npm run test:city:browser
```

Browser tests need a running dev server; use BASE_URL for a different address. Playwright Chromium can be installed with `npx playwright install chromium` on another machine. `npm run build` writes the self-contained root `index.html` and `dist/index.html`; generated files are committed for double-click play. Source changes require a rebuild.

With both the dev server and `npm run server` running: `CHROMIUM_PATH=/usr/bin/chromium npm run test:network:browser` verifies two browsers joining the same room, matching city seed, visible partners, dynamic enemy replication and matching wave clocks, disconnect handling, city reroll and absence of menu scrollbars. Override WS_URL for another WebSocket server. This has been validated locally; public Internet hosting is a separate deployment step.

## Districts, exploration and combat

Four district identities influence buildings and decoration: industrial, residential, medical and market. Courtyards have entrances in their fences, commercial/industrial lots have parked cars, and five elevated signs identify districts and the north exit. Building positions vary with the seed. All generated medkits/ammunition have tested navigation routes.

E picks up supplies within 1.5 m. Medkits restore 35 HP (maximum 100); ammo adds two magazines to the reserve (maximum twice the starting reserve). Full-health medkits remain available; taken supplies disappear for both players and remain taken after reconnecting. Rooms use host-authoritative pickup availability. The HUD shows interaction hints and nearest enemy distance/bearing.

Weapon tuning (hip fire only; viewmodel moved 20% closer to the camera):

| Weapon | Magazine | Damage | Fire interval | Sound |
| --- | --- | --- | --- | --- |
| Pistol | 8 | 10 | 0.28 s, single press | short sharp report |
| RifleBran | 24 | 25 | 0.24 s | deeper, longer report |
| RifleGod | 15 | 35 | 0.10 s | punchy medium report |
| SMG | 35 | 8 | 0.065 s | short high-frequency report |

Each sound is a distinct synthesized noise/filter/oscillator envelope, not an external audio file. Right click does not change FOV, camera or weapon pose.

| Enemy | HP | Speed | Damage | Spawn chance |
| --- | --- | --- | --- | --- |
| Zombie | 250 | 2.2 m/s | 25 | 55% |
| Runner | 125 | 3.6 m/s | 15 | 20% |
| Flying flesh eye | 300 | 2.3 m/s | 30 on projectile hit | 20% |
| Brute (3× size) | 600 | 1.1 m/s | 40 (default; unspecified in request) | 5% |

Waves last 120 seconds, followed by 10-second intermissions. Wave 1 spawns 30 enemies gradually (one every four seconds); wave 2 spawns 35; each later wave adds five. Each event rolls the confirmed type probabilities and a random corner hive. Existing enemies persist across waves. Enemies actively hunt players, with a shared navigation field cache; brutes use wider clearance. Flying eyes fire visible projectiles that collide with cover and must hit a player's body to deal damage. Corpses fade after eight seconds. Host snapshots synchronize the wave clock, dynamic enemy IDs and projectiles, including after reconnect.

The perimeter is a visible concrete wall 5.8 m tall. All environment meshes remain original procedural models. Static city geometry is batched by material for denser scenes. HUD panels show wave phase, countdown, spawned/total, alive enemies, health, weapon ammunition and context hints. Gloved hands and reload animation are retained.


Additional browser checks: `npm run test:supplies:browser`, `npm run test:revive:browser` (the latter requires the WebSocket server). The revival test may take several minutes while enemies travel from corner hives to down the host, revives them with the guest's E key, reloads the host's browser and reconnects to the saved room.

## Internet deployment

After `npm ci && npm run build`, the Dockerfile packages the verified static client and installed `ws` package and runs the server as an unprivileged user. Image build requires no package-registry access inside containers. Build: `docker build -t dead-sector .`; run: `docker run --rm -p 3000:3000 dead-sector`. Put a TLS reverse proxy in front of port 3000 for an HTTPS site and WSS connections; the proxy must support WebSocket upgrades. In the game enter the public `wss://` endpoint, create a room and give the friend the same endpoint and code. The server keeps snapshots and pickup availability in memory for two-minute reconnects; restarting it clears rooms.

Internet play cannot be validated without an actual public deployment/address. Two-client gameplay, revival, full-page reconnection and snapshot recovery have been validated locally; no public server is automatically published by this repository.

## Semantic placement and furnished interiors

- Fences are continuous 14.6 m boundary runs, joined at shared end posts. All align with the street/lot axes. No loose three-metre sections are scattered. Shared corner posts are drawn once.
- Benches face open forecourts, and bus shelters face the pedestrian strip beside the road; each bin refers to a specific bench/shelter. There are 68 uniformly spaced streetlights along pavement lines.
- Traffic cars occupy one marked travel lane and respect its direction. Parking cars are assigned to explicit bays, with their full footprints inside the markings. Crossings and the central survival route remain clear. Service crates may stand on pallets with exact support heights.
- Each of the 64 buildings has three accessible ground-floor zones: two main rooms plus a washroom. Residential rooms have beds, kitchen equipment and study furniture; clinics have consultation/treatment furniture; workshops, retail, schools, cafes and libraries use appropriate benches, counters, shelves or desks. Doors and the central passage remain clear. Upper stories are closed architectural volumes, not empty accessible floors; stairs and upper-floor access are not implemented.
- The floor/road/sidewalk mesh is a non-overlapping partition of the 200 × 200 m area. Interior floor finishes replace yard patches rather than stacking on top. Paint and facade lettering use explicit depth bias; trim pieces meet at boundaries. Furniture and benches have compound colliders at actual seat/table heights, avoiding standing above them on an oversized bounding box. Trees and shelters retain walking clearance beneath canopies.
- Instanced geometry, shared primitives and cached navigation/ray tests keep the increased detail practical. Normal, flying and giant enemies use separate height-clearance navigation layers.

`city-validation.js` checks semantic links, directions, parking containment, grounding/supports, facade and furniture overlap, room containment and connected fence junctions. Unit checks exercise 24 seeds and physically move the player through every room entrance on three cities. `npm run test:geometry:browser` compares rendered bounds against reserved footprints and renders street/interior views. These checks cover the tested geometry rules; they are not a guarantee that no visual defect can ever occur on every device and seed.
