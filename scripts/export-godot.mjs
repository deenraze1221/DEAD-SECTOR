// Reusable bridge: preserve the browser city's instancing and original weapon geometry.
import {createServer} from 'vite';
import {chromium} from 'playwright';
import {writeFile,mkdir} from 'node:fs/promises';
const vite=await createServer({server:{host:'127.0.0.1',port:0}});await vite.listen();
const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/usr/bin/chromium',headless:true,args:['--no-sandbox','--enable-unsafe-swiftshader']});
try{
const page=await browser.newPage();await page.route('**/export-fixture',r=>r.fulfill({contentType:'text/html',body:'<body></body>'}));await page.goto(vite.resolvedUrls.local[0]+'export-fixture');
const data=await page.evaluate(async()=>{
 const T=await import('/node_modules/.vite/deps/three.js');const {createMap}=await import('/map.js');const {PeerAvatar}=await import('/peer-avatar.js');
 const scene=new T.Scene(),layout=createMap(scene,42),geometries=[],materials=[],gids=new Map(),mids=new Map(),textures=[];
 function material(m){if(mids.has(m.uuid))return mids.get(m.uuid);const id=materials.length;mids.set(m.uuid,id);let texture=-1;if(m.map?.image instanceof HTMLCanvasElement){texture=textures.length;textures.push(m.map.image.toDataURL('image/png'));}materials.push({color:'#'+m.color.getHexString(),emissive:m.emissive?'#'+m.emissive.getHexString():'#000000',energy:m.emissiveIntensity||0,roughness:m.roughness??1,metalness:m.metalness||0,unlit:m.isMeshBasicMaterial||false,texture,surface:m.customProgramCacheKey?.().startsWith('city-surfaces')?Number(m.customProgramCacheKey().split('-').at(-1)):0});return id;}
 function geometry(g){if(gids.has(g.uuid))return gids.get(g.uuid);const id=geometries.length;gids.set(g.uuid,id);const a=g.attributes;geometries.push({position:Array.from(a.position.array),normal:a.normal?Array.from(a.normal.array):[],uv:a.uv?Array.from(a.uv.array):[],color:a.color?Array.from(a.color.array):[],index:g.index?Array.from(g.index.array):[],groups:g.groups});return id;}
 const batches=[];scene.updateMatrixWorld(true);scene.traverse(o=>{if(!o.isMesh)return;const transforms=[];if(o.isInstancedMesh){const m=new T.Matrix4();for(let i=0;i<o.count;i++){o.getMatrixAt(i,m);transforms.push(o.matrixWorld.clone().multiply(m).elements);}}else transforms.push(o.matrixWorld.elements);batches.push({geometry:geometry(o.geometry),materials:(Array.isArray(o.material)?o.material:[o.material]).map(material),transforms});});
 const {WEAPONS}=await import('/weapon-config.js');const avatar=new PeerAvatar(),weapons={};
 for(const config of WEAPONS){avatar.setWeapon(config.id);avatar.body.updateMatrixWorld(true);const parts=[];avatar.body.traverse(o=>{if(o.isMesh)parts.push({geometry:geometry(o.geometry),materials:(Array.isArray(o.material)?o.material:[o.material]).map(material),transform:o.matrix.elements});});weapons[config.id]={...config,parts};}
 return {version:1,seed:42,spawn:layout.spawn,terminal:layout.shop.terminal,colliders:layout.colliders,batches,geometries,materials,textures,weapons};
});
await mkdir('godot/assets',{recursive:true});for(let i=0;i<data.textures.length;i++)await writeFile(`godot/assets/label-${i}.png`,Buffer.from(data.textures[i].split(',')[1],'base64'));delete data.textures;await writeFile('godot/assets/city.json',JSON.stringify(data));console.log(`Godot bridge: ${data.batches.length} batches, ${data.colliders.length} colliders, all ten original weapons, seed ${data.seed}`);
}finally{await browser.close();await vite.close();}
