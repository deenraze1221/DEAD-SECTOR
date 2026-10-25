import * as T from 'three';
export class Supplies{
 constructor(scene,records){this.items=records.map((r,id)=>{const group=new T.Group();group.position.set(r.x,.55,r.z);group.name='supply-'+r.type;const mat=new T.MeshStandardMaterial({color:r.type==='health'?'#c8cbb7':'#aa9864'});const body=new T.Mesh(new T.BoxGeometry(.55,.35,.35),mat);group.add(body);const mark=new T.MeshBasicMaterial({color:r.type==='health'?'#bb493c':'#262f24'});for(const [w,h]of r.type==='health'?[[.3,.075],[.075,.25]]:[[.3,.15]]){const m=new T.Mesh(new T.BoxGeometry(w,h,.02),mark);m.position.z=.19;group.add(m);}scene.add(group);return {...r,id,group,taken:false};});}
 tick(time){for(const s of this.items){s.group.visible=!s.taken;s.group.rotation.y=time*.7;s.group.position.y=.55+Math.sin(time*2+s.id)*.07;}}
 nearby(position){return this.items.find(s=>!s.taken&&Math.hypot(s.x-position.x,s.z-position.z)<1.5&&position.y<1.2);}
 apply(id,hp,gun){const s=this.items[id];if(!s||s.taken)return null;if(s.type==='health'){if(hp>=100)return null;hp=Math.min(100,hp+35);}else{if(gun.reserve>=gun.config.reserve*2)return null;gun.reserve=Math.min(gun.config.reserve*2,gun.reserve+gun.config.capacity*2);}s.taken=true;return hp;}
}
