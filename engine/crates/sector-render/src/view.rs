//! First-person camera.

use glam::camera::rh::{proj::directx, view::look_to_mat4};
use glam::{EulerRot, Mat4, Quat, Vec3, Vec4};

/// First-person camera using Godot conventions: yaw 0 looks down −Z, positive
/// yaw turns left, positive pitch looks up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Vertical field of view in degrees.
    pub fov_y_degrees: f32,
    pub near: f32,
    pub far: f32,
}

/// Player eye height above the feet (`godot/scripts/player.gd`).
pub const EYE_HEIGHT: f32 = 1.65;

impl Camera {
    /// The Godot player camera standing at `feet`: 73° FOV, near 0.04, far 600.
    pub fn player_view(feet: Vec3) -> Self {
        Camera {
            position: feet + Vec3::Y * EYE_HEIGHT,
            yaw: 0.0,
            pitch: 0.0,
            fov_y_degrees: 73.0,
            near: 0.04,
            far: 600.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0) * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        Quat::from_rotation_y(self.yaw) * Vec3::X
    }

    /// View-projection with reversed depth (near = 1, far = 0) for better
    /// precision across a 600 m view distance.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// World → camera space.
    pub fn view(&self) -> Mat4 {
        look_to_mat4(self.position, self.forward(), Vec3::Y)
    }

    /// Camera space → clip space, reversed depth.
    pub fn projection(&self, aspect: f32) -> Mat4 {
        let proj = directx::perspective(self.fov_y_degrees.to_radians(), aspect, self.near, self.far);
        let reverse = Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(0.0, 0.0, -1.0, 0.0), Vec4::new(0.0, 0.0, 1.0, 1.0));
        reverse * proj
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_axes_follow_godot() {
        let mut camera = Camera::player_view(Vec3::ZERO);
        assert!(camera.forward().abs_diff_eq(Vec3::NEG_Z, 1e-6));
        camera.yaw = std::f32::consts::FRAC_PI_2;
        // Positive yaw turns left: forward becomes −X.
        assert!(camera.forward().abs_diff_eq(Vec3::NEG_X, 1e-6));
        assert!(camera.right().abs_diff_eq(Vec3::NEG_Z, 1e-6));
        camera.yaw = 0.0;
        camera.pitch = 0.3;
        assert!(camera.forward().y > 0.0);
    }

    #[test]
    fn depth_is_reversed() {
        let camera = Camera::player_view(Vec3::ZERO);
        let vp = camera.view_proj(16.0 / 9.0);
        let depth = |z: f32| {
            let clip = vp * Vec4::new(0.0, EYE_HEIGHT, z, 1.0);
            clip.z / clip.w
        };
        assert!((depth(-camera.near) - 1.0).abs() < 1e-4);
        assert!(depth(-camera.far).abs() < 1e-4);
        assert!(depth(-1.0) > depth(-100.0));
    }
}
