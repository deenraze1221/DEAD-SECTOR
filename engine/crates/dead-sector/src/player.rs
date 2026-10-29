//! First-person player movement with Godot's tuning (`player.gd`): walk and
//! sprint speeds, stamina, acceleration, jump and gravity, on a fixed 60 Hz
//! step against the city's collision boxes.

use crate::city::City;
use glam::{Vec2, Vec3};
use sector_physics::{Aabb, World};
use sector_render::Camera;
use sector_render::view::EYE_HEIGHT;

/// Simulation rate; rendering interpolates between steps.
pub const TICK: f32 = 1.0 / 60.0;
/// The body is a box the size of Godot's capsule (radius 0.32, height 1.8).
pub const HALF_WIDTH: f32 = 0.32;
pub const HEIGHT: f32 = 1.8;
/// Highest ledge (curbs, door sills) walked onto without jumping.
pub const STEP_HEIGHT: f32 = 0.3;
pub const WALK_SPEED: f32 = 4.5;
pub const SPRINT_SPEED: f32 = 7.5;
pub const GRAVITY: f32 = 20.0;
/// `7 / sqrt(2)`: about a 0.61 m jump.
pub const JUMP_SPEED: f32 = 4.949_747;
pub const STAMINA: f32 = 5.0;
/// Mouse look, radians per pixel at sensitivity 1.
pub const LOOK_RADIANS_PER_PIXEL: f32 = 0.002;
const PITCH_DOWN: f32 = std::f32::consts::FRAC_PI_3;
const PITCH_UP: f32 = std::f32::consts::FRAC_PI_2 - 0.02;
/// Below this the player has left the map and is put back at the spawn.
const FALL_LIMIT: f32 = -20.0;
/// Ground box under the whole map, as in Godot (450 × 450 m, top at y = 0).
const GROUND_HALF: f32 = 225.0;
const GRID_CELL: f32 = 4.0;

/// The city's colliders plus the ground.
pub fn collision_world(city: &City) -> World {
    let boxes = city
        .colliders
        .iter()
        .map(|c| Aabb::new(Vec3::new(c.min_x, c.min_y, c.min_z), Vec3::new(c.max_x, c.max_y, c.max_z)));
    let ground = Aabb::new(Vec3::new(-GROUND_HALF, -0.2, -GROUND_HALF), Vec3::new(GROUND_HALF, 0.0, GROUND_HALF));
    World::new(boxes.chain([ground]), GRID_CELL)
}

/// One simulation step's input.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    /// Forward (+) / back (−) and right (+) / left (−), each in −1..=1.
    pub forward: f32,
    pub right: f32,
    pub sprint: bool,
    /// Jump was pressed since the last step.
    pub jump: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub feet: Vec3,
    /// Feet position before the last step, for interpolation.
    pub previous_feet: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    pub stamina: f32,
    pub exhausted: bool,
    spawn: Vec3,
}

impl Player {
    pub fn new(spawn: Vec3) -> Self {
        Player {
            feet: spawn,
            previous_feet: spawn,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            grounded: false,
            stamina: STAMINA,
            exhausted: false,
            spawn,
        }
    }

    pub fn body(&self) -> Aabb {
        Aabb::standing(self.feet, HALF_WIDTH, HEIGHT)
    }

    /// Mouse look (pixels of mouse motion).
    pub fn look(&mut self, dx: f32, dy: f32, sensitivity: f32) {
        self.yaw -= dx * LOOK_RADIANS_PER_PIXEL * sensitivity;
        self.pitch = (self.pitch - dy * LOOK_RADIANS_PER_PIXEL * sensitivity).clamp(-PITCH_DOWN, PITCH_UP);
    }

