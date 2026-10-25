import * as THREE from 'three';

// Snap projected positions, leaving world geometry and collision shapes intact.
export function createPSX(scene, renderer) {
  const resolution = { value: new THREE.Vector2(480, 270) };
  const enabled = { value: 1 };
  const materials = new Set();
  scene.traverse(object => {
    if (!object.isMesh) return;
    for (const material of Array.isArray(object.material) ? object.material : [object.material]) materials.add(material);
  });
  for (const material of materials) {
    material.onBeforeCompile = shader => {
      shader.uniforms.psxResolution = resolution;
      shader.uniforms.psxEnabled = enabled;
      shader.vertexShader = 'uniform vec2 psxResolution;\nuniform float psxEnabled;\n' + shader.vertexShader;
      shader.vertexShader = shader.vertexShader.replace('#include <project_vertex>', `
        #include <project_vertex>
        if (psxEnabled > 0.5 && gl_Position.w > 0.0) {
          vec2 screen = gl_Position.xy / gl_Position.w;
          vec2 grid = psxResolution * 0.5;
          gl_Position.xy = floor(screen * grid + 0.5) / grid * gl_Position.w;
        }
      `);
    };
    material.customProgramCacheKey = () => 'dead-sector-psx-v1';
    material.needsUpdate = true;
  }
  return {
    resize(width, height, pixels, snapping) {
      const h = pixels ? Math.min(height, 270) : height;
      const w = Math.max(1, Math.round(h * width / height));
      renderer.setPixelRatio(pixels ? 1 : Math.min(devicePixelRatio, 1.5));
      renderer.setSize(w, h, false);
      // Keep wobble independent from resolution and screen pixel density.
      resolution.value.set(270 * width / height, 270);
      enabled.value = snapping ? 1 : 0;
    },
  };
}
