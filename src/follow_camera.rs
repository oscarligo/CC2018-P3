use crate::input::InputState;
use crate::uniforms::Uniforms;
use raylib::prelude::{Matrix, Quaternion, Vector3};

const INITIAL_DISTANCE_FACTOR: f32 = 2.6;
const MIN_DISTANCE_FACTOR: f32 = 1.2;
const MAX_DISTANCE_FACTOR: f32 = 8.0;
const ZOOM_SENSITIVITY: f32 = 0.1;
const HEIGHT_FACTOR: f32 = 0.25;
const FOLLOW_STIFFNESS: f32 = 6.5;
const THRUST_PULLBACK: f32 = 1.12;
const BOOST_PULLBACK: f32 = 1.42;
const BRAKE_PUSHIN: f32 = 0.85;
const ORBIT_SENSITIVITY: f32 = 0.008;
const MAX_ORBIT_PITCH: f32 = 80.0_f32.to_radians();
const RETURN_DELAY: f32 = 1.2;
const RETURN_SPEED: f32 = 2.5;
const ALIGNMENT_EPSILON: f32 = 0.001;

pub struct FollowCamera {
    distance: f32,
    effective_distance: f32,
    min_distance: f32,
    max_distance: f32,
    orbit_pitch: f32,
    orbit_yaw: f32,
    return_timer: f32,
    orientation: Quaternion,
}

impl FollowCamera {
    pub fn new(target_radius: f32) -> Self {
        let distance = target_radius * INITIAL_DISTANCE_FACTOR;
        Self {
            distance,
            effective_distance: distance,
            min_distance: target_radius * MIN_DISTANCE_FACTOR,
            max_distance: target_radius * MAX_DISTANCE_FACTOR,
            orbit_pitch: 0.0,
            orbit_yaw: 0.0,
            return_timer: 0.0,
            orientation: Quaternion::identity(),
        }
    }

    pub fn update(
        &mut self,
        input: &InputState,
        frame_time: f32,
        target: Vector3,
        target_orientation: Quaternion,
        uniforms: &mut Uniforms,
    ) {
        if input.zoom_delta != 0.0 {
            self.distance = (self.distance * (1.0 - input.zoom_delta * ZOOM_SENSITIVITY))
                .clamp(self.min_distance, self.max_distance);
        }

        let distance_multiplier = if input.braking {
            BRAKE_PUSHIN
        } else if input.boost {
            BOOST_PULLBACK
        } else if input.thrust {
            THRUST_PULLBACK
        } else {
            1.0
        };
        let desired_distance = (self.distance * distance_multiplier)
            .clamp(self.min_distance * 0.85, self.max_distance * 1.3);
        self.effective_distance +=
            (desired_distance - self.effective_distance) * (5.0 * frame_time).min(1.0);

        let follow_blend = (FOLLOW_STIFFNESS * frame_time).min(1.0);
        self.orientation = self
            .orientation
            .slerp(target_orientation, follow_blend)
            .normalize();

        if input.orbiting {
            self.orbit_yaw -= input.mouse_delta.x * ORBIT_SENSITIVITY;
            self.orbit_yaw = self.orbit_yaw.sin().atan2(self.orbit_yaw.cos());
            self.orbit_pitch = (self.orbit_pitch + input.mouse_delta.y * ORBIT_SENSITIVITY)
                .clamp(-MAX_ORBIT_PITCH, MAX_ORBIT_PITCH);
            self.return_timer = RETURN_DELAY;
        } else if !self.is_aligned() {
            self.return_timer = (self.return_timer - frame_time).max(0.0);
            if self.return_timer == 0.0 {
                let step = RETURN_SPEED * frame_time;
                self.orbit_pitch = move_towards(self.orbit_pitch, 0.0, step);
                self.orbit_yaw = move_towards(self.orbit_yaw, 0.0, step);
            }
        }

        let rotation = Matrix::rotate_x(self.orbit_pitch)
            * Matrix::rotate_y(self.orbit_yaw)
            * self.orientation.to_matrix();
        let distance = self.effective_distance;
        uniforms.camera_pos = target
            + transform_direction(
                &rotation,
                Vector3::new(0.0, distance * HEIGHT_FACTOR, -distance),
            );
        uniforms.camera_target = target;
        uniforms.camera_up = transform_direction(&rotation, Vector3::new(0.0, 1.0, 0.0));
    }

    pub fn effective_distance(&self) -> f32 {
        self.effective_distance
    }

    pub fn reference_distance(&self) -> f32 {
        self.min_distance * INITIAL_DISTANCE_FACTOR
    }

    fn is_aligned(&self) -> bool {
        self.orbit_pitch.abs() <= ALIGNMENT_EPSILON && self.orbit_yaw.abs() <= ALIGNMENT_EPSILON
    }
}

fn transform_direction(matrix: &Matrix, direction: Vector3) -> Vector3 {
    Vector3::new(
        matrix.m0 * direction.x + matrix.m4 * direction.y + matrix.m8 * direction.z,
        matrix.m1 * direction.x + matrix.m5 * direction.y + matrix.m9 * direction.z,
        matrix.m2 * direction.x + matrix.m6 * direction.y + matrix.m10 * direction.z,
    )
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}
