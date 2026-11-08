use crate::framebuffer::Framebuffer;
use crate::load_objects::{Material, Mesh};
use crate::render::Renderer;
use crate::uniforms::Uniforms;
use crate::vertex::Vertex;
use crate::world::{Orbit, RenderPass, WorldObject, WorldTransform};
use raylib::prelude::{Matrix, Vector3};
use std::f32::consts::TAU;
use std::sync::OnceLock;

const SEGMENTS: usize = 72;
const RADII: [f32; 6] = [0.54, 0.60, 0.70, 0.82, 0.94, 1.0];
const ALPHAS: [f32; 6] = [0.02, 0.24, 0.34, 0.20, 0.30, 0.02];

pub struct PlanetRing {
    transform: WorldTransform,
    tilt_x: f32,
    tilt_z: f32,
}

impl PlanetRing {
    pub fn new(position: Vector3, outer_radius: f32, tilt_x: f32, tilt_z: f32) -> Self {
        Self {
            transform: WorldTransform::new(
                position,
                Vector3::new(outer_radius, outer_radius, outer_radius),
            ),
            tilt_x,
            tilt_z,
        }
    }

    pub fn with_orbit(mut self, orbit: Orbit) -> Self {
        self.transform.set_orbit(orbit);
        self
    }

    fn model_matrix(&self) -> Matrix {
        Matrix::scale(
            self.transform.scale.x,
            self.transform.scale.y,
            self.transform.scale.z,
        ) * Matrix::rotate_x(self.tilt_x)
            * Matrix::rotate_z(self.tilt_z)
            * Matrix::translate(
                self.transform.position.x,
                self.transform.position.y,
                self.transform.position.z,
            )
    }
}

impl WorldObject for PlanetRing {
    fn transform(&self) -> &WorldTransform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut WorldTransform {
        &mut self.transform
    }

    fn bounding_radius(&self) -> Option<f32> {
        Some(1.0)
    }

    fn render_pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        let ring_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            ..*uniforms
        };
        renderer.render_transparent(framebuffer, &ring_uniforms, ring_mesh());
    }
}

fn ring_mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(build_ring_mesh)
}

fn build_ring_mesh() -> Mesh {
    let row_size = SEGMENTS + 1;
    let mut vertices = Vec::with_capacity(RADII.len() * row_size);
    let mut indices = Vec::with_capacity((RADII.len() - 1) * SEGMENTS * 12);

    for (radial_index, radius) in RADII.into_iter().enumerate() {
        for segment in 0..=SEGMENTS {
            let angle = segment as f32 / SEGMENTS as f32 * TAU;
            let mut vertex = Vertex::new(Vector3::new(
                radius * angle.cos(),
                0.0,
                radius * angle.sin(),
            ));
            let band = radial_index as f32 / (RADII.len() - 1) as f32;
            vertex.color = Vector3::new(0.72 + band * 0.20, 0.52 + band * 0.22, 0.30 + band * 0.18);
            vertex.alpha = ALPHAS[radial_index];
            vertices.push(vertex);
        }
    }

    for radial in 0..RADII.len() - 1 {
        for segment in 0..SEGMENTS {
            let inner = (radial * row_size + segment) as u32;
            let outer = inner + row_size as u32;
            let front = [inner, inner + 1, outer, inner + 1, outer + 1, outer];
            indices.extend_from_slice(&front);
            indices.extend(front.into_iter().rev());
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
    fn ring_has_faded_edges_and_two_sided_geometry() {
        let mesh = ring_mesh();
        assert_eq!(mesh.vertices.len(), RADII.len() * (SEGMENTS + 1));
        assert_eq!(mesh.indices.len(), (RADII.len() - 1) * SEGMENTS * 12);
        assert!(mesh.vertices[0].alpha < mesh.vertices[SEGMENTS + 1].alpha);
    }
}
