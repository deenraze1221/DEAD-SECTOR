import { chromium } from 'playwright';
import assert from 'node:assert/strict';
(async()=>{const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH || undefined,headless:true,args:['--no-sandbox','--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});try{
 const p=await browser.newPage({viewport:{width:960,height:600}});const errors=[];p.on('pageerror',e=>errors.push(e.message));p.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 await p.goto(process.env.BASE_URL || 'http://127.0.0.1:5173/app.html');await p.waitForFunction(()=>!document.getElementById('start').disabled);
 await p.evaluate(async()=>{window.readGame=(await import('/game.js')).getSnapshot;window.THREE=await import('/node_modules/.vite/deps/three.js');window.collision=(await import('/map.js')).getColliders();window.nav=(await import('/navigation.js')).createNavigation(window.collision);window.wallDistance=(await import('/enemies.js')).obstacleDistance;});
 await p.locator('#start').click();await p.locator('#hud .sector-tag').waitFor({state:'visible'});
 await p.mouse.down();await p.waitForTimeout(500);await p.mouse.up();const ammo=await p.evaluate(()=>readGame().ammo);assert.ok(ammo<(await p.evaluate(()=>readGame().capacity)));await p.keyboard.press('KeyR');
 for(let i=0;i<60;i++){if(!(await p.evaluate(()=>readGame().reloading)))break;await p.waitForTimeout(100);}assert.equal(await p.evaluate(()=>readGame().ammo),await p.evaluate(()=>readGame().capacity));
 let last=-1;
 for(let i=0;i<1100;i++){
  const command=await p.evaluate(()=>{
   const s=readGame();if(s.outcome)return {done:s.outcome};const origin=new THREE.Vector3(s.position.x,s.position.y+1.65,s.position.z);
   const alive=s.enemies.filter(e=>e.hp>0).sort((a,b)=>Math.hypot(a.x-s.position.x,a.z-s.position.z)-Math.hypot(b.x-s.position.x,b.z-s.position.z));
   let target=alive.find(e=>{const d=new THREE.Vector3(e.x,1.2,e.z).sub(origin),len=d.length();return wallDistance(origin,d.normalize(),collision,len)>=len-.1;});const fire=!!target;
   if(!target){const goal=alive[0]||{x:0,z:-32};target=nav.path(s.position,goal)[0]||goal;}
   const dx=target.x-s.position.x,dz=target.z-s.position.z;const yaw=Math.atan2(-dx,-dz),pitch=fire?Math.atan2(1.2-origin.y,Math.hypot(dx,dz)):0;
   let delta=yaw-s.yaw;while(delta>Math.PI)delta-=Math.PI*2;while(delta<-Math.PI)delta+=Math.PI*2;
   document.dispatchEvent(new MouseEvent('mousemove',{movementX:-delta/.002,movementY:-(pitch-s.pitch)/.002}));
   return {fire,remaining:s.remaining,hp:s.hp,reload:!s.ammo,exit:!alive.length&&Math.hypot(s.position.x,s.position.z+32)<2.4};
  });
  if(command.done){assert.equal(command.done,'win');console.log('PASS: real keyboard/mouse bot cleared five mutants and reached exit');break;}
  if(command.remaining!==last){console.log('Remaining:',command.remaining,'HP:',command.hp);last=command.remaining;}
  if(command.exit)await p.keyboard.press('KeyE');
  if(command.fire){await p.keyboard.up('KeyW');await p.mouse.up();await p.mouse.down();}else{await p.mouse.up();await p.keyboard.down('KeyW');}
  if(command.reload)await p.keyboard.press('KeyR');await p.waitForTimeout(70);
 }
 await p.mouse.up();await p.keyboard.up('KeyW');assert.equal(await p.evaluate(()=>readGame().outcome),'win');assert.deepEqual(errors,[]);
 await p.waitForTimeout(1200);await p.locator('#start').click();await p.locator('#hud .sector-tag').waitFor({state:'visible'});
 for(let i=0;i<1000;i++){const s=await p.evaluate(()=>readGame());if(s.outcome)break;await p.waitForTimeout(100);}
 assert.equal(await p.evaluate(()=>readGame().outcome),'dead');console.log('PASS: idle player takes damage, dies and gets retry screen');
 }finally{await browser.close()}})().catch(e=>{console.error(e);process.exit(1)});
