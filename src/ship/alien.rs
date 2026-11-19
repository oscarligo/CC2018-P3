//! UFO toys: time-based flight modes, world-space lasers and CPU additive light.
use super::{commands::Command, input::InputState};
use crate::{
    physics::{Ray, raycast_sphere},
    rendering::{
        framebuffer::Framebuffer,
        projection::{camera_depth, project_vertex},
        uniforms::Uniforms,
    },
    scene::world::World,
};
use raylib::prelude::*;
use std::f32::consts::{PI, TAU};

const CHARGE: f32 = 1.05;
const RUSH: f32 = 3.8;
const GREEN: Vector3 = Vector3 {
    x: 0.12,
    y: 1.0,
    z: 0.22,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Charge,
    Sneeze,
    Laser,
    Chase,
    Impact,
}

pub struct Drive {
    pub direction: Vector3,
    pub speed_factor: f32,
    pub steering: f32,
}
struct Puff {
    position: Vector3,
    velocity: Vector3,
    age: f32,
    life: f32,
    size: f32,
}
struct Bolt {
    origin: Vector3,
    end: Vector3,
    life: f32,
}
#[derive(Clone, Copy)]
struct Target {
    position: Vector3,
    radius: f32,
}

pub struct AlienFlight {
    pub aiming: bool,
    sneeze: Option<f32>,
    heading: Quaternion,
    clock: f32,
    spin: f32,
    spin_velocity: f32,
    radius: f32,
    pub aim: Option<Vector3>,
    cursor: Vector2,
    targets: Vec<Target>,
    orbit_phase: f32,
    orbit_center: Vector3,
    orbit_orientation: Quaternion,
    orbit_trail: std::collections::VecDeque<(f32, f32)>,
    respawn: f32,
    muzzle: Vector3,
    shot_timer: f32,
    emission: f32,
    flash: f32,
    puffs: Vec<Puff>,
    bolts: Vec<Bolt>,
    cues: Vec<Cue>,
    rng: u32,
    pub hits: u32,
    finished: bool,
}

