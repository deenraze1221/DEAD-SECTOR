# DEAD SECTOR

Single-player browser FPS with randomly generated 200 × 200 m towns and two-player shared rooms. Four user-provided weapons are retained; all environment models are now original procedural geometry. English by default; Russian in Settings.

## Play offline

Download ZIP, extract and open `index.html`. No installation or Internet needed for single player. Click Play to capture the mouse. Restart / New run generates a new city and rolls one of four weapons. Resume preserves the current city and loadout.

WASD move; Shift sprint; Space jump; LMB fire; RMB aim; R reload; Esc menu. Clear five mutants, reach the north exit at (0, −95), press E. Player starts at (0, 85), with 100 HP. Buildings and props have solid colliders; doorways are accessible.

## Original city models

15 prop categories: bench, bin, dumpster, lamp, car, concrete barrier, fence, barrel, crate, hydrant, electrical cabinet, bus shelter, pallet, street sign and bollard. Five building archetypes: apartment, office, shop, garage and clinic. Six connected avenues separate 16 lots. Every category appears in each town; seeded variation selects buildings and prop arrangements. Main roads remain clear; tests verify routes to every building, enemy spawn and exit across several seeds.

`town.js` is the deterministic layout and collision source; `city.js` constructs meshes. No uploaded environment FBX/PNG/JPG is used or retained. `assets/imported/Guns/` and `assets/models/weapons.json` contain the four user-supplied weapons and matching magazines. Re-import with `node scripts/import-weapons.js`, then build.

Pixel rendering is always enabled at 270 pixels vertically; vertex snapping and graphic toggles are removed. Models use original low-poly geometry and flat materials. Mutants have articulated limbs, ribs, jaws and skin detail.

## Open to network

A running, reachable Node.js server is required for real rooms:

```sh
npm ci
npm run build
npm run server
```

The server serves the built game on port 3000 and accepts WebSocket rooms. On the host: Open to network → enter server URL (e.g. `ws://192.168.1.10:3000`) → Create room. Give your friend the same server address and the six-character code. The friend uses Join. Both then click Play. A code alone does not make a local server accessible over the Internet. For Internet play deploy the server on an accessible host; HTTPS pages require a `wss://` endpoint. No automatic hosting or port forwarding is performed.

Implemented: two-player limit, matching city seed, visible partner, movement replication, guest shots processed by the host, shared enemy health/deaths, damage to the nearest player and common completion. The host simulates the world; menu does not pause an active network game. Host death ends the shared run. Guest death requires restarting/rejoining; revival/reconnection and anti-cheat are not implemented. Starting a new run closes the current session. Host disconnect closes the room; guest disconnect frees its slot. These are private casual rooms, not a production multiplayer service.

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

With both the dev server and `npm run server` running: `CHROMIUM_PATH=/usr/bin/chromium npm run test:network:browser` verifies two browsers joining the same room, matching city seed, visible partners, a guest killing an enemy on the host, disconnect handling, city reroll and absence of menu scrollbars. Override WS_URL for another WebSocket server. This has been validated locally; public Internet hosting is a separate deployment step.
