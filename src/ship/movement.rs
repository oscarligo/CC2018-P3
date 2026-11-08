use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::Mesh;
use crate::rendering::uniforms::Uniforms;
use crate::ship::camera::FollowCamera;
use crate::ship::effects::ShipEffects;
use crate::ship::input::InputState;
use raylib::prelude::*;

const MAX_SHIP_PITCH_TILT: f32 = 16.0_f32.to_radians();
const MAX_SHIP_YAW_TILT: f32 = 14.0_f32.to_radians();
const AUTO_BANK_MAX_ANGLE: f32 = 32.0_f32.to_radians();
const MANUAL_ROLL_MAX_ANGLE: f32 = 55.0_f32.to_radians();
const SHIP_TILT_SMOOTHING: f32 = 6.5;
const STEER_ANGULAR_ACCEL: f32 = 7.0;
const NAVIGATION_TURN_SPEED: f32 = 1.35;
const MAX_SPEED_FACTOR: f32 = 2.8;
const BOOST_SPEED_MULTIPLIER: f32 = 2.2;
const MIN_ACCELERATION_DURATION: f32 = 0.1;
const SPACE_DRAG_DECEL_FACTOR: f32 = 0.45;
const BRAKE_DECEL_FACTOR: f32 = 2.4;
const MOVEMENT_THRESHOLD: f32 = 0.01;

pub struct ShipMovement {
    model_center: Vector3,
    target: Vector3,
    camera: FollowCamera,
    effects: ShipEffects,
    ship_pitch: f32,
    ship_yaw: f32,
    ship_roll: f32,
    pitch_rate: f32,
    yaw_rate: f32,
    orientation: Quaternion,
    speed: f32,
    max_speed: f32,
    acceleration: f32,
    deceleration: f32,
    brake_deceleration: f32,
}

impl ShipMovement {
    pub fn new(mesh: &Mesh, acceleration_duration: f32) -> Self {
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
        let left_wing = mesh
            .vertices
            .iter()
            .min_by(|a, b| a.position.x.total_cmp(&b.position.x))
            .map(|vertex| vertex.position)
            .unwrap_or(target);
        let right_wing = mesh
            .vertices
            .iter()
            .max_by(|a, b| a.position.x.total_cmp(&b.position.x))
            .map(|vertex| vertex.position)
            .unwrap_or(target);
        let rear_point = mesh
            .vertices
            .iter()
            .min_by(|a, b| a.position.z.total_cmp(&b.position.z))
            .map(|vertex| vertex.position)
            .unwrap_or(target);
        let max_speed = radius * MAX_SPEED_FACTOR;

        Self {
            model_center: target,
            target,
            camera: FollowCamera::new(radius),
            effects: ShipEffects::new([left_wing, right_wing], rear_point, radius),
            ship_pitch: 0.0,
            ship_yaw: 0.0,
            ship_roll: 0.0,
            pitch_rate: 0.0,
            yaw_rate: 0.0,
            orientation: Quaternion::identity(),
            speed: 0.0,
            max_speed,
            acceleration: max_speed / acceleration_duration.max(MIN_ACCELERATION_DURATION),
            deceleration: max_speed * SPACE_DRAG_DECEL_FACTOR,
            brake_deceleration: max_speed * BRAKE_DECEL_FACTOR,
        }
    }

    pub fn update(&mut self, input: &InputState, frame_time: f32, uniforms: &mut Uniforms) {
        self.update_navigation(input, frame_time);
        self.update_ship(input, frame_time);
        self.update_motion(input.thrust, input.boost, input.braking, frame_time);
        self.target += self.world_velocity() * frame_time;
        self.camera
            .update(input, frame_time, self.target, self.orientation, uniforms);
        self.apply_model(uniforms);

        let trail_emitting = (input.boost
            || (self.speed > self.max_speed * 0.5 && self.yaw_rate.abs() > 0.35))
            && self.speed > MOVEMENT_THRESHOLD;
        self.effects.update(
            trail_emitting,
            input.thrust && !input.braking,
            frame_time,
            self.world_velocity(),
            self.speed_ratio(),
            self.camera.effective_distance(),
            self.camera.reference_distance(),
            uniforms,
        );
    }

    pub fn is_moving(&self) -> bool {
        self.speed > MOVEMENT_THRESHOLD
    }

    pub fn can_accelerate(&self) -> bool {
        self.speed < self.max_speed * BOOST_SPEED_MULTIPLIER - MOVEMENT_THRESHOLD
    }

