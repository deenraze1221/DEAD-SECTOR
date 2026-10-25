import { chromium } from 'playwright';
import assert from 'node:assert/strict';
(async () => {
  const browser = await chromium.launch({executablePath:process.env.CHROMIUM_PATH || undefined,headless:true,args:['--no-sandbox','--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
  try {
    const page = await browser.newPage({ viewport: {width:1440,height:900} });
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
    await page.goto(process.env.BASE_URL || 'http://127.0.0.1:5173/');
    await page.waitForFunction(()=>!document.getElementById('start').disabled);

    const snapshot=()=>page.evaluate(async()=> (await import('/game.js')).getSnapshot());
    async function waitForState(predicate) {
      const deadline = Date.now() + 15000;
      while (Date.now() < deadline) {
        const state = await snapshot();
        if (predicate(state)) return state;
        await page.waitForTimeout(50);
      }
      assert.fail('Game did not reach the expected state');
    }
    await page.locator('#settings-button').click();
    await page.locator('#sensitivity').fill('1.4');
    await page.reload();
    assert.equal(await page.locator('#sensitivity').inputValue(),'1.4');
    await page.locator('#start').click();
    await page.waitForFunction(()=>document.pointerLockElement?.id==='scene');
    await waitForState(state => state.running);
    const initial=await snapshot();
    await page.keyboard.down('KeyW');await page.waitForTimeout(450);await page.keyboard.up('KeyW');
    const walked=await snapshot();assert.ok(walked.position.z<initial.position.z-.5);
    await page.keyboard.press('Space');await page.waitForTimeout(120);
    assert.ok((await snapshot()).position.y>0);
    await waitForState(state => state.grounded);
    await page.mouse.move(800,400);await page.waitForTimeout(100);
    assert.notEqual((await snapshot()).yaw,0);
    await page.keyboard.down('KeyW');await page.keyboard.press('Escape');await page.waitForTimeout(100);
    await waitForState(state => !state.running);
    const paused=await snapshot();await page.waitForTimeout(350);
    assert.deepEqual((await snapshot()).position,paused.position);
    await page.keyboard.up('KeyW');
    await page.locator('#start').click();await page.waitForTimeout(200);
    await waitForState(state => state.running);
    const resume=await snapshot();await page.waitForTimeout(200);
    assert.deepEqual((await snapshot()).position,resume.position);

    await page.keyboard.press('Escape');await page.waitForTimeout(100);
    await page.locator('#restart').click();await waitForState(state => state.running);
    assert.deepEqual((await snapshot()).position,{x:0,y:0,z:16});
    await page.keyboard.press('Escape');await page.waitForTimeout(100);
    await page.setViewportSize({width:800,height:600});

    assert.deepEqual(errors,[]);
    console.log('PASS: WebGL scene, pointer lock, WASD, jump/landing, mouse look, pause/resume/input reset, restart, persisted sensitivity, resize; no console errors.');
  } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exit(1)});
