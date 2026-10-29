//! Point lights.

use glam::Vec3;

/// Maximum point lights uploaded per frame.
pub const MAX_LIGHTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    pub range: f32,
    /// Linear colour × energy.
    pub color: Vec3,
    /// May receive a shadow map (the nearest casters within the budget do).
    pub cast_shadows: bool,
}

/// Godot's OmniLight3D falloff (`get_omni_attenuation`, decay 1); the scene
/// shader uses the same curve.
pub fn omni_attenuation(distance: f32, range: f32) -> f32 {
    let mut nd = distance / range;
    nd *= nd;
    nd *= nd;
    nd = (1.0 - nd).max(0.0);
    nd * nd / distance.max(0.0001)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attenuation_matches_godot_curve() {
        assert!((omni_attenuation(1.0, 11.0) - (1.0 - (1.0f32 / 11.0).powi(4)).powi(2)).abs() < 1e-6);
        assert_eq!(omni_attenuation(11.0, 11.0), 0.0);
        assert_eq!(omni_attenuation(20.0, 11.0), 0.0);
        assert!(omni_attenuation(2.0, 11.0) > omni_attenuation(4.0, 11.0));
    }
}
