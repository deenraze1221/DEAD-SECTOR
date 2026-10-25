import test from 'node:test';import assert from 'node:assert/strict';import {Waves,chooseEnemy,ENEMY_TYPES} from '../waves.js';
test('waves distribute 30,35,40 spawns over full assaults with 10-second breaks',()=>{const w=new Waves(77),events=[];let now=0;const spawn=(type,hive)=>events.push({type,hive,time:now});w.step(3.99,spawn);assert.equal(events.length,0);for(let i=0;i<377;i++){now=3.99+i;w.step(1,spawn);}assert.equal(w.number,3);assert.equal(events.length,105);assert.ok(events.every(e=>e.hive>=0&&e.hive<4));const exact=new Waves(1),counts=[];exact.step(120,()=>counts.push(1));assert.equal(counts.length,30);assert.equal(exact.rest,true);exact.step(9.9,()=>assert.fail('No spawns in break'));assert.equal(exact.number,1);exact.step(.1,()=>assert.fail());assert.equal(exact.number,2);assert.equal(exact.spawned,0);});
test('spawn chances match confirmed 55/20/20/5 thresholds and snapshots continue deterministically',()=>{assert.equal(chooseEnemy(()=>.549),'zombie');assert.equal(chooseEnemy(()=>.55),'runner');assert.equal(chooseEnemy(()=>.75),'eye');assert.equal(chooseEnemy(()=>.95),'brute');assert.equal(Object.values(ENEMY_TYPES).reduce((n,s)=>n+s.weight,0),100);const a=new Waves(2);a.step(72,()=>{});const b=new Waves();b.restore(a.snapshot());const aa=[],bb=[];a.step(100,(...v)=>aa.push(v));b.step(100,(...v)=>bb.push(v));assert.deepEqual(aa,bb);});

test('a wave stays in clearing indefinitely until the last enemy dies, then waits ten seconds',()=>{
 const wave=new Waves(77),events=[];let alive=0;
 const spawn=(type,hive)=>{alive++;events.push({type,hive});};
 wave.step(120,spawn,()=>alive);assert.equal(alive,30);assert.equal(wave.clearing,true);assert.equal(wave.rest,false);
 wave.step(600,spawn,()=>alive);assert.equal(wave.number,1);assert.equal(events.length,30);assert.equal(wave.time,120);
 alive=1;wave.step(120,spawn,()=>alive);assert.equal(wave.number,1);
 const restored=new Waves();restored.restore(wave.snapshot());restored.step(90,()=>assert.fail('clearing snapshot cannot spawn'),()=>1);assert.equal(restored.clearing,true);
 alive=0;wave.step(9.9,spawn,()=>alive);assert.equal(wave.rest,true);assert.equal(wave.number,1);
 wave.step(.1,spawn,()=>alive);assert.equal(wave.number,2);assert.equal(wave.spawned,0);
 wave.step(120,spawn,()=>alive);assert.equal(alive,35);assert.equal(wave.clearing,true);
});
test('killing early spawns does not skip the remaining spawn schedule',()=>{
 const wave=new Waves();let spawned=0;wave.step(119,()=>spawned++,()=>0);assert.equal(spawned,29);assert.equal(wave.rest,false);wave.step(1,()=>spawned++,()=>0);assert.equal(spawned,30);assert.equal(wave.rest,true);
});
