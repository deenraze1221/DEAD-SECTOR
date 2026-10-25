import * as THREE from 'three';
import {t,remaining} from './i18n.js';
import { Player, PLAYER, FIXED_STEP } from './player.js';
import { createMap } from './map.js';
import { createPSX } from './psx.js';
import { Rifle } from './weapons.js';
import { Enemies } from './enemies.js';
import { GameAudio } from './audio.js';
import {Session} from './network.js';

const $ = id => document.getElementById(id);
const keys = new Set();
const movementKeys = new Set(['KeyW', 'KeyA', 'KeyS', 'KeyD', 'ShiftLeft', 'ShiftRight', 'Space']);
let renderer, scene, camera, player;
let psx, rifle, enemies, exit, colliders=[], world, peer, peerPose=null, netTime=0;
let seed=crypto.getRandomValues(new Uint32Array(1))[0];
const session=new Session(receiveNetwork,text=>{$('network-status').textContent=text;});
const audio = new GameAudio();
let hp=100, outcome='', shooting=false, triggerPressed=false, aiming=false, hitTime=0, hurtTime=0, elapsed=0;
let running = false, started = false, jumpQueued = false, accumulator = 0, previousTime = 0;
let sensitivity = 1;
try {
  const saved = Number(localStorage.getItem('dead-sector:sensitivity'));
  if (saved >= .2 && saved <= 2) sensitivity = saved;
} catch { /* Storage may be unavailable in private/restricted browsers. */ }
$('sensitivity').value = sensitivity;
$('sensitivity-value').value = sensitivity.toFixed(1);
$('sensitivity').addEventListener('input', event => {
  sensitivity = Number(event.target.value); $('sensitivity-value').value = sensitivity.toFixed(1);
  try { localStorage.setItem('dead-sector:sensitivity', sensitivity); } catch { /* Optional preference. */ }
});
$('settings-button').addEventListener('click', () => {
  $('network-panel').hidden=true;
  $('settings').hidden = !$('settings').hidden;
  $('settings-button').setAttribute('aria-expanded', String(!$('settings').hidden));
});

function clearInput() { keys.clear(); shooting=false; triggerPressed=false; aiming=false; jumpQueued = false; accumulator = 0; }
function setRunning(value) {
  running = value; clearInput();
  document.body.classList.toggle('playing', value);
  for (const id of ['menu', 'briefing', 'footer']) $(id).hidden = value;
  $('hud').hidden = !value;
  $('restart').hidden = !started;
  $('start').innerHTML = outcome ? t('НОВАЯ ЗАЧИСТКА <span>↗</span>') : started ? t('ПРОДОЛЖИТЬ <span>↗</span>') : t('ИГРАТЬ <span>↗</span>');
}

function rebuild(seedValue){
  if(scene){scene.traverse(o=>{o.geometry?.dispose();for(const m of (Array.isArray(o.material)?o.material:[o.material]))m?.dispose();});}
  scene=new THREE.Scene();world=createMap(scene,seedValue);seed=world.seed;colliders=world.colliders;
  player=new Player(world.spawn);enemies=new Enemies(scene,colliders,audio,world.enemySpawns);
  exit=new THREE.Mesh(new THREE.BoxGeometry(4,3,.3),new THREE.MeshBasicMaterial({color:'#873d2b'}));exit.position.set(world.exit.x,1.5,world.exit.z);scene.add(exit);
  peer=new THREE.Group();const material=new THREE.MeshStandardMaterial({color:'#7d9676'});const body=new THREE.Mesh(new THREE.CapsuleGeometry(.28,.8,3,6),material);body.position.y=1;peer.add(body);const head=new THREE.Mesh(new THREE.IcosahedronGeometry(.23,1),material);head.position.y=1.75;peer.add(head);peer.visible=false;scene.add(peer);peerPose=null;
}
function resetCombat(){session.close();rebuild(crypto.getRandomValues(new Uint32Array(1))[0]);rifle.reset();hp=100;outcome='';hitTime=0;hurtTime=0;elapsed=0;updateCamera();}
function finish(result){if(outcome)return;outcome=result;if(session.active&&session.host)session.send({type:'end',result});pause();$('message').textContent=result==='win'?t('СЕКТОР ЗАЧИЩЕН. Вы добрались до выхода.'):t('ВЫ ПОГИБЛИ. Начните зачистку заново.');}
async function enter() {
  if(outcome)resetCombat();
  audio.unlock();
  try {
    await $('scene').requestPointerLock();
  } catch {
    $('message').textContent = t('Браузер не разрешил захват мыши. Нажмите «Войти в сектор» ещё раз или откройте игру в отдельной вкладке.');
  }
}
$('start').addEventListener('click', enter);
$('restart').addEventListener('click', () => { resetCombat(); enter(); });
document.addEventListener('pointerlockchange', () => {
  const locked = document.pointerLockElement === $('scene');
  if (locked) { started = true; $('message').textContent = t('Пауза · движение остановлено. Продолжить — захватить мышь.'); }
  setRunning(locked);
  if(outcome) $('message').textContent=outcome==='win'?t('СЕКТОР ЗАЧИЩЕН. Вы добрались до выхода.'):t('ВЫ ПОГИБЛИ. Начните зачистку заново.');
  if (!locked) $('start').focus({ preventScroll: true });
});
document.addEventListener('pointerlockerror', () => {
  $('message').textContent = t('Не удалось захватить мышь. Повторите вход в сектор в отдельной вкладке браузера.');
});
function pause() { if (document.pointerLockElement) document.exitPointerLock(); setRunning(false); }
window.addEventListener('blur', pause);
document.addEventListener('visibilitychange', () => { if (document.hidden) pause(); });
document.addEventListener('keydown', event => {
  if (event.code === 'Escape' && running) { pause(); return; }
  if(running && event.code==='KeyR'){rifle.reload();event.preventDefault();return;}
  if(running && event.code==='KeyE' && !enemies.alive && Math.hypot(player.position.x,player.position.z-world.exit.z)<2.5 && player.position.y<1){if(session.active&&!session.host)session.send({type:'exit'});else finish('win');return;}
  if (!running || !movementKeys.has(event.code)) return;
  event.preventDefault();
  if (event.code === 'Space' && !keys.has('Space') && !event.repeat) jumpQueued = true;
  keys.add(event.code);
});
document.addEventListener('keyup', event => { keys.delete(event.code); });
document.addEventListener('mousemove', event => { if (running) player.look(event.movementX, event.movementY, sensitivity); });

