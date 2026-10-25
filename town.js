export const SPAWN={x:0,y:0,z:85};
export const EXIT={x:0,z:-95};
export const BUILDINGS=['apartment','office','shop','garage','clinic'];
export const PROP_TYPES=['bench','bin','dumpster','lamp','car','barrier','fence','barrel','crate','hydrant','powerbox','busstop','pallet','sign','bollard'];
export function generateTown(seed=1){let state=seed>>>0;const random=()=>{state=(state*1664525+1013904223)>>>0;return state/4294967296;};const buildings=[],props=[],colliders=[];
const box=(x,y,z,w,h,d)=>colliders.push({minX:x-w/2,maxX:x+w/2,minY:y,maxY:y+h,minZ:z-d/2,maxZ:z+d/2});
for(const [x,z,w,d]of [[0,-100,200,1],[0,100,200,1],[-100,0,1,200],[100,0,1,200]])box(x,0,z,w,5,d);
for(const x of [-82,-30,30,82])for(const z of [-82,-30,30,82]){
 const type=BUILDINGS[buildings.length<5?buildings.length:Math.floor(random()*5)],w=type==='garage'?16:12,d=type==='shop'?10:14,h=type==='apartment'?14:type==='office'?11:type==='garage'?5:7;
 const b={type,x,z:z-3,w,d,h};buildings.push(b);box(x,0,b.z-d/2,w,h,.4);box(x-w/2,0,b.z,.4,h,d);box(x+w/2,0,b.z,.4,h,d);box(x,3,b.z+d/2,w,h-3,.4);box(x-w/4-1,0,b.z+d/2,w/2-2,3,.4);box(x+w/4+1,0,b.z+d/2,w/2-2,3,.4);box(x,h,b.z,w+.4,.3,d+.4);
 for(let n=0;n<3;n++){const type=PROP_TYPES[(buildings.length*3+n+Math.floor(random()*5))%PROP_TYPES.length],px=x+(n-1)*6,pz=z+12;props.push({type,x:px,z:pz});}
}
// Guaranteed representation of every prop type, beside the main avenue, never on its route.
PROP_TYPES.forEach((type,i)=>props.push({type,x:i%2?-10:10,z:80-Math.floor(i/2)*22}));
const sizes={bench:[2,.9,.6],bin:[.6,.9,.6],dumpster:[2,1.5,1.2],lamp:[.3,5,.3],car:[2,1.5,4.5],barrier:[2.5,1.1,.7],fence:[3,2,.15],barrel:[.8,1.15,.8],crate:[1.1,1.1,1.1],hydrant:[.5,.8,.5],powerbox:[1,1.8,.65],busstop:[3,2.7,1.5],pallet:[1.4,.2,1],sign:[.2,2.6,.2],bollard:[.25,.9,.25]};
for(const p of props){[p.w,p.h,p.d]=sizes[p.type];box(p.x,0,p.z,p.w,p.h,p.d);}
return {seed:seed>>>0,buildings,props,colliders,spawn:{...SPAWN},exit:{...EXIT},enemySpawns:[[0,65],[0,35],[0,0],[0,-35],[0,-65]]};}
