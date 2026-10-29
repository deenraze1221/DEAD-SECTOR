//! Shared helpers for the GPU tests.

use glam::{Mat4, Quat, Vec3};
use sector_assets::{Instance, Vertex};

/// Unit cube (−0.5..0.5) with outward normals.
pub fn cube() -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            let mut n = [0.0; 3];
            n[axis] = sign;
            let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
            let corner = |a: f32, b: f32| {
                let mut p = [0.0; 3];
                p[axis] = 0.5 * sign;
                p[u] = a;
                p[v] = b;
                Vertex { position: p, normal: n, uv: [0.0; 2], color: [1.0; 3] }
            };
            let quad = [corner(-0.5, -0.5), corner(0.5, -0.5), corner(0.5, 0.5), corner(-0.5, 0.5)];
            // Counter-clockwise seen from outside.
            let order = if sign > 0.0 { [0, 1, 2, 0, 2, 3] } else { [0, 2, 1, 0, 3, 2] };
            vertices.extend(order.iter().map(|&i| quad[i]));
        }
    }
    vertices
}

pub fn boxed(centre: Vec3, size: Vec3) -> Instance {
    Instance {
        model: Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, centre).to_cols_array(),
        color: [1.0; 4],
    }
}
