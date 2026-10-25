import {chromium} from 'playwright';
import assert from 'node:assert/strict';
const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||undefined,headless:true,args:['--no-sandbox','--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
try{
 const errors=[],context=await browser.newContext({viewport:{width:1000,height:700}}),host=await context.newPage(),guest=await context.newPage();
 const read=p=>p.evaluate(()=>readGame());
 async function waitFor(p,predicate,description,timeout=20000){
  const deadline=Date.now()+timeout;
  while(Date.now()<deadline){const state=await read(p);if(predicate(state))return state;await p.waitForTimeout(100);}
  assert.fail(description+': '+JSON.stringify(await read(p))+' errors: '+JSON.stringify(errors));
 }
 for(const p of [host,guest]){
  p.on('pageerror',e=>errors.push(e.stack));p.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  await p.goto(process.env.BASE_URL||'http://127.0.0.1:5173/app.html');await p.waitForFunction(()=>!document.getElementById('start').disabled);
  await p.evaluate(async()=>window.readGame=(await import('/game.js')).getSnapshot);
  await p.locator('#online').click();await p.locator('#server-url').fill(process.env.WS_URL||'ws://127.0.0.1:3000');
 }
 await host.locator('#host-room').click();const room=await waitFor(host,s=>s.networkActive&&s.room,'host room creation');
 await guest.locator('#room-code').fill(room.room);await guest.locator('#join-room').click();
 const joined=await waitFor(guest,s=>s.networkActive&&s.room===room.room,'guest join');assert.equal(joined.seed,room.seed);
 await host.bringToFront();await host.locator('#start').click();await waitFor(host,s=>s.started&&s.running,'host pointer lock');
 await guest.bringToFront();await guest.locator('#start').click();await waitFor(guest,s=>s.started&&s.running,'guest pointer lock');
 const replicated=await waitFor(guest,s=>s.remaining>0,'first replicated hive spawn',90000);
 const authoritative=await read(host);assert.ok(authoritative.remaining>0);assert.equal(authoritative.peerVisible,true);assert.equal(replicated.peerVisible,true);
 assert.equal(authoritative.wave.number,replicated.wave.number);assert.ok(Math.abs(authoritative.wave.time-replicated.wave.time)<1);
 await guest.keyboard.press('Escape');await host.bringToFront();if(!(await host.locator('#network-panel').isVisible()))await host.locator('#online').click();
 await host.locator('#close-room').click();await waitFor(guest,s=>!s.networkActive,'explicit disconnect');assert.deepEqual(errors,[]);
 console.log('PASS two-browser city, synchronized waves, hive/enemy replication, visible partners and disconnect', {hostTime:authoritative.wave.time,guestTime:replicated.wave.time});
}finally{await browser.close();}
