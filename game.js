import * as THREE from 'three';
import { Player, PLAYER, FIXED_STEP } from './player.js';
import { createMap } from './map.js';
import { createPSX } from './psx.js';
import { Rifle } from './weapons.js';
import { Enemies } from './enemies.js';
import { GameAudio } from './audio.js';

const $ = id => document.getElementById(id);
const keys = new Set();
const movementKeys = new Set(['KeyW', 'KeyA', 'KeyS', 'KeyD', 'ShiftLeft', 'ShiftRight', 'Space']);
let renderer, scene, camera, player;
let psx, rifle, enemies, exit;
const audio = new GameAudio();
let hp=100, outcome='', shooting=false, triggerPressed=false, aiming=false, hitTime=0, hurtTime=0, elapsed=0;
const effects = { pixels: true, snapping: true };
for (const [id, key] of [['pixels', 'pixels'], ['vertex-snap', 'snapping']]) {
  try {
    const saved = localStorage.getItem(`dead-sector:${key}`);
    if (saved !== null) effects[key] = saved !== 'false';
  } catch { /* Preferences are optional. */ }
  $(id).checked = effects[key];
  $(id).addEventListener('change', event => {
    effects[key] = event.target.checked;
    try { localStorage.setItem(`dead-sector:${key}`, String(effects[key])); } catch { /* Optional. */ }
    if (psx) resize();
  });
}
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
  $('start').innerHTML = outcome ? 'НОВАЯ ЗАЧИСТКА <span>↗</span>' : started ? 'ПРОДОЛЖИТЬ <span>↗</span>' : 'ВОЙТИ В СЕКТОР <span>↗</span>';
}

