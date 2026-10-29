use crate::load_objects::Mesh;
use crate::uniforms::Uniforms;
use raylib::prelude::*;

pub struct InputController {
    target: Vector3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    min_distance: f32,
    max_distance: f32,
}

impl InputController {
    pub fn new(mesh: &Mesh) -> Self {
        let (min, max) = mesh.vertices.iter().fold(
            (
                Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
                Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
            ),
            |(min, max), vertex| {
                (
                    Vector3::new(
                        min.x.min(vertex.position.x),
                        min.y.min(vertex.position.y),
                        min.z.min(vertex.position.z),
                    ),
                    Vector3::new(
                        max.x.max(vertex.position.x),
                        max.y.max(vertex.position.y),
                        max.z.max(vertex.position.z),
                    ),
                )
            },
        );
        let target = (min + max) * 0.5;
        let size = max - min;
        let radius = (size.x * size.x + size.y * size.y + size.z * size.z).sqrt() * 0.5;

        Self {
            target,
            yaw: 0.0,
            pitch: 0.2,
            distance: radius * 2.5,
            min_distance: radius * 1.0,
            max_distance: radius * 8.0,
        }
    }

    pub fn update(&mut self, window: &RaylibHandle, uniforms: &mut Uniforms) {
        if window.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let delta = window.get_mouse_delta();
            self.yaw -= delta.x * 0.01;
            self.pitch = (self.pitch + delta.y * 0.01).clamp(-1.5, 1.5);
        }

        let wheel = window.get_mouse_wheel_move();
        if wheel != 0.0 {
            self.distance =
                (self.distance * (1.0 - wheel * 0.1)).clamp(self.min_distance, self.max_distance);
        }

        let horizontal_distance = self.distance * self.pitch.cos();
        uniforms.camera_pos = self.target
            + Vector3::new(
                horizontal_distance * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                -horizontal_distance * self.yaw.cos(),
            );
        uniforms.camera_target = self.target;
    }
}