    pub fn speed_ratio(&self) -> f32 {
        (self.speed / self.max_speed).clamp(0.0, BOOST_SPEED_MULTIPLIER)
    }

    pub fn draw_trail(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        self.effects.draw_trail(
            framebuffer,
            uniforms,
            self.camera.effective_distance(),
            self.camera.reference_distance(),
        );
    }

    pub fn draw_exhaust(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        self.effects
            .draw_exhaust(framebuffer, uniforms, self.camera.reference_distance());
    }

    fn world_velocity(&self) -> Vector3 {
        transform_direction(&self.orientation.to_matrix(), Vector3::new(0.0, 0.0, 1.0)) * self.speed
    }

    fn update_navigation(&mut self, input: &InputState, frame_time: f32) {
        let target_pitch = axis_value(input.pitch_down, input.pitch_up);
        let target_yaw = axis_value(input.turn_right, input.turn_left);
        let roll_turn_assist = axis_value(input.bank_right, input.bank_left) * 0.45;
        let blend = (STEER_ANGULAR_ACCEL * frame_time).min(1.0);
        self.pitch_rate += (target_pitch - self.pitch_rate) * blend;
        self.yaw_rate += ((target_yaw + roll_turn_assist) - self.yaw_rate) * blend;
        let pitch = Quaternion::from_axis_angle(
            Vector3::new(1.0, 0.0, 0.0),
            self.pitch_rate * NAVIGATION_TURN_SPEED * frame_time,
        );
        let yaw = Quaternion::from_axis_angle(
            Vector3::new(0.0, 1.0, 0.0),
            self.yaw_rate * NAVIGATION_TURN_SPEED * frame_time,
        );
        self.orientation = (self.orientation * yaw * pitch).normalize();
    }

    fn update_ship(&mut self, input: &InputState, frame_time: f32) {
        let smooth = (SHIP_TILT_SMOOTHING * frame_time).min(1.0);
        let target_pitch = (self.pitch_rate * MAX_SHIP_PITCH_TILT)
            .clamp(-MAX_SHIP_PITCH_TILT, MAX_SHIP_PITCH_TILT);
        let target_yaw =
            (self.yaw_rate * MAX_SHIP_YAW_TILT).clamp(-MAX_SHIP_YAW_TILT, MAX_SHIP_YAW_TILT);
        let auto_bank = -self.yaw_rate * AUTO_BANK_MAX_ANGLE;
        let manual_roll = axis_value(input.bank_left, input.bank_right) * MANUAL_ROLL_MAX_ANGLE;
        let target_roll = (auto_bank + manual_roll).clamp(-1.25, 1.25);
        self.ship_pitch += (target_pitch - self.ship_pitch) * smooth;
        self.ship_yaw += (target_yaw - self.ship_yaw) * smooth;
        self.ship_roll += (target_roll - self.ship_roll) * smooth;
    }

    fn update_motion(&mut self, thrust: bool, boost: bool, braking: bool, frame_time: f32) {
        if braking {
            self.speed = (self.speed - self.brake_deceleration * frame_time).max(0.0);
        } else if boost {
            self.speed = move_towards(
                self.speed,
                self.max_speed * BOOST_SPEED_MULTIPLIER,
                self.acceleration * 2.2 * frame_time,
            );
        } else if thrust {
            self.speed = move_towards(self.speed, self.max_speed, self.acceleration * frame_time);
        } else {
            self.speed = (self.speed - self.deceleration * frame_time).max(0.0);
        }
    }

    fn apply_model(&self, uniforms: &mut Uniforms) {
        let ship_direction = self.orientation.to_matrix();
        let local_attitude = Matrix::rotate_y(self.ship_yaw)
            * Matrix::rotate_x(self.ship_pitch)
            * Matrix::rotate_z(self.ship_roll);
        uniforms.model_matrix = Matrix::translate(
            -self.model_center.x,
            -self.model_center.y,
            -self.model_center.z,
        ) * local_attitude
            * ship_direction
            * Matrix::translate(self.target.x, self.target.y, self.target.z);
    }
}

fn transform_direction(matrix: &Matrix, direction: Vector3) -> Vector3 {
    Vector3::new(
        matrix.m0 * direction.x + matrix.m4 * direction.y + matrix.m8 * direction.z,
        matrix.m1 * direction.x + matrix.m5 * direction.y + matrix.m9 * direction.z,
        matrix.m2 * direction.x + matrix.m6 * direction.y + matrix.m10 * direction.z,
    )
}

fn axis_value(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}
