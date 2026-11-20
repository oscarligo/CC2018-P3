//! OBJ model loading and independent transforms for the animated weapon bay.
use super::compact;
use crate::rendering::{
    framebuffer::Framebuffer,
    load_objects::{Mesh, load_obj},
    render::Renderer,
    uniforms::Uniforms,
};
use raylib::prelude::*;

fn asset(path: &str) -> Mesh {
    compact(
        load_obj(path)
            .unwrap_or_else(|error| panic!("No se pudo cargar el modelo {path}: {error}")),
    )
}

pub(super) fn geometry() -> Mesh {
    asset("assets/star_destroyer.obj")
}

pub(super) struct WeaponBay {
    door: Mesh,
    bomb: Mesh,
    scale: f32,
    door_colors: Vec<Vector3>,
    bomb_colors: Vec<Vector3>,
}
impl WeaponBay {
    pub(super) fn new(scale: f32) -> Self {
        let door = asset("assets/star_destroyer_door.obj");
        let bomb = asset("assets/star_destroyer_bomb.obj");
        let door_colors = door.vertices.iter().map(|p| p.color).collect();
        let bomb_colors = vec![Vector3::zero(); bomb.vertices.len()];
        Self {
            door,
            bomb,
            scale,
            door_colors,
            bomb_colors,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        &mut self,
        r: &mut Renderer,
        fb: &mut Framebuffer,
        u: &Uniforms,
        open: f32,
        charge: f32,
        visible: bool,
        time: f32,
    ) {
        for side in [-1., 1.] {
            let m = Matrix::scale(self.scale, self.scale, self.scale)
                * Matrix::translate(
                    side * (0.25 + 0.64 * open) * self.scale,
                    -0.65 * self.scale,
                    self.scale,
                )
                * u.model_matrix;
            r.render_colored(
                fb,
                &Uniforms {
                    model_matrix: m,
                    ..*u
                },
                &self.door,
                &self.door_colors,
            );
        }
        if visible && open > 0.02 {
            for (out, p) in self.bomb_colors.iter_mut().zip(&self.bomb.vertices) {
                *out = if p.alpha > 1.5 {
                    p.color * (0.3 + charge * 1.4)
                } else {
                    p.color * (0.7 + charge * 0.3)
                };
            }
            let m = Matrix::rotate_z(time * 0.4)
                * Matrix::scale(self.scale, self.scale, self.scale)
                * Matrix::translate(0., (-0.68 - open * 0.38) * self.scale, self.scale)
                * u.model_matrix;
            r.render_colored(
                fb,
                &Uniforms {
                    model_matrix: m,
                    ..*u
                },
                &self.bomb,
                &self.bomb_colors,
            );
        }
    }
}
