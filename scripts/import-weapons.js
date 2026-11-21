// Builds assets/models/weapons.json from the Blockbench GLB exports in assets/weapons.
// Run: node scripts/import-weapons.js
import {readFile,writeFile} from 'node:fs/promises';
import {inflateSync} from 'node:zlib';
import * as THREE from 'three';
import {WEAPONS} from '../weapon-config.js';

// id -> model folder. Handguns are drawn 1.8x so the fixed-size hands still wrap the grip.
const MODELS={
  Pistol:['G17_Style',1.8],USP:['USP_Suppressed',1.8],Ironhawk:['Ironhawk50',1.8],Blackthorn:['Blackthorn357',1.8],
  Kureyka:['Kureyka12',1],Taiga:['Taiga12',1],Angara:['Angara12',1],Sayan:['Sayan12',1],
  Kemchug:['KemchugK3',1],Sisim:['SisimSv9',1],Bereg:['BeregA24',1],Biryusa:['BiryusaBk10',1],
  Chulym:['ChulymC5',1],Vetluga:['Vetluga_B3',1],Kazyr:['KazyrBR62',1],Katanga:['KatangaMg1',1],
};
const HANDGUNS=new Set(['Pistol','USP','Ironhawk','Blackthorn']);

function parseGlb(bytes){
  const dv=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),jsonLength=dv.getUint32(12,true);
  const json=JSON.parse(Buffer.from(bytes.subarray(20,20+jsonLength)).toString('utf8'));
  const bin=bytes.subarray(20+jsonLength+8);
  return {json,bin};
}
function accessor({json,bin},index){
  const a=json.accessors[index],view=json.bufferViews[a.bufferView],width={SCALAR:1,VEC2:2,VEC3:3,VEC4:4}[a.type];
  const offset=bin.byteOffset+(view.byteOffset||0)+(a.byteOffset||0),count=a.count*width;
  const Type={5126:Float32Array,5123:Uint16Array,5125:Uint32Array,5121:Uint8Array}[a.componentType];
  if(view.byteStride&&view.byteStride!==width*Type.BYTES_PER_ELEMENT)throw Error('Interleaved accessors are not supported');
  return new Type(bin.buffer.slice(offset,offset+count*Type.BYTES_PER_ELEMENT));
}
// Minimal PNG reader: 8-bit RGB/RGBA/palette, non-interlaced (what Blockbench exports).
function decodePng(buffer){
  let p=8,width,height,depth,type;const idat=[];let palette=null,alpha=null;
  while(p<buffer.length){
    const length=buffer.readUInt32BE(p),name=buffer.toString('ascii',p+4,p+8),data=buffer.subarray(p+8,p+8+length);p+=12+length;
    if(name==='IHDR'){width=data.readUInt32BE(0);height=data.readUInt32BE(4);depth=data[8];type=data[9];if(depth!==8||data[12])throw Error('Unsupported PNG');}
    else if(name==='PLTE')palette=data;else if(name==='tRNS')alpha=data;else if(name==='IDAT')idat.push(data);
  }
  const channels={0:1,2:3,3:1,4:2,6:4}[type],stride=width*channels,raw=inflateSync(Buffer.concat(idat)),out=Buffer.alloc(stride*height);
  for(let y=0;y<height;y++){
    const filter=raw[y*(stride+1)],row=raw.subarray(y*(stride+1)+1,(y+1)*(stride+1));
    for(let x=0;x<stride;x++){
      const a=x>=channels?out[y*stride+x-channels]:0,b=y?out[(y-1)*stride+x]:0,c=y&&x>=channels?out[(y-1)*stride+x-channels]:0;
      let v=row[x];
      if(filter===1)v+=a;else if(filter===2)v+=b;else if(filter===3)v+=(a+b)>>1;
      else if(filter===4){const pa=Math.abs(b-c),pb=Math.abs(a-c),pc=Math.abs(a+b-2*c);v+=pa<=pb&&pa<=pc?a:pb<=pc?b:c;}
      out[y*stride+x]=v&255;
    }
  }
  const pixel=(x,y)=>{
    const i=y*stride+x*channels;
    if(type===3){const k=out[i];return [palette[k*3],palette[k*3+1],palette[k*3+2]];}
    if(type===0||type===4)return [out[i],out[i],out[i]];
    return [out[i],out[i+1],out[i+2]];
  };
  return {width,height,pixel};
}
const linear=v=>{v/=255;return v<=.04045?v/12.92:Math.pow((v+.055)/1.055,2.4);};

