import * as T from 'three';
// Fixed buffers: decorative ash and electrical sparks never affect collisions.
export class AtmosphereParticles{
 constructor(scene,buildings,lamps){this.buildings=buildings;this.lamps=lamps;this.time=0;this.ash=this.make(scene,'drifting-ash',750,'#a1aaa0',.045,.48);this.sparks=this.make(scene,'lamp-sparks',64,'#ffc875',.055,.95);this.sparks.material.blending=T.AdditiveBlending;}
 make(scene,name,count,color,size,opacity){const geometry=new T.BufferGeometry();geometry.setAttribute('position',new T.BufferAttribute(new Float32Array(count*3),3).setUsage(T.DynamicDrawUsage));const p=new T.Points(geometry,new T.PointsMaterial({color,size,transparent:true,opacity,depthWrite:false,sizeAttenuation:true}));p.name=name;p.frustumCulled=false;scene.add(p);return p;}
 update(dt,position,quality,powered){this.time+=Math.max(0,dt);const time=this.time,count={low:180,medium:420,high:750}[quality]||420,a=this.ash.geometry.attributes.position;this.ash.geometry.setDrawRange(0,count);
 for(let i=0;i<count;i++){const x=position.x+((i*7.719+time*.22)%30)-15,z=position.z+((i*11.127+time*.13)%30)-15,y=((i*.731-time*.16)%9+9)%9;const roof=this.buildings.some(b=>Math.abs(x-b.x)<b.w/2&&Math.abs(z-b.z)<b.d/2&&y<b.h+.15);a.setXYZ(i,x+Math.sin(time*.4+i)*.23,roof?-100:y,z);}a.needsUpdate=true;
 const lamp=this.lamps[0],burst=time%3.7,active=lamp?.visible&&!powered&&burst<.65;this.sparks.visible=!!active;if(active){const s=this.sparks.geometry.attributes.position,n=quality==='low'?12:quality==='high'?64:32;this.sparks.geometry.setDrawRange(0,n);for(let i=0;i<n;i++){const age=(burst+i*.013)%.65,angle=i*2.399;s.setXYZ(i,lamp.position.x+Math.cos(angle)*age*.9,lamp.position.y-.06-age*age*3,lamp.position.z+Math.sin(angle)*age*.9);}s.needsUpdate=true;}
 }
}
