import * as THREE from 'three';
import { Player, PLAYER, FIXED_STEP } from './player.js';
import { createMap } from './map.js';

const $ = id => document.getElementById(id);
const keys = new Set();
const movementKeys = new Set(['KeyW', 'KeyA', 'KeyS', 'KeyD', 'ShiftLeft', 'ShiftRight', 'Space']);
let renderer, scene, camera, player;
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

function clearInput() { keys.clear(); jumpQueued = false; accumulator = 0; }
function setRunning(value) {
  running = value; clearInput();
  document.body.classList.toggle('playing', value);
  for (const id of ['menu', 'briefing', 'footer']) $(id).hidden = value;
  $('hud').hidden = !value;
  $('restart').hidden = !started;
  $('start').innerHTML = started ? 'ПРОДОЛЖИТЬ <span>↗</span>' : 'ВОЙТИ В СЕКТОР <span>↗</span>';
}

async function enter() {
  try {
    await $('scene').requestPointerLock();
  } catch {
    $('message').textContent = 'Браузер не разрешил захват мыши. Нажмите «Войти в сектор» ещё раз или откройте игру в отдельной вкладке.';
  }
}
$('start').addEventListener('click', enter);
$('restart').addEventListener('click', () => { player.reset(); updateCamera(); enter(); });
document.addEventListener('pointerlockchange', () => {
  const locked = document.pointerLockElement === $('scene');
  if (locked) { started = true; $('message').textContent = 'Пауза · движение остановлено. Продолжить — захватить мышь.'; }
  setRunning(locked);
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
  if (!running || !movementKeys.has(event.code)) return;
  event.preventDefault();
  if (event.code === 'Space' && !keys.has('Space') && !event.repeat) jumpQueued = true;
  keys.add(event.code);
});
document.addEventListener('keyup', event => { keys.delete(event.code); });
document.addEventListener('mousemove', event => { if (running) player.look(event.movementX, event.movementY, sensitivity); });

function updateCamera() {
  camera.position.set(player.position.x, player.position.y + PLAYER.eye, player.position.z);
  camera.rotation.set(player.pitch, player.yaw, 0, 'YXZ');
}
function resize() {
  camera.aspect = innerWidth / innerHeight; camera.updateProjectionMatrix();
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5)); renderer.setSize(innerWidth, innerHeight);
}

// Read-only snapshot for reproducible browser checks, without allowing state mutation.
export function getSnapshot() {
  return { running, started, position: player ? { ...player.position } : null, yaw: player?.yaw, pitch: player?.pitch, grounded: player?.grounded };
}

try {
  renderer = new THREE.WebGLRenderer({ canvas: $('scene'), antialias: false, powerPreference: 'high-performance' });
  renderer.shadowMap.enabled = true; renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.toneMappingExposure = 1.1;
  scene = new THREE.Scene(); camera = new THREE.PerspectiveCamera(78, 1, .1, 130);
  const { colliders, spawn } = createMap(scene); player = new Player(spawn);
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
      }
      $('movement').textContent = !player.grounded ? 'В ВОЗДУХЕ' : keys.has('ShiftLeft') || keys.has('ShiftRight') ? 'БЕГ' : 'ПЕШКОМ';
      updateCamera();
    } else if (!started) {
      camera.position.set(24, 10, 27); camera.lookAt(-5, 1, -8);
    }
    renderer.render(scene, camera);
  }
  renderer.setAnimationLoop(frame);
} catch (error) {
  console.error('DEAD SECTOR initialization failed:', error);
  $('start').disabled = true; $('start').textContent = 'ЗАПУСК НЕДОСТУПЕН';
  $('message').textContent = 'Не удалось запустить 3D-сцену. Нужен браузер с WebGL 2 и включённым аппаратным ускорением.';
}
