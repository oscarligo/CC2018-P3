use super::celestial::{SphereLod, sphere_mesh};
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use crate::scene::world::{RenderPass, TransformAnchor, WorldObject, WorldTransform};
use raylib::prelude::{Matrix, Vector3};

pub struct Atmosphere {
    transform: WorldTransform,
    strength: f32,
    colors: Vec<Vector3>,
    alphas: Vec<f32>,
}

impl Atmosphere {
    pub fn new(
        position: Vector3,
        body_scale: f32,
        shell_scale: f32,
        color: Vector3,
        strength: f32,
    ) -> Self {
        let vertex_count = sphere_mesh(SphereLod::Medium).vertices.len();
        Self {
            transform: WorldTransform::new(
                position,
                Vector3::new(
                    body_scale * shell_scale,
                    body_scale * shell_scale,
                    body_scale * shell_scale,
                ),
            ),
            strength,
            colors: vec![color; vertex_count],
            alphas: vec![0.0; vertex_count],
        }
    }

    pub(crate) fn attached_to(mut self, parent: TransformAnchor, local_position: Vector3) -> Self {
        self.transform.set_parent(parent, local_position);
        self
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

    fn update_fresnel(&mut self, camera_position: Vector3) {
        let direction = camera_position - self.transform.position;
        if direction.length() <= f32::EPSILON {
            return;
        }
        let view_direction = direction.normalize();
        for (alpha, vertex) in self
            .alphas
            .iter_mut()
            .zip(&sphere_mesh(SphereLod::Medium).vertices)
        {
            let facing = vertex.normal.dot(view_direction).max(0.0);
            let rim = (1.0 - facing).powf(2.2);
            *alpha = self.strength * (0.04 + rim * 0.96);
        }
    }
}

impl WorldObject for Atmosphere {
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
        self.update_fresnel(uniforms.camera_pos);
        let atmosphere_uniforms = Uniforms {
            model_matrix: self.model_matrix(),
            ..*uniforms
        };
        renderer.render_colored_transparent(
            framebuffer,
            &atmosphere_uniforms,
            sphere_mesh(SphereLod::Medium),
            &self.colors,
            &self.alphas,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresnel_is_stronger_at_the_limb() {
        let mut atmosphere =
            Atmosphere::new(Vector3::zero(), 1.0, 1.1, Vector3::new(0.2, 0.5, 1.0), 0.5);
        atmosphere.update_fresnel(Vector3::new(0.0, 0.0, -10.0));
        let mesh = sphere_mesh(SphereLod::Medium);
        let center = mesh
            .vertices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.normal.z.total_cmp(&b.normal.z))
            .map(|(index, _)| atmosphere.alphas[index])
            .unwrap();
        let limb = mesh
            .vertices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.normal.z.abs().total_cmp(&b.normal.z.abs()))
            .map(|(index, _)| atmosphere.alphas[index])
            .unwrap();
        assert!(limb > center);
    }
}
