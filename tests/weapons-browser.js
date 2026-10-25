import {chromium} from 'playwright';
import assert from 'node:assert/strict';
import {WEAPONS} from '../weapon-config.js';
const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||undefined,headless:true,args:['--no-sandbox','--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
try{
 for(const [i,spec] of WEAPONS.entries()){
  const context=await browser.newContext({viewport:{width:1200,height:800}});
  await context.addInitScript(roll=>{Math.random=()=>roll},(i+.1)/WEAPONS.length);
  const p=await context.newPage(),errors=[];p.on('pageerror',e=>errors.push(e.message));p.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  await p.goto(process.env.BASE_URL||'http://127.0.0.1:5173/app.html');await p.waitForFunction(()=>!document.getElementById('start').disabled);
  await p.evaluate(async()=>{window.readGame=(await import('/game.js')).getSnapshot;});
  const initial=await p.evaluate(()=>readGame());assert.equal(initial.weapon,spec.id);assert.equal(initial.capacity,spec.capacity);
  await p.locator('#start').click();await p.locator('#hud .sector-tag').waitFor({state:'visible'});await p.mouse.down();
  const target=initial.capacity-(spec.automatic?2:1);
  await p.waitForFunction(target=>readGame().ammo<=target,target,{timeout:30000});
  if(!spec.automatic)await p.waitForTimeout(400);await p.mouse.up();
  const fired=await p.evaluate(()=>readGame());if(spec.automatic)assert.ok(fired.ammo<=target);else assert.equal(fired.ammo,target);
  await p.keyboard.press('KeyR');await p.waitForFunction(()=>readGame().reloading,{timeout:5000});
  await p.waitForFunction(()=>!readGame().reloading,{timeout:120000});assert.equal(await p.evaluate(()=>readGame().ammo),initial.capacity);
  await p.keyboard.press('Escape');await p.waitForFunction(()=>!readGame().running);const paused=await p.evaluate(()=>readGame());
  assert.equal(paused.weapon,initial.weapon);await p.waitForTimeout(100);assert.equal((await p.evaluate(()=>readGame())).ammo,paused.ammo);assert.deepEqual(errors,[]);
  console.log('PASS',spec.id,'random loadout, fire mode, reload, pause');await context.close();
 }
}finally{await browser.close();}
