use super::alien::{beam, glow};
use crate::{
    physics::{ObjectId, Ray},
    rendering::{framebuffer::Framebuffer, projection::project_vertex, uniforms::Uniforms},
    scene::world::World,
};
use raylib::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;

const RED: Vector3 = Vector3 {
    x: 1.0,
    y: 0.04,
    z: 0.02,
};
const RANGE: f32 = 100_000.0;
const MAX_EXPLOSION_PARTICLES: usize = 360;
const PLANET_HEALTH: u16 = 24;
const MAX_CRACKS: usize = 144;

struct Bolt {
    side: usize,
    origin: Vector3,
    end: Vector3,
    life: f32,
}

struct ExplosionParticle {
    position: Vector3,
    velocity: Vector3,
    age: f32,
    life: f32,
    size: f32,
    hot: bool,
}

struct Crack {
    planet: ObjectId,
    start: Vector3,
    end: Vector3,
}

pub struct CarWeapons {
    radius: f32,
    bolts: Vec<Bolt>,
    particles: Vec<ExplosionParticle>,
    rng: u32,
    next_side: usize,
    hold_time: f32,
    continuous: bool,
    impact_cooldown: f32,
    damage: HashMap<ObjectId, u16>,
    cracks: Vec<Crack>,
}

impl CarWeapons {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            bolts: Vec::with_capacity(2),
            particles: Vec::with_capacity(96),
            rng: 0xc4a2_51d7,
            next_side: 0,
            hold_time: 0.0,
            continuous: false,
            impact_cooldown: 0.0,
            damage: HashMap::new(),
            cracks: Vec::with_capacity(MAX_CRACKS),
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        firing_pressed: bool,
        firing_held: bool,
        muzzles: [Vector3; 2],
        forward: Vector3,
        world: &mut World,
    ) -> bool {
        self.impact_cooldown = (self.impact_cooldown - dt).max(0.0);
        for bolt in &mut self.bolts {
            bolt.origin = muzzles[bolt.side];
            bolt.life -= dt;
        }
        self.bolts.retain(|bolt| bolt.life > 0.0);
        for particle in &mut self.particles {
            particle.age += dt;
            particle.position += particle.velocity * dt;
            particle.velocity *= (-2.3 * dt).exp();
        }
        self.particles
            .retain(|particle| particle.age < particle.life);
        let was_continuous = self.continuous;
        if firing_held {
            self.hold_time += dt;
        } else {
            self.hold_time = 0.0;
            self.continuous = false;
        }

        let mut sound = false;
        if firing_pressed {
            let side = self.next_side;
            self.next_side = (self.next_side + 1) % 2;
            self.fire(side, muzzles, forward, world, true);
            sound = true;
        }
        if firing_held && self.hold_time >= 0.16 {
            self.continuous = true;
            self.bolts.clear();
            let explode = self.impact_cooldown <= 0.0;
            self.fire(0, muzzles, forward, world, explode);
            self.fire(1, muzzles, forward, world, explode);
            if explode {
                self.impact_cooldown = 0.10;
            }
            sound |= !was_continuous;
        }
        sound
    }

    fn fire(
        &mut self,
        side: usize,
        muzzles: [Vector3; 2],
        forward: Vector3,
        world: &mut World,
        explode: bool,
    ) {
        let muzzle = muzzles[side];
        let Some(ray) = Ray::new(muzzle, forward) else {
            return;
        };
        let hit = world.raycast(ray, RANGE);
        let end = hit.map_or(muzzle + ray.direction * RANGE, |hit| hit.position);
        if explode && let Some(hit) = hit {
            if let Some(target) = world.target_by_id(hit.object_id) {
                self.explosion(hit.position, ray.direction);
                self.damage_planet(target, hit.position, world);
            }
        }
        self.bolts.push(Bolt {
            side,
            origin: muzzle,
            end,
            life: 0.12,
        });
    }

    fn damage_planet(
        &mut self,
        target: crate::scene::world::TravelTarget,
        hit: Vector3,
        world: &mut World,
    ) {
        let health = self.damage.entry(target.id).or_default();
        *health += 1;
        let destroyed = *health >= PLANET_HEALTH;
        self.add_cracks(target, hit);
        if destroyed {
            for _ in 0..3 {
                self.explosion(hit, (hit - target.position).normalize());
            }
            world.destroy_planet(target.id);
            self.damage.remove(&target.id);
            self.cracks.retain(|crack| crack.planet != target.id);
        }
    }

    fn add_cracks(&mut self, target: crate::scene::world::TravelTarget, hit: Vector3) {
        let normal = (hit - target.position).normalize();
        let reference = if normal.y.abs() < 0.9 {
            Vector3::new(0.0, 1.0, 0.0)
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };
        let tangent = normal.cross(reference).normalize();
        let bitangent = normal.cross(tangent).normalize();
        for _ in 0..3.min(MAX_CRACKS.saturating_sub(self.cracks.len())) {
            let angle = self.random() * TAU;
            let length = 0.025 + self.random() * 0.065;
            let start = (normal
                + tangent * ((self.random() - 0.5) * 0.018)
                + bitangent * ((self.random() - 0.5) * 0.018))
                .normalize();
            let end =
                (start + tangent * (angle.cos() * length) + bitangent * (angle.sin() * length))
                    .normalize();
            self.cracks.push(Crack {
                planet: target.id,
                start,
                end,
            });
        }
    }

    fn explosion(&mut self, position: Vector3, incoming: Vector3) {
        let count = 36.min(MAX_EXPLOSION_PARTICLES.saturating_sub(self.particles.len()));
        for index in 0..count {
            let y = self.random() * 2.0 - 1.0;
            let angle = self.random() * TAU;
            let radial = (1.0 - y * y).sqrt();
            let random_direction = Vector3::new(radial * angle.cos(), y, radial * angle.sin());
            let direction = (random_direction - incoming * 0.45).normalize();
            let speed = self.radius * (2.0 + self.random() * 5.0);
            let life = 0.32 + self.random() * 0.55;
            let size = self.radius * (0.045 + self.random() * 0.09);
            self.particles.push(ExplosionParticle {
                position,
                velocity: direction * speed,
                age: 0.0,
                life,
                size,
                hot: index % 3 != 0,
            });
        }
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    pub fn draw(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms, world: &World) {
        for crack in &self.cracks {
            let Some(target) = world.target_by_id(crack.planet) else {
                continue;
            };
            let lift = target.radius * 1.006;
            beam(
                framebuffer,
                uniforms,
                target.position + crack.start * lift,
                target.position + crack.end * lift,
                Vector3::new(1.0, 0.16, 0.01),
                0.7,
            );
        }
        for bolt in &self.bolts {
            beam(framebuffer, uniforms, bolt.origin, bolt.end, RED, 2.6);
            if let Some((position, depth)) = project_vertex(
                bolt.origin,
                uniforms.camera_pos,
                uniforms.camera_target,
                uniforms.camera_up,
                framebuffer.width as f32,
                framebuffer.height as f32,
            ) {
                glow(framebuffer, position, depth, 10.0, RED, 0.65);
            }
        }
        for particle in &self.particles {
            let Some((position, depth)) = project_vertex(
                particle.position,
                uniforms.camera_pos,
                uniforms.camera_target,
                uniforms.camera_up,
                framebuffer.width as f32,
                framebuffer.height as f32,
            ) else {
                continue;
            };
            let remaining = (1.0 - particle.age / particle.life).max(0.0);
            let size = (particle.size / depth * framebuffer.height as f32).clamp(1.0, 18.0);
            let color = if particle.hot {
                Vector3::new(1.0, 0.28, 0.03)
            } else {
                Vector3::new(0.75, 0.04, 0.02)
            };
            glow(framebuffer, position, depth, size, color, remaining * 0.8);
        }
    }

    pub fn draw_hud(&self, drawing: &mut RaylibDrawHandle, uniforms: &Uniforms) {
        let nose = crate::model_point(&uniforms.model_matrix, Vector3::new(0.0, 0.0, RANGE));
        let point = project_vertex(
            nose,
            uniforms.camera_pos,
            uniforms.camera_target,
            uniforms.camera_up,
            800.0,
            600.0,
        )
        .map_or(Vector2::new(400.0, 300.0), |(point, _)| point);
        let (x, y) = (point.x as i32, point.y as i32);
        let color = Color::WHITE;
        for (x1, y1, x2, y2) in [
            (x - 12, y, x - 4, y),
            (x + 4, y, x + 12, y),
            (x, y - 12, x, y - 4),
            (x, y + 4, x, y + 12),
        ] {
            drawing.draw_line(x1, y1, x2, y2, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{celestial_shaders::CelestialKind, objects::celestial::CelestialBody};

    #[test]
    fn repeated_clicks_alternate_sides_and_do_not_destroy_the_planet() {
        let mut world = World::new();
        let planet = world.add(
            CelestialBody::new(CelestialKind::Rocky, Vector3::new(0.0, 0.0, 80.0))
                .named("Planeta de prueba")
                .with_scale(8.0),
        );
        let mut weapons = CarWeapons::new(2.0);

        assert!(weapons.update(
            1.0 / 60.0,
            true,
            true,
            [Vector3::new(-1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)],
            Vector3::new(0.0, 0.0, 1.0),
            &mut world,
        ));

        assert_eq!(weapons.bolts.len(), 1);
        assert_eq!(weapons.bolts[0].side, 0);
        assert_eq!(weapons.particles.len(), 36);
        weapons.update(
            0.13,
            false,
            false,
            [Vector3::new(-1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)],
            Vector3::new(0.0, 0.0, 1.0),
            &mut world,
        );
        weapons.update(
            1.0 / 60.0,
            true,
            true,
            [Vector3::new(-1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)],
            Vector3::new(0.0, 0.0, 1.0),
            &mut world,
        );
        assert_eq!(weapons.bolts.len(), 1);
        assert_eq!(weapons.bolts[0].side, 1);
        assert_eq!(weapons.cracks.len(), 6);
        assert!(world.target_by_id(planet).is_some());
    }

    #[test]
    fn held_click_keeps_two_beams_attached_to_moving_muzzles() {
        let mut world = World::new();
        let mut weapons = CarWeapons::new(2.0);
        let mut muzzles = [Vector3::new(-1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)];
        for frame in 0..12 {
            weapons.update(
                1.0 / 60.0,
                frame == 0,
                true,
                muzzles,
                Vector3::new(0.0, 0.0, 1.0),
                &mut world,
            );
        }
        assert_eq!(weapons.bolts.len(), 2);

        muzzles = [Vector3::new(-1.0, 0.0, 25.0), Vector3::new(1.0, 0.0, 25.0)];
        weapons.update(
            1.0 / 60.0,
            false,
            true,
            muzzles,
            Vector3::new(0.0, 0.0, 1.0),
            &mut world,
        );
        assert_eq!(weapons.bolts.len(), 2);
        assert_eq!(weapons.bolts[0].origin, muzzles[0]);
        assert_eq!(weapons.bolts[1].origin, muzzles[1]);
    }

    #[test]
    fn enough_laser_hits_destroy_the_planet_and_clear_its_cracks() {
        let mut world = World::new();
        let planet = world.add(
            CelestialBody::new(CelestialKind::Rocky, Vector3::new(0.0, 0.0, 80.0))
                .named("Planeta frágil")
                .with_scale(8.0),
        );
        let mut weapons = CarWeapons::new(2.0);
        for _ in 0..PLANET_HEALTH {
            weapons.update(
                0.13,
                true,
                false,
                [Vector3::new(-1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)],
                Vector3::new(0.0, 0.0, 1.0),
                &mut world,
            );
        }

        assert!(world.target_by_id(planet).is_none());
        assert!(!weapons.damage.contains_key(&planet));
        assert!(weapons.cracks.iter().all(|crack| crack.planet != planet));
    }
}
