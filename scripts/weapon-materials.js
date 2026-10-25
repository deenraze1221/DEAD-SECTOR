import * as T from 'three';
// FBX has no texture maps. Paint connected mechanical parts in linear vertex colors.
export function paintWeapon(geometry,id,length){
 const p=geometry.attributes.position,n=p.count,parent=Array.from({length:n/3},(_,i)=>i),points=new Map();
 const find=i=>{while(parent[i]!==i){parent[i]=parent[parent[i]];i=parent[i];}return i;};
 for(let i=0;i<n;i++){
  const key=[p.getX(i),p.getY(i),p.getZ(i)].map(v=>Math.round(v*1e5)).join('/'),triangle=Math.floor(i/3),other=points.get(key);
  if(other!==undefined)parent[find(triangle)]=find(other);else points.set(key,triangle);
 }
 const components=new Map();for(let i=0;i<n;i++){const key=find(Math.floor(i/3));if(!components.has(key))components.set(key,new T.Box3());components.get(key).expandByPoint(new T.Vector3(p.getX(i),p.getY(i),p.getZ(i)));}
 geometry.computeBoundingBox();const whole=geometry.boundingBox,h=whole.max.y-whole.min.y,colors=new Float32Array(n*3);
 const palette=Object.fromEntries(Object.entries({steel:'#87938f',body:'#404b48',wood:'#815d43',blue:'#3d6684',grip:'#51453b'}).map(([k,v])=>[k,new T.Color(v)]));
 const wood=['Kalash','Shotgun','Shambler','Tikhar'].includes(id),air=['Helsing','Tikhar'].includes(id);
 for(let i=0;i<n;i++){
  const b=components.get(find(Math.floor(i/3))),c=b.getCenter(new T.Vector3()),s=b.getSize(new T.Vector3());
  let material='body';
  if(s.z>length*.22&&s.y<h*.33||c.y>whole.min.y+h*.72)material='steel';
  if(wood&&c.z>length*.18&&c.y<whole.min.y+h*.62)material='wood';
  if(wood&&c.z<-length*.10&&c.z>-length*.33&&c.y<whole.min.y+h*.47)material='wood';
  if(c.y<whole.min.y+h*.30&&c.z>0&&c.z<length*.24)material='grip';
  if(air&&c.z<length*.08&&c.y<whole.min.y+h*.51&&s.z>length*.15&&s.x>length*.025)material='blue';
  if(s.z>length*.45&&s.y>h*.5){
   const y=p.getY(i),z=p.getZ(i);
   if(wood&&z>length*.16&&y<whole.min.y+h*.60)material='wood';
   else if(wood&&z<-length*.08&&z>-length*.32&&y<whole.min.y+h*.47)material='wood';
   else if(y>whole.min.y+h*.72)material='steel';else material='body';
  }
  palette[material].toArray(colors,i*3);
 }
 geometry.setAttribute('color',new T.BufferAttribute(colors,3));geometry.clearGroups();geometry.addGroup(0,geometry.index?.count??n,0);
 return [{name:'Mechanical palette',color:0xffffff,vertexColors:true}];
}
export const GRIPS={
 Pistol:{trigger:[.05,-.10,.09],support:[-.045,-.065,.015]},
 Helsing:{trigger:[.045,-.10,.18],support:[-.06,-.055,-.18]},
 Tikhar:{trigger:[.045,-.075,.28],support:[-.05,-.04,-.13]},
 Shambler:{trigger:[.045,-.09,.20],support:[-.06,-.065,-.19]},
 Shotgun:{trigger:[.05,-.07,.23],support:[-.06,-.065,-.21]},
 Kalash:{trigger:[.04,-.10,.12],support:[-.05,-.045,-.16]},
 VSV:{trigger:[.045,-.08,.22],support:[-.06,-.055,-.18]},
 Scorpion:{trigger:[.04,-.12,.07],support:[-.05,-.065,-.13]},
 Vector:{trigger:[.04,-.17,.10],support:[-.05,-.12,-.07]},
 G36:{trigger:[.04,-.15,.10],support:[-.05,-.07,-.15]},
};
