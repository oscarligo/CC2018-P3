use crate::framebuffer::Framebuffer;
use crate::projection::project_vertex;
use crate::uniforms::Uniforms;
use raylib::prelude::*;

const TRAIL_CAPACITY: usize = 64;
const TRAIL_SPEED_FACTOR: f32 = 2.0;
const TRAIL_PARTICLE_LIFETIME: f32 = 0.5;
const TRAIL_EMISSION_INTERVAL: f32 = 1.0 / 30.0;
const TRAIL_COLOR: [f32; 3] = [0.85, 0.92, 1.0];
const TRAIL_POINT_RADIUS: i32 = 0;
const TRAIL_MIN_POINT_RADIUS: i32 = 1;
const TRAIL_MAX_POINT_RADIUS: i32 = 1;
const EXHAUST_CAPACITY: usize = 1200;
const EXHAUST_SPEED_REFERENCE_RATIO: f32 = 0.45;
const CORE_LIFETIME: f32 = 0.18;
const FLAME_LIFETIME: f32 = 0.44;
const SPARK_LIFETIME: f32 = 0.28;
const SMOKE_LIFETIME: f32 = 1.35;
const CORE_EMISSION_INTERVAL: f32 = 1.0 / 95.0;
const FLAME_EMISSION_INTERVAL: f32 = 1.0 / 85.0;
const SPARK_EMISSION_INTERVAL: f32 = 1.0 / 65.0;
const SMOKE_EMISSION_INTERVAL: f32 = 1.0 / 16.0;
const EXHAUST_CLOSE_DENSITY_BOOST: f32 = 1.25;
const EXHAUST_CLOSE_MAX_DENSITY: f32 = 12.0;
const CORE_SPEED_FACTOR: f32 = 2.6;
const FLAME_SPEED_FACTOR: f32 = 1.7;
const SPARK_SPEED_FACTOR: f32 = 3.0;
const SMOKE_SPEED_FACTOR: f32 = 0.45;
const SMOKE_DRAG: f32 = 1.6;
const SMOKE_RISE_FACTOR: f32 = 0.22;
const CORE_SPREAD: f32 = 0.05;
const FLAME_SPREAD: f32 = 0.22;
const SPARK_SPREAD: f32 = 0.52;
const SMOKE_SPREAD: f32 = 0.36;
const COLOR_CORE_HOT: [f32; 3] = [0.88, 0.96, 1.0];
const COLOR_CORE_WARM: [f32; 3] = [1.0, 0.92, 0.45];
const COLOR_FLAME_HOT: [f32; 3] = [1.0, 0.82, 0.2];
const COLOR_FLAME_MID: [f32; 3] = [1.0, 0.36, 0.03];
const COLOR_FLAME_COOL: [f32; 3] = [0.42, 0.06, 0.01];
const COLOR_SPARK: [f32; 3] = [1.0, 0.95, 0.65];
const COLOR_SMOKE_START: [f32; 3] = [0.26, 0.24, 0.24];
const COLOR_SMOKE_END: [f32; 3] = [0.65, 0.68, 0.72];
const EXHAUST_BASE_RADIUS: f32 = 4.8;
const EXHAUST_MIN_RADIUS: i32 = 0;
const EXHAUST_FAR_MAX_RADIUS: i32 = 1;
const EXHAUST_CLOSE_MAX_RADIUS: i32 = 20;
const EXHAUST_CLOSE_GROWTH: f32 = 3.0;

