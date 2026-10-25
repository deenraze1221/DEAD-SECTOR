import * as T from 'three';
import {createNavigation} from './navigation.js';
import {ENEMY_TYPES} from './waves.js';
const collisionBoxes=new WeakMap();
export function obstacleDistance(origin,direction,colliders,max=100){
 let cached=collisionBoxes.get(colliders);if(!cached||cached.count!==colliders.length){cached={count:colliders.length,boxes:colliders.map(b=>new T.Box3(new T.Vector3(b.minX,b.minY,b.minZ),new T.Vector3(b.maxX,b.maxY,b.maxZ)))};collisionBoxes.set(colliders,cached);}
 const ray=new T.Ray(origin,direction),point=new T.Vector3(),ex=origin.x+direction.x*max,ey=origin.y+direction.y*max,ez=origin.z+direction.z*max,minX=Math.min(origin.x,ex),maxX=Math.max(origin.x,ex),minY=Math.min(origin.y,ey),maxY=Math.max(origin.y,ey),minZ=Math.min(origin.z,ez),maxZ=Math.max(origin.z,ez);let nearest=max;
 for(const b of cached.boxes){if(b.max.x<minX||b.min.x>maxX||b.max.y<minY||b.min.y>maxY||b.max.z<minZ||b.min.z>maxZ)continue;if(ray.intersectBox(b,point))nearest=Math.min(nearest,point.distanceTo(origin));}return nearest;
}
export class Enemies{
 constructor(scene,colliders,audio,spawns=[]){this.scene=scene;this.colliders=colliders;this.audio=audio;this.items=[];this.projectiles=[];this.nextId=0;this.nav=createNavigation(colliders);this.fatNav=createNavigation(colliders,1.2,6);this.airNav=createNavigation(colliders,.8,2,3.5);this.geometry=new Map();this.materials={flesh:new T.MeshStandardMaterial({color:'#8f7060',roughness:1,flatShading:true}),uniform:new T.MeshStandardMaterial({color:'#454f36',roughness:1}),wound:new T.MeshStandardMaterial({color:'#773d37',roughness:1}),bone:new T.MeshStandardMaterial({color:'#b5b49b'}),iris:new T.MeshBasicMaterial({color:'#cb9d35'}),black:new T.MeshBasicMaterial({color:'#17201c'}),bullet:new T.MeshBasicMaterial({color:'#ee9360'})};for(const [x,z]of spawns)this.spawn('zombie',x,z);}
 mesh(parent,kind,size,material,x,y,z){const key=kind+size.join(',');if(!this.geometry.has(key))this.geometry.set(key,kind==='box'?new T.BoxGeometry(...size):kind==='sphere'?new T.IcosahedronGeometry(...size):kind==='capsule'?new T.CapsuleGeometry(...size):new T.ConeGeometry(...size));const m=new T.Mesh(this.geometry.get(key),this.materials[material]);m.position.set(x,y,z);parent.add(m);return m;}
 spawn(type,x,z,id=this.nextId++){const stats=ENEMY_TYPES[type],rig=new T.Group(),limbs=[];rig.position.set(x,type==='eye'?4.5:0,z);rig.name='mutant-'+type;this.scene.add(rig);
 if(type==='eye'){this.mesh(rig,'sphere',[.8,1],'wound',0,0,0);this.mesh(rig,'sphere',[.55,1],'bone',0,0,.48);this.mesh(rig,'sphere',[.32,1],'iris',0,0,.88);this.mesh(rig,'sphere',[.15,1],'black',0,0,1.1);for(let n=0;n<6;n++){const a=n*Math.PI/3;this.mesh(rig,'cone',[.1,.45,5],'flesh',Math.cos(a)*.64,Math.sin(a)*.64,-.2);}}
 else{rig.scale.setScalar(stats.scale);this.mesh(rig,'capsule',[.26,.32,3,6],'uniform',0,1.13,0);this.mesh(rig,'sphere',[.24,1],'flesh',0,1.72,0);this.mesh(rig,'box',[.2,.13,.08],'wound',0,1.6,.22);for(const side of [-1,1]){this.mesh(rig,'box',[.05,.035,.04],'iris',side*.085,1.78,.22);const leg=new T.Group();leg.position.set(side*.18,.85,0);rig.add(leg);this.mesh(leg,'capsule',[.09,.5,2,5],'uniform',0,-.35,0);limbs.push(leg);const arm=new T.Group();arm.position.set(side*.36,1.4,0);rig.add(arm);this.mesh(arm,'capsule',[.08,.55,2,5],'flesh',0,-.35,.1);for(let f=0;f<3;f++)this.mesh(arm,'cone',[.02,.16,4],'bone',(f-1)*.05,-.75,.14).rotation.z=Math.PI;limbs.push(arm);}for(let n=0;n<4;n++){this.mesh(rig,'box',[.4,.035,.06],'bone',0,1.3-n*.1,.26);}if(type==='brute'){const belly=this.mesh(rig,'sphere',[.42,1],'flesh',0,1.05,.1);belly.scale.set(1.3,1.1,1);}}
 const e={id,type,...stats,maxHp:stats.hp,rig,limbs,phase:id,attack:1,repath:0,path:[],aggro:true,stagger:0,sound:2+id%3,corpse:0};this.items.push(e);this.nextId=Math.max(this.nextId,id+1);return e;}
 get alive(){return this.items.reduce((n,e)=>n+(e.hp>0),0);}
 reset(){for(const e of this.items){e.hp=e.maxHp;e.stagger=0;e.corpse=0;e.rig.visible=true;e.rig.rotation.set(0,0,0);}}
 step(dt,player,hurt,other=null,hurtOther=()=>{}){
 const living=[{p:player,hurt},...(other?[{p:other,hurt:hurtOther}]:[])].filter(v=>v.p.hp!==0);
 for(const e of this.items){if(e.hp<=0){e.corpse+=dt;if(e.corpse>8)e.rig.visible=false;continue;}e.attack-=dt;e.repath-=dt;e.stagger=Math.max(0,e.stagger-dt);e.sound-=dt;if(!living.length)continue;
 const p=e.rig.position,target=living.reduce((a,b)=>Math.hypot(a.p.position.x-p.x,a.p.position.z-p.z)<Math.hypot(b.p.position.x-p.x,b.p.position.z-p.z)?a:b),goal=target.p.position,distance=Math.hypot(goal.x-p.x,goal.z-p.z);
 if(e.sound<=0){if(distance<25)this.audio.play('growl',1-distance/28);e.sound=3+e.id%3;}e.phase+=dt*(e.type==='runner'?10:5);e.rig.rotation.y=Math.atan2(goal.x-p.x,goal.z-p.z);
 if(e.stagger>0){e.rig.rotation.z=Math.sin(e.stagger*45)*.06;continue;}e.rig.rotation.z=0;
 if(e.type==='eye')p.y=4.5+Math.sin(e.phase*.4)*.3;
 const melee=e.type!=='eye'&&distance<e.radius+.85&&Math.abs(goal.y-p.y)<e.height;
 if(melee&&e.attack<=0){const origin=p.clone().add(new T.Vector3(0,Math.min(1.5,e.height*.7),0)),dir=new T.Vector3(goal.x,goal.y+1,goal.z).sub(origin),length=dir.length();if(obstacleDistance(origin,dir.normalize(),this.colliders,length)>=length-.05){target.hurt(e.damage);e.attack=e.type==='runner'?.65:.85;}}
 if(e.type==='eye'&&distance<40&&e.attack<=0){const origin=p.clone(),direction=new T.Vector3(goal.x,goal.y+1,goal.z).sub(origin),len=direction.length();if(obstacleDistance(origin,direction.normalize(),this.colliders,len)>=len-.1){const mesh=this.mesh(this.scene,'sphere',[.2,0],'bullet',p.x,p.y,p.z);this.projectiles.push({mesh,velocity:direction.multiplyScalar(10),life:6});e.attack=2.2;}}
 if(!melee&&(e.type!=='eye'||distance>16)){
  if(e.repath<=0){e.path=(e.type==='brute'?this.fatNav:e.type==='eye'?this.airNav:this.nav).path(p,goal);e.repath=.6;}
  const point=e.path[0];if(point){const dx=point.x-p.x,dz=point.z-p.z,len=Math.hypot(dx,dz),step=Math.min(len,e.speed*dt);if(len>.001){p.x+=dx/len*step;p.z+=dz/len*step;}if(len<.15)e.path.shift();}
 }
 e.limbs.forEach((limb,i)=>limb.rotation.x=Math.sin(e.phase+(i%2)*Math.PI)*.45+(i%2?-.45:0));
 }
 for(const shot of this.projectiles){shot.life-=dt;const origin=shot.mesh.position,delta=shot.velocity.clone().multiplyScalar(dt),length=delta.length(),direction=delta.clone().normalize(),ray=new T.Ray(origin.clone(),direction);let travel=obstacleDistance(origin,direction,this.colliders,length);
 for(const target of living){const p=target.p.position,hit=ray.intersectBox(new T.Box3(new T.Vector3(p.x-.32,p.y,p.z-.32),new T.Vector3(p.x+.32,p.y+1.8,p.z+.32)),new T.Vector3());if(hit&&hit.distanceTo(origin)<=travel){target.hurt(30);shot.life=0;}}
 if(travel<length-.001)shot.life=0;origin.add(delta);}
 this.items=this.items.filter(e=>{if(e.hp>0||e.corpse<8)return true;this.scene.remove(e.rig);return false;});
 this.projectiles=this.projectiles.filter(s=>{if(s.life<=0){this.scene.remove(s.mesh);return false;}return true;});
 }
 shoot(origin,direction,damage=35){const wall=obstacleDistance(origin,direction,this.colliders),ray=new T.Ray(origin,direction);let nearest=wall,hit=null;for(const e of this.items){if(e.hp<=0)continue;const p=e.rig.position,r=e.radius,box=new T.Box3(new T.Vector3(p.x-r,p.y-(e.type==='eye'?r:0),p.z-r),new T.Vector3(p.x+r,p.y+(e.type==='eye'?r:e.height),p.z+r)),point=ray.intersectBox(box,new T.Vector3());if(point&&point.distanceTo(origin)<nearest){nearest=point.distanceTo(origin);hit=e;}}
 if(!hit)return false;hit.hp=Math.max(0,hit.hp-damage);hit.stagger=.10;this.audio.play('hit');if(!hit.hp){hit.rig.rotation.x=-Math.PI/2;hit.rig.position.y=.25;}return true;}
 snapshot(){return this.items.filter(e=>e.hp>0||e.corpse<8).map(e=>({id:e.id,type:e.type,hp:e.hp,x:e.rig.position.x,y:e.rig.position.y,z:e.rig.position.z,rotation:e.rig.rotation.toArray(),limbs:e.limbs.map(l=>l.rotation.x)}));}
 apply(records){const keep=new Set(records.map(v=>v.id));this.items=this.items.filter(e=>{if(keep.has(e.id))return true;this.scene.remove(e.rig);return false;});for(const v of records){const e=this.items.find(e=>e.id===v.id)||this.spawn(v.type,v.x,v.z,v.id);e.hp=v.hp;e.rig.position.set(v.x,v.y??0,v.z);e.rig.rotation.fromArray(v.rotation);if(v.limbs)e.limbs.forEach((l,i)=>l.rotation.x=v.limbs[i]||0);}}
}
