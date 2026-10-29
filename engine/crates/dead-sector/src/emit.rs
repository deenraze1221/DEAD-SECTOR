//! Adds generated instances and colliders to a city. New instances join the
//! existing batch with the same geometry and materials, so generated content
//! adds no draw calls unless it uses a combination the export never had.

use crate::city::{Batch, City, Collider, Owner};
use crate::citygen::Aabb;
use glam::Mat4;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Emitter {
    batch_for: HashMap<(usize, Vec<usize>), usize>,
}

impl Emitter {
    pub fn new(city: &City) -> Emitter {
        let mut batch_for = HashMap::new();
        for (i, b) in city.batches.iter().enumerate() {
            batch_for.entry((b.geometry, b.materials.clone())).or_insert(i);
        }
        Emitter { batch_for }
    }

    /// Adds one instance of a geometry drawn with its material list.
    pub fn instance(
        &mut self,
        city: &mut City,
        (geometry, materials): (usize, &[usize]),
        t: Mat4,
        color: [f32; 3],
        owner: &Owner,
        role: &str,
    ) {
        let batches = &mut city.batches;
        let index = *self.batch_for.entry((geometry, materials.to_vec())).or_insert_with(|| {
            batches.push(Batch {
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
        // Batches exported without per-instance colours are white.
        if batch.colors.len() < batch.transforms.len() * 3 {
            batch.colors.resize(batch.transforms.len() * 3, 1.0);
        }
        batch.owners.resize(batch.transforms.len(), None);
        batch.roles.resize(batch.transforms.len(), "model".to_string());
        batch.transforms.push(t.to_cols_array());
        batch.colors.extend_from_slice(&color);
        batch.owners.push(Some(owner.clone()));
        batch.roles.push(role.to_string());
    }

    pub fn collider(&self, city: &mut City, b: Aabb, kind: &str, owner: Owner) {
        city.colliders.push(Collider {
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
