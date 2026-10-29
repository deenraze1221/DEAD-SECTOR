//! Shadow math: cascade splits and fitting, cube-face matrices for point
//! lights, choosing which lights get shadow maps, and culling instances
//! against a light's range. Pure CPU code.

use crate::lights::{MAX_LIGHTS, PointLight};
use crate::settings::SunShadows;
use crate::view::Camera;
use glam::camera::rh::{proj::directx, view::look_to_mat4};
use glam::{Mat4, Vec3, Vec4};
use sector_assets::SceneBuffers;

pub const MAX_CASCADES: usize = 4;
/// Upper limit for the local shadow budget.
pub const MAX_SHADOWED_LIGHTS: usize = 4;
/// How far behind a cascade (towards the sun) casters are still captured.
const CASTER_REACH: f32 = 300.0;
/// Caster culling margin around a cascade's slice, in shadow texels: normal
/// offset (≤ 2.5) plus the soft filter radius (2.5) with headroom.
const CULL_MARGIN_TEXELS: f32 = 8.0;
/// Near plane of point-light shadow faces; keeps a lamp's own housing from
/// shadowing everything.
pub const LOCAL_NEAR: f32 = 0.3;

/// Far edge of each cascade as a fraction of the shadow distance (Godot's
/// PSSM defaults for four splits: 0.1, 0.2, 0.5).
fn split_ratios(count: u32) -> &'static [f32] {
    match count {
        0 | 1 => &[1.0],
        2 => &[0.25, 1.0],
        3 => &[0.15, 0.4, 1.0],
        _ => &[0.1, 0.2, 0.5, 1.0],
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cascade {
    pub view_proj: Mat4,
    /// Far edge as distance along the camera's view direction.
    pub far: f32,
    /// World size of one shadow-map texel.
    pub texel: f32,
    /// World → light space (shared by all cascades).
    pub light_view: Mat4,
    /// Caster culling box in light space: x and y ranges, and near/far
    /// distances along the light direction. It covers the cascade's frustum
    /// slice (the only receivers that sample it) plus a filter margin, and
    /// everything between the slice and the sun.
    pub min: Vec3,
    pub max: Vec3,
}

/// Camera basis vectors (forward, right, up).
fn basis(camera: &Camera) -> (Vec3, Vec3, Vec3) {
    let forward = camera.forward();
    let right = forward.cross(Vec3::Y).normalize_or(camera.right());
    (forward, right, right.cross(forward))
}

/// Light view looking along −`to_sun`.
fn light_view(to_sun: Vec3) -> Mat4 {
    let dir = -to_sun.normalize();
    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    look_to_mat4(Vec3::ZERO, dir, up)
}

/// Fits each cascade's orthographic projection around a bounding sphere of
/// its slice of the camera frustum, snapped to whole texels so shadows do not
/// shimmer as the camera moves.
pub fn fit_cascades(camera: &Camera, aspect: f32, to_sun: Vec3, settings: &SunShadows) -> Vec<Cascade> {
    let (forward, right, up) = basis(camera);
    let tan_half = (camera.fov_y_degrees.to_radians() * 0.5).tan();
    let view = light_view(to_sun);
    let resolution = settings.resolution.max(1) as f32;
    let mut near = camera.near;
    split_ratios(settings.cascades.clamp(1, MAX_CASCADES as u32))
        .iter()
        .map(|ratio| {
            let far = (settings.distance * ratio).max(near + 0.01);
            let mut corners = Vec::with_capacity(8);
            for d in [near, far] {
                let (h, w) = (d * tan_half, d * tan_half * aspect);
                let centre = camera.position + forward * d;
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    corners.push(centre + right * (w * sx) + up * (h * sy));
                }
            }
            near = far;
            let centre = corners.iter().copied().sum::<Vec3>() / corners.len() as f32;
            let radius = corners.iter().map(|c| c.distance(centre)).fold(0.0, f32::max);
            // Quantise the radius so floating-point noise cannot change the texel size.
            let radius = (radius * 16.0).ceil() / 16.0;
            let texel = 2.0 * radius / resolution;
            let local = view.transform_point3(centre);
            let snapped = Vec3::new((local.x / texel).floor() * texel, (local.y / texel).floor() * texel, local.z);
            let projection = directx::orthographic(
                snapped.x - radius,
                snapped.x + radius,
                snapped.y - radius,
                snapped.y + radius,
                -snapped.z - radius - CASTER_REACH,
                -snapped.z + radius,
            );
            // Receivers in this cascade lie inside the slice; shadow rays run along
            // the light, so casters outside the slice's light-space footprint never
            // reach them. The margin covers normal offset and the widest filter.
            let margin = texel * CULL_MARGIN_TEXELS;
            let (mut slice_min, mut slice_max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for corner in &corners {
                let p = view.transform_point3(*corner);
                let p = Vec3::new(p.x, p.y, -p.z);
                slice_min = slice_min.min(p);
                slice_max = slice_max.max(p);
            }
            let min = Vec3::new(slice_min.x - margin, slice_min.y - margin, -snapped.z - radius - CASTER_REACH);
            let max = Vec3::new(slice_max.x + margin, slice_max.y + margin, slice_max.z + margin);
            Cascade { view_proj: projection * view, far, texel, light_view: view, min, max }
        })
        .collect()
}

/// Face order used by the shaders: +X, −X, +Y, −Y, +Z, −Z.
const FACES: [(Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::NEG_Y),
    (Vec3::NEG_X, Vec3::NEG_Y),
    (Vec3::Y, Vec3::Z),
    (Vec3::NEG_Y, Vec3::NEG_Z),
    (Vec3::Z, Vec3::NEG_Y),
    (Vec3::NEG_Z, Vec3::NEG_Y),
];

/// View-projections for the six 90° faces around a point light.
pub fn cube_faces(position: Vec3, range: f32) -> [Mat4; 6] {
    let projection = directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, LOCAL_NEAR, range.max(LOCAL_NEAR + 0.01));
    FACES.map(|(dir, up)| projection * look_to_mat4(position, dir, up))
}