document.addEventListener('mousedown', event=>{if(!running)return;if(event.button===0){shooting=true;triggerPressed=true;}if(event.button===2)aiming=true;});
document.addEventListener('mouseup', event=>{if(event.button===0)shooting=false;if(event.button===2)aiming=false;});
$('scene').addEventListener('contextmenu',event=>event.preventDefault());

function updateCamera() {
  camera.position.set(player.position.x, player.position.y + PLAYER.eye, player.position.z);
  camera.rotation.set(player.pitch, player.yaw, 0, 'YXZ');
}
function resize() {
  camera.aspect = innerWidth / innerHeight; camera.updateProjectionMatrix();
  rifle.camera.aspect=camera.aspect;rifle.camera.updateProjectionMatrix();
  psx.resize(innerWidth, innerHeight);
}

// Read-only snapshot for reproducible browser checks, without allowing state mutation.
export function getSnapshot() {
  return { running, started, position: player ? { ...player.position } : null, yaw: player?.yaw, pitch: player?.pitch, grounded: player?.grounded, hp, outcome, ammo:rifle?.ammo, reserve:rifle?.reserve, reloading:rifle?.reloadTime>0, weapon:rifle?.config.id, capacity:rifle?.config.capacity, seed,room:session.code,networkActive:session.active,peerVisible:peer?.visible,remaining:enemies?.alive, enemies:enemies?.items.map(e=>({hp:e.hp,x:e.rig.position.x,z:e.rig.position.z})) };
}

