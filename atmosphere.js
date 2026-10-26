import * as T from 'three';
// Decorative details never obstruct the tested player routes. Shared geometry/materials.
export function createAtmosphere(scene,layout){
 const box=new T.BoxGeometry(1,1,1),parts=[],mats={wood:new T.MeshStandardMaterial({color:'#67513c'}),black:new T.MeshStandardMaterial({color:'#242c2b'}),rubble:new T.MeshStandardMaterial({color:'#787568'}),rust:new T.MeshStandardMaterial({color:'#765340'}),paper:new T.MeshStandardMaterial({color:'#b4ad8b'})};
 const batches=new Map();const put=(mat,x,y,z,w,h,d,ry=0)=>{if(!batches.has(mat))batches.set(mat,[]);const o=new T.Object3D();o.position.set(x,y,z);o.scale.set(w,h,d);o.rotation.y=ry;o.updateMatrix();batches.get(mat).push(o.matrix);};
 for(const b of layout.buildings){if(b.type==='grocery')continue;const front=b.z+b.d/2;
  if(b.id%3===0)for(const side of [-1,1]){const x=b.x+side*b.w*.3;put('black',x,4.7,front+.082,.75,1,.016);for(let n=0;n<3;n++)put('wood',x,4.35+n*.30,front+.11,.98,.13,.045,0);}
  for(let n=0;n<3;n++){const x=b.x-b.w*.46+n*.14;put('black',x,.4+n*.27,front+.004,.012,.36,.008,.05);}
 }
 for(const p of layout.props){if(p.type==='dumpster'){for(let n=0;n<4;n++){const dx=.65+(n%2)*.22,dz=(Math.floor(n/2)-.5)*.35;put(n%2?'paper':'black',p.x+dx,.025,p.z+dz,.14,.035,.11,n);}}if(['car','van','hatchback'].includes(p.type)&&p.variant%4===0){put('rust',p.x,.925,p.z+.8,.55,.018,.45);}}
 for(const [mat,records]of batches){const mesh=new T.InstancedMesh(box,mats[mat],records.length);records.forEach((matrix,i)=>mesh.setMatrixAt(i,matrix));mesh.name='weathering/'+mat;scene.add(mesh);}
 const warm=new T.PointLight('#ffd48c',16,16,2);warm.position.set(layout.shop.terminal.x+1,2.6,layout.shop.terminal.z+1);scene.add(warm);
 const lamps=[];for(const p of layout.props.filter(p=>p.type==='lamp').filter((_,i)=>i%12===0)){const l=new T.PointLight('#8daac8',5,11,2);l.position.set(p.x,4.7,p.z);scene.add(l);lamps.push(l);}
 return {update(time,powered){lamps.forEach((l,i)=>l.intensity=powered?8:5*(i===0&&Math.sin(time*9)*Math.sin(time*2)>.72?.15:1));warm.intensity=powered?23:16;}};
}
