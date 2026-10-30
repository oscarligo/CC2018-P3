use crate::framebuffer::Framebuffer;
use crate::input::InputState;
use crate::load_objects::{project_vertex, Mesh};
use crate::uniforms::Uniforms;
use raylib::prelude::*;

// Camera constants
const INITIAL_CAMERA_PITCH: f32 = 0.2;
const INITIAL_CAMERA_DISTANCE_FACTOR: f32 = 2.5;
const MIN_CAMERA_DISTANCE_FACTOR: f32 = 1.0;
const MAX_CAMERA_DISTANCE_FACTOR: f32 = 8.0;
const ORBIT_SENSITIVITY: f32 = 0.01;
const MIN_CAMERA_PITCH: f32 = -1.5;
const MAX_CAMERA_PITCH: f32 = 1.5;
const ZOOM_SENSITIVITY: f32 = 0.1;

// Ship movement constants
const MAX_SHIP_TILT: f32 = 20.0_f32.to_radians();
const SHIP_TILT_SPEED: f32 = 2.5;
const MAX_SPEED_FACTOR: f32 = 2.5;
const MIN_ACCELERATION_DURATION: f32 = 0.1;
const DECELERATION_FACTOR: f32 = 0.25;
const BRAKE_DECELERATION_FACTOR: f32 = 2.0;
const MOVEMENT_THRESHOLD: f32 = 0.01;

// Trail constants
const TRAIL_CAPACITY: usize = 64;
const TRAIL_SPEED_FACTOR: f32 = 2.0;
const TRAIL_PARTICLE_LIFETIME: f32 = 0.3;
const TRAIL_EMISSION_INTERVAL: f32 = 1.0 / 30.0;
const TRAIL_PROJECTION_SCALE: f32 = 90.0;

// Trail visual
const TRAIL_COLOR: [f32; 3] = [0.8, 0.8, 0.8];
const TRAIL_POINT_RADIUS: i32 = 0;
const TRAIL_MIN_POINT_RADIUS: i32 = 0;
const TRAIL_MAX_POINT_RADIUS: i32 = 1;
const LOCAL_BACKWARD: [f32; 3] = [0.0, 0.0, -1.0];

// ROCKET EXHAUST CONSTANTS
const EXHAUST_CAPACITY: usize = 1200;

// Lifetimes
const CORE_LIFETIME: f32 = 0.18;
const FLAME_LIFETIME: f32 = 0.44;
const SPARK_LIFETIME: f32 = 0.28;
const SMOKE_LIFETIME: f32 = 1.35;

// Base emission intervals
const CORE_EMISSION_INTERVAL: f32 = 1.0 / 95.0;
const FLAME_EMISSION_INTERVAL: f32 = 1.0 / 85.0;
const SPARK_EMISSION_INTERVAL: f32 = 1.0 / 65.0;
const SMOKE_EMISSION_INTERVAL: f32 = 1.0 / 16.0;
const EXHAUST_CLOSE_DENSITY_BOOST: f32 = 1.25;
const EXHAUST_CLOSE_MAX_DENSITY: f32 = 12.0;

// Speed factors
const CORE_SPEED_FACTOR: f32 = 2.6;
const FLAME_SPEED_FACTOR: f32 = 1.7;
const SPARK_SPEED_FACTOR: f32 = 3.0;
const SMOKE_SPEED_FACTOR: f32 = 0.45;
const SMOKE_DRAG: f32 = 1.6;
const SMOKE_RISE_FACTOR: f32 = 0.22;

// Conical spread
const CORE_SPREAD: f32 = 0.05;
const FLAME_SPREAD: f32 = 0.22;
const SPARK_SPREAD: f32 = 0.52;
const SMOKE_SPREAD: f32 = 0.36;

// Thermal color palette
const COLOR_CORE_HOT: [f32; 3] = [0.88, 0.96, 1.0];
const COLOR_CORE_WARM: [f32; 3] = [1.0, 0.92, 0.45];
const COLOR_FLAME_HOT: [f32; 3] = [1.0, 0.82, 0.2];
const COLOR_FLAME_MID: [f32; 3] = [1.0, 0.36, 0.03];
const COLOR_FLAME_COOL: [f32; 3] = [0.42, 0.06, 0.01];
const COLOR_SPARK: [f32; 3] = [1.0, 0.95, 0.65];
const COLOR_SMOKE_START: [f32; 3] = [0.26, 0.24, 0.24];
const COLOR_SMOKE_END: [f32; 3] = [0.65, 0.68, 0.72];