// One flat-shaded, vertex-coloured triangle soup per node. Colour is the albedo texel under each triangle centre.
function readNodes(glb,texture){
  const nodes=new Map();
  for(const node of glb.json.nodes){
    if(node.mesh===undefined)continue;
    const positions=[],normals=[],colors=[];
    for(const primitive of glb.json.meshes[node.mesh].primitives){
      const p=accessor(glb,primitive.attributes.POSITION),uv=primitive.attributes.TEXCOORD_0!==undefined?accessor(glb,primitive.attributes.TEXCOORD_0):null;
      const index=primitive.indices!==undefined?accessor(glb,primitive.indices):Array.from({length:p.length/3},(_,i)=>i);
      for(let i=0;i<index.length;i+=3){
        const [a,b,c]=[index[i],index[i+1],index[i+2]];
        const pa=new THREE.Vector3().fromArray(p,a*3),pb=new THREE.Vector3().fromArray(p,b*3),pc=new THREE.Vector3().fromArray(p,c*3);
        if(node.translation)for(const q of [pa,pb,pc])q.add(new THREE.Vector3().fromArray(node.translation));
        const n=pb.clone().sub(pa).cross(pc.clone().sub(pa)).normalize();
        let rgb=[.5,.5,.5];
        if(uv&&texture){
          const u=(uv[a*2]+uv[b*2]+uv[c*2])/3,v=(uv[a*2+1]+uv[b*2+1]+uv[c*2+1])/3;
          const x=Math.min(texture.width-1,Math.max(0,Math.floor(u*texture.width))),y=Math.min(texture.height-1,Math.max(0,Math.floor(v*texture.height)));
          rgb=texture.pixel(x,y).map(linear);
        }
        for(const q of [pa,pb,pc]){positions.push(q.x,q.y,q.z);normals.push(n.x,n.y,n.z);colors.push(...rgb);}
      }
    }
    nodes.set(node.name,{positions,normals,colors});
  }
  return nodes;
}
function names(glb){return new Set(glb.json.nodes.map(n=>n.name));}
function boundsOf(parts){
  const box=new THREE.Box3();
  for(const part of parts)for(let i=0;i<part.positions.length;i+=3)box.expandByPoint(new THREE.Vector3(part.positions[i],part.positions[i+1],part.positions[i+2]));
  return box;
}
function merge(parts){
  const out={positions:[],normals:[],colors:[]};
  for(const part of parts)for(const key of Object.keys(out))for(const v of part[key])out[key].push(v);
  return out;
}

const records=[];
for(const {id,label} of WEAPONS){
  const [folder,factor]=MODELS[id]||(()=>{throw Error('No model for '+id);})(),dir=`assets/weapons/${folder}/`;
  const glb=parseGlb(await readFile(dir+folder+'.glb'));
  const colorTexture=glb.json.materials[0].pbrMetallicRoughness.baseColorTexture.index,image=glb.json.images[glb.json.textures[colorTexture].source],view=glb.json.bufferViews[image.bufferView];
  const texture=decodePng(Buffer.from(glb.bin.subarray(view.byteOffset||0,(view.byteOffset||0)+view.byteLength)));
  const all=readNodes(glb,texture);
  let magazineNames=new Set();
  try{magazineNames=names(parseGlb(await readFile(dir+folder+'_Magazine.glb')));}catch{}
  const bodyParts=[...all].filter(([n])=>!magazineNames.has(n)).map(([,v])=>v),magazineParts=[...all].filter(([n])=>magazineNames.has(n)).map(([,v])=>v);
  const box=boundsOf(bodyParts),center=box.getCenter(new THREE.Vector3()),scale=factor;
  // Blockbench export: forward is -X, up is +Y. The game wants forward -Z, so rotate -90 degrees about Y.
  const place=part=>{
    const g=new THREE.BufferGeometry();
    g.setAttribute('position',new THREE.Float32BufferAttribute(part.positions,3));g.setAttribute('normal',new THREE.Float32BufferAttribute(part.normals,3));g.setAttribute('color',new THREE.Float32BufferAttribute(part.colors,3));
    g.translate(-center.x,-center.y,-center.z);g.scale(scale,scale,scale);g.rotateY(-Math.PI/2);
    g.addGroup(0,part.positions.length/3,0);
    for(const attribute of Object.values(g.attributes))for(let i=0;i<attribute.array.length;i++)attribute.array[i]=Math.round(attribute.array[i]*1e5)/1e5;
    return {geometry:g.toJSON(),materials:[{name:'Albedo',color:0xffffff,vertexColors:true}],positions:g.attributes.position.array};
  };
  const body=place(merge(bodyParts)),magazine=magazineParts.length?place(merge(magazineParts)):null;
  const length=(box.max.x-box.min.x)*scale,positions=body.positions;
  let lo=Infinity,hi=-Infinity;
  for(let i=0;i<positions.length;i+=3)if(positions[i+2]<-length*.49){lo=Math.min(lo,positions[i+1]);hi=Math.max(hi,positions[i+1]);}
  // Hand anchors from the Trigger part: trigger hand sits on the grip, support hand on the forend (or the grip for handguns).
  const trigger=boundsOf([...readNodes(parseGlb(await readFile(dir+folder+'_Trigger.glb')),null).values()]).getCenter(new THREE.Vector3());
  const tz=(trigger.x-center.x)*scale,ty=(trigger.y-center.y)*scale;
  const grips=HANDGUNS.has(id)
    ?{trigger:[.04,ty-.035,tz+.03],support:[-.04,ty-.055,tz-.01]}
    :(()=>{
      const sz=tz+(-length/2-tz)*.5;let low=Infinity,high=-Infinity;
      for(let i=0;i<positions.length;i+=3)if(Math.abs(positions[i+2]-sz)<.05){low=Math.min(low,positions[i+1]);high=Math.max(high,positions[i+1]);}
      return {trigger:[.04,ty-.02,tz+.03],support:[-.045,(low+high)/2-.01,sz]};
    })();
  const round=a=>a.map(v=>Math.round(v*1e3)/1e3);
  delete body.positions;if(magazine)delete magazine.positions;
  records.push({id,label,length:Math.round(length*1e3)/1e3,grips:{trigger:round(grips.trigger),support:round(grips.support)},muzzleY:Math.round((lo+hi)/2*1e5)/1e5,source:folder+'.glb',body,magazine});
}
await writeFile('assets/models/weapons.json',JSON.stringify(records));
console.log(`Imported ${records.length} weapons from assets/weapons.`);
