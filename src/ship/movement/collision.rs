use super::*;

impl ShipMovement {
    pub(super) fn advance_with_collisions(&mut self, dt: f32, world: &crate::scene::world::World) {
        if dt <= 0.0 {
            return;
        }
        let mut remaining = dt;
        let forward = self.world_velocity();
        let mut velocity = self
            .local
            .as_ref()
            .map_or(forward, |local| local.velocity(self.target, forward, dt))
            + self.recoil;
        for _ in 0..6 {
            let delta = velocity * remaining;
            let Some(hit) = world.sweep_vehicle(self.target, delta, self.radius) else {
                self.target += delta;
                break;
            };
            self.target = hit.safe_position;
            let incoming = velocity.dot(hit.normal);
            if incoming < 0.0 {
                velocity -= hit.normal * (1.65 * incoming);
            }
            let outward = velocity.dot(hit.normal);
            velocity += hit.normal * (self.max_speed * 1.2 - outward).max(0.0);
            self.recoil = velocity;
            self.speed = 0.0;
            if self.impact_cooldown <= 0.0 {
                self.impact.burst(
                    self.target - hit.normal * self.radius * 0.7,
                    hit.normal,
                    self.radius,
                );
                self.impact_cooldown = 0.20;
                if let Some(alien) = &mut self.alien {
                    alien.collide();
                }
                self.effects.clear();
            }
            remaining *= 1.0 - hit.fraction;
            if remaining < 0.00001 {
                break;
            }
        }
        self.recoil *= (-2.4 * dt).exp();
    }
}