// Exhaust visual
const EXHAUST_BASE_RADIUS: f32 = 4.8;
const EXHAUST_MIN_RADIUS: i32 = 0;
const EXHAUST_FAR_MAX_RADIUS: i32 = 1;
const EXHAUST_CLOSE_MAX_RADIUS: i32 = 20;
const EXHAUST_CLOSE_GROWTH: f32 = 3.0;

pub struct ShipMovement {
    model_center: Vector3,
    target: Vector3,
    camera_yaw: f32,
    camera_pitch: f32,
    camera_distance: f32,
    min_distance: f32,
    max_distance: f32,
    ship_pitch: f32,
    ship_yaw: f32,
    ship_roll: f32,
    wing_tips: [Vector3; 2],
    trail: Vec<TrailParticle>,
    trail_speed: f32,
    emission_timer: f32,
    rear_point: Vector3,
    exhaust: Vec<ExhaustParticle>,
    core_timer: f32,
    flame_timer: f32,
    spark_timer: f32,
    smoke_timer: f32,
    rng_state: u32,
    speed: f32,
    max_speed: f32,
    acceleration: f32,
    deceleration: f32,
    brake_deceleration: f32,
}

struct TrailParticle {
    position: Vector3,
    velocity: Vector3,
    lifetime: f32,
    wing: usize,
}

#[derive(Clone, Copy)]
enum ExhaustKind {
    Core,
    Flame,
    Spark,
    Smoke,
}

struct ExhaustParticle {
    position: Vector3,
    velocity: Vector3,
    lifetime: f32,
    max_lifetime: f32,
    kind: ExhaustKind,
}

impl ShipMovement {
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