    /// One fixed step of `TICK` seconds.
    pub fn step(&mut self, world: &World, input: Input) {
        let dt = TICK;
        self.previous_feet = self.feet;
        let mut wish = Vec2::new(input.right, -input.forward);
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }
        let moving = wish.length_squared() > 0.0001;
        if self.exhausted && self.stamina >= STAMINA {
            self.exhausted = false;
        }
        let sprint = input.sprint && moving && self.stamina > 0.0 && !self.exhausted;
        self.stamina = (self.stamina + if sprint { -dt } else { dt }).clamp(0.0, STAMINA);
        if self.stamina == 0.0 {
            self.exhausted = true;
        }

        // Godot: direction = basis * (x, 0, y), then move_toward the target speed.
        let (sin, cos) = self.yaw.sin_cos();
        let direction = Vec2::new(wish.x * cos + wish.y * sin, -wish.x * sin + wish.y * cos);
        let target = direction * if sprint { SPRINT_SPEED } else { WALK_SPEED };
        let acceleration = if moving { 36.0 } else { 55.0 } / 1.15;
        let horizontal = move_toward(Vec2::new(self.velocity.x, self.velocity.z), target, acceleration * dt);
        self.velocity.x = horizontal.x;
        self.velocity.z = horizontal.y;
        if !self.grounded {
            self.velocity.y -= GRAVITY * dt;
        } else if input.jump {
            self.velocity.y = JUMP_SPEED;
        } else {
            self.velocity.y = -0.1;
        }

        let moved = world.move_box(self.body(), self.velocity * dt, STEP_HEIGHT);
        self.feet += moved.offset;
        self.grounded = moved.grounded;
        if moved.grounded || moved.ceiling {
            self.velocity.y = self.velocity.y.min(0.0).max(if moved.grounded { 0.0 } else { f32::MIN });
        }
        if moved.blocked {
            // Slide: keep only the velocity that was not stopped.
            self.velocity.x = moved.offset.x / dt;
            self.velocity.z = moved.offset.z / dt;
        }
        if self.feet.y < FALL_LIMIT || !self.feet.is_finite() {
            *self = Player { yaw: self.yaw, pitch: self.pitch, ..Player::new(self.spawn) };
        }
    }

    /// Eye camera, interpolated `alpha` (0..=1) of the way through the last step.
    pub fn camera(&self, alpha: f32, mut camera: Camera) -> Camera {
        let feet = self.previous_feet.lerp(self.feet, alpha.clamp(0.0, 1.0));
        camera.position = feet + Vec3::Y * EYE_HEIGHT;
        camera.yaw = self.yaw;
        camera.pitch = self.pitch;
        camera
    }
}

