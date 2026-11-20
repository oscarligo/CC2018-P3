use super::*;

impl ShipMovement {
    /// Match the target's orbital translation and slowly circle it during weapon operation.
    pub fn weapon_station(
        &mut self,
        previous: &crate::scene::world::TravelTarget,
        current: &crate::scene::world::TravelTarget,
        dt: f32,
        uniforms: &mut Uniforms,
        world: &crate::scene::world::World,
    ) {
        let offset = self.target - previous.position;
        let a = dt * 0.016;
        let rotated = Vector3::new(
            offset.x * a.cos() + offset.z * a.sin(),
            offset.y,
            -offset.x * a.sin() + offset.z * a.cos(),
        );
        let desired = current.position + rotated;
        self.target = world
            .sweep_vehicle(self.target, desired - self.target, self.radius)
            .map_or(desired, |h| h.safe_position);
        self.orientation = Quaternion::from_vector3_to_vector3(
            Vector3::new(0., 0., 1.),
            (current.position - self.target).normalize(),
        );
        self.local = None;
        self.speed = 0.;
        self.recoil = Vector3::zero();
        self.pitch_rate = 0.;
        self.yaw_rate = 0.;
        self.ship_pitch = 0.;
        self.ship_roll = 0.;
        self.ship_yaw = 0.;
        self.effects.clear();
        self.impact.update(dt);
        self.clock += dt;
        self.camera.snap(self.orientation);
        self.camera.update(
            &InputState::default(),
            dt,
            self.target,
            self.orientation,
            false,
            uniforms,
        );
        // Three-quarter view: the ship and the target both remain visible.
        let m = self.orientation.to_matrix();
        let right = transform_direction(&m, Vector3::new(1., 0., 0.));
        let up = transform_direction(&m, Vector3::new(0., 1., 0.));
        let forward = (current.position - self.target).normalize();
        let desired_camera = self.target + right * self.radius * 2.8 + up * self.radius * 1.05
            - forward * self.radius * 3.5;
        uniforms.camera_pos = world
            .sweep_vehicle(self.target, desired_camera - self.target, self.radius * 0.1)
            .map_or(desired_camera, |h| h.safe_position);
        uniforms.camera_target = self.target + forward * self.radius * 3.0;
        uniforms.camera_up = up;
        self.apply_model(uniforms);
    }

    /// Arrive outside the surface and rings, stopped and facing the destination.
    pub fn teleport(
        &mut self,
        destination: &crate::scene::world::TravelTarget,
        world: &crate::scene::world::World,
    ) -> bool {
        let clearance = (self.camera.reference_distance() * 4.0).max(90.0);
        let mut arrival = None;
        let mut overlaps = Vec::new();
        for attempt in 0..64 {
            let angle = attempt as f32 * 2.399963;
            let direction = Vector3::new(angle.sin() + 0.42, 0.28, -angle.cos()).normalize();
            let distance = destination.radius * (3.6 + (attempt / 16) as f32 * 0.8) + clearance;
            let position = destination.position + direction * distance;
            world.query_sphere(
                position,
                self.camera.reference_distance() * 1.5,
                &mut overlaps,
            );
            if overlaps.is_empty() {
                arrival = Some((position, direction));
                break;
            }
        }
        let Some((position, direction)) = arrival else {
            return false;
        };
        self.target = position;
        self.local = None;
        self.orientation =
            Quaternion::from_vector3_to_vector3(Vector3::new(0., 0., 1.), -direction);
        self.speed = 0.0;
        self.pitch_rate = 0.0;
        self.yaw_rate = 0.0;
        self.ship_pitch = 0.0;
        self.ship_yaw = 0.0;
        self.ship_roll = 0.0;
        self.camera.snap(self.orientation);
        self.effects.clear();
        self.impact.clear();
        self.recoil = Vector3::zero();
        self.impact_cooldown = 0.0;
        if let Some(alien) = &mut self.alien {
            alien.reset();
        }
        true
    }
}
