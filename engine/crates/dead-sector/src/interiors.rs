//! Seeded interiors, ported from `godot/scripts/interior_layout.gd`.
//!
//! Furniture prefabs (visible parts plus colliders, relative to the item's
//! anchor) are captured from the exported interiors. Each building except the
//! store then gets a random floor plan: plaster partitions with doorways, a
//! kept-clear corridor, and rooms furnished from a per-building-type program.
//! An item is placed against a room wall only where its padded footprint
//! misses every placed item and every doorway/corridor box.

use crate::city::{City, Collider, Owner};
use crate::citygen::{Aabb, matrix};
use glam::{Mat4, Vec3};
use sector_assets::color::parse_hex_color;
use sector_assets::rng::Rng;
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

/// Prefabs kept per furniture type.
const PREFABS_PER_TYPE: usize = 5;
const PLACE_ATTEMPTS: usize = 12;
const WALL_HEIGHT: f32 = 3.12;
/// The store keeps its exported interior.
const STORE_ID: i64 = 55;
const RESIDENTIAL: [&str; 4] = ["apartment", "hotel", "motel", "residentialHighrise"];
const SERVICE: [&str; 8] =
    ["garage", "warehouse", "workshop", "utility", "firestation", "freightDepot", "observatory", "communicationsTower"];

#[derive(Debug, Clone)]
struct Part {
    geometry: usize,
    materials: Vec<usize>,
    local: Mat4,
    color: [f32; 3],
}

#[derive(Debug, Clone)]
struct Prefab {
    w: f32,
    d: f32,
    parts: Vec<Part>,
    boxes: Vec<Aabb>,
}

/// Furniture prefabs by type, captured from the exported city.
#[derive(Debug, Clone, Default)]
pub struct Library {
    types: Vec<(String, Vec<Prefab>)>,
}

impl Library {
    pub fn capture(city: &City) -> Library {
        let anchor = |a: &crate::city::SourceAsset| {
            Mat4::from_translation(Vec3::new(a.x, a.y, a.z)) * Mat4::from_rotation_y(a.yaw)
        };
        let assets: HashMap<String, &crate::city::SourceAsset> =
            city.source_assets.iter().map(|a| (a.id.key(), a)).filter(|(id, _)| id.starts_with("interior-")).collect();
        // Parts per item, in order of first appearance.
        let mut order: Vec<String> = Vec::new();
        let mut parts: HashMap<String, Vec<Part>> = HashMap::new();
        for batch in &city.batches {
            for (i, owner) in batch.owners.iter().enumerate() {
                let Some(id) = owner.as_ref().map(Owner::key) else { continue };
                let Some(item) = assets.get(&id) else { continue };
                let color = if batch.colors.len() > i * 3 + 2 {
                    [batch.colors[i * 3], batch.colors[i * 3 + 1], batch.colors[i * 3 + 2]]
                } else {
                    [1.0; 3]
                };
                let local = anchor(item).inverse() * matrix(&batch.transforms[i]);
                let entry = parts.entry(id.clone()).or_insert_with(|| {
                    order.push(id.clone());
                    Vec::new()
                });
                entry.push(Part { geometry: batch.geometry, materials: batch.materials.clone(), local, color });
            }
        }
        let mut boxes: HashMap<String, Vec<Aabb>> = HashMap::new();
        for c in &city.colliders {
            let Some(id) = c.owner.as_ref().map(Owner::key) else { continue };
            if let Some(item) = assets.get(&id) {
                let world = Aabb::new(Vec3::new(c.min_x, c.min_y, c.min_z), Vec3::new(c.max_x, c.max_y, c.max_z));
                boxes.entry(id).or_default().push(world.transformed(&anchor(item).inverse()));
            }
        }
        let mut library = Library::default();
        for id in order {
            let item = assets[&id];
            let prefab = Prefab {
                w: item.w,
                d: item.d,
                parts: parts.remove(&id).unwrap(),
                boxes: boxes.remove(&id).unwrap_or_default(),
            };
            match library.types.iter_mut().find(|(k, _)| *k == item.kind) {
                Some((_, list)) if list.len() >= PREFABS_PER_TYPE => {}
                Some((_, list)) => list.push(prefab),
                None => library.types.push((item.kind.clone(), vec![prefab])),
            }
        }
        library
    }

    fn get(&self, kind: &str) -> Option<&[Prefab]> {
        self.types.iter().find(|(k, _)| k == kind).map(|(_, list)| list.as_slice())
    }

    pub fn type_count(&self) -> usize {
        self.types.len()
    }
}

