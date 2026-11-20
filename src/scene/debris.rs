//! Deterministic, bounded debris geometry, transformed and rasterized on the CPU.
use crate::{
    rendering::{
        framebuffer::Framebuffer, load_objects::Mesh, render::Renderer, uniforms::Uniforms,
        vertex::Vertex,
    },
    scene::world::{WorldObject, WorldTransform, WorldUpdate},
};
use raylib::prelude::*;

pub struct PlanetDebris {
    transform: WorldTransform,
    radius: f32,
    age: f32,
    mesh: Mesh,
    colors: Vec<Vector3>,
}

impl PlanetDebris {
    pub fn new(position: Vector3, radius: f32) -> Self {
        Self {
            transform: WorldTransform::new(position, Vector3::one()),
            radius,
            age: 0.0,
            mesh: Mesh {
                vertices: Vec::with_capacity(1152),
                indices: Vec::with_capacity(1152),
                materials: vec![],
            },
            colors: Vec::with_capacity(1152),
        }
    }
    fn rebuild(&mut self) {
        self.mesh.vertices.clear();
        self.mesh.indices.clear();
        self.colors.clear();
        for i in 0..96 {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 96.0;
            let a = i as f32 * 2.399963;
            let d = Vector3::new(
                (1.0 - y * y).sqrt() * a.cos(),
                y,
                (1.0 - y * y).sqrt() * a.sin(),
            );
            let distance =
                self.radius * (0.5 + self.age.min(18.0) * (0.16 + (i % 7) as f32 * 0.025));
            let center = self.transform.position + d * distance;
            let s = self.radius * (0.045 + (i % 5) as f32 * 0.017);
            let rotation =
                Matrix::rotate_y(a + self.age * 0.23) * Matrix::rotate_x(a * 0.7 + self.age * 0.17);
            let p = [
                Vector3::new(0.0, s, 0.0),
                Vector3::new(-s, -s, s),
                Vector3::new(s, -s, s),
                Vector3::new(0.0, -s, -s),
            ]
            .map(|p| {
                Vector3::new(
                    rotation.m0 * p.x + rotation.m4 * p.y + rotation.m8 * p.z,
                    rotation.m1 * p.x + rotation.m5 * p.y + rotation.m9 * p.z,
                    rotation.m2 * p.x + rotation.m6 * p.y + rotation.m10 * p.z,
                ) + center
            });
            for face in [[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]] {
                let n = (p[face[1]] - p[face[0]])
                    .cross(p[face[2]] - p[face[0]])
                    .normalize();
                let light = 0.32 + n.dot(Vector3::new(-0.4, 0.8, -0.5).normalize()).max(0.0) * 0.7;
                let heat = (-self.age * 0.38).exp();
                let color =
                    Vector3::new(0.32, 0.26, 0.24) * light + Vector3::new(0.9, 0.20, 0.015) * heat;
                for index in face {
                    let mut vertex = Vertex::new(p[index]);
                    vertex.normal = n;
                    self.mesh.indices.push(self.mesh.vertices.len() as u32);
                    self.mesh.vertices.push(vertex);
                    self.colors.push(color);
                }
            }
        }
    }
}
impl WorldObject for PlanetDebris {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }
    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }
    fn bounding_radius(&self) -> Option<f32> {
        Some(self.radius * 6.5)
    }
    fn update(&mut self, update: &WorldUpdate) {
        self.age += update.frame_time;
    }
    fn draw(&mut self, renderer: &mut Renderer, fb: &mut Framebuffer, u: &Uniforms) {
        self.rebuild();
        renderer.render_colored(
            fb,
            &Uniforms {
                model_matrix: Matrix::identity(),
                ..*u
            },
            &self.mesh,
            &self.colors,
        );
    }
}
