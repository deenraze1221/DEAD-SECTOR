import * as T from 'three';
// Instanced construction keeps detailed models practical in a 64-building city.
export function createCity(scene,layout){
 const palettes={concrete:'#8b8e7e',brick:'#8d6652',plaster:'#a6a58c',dark:'#303c35',wood:'#806448',steel:'#627269',rust:'#865539',glass:'#3c5654',rubber:'#202924',fabric:'#77816a',white:'#c4c6b3',leaf:'#526a46',bark:'#5d4d3b',light:'#e1d79f',road:'#343c37',pavement:'#808574',yard:'#727e62',floor:'#9a9380',paint:'#b7b79b'};
 const geometries={box:new T.BoxGeometry(1,1,1),cylinder:new T.CylinderGeometry(1,1,1,10),sphere:new T.IcosahedronGeometry(1,1),torus:new T.TorusGeometry(1,.1,5,12),cone:new T.ConeGeometry(1,1,8),plane:new T.PlaneGeometry(1,1)},materials={},batches=new Map();
 for(const [name,color]of Object.entries(palettes)){const m=name==='light'?new T.MeshBasicMaterial({color}):new T.MeshStandardMaterial({color,roughness:name==='glass'?.45:.92,metalness:name==='steel'?.35:0});
 if(name==='paint'||name==='glass'){m.polygonOffset=true;m.polygonOffsetFactor=-2;m.polygonOffsetUnits=-2;}
 materials[name]=m;}
 for(const type of new Set(layout.buildings.map(b=>b.type))){const canvas=document.createElement('canvas');canvas.width=256;canvas.height=48;const ctx=canvas.getContext('2d');ctx.fillStyle='#ded9ba';ctx.textAlign='center';ctx.textBaseline='middle';ctx.font='bold 28px monospace';ctx.fillText(type.toUpperCase(),128,25,242);const texture=new T.CanvasTexture(canvas);texture.colorSpace=T.SRGBColorSpace;texture.magFilter=T.NearestFilter;materials['label-'+type]=new T.MeshBasicMaterial({map:texture,alphaTest:.4,polygonOffset:true,polygonOffsetFactor:-2,polygonOffsetUnits:-2});}
 const anchor=new T.Matrix4(),local=new T.Matrix4(),q=new T.Quaternion(),scale=new T.Vector3(),position=new T.Vector3();let model=null;const actualBounds=new Map();for(const geo of Object.values(geometries))geo.computeBoundingBox();
 function part(shape,mat,x,y,z,w,h,d,rx=0,ry=0,rz=0,tint=0){const batchKey=shape+'/'+mat;if(!batches.has(batchKey))batches.set(batchKey,[]);q.setFromEuler(new T.Euler(rx,ry,rz));local.compose(position.set(x,y,z),q,scale.set(w,h,d));const matrix=anchor.clone().multiply(local);batches.get(batchKey).push({matrix,tint:model?.variant??0});if(model?.id!==undefined){const key=String(model.id);if(!actualBounds.has(key))actualBounds.set(key,new T.Box3());actualBounds.get(key).union(geometries[shape].boundingBox.clone().applyMatrix4(matrix));}}
 const box=(mat,w,h,d,x,y,z,rx=0,ry=0,rz=0)=>part('box',mat,x,y,z,w,h,d,rx,ry,rz);
 const cyl=(mat,r,h,x,y,z,rx=0,rz=0)=>part('cylinder',mat,x,y,z,r,h,r,rx,0,rz);
 function begin(item){model=item;anchor.makeRotationY(item.yaw||0);anchor.setPosition(item.x,item.y||0,item.z);}
 function legs(w,d,height=.7,mat='steel'){for(const x of [-w/2+.07,w/2-.07])for(const z of [-d/2+.07,d/2-.07])box(mat,.07,height,.07,x,height/2,z);}
 function furnishing(f){begin(f);const {w,h,d,type,variant:v}=f;
  if(['desk','table','workbench'].includes(type)){const top=.76;legs(w,d,top-.06);box('wood',w,.06,d,0,top-.03,0);if(type==='desk'){box('steel',.3,.025,.2,0,top+.0125,-.08);box('steel',.05,.19,.05,0,top+.12,-.12);box('dark',.5,.32,.055,0,top+.36,-.15);box('glass',.44,.26,.01,0,top+.36,-.116);box('dark',.4,.025,.14,0,top+.0125,.2);}if(type==='workbench'){box('steel',w,.28,.05,0,1.1,-d/2+.025);for(let i=0;i<5;i++)box('dark',.045,.2,.045,-w*.35+i*w*.17,1.08,-d/2+.07);}}
  else if(type==='chair'||type==='stool'){legs(w,d,.44);box('wood',w,.06,d,0,.47,0);if(type==='chair'){for(const x of [-w/2+.035,w/2-.035])box('steel',.07,.45,.07,x,.725,-d/2+.035);box('fabric',w-.1,.26,.07,0,.81,-d/2+.035);}}
  else if(type==='bed'||type==='examBed'){legs(w,d,.22);box('steel',w,.12,d,0,.28,0);box('white',w-.04,.18,d-.05,0,.43,0);box('fabric',w-.08,.04,d*.58,0,.54,d*.14);box('white',w*.7,.12,.35,0,.58,-d*.32);if(type==='bed')box('wood',w,.6,.08,0,.65,-d/2+.04);}
  else if(type==='sofa'){legs(w,d,.12,'wood');box('fabric',w,.3,d,0,.27,0);box('fabric',w,.48,.18,0,.66,-d/2+.09);for(const x of [-w/2+.08,w/2-.08])box('wood',.16,.35,d,x,.58,0);}
  else if(type==='sink'){legs(w,d,.82);box('white',w,.08,d,0,.86,0);box('steel',.04,.15,.04,0,.975,-d*.25);box('steel',.04,.04,.16,0,1.03,-d*.1);}
  else if(type==='toilet'){part('sphere','white',0,.37,.05,.21,.2,.3);box('white',.22,.22,.3,0,.11,.08);box('white',w,.42,.2,0,.61,-d/2+.1);part('sphere','dark',0,.54,.12,.16,.015,.22);}
  else if(type==='tvstand'){legs(w,d,.34,'wood');box('wood',w,.08,d,0,.38,0);box('steel',.25,.06,.15,0,.45,0);box('dark',w*.9,.65,.07,0,.82,-.03);box('glass',w*.8,.54,.012,0,.82,.012);}
  else if(type==='shelf'){for(const x of [-w/2+.03,w/2-.03])box('steel',.06,h,d,x,h/2,0);box('wood',w-.12,h,.04,0,h/2,-d/2+.02);for(let level=0;level<4;level++){const y=.1+level*.43;box('wood',w-.12,.06,d-.04,0,y,0);for(let n=0;n<3;n++)box(n%2?'white':'rust',.14,.25,d*.6,-w*.28+n*w*.28,y+.155,0);}}
  else{const bodyHeight=type==='counter'?1:h;box(type==='counter'?'wood':'steel',w,bodyHeight,d-.08,0,bodyHeight/2,-.04);const count=type==='fridge'?2:3;for(let i=0;i<count;i++){const y=bodyHeight*(i+.5)/count;box('dark',w-.1,.018,.012,0,bodyHeight*i/count+.015,d/2-.03);box('wood',.045,.12,.02,w*.3,y,d/2-.01);}if(type==='washer'||type==='oven')part('sphere','glass',0,h*.5,d/2-.012,w*.32,h*.3,.012);if(type==='counter')box('dark',.3,.25,.1,0,1.125,-.1);}

 }
 // Every visible structural piece has one owner; wall/ceiling joins meet without duplicate faces.
 for(const b of layout.buildings){begin(b);const {w,d,h,variant:v}=b,t=.22,door=b.door,wall=['concrete','brick','plaster','concrete'][v%4];
  box(wall,w,3.12,t,0,1.56,-d/2+t/2);for(const side of [-1,1])box(wall,t,3.12,d-2*t,side*(w/2-t/2),1.56,0);for(const side of [-1,1])box(wall,(w-door)/2,3,t,side*(w/4+door/4),1.5,d/2-t/2);box(wall,w,.12,t,0,3.06,d/2-t/2);box('floor',w,.18,d,0,3.21,0);
  for(const p of b.partitions)box('plaster',p.maxX-p.minX,p.maxY-p.minY,p.maxZ-p.minZ,(p.minX+p.maxX)/2-b.x,(p.maxY+p.minY)/2,(p.minZ+p.maxZ)/2-b.z);
  for(const side of [-1,1])box('wood',.08,3,.08,side*(door/2+.04),1.5,d/2+.04);box('wood',door+.16,.08,.08,0,3.04,d/2+.04);part('plane','label-'+b.type,0,3.04,d/2+.081,door+.08,.065,1);
  const stepped=v%3===1,levels=Math.ceil((h-3.3)/2.8);let roof={x:0,z:0,w,d};
  for(let level=0;level<levels;level++){
    const bottom=3.3+level*2.8,top=Math.min(h,bottom+2.8),uw=w-(stepped?level*.45:0),ud=d-(stepped?level*.35:0),ux=stepped?level*.08:0,uz=0;
    box(wall,uw,top-bottom,ud,ux,(top+bottom)/2,uz);roof={x:ux,z:uz,w:uw,d:ud};
    const wy=bottom+(top-bottom)*.53,wh=Math.min(1.35,top-bottom-.6),spacing=1.8+(v%4)*.15;
    if(wh>.5)for(let x=-uw/2+1.2;x<uw/2-1;x+=spacing){
      box('glass',.94,wh,.06,ux+x,wy,uz+ud/2+.04);box('concrete',1.1,.08,.22,ux+x,wy-wh/2-.06,uz+ud/2+.11);
      if(v%2){for(const side of [-1,1])box('dark',.05,wh+.1,.07,ux+x+side*.51,wy,uz+ud/2+.08);}
    }
    if(wh>.5)for(const side of [-1,1])for(let z=-ud/2+1.4;z<ud/2-1;z+=2.3)box('glass',.06,wh,.85,ux+side*(uw/2+.04),wy,uz+z);
    if(v%2){for(const side of [-1,1]){box('dark',uw+.12,.07,.06,ux,top-.035,uz+side*(ud/2+.03));box('dark',.06,.07,ud,ux+side*(uw/2+.03),top-.035,uz);}}
    if(['apartment','hotel'].includes(b.type)&&level===0){
      const face=uz+ud/2,by=bottom+.28;box('steel',2,.08,.8,ux-uw*.2,by,face+.4);for(const side of [-1,1])box('steel',.05,.7,.05,ux-uw*.2+side*.95,by+.39,face+.75);box('steel',2,.05,.05,ux-uw*.2,by+.765,face+.75);
      for(const x of [-.8,.8])box('steel',.05,Math.hypot(.5,.6),.05,ux-uw*.2+x,by-.25,face+.3,Math.atan2(.6,.5));
    }
  }
  box('dark',roof.w+.12,.16,roof.d+.12,roof.x,h+.08,roof.z);const ry=h+.16;
  box('steel',1.3,.7,1,roof.x+roof.w*.22,ry+.35,-roof.d*.15);for(let i=0;i<4;i++)box('dark',1.15,.035,.04,roof.x+roof.w*.22,ry+.25+i*.1,-roof.d*.15+.51);
  if(v%4===0){cyl('steel',.65,.9,roof.x-roof.w*.23,ry+.45,-roof.d*.18);box('steel',1.45,.1,1.45,roof.x-roof.w*.23,ry+.95,-roof.d*.18);}else if(v%4===1)cyl('rust',.1,1.8,roof.x-roof.w*.3,ry+.9,0);else if(v%4===2){box('steel',roof.w*.4,.16,roof.d*.25,roof.x,ry+.08,roof.d*.15);box('glass',roof.w*.38,.03,roof.d*.23,roof.x,ry+.175,roof.d*.15);}
  if(['shop','cafe','bakery','postoffice','clinic'].includes(b.type)){
    for(const side of [-1,1])box('steel',.06,1.105,.06,side*w*.3,3.6725,d/2+.06);box('steel',w*.65,.05,.06,0,4.25,d/2+.06);
    box('rust',w*.7,.15,1,0,3.55,d/2+.5);box('dark',w*.5,.45,.08,0,4,d/2+.05);part('plane','label-'+b.type,b.type==='clinic'?w*.06:0,4,d/2+.091,w*(b.type==='clinic'?.31:.47),.38,1);
    if(b.type==='clinic'){box('light',.14,.32,.03,-w*.18,4,d/2+.105);for(const side of [-1,1])box('light',.12,.1,.03,-w*.18+side*.13,4,d/2+.105);}
  }

 }
 for(const f of layout.furniture)furnishing(f);
 const posts=new Set();
 for(const p of layout.props){begin(p);const {type,w,h,d,variant:v}=p;
  if(type==='bench'||type==='busstop'){for(let i=0;i<4;i++){box('wood',type==='bench'?w-.1:w-.3,.055,.12,0,.475,-.21+i*.14);box('wood',w-.15,.09,.06,0,.65+i*.09,-.24);}for(const x of [-w*.38,w*.38]){box('steel',.09,.45,.48,x,.225,0);box('steel',.09,.45,.08,x,.725,-.24);}if(type==='busstop'){for(const x of [-w/2+.06,w/2-.06])box('steel',.12,h-.1,.12,x,(h-.1)/2,-d/2+.06);box('steel',w,.1,d,0,h-.05,0);}}
  else if(type==='fence'){const length=p.length,segments=Math.ceil(length/2.4);for(let i=0;i<=segments;i++){const x=-length/2+i*length/segments,point=new T.Vector3(x,0,0).applyMatrix4(anchor),key=`${point.x.toFixed(3)}/${point.z.toFixed(3)}`;if(!posts.has(key)){posts.add(key);box('steel',.1,h,.1,x,h/2,0);}}
   for(const y of [.2,h-.2])box('steel',length-.1,.06,.06,0,y,0);for(let x=-length/2+.2;x<length/2-.15;x+=.24)box('steel',.025,h-.4,.025,x,h/2,0);
  }
  else if(type==='car'||type==='van'||type==='hatchback'){const color=v%3?'steel':'rust';box(color,w-.4,.5,d-.12,0,.65,0);const cabin=type==='van'?d*(.6+(v%3)*.02):type==='hatchback'?d*(.49+(v%3)*.02):d*(.43+(v%3)*.02);box('glass',w-.42,h-.98,cabin,0,(h+.82)/2,-.08);box(color,w-.32,.08,cabin+.1,0,h-.04,-.08);
   for(const z of [-d*.32,d*.32]){cyl('steel',.05,w-.36,0,.32,z,0,Math.PI/2);for(const side of [-1,1]){cyl('rubber',.32,.18,side*(w/2-.09),.32,z,0,Math.PI/2);box('steel',.06,.03,.06,side*w*.3,.385,z);}}for(const x of [-w*.32,w*.32])box('light',.3,.13,.06,x,.72,-d/2+.03);box('dark',w-.2,.13,.09,0,.4,-d/2+.045);box('dark',w-.2,.13,.09,0,.4,d/2-.045);
  }
  else if(type==='lamp'){cyl('steel',.09,h-.2,0,(h-.2)/2,0);box('steel',.24,.12,.24,0,h-.14,0);box('light',.25,.16,.25,0,h-.08,0);}
  else if(['tree','birch','pine'].includes(type)){cyl(type==='birch'?'white':'bark',.11,h*.65,0,h*.325,0);if(type==='pine'){for(let n=0;n<3;n++)part('cone','leaf',0,h*.45+n*h*.16,0,w*.5-n*.15,h*.45,d*.5-n*.15);}else{part('sphere','leaf',0,h*.72,0,w*.5,h*.28,d*.5);}}
  else if(type==='bush'){cyl('bark',.04,h*.25,0,h*.125,0);for(const x of [-w*.2,w*.2])part('sphere','leaf',x,h*.55,0,w*.3,h*.4,d*.5);}
  else if(type==='planter'){box('concrete',w,.35,d,0,.175,0);box('yard',w-.1,.03,d-.1,0,.365,0);for(const x of [-w*.3,0,w*.3]){cyl('bark',.025,.15,x,.45,0);part('sphere','leaf',x,.535,0,.18,.115,.16);}}
  else if(type==='parkingmeter'){cyl('steel',.045,.95,0,.475,0);box('steel',.25,.35,.22,0,1.125,0);box('glass',.16,.12,.01,0,1.16,.115);}
  else if(type==='mailbox'){cyl('steel',.065,.75,0,.375,0);box('steel',w,.55,d,0,1.025,0);box('dark',w*.75,.04,.012,0,1.12,d/2-.006);}
  else if(type==='bike'){
    for(const x of [-.5,.5])part('torus','rubber',x,.286,0,.26,.26,.26);
    const rod=(a,b,r=.025)=>{const delta=new T.Vector3(...b).sub(new T.Vector3(...a)),euler=new T.Euler().setFromQuaternion(new T.Quaternion().setFromUnitVectors(new T.Vector3(0,1,0),delta.clone().normalize()));part('cylinder','steel',(a[0]+b[0])/2,(a[1]+b[1])/2,(a[2]+b[2])/2,r,delta.length(),r,euler.x,euler.y,euler.z);};
    for(const [a,b]of [[[0,.32,0],[-.5,.286,0]],[[0,.32,0],[-.15,.7,0]],[[-.5,.286,0],[-.15,.7,0]],[[-.15,.7,0],[.35,.7,0]],[[0,.32,0],[.35,.7,0]],[[.35,.7,0],[.5,.286,0]],[[.35,.7,0],[.4,.94,0]],[[.4,.94,-.2],[.4,.94,.2]]])rod(a,b);box('dark',.28,.06,.18,-.15,.76,0);
  }
  else if(type==='barrel'||type==='bin'){cyl(type==='barrel'?'rust':'steel',w*.45,h-.03,0,(h-.03)/2,0);for(const y of [.04,h*.28,h*.72,h-.04])cyl('dark',w*.48,.035,0,y,0);cyl('steel',w*.46,.03,0,h-.015,0);}
  else if(type==='crate'||type==='pallet'){const height=type==='pallet'?.2:h;for(let i=0;i<5;i++)box('wood',w/5-.012,.05,d,-w/2+(i+.5)*w/5,height-.025,0);for(const x of [-w/2+.06,w/2-.06])box('wood',.12,height-.05,d,x,(height-.05)/2,0);if(type==='crate')for(let i=0;i<5;i++)for(const z of [-d/2+.025,d/2-.025])box('wood',w/5-.012,height-.05,.05,-w/2+(i+.5)*w/5,(height-.05)/2,z);}
  else if(type==='dumpster'){box('steel',w-.08,h-.2,d-.08,0,(h-.2)/2+.1,0);box('dark',w,.1,d,0,h-.05,0);for(const x of [-w*.35,w*.35])for(const z of [-d*.3,d*.3])cyl('rubber',.08,.2,x,.1,z);}
  else if(type==='barrier'){box('concrete',w,.22,d,0,.11,0);box('concrete',w-.12,h-.22,d*.55,0,.22+(h-.22)/2,0);for(let i=0;i<5;i++)box('dark',.16,.15,.02,-w*.35+i*w*.175,h-.15,-d*.28);}
  else if(type==='hydrant'){cyl('rust',.14,.7,0,.35,0);cyl('rust',.18,.1,0,.75,0);box('steel',.5,.14,.15,0,.5,0);}
  else if(type==='bollard')cyl('steel',.12,h,0,h/2,0);
  else if(type==='sign'){cyl('steel',.035,h-.1,0,(h-.1)/2,0);box('dark',w,.4,.08,0,h-.2,0);}
  else{box('steel',w,h,d,0,h/2,0);for(let n=0;n<6;n++)box('dark',w*.7,.03,.015,0,h*.2+n*h*.1,-d/2+.008);}
 }
 // One exact terrain surface per point. Interior finishes replace, rather than overlay, yard tiles.
 anchor.identity();model=null;
 function surface(x,z,w,d,type){part('plane',type,x,0,z,w,d,1,-Math.PI/2);}
 for(const s of layout.surfaces){const x=(s.minX+s.maxX)/2,z=(s.minZ+s.maxZ)/2;if(s.type!=='yard'){surface(x,z,s.maxX-s.minX,s.maxZ-s.minZ,s.type);continue;}const b=layout.buildings.find(b=>x>b.cx-8&&x<b.cx+8&&z>b.cz-8&&z<b.cz+8);if(!b){surface(x,z,s.maxX-s.minX,s.maxZ-s.minZ,'yard');continue;}
  const xs=[s.minX,b.x-b.w/2,b.x+b.w/2,s.maxX].sort((a,b)=>a-b),zs=[s.minZ,b.z-b.d/2,b.z+b.d/2,s.maxZ].sort((a,b)=>a-b);for(let i=0;i<3;i++)for(let j=0;j<3;j++){const px=(xs[i]+xs[i+1])/2,pz=(zs[j]+zs[j+1])/2,inside=px>b.x-b.w/2&&px<b.x+b.w/2&&pz>b.z-b.d/2&&pz<b.z+b.d/2;surface(px,pz,xs[i+1]-xs[i],zs[j+1]-zs[j],inside?'floor':pz>b.z+b.d/2?'pavement':'yard');}
 }
 for(const m of layout.markings)part('plane','paint',m.x,.002,m.z,m.w,m.d,1,-Math.PI/2);
 for(const bay of layout.parking){for(const side of [-1,1])part('plane','paint',bay.x+side*bay.w/2,.002,bay.z,.055,bay.d,1,-Math.PI/2);part('plane','paint',bay.x,.002,bay.z-bay.d/2,bay.w,.055,1,-Math.PI/2);}
 // Instanced materials support geometry reuse without thousands of draw calls.
 for(const [key,records]of batches){const [shape,mat]=key.split('/'),mesh=new T.InstancedMesh(geometries[shape],materials[mat],records.length);records.forEach((record,i)=>{mesh.setMatrixAt(i,record.matrix);const v=record.tint%16,shade=.9+(v%4)*.035;mesh.setColorAt(i,new T.Color(shade,shade-(v%3)*.015,shade-(v%5)*.012));});mesh.instanceMatrix.needsUpdate=true;mesh.castShadow=shape!=='plane'&&mat!=='glass'&&mat!=='light';mesh.receiveShadow=mat!=='light';mesh.computeBoundingBox();mesh.computeBoundingSphere();mesh.name='city/'+key;scene.add(mesh);}
 scene.userData.city={instances:[...batches.values()].reduce((n,v)=>n+v.length,0),drawBatches:batches.size,assetCount:layout.props.length+layout.furniture.length+layout.buildings.length,bounds:Object.fromEntries([...actualBounds].map(([key,b])=>[key,{min:b.min.toArray(),max:b.max.toArray()}]))};
}
