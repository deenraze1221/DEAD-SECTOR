import * as THREE from 'three';
import {t} from './i18n.js';
import { Player, PLAYER, FIXED_STEP } from './player.js';
import { createMap } from './map.js';
import { createPSX } from './psx.js';
import { Rifle,WEAPONS } from './weapons.js';
import { Enemies } from './enemies.js';
import { GameAudio } from './audio.js';
import {Session} from './network.js';
import {Supplies} from './supplies.js';
import {Waves} from './waves.js';

const $ = id => document.getElementById(id);
const keys = new Set();
const movementKeys = new Set(['KeyW', 'KeyA', 'KeyS', 'KeyD', 'ShiftLeft', 'ShiftRight', 'Space']);
let renderer, scene, camera, player;
let psx, rifle, enemies, colliders=[], world, waves, peer, peerPose=null, netTime=0,supplies,footstep=0,downed=false;
let seed=crypto.getRandomValues(new Uint32Array(1))[0];
const session=new Session(receiveNetwork,text=>{$('network-status').textContent=text;});
const audio = new GameAudio();
let hp=100, outcome='', shooting=false, triggerPressed=false, hitTime=0, hurtTime=0, elapsed=0;
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

function clearInput() { keys.clear(); shooting=false; triggerPressed=false; jumpQueued = false; accumulator = 0; }
function setRunning(value) {
  running = value; clearInput();
  document.body.classList.toggle('playing', value);
  for (const id of ['menu', 'briefing', 'footer']) $(id).hidden = value;
  $('hud').hidden = !value;
  $('restart').hidden = !started;
  $('start').innerHTML = outcome ? t('НОВЫЙ ЗАБЕГ <span>↗</span>') : started ? t('ПРОДОЛЖИТЬ <span>↗</span>') : t('ИГРАТЬ <span>↗</span>');
}

function rebuild(seedValue){
  if(scene){scene.traverse(o=>{if(o.isInstancedMesh)o.dispose();o.geometry?.dispose();for(const m of (Array.isArray(o.material)?o.material:[o.material]))if(m){m.map?.dispose();m.dispose();}});}
  scene=new THREE.Scene();world=createMap(scene,seedValue);seed=world.seed;colliders=world.colliders;
  player=new Player(world.spawn);supplies=new Supplies(scene,world.supplies);enemies=new Enemies(scene,colliders,audio);waves=new Waves(seed);
  peer=new THREE.Group();const material=new THREE.MeshStandardMaterial({color:'#7d9676'});const body=new THREE.Mesh(new THREE.CapsuleGeometry(.28,.8,3,6),material);body.position.y=1;peer.add(body);const head=new THREE.Mesh(new THREE.IcosahedronGeometry(.23,1),material);head.position.y=1.75;peer.add(head);peer.visible=false;scene.add(peer);peerPose=null;
}
function resetCombat(){session.close();rebuild(crypto.getRandomValues(new Uint32Array(1))[0]);rifle.reset();hp=100;outcome='';hitTime=0;hurtTime=0;elapsed=0;footstep=0;downed=false;updateCamera();}
function receiveDamage(damage){if(hp<=0)return;hp=Math.max(0,hp-damage);hurtTime=.3;audio.play('hurt');if(!hp){if(session.active){downed=true;pause();$('message').textContent=t('Ожидайте помощи напарника · E рядом с раненым');}else finish('dead');}}
function finish(result){if(outcome)return;outcome=result;if(session.active&&session.host)session.send({type:'end',result});pause();$('message').textContent=result==='win'?t('СЕКТОР ЗАЧИЩЕН. Вы добрались до выхода.'):t('ВЫ ПОГИБЛИ. Начните зачистку заново.');}
async function enter() {
  if(session.reconnecting||session.hostAway){$('message').textContent=t('Переподключение. Дождитесь хозяина комнаты.');return;}
  if(session.active&&hp<=0){$('message').textContent=t('Ожидайте помощи напарника · E рядом с раненым');return;}
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
  if(running && event.code==='KeyE'){
    if(session.active&&peerPose?.hp<=0&&Math.hypot(peerPose.x-player.position.x,peerPose.z-player.position.z)<2.5){session.send({type:'revive'});return;}
    const supply=supplies.nearby(player.position);
    if(supply){if(session.active&&!session.host)session.send({type:'pickup',id:supply.id});else{const next=supplies.apply(supply.id,hp,rifle);if(next!==null){hp=next;audio.play('reload');}}return;}
  }
  if(running && event.code==='KeyR'){rifle.reload();event.preventDefault();return;}
  if (!running || !movementKeys.has(event.code)) return;
  event.preventDefault();
  if (event.code === 'Space' && !keys.has('Space') && !event.repeat) jumpQueued = true;
  keys.add(event.code);
});
document.addEventListener('keyup', event => { keys.delete(event.code); });
document.addEventListener('mousemove', event => { if (running) player.look(event.movementX, event.movementY, sensitivity); });

