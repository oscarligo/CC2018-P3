//! A moving planetary reference frame, controlled by the pilot rather than an autopilot.
use crate::scene::world::TravelTarget;
use raylib::prelude::*;

pub(super) struct LocalFlight {
    pub body: TravelTarget,
}

impl LocalFlight {
    /// Follow a great-circle arc, keeping pitch as the pilot's climb/descent input.
    pub fn velocity(&self, position: Vector3, velocity: Vector3, dt: f32) -> Vector3 {
        if dt <= 0.0 {
            return velocity;
        }
        let radial = position - self.body.position;
        let distance = radial.length().max(0.001);
        let up = radial * (1.0 / distance);
        let climb = velocity.dot(up);
        let tangent = velocity - up * climb;
        let speed = tangent.length();
        let angle = speed * dt / distance;
        let next_up = if speed > 0.0001 {
            up * angle.cos() + tangent * (1.0 / speed) * angle.sin()
        } else {
            up
        };
        (self.body.position + next_up * (distance + climb * dt) - position) * (1.0 / dt)
    }

    pub fn tangent_orientation(&self, position: Vector3, orientation: Quaternion) -> Quaternion {
        let up = (position - self.body.position).normalize();
        let forward = direction(orientation, Vector3::new(0., 0., 1.));
        let tangent = forward - up * forward.dot(up);
        let aligned = Quaternion::from_vector3_to_vector3(Vector3::new(0., 1., 0.), up);
        let initial = direction(aligned, Vector3::new(0., 0., 1.));
        let tangent = if tangent.length_sqr() > 0.0001 {
            tangent.normalize()
        } else {
            initial
        };
        let angle = up.dot(initial.cross(tangent)).atan2(initial.dot(tangent));
        (Quaternion::from_axis_angle(up, angle) * aligned).normalize()
    }
}

fn direction(rotation: Quaternion, value: Vector3) -> Vector3 {
    let m = rotation.to_matrix();
    Vector3::new(
        m.m0 * value.x + m.m4 * value.y + m.m8 * value.z,
        m.m1 * value.x + m.m5 * value.y + m.m9 * value.z,
        m.m2 * value.x + m.m6 * value.y + m.m10 * value.z,
    )
}
