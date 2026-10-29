//! Seeded city layout, ported from `godot/scripts/city_generator.gd`.
//!
//! The city is a 10 × 10 grid of 24 m plots. Fixed plots (corners, the store
//! block) and plots that shared street furniture reaches into stay as
//! exported. Every other plot receives a building chosen from its district:
//! each building type in the district appears at least once, the rest are
//! picked with replacement, so a seed changes building counts as well as
//! order. A plot's whole contents (instances, colliders, assets) are copied
//! from the chosen source plot, shifted by the difference of plot centres.

use crate::city::{Bounds, City, Owner};
use crate::interiors;
use glam::{Mat4, Vec3};
use sector_assets::rng::Rng;
use std::collections::BTreeMap;
use std::time::Instant;

pub const PLOTS: usize = 100;
pub const PLOT_SIZE: f32 = 24.0;
/// Corners and the four store-block plots never move.
pub const FIXED_PLOTS: [usize; 8] = [0, 9, 90, 99, 44, 45, 54, 55];

/// Axis-aligned box in world or local space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Aabb { min, max }
    }

    pub fn from_bounds(b: &Bounds) -> Self {
        Aabb::new(Vec3::from(b.min), Vec3::from(b.max))
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Godot's `AABB.intersects`: touching faces do not count.
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.cmplt(other.max).all() && self.max.cmpgt(other.min).all()
    }

    /// Bounds of the box after an affine transform.
    pub fn transformed(&self, m: &Mat4) -> Aabb {
        let center = m.transform_point3(self.center());
        let half = self.size() * 0.5;
        let extent = m.x_axis.truncate().abs() * half.x
            + m.y_axis.truncate().abs() * half.y
            + m.z_axis.truncate().abs() * half.z;
        Aabb::new(center - extent, center + extent)
    }

    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb::new(self.min.min(other.min), self.max.max(other.max))
    }
}

/// Plot centre on the XZ plane.
pub fn plot_center(plot: usize) -> (f32, f32) {
    (-108.0 + PLOT_SIZE * (plot % 10) as f32, -108.0 + PLOT_SIZE * (plot / 10) as f32)
}

/// The plot whose centre is nearest to a point (Godot's `roundi`), or `None` off the grid.
pub fn plot_at(x: f32, z: f32) -> Option<usize> {
    let gx = ((x + 108.0) / PLOT_SIZE).round();
    let gz = ((z + 108.0) / PLOT_SIZE).round();
    if !(0.0..=9.0).contains(&gx) || !(0.0..=9.0).contains(&gz) {
        return None;
    }
    Some(gz as usize * 10 + gx as usize)
}

/// The plot that wholly contains a box, or `None` for shared things (streets, walls).
fn zone_for(b: &Aabb) -> Option<usize> {
    let mid = b.center();
    let plot = plot_at(mid.x, mid.z)?;
    let (cx, cz) = plot_center(plot);
    let inside = b.min.x >= cx - 11.8 && b.max.x <= cx + 11.8 && b.min.z >= cz - 11.8 && b.max.z <= cz + 11.8;
    inside.then_some(plot)
}

/// Shared objects that reach into a plot pin it, so copies never collide with them.
fn pin_intrusions(b: &Aabb, pinned: &mut [bool; PLOTS]) {
    if b.max.y < 0.2 || b.size().x > 25.0 || b.size().z > 25.0 {
        return;
    }
    for (plot, pin) in pinned.iter_mut().enumerate() {
        let (cx, cz) = plot_center(plot);
        if b.min.x < cx + 9.3 && b.max.x > cx - 9.3 && b.min.z < cz + 9.3 && b.max.z > cz - 9.3 {
            *pin = true;
        }
    }
}

/// Building id of a plot (ids run column-major).
pub fn target_id(plot: usize) -> i64 {
    ((plot % 10) * 10 + plot / 10) as i64
}

fn remap_owner(owner: &Owner, target: usize) -> Owner {
    let id = owner.key();
    if let Some(rest) = id.strip_prefix("interior-") {
        let tail = rest.split_once('-').map(|(_, t)| t).unwrap_or("");
        return Owner::Str(format!("interior-{}-{tail}", target_id(target)));
    }
    if id.parse::<i64>().is_ok() {
        return Owner::Int(target_id(target));
    }
    Owner::Str(format!("{id}@{target}"))
}

