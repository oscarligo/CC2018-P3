use crate::celestial_shaders::CelestialKind;
use crate::framebuffer::Framebuffer;
use crate::load_objects::{Mesh, load_obj};
use crate::render::Renderer;
use crate::shaders::FragmentShader;
use crate::uniforms::Uniforms;
use crate::world::{WorldObject, WorldTransform, WorldUpdate};
use raylib::prelude::{Matrix, Vector3};

const SPHERE_PATH: &str = "assets/sphere.obj";
const BODY_SCALE: f32 = 165.0;
const ROTATION_SPEED: f32 = 0.08;

pub struct CelestialBody {
    mesh: Mesh,
    pub(crate) transform: WorldTransform,
    kind: CelestialKind,
    rotation: f32,
    time: f32,
}

impl CelestialBody {
    pub fn new(kind: CelestialKind, position: Vector3) -> Self {
        Self {
            mesh: load_obj(SPHERE_PATH).expect("Failed to load assets/sphere.obj"),
            transform: WorldTransform::new(
                position,
                Vector3::new(BODY_SCALE, BODY_SCALE, BODY_SCALE),
            ),
            kind,
            rotation: 0.0,
            time: 0.0,
        }
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
        self.rotation += ROTATION_SPEED * update.frame_time;
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
        renderer.render(
            framebuffer,
            &body_uniforms,
            &self.mesh,
            FragmentShader::Celestial {
                kind: self.kind,
                time: self.time,
            },
        );
    }
}
