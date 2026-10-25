import {readFile,writeFile} from 'node:fs/promises';
import * as THREE from 'three';
import {FBXLoader} from 'three/addons/loaders/FBXLoader.js';
const bytes=await readFile('assets/imported/Guns/SciFi_gun_2-2.fbx');
const root=new FBXLoader().parse(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength),'');root.updateMatrixWorld(true);
const records=[];
for(const [id,label,length] of [['RifleGod','GOD / ШТУРМОВАЯ',.95],['RifleBran','BRAN / ВИНТОВКА',.95],['SMG','SMG / ПИСТОЛЕТ-ПУЛЕМЁТ',.75],['Pistol','ПИСТОЛЕТ',.48]]){
 const body=root.getObjectByName(id),mag=root.getObjectByName(id+'Magazine');if(!body?.isMesh||!mag?.isMesh)throw Error('Missing weapon or matching magazine: '+id);
 const bounds=new THREE.Box3().setFromObject(body),center=bounds.getCenter(new THREE.Vector3()),scale=length/(bounds.max.z-bounds.min.z);
 function convert(mesh){const g=mesh.geometry.clone().applyMatrix4(mesh.matrixWorld);g.translate(0,-center.y,-center.z);g.scale(scale,scale,scale);g.rotateY(Math.PI);return {geometry:g.toJSON(),materials:(Array.isArray(mesh.material)?mesh.material:[mesh.material]).map(m=>({name:m.name,color:m.color.getHex()}))};}
 const bodyPart=convert(body),magPart=convert(mag);
 const positions=bodyPart.geometry.data.attributes.position.array;let lo=Infinity,hi=-Infinity;
 for(let i=0;i<positions.length;i+=3)if(positions[i+2]<-length*.49){lo=Math.min(lo,positions[i+1]);hi=Math.max(hi,positions[i+1]);}
 records.push({id,label,length,muzzleY:(lo+hi)/2,body:bodyPart,magazine:magPart});
}
await writeFile('assets/models/weapons.json',JSON.stringify(records));console.log('Converted 4 weapons and 4 named magazines.');
