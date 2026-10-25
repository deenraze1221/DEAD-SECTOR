import * as T from 'three';import {generateTown,SPAWN} from './town.js';import {createCity} from './city.js';export {SPAWN};
export function getColliders(seed=1){return generateTown(seed).colliders;}
export function createMap(scene,seed=1){const layout=generateTown(seed);scene.background=new T.Color('#303a33');scene.fog=new T.FogExp2('#303a33',.016);scene.add(new T.HemisphereLight('#bec7b5','#25291f',2));const sun=new T.DirectionalLight('#dfd7b6',2.5);sun.position.set(-30,60,25);scene.add(sun);
const ground=new T.Mesh(new T.PlaneGeometry(200,200),new T.MeshStandardMaterial({color:'#505a4b',roughness:1}));ground.rotation.x=-Math.PI/2;scene.add(ground);
const road=new T.MeshStandardMaterial({color:'#303934',roughness:1}),paint=new T.MeshStandardMaterial({color:'#aaa785'});
for(const axis of [0,1])for(const offset of [-60,0,60]){const r=new T.Mesh(new T.PlaneGeometry(axis?200:12,axis?12:200),road);r.rotation.x=-Math.PI/2;r.position.set(axis?0:offset,.015,axis?offset:0);scene.add(r);for(let n=-96;n<99;n+=8){const m=new T.Mesh(new T.PlaneGeometry(axis?3:.12,axis?.12:3),paint);m.rotation.x=-Math.PI/2;m.position.set(axis?n:offset,.02,axis?offset:n);scene.add(m);}}
const pavement=new T.MeshStandardMaterial({color:'#6a7062',roughness:1});
for(const axis of [0,1])for(const offset of [-60,0,60])for(const side of [-1,1]){const strip=new T.Mesh(new T.PlaneGeometry(axis?200:2.5,axis?2.5:200),pavement);strip.rotation.x=-Math.PI/2;strip.position.set(axis?0:offset+side*7.4,.009,axis?offset+side*7.4:0);scene.add(strip);}
for(const b of layout.buildings){const yard=new T.Mesh(new T.PlaneGeometry(b.w+6,b.d+7),pavement);yard.rotation.x=-Math.PI/2;yard.position.set(b.x,.005,b.z+1);scene.add(yard);}
createCity(scene,layout);return {...layout,spawn:layout.spawn};}