fn move_toward(from: Vec2, to: Vec2, delta: f32) -> Vec2 {
    let gap = to - from;
    let length = gap.length();
    if length <= delta || length < 1e-6 { to } else { from + gap / length * delta }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> World {
        World::new([Aabb::new(Vec3::new(-50.0, -0.2, -50.0), Vec3::new(50.0, 0.0, 50.0))], GRID_CELL)
    }

    fn run(player: &mut Player, world: &World, input: Input, ticks: usize) {
        for _ in 0..ticks {
            player.step(world, input);
        }
    }

    #[test]
    fn falls_and_stands_on_the_ground() {
        let world = flat();
        let mut p = Player::new(Vec3::new(0.0, 3.0, 0.0));
        run(&mut p, &world, Input::default(), 120);
        assert!(p.grounded);
        assert!(p.feet.y >= 0.0 && p.feet.y < 0.01, "{:?}", p.feet);
    }

    #[test]
    fn walks_forward_at_walk_speed() {
        let world = flat();
        let mut p = Player::new(Vec3::ZERO);
        run(&mut p, &world, Input::default(), 10);
        let start = p.feet;
        run(&mut p, &world, Input { forward: 1.0, ..Default::default() }, 120);
        // Yaw 0 looks down −Z; after the ramp-up the speed is 4.5 m/s.
        assert!((p.velocity.z + WALK_SPEED).abs() < 1e-3, "{:?}", p.velocity);
        let travelled = start.z - p.feet.z;
        assert!(travelled > 8.0 && travelled < 9.0, "{travelled}");
        assert!((p.feet.x - start.x).abs() < 1e-4);
    }

    #[test]
    fn strafes_relative_to_yaw() {
        let world = flat();
        let mut p = Player::new(Vec3::ZERO);
        p.yaw = std::f32::consts::FRAC_PI_2; // facing −X
        run(&mut p, &world, Input { forward: 1.0, ..Default::default() }, 60);
        assert!(p.feet.x < -3.0 && p.feet.z.abs() < 1e-3, "{:?}", p.feet);
        let mut q = Player::new(Vec3::ZERO);
        run(&mut q, &world, Input { right: 1.0, ..Default::default() }, 60);
        assert!(q.feet.x > 3.0, "{:?}", q.feet);
    }

    #[test]
    fn sprint_is_faster_and_runs_out() {
        let world = flat();
        let mut p = Player::new(Vec3::ZERO);
        run(&mut p, &world, Input { forward: 1.0, sprint: true, ..Default::default() }, 60);
        assert!((p.velocity.z + SPRINT_SPEED).abs() < 1e-3);
        run(&mut p, &world, Input { forward: 1.0, sprint: true, ..Default::default() }, 300);
        assert!(p.exhausted);
        assert!((p.velocity.z + WALK_SPEED).abs() < 1e-3, "{:?}", p.velocity);
    }

    #[test]
    fn jumps_about_sixty_centimetres() {
        let world = flat();
        let mut p = Player::new(Vec3::ZERO);
        run(&mut p, &world, Input::default(), 5);
        p.step(&world, Input { jump: true, ..Default::default() });
        let mut peak: f32 = 0.0;
        for _ in 0..120 {
            p.step(&world, Input::default());
            peak = peak.max(p.feet.y);
        }
        assert!((0.55..0.66).contains(&peak), "{peak}");
        assert!(p.grounded);
    }

    #[test]
    fn walls_block_and_the_player_never_overlaps() {
        let wall = Aabb::new(Vec3::new(-40.0, 0.0, -3.0), Vec3::new(40.0, 3.0, -2.8));
        let world = World::new([Aabb::new(Vec3::new(-50.0, -0.2, -50.0), Vec3::new(50.0, 0.0, 50.0)), wall], GRID_CELL);
        let mut p = Player::new(Vec3::ZERO);
        p.yaw = 0.3;
        for _ in 0..240 {
            p.step(&world, Input { forward: 1.0, sprint: true, ..Default::default() });
            assert!(!world.overlaps(&p.body()));
        }
        assert!(p.feet.z > -2.8 + HALF_WIDTH - 0.01, "{:?}", p.feet);
        assert!(p.feet.x < -3.0, "slides along the wall: {:?}", p.feet);
    }

    #[test]
    fn respawns_after_leaving_the_map() {
        let world = World::new([], GRID_CELL);
        let mut p = Player::new(Vec3::new(1.0, 0.0, 2.0));
        run(&mut p, &world, Input::default(), 200);
        assert!(p.feet.y > FALL_LIMIT);
    }

    #[test]
    fn look_clamps_pitch() {
        let mut p = Player::new(Vec3::ZERO);
        p.look(100.0, -10_000.0, 1.0);
        assert_eq!(p.pitch, PITCH_UP);
        assert!((p.yaw + 0.2).abs() < 1e-6);
        p.look(0.0, 10_000.0, 1.0);
        assert_eq!(p.pitch, -PITCH_DOWN);
    }

    #[test]
    fn camera_interpolates_eye() {
        let mut p = Player::new(Vec3::ZERO);
        p.feet = Vec3::new(1.0, 0.0, 0.0);
        let camera = p.camera(0.5, Camera::player_view(Vec3::ZERO));
        assert_eq!(camera.position, Vec3::new(0.5, EYE_HEIGHT, 0.0));
    }
}