pub struct ShipEffects {
    wing_tips: [Vector3; 2],
    rear_point: Vector3,
    trail_speed: f32,
    trail: Vec<TrailParticle>,
    exhaust: Vec<ExhaustParticle>,
    trail_timer: f32,
    core_timer: f32,
    flame_timer: f32,
    spark_timer: f32,
    smoke_timer: f32,
    rng_state: u32,
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

impl ShipEffects {
    pub fn new(wing_tips: [Vector3; 2], rear_point: Vector3, ship_radius: f32) -> Self {
        Self {
            wing_tips,
            rear_point,
            trail_speed: ship_radius * TRAIL_SPEED_FACTOR,
            trail: Vec::with_capacity(TRAIL_CAPACITY),
            exhaust: Vec::with_capacity(EXHAUST_CAPACITY),
            trail_timer: 0.0,
            core_timer: 0.0,
            flame_timer: 0.0,
            spark_timer: 0.0,
            smoke_timer: 0.0,
            rng_state: 0x8542_5987,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        trail_emitting: bool,
        exhaust_emitting: bool,
        frame_time: f32,
        ship_velocity: Vector3,
        speed_ratio: f32,
        camera_distance: f32,
        reference_distance: f32,
        uniforms: &Uniforms,
    ) {
        self.update_trail(trail_emitting, frame_time, ship_velocity, uniforms);
        self.update_exhaust(
            exhaust_emitting,
            frame_time,
            ship_velocity,
            speed_ratio,
            camera_distance,
            reference_distance,
            uniforms,
        );
    }

    fn update_trail(
        &mut self,
        emitting: bool,
        frame_time: f32,
        ship_velocity: Vector3,
        uniforms: &Uniforms,
    ) {
        for particle in &mut self.trail {
            particle.position += particle.velocity * frame_time;
            particle.lifetime -= frame_time;
        }
        self.trail.retain(|particle| particle.lifetime > 0.0);
        if !emitting {
            self.trail_timer = 0.0;
            return;
        }
        self.trail_timer += frame_time;
        while self.trail_timer >= TRAIL_EMISSION_INTERVAL {
            self.trail_timer -= TRAIL_EMISSION_INTERVAL;
            let backward =
                transform_direction(&uniforms.model_matrix, Vector3::new(0.0, 0.0, -1.0));
            for (wing, wing_tip) in self.wing_tips.into_iter().enumerate() {
                self.trail.push(TrailParticle {
                    position: transform_point(&uniforms.model_matrix, wing_tip),
                    velocity: ship_velocity + backward * self.trail_speed,
                    lifetime: TRAIL_PARTICLE_LIFETIME,
                    wing,
                });
            }
        }
    }

    pub fn draw_trail(
        &self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        camera_distance: f32,
        reference_distance: f32,
    ) {
        let point_radius = scaled_trail_radius(camera_distance, reference_distance);
        let mut previous = [None, None];
        for particle in &self.trail {
            let Some((position, depth)) = project(framebuffer, uniforms, particle.position) else {
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

    #[allow(clippy::too_many_arguments)]
    fn update_exhaust(
        &mut self,
        emitting: bool,
        frame_time: f32,
        ship_velocity: Vector3,
        speed_ratio: f32,
        camera_distance: f32,
        reference_distance: f32,
        uniforms: &Uniforms,
    ) {
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

        let thrust_power = (speed_ratio / EXHAUST_SPEED_REFERENCE_RATIO).clamp(0.5, 2.5);
        self.core_timer = (self.core_timer + frame_time * thrust_power).min(0.2);
        self.flame_timer = (self.flame_timer + frame_time * thrust_power).min(0.2);
        self.spark_timer = (self.spark_timer + frame_time * thrust_power).min(0.2);
        self.smoke_timer = (self.smoke_timer + frame_time * thrust_power).min(0.2);

        let origin = transform_point(&uniforms.model_matrix, self.rear_point);
        let backward = transform_direction(&uniforms.model_matrix, Vector3::new(0.0, 0.0, -1.0));
        let right = transform_direction(&uniforms.model_matrix, Vector3::new(1.0, 0.0, 0.0));
        let up = transform_direction(&uniforms.model_matrix, Vector3::new(0.0, 1.0, 0.0));
        let nozzle_radius = self.trail_speed / TRAIL_SPEED_FACTOR * 0.085;
        let zoom_ratio = reference_distance / camera_distance.max(0.001);
        let density = if zoom_ratio > 1.0 {
            (zoom_ratio * zoom_ratio * EXHAUST_CLOSE_DENSITY_BOOST).min(EXHAUST_CLOSE_MAX_DENSITY)
        } else {
            zoom_ratio.clamp(0.8, 1.0)
        };

        self.emit(
            ExhaustKind::Core,
            CORE_EMISSION_INTERVAL / density,
            CORE_LIFETIME,
            CORE_SPEED_FACTOR,
            CORE_SPREAD,
            0.55,
            0.9,
            0.2,
            true,
            frame_time,
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
        self.emit(
            ExhaustKind::Flame,
            FLAME_EMISSION_INTERVAL / density,
            FLAME_LIFETIME,
            FLAME_SPEED_FACTOR,
            FLAME_SPREAD,
            0.95,
            0.8,
            0.4,
            true,
            frame_time,
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
        self.emit(
            ExhaustKind::Spark,
            SPARK_EMISSION_INTERVAL / density,
            SPARK_LIFETIME,
            SPARK_SPEED_FACTOR,
            SPARK_SPREAD,
            0.5,
            0.7,
            0.6,
            false,
            frame_time,
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
        self.emit_smoke(
            SMOKE_EMISSION_INTERVAL / density.min(2.5),
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        kind: ExhaustKind,
        interval: f32,
        max_lifetime: f32,
        speed_factor: f32,
        spread: f32,
        nozzle_factor: f32,
        speed_base: f32,
        speed_variance: f32,
        sub_step: bool,
        frame_time: f32,
        origin: Vector3,
        backward: Vector3,
        right: Vector3,
        up: Vector3,
        ship_velocity: Vector3,
        nozzle_radius: f32,
    ) {
        while *self.timer_mut(kind) >= interval {
            *self.timer_mut(kind) -= interval;
            let jitter_x = self.next_rand_bipolar() * spread;
            let jitter_y = self.next_rand_bipolar() * spread;
            let speed = self.trail_speed
                * speed_factor
                * (speed_base + self.next_rand_f32() * speed_variance);
            let velocity = ship_velocity
                + backward * speed
                + right * (jitter_x * self.trail_speed)
                + up * (jitter_y * self.trail_speed);
            let offset_x = self.next_rand_bipolar() * nozzle_radius * nozzle_factor;
            let offset_y = self.next_rand_bipolar() * nozzle_radius * nozzle_factor;
            let step = if sub_step {
                self.next_rand_f32() * frame_time * 0.5
            } else {
                0.0
            };
            let lifetime = if matches!(kind, ExhaustKind::Spark) {
                max_lifetime * (0.7 + self.next_rand_f32() * 0.6)
            } else {
                max_lifetime
            };
            self.exhaust.push(ExhaustParticle {
                position: origin + right * offset_x + up * offset_y + velocity * step,
                velocity,
                lifetime,
                max_lifetime,
                kind,
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_smoke(
        &mut self,
        interval: f32,
        origin: Vector3,
        backward: Vector3,
        right: Vector3,
        up: Vector3,
        ship_velocity: Vector3,
        nozzle_radius: f32,
    ) {
        while self.smoke_timer >= interval {
            self.smoke_timer -= interval;
            let jitter_x = self.next_rand_bipolar() * SMOKE_SPREAD;
            let jitter_y = self.next_rand_bipolar() * SMOKE_SPREAD;
            let speed = self.trail_speed * SMOKE_SPEED_FACTOR * (0.7 + self.next_rand_f32() * 0.5);
            let offset_x = self.next_rand_bipolar() * nozzle_radius;
            let offset_y = self.next_rand_bipolar() * nozzle_radius;
            self.exhaust.push(ExhaustParticle {
                position: origin + right * offset_x + up * offset_y,
                velocity: ship_velocity
                    + backward * speed
                    + right * (jitter_x * self.trail_speed)
                    + up * (jitter_y * self.trail_speed)
                    + Vector3::new(0.0, self.trail_speed * SMOKE_RISE_FACTOR, 0.0),
                lifetime: SMOKE_LIFETIME,
                max_lifetime: SMOKE_LIFETIME,
                kind: ExhaustKind::Smoke,
            });
        }
    }

    pub fn draw_exhaust(
        &self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        reference_distance: f32,
    ) {
        for particle in &self.exhaust {
            let Some((position, depth)) = project(framebuffer, uniforms, particle.position) else {
                continue;
            };
            let raw_scale =
                reference_distance / (particle.position - uniforms.camera_pos).length().max(0.1);
            let t = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);
            let origin_boost = t * t * t;
            let (color, radius_mult) = match particle.kind {
                ExhaustKind::Core => (
                    lerp_color(COLOR_CORE_WARM, COLOR_CORE_HOT, t) * (0.85 + 0.15 * t),
                    0.65 + origin_boost * 1.15,
                ),
                ExhaustKind::Flame => {
                    let color = if t > 0.5 {
                        lerp_color(COLOR_FLAME_MID, COLOR_FLAME_HOT, (t - 0.5) * 2.0)
                    } else {
                        lerp_color(COLOR_FLAME_COOL, COLOR_FLAME_MID, t * 2.0)
                    };
                    (
                        color * (t * 1.35).min(1.0),
                        0.55 + t * 0.65 + origin_boost * 0.95,
                    )
                }
                ExhaustKind::Spark => (
                    lerp_color(COLOR_FLAME_MID, COLOR_SPARK, t) * (0.85 + 0.15 * t),
                    0.35 + origin_boost * 0.65,
                ),
                ExhaustKind::Smoke => {
                    let progress = 1.0 - t;
                    (
                        lerp_color(COLOR_SMOKE_START, COLOR_SMOKE_END, progress) * (t * 0.6),
                        0.9 + progress * 1.6,
                    )
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

    fn timer_mut(&mut self, kind: ExhaustKind) -> &mut f32 {
        match kind {
            ExhaustKind::Core => &mut self.core_timer,
            ExhaustKind::Flame => &mut self.flame_timer,
            ExhaustKind::Spark => &mut self.spark_timer,
            ExhaustKind::Smoke => &mut self.smoke_timer,
        }
    }

    fn next_rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state as f32 / u32::MAX as f32
    }

    fn next_rand_bipolar(&mut self) -> f32 {
        self.next_rand_f32() * 2.0 - 1.0
    }
}

fn project(
    framebuffer: &Framebuffer,
    uniforms: &Uniforms,
    point: Vector3,
) -> Option<(Vector2, f32)> {
    project_vertex(
        point,
        uniforms.camera_pos,
        uniforms.camera_target,
        uniforms.camera_up,
        framebuffer.width as f32,
        framebuffer.height as f32,
    )
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