/// A furnished room on the XZ plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Room {
    pub building: i64,
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Room {
    fn size(&self) -> (f32, f32) {
        (self.max_x - self.min_x, self.max_z - self.min_z)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub rooms: Vec<Room>,
    /// Doorways and corridors that must stay clear, per building.
    pub reserved: Vec<(i64, Aabb)>,
    /// Padded footprints of placed furniture, per building.
    pub footprints: Vec<(i64, Aabb)>,
    pub furniture: usize,
    pub floorplans: Vec<(i64, i32)>,
}

struct Builder<'a> {
    city: &'a mut City,
    library: &'a Library,
    rng: Rng,
    unit_box: usize,
    plaster: usize,
    /// Batch index of each (geometry, materials) for new instances.
    fresh: HashMap<(usize, Vec<usize>), usize>,
    occupied: Vec<Aabb>,
    reserved: Vec<Aabb>,
    report: Report,
}

/// Replaces every interior except the store's with a seeded one.
pub fn rebuild(city: &mut City, library: &Library, seed: u64) -> Report {
    let unit_box = city.geometries.iter().position(|g| g.position.len() == 72 && g.index.len() == 36).unwrap_or(0);
    let plaster_color = parse_hex_color("#a6a58c").unwrap();
    let plaster = city.materials.iter().position(|m| m.color == plaster_color).unwrap_or(0);
    remove_generated_content(city);

    let buildings = city.buildings.clone();
    let mut builder = Builder {
        city,
        library,
        rng: Rng::new(seed ^ 0x524f_4f4d),
        unit_box,
        plaster,
        fresh: HashMap::new(),
        occupied: Vec::new(),
        reserved: Vec::new(),
        report: Report::default(),
    };
    for b in buildings.iter().filter(|b| b.kind != "grocery") {
        builder.building(b);
    }
    builder.report
}

/// Drops exported furniture and partitions, except the store's.
fn remove_generated_content(city: &mut City) {
    let keep = |owner: Option<&Owner>, role: &str| {
        let key = owner.map(Owner::key).unwrap_or_default();
        let store_interior = format!("interior-{STORE_ID}-");
        !((key.starts_with("interior-") && !key.starts_with(&store_interior))
            || (role == "partition" && key != STORE_ID.to_string()))
    };
    for batch in &mut city.batches {
        let count = batch.transforms.len();
        let mut transforms = Vec::with_capacity(count);
        let mut colors = Vec::new();
        let mut owners = Vec::with_capacity(count);
        let mut roles = Vec::with_capacity(count);
        for i in 0..count {
            let owner = batch.owners.get(i).cloned().flatten();
            let role = batch.roles.get(i).cloned().unwrap_or_default();
            if !keep(owner.as_ref(), &role) {
                continue;
            }
            transforms.push(batch.transforms[i]);
            if !batch.colors.is_empty() {
                colors.extend_from_slice(&batch.colors[i * 3..i * 3 + 3]);
            }
            owners.push(owner);
            roles.push(role);
        }
        batch.transforms = transforms;
        batch.colors = colors;
        batch.owners = owners;
        batch.roles = roles;
    }
    city.colliders.retain(|c| {
        let key = c.owner.as_ref().map(Owner::key).unwrap_or_default();
        match c.kind.as_str() {
            "furniture" => key.starts_with(&format!("interior-{STORE_ID}-")),
            "partition" => key == STORE_ID.to_string(),
            _ => true,
        }
    });
}

