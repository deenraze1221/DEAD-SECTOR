// Physics is independent of rendering so fixed-step collision rules can be tested.
export const PLAYER = Object.freeze({ radius: 0.32, height: 1.8, eye: 1.65, walk: 4.5, run: 7.5, gravity: 20, jump: 7 });
export const FIXED_STEP = 1 / 120;

export function overlapsBody(position, box) {
  return position.x + PLAYER.radius > box.minX && position.x - PLAYER.radius < box.maxX &&
    position.z + PLAYER.radius > box.minZ && position.z - PLAYER.radius < box.maxZ &&
    position.y + PLAYER.height > box.minY && position.y < box.maxY;
}

export class Player {
  constructor(spawn) { this.spawn = { ...spawn }; this.reset(); }
  reset() {
    this.position = { ...this.spawn }; this.yaw = 0; this.pitch = 0;
    this.velocityY = 0; this.grounded = true; this.distance = 0;
  }
  look(dx, dy, sensitivity) {
    this.yaw -= dx * 0.002 * sensitivity;
    this.pitch = Math.max(-Math.PI / 2 + 0.02, Math.min(Math.PI / 2 - 0.02, this.pitch - dy * 0.002 * sensitivity));
  }
  step(dt, input, colliders) {
    let forward = Number(input.forward) - Number(input.backward);
    let right = Number(input.right) - Number(input.left);
    const magnitude = Math.hypot(forward, right);
    if (magnitude > 0) { forward /= magnitude; right /= magnitude; }
    const speed = input.sprint ? PLAYER.run : PLAYER.walk;
    const dx = (right * Math.cos(this.yaw) - forward * Math.sin(this.yaw)) * speed * dt;
    const dz = (-right * Math.sin(this.yaw) - forward * Math.cos(this.yaw)) * speed * dt;
    const oldX = this.position.x, oldZ = this.position.z;
    // Resolve axes separately: the player slides along walls instead of sticking.
    for (const [axis, delta, min, max] of [['x', dx, 'minX', 'maxX'], ['z', dz, 'minZ', 'maxZ']]) {
      this.position[axis] += delta;
      for (const box of colliders) {
        if (!overlapsBody(this.position, box)) continue;
        if (delta > 0) this.position[axis] = box[min] - PLAYER.radius;
        else if (delta < 0) this.position[axis] = box[max] + PLAYER.radius;
      }
    }
    this.distance += Math.hypot(this.position.x - oldX, this.position.z - oldZ);
    if (input.jump && this.grounded) { this.velocityY = PLAYER.jump; this.grounded = false; }
    const previousY = this.position.y;
    this.velocityY -= PLAYER.gravity * dt;
    this.position.y += this.velocityY * dt;
    this.grounded = false;
    for (const box of colliders) {
      if (!overlapsBody(this.position, box)) continue;
      if (this.velocityY <= 0 && previousY >= box.maxY - 0.001) {
        this.position.y = box.maxY; this.velocityY = 0; this.grounded = true;
      } else if (this.velocityY > 0 && previousY + PLAYER.height <= box.minY + 0.001) {
        this.position.y = box.minY - PLAYER.height; this.velocityY = 0;
      }
    }
    if (this.position.y <= 0) { this.position.y = 0; this.velocityY = 0; this.grounded = true; }
  }
}
