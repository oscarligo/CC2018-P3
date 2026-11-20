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

pub(super) struct EngineLights {
    sources: [(Vector3, f32); 7],
}

impl EngineLights {
    pub(super) fn new(scale: f32) -> Self {
        // Centers and mouth radii correspond to tools/build_destroyer_engines.py.
        let layout = [
            (-1.95, -0.13, 0.51),
            (0.0, -0.13, 0.55),
            (1.95, -0.13, 0.51),
            (-0.98, 0.30, 0.255),
            (-0.98, -0.44, 0.255),
            (0.98, 0.30, 0.255),
            (0.98, -0.44, 0.255),
        ];
        Self {
            sources: layout.map(|(x, y, r)| (Vector3::new(x, y, -3.76) * scale, r * 0.69 * scale)),
        }
    }

    pub(super) fn draw(&self, fb: &mut Framebuffer, u: &Uniforms, time: f32) {
        use crate::rendering::projection::{project_vertex, projected_sphere_radius};
        let aft = super::direction(&u.model_matrix, Vector3::new(0., 0., -1.)).normalize();
        for (local, radius) in self.sources {
            let position = super::point(&u.model_matrix, local);
            let facing = aft.dot((u.camera_pos - position).normalize()).max(0.);
            if facing < 0.08 {
                continue;
            }
            if let Some((p, depth)) = project_vertex(
                position,
                u.camera_pos,
                u.camera_target,
                u.camera_up,
                fb.width as f32,
                fb.height as f32,
            ) {
                let pixels =
                    (projected_sphere_radius(position, radius, u, fb.height) * 1.5).clamp(1., 90.);
                crate::ship::alien::glow(
                    fb,
                    p,
                    depth,
                    pixels,
                    Vector3::new(0.30, 0.83, 1.),
                    0.18 * facing * facing * (1. + 0.025 * (time * 5. + local.x).sin()),
                );
            }
        }
    }
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
