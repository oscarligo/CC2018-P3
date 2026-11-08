use super::celestial::{BodyShape, shape_radius};
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

pub struct RockField {
    transform: WorldTransform,
    mesh: Mesh,
}

impl RockField {
    pub(crate) fn attached_to(
        parent: TransformAnchor,
        body_scale: f32,
        shape: BodyShape,
        seed: u32,
        count: usize,
    ) -> Self {
        let mut transform = WorldTransform::new(
            Vector3::zero(),
            Vector3::new(body_scale, body_scale, body_scale),
        );
        transform.set_parent(parent, Vector3::zero());
        Self {
            transform,
            mesh: build_rock_field_mesh(shape, seed, count),
        }
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

impl WorldObject for RockField {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn bounding_radius(&self) -> Option<f32> {
        Some(1.25)
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let field_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            ..*uniforms
        };
        renderer.render_lit(framebuffer, &field_uniforms, &self.mesh);
    }
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

fn build_rock_field_mesh(shape: BodyShape, seed: u32, count: usize) -> Mesh {
    let source = rock_mesh();
    let mut random = DeterministicRandom::new(seed);
    let mut vertices = Vec::with_capacity(source.vertices.len() * count);
    let mut indices = Vec::with_capacity(source.indices.len() * count);

    for _ in 0..count {
        let y = random.range(-1.0, 1.0);
        let angle = random.range(0.0, TAU);
        let horizontal = (1.0 - y * y).sqrt();
        let direction = Vector3::new(horizontal * angle.cos(), y, horizontal * angle.sin());
        let size = random.range(0.025, 0.060);
        let center = direction * (shape_radius(direction, shape) + size * 0.65);
        let align = Quaternion::from_vector3_to_vector3(Vector3::new(0.0, 1.0, 0.0), direction);
        let twist = Quaternion::from_axis_angle(direction, random.range(0.0, TAU));
        let rotation = (align * twist).normalize().to_matrix();
        let first_vertex = vertices.len() as u32;
        let color_variation = random.range(0.78, 1.12);

        vertices.extend(source.vertices.iter().map(|source| {
            let mut vertex = *source;
            vertex.position = center + transform_direction(&rotation, source.position * size);
            vertex.normal = transform_direction(&rotation, source.normal).normalize();
            vertex.color *= color_variation;
            vertex
        }));
        indices.extend(source.indices.iter().map(|index| first_vertex + index));
    }

    Mesh {
        vertices,
        indices,
        materials: vec![Material {
            diffuse_texture: None,
        }],
    }
}

fn transform_direction(matrix: &Matrix, direction: Vector3) -> Vector3 {
    Vector3::new(
        matrix.m0 * direction.x + matrix.m4 * direction.y + matrix.m8 * direction.z,
        matrix.m1 * direction.x + matrix.m5 * direction.y + matrix.m9 * direction.z,
        matrix.m2 * direction.x + matrix.m6 * direction.y + matrix.m10 * direction.z,
    )
}

struct DeterministicRandom(u32);

impl DeterministicRandom {
    fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32
    }

    fn range(&mut self, minimum: f32, maximum: f32) -> f32 {
        minimum + (maximum - minimum) * self.unit()
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

    #[test]
    fn rock_fields_are_deterministic_and_batched() {
        let first = build_rock_field_mesh(BodyShape::Rocky, 42, 4);
        let second = build_rock_field_mesh(BodyShape::Rocky, 42, 4);

        assert_eq!(first.vertices.len(), rock_mesh().vertices.len() * 4);
        assert_eq!(first.indices.len(), rock_mesh().indices.len() * 4);
        assert_eq!(first.vertices[10].position, second.vertices[10].position);
        assert_eq!(first.indices, second.indices);
        assert!(
            first
                .vertices
                .iter()
                .all(|vertex| vertex.position.length() <= 1.25)
        );
    }
}