pub fn matrix(m: &[f32; 16]) -> Mat4 {
    Mat4::from_cols_array(m)
}

pub fn geometry_bounds(city: &City) -> Vec<Aabb> {
    city.geometries
        .iter()
        .map(|g| {
            let mut b = Aabb::new(Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for p in g.position.chunks_exact(3) {
                let v = Vec3::new(p[0], p[1], p[2]);
                b = Aabb::new(b.min.min(v), b.max.max(v));
            }
            b
        })
        .collect()
}

/// What the layout step decided, for logs and tests.
#[derive(Debug, Clone)]
pub struct Layout {
    pub seed: u64,
    pub pinned: [bool; PLOTS],
    /// Source plot copied into each plot.
    pub source_for: [usize; PLOTS],
}

impl Layout {
    pub fn moved_plots(&self) -> usize {
        self.source_for.iter().enumerate().filter(|(t, s)| t != *s).count()
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub layout: Layout,
    pub interiors: interiors::Report,
    pub millis: f64,
}

/// Rebuilds `city` for a seed: new plot layout, then new interiors. Deterministic:
/// the same seed and source city give the same result on every machine.
pub fn generate(city: &mut City, seed: u64) -> Report {
    let started = Instant::now();
    // Furniture prefabs come from the exported interiors, before plots move.
    let library = interiors::Library::capture(city);
    let layout = apply_layout(city, seed);
    let interiors = interiors::rebuild(city, &library, seed);
    Report { layout, interiors, millis: started.elapsed().as_secs_f64() * 1e3 }
}

/// `city_generator.gd::apply`.
pub fn apply_layout(city: &mut City, seed: u64) -> Layout {
    let mut pinned = [false; PLOTS];
    for plot in FIXED_PLOTS {
        pinned[plot] = true;
    }
    let geometry = geometry_bounds(city);
    let mut owner_zone: BTreeMap<String, Option<usize>> = BTreeMap::new();
    for (key, bounds) in &city.source_bounds {
        let b = Aabb::from_bounds(bounds);
        let zone = zone_for(&b);
        owner_zone.insert(key.clone(), zone);
        if zone.is_none() {
            pin_intrusions(&b, &mut pinned);
        }
    }
    let lookup = |owner: Option<&Owner>, fallback: &dyn Fn() -> Aabb| -> Option<usize> {
        match owner.and_then(|o| owner_zone.get(&o.key())) {
            Some(zone) => *zone,
            None => zone_for(&fallback()),
        }
    };

    let mut batch_zones = Vec::with_capacity(city.batches.len());
    for batch in &city.batches {
        let bounds = geometry[batch.geometry];
        let zones: Vec<Option<usize>> = batch
            .transforms
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let world = || bounds.transformed(&matrix(t));
                let zone = lookup(batch.owners.get(i).and_then(Option::as_ref), &world);
                if zone.is_none() {
                    pin_intrusions(&world(), &mut pinned);
                }
                zone
            })
            .collect();
        batch_zones.push(zones);
    }

    // Unpinned plots grouped by district, in building order.
    let building_by_zone: BTreeMap<usize, usize> =
        city.buildings.iter().enumerate().filter_map(|(i, b)| plot_at(b.cx, b.cz).map(|plot| (plot, i))).collect();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for b in &city.buildings {
        let Some(plot) = plot_at(b.cx, b.cz) else { continue };
        if pinned[plot] {
            continue;
        }
        match groups.iter_mut().find(|(d, _)| *d == b.district) {
            Some((_, plots)) => plots.push(plot),
            None => groups.push((b.district.clone(), vec![plot])),
        }
    }

    let mut rng = Rng::new(seed);
    let mut source_for: [usize; PLOTS] = std::array::from_fn(|i| i);
    for (_, group) in &groups {
        let mut by_type: Vec<(&str, Vec<usize>)> = Vec::new();
        for &plot in group {
            let kind = city.buildings[building_by_zone[&plot]].kind.as_str();
            match by_type.iter_mut().find(|(k, _)| *k == kind) {
                Some((_, plots)) => plots.push(plot),
                None => by_type.push((kind, vec![plot])),
            }
        }
        let mut targets = group.clone();
        rng.shuffle(&mut targets);
        for (i, &target) in targets.iter().enumerate() {
            // Every type once, then extra variants with replacement.
            let kind = if i < by_type.len() { i } else { rng.index(by_type.len()) };
            let choices = &by_type[kind].1;
            source_for[target] = choices[rng.index(choices.len())];
        }
    }
    let mut destinations: Vec<Vec<usize>> = vec![Vec::new(); PLOTS];
    for (target, &source) in source_for.iter().enumerate() {
        destinations[source].push(target);
    }
    let shift = |zone: usize, target: usize| {
        let (sx, sz) = plot_center(zone);
        let (tx, tz) = plot_center(target);
        (tx - sx, tz - sz)
    };
    let targets_of = |zone: Option<usize>| -> Vec<Option<usize>> {
        match zone {
            Some(z) => destinations[z].iter().map(|&t| Some(t)).collect(),
            None => vec![None],
        }
    };

    for (batch, zones) in city.batches.iter_mut().zip(&batch_zones) {
        let count = batch.transforms.len();
        let mut transforms = Vec::with_capacity(count);
        let mut colors = Vec::new();
        let mut owners = Vec::with_capacity(count);
        let mut roles = Vec::with_capacity(count);
        for (i, zone) in zones.iter().enumerate() {
            let owner = batch.owners.get(i).cloned().flatten();
            let role = batch.roles.get(i).cloned().unwrap_or_else(|| "model".to_string());
            for target in targets_of(*zone) {
                let mut values = batch.transforms[i];
                let mut new_owner = owner.clone();
                if let (Some(t), Some(z)) = (target, *zone) {
                    let (dx, dz) = shift(z, t);
                    values[12] += dx;
                    values[14] += dz;
                    new_owner = owner.as_ref().map(|o| remap_owner(o, t));
                }
                transforms.push(values);
                owners.push(new_owner);
                roles.push(role.clone());
                if !batch.colors.is_empty() {
                    colors.extend_from_slice(&batch.colors[i * 3..i * 3 + 3]);
                }
            }
        }
        batch.transforms = transforms;
        batch.colors = colors;
        batch.owners = owners;
        batch.roles = roles;
    }

    let mut colliders = Vec::with_capacity(city.colliders.len());
    for c in &city.colliders {
        let b = Aabb::new(Vec3::new(c.min_x, c.min_y, c.min_z), Vec3::new(c.max_x, c.max_y, c.max_z));
        let zone = lookup(c.owner.as_ref(), &|| b);
        for target in targets_of(zone) {
            let mut copy = c.clone();
            if let (Some(t), Some(z)) = (target, zone) {
                let (dx, dz) = shift(z, t);
                copy.min_x += dx;
                copy.max_x += dx;
                copy.min_z += dz;
                copy.max_z += dz;
                copy.owner = c.owner.as_ref().map(|o| remap_owner(o, t));
            }
            colliders.push(copy);
        }
    }
    city.colliders = colliders;

    let originals = std::mem::take(&mut city.buildings);
    city.buildings = (0..PLOTS)
        .map(|target| {
            let source = source_for[target];
            let mut b = originals[building_by_zone[&source]].clone();
            let (dx, dz) = shift(source, target);
            let (cx, cz) = plot_center(target);
            b.x += dx;
            b.z += dz;
            b.cx = cx;
            b.cz = cz;
            b.id = target_id(target);
            b
        })
        .collect();

    let mut assets = Vec::with_capacity(city.source_assets.len());
    let mut bounds = BTreeMap::new();
    for a in &city.source_assets {
        let key = a.id.key();
        let zone = owner_zone.get(&key).copied().flatten();
        for target in targets_of(zone) {
            let mut copy = a.clone();
            let mut moved = city.source_bounds.get(&key).copied();
            if let (Some(t), Some(z)) = (target, zone) {
                let (dx, dz) = shift(z, t);
                copy.x += dx;
                copy.z += dz;
                copy.id = remap_owner(&a.id, t);
                if copy.building.is_some() {
                    copy.building = Some(target_id(t));
                }
                if let Some(m) = moved.as_mut() {
                    m.min[0] += dx;
                    m.max[0] += dx;
                    m.min[2] += dz;
                    m.max[2] += dz;
                }
            }
            if let Some(m) = moved {
                bounds.insert(copy.id.key(), m);
            }
            assets.push(copy);
        }
    }
    city.source_assets = assets;
    city.source_bounds = bounds;
    city.seed = seed;

    Layout { seed, pinned, source_for }
}
