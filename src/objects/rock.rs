use crate::interaction::Interaction;
use crate::physics::Collider;
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::{Material, Mesh};
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use crate::rendering::vertex::Vertex;
use crate::scene::world::{TransformAnchor, WorldObject, WorldTransform};
use raylib::prelude::{Matrix, Quaternion, Vector3};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

const LATITUDE_SEGMENTS: usize = 4;
const LONGITUDE_SEGMENTS: usize = 6;
const ROCK_BOUNDING_RADIUS: f32 = 1.45;

pub struct SurfaceRock {
    transform: WorldTransform,
}

impl SurfaceRock {
    pub(crate) fn attached_to(
        parent: TransformAnchor,
        surface_direction: Vector3,
        scale: Vector3,
        twist: f32,
    ) -> Self {
        let direction = surface_direction.normalize();
        let mut transform = WorldTransform::new(Vector3::zero(), scale);
        transform.set_parent(parent, direction * 1.08);
        let align = Quaternion::from_vector3_to_vector3(Vector3::new(0.0, 1.0, 0.0), direction);
        let twist = Quaternion::from_axis_angle(direction, twist);
        transform.set_local_rotation((align * twist).normalize());
        Self { transform }
    }

    fn model_matrix(&self) -> Matrix {
        Matrix::scale(
            self.transform.scale.x,
            self.transform.scale.y,
            self.transform.scale.z,
        ) * self.transform.rotation.to_matrix()
            * Matrix::translate(
                self.transform.position.x,
                self.transform.position.y,
                self.transform.position.z,
            )
    }
}

impl WorldObject for SurfaceRock {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn bounding_radius(&self) -> Option<f32> {
        Some(ROCK_BOUNDING_RADIUS)
    }

    fn collider(&self) -> Option<Collider> {
        Some(Collider::Sphere {
            radius: ROCK_BOUNDING_RADIUS,
        })
    }

    fn interaction(&self) -> Option<Interaction> {
        Some(Interaction {
            prompt: "[F] Examinar formacion rocosa",
            response: "La roca esta unida a la corteza y gira junto con el planeta.",
        })
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let rock_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            ..*uniforms
        };
        renderer.render_lit(framebuffer, &rock_uniforms, rock_mesh());
    }
}

fn rock_mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(build_rock_mesh)
}

fn build_rock_mesh() -> Mesh {
    let row = LONGITUDE_SEGMENTS + 1;
    let mut vertices = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * row);
    let mut indices = Vec::with_capacity(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6);

    for latitude in 0..=LATITUDE_SEGMENTS {
        let phi = latitude as f32 / LATITUDE_SEGMENTS as f32 * PI;
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let theta = longitude as f32 / LONGITUDE_SEGMENTS as f32 * TAU;
            let normal = Vector3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            let variation = 0.82
                + ((latitude * 17 + longitude * 31) % 7) as f32 * 0.045
                + normal.y.abs() * 0.08;
            let position = Vector3::new(normal.x, normal.y * 1.18, normal.z) * variation;
            let mut vertex = Vertex::new(position);
            vertex.normal = normal;
            let shade = 0.38 + variation * 0.16;
            vertex.color = Vector3::new(shade * 1.05, shade * 0.92, shade * 0.78);
            vertices.push(vertex);
        }
    }

    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let current = (latitude * row + longitude) as u32;
            let next = current + row as u32;
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
    fn rocks_share_a_small_closed_mesh() {
        let mesh = rock_mesh();
        assert!(std::ptr::eq(mesh, rock_mesh()));
        assert_eq!(
            mesh.vertices.len(),
            (LATITUDE_SEGMENTS + 1) * (LONGITUDE_SEGMENTS + 1)
        );
        assert_eq!(
            mesh.indices.len(),
            LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6
        );
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| vertex.position.length() <= ROCK_BOUNDING_RADIUS)
        );
    }
}
