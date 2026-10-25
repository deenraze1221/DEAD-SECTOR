import * as THREE from 'three';

// Reusable low-poly prop models. Coordinates describe the complete footprint,
// including rims, so the same placements can drive rendering and collisions.
export const PROPS = [
  { type: 'crate', x: -1, z: -1, w: 1.6, h: .65, d: 1.6 },
  { type: 'crate', x: -3.2, z: 10, w: 1.25, h: 1.1, d: 1.25 },
  { type: 'crate', x: -4.7, z: 10.3, w: 1.1, h: .85, d: 1.1 },
  { type: 'crate', x: -16, z: -7, w: 1.4, h: 1.2, d: 1.25 },
  { type: 'barrel', x: 3.5, z: 9, w: .76, h: 1.1, d: .76 },
  { type: 'barrel', x: 4.4, z: 8.7, w: .76, h: 1.1, d: .76, green: true },
  { type: 'barrel', x: -21, z: 4, w: .76, h: 1.1, d: .76 },
  { type: 'barrel', x: -22, z: 4.2, w: .76, h: 1.1, d: .76, green: true },
];

export function propColliders() {
  return PROPS.map(p => ({ minX: p.x - p.w / 2, maxX: p.x + p.w / 2,
    minY: 0, maxY: p.h, minZ: p.z - p.d / 2, maxZ: p.z + p.d / 2 }));
}

function surface(kind) {
  const c = document.createElement('canvas'); c.width = c.height = 128;
  const ctx = c.getContext('2d');
  ctx.fillStyle = kind === 'wood' ? '#65513a' : kind === 'green' ? '#495647' : '#815037';
  ctx.fillRect(0, 0, 128, 128);
  let seed = 1741;
  const random = () => { seed = (1664525 * seed + 1013904223) >>> 0; return seed / 4294967296; };
  for (let i = 0; i < 1400; i++) {
    const x = random() * 128, y = random() * 128;
    ctx.fillStyle = random() > .5 ? '#1d221b33' : '#bb9b6544';
    ctx.fillRect(x, y, kind === 'wood' ? 1 : 1 + random() * 5, kind === 'wood' ? 4 + random() * 28 : 1 + random() * 4);
  }
  const map = new THREE.CanvasTexture(c); map.colorSpace = THREE.SRGBColorSpace;
  map.magFilter = THREE.NearestFilter;
  return new THREE.MeshStandardMaterial({ map, roughness: .95, metalness: kind === 'wood' ? 0 : .25 });
}

export function createPropModels(scene) {
  const metal = new THREE.MeshStandardMaterial({ color: '#333b36', roughness: .78, metalness: .55 });
  const edge = new THREE.MeshStandardMaterial({ color: '#806646', roughness: 1 });
  const wood = surface('wood'), rust = surface('rust'), green = surface('green');
  function part(group, geometry, material, x, y, z) {
    const mesh = new THREE.Mesh(geometry, material); mesh.position.set(x, y, z);
    mesh.castShadow = mesh.receiveShadow = true; group.add(mesh); return mesh;
  }
  function box(group, w, h, d, mat, x, y, z) { return part(group, new THREE.BoxGeometry(w, h, d), mat, x, y, z); }
  for (const p of PROPS) {
    const group = new THREE.Group(); group.name = p.type; group.position.set(p.x, 0, p.z);
    if (p.type === 'barrel') {
      const radius = p.w / 2, h = p.h;
      // Profile supplies the rolled ends and bulging steel body, rather than a plain cylinder.
      const profile = [[.90, .02], [.94, .05], [.92, .12], [.96, .25], [.96, .75], [.92, .88], [.94, .95], [.90, .98]];
      const body = new THREE.LatheGeometry(profile.map(([r, y]) => new THREE.Vector2(radius * r, y * h)), 16);
      part(group, body, p.green ? green : rust, 0, 0, 0);
      for (const y of [.04, .28, .72, .96]) {
        const ring = part(group, new THREE.TorusGeometry(radius * .94, .018, 4, 16), metal, 0, y * h, 0);
        ring.rotation.x = Math.PI / 2;
      }
      for (const y of [.025, .98]) part(group, new THREE.CylinderGeometry(radius * .89, radius * .89, .018, 16), metal, 0, y * h, 0);
      const lid = part(group, new THREE.CylinderGeometry(radius * .83, radius * .83, .01, 16), p.green ? green : rust, 0, .992 * h, 0);
      lid.name = 'recessed-lid';
      part(group, new THREE.CylinderGeometry(.045, .045, .018, 8), metal, .14, h - .009, .07);
      box(group, .16, .12, .012, edge, 0, h * .51, radius * .96);
    } else {
      const { w, h, d } = p;
      box(group, w - .08, h - .04, d - .08, metal, 0, h / 2, 0);
      // Individual slats leave dark seams on all four sides and the lid.
      for (let i = 0; i < 5; i++) {
        const x = -w / 2 + (i + .5) * w / 5;
        for (const z of [-d / 2 + .026, d / 2 - .026]) box(group, w / 5 - .012, h - .04, .045, wood, x, h / 2, z);
        box(group, w / 5 - .012, .04, d - .04, wood, x, h - .02, 0);
        const z = -d / 2 + (i + .5) * d / 5;
        for (const sx of [-w / 2 + .026, w / 2 - .026]) box(group, .045, h - .04, d / 5 - .012, wood, sx, h / 2, z);
      }
      for (const x of [-w / 2 + .06, w / 2 - .06]) for (const z of [-d / 2 + .06, d / 2 - .06]) box(group, .12, h, .12, edge, x, h / 2, z);
      for (const y of [.08, h - .08]) {
        for (const z of [-d / 2 + .018, d / 2 - .018]) box(group, w, .11, .035, edge, 0, y, z);
        for (const x of [-w / 2 + .018, w / 2 - .018]) box(group, .035, .11, d, edge, x, y, 0);
      }
      // Diagonal face brace and steel straps with visible rivets.
      const brace = box(group, .095, Math.hypot(w - .26, h - .3), .032, edge, 0, h / 2, d / 2 - .016);
      brace.rotation.z = -Math.atan2(w - .26, h - .3);
      for (const x of [-w * .28, w * .28]) {
        for (const z of [-d / 2 + .008, d / 2 - .008]) {
          box(group, .045, h, .014, metal, x, h / 2, z);
          for (const y of [.08, h - .08]) {
            const rivet = part(group, new THREE.SphereGeometry(.019, 6, 4), metal, x, y, z);
            rivet.scale.z = .4;
          }
        }
        box(group, .045, .012, d, metal, x, h - .006, 0);
      }
    }
    scene.add(group);
  }
}
