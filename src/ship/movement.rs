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
const LOCAL_SPEED_FACTOR: f32 = 0.55;

pub struct ShipMovement {
    local: Option<super::local::LocalFlight>,
    local_notice: f32,
    pub alien: Option<super::alien::AlienFlight>,
    model_center: Vector3,
    target: Vector3,
    camera: FollowCamera,
    effects: ShipEffects,
    impact: super::impact::ImpactEffects,
    radius: f32,
    muzzle_local: Option<Vector3>,
    recoil: Vector3,
    impact_cooldown: f32,
    clock: f32,
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
    pub fn configure_vehicle(&mut self, base_radius: f32, acceleration_duration: f32, alien: bool) {
        let multiplier = if alien { 3.0 } else { 1.0 };
        self.max_speed = base_radius * MAX_SPEED_FACTOR * multiplier;
        self.acceleration = self.max_speed / acceleration_duration.max(MIN_ACCELERATION_DURATION);
        self.deceleration = self.max_speed * SPACE_DRAG_DECEL_FACTOR;
        self.brake_deceleration = self.max_speed * BRAKE_DECEL_FACTOR;
        self.effects.set_alien(alien);
        self.alien = alien.then(|| super::alien::AlienFlight::new(base_radius));
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
            local: None,
            local_notice: 0.0,
            alien: None,
            model_center: target,
            target,
            camera: FollowCamera::new(radius),
            effects: ShipEffects::new([left_wing, right_wing], rear_point, radius),
            impact: super::impact::ImpactEffects::default(),
            radius,
            muzzle_local: None,
            recoil: Vector3::zero(),
            impact_cooldown: 0.0,
            clock: 0.0,
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

    pub fn command(&mut self, command: super::commands::Command) {
        if command == super::commands::Command::Sneeze {
            self.local = None;
        }
        if let Some(alien) = &mut self.alien {
            alien.command(command, self.orientation);
            if command == super::commands::Command::Chase {
                self.speed = self
                    .speed
                    .clamp(0.0, self.max_speed * BOOST_SPEED_MULTIPLIER);
            }
        }
    }

    pub fn set_muzzle(&mut self, point: Option<Vector3>) {
        self.muzzle_local = point;
    }

    pub fn update(
        &mut self,
        input: &InputState,
        frame_time: f32,
        uniforms: &mut Uniforms,
        world: &crate::scene::world::World,
    ) {
        self.update_local(input, frame_time, world);
        self.clock += frame_time;
        self.impact.update(frame_time);
        self.impact_cooldown = (self.impact_cooldown - frame_time).max(0.0);
        let drive = self.alien.as_mut().and_then(|alien| {
            alien.update(
                input,
                frame_time,
                self.target,
                self.orientation,
                self.max_speed,
                uniforms,
                world,
            )
        });
        if self.alien.as_ref().is_some_and(|a| a.just_finished()) {
            self.speed = self
                .speed
                .clamp(0., self.max_speed * BOOST_SPEED_MULTIPLIER);
        }
        if let Some(drive) = &drive {
            let desired =
                Quaternion::from_vector3_to_vector3(Vector3::new(0., 0., 1.), drive.direction);
            self.orientation = self
                .orientation
                .slerp(desired, 1. - (-drive.steering * frame_time).exp())
                .normalize();
            self.speed = move_towards(
                self.speed,
                self.max_speed * drive.speed_factor,
                self.max_speed * 32. * frame_time,
            );
            self.advance_with_collisions(frame_time, world);
        } else {
            // Bound integration error at low FPS; input edges, weapons and particles run once per frame.
            let steps = (frame_time / (1.0 / 120.0)).ceil().max(1.0) as usize;
            let dt = frame_time / steps as f32;
            for _ in 0..steps {
                let previous = self.target;
                self.update_navigation(input, dt);
                self.update_motion(input.thrust, input.boost, input.braking, dt);
                self.advance_with_collisions(dt, world);
                if let Some(local) = &self.local {
                    let from = (previous - local.body.position).normalize();
                    let to = (self.target - local.body.position).normalize();
                    self.orientation = (Quaternion::from_vector3_to_vector3(from, to)
                        * self.orientation)
                        .normalize();
                }
            }
        }
        self.update_ship(input, frame_time);
        self.camera
            .update(input, frame_time, self.target, self.orientation, uniforms);
        // Keep the follow camera on this side of a planet during close passes and climbs.
        if let Some(hit) = world.sweep_vehicle(
            self.target,
            uniforms.camera_pos - self.target,
            self.radius * 0.10,
        ) {
            uniforms.camera_pos = hit.safe_position;
        }
        uniforms.camera_pos +=
            Vector3::new((self.clock * 79.0).sin(), (self.clock * 57.0).cos(), 0.0)
                * self.radius
                * 0.045
                * self.impact.shake;
        self.apply_model(uniforms);
        if frame_time > 0.0
            && let Some(alien) = &mut self.alien
        {
            let local = self.muzzle_local.unwrap_or(
                self.model_center
                    + Vector3::new(self.radius * 0.44, -self.radius * 0.03, -self.radius * 0.24),
            );
            let m = uniforms.model_matrix;
            let muzzle = Vector3::new(
                m.m0 * local.x + m.m4 * local.y + m.m8 * local.z + m.m12,
                m.m1 * local.x + m.m5 * local.y + m.m9 * local.z + m.m13,
                m.m2 * local.x + m.m6 * local.y + m.m10 * local.z + m.m14,
            );
            let local_pilot =
                self.model_center + Vector3::new(0.0, self.radius * 0.14, -self.radius * 0.03);
            let pilot = Vector3::new(
                m.m0 * local_pilot.x + m.m4 * local_pilot.y + m.m8 * local_pilot.z + m.m12,
                m.m1 * local_pilot.x + m.m5 * local_pilot.y + m.m9 * local_pilot.z + m.m13,
                m.m2 * local_pilot.x + m.m6 * local_pilot.y + m.m10 * local_pilot.z + m.m14,
            );
            alien.orbit_target(pilot, self.orientation, frame_time);
            alien.weapons(input, muzzle, self.max_speed, uniforms, world);
        }

        let trail_emitting = (input.boost
            || drive.is_some()
            || (self.speed > self.max_speed * 0.5 && self.yaw_rate.abs() > 0.35))
            && self.speed > MOVEMENT_THRESHOLD;
        self.effects.update(
            trail_emitting,
            (input.thrust || drive.is_some()) && !input.braking,
            frame_time,
            self.world_velocity(),
            self.speed_ratio(),
            self.camera.effective_distance(),
            self.camera.reference_distance(),
            uniforms,
        );
    }

    pub fn is_moving(&self) -> bool {
        self.speed > MOVEMENT_THRESHOLD || self.recoil.length_sqr() > 0.01
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
        if let Some(alien) = &self.alien {
            alien.draw(framebuffer, uniforms, self.target);
        }
        self.impact.draw(framebuffer, uniforms);
    }

    pub fn take_impact(&mut self) -> bool {
        self.impact.take_event()
    }

    fn update_local(&mut self, input: &InputState, dt: f32, world: &crate::scene::world::World) {
        self.local_notice = (self.local_notice - dt).max(0.0);
        if dt <= 0.0 {
            return;
        }
        if input.local_pressed && self.local.take().is_none() {
            if let Some(body) = world.navigation_target(None, self.target)
                && (self.target - body.position).length() <= body.radius * 6.0 + self.radius * 8.0
            {
                let local = super::local::LocalFlight { body };
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
    }

    pub fn draw_navigation(&self, d: &mut RaylibDrawHandle) {
        let y = if self.alien.is_some() { 120 } else { 52 };
        let label = if let Some(local) = &self.local {
            let altitude =
                ((self.target - local.body.position).length() - local.body.radius).max(0.0);
            format!(
                "LOCAL: {} / ALT {:.0} / L: salir",
                local.body.name, altitude
            )
        } else if self.local_notice > 0.0 {
            "Acercate a un planeta para activar el vuelo local".to_owned()
        } else {
            "L: vuelo local cerca de un planeta".to_owned()
        };
        let width = d.measure_text(&label, 13) + 24;
        d.draw_rectangle(16, y, width, 27, Color::new(7, 18, 38, 205));
        d.draw_text(&label, 28, y + 7, 13, Color::new(139, 209, 255, 255));
    }

    fn advance_with_collisions(&mut self, dt: f32, world: &crate::scene::world::World) {
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

    fn world_velocity(&self) -> Vector3 {
        transform_direction(&self.orientation.to_matrix(), Vector3::new(0.0, 0.0, 1.0)) * self.speed
    }

    fn update_navigation(&mut self, input: &InputState, frame_time: f32) {
        let target_pitch = axis_value(input.pitch_down, input.pitch_up);
        let target_yaw = axis_value(input.turn_right, input.turn_left);
        let roll_turn_assist = axis_value(input.bank_right, input.bank_left) * 0.45;
        let blend = 1.0 - (-STEER_ANGULAR_ACCEL * frame_time).exp();
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
        let smooth = 1.0 - (-SHIP_TILT_SMOOTHING * frame_time).exp();
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
        if let Some(local) = &self.local {
            let limit = (self.max_speed * LOCAL_SPEED_FACTOR).min(local.body.radius * 0.65);
            let target = if (thrust || boost) && !braking {
                limit
            } else {
                0.0
            };
            let acceleration = if braking || self.speed > limit {
                self.brake_deceleration * 0.5
            } else {
                self.acceleration * LOCAL_SPEED_FACTOR
            };
            self.speed = move_towards(self.speed, target, acceleration * frame_time);
            return;
        }
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
        let special = self.alien.as_ref().map_or(Matrix::identity(), |a| {
            let r = a.visual_rotation();
            let s = a.visual_scale();
            Matrix::scale(s.x, s.y, s.z)
                * Matrix::rotate_y(r.y)
                * Matrix::rotate_x(r.x)
                * Matrix::rotate_z(r.z)
        });
        uniforms.model_matrix = Matrix::translate(
            -self.model_center.x,
            -self.model_center.y,
            -self.model_center.z,
        ) * special
            * local_attitude
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::vehicle::Vehicle;

    #[test]
    fn a_local_dive_still_bounces_and_the_camera_stays_outside() {
        use crate::{
            celestial_shaders::CelestialKind, objects::celestial::CelestialBody,
            scene::world::World,
        };
        let car = Vehicle::car();
        let mut world = World::new();
        world.add(
            CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
                .named("Prueba")
                .with_scale(100.),
        );
        let mut ship = ShipMovement::new(&car.mesh, 2.);
        ship.target = Vector3::new(100. + ship.radius * 3., 0., 0.);
        let body = world.navigation_target(None, ship.target).unwrap();
        ship.local = Some(super::super::local::LocalFlight { body });
        ship.orientation = Quaternion::from_vector3_to_vector3(
            Vector3::new(0., 0., 1.),
            Vector3::new(-1., 0., 0.),
        );
        ship.speed = 10_000.;
        let mut u = car.preview_uniforms(0.);
        ship.update(
            &InputState {
                thrust: true,
                ..Default::default()
            },
            0.1,
            &mut u,
            &world,
        );
        assert!(ship.target.length() > 100. + ship.radius);
        assert!(u.camera_pos.length() > 100.);
        assert!(ship.take_impact());
    }

    #[test]
    fn local_flight_circles_without_altitude_drift_and_exits_without_a_jump() {
        use crate::{
            celestial_shaders::CelestialKind, objects::celestial::CelestialBody,
            scene::world::World,
        };
        let car = Vehicle::car();
        let mut world = World::new();
        world.add(
            CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
                .named("Prueba")
                .with_scale(100.),
        );
        let mut ship = ShipMovement::new(&car.mesh, 2.);
        let distance = 100. + ship.radius * 5.;
        ship.target = Vector3::new(distance, 0., 0.);
        let mut u = car.preview_uniforms(0.);
        ship.update(
            &InputState {
                local_pressed: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        assert!(ship.local.is_some());
        let start = ship.target;
        let mut length = 0.;
        for _ in 0..12000 {
            let before = ship.target;
            ship.update(
                &InputState {
                    thrust: true,
                    boost: true,
                    ..Default::default()
                },
                1. / 60.,
                &mut u,
                &world,
            );
            length += (ship.target - before).length();
            assert!(
                (ship.target.length() - distance).abs() < 0.15,
                "altitud {}",
                ship.target.length()
            );
            assert!(!ship.take_impact());
            if length > std::f32::consts::TAU * distance {
                break;
            }
        }
        assert!(
            length > std::f32::consts::TAU * distance,
            "Debe completar una vuelta"
        );
        assert!((ship.target - start).length() < distance * 0.08);
        let before = ship.target;
        let speed = ship.speed;
        ship.update(
            &InputState {
                local_pressed: true,
                thrust: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        assert!(ship.local.is_none());
        assert!((ship.target - before).length() < speed * 0.04);
        assert!(ship.speed >= speed);
    }

    #[test]
    fn local_reference_follows_the_orbit_and_ignores_a_paused_toggle() {
        use crate::{
            celestial_shaders::CelestialKind,
            objects::celestial::CelestialBody,
            scene::world::{Orbit, World, WorldUpdate},
        };
        let car = Vehicle::car();
        let mut world = World::new();
        let center = Vector3::new(500., 0., 0.);
        world.add(
            CelestialBody::new(CelestialKind::Rocky, center)
                .named("Prueba")
                .with_scale(100.)
                .with_orbit(Orbit::new(
                    Vector3::zero(),
                    center,
                    Vector3::new(0., 1., 0.),
                    0.1,
                )),
        );
        let mut ship = ShipMovement::new(&car.mesh, 2.);
        ship.target = center + Vector3::new(0., 150., 0.);
        let mut u = car.preview_uniforms(0.);
        ship.update(
            &InputState {
                local_pressed: true,
                ..Default::default()
            },
            0.,
            &mut u,
            &world,
        );
        assert!(ship.local.is_none());
        ship.update(
            &InputState {
                local_pressed: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        let relative = ship.target - center;
        for _ in 0..120 {
            world.update(&WorldUpdate {
                frame_time: 1. / 60.,
            });
            ship.update(&InputState::default(), 1. / 60., &mut u, &world);
            let target = world
                .navigation_target(Some("Prueba"), ship.target)
                .unwrap();
            assert!((ship.target - target.position - relative).length() < 0.01);
        }
    }

    #[test]
    fn flight_response_matches_at_30_60_and_144_fps() {
        let car = Vehicle::car();
        let world = crate::scene::world::World::new();
        let simulate = |fps: usize| {
            let mut ship = ShipMovement::new(&car.mesh, 2.);
            let mut u = car.preview_uniforms(0.);
            for stage in 0..3 {
                for _ in 0..fps * 2 {
                    ship.update(
                        &InputState {
                            thrust: stage < 2,
                            boost: stage < 2,
                            braking: stage == 2,
                            turn_right: stage == 1,
                            pitch_up: stage == 1,
                            ..Default::default()
                        },
                        1. / fps as f32,
                        &mut u,
                        &world,
                    );
                }
            }
            (ship.target, ship.orientation, ship.speed, u.camera_pos)
        };
        let baseline = simulate(60);
        for fps in [30, 144] {
            let result = simulate(fps);
            assert!(
                (result.0 - baseline.0).length() < car.radius * 0.08,
                "FPS {fps}: {:?} / {:?}",
                result.0,
                baseline.0
            );
            let a = transform_direction(&result.1.to_matrix(), Vector3::new(0., 0., 1.));
            let b = transform_direction(&baseline.1.to_matrix(), Vector3::new(0., 0., 1.));
            assert!(a.dot(b) > 0.9999);
            assert!((result.2 - baseline.2).abs() < 0.001);
            assert!((result.3 - baseline.3).length() < car.radius * 0.15);
        }
    }

    #[test]
    fn thrust_and_spin_are_independent_without_steering_the_camera() {
        let car = Vehicle::car();
        let saucer = Vehicle::saucer(car.radius);
        let mut ship = ShipMovement::new(&saucer.mesh, 2.0);
        ship.configure_vehicle(car.radius, 2.0, true);
        let world = crate::scene::world::World::new();
        let mut u = saucer.preview_uniforms(0.0);
        for _ in 0..60 {
            ship.update(
                &InputState {
                    thrust: true,
                    boost: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                &mut u,
                &world,
            );
        }
        assert!(ship.speed > 0.0);
        assert_eq!(ship.alien.as_ref().unwrap().visual_rotation().y, 0.0);
        let before = ship.speed;
        for _ in 0..30 {
            ship.update(
                &InputState {
                    spin_held: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                &mut u,
                &world,
            );
        }
        assert!(ship.speed < before);
        assert!(ship.alien.as_ref().unwrap().visual_rotation().y > 0.1);
        assert_eq!(ship.orientation, Quaternion::identity());
    }

    #[test]
    fn both_vehicles_bounce_instead_of_tunneling_through_a_planet() {
        use crate::{celestial_shaders::CelestialKind, objects::celestial::CelestialBody};
        let car = Vehicle::car();
        let saucer = Vehicle::saucer(car.radius);
        for vehicle in [&car, &saucer] {
            let mut ship = ShipMovement::new(&vehicle.mesh, 2.0);
            ship.configure_vehicle(
                car.radius,
                2.0,
                vehicle.kind == super::super::vehicle::VehicleKind::Saucer,
            );
            ship.target = Vector3::zero();
            ship.speed = 10_000.0;
            let center = Vector3::new(0., 0., 100.);
            let mut world = crate::scene::world::World::new();
            world.add(CelestialBody::new(CelestialKind::Rocky, center).with_scale(20.0));
            ship.advance_with_collisions(0.1, &world);
            assert!(ship.target.z < 100.0 - 20.0 - ship.radius);
            assert!(ship.recoil.z < 0.0);
            assert_eq!(ship.speed, 0.0);
            assert!(ship.take_impact());
            assert!(!ship.take_impact());
            for _ in 0..120 {
                ship.advance_with_collisions(1.0 / 60.0, &world);
                assert!((ship.target - center).length() > 20.0 + ship.radius);
            }
            ship.target = center + Vector3::new(1., 0., 0.);
            ship.recoil = Vector3::zero();
            ship.advance_with_collisions(1.0 / 60.0, &world);
            assert!((ship.target - center).length() > 20.0 + ship.radius);
        }
    }

    #[test]
    fn aiming_does_not_move_the_vehicle_or_change_its_heading() {
        let car = Vehicle::car();
        let saucer = Vehicle::saucer(car.radius);
        let mut ship = ShipMovement::new(&saucer.mesh, 2.0);
        ship.configure_vehicle(car.radius, 2.0, true);
        let world = crate::scene::world::World::new();
        let mut u = saucer.preview_uniforms(0.0);
        let position = ship.target;
        let heading = ship.orientation;
        ship.command(super::super::commands::Command::Chase);
        for i in 0..120 {
            ship.update(
                &InputState {
                    mouse_position: Vector2::new((i * 5) as f32, 250.0),
                    ..Default::default()
                },
                1.0 / 60.0,
                &mut u,
                &world,
            );
        }
        assert_eq!(ship.target, position);
        assert_eq!(ship.orientation, heading);
    }

    #[test]
    fn sneeze_covers_a_large_distance_and_returns_to_normal_speed() {
        let car = Vehicle::car();
        let vehicle = Vehicle::saucer(car.radius);
        let mut ship = ShipMovement::new(&vehicle.mesh, 2.0);
        ship.configure_vehicle(car.radius, 2.0, true);
        let world = crate::scene::world::World::new();
        let mut u = vehicle.preview_uniforms(0.0);
        ship.update(&InputState::default(), 0.0, &mut u, &world);
        ship.command(super::super::commands::Command::Sneeze);
        let mut distance = 0.0;
        let mut previous = ship.target;
        for _ in 0..330 {
            ship.update(&InputState::default(), 1.0 / 60.0, &mut u, &world);
            distance += (ship.target - previous).length();
            previous = ship.target;
        }
        assert!(
            distance > car.radius * 350.0,
            "distancia {distance}, radio {}",
            car.radius
        );
        assert!(ship.speed <= ship.max_speed * BOOST_SPEED_MULTIPLIER);
        assert!(!ship.alien.as_ref().unwrap().active());
    }

    #[test]
    fn saucer_acceleration_and_top_speed_are_three_times_the_car() {
        let car = Vehicle::car();
        let saucer = Vehicle::saucer(car.radius);
        let mut normal = ShipMovement::new(&car.mesh, 2.0);
        let mut alien = ShipMovement::new(&saucer.mesh, 2.0);
        normal.configure_vehicle(car.radius, 2.0, false);
        alien.configure_vehicle(car.radius, 2.0, true);
        for _ in 0..300 {
            normal.update_motion(true, true, false, 1.0 / 60.0);
            alien.update_motion(true, true, false, 1.0 / 60.0);
            assert!((alien.speed - normal.speed * 3.0).abs() < 0.001);
        }
        for _ in 0..300 {
            normal.update_motion(false, false, true, 1.0 / 60.0);
            alien.update_motion(false, false, true, 1.0 / 60.0);
        }
        assert_eq!(normal.speed, 0.0);
        assert_eq!(alien.speed, 0.0);
    }

    #[test]
    fn every_planet_has_a_safe_jump_after_orbiting() {
        let mut world = crate::scene::build_solar_system();
        world.update(&crate::scene::world::WorldUpdate { frame_time: 51.0 });
        let car = Vehicle::car();
        let saucer = Vehicle::saucer(car.radius);
        let mut ship = ShipMovement::new(&saucer.mesh, 2.0);
        ship.configure_vehicle(car.radius, 2.0, true);
        for target in world.travel_targets() {
            ship.speed = 100.0;
            assert!(ship.teleport(&target, &world), "{}", target.name);
            assert_eq!(ship.speed, 0.0);
            let offset = target.position - ship.target;
            assert!(offset.length() > target.radius * 3.0);
            let forward =
                transform_direction(&ship.orientation.to_matrix(), Vector3::new(0., 0., 1.));
            assert!(forward.dot(offset.normalize()) > 0.999);
            let mut hits = Vec::new();
            world.query_sphere(
                ship.target,
                ship.camera.reference_distance() * 1.5,
                &mut hits,
            );
            assert!(hits.is_empty(), "{}", target.name);
        }
    }
}