impl AlienFlight {
    pub fn new(radius: f32) -> Self {
        Self {
            aiming: false,
            sneeze: None,
            heading: Quaternion::identity(),
            clock: 0.,
            spin: 0.,
            spin_velocity: 0.,
            radius,
            aim: None,
            cursor: Vector2::new(400., 300.),
            targets: Vec::new(),
            orbit_phase: 0.0,
            orbit_center: Vector3::zero(),
            orbit_orientation: Quaternion::identity(),
            orbit_trail: std::collections::VecDeque::new(),
            respawn: 0.0,
            muzzle: Vector3::zero(),
            shot_timer: 0.,
            emission: 0.,
            flash: 0.,
            puffs: Vec::new(),
            bolts: Vec::new(),
            cues: Vec::new(),
            rng: 0xCA7_A11E,
            hits: 0,
            finished: false,
        }
    }
    pub fn command(&mut self, command: Command, heading: Quaternion) {
        match command {
            Command::Sneeze if self.sneeze.is_none() => {
                self.aiming = false;
                self.aim = None;
                self.sneeze = Some(0.);
                self.heading = heading;
                self.cues.push(Cue::Charge);
            }
            Command::Chase => {
                self.sneeze = None;
                self.aiming = !self.aiming;
                self.targets.clear();
                self.orbit_trail.clear();
                self.respawn = 0.0;
                self.aim = None;
                self.cues.push(Cue::Chase);
            }
            _ => {}
        }
    }
    pub fn reset(&mut self) {
        self.aiming = false;
        self.sneeze = None;
        self.aim = None;
        self.puffs.clear();
        self.bolts.clear();
        self.targets.clear();
        self.orbit_trail.clear();
        self.respawn = 0.0;
        self.cues.clear();
        self.spin = 0.;
        self.spin_velocity = 0.;
        self.flash = 0.;
        self.emission = 0.;
        self.shot_timer = 0.;
    }
    pub fn take_cues(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }
    pub fn just_finished(&self) -> bool {
        self.finished
    }
    pub fn active(&self) -> bool {
        self.aiming || self.sneeze.is_some()
    }
    pub fn phase(&self) -> &'static str {
        match self.sneeze {
            Some(t) if t < CHARGE => "A... A... ACHU!",
            Some(_) => "ACHUUUU / SIN CONTROL",
            None if self.aiming => "MISHMISH / PUNTERIA",
            _ => "OVNI / VUELO LIBRE",
        }
    }
    pub fn visual_rotation(&self) -> Vector3 {
        let intensity = match self.sneeze {
            Some(t) if t < CHARGE => 0.07 + t * 0.13,
            Some(_) => 0.48,
            _ => 0.0,
        };
        Vector3::new(
            (self.clock * 31.).sin() * intensity,
            self.spin,
            (self.clock * 23.).cos() * intensity
                + self.spin_velocity * 0.015 * (self.clock * 12.).sin(),
        )
    }
    pub fn visual_scale(&self) -> Vector3 {
        let charge = self
            .sneeze
            .filter(|t| *t < CHARGE)
            .map_or(0., |t| t / CHARGE);
        Vector3::new(1. + charge * 0.15, 1. - charge * 0.25, 1. + charge * 0.15)
    }

    pub fn update(
        &mut self,
        input: &InputState,
        dt: f32,
        position: Vector3,
        _orientation: Quaternion,
        _max_speed: f32,
        _u: &Uniforms,
        _world: &World,
    ) -> Option<Drive> {
        self.finished = false;
        if dt <= 0. {
            return None;
        }
        self.clock += dt;
        self.flash = (self.flash - dt * 2.8).max(0.);
        self.shot_timer = (self.shot_timer - dt).max(0.);
        if input.spin_pressed {
            self.spin_velocity = (self.spin_velocity + 18.).min(30.);
        }
        if input.spin_held && !input.braking {
            self.spin_velocity += (20.0 - self.spin_velocity) * (1.0 - (-5.0 * dt).exp());
        }
        self.spin = (self.spin + self.spin_velocity * dt).rem_euclid(TAU);
        self.spin_velocity *= (-1.8 * dt).exp();
        if input.braking && self.active() {
            self.sneeze = None;
            self.aiming = false;
            self.aim = None;
            self.finished = true;
            self.cues.clear();
        }
        for puff in &mut self.puffs {
            puff.age += dt;
            puff.position += puff.velocity * dt;
            puff.velocity *= (-1.5 * dt).exp();
        }
        self.puffs.retain(|p| p.age < p.life);
        for bolt in &mut self.bolts {
            bolt.life -= dt;
        }
        self.bolts.retain(|b| b.life > 0.0);

        if let Some(previous) = self.sneeze {
            let t = previous + dt;
            self.sneeze = Some(t);
            if previous < CHARGE && t >= CHARGE {
                self.cues.push(Cue::Sneeze);
                self.flash = 1.;
                self.cloud(position, 110, 6.0);
            }
            if t >= CHARGE + RUSH {
                self.sneeze = None;
                self.finished = true;
                return None;
            }
            if t < CHARGE {
                return Some(Drive {
                    direction: rotate(self.heading, Vector3::new(0., 0., 1.)),
                    speed_factor: -0.20 * (t / CHARGE),
                    steering: 4.,
                });
            }
            let burst = t - CHARGE;
            let chaos = Vector3::new(
                (burst * 10.4).sin() * 0.62 + (burst * 3.7).sin() * 0.32,
                (burst * 8.3).sin() * 0.44,
                1.0,
            )
            .normalize();
            let envelope = (PI * (burst / RUSH).clamp(0., 1.)).sin().max(0.).powf(0.55);
            let direction = rotate(self.heading, chaos);
            self.emission += dt;
            while self.emission >= 0.018 {
                self.emission -= 0.018;
                self.cloud(position - direction * self.radius * 1.2, 4, 2.8);
            }
            return Some(Drive {
                direction,
                speed_factor: 3.0 + envelope * 22.0,
                steering: 10.,
            });
        }
        if !self.aiming {
            return None;
        }
        None
    }

    fn orbit_offset(&self, angle: f32) -> Vector3 {
        Vector3::new(angle.cos() * 1.12, angle.sin() * 0.60, angle.sin() * 0.44) * self.radius
    }

    /// Only the cat and elapsed time drive the target, never the cursor or the ship's steering.
    pub fn orbit_target(&mut self, pilot: Vector3, heading: Quaternion, dt: f32) {
        self.orbit_center = pilot;
        self.orbit_orientation = heading;
        if !self.aiming || dt <= 0.0 {
            return;
        }
        self.orbit_phase = (self.orbit_phase + dt * 1.15).rem_euclid(TAU);
        self.respawn = (self.respawn - dt).max(0.0);
        for (_, age) in &mut self.orbit_trail {
            *age += dt;
        }
        self.orbit_trail.retain(|(_, age)| *age < 0.72);
        let position = pilot + rotate(heading, self.orbit_offset(self.orbit_phase));
        if self.targets.is_empty() && self.respawn <= 0.0 {
            self.targets.push(Target {
                position,
                radius: self.radius * 0.13,
            });
        }
        if let Some(target) = self.targets.first_mut() {
            target.position = position;
            self.orbit_trail.push_back((self.orbit_phase, 0.0));
            while self.orbit_trail.len() > 40 {
                self.orbit_trail.pop_front();
            }
        }
    }

    /// Called after the current model and chase camera matrices are resolved.
    pub fn weapons(
        &mut self,
        input: &InputState,
        muzzle: Vector3,
        max_speed: f32,
        u: &Uniforms,
        world: &World,
    ) {
        self.muzzle = muzzle;
        for shot in &mut self.bolts {
            shot.origin = muzzle;
        }
        self.cursor = input.mouse_position;
        if !self.aiming {
            return;
        }
        let forward = (u.camera_target - u.camera_pos).normalize();
        let right = u.camera_up.cross(forward).normalize();
        let up = forward.cross(right);
        let x = (input.mouse_position.x / 800.0 * 2.0 - 1.0).clamp(-1.0, 1.0);
        let y = (1.0 - input.mouse_position.y / 600.0 * 2.0).clamp(-1.0, 1.0);
        let direction = (forward + right * (x * 0.7698) + up * (y * 0.57735)).normalize();
        let range = max_speed * 60.0;
        let camera_ray = Ray::new(u.camera_pos, direction).unwrap();
        let mut distance = world
            .raycast(camera_ray, range)
            .map_or(range, |hit| hit.distance);
        for target in &self.targets {
            if let Some(d) = raycast_sphere(camera_ray, target.position, target.radius, distance) {
                distance = d;
            }
        }
        let aim = u.camera_pos + direction * distance;
        self.aim = Some(aim);
        if !input.firing || self.shot_timer > 0.0 {
            return;
        }
        self.shot_timer = 0.10;
        // Camera ray chooses the aim point; the actual shot starts at the UFO's muzzle.
        let Some(ray) = Ray::new(muzzle, aim - muzzle) else {
            return;
        };
        let mut distance = (aim - muzzle).length() + 0.1;
        if let Some(hit) = world.raycast(ray, distance) {
            distance = hit.distance;
        }
        let mut scored = None;
        for (i, target) in self.targets.iter().enumerate() {
            if let Some(d) = raycast_sphere(ray, target.position, target.radius, distance) {
                distance = d;
                scored = Some(i);
            }
        }
        let end = muzzle + ray.direction * distance;
        if let Some(index) = scored {
            self.targets.remove(index);
            self.respawn = 0.30;
            self.orbit_trail.clear();
            self.hits += 1;
            self.cues.push(Cue::Chase);
            self.cloud(end, 28, 2.0);
        } else if distance < range * 0.99 {
            self.cloud(end, 14, 1.3);
        }
        self.bolts.push(Bolt {
            origin: muzzle,
            end,
            life: 0.11,
        });
        self.cues.push(Cue::Laser);
    }

    pub fn collide(&mut self) {
        self.sneeze = None;
        self.finished = true;
        self.cues.clear();
        self.bolts.clear();
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.
    }
    fn cloud(&mut self, center: Vector3, count: usize, scale: f32) {
        // Hard limit bounds both simulation and additive fill cost during repeated laser fire.
        for _ in 0..count.min(550usize.saturating_sub(self.puffs.len())) {
            let y = self.random() * 2. - 1.;
            let a = self.random() * TAU;
            let r = (1. - y * y).sqrt();
            let direction = Vector3::new(r * a.cos(), y, r * a.sin());
            let speed = self.radius * scale * (1.5 + self.random() * 3.);
            let life = 0.5 + self.random() * 1.0;
            let size = self.radius * (0.12 + self.random() * 0.25) * scale;
            self.puffs.push(Puff {
                position: center,
                velocity: direction * speed,
                age: 0.,
                life,
                size,
            });
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer, u: &Uniforms, ship: Vector3) {
        for puff in &self.puffs {
            if let Some((screen, depth)) = project(puff.position, fb, u) {
                let t = puff.age / puff.life;
                let size = (puff.size * (0.5 + t * 2.5) / depth * 520.).clamp(1., 72.);
                glow(fb, screen, depth, size, GREEN, (1. - t).powi(2) * 0.65);
                glow(
                    fb,
                    screen,
                    depth,
                    size * 0.22,
                    Vector3::new(0.65, 1., 0.45),
                    (1. - t) * 0.85,
                );
            }
        }
        for bolt in &self.bolts {
            // Keep the short pulse visibly connected to the craft, never to the spectator.
            beam(
                fb,
                u,
                bolt.origin,
                bolt.end,
                GREEN,
                2.0 + bolt.life / 0.11 * 1.5,
            );
            if let Some((screen, depth)) = project(self.muzzle, fb, u) {
                glow(fb, screen, depth, 13., GREEN, 0.7);
            }
        }
        if self.aiming {
            for &(angle, age) in &self.orbit_trail {
                let point =
                    self.orbit_center + rotate(self.orbit_orientation, self.orbit_offset(angle));
                if let Some((screen, depth)) = project(point, fb, u) {
                    let life = (1.0 - age / 0.72).max(0.0);
                    target_light(fb, screen, depth, 3.0 + life * 6.0, life);
                }
            }
            for target in &self.targets {
                if let Some((screen, depth)) = project(target.position, fb, u) {
                    let radius = target.radius / depth * 520.0;
                    target_light(fb, screen, depth, 5.0, 1.0);
                    for i in 0..64 {
                        let a = TAU * i as f32 / 64.0;
                        let p = screen + Vector2::new(a.cos(), a.sin()) * radius;
                        target_light(fb, p, depth, 2.2, 0.95);
                    }
                }
            }
        }
        if self.flash > 0.
            && let Some((screen, depth)) = project(ship, fb, u)
        {
            let size = 30. + (1. - self.flash) * 500.;
            glow(fb, screen, depth * 0.95, size, GREEN, self.flash * 0.16);
            for i in 0..100 {
                let a = TAU * i as f32 / 100.;
                glow(
                    fb,
                    screen + Vector2::new(a.cos(), a.sin()) * size,
                    0.01,
                    12.,
                    GREEN,
                    self.flash * 0.6,
                );
            }
        }
    }

    pub fn draw_hud(&self, d: &mut RaylibDrawHandle) {
        let blue = Color::new(124, 200, 255, 255);
        d.draw_rectangle(16, 52, 520, 60, Color::new(7, 18, 38, 205));
        d.draw_text(self.phase(), 28, 61, 18, blue);
        d.draw_text(
            "achu | mishmish | Clic: laser | Espacio: acelerar | R: giro",
            28,
            87,
            14,
            Color::new(185, 208, 238, 255),
        );
        if self.aiming {
            d.draw_text(&format!("Aciertos: {}", self.hits), 375, 61, 18, blue);
            let x = self.cursor.x as i32;
            let y = self.cursor.y as i32;
            d.draw_circle_lines(x, y, 8.0, blue);
            d.draw_line(x - 15, y, x - 5, y, blue);
            d.draw_line(x + 5, y, x + 15, y, blue);
            d.draw_line(x, y - 15, x, y - 5, blue);
            d.draw_line(x, y + 5, x, y + 15, blue);
        }
        if self.active() {
            d.draw_text("SHIFT: recuperar control", 28, 121, 15, blue);
        }
    }
}