document.addEventListener('mousedown', event=>{if(!running)return;if(event.button===0){shooting=true;triggerPressed=true;}});
document.addEventListener('mouseup', event=>{if(event.button===0)shooting=false;});
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
  return { running, started, position: player ? { ...player.position } : null, yaw: player?.yaw, pitch: player?.pitch, grounded: player?.grounded,fov:camera?.fov,wave:waves?.snapshot(), hp, outcome, ammo:rifle?.ammo, reserve:rifle?.reserve, reloading:rifle?.reloadTime>0, weapon:rifle?.config.id, capacity:rifle?.config.capacity, seed,room:session.code,networkActive:session.active,peerVisible:peer?.visible,downed,supplies:supplies?.items.map(s=>({id:s.id,type:s.type,x:s.x,z:s.z,taken:s.taken})),remaining:enemies?.alive, enemies:enemies?.items.map(e=>({id:e.id,type:e.type,hp:e.hp,fast:e.type==='runner',x:e.rig.position.x,y:e.rig.position.y,z:e.rig.position.z})) };
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
        if(hp>0&&((shooting && rifle.config.automatic)||triggerPressed) && rifle.fire()){
          const direction=camera.getWorldDirection(new THREE.Vector3());
          if(session.active&&!session.host)session.send({type:'shot',direction:direction.toArray(),weapon:rifle.config.id});else if(enemies.shoot(camera.position,direction,rifle.config.damage))hitTime=.15;
          player.pitch=Math.min(Math.PI/2-.02,player.pitch+.008);
        }
        triggerPressed=false;
        if(!session.active||session.host){
          waves.step(FIXED_STEP,(type,index)=>{const h=world.hives[index];enemies.spawn(type,h.x-Math.sign(h.x)*1.5,h.z-Math.sign(h.z)*1.5);},()=>enemies.alive);
          player.hp=hp;if(hp>0||peerPose?.hp>0)enemies.step(FIXED_STEP,player,receiveDamage,peerPose&&peerPose.hp>0?{position:peerPose}:null,amount=>session.send({type:'damage',amount}));else if(peerPose)finish('dead');
        }
        hitTime=Math.max(0,hitTime-FIXED_STEP);hurtTime=Math.max(0,hurtTime-FIXED_STEP);
        if(!running&&!session.active)break;
      }
      $('movement').textContent = !player.grounded ? t('В ВОЗДУХЕ') : keys.has('ShiftLeft') || keys.has('ShiftRight') ? t('БЕГ') : t('ПЕШКОМ');
      if(running&&player.grounded&&player.distance-footstep>1.7){audio.play('step');footstep=player.distance;}
      updateCamera();
    } else if (!started) {
      camera.position.set(36, 28, 95); camera.lookAt(0, 2, 40);
    }
    $('health').textContent=hp;$('health-bar').style.width=`${hp*1.35}px`;
    $('ammo').textContent=rifle.ammo;$('reserve').textContent=rifle.reserve;
    $('weapon-state').textContent=rifle.reloadTime?t('ПЕРЕЗАРЯДКА…'):!rifle.ammo?t('R — ПЕРЕЗАРЯДИТЬ'):t(rifle.config.label);
    $('objective').textContent=`${t('ВОЛНА')} ${waves.number} · ${waves.clearing?t('ДОБЕЙТЕ ОСТАВШИХСЯ'):waves.rest?t('ПЕРЕРЫВ'):t('НАСТУПЛЕНИЕ')}${waves.clearing?'':` ${Math.ceil((waves.rest?10:120)-waves.time)}s`}`;
    $('wave-details').textContent=`${t('ЖИВЫХ')}: ${enemies.alive} · ${t('ВЫШЛО')}: ${waves.spawned}/${waves.total}`;
    $('hit-marker').hidden=hitTime<=0;document.body.classList.toggle('hurt',hurtTime>0&&running);
    supplies.tick(elapsed);
    const near=supplies.nearby(player.position);const living=enemies.items.filter(e=>e.hp>0);const nearest=living.sort((a,b)=>a.rig.position.distanceTo(camera.position)-b.rig.position.distanceTo(camera.position))[0];
    $('interaction').textContent=hp<=0?t('Ожидайте помощи напарника · E рядом с раненым'):session.active&&peerPose?.hp<=0&&Math.hypot(peerPose.x-player.position.x,peerPose.z-player.position.z)<2.5?t('E · Поднять напарника'):near?t(near.type==='health'?'E · Аптечка +35 HP':'E · Патроны'):nearest?`${t('Ближайший враг')} · ${Math.round(Math.hypot(nearest.rig.position.x-player.position.x,nearest.rig.position.z-player.position.z))} m · ${Math.atan2(nearest.rig.position.x-player.position.x,player.position.z-nearest.rig.position.z)*180/Math.PI|0}° N`:t('Выживайте. Ульи находятся в четырёх углах города.');
    enemies.updateEffects((running||(session.active&&started&&!outcome))?delta:0,session.active&&!session.host);
    rifle.pose(elapsed,keys.size>0);
    if(session.active){netTime+=delta;if(netTime>.1){netTime=0;session.send({type:'pose',...player.position,yaw:player.yaw,hp,ammo:rifle.ammo,reserve:rifle.reserve,weapon:rifle.config.id});if(session.host)session.send({type:'state',wave:waves.snapshot(),taken:supplies.items.filter(s=>s.taken).map(s=>s.id),enemies:enemies.snapshot(),projectiles:enemies.projectiles.map(p=>p.mesh.position.toArray())});}}
    renderer.autoClear=true;renderer.render(scene, camera);
    if(started&&!outcome){renderer.autoClear=false;renderer.clearDepth();renderer.render(rifle.scene,rifle.camera);}
  }
  renderer.setAnimationLoop(frame);
} catch (error) {
  console.error('DEAD SECTOR initialization failed:', error);
  $('start').disabled = true; $('start').textContent = t('ЗАПУСК НЕДОСТУПЕН');
  $('message').textContent = t('Не удалось запустить 3D-сцену. Нужен браузер с WebGL 2 и включённым аппаратным ускорением.');
}

