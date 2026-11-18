//! Geometría CPU sencilla para objetos que ocupan pocos píxeles.
use crate::rendering::load_objects::{Material, Mesh};
use crate::rendering::vertex::Vertex;
use raylib::prelude::Vector3;

pub(super) struct Builder {
    pub mesh: Mesh,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            mesh: Mesh {
                vertices: Vec::new(),
                indices: Vec::new(),
                materials: vec![Material {
                    diffuse_texture: None,
                }],
            },
        }
    }

    pub fn face(&mut self, a: Vector3, b: Vector3, c: Vector3, outward: Vector3, color: Vector3) {
        let (b, c) = if (b - a).cross(c - a).dot(outward) < 0.0 {
            (c, b)
        } else {
            (b, c)
        };
        let normal = (b - a).cross(c - a).normalize();
        for position in [a, b, c] {
            let mut vertex = Vertex::new(position);
            vertex.normal = normal;
            vertex.color = color;
            self.mesh.indices.push(self.mesh.vertices.len() as u32);
            self.mesh.vertices.push(vertex);
        }
    }

    pub fn block(&mut self, center: Vector3, size: Vector3, color: Vector3) {
        let points = [
            (-1.0, -1.0, -1.0),
            (1.0, -1.0, -1.0),
            (1.0, 1.0, -1.0),
            (-1.0, 1.0, -1.0),
            (-1.0, -1.0, 1.0),
            (1.0, -1.0, 1.0),
            (1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
        ]
        .map(|(x, y, z)| center + Vector3::new(x * size.x, y * size.y, z * size.z) * 0.5);
        for [a, b, c, d] in [
            [0, 1, 2, 3],
            [4, 7, 6, 5],
            [0, 3, 7, 4],
            [1, 5, 6, 2],
            [0, 4, 5, 1],
            [3, 2, 6, 7],
        ] {
            let outward = (points[a] + points[b] + points[c] + points[d]) * 0.25 - center;
            self.face(points[a], points[b], points[c], outward, color);
            self.face(points[a], points[c], points[d], outward, color);
        }
    }

    pub fn crown(&mut self, center: Vector3, radius: Vector3, color: Vector3) {
        let top = center + Vector3::new(0.0, radius.y, 0.0);
        let bottom = center - Vector3::new(0.0, radius.y, 0.0);
        let ring = [
            Vector3::new(radius.x, 0.0, 0.0),
            Vector3::new(0.0, 0.0, radius.z),
            Vector3::new(-radius.x, 0.0, 0.0),
            Vector3::new(0.0, 0.0, -radius.z),
        ];
        for i in 0..4 {
            let a = center + ring[i];
            let b = center + ring[(i + 1) % 4];
            self.face(top, a, b, ring[i], color);
            self.face(bottom, b, a, ring[i], color);
        }
    }
}

pub(super) fn tree(giant: bool) -> Mesh {
    let mut builder = Builder::new();
    let wood = Vector3::new(0.38, 0.22, 0.10);
    let leaf = Vector3::new(0.18, 0.36, 0.09);
    if giant {
        builder.block(
            Vector3::new(0.0, 4.0, 0.0),
            Vector3::new(1.35, 8.0, 1.35),
            wood,
        );
        builder.crown(
            Vector3::new(0.0, 9.5, 0.0),
            Vector3::new(4.6, 2.7, 4.6),
            leaf,
        );
    } else {
        builder.block(
            Vector3::new(0.0, 1.4, 0.0),
            Vector3::new(0.36, 2.8, 0.36),
            wood,
        );
        builder.crown(
            Vector3::new(0.0, 2.2, 0.0),
            Vector3::new(1.15, 1.1, 1.15),
            leaf,
        );
        builder.crown(
            Vector3::new(0.0, 3.4, 0.0),
            Vector3::new(0.65, 1.0, 0.65),
            leaf,
        );
    }
    builder.mesh
}

/// Histéresis: el umbral de entrada es mayor que el de salida.
pub(super) fn choose_full(current: bool, projected_radius: f32, enter: f32, exit: f32) -> bool {
    if current {
        projected_radius >= exit
    } else {
        projected_radius >= enter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_hysteresis_keeps_state_between_thresholds() {
        assert!(!choose_full(false, 15.0, 18.0, 12.0));
        assert!(choose_full(true, 15.0, 18.0, 12.0));
        assert!(choose_full(false, 18.0, 18.0, 12.0));
        assert!(!choose_full(true, 11.0, 18.0, 12.0));
    }
}
