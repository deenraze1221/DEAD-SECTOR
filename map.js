import * as THREE from 'three';

export const SPAWN = Object.freeze({ x: 0, y: 0, z: 16 });
// Static training layout. Dimensions are metres, y is the bottom of an object.
const blocks = [];
const block = (x, y, z, w, h, d, material = 'concrete', solid = true) => blocks.push({ x, y, z, w, h, d, material, solid });
// Perimeter, fully closed even where the fog hides it.
block(0, 0, -35, 70, 5, 1); block(0, 0, 35, 70, 5, 1);
block(-35, 0, 0, 1, 5, 70); block(35, 0, 0, 1, 5, 70);
// Walk-in building. Front doorway is 3 m wide; side walls and roof are solid.
block(-13, 0, -12, 12, 5, .5); block(-19, 0, -7, .5, 5, 10); block(-7, 0, -7, .5, 5, 10);
block(-17, 0, -2, 4, 5, .5); block(-9.5, 0, -2, 5, 5, .5);
block(-13.5, 3, -2, 3, 2, .5); block(-13, 5, -7, 12.5, .4, 10.5);
// Background massing and crates.
block(18, 0, -21, 14, 9, 11); block(-26, 0, -24, 10, 12, 12);
block(25, 0, 13, 10, 7, 13); block(-26, 0, 17, 9, 8, 13);
block(7, 0, -5, 6, 2.5, 2.5, 'rust'); block(12, 0, -5, 2, 3, 2, 'rust');
block(-4, 0, 6, 4, 1.3, .8); block(6, 0, 10, 4, 1.3, .8);
block(0, 0, -12, 4, 1.3, .8); block(-1, 0, -1, 1.6, .65, 1.6, 'rust');
block(17, 0, 3, 2.5, 1.2, 5, 'rust'); block(17, 1.2, 3, 2.2, .8, 2.5, 'dark');
block(-21, 0, 4, 1.3, 1.2, 1.3, 'rust'); block(-22.5, 0, 4, 1.3, 1.2, 1.3, 'rust');

export function getColliders() {
  return blocks.filter(b => b.solid).map(b => ({
    minX: b.x - b.w / 2, maxX: b.x + b.w / 2,
    minY: b.y, maxY: b.y + b.h,
    minZ: b.z - b.d / 2, maxZ: b.z + b.d / 2,
  }));
}

function texture(base, variation) {
  const canvas = document.createElement('canvas'); canvas.width = canvas.height = 64;
  const context = canvas.getContext('2d'); const image = context.createImageData(64, 64);
  let seed = 731;
  for (let i = 0; i < image.data.length; i += 4) {
    seed = (1664525 * seed + 1013904223) >>> 0;
    const noise = ((seed / 4294967296) - .5) * variation;
    for (let c = 0; c < 3; c++) image.data[i + c] = base[c] + noise;
    image.data[i + 3] = 255;
  }
  context.putImageData(image, 0, 0);
  const map = new THREE.CanvasTexture(canvas);
  map.colorSpace = THREE.SRGBColorSpace; map.magFilter = THREE.NearestFilter;
  map.wrapS = map.wrapT = THREE.RepeatWrapping;
  return map;
}

function sign(scene, text, x, y, z, width = 5) {
  const canvas = document.createElement('canvas'); canvas.width = 512; canvas.height = 128;
  const ctx = canvas.getContext('2d'); ctx.fillStyle = '#272e26'; ctx.fillRect(0, 0, 512, 128);
  ctx.strokeStyle = '#b8c081'; ctx.lineWidth = 5; ctx.strokeRect(8, 8, 496, 112);
  ctx.fillStyle = '#cbd1a3'; ctx.font = 'bold 46px monospace'; ctx.textAlign = 'center'; ctx.fillText(text, 256, 79);
  const map = new THREE.CanvasTexture(canvas); map.colorSpace = THREE.SRGBColorSpace;
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(width, width / 4), new THREE.MeshBasicMaterial({ map }));
  mesh.position.set(x, y, z); scene.add(mesh);
}

