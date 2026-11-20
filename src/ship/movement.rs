use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::load_objects::Mesh;
use crate::rendering::uniforms::Uniforms;
use crate::ship::camera::FollowCamera;
use crate::ship::effects::ShipEffects;
use crate::ship::input::InputState;
use raylib::prelude::*;

mod collision;
mod local_flight;
mod navigation;
mod presentation;
mod travel;

#[cfg(test)]
mod local_preview;
#[cfg(test)]
mod local_tests;
#[cfg(test)]
mod tests;

const MAX_SPEED_FACTOR: f32 = 2.8;
const BOOST_SPEED_MULTIPLIER: f32 = 2.2;
const MIN_ACCELERATION_DURATION: f32 = 0.1;
const SPACE_DRAG_DECEL_FACTOR: f32 = 0.45;
const BRAKE_DECEL_FACTOR: f32 = 2.4;
const MOVEMENT_THRESHOLD: f32 = 0.01;

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
    unbounded_thrust: bool,
}

impl ShipMovement {
    pub fn position(&self) -> Vector3 {
        self.target
    }

    pub fn configure_vehicle(&mut self, base_radius: f32, acceleration_duration: f32, alien: bool) {
        let multiplier = if alien { 3.0 } else { 1.0 };
        self.max_speed = base_radius * MAX_SPEED_FACTOR * multiplier;
        self.acceleration = self.max_speed / acceleration_duration.max(MIN_ACCELERATION_DURATION);
        self.deceleration = self.max_speed * SPACE_DRAG_DECEL_FACTOR;
        self.brake_deceleration = self.max_speed * BRAKE_DECEL_FACTOR;
        self.unbounded_thrust = false;
        self.effects.set_alien(alien);
        self.alien = alien.then(|| super::alien::AlienFlight::new(base_radius));
    }

    pub fn configure_unbounded_car_thrust(&mut self) {
        self.unbounded_thrust = true;
        self.max_speed *= 2.0;
        self.acceleration *= 2.0;
        self.deceleration *= 2.0;
        self.brake_deceleration *= 2.0;
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
            unbounded_thrust: false,
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
                    let transport = Quaternion::from_vector3_to_vector3(from, to);
                    self.orientation = (transport * self.orientation).normalize();
                    self.camera.transport(transport);
                }
            }
        }
        self.update_ship(input, frame_time);
        let camera_orientation = self.local.as_ref().map_or(self.orientation, |local| {
            local.tangent_orientation(self.target, self.orientation)
        });
        self.camera.update(
            input,
            frame_time,
            self.target,
            camera_orientation,
            self.local.is_some(),
            uniforms,
        );
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
        if self.unbounded_thrust && self.local.is_none() {
            return true;
        }
        let limit = self
            .local
            .as_ref()
            .map_or(self.max_speed * BOOST_SPEED_MULTIPLIER, |local| {
                local.speed_limit(self.target, self.radius)
            });
        self.speed < limit - MOVEMENT_THRESHOLD
    }

    pub fn speed_ratio(&self) -> f32 {
        let reference = self.local.as_ref().map_or(self.max_speed, |local| {
            local.speed_limit(self.target, self.radius)
        });
        (self.speed / reference.max(0.001)).clamp(0.0, BOOST_SPEED_MULTIPLIER)
    }

    pub fn take_impact(&mut self) -> bool {
        self.impact.take_event()
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
