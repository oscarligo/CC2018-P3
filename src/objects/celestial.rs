use crate::celestial_shaders::CelestialKind;
use crate::framebuffer::Framebuffer;
use crate::load_objects::{Material, Mesh};
use crate::projection::projected_sphere_radius;
use crate::render::Renderer;
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use crate::world::{WorldObject, WorldTransform, WorldUpdate};
use raylib::prelude::{Matrix, Vector3};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

const BODY_SCALE: f32 = 165.0;
const ROTATION_SPEED: f32 = 0.08;
const HIGH_LATITUDE_SEGMENTS: usize = 36;
const HIGH_LONGITUDE_SEGMENTS: usize = 54;
const MEDIUM_LATITUDE_SEGMENTS: usize = 18;
const MEDIUM_LONGITUDE_SEGMENTS: usize = 27;
const LOW_LATITUDE_SEGMENTS: usize = 8;
const LOW_LONGITUDE_SEGMENTS: usize = 12;
const HIGH_LOD_RADIUS: f32 = 150.0;
const MEDIUM_LOD_RADIUS: f32 = 40.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SphereLod {
    High,
    Medium,
    Low,
}

pub struct CelestialBody {
    colors: Vec<Vector3>,
    lod: SphereLod,
    pub(crate) transform: WorldTransform,
    kind: CelestialKind,
    rotation: f32,
    time: f32,
}

impl CelestialBody {
    pub fn new(kind: CelestialKind, position: Vector3) -> Self {
        let lod = SphereLod::Low;
        let mesh = sphere_mesh(lod);
        let mut body = Self {
            colors: vec![Vector3::zero(); mesh.vertices.len()],
            lod,
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
        for (color, vertex) in self.colors.iter_mut().zip(&sphere_mesh(self.lod).vertices) {
            let point = vertex.normal;
            let world_normal = Vector3::new(
                point.x * cos_rotation + point.z * sin_rotation,
                point.y,
                -point.x * sin_rotation + point.z * cos_rotation,
            );
            *color = crate::celestial_shaders::shade(self.kind, point, world_normal, self.time);
        }
    }

    fn select_lod(&mut self, projected_radius: f32) {
        let lod = if projected_radius >= HIGH_LOD_RADIUS {
            SphereLod::High
        } else if projected_radius >= MEDIUM_LOD_RADIUS {
            SphereLod::Medium
        } else {
            SphereLod::Low
        };
        if lod == self.lod {
            return;
        }
        self.lod = lod;
        self.colors
            .resize(sphere_mesh(lod).vertices.len(), Vector3::zero());
        self.update_surface();
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

    fn bounding_radius(&self) -> Option<f32> {
        Some(1.0)
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let scale = self
            .transform
            .scale
            .x
            .abs()
            .max(self.transform.scale.y.abs())
            .max(self.transform.scale.z.abs());
        let radius =
            projected_sphere_radius(self.transform.position, scale, uniforms, framebuffer.height);
        self.select_lod(radius);
        let body_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            camera_pos: uniforms.camera_pos,
            camera_target: uniforms.camera_target,
            camera_up: uniforms.camera_up,
        };
        renderer.render_colored(
            framebuffer,
            &body_uniforms,
            sphere_mesh(self.lod),
            &self.colors,
        );
    }
}

fn sphere_mesh(lod: SphereLod) -> &'static Mesh {
    static HIGH: OnceLock<Mesh> = OnceLock::new();
    static MEDIUM: OnceLock<Mesh> = OnceLock::new();
    static LOW: OnceLock<Mesh> = OnceLock::new();
    match lod {
        SphereLod::High => {
            HIGH.get_or_init(|| build_sphere_mesh(HIGH_LATITUDE_SEGMENTS, HIGH_LONGITUDE_SEGMENTS))
        }
        SphereLod::Medium => MEDIUM
            .get_or_init(|| build_sphere_mesh(MEDIUM_LATITUDE_SEGMENTS, MEDIUM_LONGITUDE_SEGMENTS)),
        SphereLod::Low => {
            LOW.get_or_init(|| build_sphere_mesh(LOW_LATITUDE_SEGMENTS, LOW_LONGITUDE_SEGMENTS))
        }
    }
}

fn build_sphere_mesh(latitude_segments: usize, longitude_segments: usize) -> Mesh {
    let row_size = longitude_segments + 1;
    let mut vertices = Vec::with_capacity((latitude_segments + 1) * row_size);
    let mut indices = Vec::with_capacity(latitude_segments * longitude_segments * 6);

    for latitude in 0..=latitude_segments {
        let phi = latitude as f32 / latitude_segments as f32 * PI;
        for longitude in 0..=longitude_segments {
            let theta = longitude as f32 / longitude_segments as f32 * TAU;
            let normal = Vector3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            let mut vertex = Vertex::new(normal);
            vertex.normal = normal;
            vertices.push(vertex);
        }
    }

    for latitude in 0..latitude_segments {
        for longitude in 0..longitude_segments {
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
        let mesh = sphere_mesh(SphereLod::High);
        assert_eq!(
            mesh.vertices.len(),
            (HIGH_LATITUDE_SEGMENTS + 1) * (HIGH_LONGITUDE_SEGMENTS + 1)
        );
        assert_eq!(
            mesh.indices.len(),
            HIGH_LATITUDE_SEGMENTS * HIGH_LONGITUDE_SEGMENTS * 6
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

    #[test]
    fn celestial_bodies_share_the_same_geometry() {
        assert!(std::ptr::eq(
            sphere_mesh(SphereLod::High),
            sphere_mesh(SphereLod::High)
        ));
        let body = CelestialBody::new(CelestialKind::Rocky, Vector3::zero());
        assert_eq!(
            body.colors.len(),
            sphere_mesh(SphereLod::Low).vertices.len()
        );
    }

    #[test]
    fn lod_selection_uses_projected_size() {
        let mut body = CelestialBody::new(CelestialKind::Rocky, Vector3::zero());
        body.select_lod(MEDIUM_LOD_RADIUS);
        assert_eq!(body.lod, SphereLod::Medium);
        body.select_lod(HIGH_LOD_RADIUS);
        assert_eq!(body.lod, SphereLod::High);
        assert_eq!(
            body.colors.len(),
            sphere_mesh(SphereLod::High).vertices.len()
        );
    }
}