impl Builder<'_> {
    fn building(&mut self, b: &crate::city::Building) {
        self.occupied.clear();
        self.reserved.clear();
        let id = b.id;
        let left = b.x - b.w / 2.0 + 0.3;
        let right = b.x + b.w / 2.0 - 0.3;
        let back = b.z - b.d / 2.0 + 0.3;
        let front = b.z + b.d / 2.0 - 0.3;
        let plan = self.rng.range_i32(0, 2);
        self.report.floorplans.push((id, plan));
        let room = |min_x: f32, min_z: f32, w: f32, d: f32| Room {
            building: id,
            min_x,
            max_x: min_x + w,
            min_z,
            max_z: min_z + d,
        };
        let clear = |x: f32, z: f32, w: f32, d: f32| Aabb::new(Vec3::new(x, 0.0, z), Vec3::new(x + w, 3.0, z + d));
        let mut rooms = Vec::new();
        if plan == 0 {
            // Two side rooms off a central corridor, plus a small back room.
            let opening = b.z + self.rng.range_f32(-0.8, 0.8);
            for side in [-1.0, 1.0] {
                let x = b.x + side * 0.95;
                self.wall(x, (back + opening - 0.8) / 2.0, 0.12, opening - 0.8 - back, id);
                self.wall(x, (opening + 0.8 + front) / 2.0, 0.12, front - opening - 0.8, id);
                let (min_x, max_x) = if side < 0.0 { (left, x - 0.1) } else { (x + 0.1, right) };
                let r = room(min_x, back, max_x - min_x, front - back);
                rooms.push(r);
                self.reserved.push(clear(r.min_x, opening - 0.6, max_x - min_x, 1.2));
            }
            // The corridor starts at the back-room walls' face (Godot's box began 1 cm inside them).
            let corridor = back + 1.7 + 0.06;
            self.reserved.push(clear(b.x - 0.8, corridor, 1.6, front + 0.8 - corridor));
            rooms.push(room(b.x - 0.85, back, 1.7, 1.6));
            self.wall(b.x - 0.7, back + 1.7, 0.5, 0.12, id);
            self.wall(b.x + 0.7, back + 1.7, 0.5, 0.12, id);
        } else {
            // Front and back halves split by a wall with a gap; plan 1 adds a washroom.
            let split = back + (front - back) * if plan == 1 { 0.47 } else { 0.32 };
            let gap_x = b.x + self.rng.range_f32(-0.6, 0.6);
            self.wall((left + gap_x - 0.85) / 2.0, split, gap_x - 0.85 - left, 0.12, id);
            self.wall((right + gap_x + 0.85) / 2.0, split, right - gap_x - 0.85, 0.12, id);
            rooms.push(room(left, back, right - left, split - back - 0.1));
            rooms.push(room(left, split + 0.1, right - left, front - split - 0.1));
            self.reserved.push(clear(gap_x - 0.75, back, 1.5, front - back + 0.6));
            if plan == 1 {
                let depth = rooms[0].max_z - rooms[0].min_z;
                rooms[0] = room(left + 2.1, back, right - left - 2.1, depth);
                rooms.push(room(left, back, 1.95, depth.min(1.8)));
                self.wall(left + 2.05, back + 0.9, 0.12, 1.8, id);
                self.wall(left + 0.25, back + 1.85, 0.5, 0.12, id);
                self.wall(left + 1.8, back + 1.85, 0.5, 0.12, id);
            }
        }
        let residential = RESIDENTIAL.contains(&b.kind.as_str());
        let service = SERVICE.contains(&b.kind.as_str());
        for (index, r) in rooms.iter().enumerate() {
            self.report.rooms.push(*r);
            let program: &[&str] = match index {
                2 => &["toilet", "sink"],
                0 if residential => &["bed", "cabinet", "tvstand", "stool"],
                _ if residential => &["sofa", "counter", "fridge", "table", "sink"],
                _ if service => &["workbench", "shelf", "locker", "crate"],
                _ => &["desk", "shelf", "cabinet", "table", "sofa"],
            };
            for kind in program {
                for _ in 0..PLACE_ATTEMPTS {
                    if self.place(kind, r) {
                        break;
                    }
                }
            }
        }
        self.report.reserved.extend(self.reserved.iter().map(|r| (id, *r)));
        self.report.footprints.extend(self.occupied.iter().map(|r| (id, *r)));
    }

    /// One attempt to put a `kind` item against a wall of `room`.
    fn place(&mut self, kind: &str, room: &Room) -> bool {
        let library = self.library;
        let Some(prefabs) = library.get(kind) else { return false };
        let prefab = &prefabs[self.rng.index(prefabs.len())];
        let side = self.rng.range_i32(0, 3);
        let yaw = side as f32 * FRAC_PI_2;
        let (sx, sz) = if side % 2 == 0 { (prefab.w, prefab.d) } else { (prefab.d, prefab.w) };
        let (room_w, room_d) = room.size();
        if sx > room_w - 0.3 || sz > room_d - 0.3 {
            return false;
        }
        let mut x = self.rng.range_f32(room.min_x + sx / 2.0 + 0.12, room.max_x - sx / 2.0 - 0.12);
        let mut z = self.rng.range_f32(room.min_z + sz / 2.0 + 0.12, room.max_z - sz / 2.0 - 0.12);
        match side {
            0 => z = room.min_z + sz / 2.0 + 0.12,
            1 => x = room.min_x + sx / 2.0 + 0.12,
            2 => z = room.max_z - sz / 2.0 - 0.12,
            _ => x = room.max_x - sx / 2.0 - 0.12,
        }
        let footprint = Aabb::new(
            Vec3::new(x - sx / 2.0 - 0.08, 0.0, z - sz / 2.0 - 0.08),
            Vec3::new(x + sx / 2.0 + 0.08, 3.0, z + sz / 2.0 + 0.08),
        );
        if self.blocked(&footprint) {
            return false;
        }
        self.occupied.push(footprint);
        let anchor = Mat4::from_translation(Vec3::new(x, 0.0, z)) * Mat4::from_rotation_y(yaw);
        self.instantiate(prefab, &anchor, room.building);
        if ["desk", "table", "workbench", "counter"].contains(&kind) && library.get("chair").is_some() {
            let seat = anchor.transform_point3(Vec3::new(0.0, 0.0, prefab.d / 2.0 + 0.52));
            self.place_chair(room, seat, yaw + PI);
        }
        true
    }

    fn place_chair(&mut self, room: &Room, point: Vec3, yaw: f32) {
        let library = self.library;
        let prefabs = library.get("chair").unwrap();
        let prefab = &prefabs[self.rng.index(prefabs.len())];
        let (sx, sz) = if yaw.sin().abs() > 0.5 { (prefab.d, prefab.w) } else { (prefab.w, prefab.d) };
        let footprint = Aabb::new(
            Vec3::new(point.x - sx / 2.0 - 0.06, 0.0, point.z - sz / 2.0 - 0.06),
            Vec3::new(point.x + sx / 2.0 + 0.06, 3.0, point.z + sz / 2.0 + 0.06),
        );
        if footprint.min.x < room.min_x + 0.08
            || footprint.max.x > room.max_x - 0.08
            || footprint.min.z < room.min_z + 0.08
            || footprint.max.z > room.max_z - 0.08
            || self.blocked(&footprint)
        {
            return;
        }
        self.occupied.push(footprint);
        let anchor = Mat4::from_translation(point) * Mat4::from_rotation_y(yaw);
        self.instantiate(prefab, &anchor, room.building);
    }

    fn blocked(&self, footprint: &Aabb) -> bool {
        self.occupied.iter().chain(&self.reserved).any(|b| b.intersects(footprint))
    }

    fn instantiate(&mut self, prefab: &Prefab, anchor: &Mat4, building: i64) {
        let id = Owner::Str(format!("fresh-{building}-{}", self.report.furniture));
        for part in &prefab.parts {
            self.append(part.geometry, &part.materials, *anchor * part.local, part.color, &id);
        }
        for b in &prefab.boxes {
            self.collider(b.transformed(anchor), "furniture", id.clone());
        }
        self.report.furniture += 1;
    }

    /// A plaster partition with its collider; skipped when degenerate.
    fn wall(&mut self, x: f32, z: f32, w: f32, d: f32, building: i64) {
        if w <= 0.01 || d <= 0.01 {
            return;
        }
        let t =
            Mat4::from_translation(Vec3::new(x, WALL_HEIGHT / 2.0, z)) * Mat4::from_scale(Vec3::new(w, WALL_HEIGHT, d));
        let owner = Owner::Str(format!("partition-{building}"));
        self.append(self.unit_box, &[self.plaster], t, [1.0; 3], &owner);
        let b = Aabb::new(Vec3::new(x - w / 2.0, 0.0, z - d / 2.0), Vec3::new(x + w / 2.0, WALL_HEIGHT, z + d / 2.0));
        self.collider(b, "partition", Owner::Int(building));
    }

    /// Adds an instance to the shared batch for its geometry and materials.
    fn append(&mut self, geometry: usize, materials: &[usize], t: Mat4, color: [f32; 3], owner: &Owner) {
        let key = (geometry, materials.to_vec());
        let batches = &mut self.city.batches;
        let index = *self.fresh.entry(key).or_insert_with(|| {
            batches.push(crate::city::Batch {
                name: format!("generated/{geometry}"),
                geometry,
                materials: materials.to_vec(),
                transforms: Vec::new(),
                colors: Vec::new(),
                owners: Vec::new(),
                roles: Vec::new(),
            });
            batches.len() - 1
        });
        let batch = &mut batches[index];
        batch.transforms.push(t.to_cols_array());
        batch.colors.extend_from_slice(&color);
        batch.owners.push(Some(owner.clone()));
        batch.roles.push("fresh".to_string());
    }

    fn collider(&mut self, b: Aabb, kind: &str, owner: Owner) {
        self.city.colliders.push(Collider {
            min_x: b.min.x,
            max_x: b.max.x,
            min_y: b.min.y,
            max_y: b.max.y,
            min_z: b.min.z,
            max_z: b.max.z,
            kind: kind.to_string(),
            owner: Some(owner),
        });
    }
}
