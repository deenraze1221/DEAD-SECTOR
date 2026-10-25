import { readFile, writeFile } from 'node:fs/promises';
import * as THREE from 'three';
import { FBXLoader } from 'three/addons/loaders/FBXLoader.js';

// Offline conversion keeps the original FBX and embeds the mesh in the standalone build.
const bytes = await readFile('assets/models/gun.fbx');
const source = new FBXLoader().parse(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength), '');
source.updateMatrixWorld(true);
const meshes = [];
source.traverse(object => { if (object.isMesh) meshes.push(object); });
if (meshes.length !== 1) throw new Error('Expected the supplied single-mesh weapon');
const geometry = meshes[0].geometry.clone().applyMatrix4(meshes[0].matrixWorld);
geometry.computeBoundingBox();
const bounds = geometry.boundingBox, center = bounds.getCenter(new THREE.Vector3());
geometry.translate(-center.x, -center.y, -center.z);
geometry.scale(.9 / (bounds.max.z - bounds.min.z), .9 / (bounds.max.z - bounds.min.z), .9 / (bounds.max.z - bounds.min.z));
geometry.computeBoundingBox();
await writeFile('assets/models/gun.geometry.json', JSON.stringify(geometry.toJSON()));
console.log(`Converted original FBX: ${geometry.attributes.position.count / 3} triangles`);
