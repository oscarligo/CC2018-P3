use crate::input::InputState;
use crate::load_objects::Mesh;
use crate::uniforms::Uniforms;
use raylib::prelude::*;

pub struct ShipMovement {
    target: Vector3,
    camera_yaw: f32,
    camera_pitch: f32,
    camera_distance: f32,
    min_distance: f32,
    max_distance: f32,
    ship_pitch: f32,
    ship_yaw: f32,
    ship_roll: f32,
}

impl ShipMovement {
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
            camera_yaw: 0.0,
            camera_pitch: 0.2,
            camera_distance: radius * 2.5,
            min_distance: radius,
            max_distance: radius * 8.0,
            ship_pitch: 0.0,
            ship_yaw: 0.0,
            ship_roll: 0.0,
        }
    }

    pub fn update(&mut self, input: &InputState, frame_time: f32, uniforms: &mut Uniforms) {
        self.update_camera(input);
        self.update_ship(input, frame_time);
        self.apply(uniforms);
    }

    fn update_camera(&mut self, input: &InputState) {
        self.camera_yaw -= input.orbit_delta.x * 0.01;
        self.camera_pitch = (self.camera_pitch + input.orbit_delta.y * 0.01).clamp(-1.5, 1.5);
        if input.zoom_delta != 0.0 {
            self.camera_distance = (self.camera_distance * (1.0 - input.zoom_delta * 0.1))
                .clamp(self.min_distance, self.max_distance);
        }
    }

    fn update_ship(&mut self, input: &InputState, frame_time: f32) {
        const MAX_TILT: f32 = 20.0_f32.to_radians();
        const TILT_SPEED: f32 = 2.5;
        let step = TILT_SPEED * frame_time;

        self.ship_pitch = move_towards(
            self.ship_pitch,
            axis_target(input.pitch_up, input.pitch_down, MAX_TILT),
            step,
        );
        self.ship_yaw = move_towards(
            self.ship_yaw,
            axis_target(input.turn_left, input.turn_right, MAX_TILT),
            step,
        );
        self.ship_roll = move_towards(
            self.ship_roll,
            axis_target(input.bank_left, input.bank_right, MAX_TILT),
            step,
        );
    }

    fn apply(&self, uniforms: &mut Uniforms) {
        uniforms.model_matrix = Matrix::translate(self.target.x, self.target.y, self.target.z)
            * Matrix::rotate_xyz(Vector3::new(self.ship_pitch, self.ship_yaw, self.ship_roll))
            * Matrix::translate(-self.target.x, -self.target.y, -self.target.z);

        let horizontal_distance = self.camera_distance * self.camera_pitch.cos();
        uniforms.camera_pos = self.target
            + Vector3::new(
                horizontal_distance * self.camera_yaw.sin(),
                self.camera_distance * self.camera_pitch.sin(),
                -horizontal_distance * self.camera_yaw.cos(),
            );
        uniforms.camera_target = self.target;
    }
}

fn axis_target(positive: bool, negative: bool, amount: f32) -> f32 {
    match (positive, negative) {
        (true, false) => amount,
        (false, true) => -amount,
        _ => 0.0,
    }
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}
