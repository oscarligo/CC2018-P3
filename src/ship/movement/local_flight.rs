use super::navigation::NAVIGATION_TURN_SPEED;
use super::*;

pub(super) const LOCAL_PITCH_LIMIT: f32 = 28.0_f32.to_radians();
const LOCAL_LEVEL_STIFFNESS: f32 = 4.0;

impl ShipMovement {
    pub(super) fn update_local(
        &mut self,
        input: &InputState,
        dt: f32,
        world: &crate::scene::world::World,
    ) {
        self.local_notice = (self.local_notice - dt).max(0.0);
        if dt <= 0.0 {
            return;
        }
        if input.local_pressed && self.local.take().is_none() {
            if let Some(body) = world.navigation_target(None, self.target)
                && (self.target - body.position).length() <= body.radius * 6.0 + self.radius * 8.0
            {
                let mut local = crate::ship::local::LocalFlight::new(body);
                local.approach_altitude = Some((body.radius * 0.05).max(self.radius * 3.5));
                self.orientation = local.tangent_orientation(self.target, self.orientation);
                self.local = Some(local);
                if let Some(alien) = &mut self.alien {
                    alien.reset();
                }
            } else {
                self.local_notice = 3.0;
            }
        }
        if let Some(local) = &mut self.local {
            if let Some(body) = world.navigation_target(Some(local.body.name), self.target)
                && (self.target - body.position).length() < body.radius * 9.0 + self.radius * 8.0
            {
                let delta = body.position - local.body.position;
                self.target = world
                    .sweep_vehicle(self.target, delta, self.radius)
                    .map_or(self.target + delta, |hit| hit.safe_position);
                local.body = body;
            } else {
                self.local = None;
            }
        }
        self.approach_local_surface(input, dt, world);
    }

    /// Ease into a low flyover without teleporting or moving through another body.
    fn approach_local_surface(
        &mut self,
        input: &InputState,
        dt: f32,
        world: &crate::scene::world::World,
    ) {
        let Some(local) = &mut self.local else {
            return;
        };
        if input.pitch_up || input.pitch_down {
            local.approach_altitude = None;
        }
        let Some(altitude) = local.approach_altitude else {
            return;
        };
        if input.braking || !local.cruising {
            return;
        }
        let radial = self.target - local.body.position;
        let distance = radial.length();
        let remaining = distance - local.body.radius - altitude;
        if remaining <= self.radius * 0.01 {
            local.approach_altitude = None;
            return;
        }
        let max_speed = (local.body.radius * 0.35).max(self.radius * 6.0);
        let step = (remaining * (1.0 - (-1.5 * dt).exp())).min(max_speed * dt);
        let delta = radial * (-step / distance);
        if let Some(hit) = world.sweep_vehicle(self.target, delta, self.radius) {
            self.target = hit.safe_position;
            local.approach_altitude = None;
        } else {
            self.target += delta;
        }
    }
    pub(super) fn update_local_heading(&mut self, frame_time: f32) -> bool {
        let Some(local) = &self.local else {
            return false;
        };
        let up = (self.target - local.body.position).normalize();
        let forward = transform_direction(&self.orientation.to_matrix(), Vector3::new(0., 0., 1.));
        let pitch = forward.dot(up).clamp(-1., 1.).asin();
        let desired_pitch = -self.pitch_rate * LOCAL_PITCH_LIMIT;
        let pitch =
            pitch + (desired_pitch - pitch) * (1. - (-LOCAL_LEVEL_STIFFNESS * frame_time).exp());
        let level = local.tangent_orientation(self.target, self.orientation);
        let yaw = Quaternion::from_axis_angle(
            Vector3::new(0., 1., 0.),
            self.yaw_rate * NAVIGATION_TURN_SPEED * frame_time,
        );
        self.orientation =
            (level * yaw * Quaternion::from_axis_angle(Vector3::new(1., 0., 0.), -pitch))
                .normalize();
        true
    }

    pub(super) fn update_local_speed(&mut self, thrust: bool, braking: bool, dt: f32) -> bool {
        let Some(local) = &mut self.local else {
            return false;
        };
        if braking {
            local.cruising = false;
        } else if thrust {
            local.cruising = true;
        }
        let limit = local.speed_limit(self.target, self.radius);
        let target = if local.cruising {
            limit * if thrust { 1.45 } else { 1.0 }
        } else {
            0.0
        };
        let acceleration = if braking || self.speed > target {
            (limit * 3.0).max(self.brake_deceleration)
        } else {
            (limit * 0.8).max(self.acceleration)
        };
        self.speed = move_towards(self.speed, target, acceleration * dt);
        true
    }
}
