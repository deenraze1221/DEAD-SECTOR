import * as T from 'three';
// All meshes are original procedural models; no imported environment assets.
export function createCity(scene,layout){
const mats={concrete:new T.MeshStandardMaterial({color:'#777b69',roughness:1}),dark:new T.MeshStandardMaterial({color:'#29352f',roughness:.9}),wood:new T.MeshStandardMaterial({color:'#756047',roughness:1}),steel:new T.MeshStandardMaterial({color:'#556259',metalness:.4,roughness:.75}),rust:new T.MeshStandardMaterial({color:'#86583d',roughness:.9}),light:new T.MeshBasicMaterial({color:'#d5dea5'}),glass:new T.MeshStandardMaterial({color:'#384b49',metalness:.3,roughness:.4})};
const canvas=document.createElement('canvas');canvas.width=canvas.height=64;const context=canvas.getContext('2d'),image=context.createImageData(64,64);for(let i=0;i<image.data.length;i+=4){const value=205+Math.sin(i*57.13)*14;image.data[i]=image.data[i+1]=image.data[i+2]=value;image.data[i+3]=255;}context.putImageData(image,0,0);const detail=new T.CanvasTexture(canvas);detail.colorSpace=T.SRGBColorSpace;detail.wrapS=detail.wrapT=T.RepeatWrapping;detail.magFilter=T.NearestFilter;for(const [key,material]of Object.entries(mats))if(!['light','glass'].includes(key))material.map=detail;
function mesh(g,geo,mat,x,y,z,rx=0,rz=0){const pos=geo.attributes.position,n=geo.attributes.normal,uv=geo.attributes.uv;if(uv&&n)for(let i=0;i<pos.count;i++){const nx=Math.abs(n.getX(i)),ny=Math.abs(n.getY(i)),nz=Math.abs(n.getZ(i));uv.setXY(i,nx>ny&&nx>nz?pos.getZ(i):pos.getX(i),ny>nx&&ny>nz?pos.getZ(i):pos.getY(i));}const m=new T.Mesh(geo,mats[mat]);m.position.set(x,y,z);m.rotation.set(rx,0,rz);m.castShadow=m.receiveShadow=true;g.add(m);return m;}
const box=(g,w,h,d,mat,x,y,z)=>mesh(g,new T.BoxGeometry(w,h,d),mat,x,y,z);
const cyl=(g,r,h,mat,x,y,z)=>mesh(g,new T.CylinderGeometry(r,r,h,10),mat,x,y,z);
for(const b of layout.buildings){const g=new T.Group();g.name='building-'+b.type;g.position.set(b.x,0,b.z);scene.add(g);const {w,d,h}=b;
box(g,w,h,.4,'concrete',0,h/2,-d/2);for(const x of [-w/2,w/2])box(g,.4,h,d,'concrete',x,h/2,0);box(g,w,h-3,.4,'concrete',0,(h+3)/2,d/2);for(const x of [-w/4-1,w/4+1])box(g,w/2-2,3,.4,'concrete',x,1.5,d/2);box(g,w+.4,.3,d+.4,'dark',0,h+.15,0);
for(let y=4.2;y<h-1;y+=2.8)for(let x=-w/2+1.6;x<w/2-1;x+=2.7){box(g,1.1,1.5,.06,'glass',x,y,d/2+.23);box(g,1.3,.08,.3,'dark',x,y-.8,d/2+.3);}
box(g,4,.25,1.8,'rust',0,3.15,d/2+.6);box(g,3.5,.5,.08,b.type==='clinic'?'light':'steel',0,3.65,d/2+.26);
if(b.type==='clinic'){box(g,.3,1.3,.08,'light',w/2-1,2,d/2+.3);box(g,1.1,.3,.08,'light',w/2-1,2,d/2+.31);}
if(b.type==='garage')for(let i=0;i<7;i++)box(g,3,.04,.1,'steel',0,3.9+i*.12,d/2+.25);
if(b.type==='shop')box(g,w,.2,2,'wood',0,3.2,d/2+.5);
if(b.type==='apartment')for(let y=5;y<h;y+=2.8){box(g,2,.12,1,'steel',-3,y,d/2+.5);box(g,2,.6,.08,'steel',-3,y+.3,d/2+1);}
box(g,1.8,1,1.5,'steel',w*.2,h+.8,0);cyl(g,.15,2,'rust',-w*.3,h+1,-d*.25);
}
for(const p of layout.props){const g=new T.Group();g.name='own-'+p.type;g.position.set(p.x,0,p.z);scene.add(g);
const {type,w,h,d}=p;
if(type==='bench'||type==='busstop'){for(let i=0;i<4;i++){box(g,w-.2,.08,.12,'wood',0,.46,-.22+i*.14);box(g,w-.2,.1,.06,'wood',0,.65+i*.13,-.24);}for(const x of [-w*.4,w*.4])box(g,.1,.5,.5,'steel',x,.25,0);if(type==='busstop'){for(const x of [-w/2+.1,w/2-.1])box(g,.1,h,.1,'steel',x,h/2,-d/2+.1);box(g,w,.15,d,'steel',0,h,0);}}
else if(type==='barrel'||type==='bin'){cyl(g,w*.45,h-.05,type==='barrel'?'rust':'steel',0,h/2,0);for(const y of [.06,h*.3,h*.7,h-.03]){const ring=mesh(g,new T.TorusGeometry(w*.46,.025,4,12),'dark',0,y,0);ring.rotation.x=Math.PI/2;}cyl(g,w*.45,.03,'dark',0,h,0);}
else if(type==='lamp'){cyl(g,.09,4.8,'steel',0,2.4,0);box(g,.8,.1,.12,'steel',.3,4.8,0);box(g,.5,.1,.4,'light',.6,4.73,0);const l=new T.PointLight('#d5dca3',14,12,2);l.position.set(p.x+.6,4.6,p.z);scene.add(l);}
else if(type==='car'){box(g,1.9,.65,4.3,'steel',0,.65,0);box(g,1.65,.65,2.1,'glass',0,1.22,-.15);box(g,1.8,.1,2.2,'steel',0,1.55,-.15);for(const x of [-.95,.95])for(const z of [-1.45,1.45])mesh(g,new T.CylinderGeometry(.34,.34,.18,10),'dark',x,.34,z,0,Math.PI/2);for(const x of [-.65,.65])box(g,.35,.16,.08,'light',x,.8,-2.16);box(g,1.6,.12,.12,'dark',0,.4,-2.22);}
else if(type==='fence'){for(const x of [-1.45,1.45])box(g,.08,2,.08,'steel',x,1,0);for(let i=0;i<12;i++)box(g,.025,1.8,.025,'steel',-1.35+i*.245,1,0);for(const y of [.2,1.8])box(g,3,.05,.07,'rust',0,y,0);}
else if(type==='crate'||type==='pallet'){for(let i=0;i<5;i++){box(g,w/5-.02,.09,d,'wood',-w/2+(i+.5)*w/5,h-.05,0);if(type==='crate')for(const z of [-d/2+.03,d/2-.03])box(g,w/5-.02,h,.06,'wood',-w/2+(i+.5)*w/5,h/2,z);}for(const x of [-w/2+.08,w/2-.08])box(g,.12,h,d,'wood',x,h/2,0);}
else if(type==='barrier'){box(g,w,.25,d,'concrete',0,.125,0);box(g,w-.15,.8,d*.5,'concrete',0,.65,0);for(let i=0;i<5;i++){const stripe=box(g,.18,.25,.015,'dark',-w*.4+i*w*.2,.8,-d*.26);stripe.rotation.z=.5;}}
else if(type==='dumpster'){box(g,w,h*.85,d,'steel',0,h*.45,0);box(g,w+.08,.1,d+.08,'dark',0,h,0);for(const x of [-w*.35,w*.35])for(const z of [-d*.35,d*.35])cyl(g,.08,.12,'dark',x,.06,z);box(g,.5,.08,.08,'rust',0,h*.8,-d/2-.05);}
else if(type==='hydrant'){cyl(g,.15,.7,'rust',0,.35,0);cyl(g,.2,.1,'rust',0,.75,0);box(g,.5,.14,.15,'steel',0,.5,0);}
else if(type==='bollard')cyl(g,.12,h,'rust',0,h/2,0);
else if(type==='sign'){cyl(g,.04,h,'steel',0,h/2,0);box(g,.8,.5,.06,'dark',0,h-.3,0);box(g,.6,.08,.07,'light',0,h-.3,-.02);}
else {box(g,w,h,d,'steel',0,h/2,0);for(let i=0;i<7;i++)box(g,w*.75,.04,.05,'dark',0,.35+i*.15,-d/2-.02);box(g,.1,.1,.06,'light',w*.3,h*.8,-d/2-.02);}
}
}