try {
  renderer = new THREE.WebGLRenderer({ canvas: $('scene'), antialias: false, powerPreference: 'high-performance' });
  renderer.shadowMap.enabled = true; renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.1;
  scene = new THREE.Scene(); camera = new THREE.PerspectiveCamera(78, 1, .1, 260);
  rebuild(seed);rifle=new Rifle(audio);
  psx = createPSX(scene, renderer);
  updateCamera(); resize(); window.addEventListener('resize', resize);
  $('scene').addEventListener('webglcontextlost', event => {
    event.preventDefault(); pause(); $('start').disabled = true;
    $('message').textContent = t('Графический контекст потерян. Перезагрузите страницу, чтобы восстановить сектор.');
  });
  $('start').disabled = false; setRunning(false);
  function frame(time) {
    const delta = Math.min(Math.max((time - previousTime) / 1000, 0), .1); previousTime = time;
    if (running || (session.active && started && !outcome)) {
      accumulator += delta;
      while (accumulator >= FIXED_STEP) {
        player.step(FIXED_STEP, {
          forward: keys.has('KeyW'), backward: keys.has('KeyS'), left: keys.has('KeyA'), right: keys.has('KeyD'),
          sprint: keys.has('ShiftLeft') || keys.has('ShiftRight'), jump: jumpQueued,
        }, colliders);
        jumpQueued = false; accumulator -= FIXED_STEP;
        elapsed+=FIXED_STEP;rifle.step(FIXED_STEP);
        updateCamera();camera.updateMatrixWorld();
        if(((shooting && rifle.config.automatic)||triggerPressed) && rifle.fire()){
          const direction=camera.getWorldDirection(new THREE.Vector3());
          if(session.active&&!session.host)session.send({type:'shot',direction:direction.toArray(),weapon:rifle.config.id});else if(enemies.shoot(camera.position,direction,rifle.config.damage))hitTime=.15;
          player.pitch=Math.min(Math.PI/2-.02,player.pitch+.008);
        }
        triggerPressed=false;
        if(!session.active||session.host){
          enemies.step(FIXED_STEP,player,damage=>{hp=Math.max(0,hp-damage);hurtTime=.3;audio.play('hurt');if(!hp)finish('dead');},peerPose&&peerPose.hp>0?{position:peerPose}:null,()=>session.send({type:'damage'}));
        }
        hitTime=Math.max(0,hitTime-FIXED_STEP);hurtTime=Math.max(0,hurtTime-FIXED_STEP);
        if(!running&&!session.active)break;
      }
      $('movement').textContent = !player.grounded ? t('В ВОЗДУХЕ') : keys.has('ShiftLeft') || keys.has('ShiftRight') ? t('БЕГ') : t('ПЕШКОМ');
      const fov=aiming?55:78;if(camera.fov!==fov){camera.fov=fov;camera.updateProjectionMatrix();}
      updateCamera();
    } else if (!started) {
      camera.position.set(36, 28, 95); camera.lookAt(0, 2, 40);
    }
    $('health').textContent=hp;$('health-bar').style.width=`${hp*1.35}px`;
    $('ammo').textContent=rifle.ammo;$('reserve').textContent=rifle.reserve;
    $('weapon-state').textContent=rifle.reloadTime?t('ПЕРЕЗАРЯДКА…'):!rifle.ammo?t('R — ПЕРЕЗАРЯДИТЬ'):t(rifle.config.label);
    $('objective').textContent=enemies.alive?remaining(enemies.alive):t('ВЫХОД ОТКРЫТ · СЕВЕРНЫЕ ВОРОТА · E');
    $('hit-marker').hidden=hitTime<=0;document.body.classList.toggle('hurt',hurtTime>0&&running);
    exit.material.color.set(enemies.alive?'#873d2b':'#afdb65');
    rifle.pose(elapsed,keys.size>0,aiming);
    if(session.active){netTime+=delta;if(netTime>.1){netTime=0;session.send({type:'pose',...player.position,yaw:player.yaw,hp});if(session.host)session.send({type:'state',enemies:enemies.items.map(e=>({hp:e.hp,x:e.rig.position.x,z:e.rig.position.z,rotation:e.rig.rotation.toArray()}))});}}
    renderer.autoClear=true;renderer.render(scene, camera);
    if(started&&!outcome){renderer.autoClear=false;renderer.clearDepth();renderer.render(rifle.scene,rifle.camera);}
  }
  renderer.setAnimationLoop(frame);
} catch (error) {
  console.error('DEAD SECTOR initialization failed:', error);
  $('start').disabled = true; $('start').textContent = t('ЗАПУСК НЕДОСТУПЕН');
  $('message').textContent = t('Не удалось запустить 3D-сцену. Нужен браузер с WebGL 2 и включённым аппаратным ускорением.');
}

function receiveNetwork(m){
 if(m.type==='room'&&!m.host){rebuild(m.seed);player.position.x=2;hp=100;outcome='';started=false;setRunning(false);}
 if(m.type==='pose'){peerPose=m;peer.visible=true;peer.position.set(m.x,m.y,m.z);peer.rotation.y=m.yaw;}
 if(m.type==='closed'||m.type==='left'){peerPose=null;if(peer)peer.visible=false;}
 if(m.type==='state'&&!session.host){m.enemies.forEach((v,i)=>{const e=enemies.items[i];if(!e)return;e.hp=v.hp;e.rig.position.set(v.x,v.hp?0:.25,v.z);e.rig.rotation.fromArray(v.rotation);});}
 if(m.type==='shot'&&session.host){const direction=new THREE.Vector3(...m.direction);if(direction.lengthSq()>.1)enemies.shoot(new THREE.Vector3(...m.origin),direction.normalize(),m.damage);}
 if(m.type==='damage'){hp=Math.max(0,hp-m.amount);hurtTime=.3;if(!hp)finish('dead');}
 if(m.type==='end'&&!session.host)finish(m.result);
 if(m.type==='exit'&&session.host&&!enemies.alive)finish('win');
}
$('online').addEventListener('click',()=>{$('settings').hidden=true;$('network-panel').hidden=!$('network-panel').hidden;});
$('host-room').addEventListener('click',()=>session.connect($('server-url').value,'create',seed));
$('join-room').addEventListener('click',()=>session.connect($('server-url').value,'join',seed,$('room-code').value.trim()));
$('close-room').addEventListener('click',()=>{session.close();peer.visible=false;$('network-status').textContent=t('Сессия закрыта');});
$('server-url').value=location.protocol.startsWith('http')?`${location.protocol==='https:'?'wss':'ws'}://${location.hostname}:3000`:'ws://localhost:3000';
window.addEventListener('game-language-change',()=>{setRunning(false);$('message').textContent=t($('message').textContent);});