function resetCombat(){player.reset();rifle.reset();enemies.reset();hp=100;outcome='';hitTime=0;hurtTime=0;elapsed=0;updateCamera();}
function finish(result){if(outcome)return;outcome=result;pause();$('message').textContent=result==='win'?'СЕКТОР ЗАЧИЩЕН. Вы добрались до выхода.':'ВЫ ПОГИБЛИ. Начните зачистку заново.';}
async function enter() {
  if(outcome)resetCombat();
  audio.unlock();
  try {
    await $('scene').requestPointerLock();
  } catch {
    $('message').textContent = 'Браузер не разрешил захват мыши. Нажмите «Войти в сектор» ещё раз или откройте игру в отдельной вкладке.';
  }
}
$('start').addEventListener('click', enter);
$('restart').addEventListener('click', () => { resetCombat(); enter(); });
document.addEventListener('pointerlockchange', () => {
  const locked = document.pointerLockElement === $('scene');
  if (locked) { started = true; $('message').textContent = 'Пауза · движение остановлено. Продолжить — захватить мышь.'; }
  setRunning(locked);
  if(outcome) $('message').textContent=outcome==='win'?'СЕКТОР ЗАЧИЩЕН. Вы добрались до выхода.':'ВЫ ПОГИБЛИ. Начните зачистку заново.';
  if (!locked) $('start').focus({ preventScroll: true });
});
document.addEventListener('pointerlockerror', () => {
  $('message').textContent = 'Не удалось захватить мышь. Повторите вход в сектор в отдельной вкладке браузера.';
});
function pause() { if (document.pointerLockElement) document.exitPointerLock(); setRunning(false); }
window.addEventListener('blur', pause);
document.addEventListener('visibilitychange', () => { if (document.hidden) pause(); });
document.addEventListener('keydown', event => {
  if (event.code === 'Escape' && running) { pause(); return; }
  if(running && event.code==='KeyR'){rifle.reload();event.preventDefault();return;}
  if(running && event.code==='KeyE' && !enemies.alive && Math.hypot(player.position.x,player.position.z+32)<2.5 && player.position.y<1){finish('win');return;}
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
  psx.resize(innerWidth, innerHeight, effects.pixels, effects.snapping);
}

// Read-only snapshot for reproducible browser checks, without allowing state mutation.
export function getSnapshot() {
  return { running, started, position: player ? { ...player.position } : null, yaw: player?.yaw, pitch: player?.pitch, grounded: player?.grounded, hp, outcome, ammo:rifle?.ammo, reserve:rifle?.reserve, reloading:rifle?.reloadTime>0, weapon:rifle?.config.id, capacity:rifle?.config.capacity, remaining:enemies?.alive, enemies:enemies?.items.map(e=>({hp:e.hp,x:e.rig.position.x,z:e.rig.position.z})) };
}

try {
  renderer = new THREE.WebGLRenderer({ canvas: $('scene'), antialias: false, powerPreference: 'high-performance' });
  renderer.shadowMap.enabled = true; renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.1;
  scene = new THREE.Scene(); camera = new THREE.PerspectiveCamera(78, 1, .1, 130);
  const { colliders, spawn } = createMap(scene); player = new Player(spawn);
  rifle=new Rifle(audio);enemies=new Enemies(scene,colliders,audio);
  exit=new THREE.Mesh(new THREE.BoxGeometry(3,3,.15),new THREE.MeshBasicMaterial({color:'#873d2b'}));exit.position.set(0,1.5,-34.35);scene.add(exit);
  psx = createPSX(scene, renderer);
  updateCamera(); resize(); window.addEventListener('resize', resize);
  $('scene').addEventListener('webglcontextlost', event => {
    event.preventDefault(); pause(); $('start').disabled = true;
    $('message').textContent = 'Графический контекст потерян. Перезагрузите страницу, чтобы восстановить сектор.';
  });
  $('start').disabled = false; setRunning(false);
  function frame(time) {
    const delta = Math.min(Math.max((time - previousTime) / 1000, 0), .1); previousTime = time;
    if (running) {
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
          if(enemies.shoot(camera.position,direction,rifle.config.damage))hitTime=.15;
          player.pitch=Math.min(Math.PI/2-.02,player.pitch+.008);
        }
        triggerPressed=false;
        enemies.step(FIXED_STEP,player,damage=>{hp=Math.max(0,hp-damage);hurtTime=.3;audio.play('hurt');if(!hp)finish('dead');});
        hitTime=Math.max(0,hitTime-FIXED_STEP);hurtTime=Math.max(0,hurtTime-FIXED_STEP);
        if(!running)break;
      }
      $('movement').textContent = !player.grounded ? 'В ВОЗДУХЕ' : keys.has('ShiftLeft') || keys.has('ShiftRight') ? 'БЕГ' : 'ПЕШКОМ';
      const fov=aiming?55:78;if(camera.fov!==fov){camera.fov=fov;camera.updateProjectionMatrix();}
      updateCamera();
    } else if (!started) {
      camera.position.set(24, 10, 27); camera.lookAt(-5, 1, -8);
    }
    $('health').textContent=hp;$('health-bar').style.width=`${hp*1.35}px`;
    $('ammo').textContent=rifle.ammo;$('reserve').textContent=rifle.reserve;
    $('weapon-state').textContent=rifle.reloadTime?'ПЕРЕЗАРЯДКА…':!rifle.ammo?'R — ПЕРЕЗАРЯДИТЬ':rifle.config.label;
    $('objective').textContent=enemies.alive?`ЗАЧИСТИТЕ СЕКТОР · ОСТАЛОСЬ ${enemies.alive}`:'ВЫХОД ОТКРЫТ · СЕВЕРНЫЕ ВОРОТА · E';
    $('hit-marker').hidden=hitTime<=0;document.body.classList.toggle('hurt',hurtTime>0&&running);
    exit.material.color.set(enemies.alive?'#873d2b':'#afdb65');
    rifle.pose(elapsed,keys.size>0,aiming);
    renderer.autoClear=true;renderer.render(scene, camera);
    if(started&&!outcome){renderer.autoClear=false;renderer.clearDepth();renderer.render(rifle.scene,rifle.camera);}
  }
  renderer.setAnimationLoop(frame);
} catch (error) {
  console.error('DEAD SECTOR initialization failed:', error);
  $('start').disabled = true; $('start').textContent = 'ЗАПУСК НЕДОСТУПЕН';
  $('message').textContent = 'Не удалось запустить 3D-сцену. Нужен браузер с WebGL 2 и включённым аппаратным ускорением.';
}