function applyState(m){if(m.wave)waves.restore(m.wave);for(const id of m.taken||[])if(supplies.items[id])supplies.items[id].taken=true;enemies.apply(m.enemies);
 if(!session.host){for(const p of enemies.projectiles)scene.remove(p.mesh);enemies.projectiles=(m.projectiles||[]).map(pos=>({mesh:enemies.mesh(scene,'sphere',[.2,0],'bullet',...pos),life:6,velocity:new THREE.Vector3()}));}}
function receiveNetwork(m){
 if(m.type==='transport-lost'||(m.type==='away'&&m.role==='host')){pause();$('message').textContent=t('Переподключение. Дождитесь хозяина комнаты.');}

 if(m.type==='room'&&(!m.host||m.resume)){
   if(!m.resume||seed!==m.seed)rebuild(m.seed);
   if(!m.resume){player.position.x=2;hp=100;outcome='';started=false;}
   if(m.selfPose){Object.assign(player.position,{x:m.selfPose.x,y:m.selfPose.y,z:m.selfPose.z});hp=m.selfPose.hp;player.yaw=m.selfPose.yaw;rifle.reset(m.selfPose.weapon);rifle.ammo=m.selfPose.ammo??rifle.ammo;rifle.reserve=m.selfPose.reserve??rifle.reserve;}
   if(m.snapshot)applyState(m.snapshot);setRunning(false);downed=hp<=0;if(m.result)finish(m.result);
 }

 if(m.type==='pose'){peerPose=m;peer.visible=true;peer.position.set(m.x,m.y,m.z);peer.rotation.y=m.yaw;}
 if(m.type==='closed'||m.type==='left'){peerPose=null;if(peer)peer.visible=false;if(m.type==='closed'&&hp<=0)finish('dead');}
 if(m.type==='state'&&!session.host)applyState(m);
 if(m.type==='shot'&&session.host){const direction=new THREE.Vector3(...m.direction);if(direction.lengthSq()>.1)enemies.shoot(new THREE.Vector3(...m.origin),direction.normalize(),m.damage);}
 if(m.type==='damage')receiveDamage(m.amount);
 if(m.type==='revived'){hp=m.hp;downed=false;outcome='';$('message').textContent=t('Напарник помог. Продолжайте игру.');}
 if(m.type==='pickup'&&session.host&&peerPose){const s=supplies.items[m.id];if(s&&!s.taken&&Math.hypot(s.x-peerPose.x,s.z-peerPose.z)<1.6&&peerPose.hp>0&&((s.type==='health'&&peerPose.hp<100)||(s.type==='ammo'&&peerPose.reserve<(WEAPONS.find(w=>w.id===peerPose.weapon)?.reserve||90)*2))){s.taken=true;session.send({type:'grant',id:s.id,kind:s.type});}}
 if(m.type==='grant'){if(m.kind==='health')hp=Math.min(100,hp+35);else rifle.reserve=Math.min(rifle.config.reserve*2,rifle.reserve+rifle.config.capacity*2);supplies.items[m.id].taken=true;audio.play('reload');}
 if(m.type==='end'&&!session.host)finish(m.result);
}
$('online').addEventListener('click',()=>{$('settings').hidden=true;$('network-panel').hidden=!$('network-panel').hidden;});
$('host-room').addEventListener('click',()=>session.connect($('server-url').value,'create',seed));
$('join-room').addEventListener('click',()=>session.connect($('server-url').value,'join',seed,$('room-code').value.trim()));
$('resume-room').addEventListener('click',()=>session.resumeSaved());
$('close-room').addEventListener('click',()=>{session.close();peer.visible=false;$('network-status').textContent=t('Сессия закрыта');});
$('server-url').value=location.protocol==='https:'?`wss://${location.host}`:location.protocol==='http:'?`ws://${location.hostname}:3000`:'ws://localhost:3000';
window.addEventListener('game-language-change',()=>{setRunning(false);$('message').textContent=t($('message').textContent);});
