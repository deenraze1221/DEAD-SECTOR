import * as THREE from 'three';
import { createNavigation } from './navigation.js';
export function obstacleDistance(origin,direction,colliders,max=100){
 const ray=new THREE.Ray(origin,direction),point=new THREE.Vector3();let nearest=max;
 for(const b of colliders){const box=new THREE.Box3(new THREE.Vector3(b.minX,b.minY,b.minZ),new THREE.Vector3(b.maxX,b.maxY,b.maxZ));if(ray.intersectBox(box,point))nearest=Math.min(nearest,point.distanceTo(origin));}return nearest;
}
export class Enemies {
 constructor(scene,colliders,audio){
  this.colliders=colliders;this.nav=createNavigation(colliders);this.audio=audio;this.items=[];
  const flesh=new THREE.MeshStandardMaterial({color:'#8a7866',roughness:1,flatShading:true});
  const uniform=new THREE.MeshStandardMaterial({color:'#424b36',roughness:1});
  const wound=new THREE.MeshStandardMaterial({color:'#68352c',roughness:1});
  const eye=new THREE.MeshBasicMaterial({color:'#dbbd71'});
  for(const [i,[x,z]] of [[-5,1],[9,1],[-13,-7],[5,-16],[-9,19]].entries()){
   const rig=new THREE.Group();rig.position.set(x,0,z);scene.add(rig);
   const mesh=(geo,mat,parent,px,py,pz)=>{const m=new THREE.Mesh(geo,mat);m.position.set(px,py,pz);m.castShadow=m.receiveShadow=true;parent.add(m);return m;};
   mesh(new THREE.BoxGeometry(.6,.67,.33),uniform,rig,0,1.13,0);
   mesh(new THREE.IcosahedronGeometry(.23,0),wound,rig,.2,1.42,.06);
   const head=mesh(new THREE.IcosahedronGeometry(.245,1),flesh,rig,0,1.72,.04);head.scale.set(.8,1.15,.9);
   mesh(new THREE.BoxGeometry(.2,.16,.05),wound,rig,0,1.62,.23);
   for(const x of [-.085,.085])mesh(new THREE.BoxGeometry(.04,.028,.025),eye,rig,x,1.78,.24);
   const limbs=[];
   for(const side of [-1,1]){
    const leg=new THREE.Group();leg.position.set(side*.18,.85,0);rig.add(leg);mesh(new THREE.BoxGeometry(.2,.69,.23),uniform,leg,0,-.35,0);mesh(new THREE.BoxGeometry(.22,.15,.34),uniform,leg,0,-.75,.06);limbs.push(leg);
    const arm=new THREE.Group();arm.position.set(side*.39,1.4,0);arm.rotation.z=side*.13;rig.add(arm);mesh(new THREE.BoxGeometry(.15,.64,.17),flesh,arm,0,-.31,.12);mesh(new THREE.BoxGeometry(.18,.17,.14),wound,arm,0,-.67,.15);
    for(let f=0;f<3;f++)mesh(new THREE.ConeGeometry(.024,.16,4),flesh,arm,(f-1)*.055,-.8,.15).rotation.z=Math.PI;
    limbs.push(arm);
   }
   this.items.push({rig,limbs,hp:100,attack:0,repath:0,path:[],phase:i,aggro:false,flash:0});
  }
 }
 get alive(){return this.items.filter(e=>e.hp>0).length;}
 reset(){const positions=[[-5,1],[9,1],[-13,-7],[5,-16],[-9,19]];this.items.forEach((e,i)=>{e.hp=100;e.rig.position.set(positions[i][0],0,positions[i][1]);e.rig.rotation.set(0,0,0);e.attack=0;e.repath=0;e.path=[];e.aggro=false;e.limbs.forEach(l=>l.rotation.x=0);});}
 step(dt,player,hurt){
  for(const e of this.items){if(e.hp<=0)continue;e.attack-=dt;e.repath-=dt;const p=e.rig.position;
   const distance=Math.hypot(player.position.x-p.x,player.position.z-p.z);
   if(!e.aggro&&distance<25){const origin=p.clone().add(new THREE.Vector3(0,1.5,0)),target=new THREE.Vector3(player.position.x,player.position.y+1.5,player.position.z);const dir=target.sub(origin);const length=dir.length();if(obstacleDistance(origin,dir.normalize(),this.colliders,length)>=length-.1)e.aggro=true;}
   if(!e.aggro)continue;
   if(distance<1.25&&Math.abs(player.position.y-p.y)<1.3){
    const origin=p.clone().add(new THREE.Vector3(0,1,0)),dir=new THREE.Vector3(player.position.x,player.position.y+1,player.position.z).sub(origin);const length=dir.length();
    if(e.attack<=0&&obstacleDistance(origin,dir.normalize(),this.colliders,length)>=length-.05){hurt(12);e.attack=.95;}
   }else{
    if(e.repath<=0){e.path=this.nav.path(p,player.position);e.repath=.65;}
    const target=e.path[0];if(target){const dx=target.x-p.x,dz=target.z-p.z,len=Math.hypot(dx,dz),step=Math.min(len,dt*1.5);if(len>.001){p.x+=dx/len*step;p.z+=dz/len*step;}if(len<.12)e.path.shift();}
   }
   e.phase+=dt*5;e.rig.rotation.y=Math.atan2(player.position.x-p.x,player.position.z-p.z);
   e.limbs.forEach((l,i)=>{l.rotation.x=Math.sin(e.phase+(i%2)*Math.PI)*.4+(i%2===1?-.5:0);});
  }
 }
 shoot(origin,direction){
  const wall=obstacleDistance(origin,direction,this.colliders);const ray=new THREE.Ray(origin,direction),point=new THREE.Vector3();let hit=null,nearest=wall;
  for(const e of this.items){if(e.hp<=0)continue;const p=e.rig.position;const box=new THREE.Box3(new THREE.Vector3(p.x-.43,p.y,p.z-.43),new THREE.Vector3(p.x+.43,p.y+2,p.z+.43));if(ray.intersectBox(box,point)){const d=point.distanceTo(origin);if(d<nearest){nearest=d;hit=e;}}}
  if(hit){hit.hp=Math.max(0,hit.hp-34);hit.aggro=true;this.audio.play('hit');if(!hit.hp){hit.rig.rotation.x=-Math.PI/2;hit.rig.position.y=.25;}return true;}return false;
 }
}
