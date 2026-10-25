import test from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import { Rifle } from '../weapons.js';
import { Enemies, obstacleDistance } from '../enemies.js';
import { createNavigation } from '../navigation.js';
import { getColliders } from '../map.js';
const audio={play(){}};
test('rifle enforces fire rate, magazine size, reserve and reload timing',()=>{
 const gun=new Rifle(audio,()=>0);for(let i=0;i<30;i++){assert.equal(gun.fire(),true);assert.equal(gun.fire(),false);gun.step(.12);}assert.equal(gun.ammo,0);assert.equal(gun.fire(),false);
 gun.reload();gun.step(.8);assert.equal(gun.fire(),false);assert.equal(gun.ammo,0);gun.step(1);assert.equal(gun.ammo,30);assert.equal(gun.reserve,60);
 gun.fire();gun.step(.12);gun.reload();gun.step(2);assert.equal(gun.ammo,30);assert.equal(gun.reserve,59);gun.reset();assert.equal(gun.reserve,90);
});
test('obstacles occlude shots, three hits kill a mutant, dead mutants cannot be hit',()=>{
 const enemies=new Enemies(new THREE.Scene(),[],audio);const origin=new THREE.Vector3(-5,1.5,5),direction=new THREE.Vector3(0,0,-1);
 const wall={minX:-6,maxX:-4,minY:0,maxY:3,minZ:2,maxZ:3};enemies.colliders=[wall];assert.equal(enemies.shoot(origin,direction),false);assert.equal(enemies.items[0].hp,100);
 enemies.colliders=[];for(let i=0;i<3;i++)assert.equal(enemies.shoot(origin,direction),true);assert.equal(enemies.items[0].hp,0);assert.equal(enemies.alive,4);assert.equal(enemies.shoot(origin,direction),false);
 enemies.reset();assert.equal(enemies.alive,5);assert.equal(enemies.items[0].rig.rotation.x,0);
});
test('all five enemies can be killed, enabling the clear-sector condition',()=>{
 const enemies=new Enemies(new THREE.Scene(),[],audio);
 for(const e of enemies.items){const origin=e.rig.position.clone().add(new THREE.Vector3(0,1.5,2));for(let i=0;i<3;i++)enemies.shoot(origin,new THREE.Vector3(0,0,-1));}
 assert.equal(enemies.alive,0);
});
test('mutant pursues, damages at attack cadence, and dead mutant stops attacking',()=>{
 const enemies=new Enemies(new THREE.Scene(),[],audio);for(const e of enemies.items.slice(1))e.hp=0;
 const e=enemies.items[0],player={position:{x:-5,y:0,z:7}};let damage=0;
 for(let i=0;i<1200;i++)enemies.step(1/120,player,n=>damage+=n);
 assert.ok(e.rig.position.z>4);assert.ok(damage>=12);assert.ok(damage<120);e.hp=0;const before=damage;for(let i=0;i<240;i++)enemies.step(1/120,player,n=>damage+=n);assert.equal(damage,before);
});
test('navigation routes through building doorway and around cover without intersecting obstacles',()=>{
 const colliders=getColliders(),nav=createNavigation(colliders);
 for(const [from,to] of [[{x:-13,z:-7},{x:0,z:16}],[{x:9,z:1},{x:0,z:16}],[{x:5,z:-16},{x:0,z:-32}]]){
  const path=nav.path(from,to);assert.ok(path.length>0);
  for(const p of path)assert.equal(colliders.some(b=>b.minY<1.8&&p.x>b.minX-.4&&p.x<b.maxX+.4&&p.z>b.minZ-.4&&p.z<b.maxZ+.4),false);
 }
 { const d=obstacleDistance(new THREE.Vector3(0,1,0),new THREE.Vector3(0,0,-1),[{minX:-1,maxX:1,minY:0,maxY:3,minZ:-5,maxZ:-4}]);assert.equal(d,4); }
});

test('random loadout covers all four authored weapon/magazine pairs, with independent capacities and damage',()=>{
 for(let i=0;i<4;i++){
  const gun=new Rifle(audio,()=>(i+.1)/4);assert.equal(gun.config.id,['RifleGod','RifleBran','SMG','Pistol'][i]);
  assert.equal(gun.magazine.name,gun.config.id+'Magazine');assert.equal(gun.gun.name,gun.config.id);
  assert.equal(gun.variants.filter(v=>v.group.visible).length,1);assert.equal(gun.ammo,gun.config.capacity);
  gun.fire();gun.reload();gun.pose(0,false,false);assert.notEqual(gun.reloadTime,0);gun.step(gun.config.reload+.1);assert.equal(gun.ammo,gun.config.capacity);assert.equal(gun.reserve,gun.config.reserve-1);
  const enemies=new Enemies(new THREE.Scene(),[],audio),e=enemies.items[0],origin=new THREE.Vector3(-5,1.5,5),direction=new THREE.Vector3(0,0,-1);
  enemies.shoot(origin,direction,gun.config.damage);assert.equal(e.hp,100-gun.config.damage);
 }
});
test('a new run rerolls the weapon, clears previous reload and resets only the chosen magazine',()=>{
 let roll=0;const gun=new Rifle(audio,()=>roll);gun.fire();gun.reload();gun.pose(0,false,false);roll=.99;gun.reset();
 assert.equal(gun.config.id,'Pistol');assert.equal(gun.reloadTime,0);assert.equal(gun.ammo,12);assert.equal(gun.reserve,60);assert.equal(gun.magazine.position.y,0);
});
