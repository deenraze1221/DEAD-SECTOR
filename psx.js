export function createPSX(scene, renderer) {
  return {resize(width,height){const h=Math.min(height,540);renderer.setPixelRatio(1);renderer.setSize(Math.max(1,Math.round(h*width/height)),h,false);}};
}
