use crate::celestial_shaders::CelestialKind;
use crate::framebuffer::Framebuffer;
use crate::load_objects::{Material, Mesh};
use crate::render::Renderer;
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use crate::world::{WorldObject, WorldTransform, WorldUpdate};
use raylib::prelude::{Matrix, Vector3};
use std::f32::consts::{PI, TAU};

const BODY_SCALE: f32 = 165.0;
const ROTATION_SPEED: f32 = 0.08;
const LATITUDE_SEGMENTS: usize = 36;
const LONGITUDE_SEGMENTS: usize = 54;

pub struct CelestialBody {
    mesh: Mesh,
    pub(crate) transform: WorldTransform,
    kind: CelestialKind,
    rotation: f32,
    time: f32,
}

impl CelestialBody {
    pub fn new(kind: CelestialKind, position: Vector3) -> Self {
        let mut body = Self {
            mesh: sphere_mesh(),
            transform: WorldTransform::new(
                position,
                Vector3::new(BODY_SCALE, BODY_SCALE, BODY_SCALE),
            ),
            kind,
            rotation: 0.0,
            time: 0.0,
        };
        body.update_surface();
        body
    }

    fn model_matrix(&self) -> Matrix {
        Matrix::scale(
            self.transform.scale.x,
            self.transform.scale.y,
            self.transform.scale.z,
        ) * Matrix::rotate_y(self.rotation)
            * Matrix::translate(
                self.transform.position.x,
                self.transform.position.y,
                self.transform.position.z,
            )
    }

    fn update_surface(&mut self) {
        let (sin_rotation, cos_rotation) = self.rotation.sin_cos();
        for vertex in &mut self.mesh.vertices {
            let point = vertex.normal;
            let world_normal = Vector3::new(
                point.x * cos_rotation + point.z * sin_rotation,
                point.y,
                -point.x * sin_rotation + point.z * cos_rotation,
            );
            vertex.color =
                crate::celestial_shaders::shade(self.kind, point, world_normal, self.time);
        }
    }
}

impl WorldObject for CelestialBody {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn update(&mut self, update: &WorldUpdate) {
        self.time += update.frame_time;
        self.rotation = (self.rotation + ROTATION_SPEED * update.frame_time) % TAU;
        self.update_surface();
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let body_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            camera_pos: uniforms.camera_pos,
            camera_target: uniforms.camera_target,
            camera_up: uniforms.camera_up,
        };
        renderer.render(framebuffer, &body_uniforms, &self.mesh);
    }
}

fn sphere_mesh() -> Mesh {
    let row_size = LONGITUDE_SEGMENTS + 1;
    let mut vertices = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * row_size);
    let mut indices = Vec::with_capacity(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6);

    for latitude in 0..=LATITUDE_SEGMENTS {
        let phi = latitude as f32 / LATITUDE_SEGMENTS as f32 * PI;
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let theta = longitude as f32 / LONGITUDE_SEGMENTS as f32 * TAU;
            let normal = Vector3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            let mut vertex = Vertex::new(normal);
            vertex.normal = normal;
            vertices.push(vertex);
        }
    }

    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let current = (latitude * row_size + longitude) as u32;
            let next = current + row_size as u32;
            indices.extend_from_slice(&[current, current + 1, next, current + 1, next + 1, next]);
        }
    }

    Mesh {
        vertices,
        indices,
        materials: vec![Material {
            diffuse_texture: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_sphere_has_the_expected_topology() {
        let mesh = sphere_mesh();
        assert_eq!(
            mesh.vertices.len(),
            (LATITUDE_SEGMENTS + 1) * (LONGITUDE_SEGMENTS + 1)
        );
        assert_eq!(
            mesh.indices.len(),
            LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| (vertex.normal.length() - 1.0).abs() < 0.001));
        assert!(mesh.indices.chunks_exact(3).all(|indices| {
            let a = mesh.vertices[indices[0] as usize].position;
            let b = mesh.vertices[indices[1] as usize].position;
            let c = mesh.vertices[indices[2] as usize].position;
            let face_normal = (b - a).cross(c - a);
            face_normal.length() < 0.001 || face_normal.dot(a + b + c) > 0.0
        }));
    }
}