/// Which face of [`cube_faces`] sees direction `d` (same rule as the shader).
#[cfg(test)]
pub fn cube_face(d: Vec3) -> usize {
    let a = d.abs();
    if a.x >= a.y && a.x >= a.z {
        if d.x >= 0.0 { 0 } else { 1 }
    } else if a.y >= a.z {
        if d.y >= 0.0 { 2 } else { 3 }
    } else if d.z >= 0.0 {
        4
    } else {
        5
    }
}

/// Indices of the lights that get shadow maps: the `budget` nearest shadow
/// casters among the first [`MAX_LIGHTS`].
pub fn pick_shadowed(lights: &[PointLight], eye: Vec3, budget: u32) -> Vec<usize> {
    let mut casters: Vec<usize> =
        lights.iter().take(MAX_LIGHTS).enumerate().filter(|(_, l)| l.cast_shadows).map(|(i, _)| i).collect();
    casters.sort_by(|&a, &b| {
        lights[a].position.distance_squared(eye).total_cmp(&lights[b].position.distance_squared(eye))
    });
    casters.truncate((budget as usize).min(MAX_SHADOWED_LIGHTS));
    casters
}

/// World-space bounds of one instance.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    /// xyz = centre, w = radius.
    sphere: Vec4,
    min: Vec3,
    max: Vec3,
}

/// World-space bounds for every instance of every draw, used to cull shadow casters.
pub struct InstanceBounds {
    per_draw: Vec<Vec<Bounds>>,
    /// First instance of each draw in the scene's instance buffer.
    first_instance: Vec<u32>,
}

