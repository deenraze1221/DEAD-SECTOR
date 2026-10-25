export const SPAWN={x:0,y:0,z:85};export const EXIT={x:0,z:-95};
export const HIVES=[{x:-96,z:-96},{x:96,z:-96},{x:-96,z:96},{x:96,z:96}];
export const BUILDINGS=['apartment','office','shop','garage','clinic'];export const PROP_TYPES=['bench','bin','dumpster','lamp','car','barrier','fence','barrel','crate','hydrant','powerbox','busstop','pallet','sign','bollard'];
export function generateTown(seed=1){let state=seed>>>0;const random=()=>{state=(state*1664525+1013904223)>>>0;return state/4294967296;};const buildings=[],props=[],colliders=[],supplies=[],districts=[],walls=[];
const box=(x,y,z,w,h,d)=>colliders.push({minX:x-w/2,maxX:x+w/2,minY:y,maxY:y+h,minZ:z-d/2,maxZ:z+d/2});
for(const [x,z,w,d]of [[0,-100,200,1],[0,100,200,1],[-100,0,1,200],[100,0,1,200]]){box(x,0,z,w,5.8,d);walls.push({x,z,w,h:5.8,d});}
for(const x of [-84,-60,-36,-12,12,36,60,84])for(const z of [-84,-60,-36,-12,12,36,60,84]){
 const district=x<0?(z<0?'industrial':'residential'):(z<0?'medical':'market');const choices={industrial:['garage','office'],residential:['apartment','shop'],medical:['clinic','office'],market:['shop','apartment']}[district];const type=buildings.length<5?BUILDINGS[buildings.length]:choices[Math.floor(random()*choices.length)],w=type==='garage'?12:10,d=type==='shop'?9:11,h=type==='apartment'?14:type==='office'?11:type==='garage'?5:7;
 const b={type,x,z:z-1+(random()-.5)*1.5,w,d,h,district};buildings.push(b);box(x,0,b.z-d/2,w,h,.4);box(x-w/2,0,b.z,.4,h,d);box(x+w/2,0,b.z,.4,h,d);box(x,3,b.z+d/2,w,h-3,.4);box(x-w/4-1,0,b.z+d/2,w/2-2,3,.4);box(x+w/4+1,0,b.z+d/2,w/2-2,3,.4);box(x,h,b.z,w+.4,.3,d+.4);
 for(let n=0;n<3;n++)props.push({type:PROP_TYPES[(buildings.length*3+n+Math.floor(random()*5))%15],x:x+(n===0?-8:8),z:z-5+n*4});
 supplies.push({type:buildings.length%2?'health':'ammo',x:b.x,z:b.z+2});if(buildings.length%4===0){for(const dx of [-7.5,-4.5,4.5,7.5])props.push({type:'fence',x:x+dx,z:z+8});}
 districts.push({name:district,x,z});
}
const sizes={bench:[2,.9,.6],bin:[.6,.9,.6],dumpster:[2,1.5,1.2],lamp:[.3,5,.3],car:[2,1.5,4.5],barrier:[2.5,1.1,.7],fence:[3,2,.15],barrel:[.8,1.15,.8],crate:[1.1,1.1,1.1],hydrant:[.5,.8,.5],powerbox:[1,1.8,.65],busstop:[3,2.7,1.5],pallet:[1.4,.2,1],sign:[.2,2.6,.2],bollard:[.25,.9,.25]};
for(const p of props){[p.w,p.h,p.d]=sizes[p.type];box(p.x,0,p.z,p.w,p.h,p.d);}
return {seed:seed>>>0,buildings,props,colliders,walls,hives:HIVES,spawn:{...SPAWN},exit:{...EXIT},supplies:[{type:'ammo',x:3,z:82},{type:'health',x:-3,z:80},...supplies],districts,enemySpawns:[]};}
