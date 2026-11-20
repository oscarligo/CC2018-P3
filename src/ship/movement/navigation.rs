use super::*;

const STEER_ANGULAR_ACCEL: f32 = 7.0;
pub(super) const NAVIGATION_TURN_SPEED: f32 = 1.35;

impl ShipMovement {
    pub(super) fn world_velocity(&self) -> Vector3 {
        transform_direction(&self.orientation.to_matrix(), Vector3::new(0.0, 0.0, 1.0)) * self.speed
    }

    pub(super) fn update_navigation(&mut self, input: &InputState, frame_time: f32) {
        let target_pitch = axis_value(input.pitch_down, input.pitch_up);
        let target_yaw = axis_value(input.turn_right, input.turn_left);
        let roll_turn_assist = axis_value(input.bank_right, input.bank_left) * 0.45;
        let blend = 1.0 - (-STEER_ANGULAR_ACCEL * frame_time).exp();
        self.pitch_rate += (target_pitch - self.pitch_rate) * blend;
        self.yaw_rate += ((target_yaw + roll_turn_assist) - self.yaw_rate) * blend;
        if self.update_local_heading(frame_time) {
            return;
        }
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

    pub(super) fn update_motion(
        &mut self,
        thrust: bool,
        boost: bool,
        braking: bool,
        frame_time: f32,
    ) {
        if self.update_local_speed(thrust || boost, braking, frame_time) {
            return;
        }
        if braking {
            let deceleration = if self.unbounded_thrust {
                self.brake_deceleration.max(self.speed * 1.5)
            } else {
                self.brake_deceleration
            };
            self.speed = (self.speed - deceleration * frame_time).max(0.0);
        } else if self.unbounded_thrust && (thrust || boost) {
            let acceleration = self.acceleration * if boost { 2.2 } else { 1.0 };
            self.speed += acceleration * frame_time;
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
}