/// The instances that can cast into one cascade, grouped by draw.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CascadeCasters {
    /// Absolute instance indices, consecutive per draw.
    pub indices: Vec<u32>,
    /// (draw index, first entry in `indices`, count)
    pub runs: Vec<(usize, u32, u32)>,
}

impl InstanceBounds {
    pub fn new(scene: &SceneBuffers) -> Self {
        let per_draw = scene
            .draws
            .iter()
            .map(|d| {
                let indices = &scene.indices[d.first_index as usize..(d.first_index + d.index_count) as usize];
                let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for &i in indices {
                    let p = Vec3::from(scene.vertices[(d.base_vertex + i as i32) as usize].position);
                    min = min.min(p);
                    max = max.max(p);
                }
                let (centre, radius) = ((min + max) * 0.5, (max - min).length() * 0.5);
                scene.instances[d.first_instance as usize..(d.first_instance + d.instance_count) as usize]
                    .iter()
                    .map(|instance| {
                        let m = Mat4::from_cols_array(&instance.model);
                        let axis = |v: Vec4| v.truncate().length();
                        let scale = axis(m.x_axis).max(axis(m.y_axis)).max(axis(m.z_axis));
                        let (mut world_min, mut world_max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                        for corner in 0..8u32 {
                            let pick = |bit: u32, lo: f32, hi: f32| if corner & bit == 0 { lo } else { hi };
                            let local = Vec3::new(pick(1, min.x, max.x), pick(2, min.y, max.y), pick(4, min.z, max.z));
                            let p = m.transform_point3(local);
                            world_min = world_min.min(p);
                            world_max = world_max.max(p);
                        }
                        Bounds {
                            sphere: m.transform_point3(centre).extend(radius * scale),
                            min: world_min,
                            max: world_max,
                        }
                    })
                    .collect()
            })
            .collect();
        InstanceBounds { per_draw, first_instance: scene.draws.iter().map(|d| d.first_instance).collect() }
    }

    /// Casters for each cascade: instances whose bounding sphere overlaps the
    /// cascade's light-space box (its footprint, plus everything between the
    /// box and the sun). Conservative, so the shadow maps are unchanged.
    /// `include` filters draws (e.g. skips transparent ones).
    pub fn cascade_casters(&self, cascades: &[Cascade], include: impl Fn(usize) -> bool) -> Vec<CascadeCasters> {
        let mut out = vec![CascadeCasters::default(); cascades.len()];
        let Some(view) = cascades.first().map(|c| c.light_view) else { return out };
        for (draw, bounds) in self.per_draw.iter().enumerate().filter(|(d, _)| include(*d)) {
            let starts: Vec<u32> = out.iter().map(|c| c.indices.len() as u32).collect();
            for (offset, b) in bounds.iter().enumerate() {
                let local = view.transform_point3(b.sphere.truncate());
                // Distance along the light direction is −z in light view space.
                let point = Vec3::new(local.x, local.y, -local.z);
                let r = b.sphere.w;
                for (cascade, casters) in cascades.iter().zip(out.iter_mut()) {
                    if (point + r).cmpge(cascade.min).all() && (point - r).cmple(cascade.max).all() {
                        casters.indices.push(self.first_instance[draw] + offset as u32);
                    }
                }
            }
            for (casters, start) in out.iter_mut().zip(starts) {
                let count = casters.indices.len() as u32 - start;
                if count > 0 {
                    casters.runs.push((draw, start, count));
                }
            }
        }
        out
    }

    /// For each draw, the instance offsets (relative to the draw) that can
    /// shadow a point light at `centre` with `radius`: those touching its range,
    /// except objects whose box contains the light itself (its own fixture, such
    /// as a lamp post), which would otherwise shadow everything around it.
    pub fn casters_for_light(&self, centre: Vec3, radius: f32) -> Vec<Vec<u32>> {
        self.per_draw
            .iter()
            .map(|bounds| {
                bounds
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.sphere.truncate().distance(centre) < b.sphere.w + radius)
                    .filter(|(_, b)| !(centre.cmpge(b.min).all() && centre.cmple(b.max).all()))
                    .map(|(i, _)| i as u32)
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sector_assets::{Instance, MaterialParams, SceneBuilder, Vertex};

    fn camera() -> Camera {
        Camera::player_view(Vec3::new(3.0, 0.0, -7.0))
    }

    fn sun() -> SunShadows {
        SunShadows { resolution: 2048, cascades: 4, distance: 90.0, filter: crate::settings::ShadowFilter::Pcf }
    }

    #[test]
    fn splits_follow_godot_ratios() {
        let cascades = fit_cascades(&camera(), 16.0 / 9.0, Vec3::new(0.3, 0.8, 0.2), &sun());
        assert_eq!(cascades.iter().map(|c| c.far).collect::<Vec<_>>(), vec![9.0, 18.0, 45.0, 90.0]);
        // Farther cascades cover more ground per texel.
        assert!(cascades.windows(2).all(|w| w[0].texel < w[1].texel));
        let two = fit_cascades(&camera(), 1.0, Vec3::Y, &SunShadows { cascades: 2, ..sun() });
        assert_eq!(two.len(), 2);
    }

    #[test]
    fn cascades_contain_their_frustum_slice_and_casters_towards_the_sun() {
        let cam = camera();
        let to_sun = Vec3::new(0.3, 0.8, 0.2).normalize();
        for (i, cascade) in fit_cascades(&cam, 16.0 / 9.0, to_sun, &sun()).iter().enumerate() {
            let inside = |p: Vec3| {
                let c = cascade.view_proj.project_point3(p);
                c.x.abs() <= 1.0 && c.y.abs() <= 1.0 && (0.0..=1.0).contains(&c.z)
            };
            let point = cam.position + cam.forward() * (cascade.far * 0.9);
            assert!(inside(point), "cascade {i} misses its own slice");
            // A caster 30 m towards the sun projects in front of (smaller depth than) the receiver.
            let caster = point + to_sun * 30.0;
            assert!(inside(caster));
            assert!(cascade.view_proj.project_point3(caster).z < cascade.view_proj.project_point3(point).z);
        }
    }

    #[test]
    fn cascades_are_stable_under_small_camera_moves() {
        let a = fit_cascades(&camera(), 1.5, Vec3::new(0.3, 0.8, 0.2), &sun());
        let mut moved = camera();
        moved.position += Vec3::new(0.001, 0.0, 0.0);
        let b = fit_cascades(&moved, 1.5, Vec3::new(0.3, 0.8, 0.2), &sun());
        // Same texel size; the projection only ever jumps by whole texels.
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.texel, y.texel);
            let shift = (x.view_proj.col(3) - y.view_proj.col(3)).truncate().truncate().abs();
            let texel_ndc = 2.0 / sun().resolution as f32;
            assert!(shift.x < 1e-4 || (shift.x / texel_ndc - (shift.x / texel_ndc).round()).abs() < 1e-2);
        }
    }

    #[test]
    fn cascade_culling_keeps_footprint_and_sunward_casters_only() {
        let mut builder = SceneBuilder::new();
        let material = builder.add_material(MaterialParams::default());
        let v = |x, z| Vertex { position: [x, 0.0, z], normal: [0.0, 1.0, 0.0], uv: [0.0; 2], color: [1.0; 3] };
        let mesh = builder.add_mesh(&[v(-0.5, -0.5), v(0.5, -0.5), v(0.0, 0.5)], None);
        let at = |p: Vec3| Instance { model: Mat4::from_translation(p).to_cols_array(), color: [1.0; 4] };
        let cam = camera();
        let ahead = cam.position + cam.forward() * 5.0;
        let to_sun = Vec3::Y;
        let instances = builder.add_instances(&[
            at(ahead),                              // in the first cascade
            at(ahead + Vec3::new(500.0, 0.0, 0.0)), // far to the side: culled everywhere
            at(ahead + to_sun * 150.0),             // high above (towards the sun): still a caster
            at(ahead - to_sun * 150.0),             // far below the receivers: culled
            at(cam.position - cam.forward() * 3.0), // behind the camera: in the fitted sphere, not the slice
        ]);
        builder.add_draw(0, mesh, None, material, instances);
        let bounds = InstanceBounds::new(&builder.finish());
        let cascades = fit_cascades(&cam, 16.0 / 9.0, to_sun, &sun());
        let casters = bounds.cascade_casters(&cascades, |_| true);
        assert_eq!(casters[0].indices, vec![0, 2]);
        assert_eq!(casters[0].runs, vec![(0, 0, 2)]);
        assert!(casters.iter().all(|c| !c.indices.contains(&1) && !c.indices.contains(&3)));
        assert!(!casters[0].indices.contains(&4), "slice culling should drop casters behind the camera");
        assert!(bounds.cascade_casters(&cascades, |_| false).iter().all(|c| c.runs.is_empty()));
    }

    #[test]
    fn cube_faces_cover_all_directions() {
        let light = Vec3::new(1.0, 4.7, -2.0);
        let faces = cube_faces(light, 11.0);
        for d in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z, Vec3::new(0.6, -0.5, 0.62)] {
            let face = cube_face(d);
            let p = faces[face].project_point3(light + d.normalize() * 5.0);
            assert!(p.x.abs() <= 1.0 && p.y.abs() <= 1.0 && (0.0..=1.0).contains(&p.z), "{d:?} → face {face}");
        }
    }

