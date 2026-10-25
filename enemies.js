import * as T from 'three';
import {createNavigation} from './navigation.js';
import {ENEMY_TYPES} from './waves.js';
import {createMutantVisuals,poseMutant,ZOMBIE_SKINS} from './mutant-visuals.js';
import {GoreParticles} from './gore-particles.js';
const collisionBoxes=new WeakMap();
export function obstacleDistance(origin,direction,colliders,max=100){
 let cached=collisionBoxes.get(colliders);if(!cached||cached.count!==colliders.length){cached={count:colliders.length,boxes:colliders.map(b=>new T.Box3(new T.Vector3(b.minX,b.minY,b.minZ),new T.Vector3(b.maxX,b.maxY,b.maxZ)))};collisionBoxes.set(colliders,cached);}
 const ray=new T.Ray(origin,direction),point=new T.Vector3(),ex=origin.x+direction.x*max,ey=origin.y+direction.y*max,ez=origin.z+direction.z*max,minX=Math.min(origin.x,ex),maxX=Math.max(origin.x,ex),minY=Math.min(origin.y,ey),maxY=Math.max(origin.y,ey),minZ=Math.min(origin.z,ez),maxZ=Math.max(origin.z,ez);let nearest=max;
 for(const b of cached.boxes){if(b.max.x<minX||b.min.x>maxX||b.max.y<minY||b.min.y>maxY||b.max.z<minZ||b.min.z>maxZ)continue;if(ray.intersectBox(b,point))nearest=Math.min(nearest,point.distanceTo(origin));}return nearest;
}
export class Enemies{
 constructor(scene,colliders,audio,spawns=[]){this.scene=scene;this.colliders=colliders;this.audio=audio;this.items=[];this.projectiles=[];this.nextId=0;this.nav=createNavigation(colliders,.5);this.fatNav=createNavigation(colliders,2.15,6);this.airNav=createNavigation(colliders,.8,2,3.5);this.geometry=new Map();this.skinMaterials=new Map();this.blood=new GoreParticles(scene);this.materials={flesh:new T.MeshStandardMaterial({color:'#8f7060',roughness:1,flatShading:true}),uniform:new T.MeshStandardMaterial({color:'#454f36',roughness:1}),wound:new T.MeshStandardMaterial({color:'#773d37',roughness:1}),bone:new T.MeshStandardMaterial({color:'#b5b49b'}),iris:new T.MeshBasicMaterial({color:'#cb9d35'}),black:new T.MeshBasicMaterial({color:'#17201c'}),bullet:new T.MeshBasicMaterial({color:'#ee9360'})};for(const [x,z]of spawns)this.spawn('zombie',x,z);}
 material(color,basic=false){const key=color+basic;if(!this.skinMaterials.has(key))this.skinMaterials.set(key,basic?new T.MeshBasicMaterial({color}):new T.MeshStandardMaterial({color,roughness:.94,flatShading:true}));return this.skinMaterials.get(key);}
 mesh(parent,kind,size,material,x,y,z){
  const key=kind+size.join(',');
  if(!this.geometry.has(key))this.geometry.set(key,kind==='box'?new T.BoxGeometry(...size):kind==='sphere'?new T.IcosahedronGeometry(...size):kind==='capsule'?new T.CapsuleGeometry(...size):kind==='cylinder'?new T.CylinderGeometry(...size):new T.ConeGeometry(...size));
  const m=new T.Mesh(this.geometry.get(key),typeof material==='string'?this.materials[material]:material);m.position.set(x,y,z);parent.add(m);return m;
 }
 spawn(type,x,z,id=this.nextId++,skin=id%ZOMBIE_SKINS.length){
  const stats=ENEMY_TYPES[type],rig=new T.Group();rig.position.set(x,type==='eye'?4.5:0,z);rig.scale.setScalar(stats.scale);rig.name='mutant-'+type;this.scene.add(rig);
  skin=Math.max(0,Math.min(ZOMBIE_SKINS.length-1,Math.floor(skin)));
  const visual=createMutantVisuals(this,type,skin);rig.add(visual.root);
  const e={id,type,skin,...stats,maxHp:stats.hp,rig,visual,limbs:visual.limbs,phase:id*1.37,motion:0,attackTime:0,attack:1,repath:0,path:[],aggro:true,stagger:0,sound:2+id%3,corpse:0,dead:false};
  this.items.push(e);this.nextId=Math.max(this.nextId,id+1);poseMutant(e);return e;
 }
 die(enemy,emit=true){if(enemy.dead)return;enemy.dead=true;enemy.hp=0;enemy.corpse=0;enemy.rig.visible=false;if(emit)this.blood.burst(enemy);}
 updateEffects(dt,remote=false){
  this.blood.step(dt);
  if(remote)for(const e of this.items){if(e.hp<=0)continue;e.phase+=dt*e.speed/(e.scale*.9)*Math.PI*2*e.motion;e.attackTime=Math.max(0,e.attackTime-dt);poseMutant(e);}
 }
 get alive(){return this.items.reduce((n,e)=>n+(e.hp>0),0);}
 reset(){this.blood.clear();for(const e of this.items){e.hp=e.maxHp;e.stagger=0;e.corpse=0;e.dead=false;e.motion=0;e.attackTime=0;e.rig.visible=true;e.rig.rotation.set(0,0,0);poseMutant(e);}}
 step(dt,player,hurt,other=null,hurtOther=()=>{}){
 const living=[{p:player,hurt},...(other?[{p:other,hurt:hurtOther}]:[])].filter(v=>v.p.hp!==0);
 for(const e of this.items){if(e.hp<=0){this.die(e);e.corpse+=dt;continue;}e.attack-=dt;e.repath-=dt;e.stagger=Math.max(0,e.stagger-dt);e.sound-=dt;e.attackTime=Math.max(0,e.attackTime-dt);if(!living.length)continue;
 const p=e.rig.position,target=living.reduce((a,b)=>Math.hypot(a.p.position.x-p.x,a.p.position.z-p.z)<Math.hypot(b.p.position.x-p.x,b.p.position.z-p.z)?a:b),goal=target.p.position,distance=Math.hypot(goal.x-p.x,goal.z-p.z);
 if(e.sound<=0){if(distance<25)this.audio.play('growl',1-distance/28);e.sound=3+e.id%3;}const beforeX=p.x,beforeZ=p.z,yaw=Math.atan2(goal.x-p.x,goal.z-p.z),turn=Math.atan2(Math.sin(yaw-e.rig.rotation.y),Math.cos(yaw-e.rig.rotation.y));e.rig.rotation.y+=T.MathUtils.clamp(turn,-dt*3,dt*3);
 if(e.stagger>0){e.rig.rotation.z=Math.sin(e.stagger*45)*.035;e.motion=0;poseMutant(e);continue;}e.rig.rotation.z=0;
 if(e.type==='eye')p.y=4.5+Math.sin(e.phase*.4)*.3;
 const melee=e.type!=='eye'&&distance<e.radius+.85&&Math.abs(goal.y-p.y)<e.height;
 if(melee&&e.attack<=0){const origin=p.clone().add(new T.Vector3(0,Math.min(1.5,e.height*.7),0)),dir=new T.Vector3(goal.x,goal.y+1,goal.z).sub(origin),length=dir.length();if(obstacleDistance(origin,dir.normalize(),this.colliders,length)>=length-.05){target.hurt(e.damage);e.attackTime=.42;e.attack=e.type==='runner'?.65:.85;}}
 if(e.type==='eye'&&distance<40&&e.attack<=0){const origin=p.clone(),direction=new T.Vector3(goal.x,goal.y+1,goal.z).sub(origin),len=direction.length();if(obstacleDistance(origin,direction.normalize(),this.colliders,len)>=len-.1){const mesh=this.mesh(this.scene,'sphere',[.2,0],'bullet',p.x,p.y,p.z);this.projectiles.push({mesh,velocity:direction.multiplyScalar(10),life:6});e.attack=2.2;}}
 if(!melee&&(e.type!=='eye'||distance>16)){
  if(e.repath<=0){e.path=(e.type==='brute'?this.fatNav:e.type==='eye'?this.airNav:this.nav).path(p,goal);e.repath=.6;}
  const point=e.path[0];if(point){const dx=point.x-p.x,dz=point.z-p.z,len=Math.hypot(dx,dz),step=Math.min(len,e.speed*dt);if(len>.001){p.x+=dx/len*step;p.z+=dz/len*step;}if(len<.15)e.path.shift();}
 }
 const travel=Math.hypot(p.x-beforeX,p.z-beforeZ);e.motion=T.MathUtils.damp(e.motion,travel>1e-6?1:0,14,dt);e.phase+=travel/(e.scale*.9)*Math.PI*2;poseMutant(e);
 }
 for(const shot of this.projectiles){shot.life-=dt;const origin=shot.mesh.position,delta=shot.velocity.clone().multiplyScalar(dt),length=delta.length(),direction=delta.clone().normalize(),ray=new T.Ray(origin.clone(),direction);let travel=obstacleDistance(origin,direction,this.colliders,length);
 for(const target of living){const p=target.p.position,hit=ray.intersectBox(new T.Box3(new T.Vector3(p.x-.32,p.y,p.z-.32),new T.Vector3(p.x+.32,p.y+1.8,p.z+.32)),new T.Vector3());if(hit&&hit.distanceTo(origin)<=travel){target.hurt(30);shot.life=0;}}
 if(travel<length-.001)shot.life=0;origin.add(delta);}
 this.items=this.items.filter(e=>{if(e.hp>0||e.corpse<2)return true;this.scene.remove(e.rig);return false;});
 this.projectiles=this.projectiles.filter(s=>{if(s.life<=0){this.scene.remove(s.mesh);return false;}return true;});
 }
 shoot(origin,direction,damage=35){const wall=obstacleDistance(origin,direction,this.colliders),ray=new T.Ray(origin,direction);let nearest=wall,hit=null;for(const e of this.items){if(e.hp<=0)continue;const p=e.rig.position,r=e.radius,box=new T.Box3(new T.Vector3(p.x-r,p.y-(e.type==='eye'?r:0),p.z-r),new T.Vector3(p.x+r,p.y+(e.type==='eye'?r:e.height),p.z+r)),point=ray.intersectBox(box,new T.Vector3());if(point&&point.distanceTo(origin)<nearest){nearest=point.distanceTo(origin);hit=e;}}
 if(!hit)return false;hit.hp=Math.max(0,hit.hp-damage);hit.stagger=.10;this.audio.play('hit');if(!hit.hp)this.die(hit);return true;}
 snapshot(){return this.items.filter(e=>e.hp>0||e.corpse<2).map(e=>({id:e.id,type:e.type,skin:e.skin,hp:e.hp,x:e.rig.position.x,y:e.rig.position.y,z:e.rig.position.z,rotation:e.rig.rotation.toArray(),phase:e.phase,motion:e.motion,attackTime:e.attackTime,corpse:e.corpse}));}
 apply(records){
  const keep=new Set(records.map(v=>v.id));
  this.items=this.items.filter(e=>{if(keep.has(e.id))return true;this.scene.remove(e.rig);return false;});
  for(const v of records){
   const existing=this.items.find(e=>e.id===v.id),e=existing||this.spawn(v.type,v.x,v.z,v.id,v.skin);
   const wasAlive=existing?.hp>0;e.hp=v.hp;e.rig.position.set(v.x,v.y??0,v.z);if(v.rotation)e.rig.rotation.fromArray(v.rotation);
   if(e.hp<=0){this.die(e,wasAlive||(v.corpse??0)<.3);e.corpse=v.corpse??0;}
   else{e.rig.visible=true;e.dead=false;e.phase=v.phase??e.phase;e.motion=v.motion??0;e.attackTime=v.attackTime??0;poseMutant(e);}
  }
 }
}
