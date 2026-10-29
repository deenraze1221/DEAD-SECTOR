//! Local lights: the store lamp and the street lamps nearest the player
//! (`godot/scripts/atmosphere.gd`, `main.gd::setup_environment()`).

use crate::city::Building;
use glam::Vec3;
use sector_assets::color::linear_hex;
use sector_render::PointLight;

fn linear(hex: &str) -> Vec3 {
    Vec3::from(linear_hex(hex))
}

/// City quadrant 0..3 used for blackouts (`battle.gd::district_of`).
pub fn district_of(x: f32, z: f32) -> u8 {
    match (x < 0.0, z < 0.0) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    }
}

/// The warm light over the SECTOR MART terminal.
pub fn store_light(terminal_x: f32, terminal_z: f32) -> PointLight {
    PointLight { position: Vec3::new(terminal_x, 2.5, terminal_z), range: 9.0, color: linear("#d5b882") * 1.3 }
}

/// Up to `count` lit street lamps within 35 m of `eye`, nearest first, skipping
/// lamps in a blacked-out district. Lights hang 4.7 m above the lamp base.
pub fn street_lights(lamps: &[Vec3], eye: Vec3, count: usize, power_cut: Option<u8>, energy: f32) -> Vec<PointLight> {
    let mut live: Vec<Vec3> = lamps.iter().copied().filter(|l| Some(district_of(l.x, l.z)) != power_cut).collect();
    live.sort_by(|a, b| a.distance_squared(eye).total_cmp(&b.distance_squared(eye)));
    let color = linear("#d4bd86") * energy;
    live.into_iter()
        .take(count)
        .filter(|l| l.distance(eye) < 35.0)
        .map(|l| PointLight { position: l + Vec3::Y * 4.7, range: 11.0, color })
        .collect()
}

/// Light pools under every lamp (`atmosphere.gd`): 5.4 m wide, hidden in a blackout.
pub fn pool_decals(lamps: &[Vec3], power_cut: Option<u8>) -> Vec<[f32; 4]> {
    lamps
        .iter()
        .map(|l| {
            let radius = if Some(district_of(l.x, l.z)) == power_cut { 0.0 } else { 2.7 };
            [l.x, l.y + 0.035, l.z, radius]
        })
        .collect()
}

/// True when `p` is under or right next to a building on its plot, where Godot
/// stops raindrops hitting the lens (`combat_fx.gd`).
pub fn sheltered(buildings: &[Building], p: Vec3) -> bool {
    let plot = |v: f32| ((v + 108.0) / 24.0).round();
    let (px, pz) = (plot(p.x), plot(p.z));
    buildings.iter().any(|b| {
        plot(b.cx) == px
            && plot(b.cz) == pz
            && (p.x - b.x).abs() < b.w / 2.0 + 4.0
            && (p.z - b.z).abs() < b.d / 2.0 + 4.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools_follow_blackout() {
        let lamps = [Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 0.0, 5.0)];
        assert_eq!(pool_decals(&lamps, None), vec![[-5.0, 0.035, -5.0, 2.7], [5.0, 0.035, 5.0, 2.7]]);
        assert_eq!(pool_decals(&lamps, Some(3))[1][3], 0.0);
    }

    #[test]
    fn districts_are_quadrants() {
        assert_eq!([(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)].map(|(x, z)| district_of(x, z)), [0, 1, 2, 3]);
    }

    #[test]
    fn lamp_selection_is_nearest_live_and_in_range() {
        let lamps =
            [Vec3::new(5.0, 0.0, 5.0), Vec3::new(-3.0, 0.0, 3.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(60.0, 0.0, 60.0)];
        let lights = street_lights(&lamps, Vec3::ZERO, 8, None, 0.9);
        assert_eq!(lights.iter().map(|l| l.position.x).collect::<Vec<_>>(), vec![1.0, -3.0, 5.0]);
        assert_eq!(lights[0].position.y, 4.7);
        assert_eq!(street_lights(&lamps, Vec3::ZERO, 2, None, 0.9).len(), 2);
        // District 1 (x < 0, z ≥ 0) blacked out.
        let cut = street_lights(&lamps, Vec3::ZERO, 8, Some(1), 0.9);
        assert!(cut.iter().all(|l| l.position.x > 0.0));
        assert_eq!(cut.len(), 2);
    }

    #[test]
    fn shelter_uses_plot_building_with_margin() {
        let building = Building {
            kind: "shop".into(),
            x: 0.0,
            z: 0.0,
            w: 10.0,
            d: 10.0,
            h: 8.0,
            cx: 12.0,
            cz: 12.0,
            district: "market".into(),
        };
        // Plot centred at (12, 12) covers x, z in [0, 24).
        assert!(sheltered(std::slice::from_ref(&building), Vec3::new(8.5, 0.0, 1.0)));
        assert!(!sheltered(std::slice::from_ref(&building), Vec3::new(9.5, 0.0, 1.0)));
        assert!(!sheltered(std::slice::from_ref(&building), Vec3::new(-1.0, 0.0, 1.0)));
    }
}