export function createMap(scene) {
  scene.background = new THREE.Color('#303a33'); scene.fog = new THREE.FogExp2('#303a33', .027);
  scene.add(new THREE.HemisphereLight('#b0bbae', '#25271b', 1.9));
  const sun = new THREE.DirectionalLight('#d2d6ba', 2.1); sun.position.set(-15, 30, 12);
  sun.castShadow = true; sun.shadow.mapSize.set(1024, 1024);
  Object.assign(sun.shadow.camera, { left: -38, right: 38, top: 38, bottom: -38, near: 1, far: 100 });
  sun.shadow.bias = -.001; scene.add(sun);
  const mats = {
    concrete: new THREE.MeshStandardMaterial({ map: texture([103, 108, 92], 35), roughness: 1 }),
    rust: new THREE.MeshStandardMaterial({ map: texture([91, 69, 47], 45), roughness: .95 }),
    dark: new THREE.MeshStandardMaterial({ color: '#232c27', roughness: .8 }),
  };
  const groundMap = texture([66, 73, 60], 36); groundMap.repeat.set(36, 36);
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(220, 220), new THREE.MeshStandardMaterial({ map: groundMap, roughness: 1 }));
  ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
  for (const b of blocks) {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(b.w, b.h, b.d), mats[b.material]);
    mesh.position.set(b.x, b.y + b.h / 2, b.z); mesh.castShadow = mesh.receiveShadow = true; scene.add(mesh);
  }
  const paint = new THREE.MeshStandardMaterial({ color: '#b4b18a', roughness: 1 });
  for (let z = -28; z < 30; z += 6) {
    const stripe = new THREE.Mesh(new THREE.PlaneGeometry(.12, 2.5), paint);
    stripe.rotation.x = -Math.PI / 2; stripe.position.set(2, .015, z); scene.add(stripe);
  }
  const lit = new THREE.MeshBasicMaterial({ color: '#d2dc94' });
  for (const [x, z] of [[-5, -16], [11, 15], [-21, -1]]) {
    const pole = new THREE.Mesh(new THREE.CylinderGeometry(.09, .12, 6, 6), mats.rust);
    pole.position.set(x, 3, z); scene.add(pole);
    const head = new THREE.Mesh(new THREE.BoxGeometry(1.3, .18, .6), lit); head.position.set(x, 6, z); scene.add(head);
    const light = new THREE.PointLight('#d9dd8a', 12, 13, 2); light.position.set(x, 5.7, z); scene.add(light);
    // Include narrow props in physics as well as visible geometry.
  }
  // Building windows are emissive surfaces, not expensive individual lights.
  for (const [x, baseZ, width, height] of [[18, -15.45, 14, 9], [-26, -17.95, 10, 12], [25, 19.55, 10, 7], [-26, 23.55, 9, 8]]) {
    for (let floor = 2; floor < height; floor += 2.5) for (let col = -width / 2 + 1.5; col < width / 2 - 1; col += 2.5) {
      const window = new THREE.Mesh(new THREE.PlaneGeometry(.85, 1.25), (col + floor) % 3 > 1 ? lit : mats.dark);
      window.position.set(x + col, floor, baseZ); scene.add(window);
    }
  }
  sign(scene, 'SECTOR / 00', 0, 3, -34.45, 8);
  sign(scene, 'TRAINING', -13, 4, -1.72, 4.6);
  sign(scene, 'KEEP CLEAR', 7, 1.6, -3.73, 3.8);
  const colliders = getColliders();
  for (const [x, z] of [[-5, -16], [11, 15], [-21, -1]]) colliders.push({ minX: x - .12, maxX: x + .12, minY: 0, maxY: 6, minZ: z - .12, maxZ: z + .12 });
  return { colliders, spawn: SPAWN };
}