    #[test]
    fn nearest_casters_within_budget() {
        let light = |x: f32, cast_shadows| PointLight {
            position: Vec3::new(x, 0.0, 0.0),
            range: 5.0,
            color: Vec3::ONE,
            cast_shadows,
        };
        let lights = [light(10.0, true), light(1.0, false), light(3.0, true), light(-2.0, true)];
        assert_eq!(pick_shadowed(&lights, Vec3::ZERO, 2), vec![3, 2]);
        assert_eq!(pick_shadowed(&lights, Vec3::ZERO, 0), Vec::<usize>::new());
        assert_eq!(pick_shadowed(&lights, Vec3::ZERO, 99).len(), 3);
    }

    #[test]
    fn culls_instances_outside_the_light() {
        let mut builder = SceneBuilder::new();
        let material = builder.add_material(MaterialParams::default());
        let v = |x, z| Vertex { position: [x, 0.0, z], normal: [0.0, 1.0, 0.0], uv: [0.0; 2], color: [1.0; 3] };
        let mesh = builder.add_mesh(&[v(-0.5, -0.5), v(0.5, -0.5), v(0.0, 0.5)], None);
        let at = |x: f32, scale: f32| Instance {
            model: Mat4::from_scale_rotation_translation(
                Vec3::splat(scale),
                glam::Quat::IDENTITY,
                Vec3::new(x, 0.0, 0.0),
            )
            .to_cols_array(),
            color: [1.0; 4],
        };
        let instances = builder.add_instances(&[at(0.0, 1.0), at(20.0, 1.0), at(-30.0, 40.0)]);
        builder.add_draw(0, mesh, None, material, instances);
        let bounds = InstanceBounds::new(&builder.finish());
        // The far instance is scaled up enough to reach the light.
        assert_eq!(bounds.casters_for_light(Vec3::new(0.0, 1.0, 0.0), 5.0), vec![vec![0, 2]]);
        assert_eq!(bounds.casters_for_light(Vec3::new(20.0, 1.0, 0.0), 1.0), vec![vec![1]]);
        // A light inside an object's box (a lamp inside its own post) is not shadowed by it.
        assert_eq!(bounds.casters_for_light(Vec3::new(0.1, 0.0, 0.1), 5.0), vec![vec![2]]);
    }
}