        Self {
            model_center: target,
            target,
            camera_yaw: 0.0,
            camera_pitch: INITIAL_CAMERA_PITCH,
            camera_distance: radius * INITIAL_CAMERA_DISTANCE_FACTOR,
            min_distance: radius * MIN_CAMERA_DISTANCE_FACTOR,
            max_distance: radius * MAX_CAMERA_DISTANCE_FACTOR,
            ship_pitch: 0.0,
            ship_yaw: 0.0,
            ship_roll: 0.0,
            wing_tips: [left_wing, right_wing],
            trail: Vec::with_capacity(TRAIL_CAPACITY),
            trail_speed: radius * TRAIL_SPEED_FACTOR,
            emission_timer: 0.0,
            rear_point,
            exhaust: Vec::with_capacity(EXHAUST_CAPACITY),
            core_timer: 0.0,
            flame_timer: 0.0,
            spark_timer: 0.0,
            smoke_timer: 0.0,
            rng_state: 0x8542_5987,
            speed: 0.0,
            max_speed: radius * MAX_SPEED_FACTOR,
            acceleration: radius * MAX_SPEED_FACTOR
                / acceleration_duration.max(MIN_ACCELERATION_DURATION),
            deceleration: radius * DECELERATION_FACTOR,
            brake_deceleration: radius * BRAKE_DECELERATION_FACTOR,
        }
    }

    fn next_rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32) / (u32::MAX as f32)
    }

    fn next_rand_bipolar(&mut self) -> f32 {
        self.next_rand_f32() * 2.0 - 1.0
    }

    pub fn update(&mut self, input: &InputState, frame_time: f32, uniforms: &mut Uniforms) {
        self.update_camera(input);
        self.update_ship(input, frame_time);
        self.update_motion(input.thrust, input.braking, frame_time);
        self.apply(uniforms);
        self.update_trail(self.speed > MOVEMENT_THRESHOLD, frame_time, uniforms);
        self.update_exhaust(input.thrust && !input.braking, frame_time, uniforms);
    }

    pub fn is_moving(&self) -> bool {
        self.speed > MOVEMENT_THRESHOLD
    }

    pub fn can_accelerate(&self) -> bool {
        self.speed < self.max_speed - MOVEMENT_THRESHOLD
    }

    pub fn speed_ratio(&self) -> f32 {
        (self.speed / self.max_speed).clamp(0.0, 1.0)
    }

    fn update_camera(&mut self, input: &InputState) {
        self.camera_yaw -= input.orbit_delta.x * ORBIT_SENSITIVITY;
        self.camera_pitch = (self.camera_pitch + input.orbit_delta.y * ORBIT_SENSITIVITY)
            .clamp(MIN_CAMERA_PITCH, MAX_CAMERA_PITCH);
        if input.zoom_delta != 0.0 {
            self.camera_distance = (self.camera_distance
                * (1.0 - input.zoom_delta * ZOOM_SENSITIVITY))
                .clamp(self.min_distance, self.max_distance);
        }
    }

    fn update_ship(&mut self, input: &InputState, frame_time: f32) {
        let step = SHIP_TILT_SPEED * frame_time;

        self.ship_pitch = move_towards(
            self.ship_pitch,
            axis_target(input.pitch_up, input.pitch_down, MAX_SHIP_TILT),
            step,
        );
        self.ship_yaw = move_towards(
            self.ship_yaw,
            axis_target(input.turn_left, input.turn_right, MAX_SHIP_TILT),
            step,
        );
        self.ship_roll = move_towards(
            self.ship_roll,
            axis_target(input.bank_left, input.bank_right, MAX_SHIP_TILT),
            step,
        );
    }

    fn update_motion(&mut self, thrust: bool, braking: bool, frame_time: f32) {
        if braking {
            self.speed = (self.speed - self.brake_deceleration * frame_time).max(0.0);
        } else if thrust {
            self.speed = (self.speed + self.acceleration * frame_time).min(self.max_speed);
        } else {
            self.speed = (self.speed - self.deceleration * frame_time).max(0.0);
        }
    }

    fn apply(&self, uniforms: &mut Uniforms) {
        uniforms.model_matrix = Matrix::translate(self.target.x, self.target.y, self.target.z)
            * Matrix::rotate_xyz(Vector3::new(self.ship_pitch, self.ship_yaw, self.ship_roll))
            * Matrix::translate(
                -self.model_center.x,
                -self.model_center.y,
                -self.model_center.z,
            );

        let horizontal_distance = self.camera_distance * self.camera_pitch.cos();
        uniforms.camera_pos = self.target
            + Vector3::new(
                horizontal_distance * self.camera_yaw.sin(),
                self.camera_distance * self.camera_pitch.sin(),
                -horizontal_distance * self.camera_yaw.cos(),
            );
        uniforms.camera_target = self.target;
    }

    fn update_trail(&mut self, emitting: bool, frame_time: f32, uniforms: &Uniforms) {
        for particle in &mut self.trail {
            particle.position += particle.velocity * frame_time;
            particle.lifetime -= frame_time;
        }
        self.trail.retain(|particle| particle.lifetime > 0.0);

        if !emitting {
            self.emission_timer = 0.0;
            return;
        }

        self.emission_timer += frame_time;
        while self.emission_timer >= TRAIL_EMISSION_INTERVAL {
            self.emission_timer -= TRAIL_EMISSION_INTERVAL;
            let backward = transform_direction(
                &uniforms.model_matrix,
                Vector3::new(LOCAL_BACKWARD[0], LOCAL_BACKWARD[1], LOCAL_BACKWARD[2]),
            );
            for (wing, wing_tip) in self.wing_tips.into_iter().enumerate() {
                self.trail.push(TrailParticle {
                    position: transform_point(&uniforms.model_matrix, wing_tip),
                    velocity: backward * (self.trail_speed + self.speed),
                    lifetime: TRAIL_PARTICLE_LIFETIME,
                    wing,
                });
            }
        }
    }

    pub fn draw_trail(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        let point_radius = scaled_trail_radius(
            self.camera_distance,
            self.min_distance * INITIAL_CAMERA_DISTANCE_FACTOR,
        );
        let mut previous = [None, None];
        for particle in &self.trail {
            let Some((position, depth)) = project_vertex(
                particle.position,
                uniforms.camera_pos,
                uniforms.camera_target,
                TRAIL_PROJECTION_SCALE,
                framebuffer.width as f32,
                framebuffer.height as f32,
            ) else {
                continue;
            };
            let intensity = (particle.lifetime / TRAIL_PARTICLE_LIFETIME).clamp(0.0, 1.0);
            let color = Vector3::new(TRAIL_COLOR[0], TRAIL_COLOR[1], TRAIL_COLOR[2]) * intensity;
            if let Some((start, start_depth)) = previous[particle.wing] {
                draw_thick_line(
                    framebuffer,
                    start,
                    position,
                    start_depth,
                    depth,
                    point_radius,
                    color,
                );
            } else {
                draw_square(framebuffer, position, depth, point_radius, color);
            }
            previous[particle.wing] = Some((position, depth));
        }
    }

    fn update_exhaust(&mut self, emitting: bool, frame_time: f32, uniforms: &Uniforms) {
        for particle in &mut self.exhaust {
            if matches!(particle.kind, ExhaustKind::Smoke) {
                particle.velocity *= (1.0 - SMOKE_DRAG * frame_time).max(0.0);
            }
            particle.position += particle.velocity * frame_time;
            particle.lifetime -= frame_time;
        }
        self.exhaust.retain(|particle| particle.lifetime > 0.0);

        if !emitting {
            self.core_timer = 0.0;
            self.flame_timer = 0.0;
            self.spark_timer = 0.0;
            self.smoke_timer = 0.0;
            return;
        }

        self.core_timer = (self.core_timer + frame_time).min(0.2);
        self.flame_timer = (self.flame_timer + frame_time).min(0.2);
        self.spark_timer = (self.spark_timer + frame_time).min(0.2);
        self.smoke_timer = (self.smoke_timer + frame_time).min(0.2);

        let origin = transform_point(&uniforms.model_matrix, self.rear_point);
        let backward = transform_direction(
            &uniforms.model_matrix,
            Vector3::new(LOCAL_BACKWARD[0], LOCAL_BACKWARD[1], LOCAL_BACKWARD[2]),
        );
        let local_right = transform_direction(&uniforms.model_matrix, Vector3::new(1.0, 0.0, 0.0));
        let local_up = transform_direction(&uniforms.model_matrix, Vector3::new(0.0, 1.0, 0.0));

        let ship_radius = self.trail_speed / TRAIL_SPEED_FACTOR;
        let nozzle_radius = ship_radius * 0.085;

        let reference_distance = self.min_distance * INITIAL_CAMERA_DISTANCE_FACTOR;
        let zoom_ratio = reference_distance / self.camera_distance.max(0.001);
        let emission_density = if zoom_ratio > 1.0 {
            (zoom_ratio * zoom_ratio * EXHAUST_CLOSE_DENSITY_BOOST).min(EXHAUST_CLOSE_MAX_DENSITY)
        } else {
            zoom_ratio.clamp(0.8, 1.0)
        };

        let core_interval = CORE_EMISSION_INTERVAL / emission_density;
        let flame_interval = FLAME_EMISSION_INTERVAL / emission_density;
        let spark_interval = SPARK_EMISSION_INTERVAL / emission_density;
        let smoke_interval = SMOKE_EMISSION_INTERVAL / emission_density.min(2.5);

        while self.core_timer >= core_interval {
            self.core_timer -= core_interval;
            let jitter_x = self.next_rand_bipolar() * CORE_SPREAD;
            let jitter_y = self.next_rand_bipolar() * CORE_SPREAD;
            let speed = self.speed
                + self.trail_speed * CORE_SPEED_FACTOR * (0.9 + self.next_rand_f32() * 0.2);
            let velocity = backward * speed
                + local_right * (jitter_x * self.trail_speed)
                + local_up * (jitter_y * self.trail_speed);
            let sub_step = self.next_rand_f32() * frame_time * 0.5;
            let offset_x = self.next_rand_bipolar() * nozzle_radius * 0.55;
            let offset_y = self.next_rand_bipolar() * nozzle_radius * 0.55;
            let spawn_pos =
                origin + local_right * offset_x + local_up * offset_y + velocity * sub_step;

            self.exhaust.push(ExhaustParticle {
                position: spawn_pos,
                velocity,
                lifetime: CORE_LIFETIME,
                max_lifetime: CORE_LIFETIME,
                kind: ExhaustKind::Core,
            });
        }

        while self.flame_timer >= flame_interval {
            self.flame_timer -= flame_interval;
            let jitter_x = self.next_rand_bipolar() * FLAME_SPREAD;
            let jitter_y = self.next_rand_bipolar() * FLAME_SPREAD;
            let speed = self.speed
                + self.trail_speed * FLAME_SPEED_FACTOR * (0.8 + self.next_rand_f32() * 0.4);
            let velocity = backward * speed
                + local_right * (jitter_x * self.trail_speed)
                + local_up * (jitter_y * self.trail_speed);
            let sub_step = self.next_rand_f32() * frame_time * 0.5;
            let offset_x = self.next_rand_bipolar() * nozzle_radius * 0.95;
            let offset_y = self.next_rand_bipolar() * nozzle_radius * 0.95;
            let spawn_pos =
                origin + local_right * offset_x + local_up * offset_y + velocity * sub_step;

            self.exhaust.push(ExhaustParticle {
                position: spawn_pos,
                velocity,
                lifetime: FLAME_LIFETIME,
                max_lifetime: FLAME_LIFETIME,
                kind: ExhaustKind::Flame,
            });
        }

        while self.spark_timer >= spark_interval {
            self.spark_timer -= spark_interval;
            let jitter_x = self.next_rand_bipolar() * SPARK_SPREAD;
            let jitter_y = self.next_rand_bipolar() * SPARK_SPREAD;
            let speed = self.speed
                + self.trail_speed * SPARK_SPEED_FACTOR * (0.7 + self.next_rand_f32() * 0.6);
            let velocity = backward * speed
                + local_right * (jitter_x * self.trail_speed)
                + local_up * (jitter_y * self.trail_speed);
            let offset_x = self.next_rand_bipolar() * nozzle_radius * 0.5;
            let offset_y = self.next_rand_bipolar() * nozzle_radius * 0.5;
            let spawn_pos = origin + local_right * offset_x + local_up * offset_y;
            let lifetime = SPARK_LIFETIME * (0.7 + self.next_rand_f32() * 0.6);

            self.exhaust.push(ExhaustParticle {
                position: spawn_pos,
                velocity,
                lifetime,
                max_lifetime: SPARK_LIFETIME,
                kind: ExhaustKind::Spark,
            });
        }

        while self.smoke_timer >= smoke_interval {
            self.smoke_timer -= smoke_interval;
            let jitter_x = self.next_rand_bipolar() * SMOKE_SPREAD;
            let jitter_y = self.next_rand_bipolar() * SMOKE_SPREAD;
            let speed = self.speed
                + self.trail_speed * SMOKE_SPEED_FACTOR * (0.7 + self.next_rand_f32() * 0.5);
            let offset_x = self.next_rand_bipolar() * nozzle_radius;
            let offset_y = self.next_rand_bipolar() * nozzle_radius;
            let spawn_pos = origin + local_right * offset_x + local_up * offset_y;
            let velocity = backward * speed
                + local_right * (jitter_x * self.trail_speed)
                + local_up * (jitter_y * self.trail_speed)
                + Vector3::new(0.0, self.trail_speed * SMOKE_RISE_FACTOR, 0.0);

            self.exhaust.push(ExhaustParticle {
                position: spawn_pos,
                velocity,
                lifetime: SMOKE_LIFETIME,
                max_lifetime: SMOKE_LIFETIME,
                kind: ExhaustKind::Smoke,
            });
        }
    }

    pub fn draw_exhaust(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        let reference_distance = self.min_distance * INITIAL_CAMERA_DISTANCE_FACTOR;

        for particle in &self.exhaust {
            let Some((position, depth)) = project_vertex(
                particle.position,
                uniforms.camera_pos,
                uniforms.camera_target,
                TRAIL_PROJECTION_SCALE,
                framebuffer.width as f32,
                framebuffer.height as f32,
            ) else {
                continue;
            };

            let dist_to_cam = (particle.position - uniforms.camera_pos).length().max(0.1);
            let raw_scale = reference_distance / dist_to_cam;

            // t = 1.0 (pegado al origen/tobera), t = 0.0 (lejos del origen/extinguiéndose)
            let t = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);
            // Boost concentrado en el origen (salida de la tobera)
            let origin_boost = t * t * t;

            let (color, radius_mult) = match particle.kind {
                ExhaustKind::Core => {
                    let base_col = lerp_color(COLOR_CORE_WARM, COLOR_CORE_HOT, t);
                    let intensity = 0.85 + 0.15 * t;
                    // Grande en el origen (t=1.0), se afila a chispa pequeña al salir
                    (base_col * intensity, 0.65 + origin_boost * 1.15)
                }
                ExhaustKind::Flame => {
                    let base_col = if t > 0.5 {
                        lerp_color(COLOR_FLAME_MID, COLOR_FLAME_HOT, (t - 0.5) * 2.0)
                    } else {
                        lerp_color(COLOR_FLAME_COOL, COLOR_FLAME_MID, t * 2.0)
                    };
                    let intensity = (t * 1.35).min(1.0);
                    // Muy prominente al salir del origen y se encoge como chispas al alejarse
                    (base_col * intensity, 0.55 + t * 0.65 + origin_boost * 0.95)
                }
                ExhaustKind::Spark => {
                    let base_col = lerp_color(COLOR_FLAME_MID, COLOR_SPARK, t);
                    (base_col * (0.85 + 0.15 * t), 0.35 + origin_boost * 0.65)
                }
                ExhaustKind::Smoke => {
                    let progress = 1.0 - t;
                    let base_col = lerp_color(COLOR_SMOKE_START, COLOR_SMOKE_END, progress);
                    let alpha = (t * 0.6).clamp(0.0, 1.0);
                    (base_col * alpha, 0.9 + progress * 1.6)
                }
            };

            let far_radius = ((EXHAUST_BASE_RADIUS * radius_mult * raw_scale.min(1.0)).round()
                as i32)
                .clamp(EXHAUST_MIN_RADIUS, EXHAUST_FAR_MAX_RADIUS);
            let close_growth =
                ((raw_scale - 1.0).max(0.0) * EXHAUST_CLOSE_GROWTH * EXHAUST_CLOSE_DENSITY_BOOST)
                    .round() as i32;
            let radius =
                (far_radius + close_growth).clamp(EXHAUST_MIN_RADIUS, EXHAUST_CLOSE_MAX_RADIUS);

            draw_square(framebuffer, position, depth, radius, color);
        }
    }
}

fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> Vector3 {
    let t = t.clamp(0.0, 1.0);
    Vector3::new(
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    )
}

fn scaled_trail_radius(camera_distance: f32, reference_distance: f32) -> i32 {
    ((TRAIL_POINT_RADIUS as f32 * reference_distance / camera_distance).round() as i32)
        .clamp(TRAIL_MIN_POINT_RADIUS, TRAIL_MAX_POINT_RADIUS)
}

fn draw_thick_line(
    framebuffer: &mut Framebuffer,
    start: Vector2,
    end: Vector2,
    start_depth: f32,
    end_depth: f32,
    radius: i32,
    color: Vector3,
) {
    let delta = end - start;
    let steps = delta.x.abs().max(delta.y.abs()).ceil() as i32;
    if steps == 0 {
        draw_square(framebuffer, start, start_depth, radius, color);
        return;
    }
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        draw_square(
            framebuffer,
            start + delta * t,
            start_depth + (end_depth - start_depth) * t,
            radius,
            color,
        );
    }
}

fn draw_square(
    framebuffer: &mut Framebuffer,
    position: Vector2,
    depth: f32,
    radius: i32,
    color: Vector3,
) {
    for y in -radius..=radius {
        for x in -radius..=radius {
            framebuffer.point(position.x as i32 + x, position.y as i32 + y, depth, color);
        }
    }
}

fn transform_point(matrix: &Matrix, point: Vector3) -> Vector3 {
    transform(matrix, point, 1.0)
}

fn transform_direction(matrix: &Matrix, direction: Vector3) -> Vector3 {
    transform(matrix, direction, 0.0)
}

fn transform(matrix: &Matrix, vector: Vector3, w: f32) -> Vector3 {
    Vector3::new(
        matrix.m0 * vector.x + matrix.m4 * vector.y + matrix.m8 * vector.z + matrix.m12 * w,
        matrix.m1 * vector.x + matrix.m5 * vector.y + matrix.m9 * vector.z + matrix.m13 * w,
        matrix.m2 * vector.x + matrix.m6 * vector.y + matrix.m10 * vector.z + matrix.m14 * w,
    )
}

fn axis_target(positive: bool, negative: bool, amount: f32) -> f32 {
    match (positive, negative) {
        (true, false) => amount,
        (false, true) => -amount,
        _ => 0.0,
    }
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}