fn rotate(q: Quaternion, v: Vector3) -> Vector3 {
    v.rotate_by_quaternion(q)
}
fn project(p: Vector3, fb: &Framebuffer, u: &Uniforms) -> Option<(Vector2, f32)> {
    project_vertex(
        p,
        u.camera_pos,
        u.camera_target,
        u.camera_up,
        fb.width as f32,
        fb.height as f32,
    )
}
fn target_light(fb: &mut Framebuffer, p: Vector2, depth: f32, radius: f32, power: f32) {
    glow(fb, p, depth, radius, GREEN, power * 0.45);
    // Preserve the green hue even against the orange solar disc.
    let core = (radius * 0.32).max(1.0);
    let extent = core.ceil() as i32;
    for y in -extent..=extent {
        for x in -extent..=extent {
            let falloff = (1.0 - (x * x + y * y) as f32 / (core * core)).max(0.0);
            if falloff > 0.0 {
                fb.blend_point(
                    p.x as i32 + x,
                    p.y as i32 + y,
                    depth,
                    GREEN,
                    power * falloff * 0.95,
                    false,
                );
            }
        }
    }
}

pub(super) fn glow(
    fb: &mut Framebuffer,
    p: Vector2,
    depth: f32,
    radius: f32,
    color: Vector3,
    power: f32,
) {
    let radius = radius.max(1.);
    let r = radius.ceil() as i32;
    let cx = p.x.round() as i32;
    let cy = p.y.round() as i32;
    let inverse = 1. / (radius * radius);
    for y in (-r).max(-cy)..=r.min(fb.height as i32 - 1 - cy) {
        for x in (-r).max(-cx)..=r.min(fb.width as i32 - 1 - cx) {
            let f = 1. - (x * x + y * y) as f32 * inverse;
            if f > 0. {
                fb.add_light(cx + x, cy + y, depth, color, power * f * f);
            }
        }
    }
}
fn beam(
    fb: &mut Framebuffer,
    u: &Uniforms,
    mut a: Vector3,
    mut b: Vector3,
    color: Vector3,
    width: f32,
) {
    let Some(da) = camera_depth(a, u) else {
        return;
    };
    let Some(db) = camera_depth(b, u) else {
        return;
    };
    if da < 1.1 && db < 1.1 {
        return;
    }
    if da < 1.1 {
        a = a + (b - a) * ((1.1 - da) / (db - da));
    }
    if db < 1.1 {
        b = b + (a - b) * ((1.1 - db) / (camera_depth(a, u).unwrap() - db));
    }
    let (Some((pa, za)), Some((pb, zb))) = (project(a, fb, u), project(b, fb, u)) else {
        return;
    };
    let length = (pb - pa).length();
    let steps = (length.ceil() as usize).clamp(1, 1600);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let depth = 1. / ((1. - t) / za + t / zb);
        let p = pa + (pb - pa) * t;
        glow(fb, p, depth, width * 2.5, color, 0.13);
        glow(
            fb,
            p,
            depth,
            width * 0.65,
            Vector3::new(0.7, 1., 0.75),
            0.85,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn uniforms() -> Uniforms {
        Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::new(0., 5., -25.),
            camera_target: Vector3::zero(),
            camera_up: Vector3::new(0., 1., 0.),
            light_position: None,
        }
    }
    #[test]
    fn sneeze_charges_bursts_then_returns_control_and_brake_cancels() {
        let mut alien = AlienFlight::new(5.);
        let u = uniforms();
        let world = World::new();
        alien.command(Command::Sneeze, Quaternion::identity());
        let mut fastest = 0_f32;
        for _ in 0..330 {
            if let Some(d) = alien.update(
                &InputState::default(),
                1. / 60.,
                Vector3::zero(),
                Quaternion::identity(),
                20.,
                &u,
                &world,
            ) {
                fastest = fastest.max(d.speed_factor);
                assert!(d.direction.x.is_finite());
            }
        }
        assert!(fastest > 24.);
        assert!(!alien.active());
        assert_eq!(alien.take_cues(), [Cue::Charge, Cue::Sneeze]);
        alien.command(Command::Chase, Quaternion::identity());
        alien.update(
            &InputState {
                braking: true,
                ..Default::default()
            },
            0.02,
            Vector3::zero(),
            Quaternion::identity(),
            20.,
            &u,
            &world,
        );
        assert!(!alien.active());
        assert!(alien.just_finished());
    }
    #[test]
    fn one_target_orbits_the_pilot_with_a_trail_independently_of_the_mouse() {
        let mut a = AlienFlight::new(5.);
        let mut b = AlienFlight::new(5.);
        let u = uniforms();
        let world = World::new();
        a.command(Command::Chase, Quaternion::identity());
        b.command(Command::Chase, Quaternion::identity());
        let pilot = Vector3::new(100., 7., 30.);
        let mut first = None;
        for frame in 0..120 {
            for (alien, mouse) in [
                (&mut a, Vector2::new(10., 20.)),
                (&mut b, Vector2::new(790., 590.)),
            ] {
                assert!(
                    alien
                        .update(
                            &InputState {
                                mouse_position: mouse,
                                ..Default::default()
                            },
                            1.0 / 60.0,
                            pilot,
                            Quaternion::identity(),
                            20.0,
                            &u,
                            &world
                        )
                        .is_none()
                );
                alien.orbit_target(pilot, Quaternion::identity(), 1.0 / 60.0);
                assert_eq!(alien.targets.len(), 1);
                assert_eq!(alien.hits, 0);
                assert!((alien.targets[0].position - pilot).length() < 6.0);
            }
            if frame == 0 {
                first = Some(a.targets[0].position);
            }
            assert_eq!(a.targets[0].position, b.targets[0].position);
        }
        assert!((a.targets[0].position - first.unwrap()).length() > 3.0);
        assert!(a.orbit_trail.len() > 10);
        assert!(a.orbit_trail.len() <= 40);
        assert_eq!(a.visual_rotation().y, 0.0);
    }

    #[test]
    fn shots_start_at_the_muzzle_and_only_score_when_aimed_at_a_target() {
        let mut alien = AlienFlight::new(5.);
        let mut u = uniforms();
        let world = World::new();
        u.camera_pos = Vector3::new(0., 0., -25.);
        u.camera_target = Vector3::new(0., 0., 100.);
        alien.command(Command::Chase, Quaternion::identity());
        alien.targets.push(Target {
            position: Vector3::new(0., 0., 100.),
            radius: 8.0,
        });
        let muzzle = Vector3::new(5., 0., 3.);
        let miss = InputState {
            firing: true,
            mouse_position: Vector2::new(30., 30.),
            ..Default::default()
        };
        alien.weapons(&miss, muzzle, 20., &u, &world);
        assert_eq!(alien.hits, 0);
        assert_eq!(alien.targets.len(), 1);
        assert_eq!(alien.bolts[0].origin, muzzle);
        assert_ne!(alien.bolts[0].origin, u.camera_pos);
        assert_eq!(alien.spin_velocity, 0.0);
        alien.shot_timer = 0.0;
        let hit = InputState {
            firing: true,
            mouse_position: Vector2::new(400., 300.),
            ..Default::default()
        };
        alien.weapons(&hit, muzzle, 20., &u, &world);
        assert_eq!(alien.hits, 1);
        assert!(alien.targets.is_empty());
        let spin = InputState {
            spin_pressed: true,
            ..Default::default()
        };
        alien.update(
            &spin,
            0.02,
            Vector3::zero(),
            Quaternion::identity(),
            20.,
            &u,
            &world,
        );
        assert!(alien.visual_rotation().y > 0.0);
    }

    #[test]
    fn planets_block_laser_hits_on_targets_behind_them() {
        use crate::{celestial_shaders::CelestialKind, objects::celestial::CelestialBody};
        let mut world = World::new();
        world.add(
            CelestialBody::new(CelestialKind::Rocky, Vector3::new(0., 0., 50.)).with_scale(12.0),
        );
        let mut alien = AlienFlight::new(5.);
        let mut u = uniforms();
        u.camera_pos = Vector3::new(0., 0., -25.);
        u.camera_target = Vector3::new(0., 0., 100.);
        alien.command(Command::Chase, Quaternion::identity());
        alien.targets.push(Target {
            position: Vector3::new(0., 0., 100.),
            radius: 8.0,
        });
        let input = InputState {
            firing: true,
            mouse_position: Vector2::new(400., 300.),
            ..Default::default()
        };
        alien.weapons(&input, Vector3::new(0., 0., 3.), 20., &u, &world);
        assert_eq!(alien.hits, 0);
        assert_eq!(alien.targets.len(), 1);
        assert!(alien.bolts[0].end.z < 39.0);
    }
}
