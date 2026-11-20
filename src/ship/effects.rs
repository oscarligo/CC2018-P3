use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::projection::{project_vertex, projected_sphere_radius};
use crate::rendering::uniforms::Uniforms;
use raylib::prelude::*;
use std::f32::consts::TAU;

// --- Wingtip trail ---
const TRAIL_CAPACITY: usize = 64;
const PARTICLE_INTENSITY: f32 = 0.25;
const TRAIL_SPEED_FACTOR: f32 = 2.0;
const TRAIL_PARTICLE_LIFETIME: f32 = 0.5;
const TRAIL_EMISSION_INTERVAL: f32 = 1.0 / 30.0;
const TRAIL_COLOR: [f32; 3] = [0.85, 0.92, 1.0];
const TRAIL_POINT_RADIUS: i32 = 0;
const TRAIL_MIN_POINT_RADIUS: i32 = 1;
const TRAIL_MAX_POINT_RADIUS: i32 = 1;

// --- Exhaust physics & thermodynamics ---
const GAS_CAPACITY: usize = 420;
const SPARK_CAPACITY: usize = 180;
const EXHAUST_SPEED_REFERENCE_RATIO: f32 = 0.45;
const CORE_EMISSION_INTERVAL: f32 = 1.0 / 80.0;
const PLUME_EMISSION_INTERVAL: f32 = 1.0 / 95.0;
const SPARK_EMISSION_INTERVAL: f32 = 1.0 / 50.0;
const EXHAUST_CLOSE_DENSITY_BOOST: f32 = 1.0;
const EXHAUST_CLOSE_MAX_DENSITY: f32 = 1.55;

const CORE_LIFETIME: f32 = 0.14;
const PLUME_LIFETIME: f32 = 0.65;
const SPARK_LIFETIME: f32 = 0.48;
const WAKE_VELOCITY_INHERITANCE: f32 = 0.72;

const CORE_EXIT_SPEED: f32 = 2.4;
const PLUME_EXIT_SPEED: f32 = 2.85;
const SPARK_EXIT_SPEED: f32 = 5.4;

const FLAME_DRAG: f32 = 3.4;
const SMOKE_DRAG: f32 = 0.0;
const SPARK_DRAG: f32 = 3.4;
const SMOKE_BUOYANCY: f32 = 0.42;
const SMOKE_TRANSITION_TEMP: f32 = 0.0;

const NOZZLE_SEGMENTS: usize = 10;

// --- Pixel-art field & palettes ---
const PIXEL_CELL_SIZE: i32 = 3;
const MAX_GRID_CELLS: usize = 48_000;
const FLAME_THRESHOLD: f32 = 0.56;
const SMOKE_THRESHOLD: f32 = 999.50;
const GLOW_THRESHOLD: f32 = 0.22;

const FIRE_PALETTE: [[u8; 3]; 7] = [
    [98, 20, 8],
    [164, 34, 8],
    [220, 62, 10],
    [242, 108, 12],
    [250, 165, 25],
    [255, 218, 69],
    [255, 250, 195],
];

const ALIEN_FIRE_PALETTE: [[u8; 3]; 7] = [
    [10, 68, 36],
    [18, 118, 52],
    [32, 172, 70],
    [62, 218, 92],
    [118, 244, 122],
    [182, 255, 168],
    [235, 255, 232],
];

const SMOKE_PALETTE: [[u8; 3]; 4] = [[24, 22, 26], [44, 42, 48], [70, 68, 76], [104, 102, 112]];

const BAYER_4X4: [[f32; 4]; 4] = [
    [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
    [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
    [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
];

pub struct ShipEffects {
    alien: bool,
    wing_tips: [Vector3; 2],
    rear_point: Vector3,
    ship_radius: f32,
    trail_speed: f32,
    trail: Vec<TrailParticle>,
    gas: Vec<GasParcel>,
    sparks: Vec<SparkParticle>,
    prev_nozzle_origin: Option<Vector3>,
    engine_power: f32,
    exhaust_time: f32,
    trail_timer: f32,
    core_timer: f32,
    plume_timer: f32,
    spark_timer: f32,
    rng_state: u32,
}

struct TrailParticle {
    position: Vector3,
    velocity: Vector3,
    lifetime: f32,
    wing: usize,
}

#[derive(Clone, Copy)]
struct GasParcel {
    position: Vector3,
    velocity: Vector3,
    temperature: f32,
    cooling_rate: f32,
    radius: f32,
    initial_radius: f32,
    expansion_rate: f32,
    age: f32,
    max_lifetime: f32,
    seed: u32,
    is_core: bool,
}

#[derive(Clone, Copy)]
struct SparkParticle {
    position: Vector3,
    prev_position: Vector3,
    velocity: Vector3,
    lifetime: f32,
    max_lifetime: f32,
    seed: u32,
}

#[derive(Clone, Copy)]
struct FieldSplat {
    center: Vector2,
    depth: f32,
    radius: f32,
    heat: f32,
    core_heat: f32,
    smoke: f32,
}

#[derive(Clone, Copy)]
struct FieldCell {
    heat: f32,
    core_heat: f32,
    smoke: f32,
    flame_depth: f32,
    smoke_depth: f32,
}

impl Default for FieldCell {
    fn default() -> Self {
        Self {
            heat: 0.0,
            core_heat: 0.0,
            smoke: 0.0,
            flame_depth: f32::INFINITY,
            smoke_depth: f32::INFINITY,
        }
    }
}

struct ExhaustField {
    origin_x: i32,
    origin_y: i32,
    width: usize,
    height: usize,
    cell_size: i32,
    cells: Vec<FieldCell>,
}

impl ExhaustField {
    fn build(splats: &[FieldSplat], fb_width: i32, fb_height: i32) -> Option<Self> {
        if fb_width <= 0 || fb_height <= 0 || splats.is_empty() {
            return None;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for splat in splats {
            let pad = splat.radius.max(PIXEL_CELL_SIZE as f32) + PIXEL_CELL_SIZE as f32;
            min_x = min_x.min(splat.center.x - pad);
            min_y = min_y.min(splat.center.y - pad);
            max_x = max_x.max(splat.center.x + pad);
            max_y = max_y.max(splat.center.y + pad);
        }
        if max_x < 0.0 || max_y < 0.0 || min_x >= fb_width as f32 || min_y >= fb_height as f32 {
            return None;
        }
        let min_x = (min_x.floor() as i32).clamp(0, fb_width - 1);
        let min_y = (min_y.floor() as i32).clamp(0, fb_height - 1);
        let max_x = (max_x.ceil() as i32).clamp(0, fb_width - 1);
        let max_y = (max_y.ceil() as i32).clamp(0, fb_height - 1);

        let mut cell_size = PIXEL_CELL_SIZE;
        let (origin_x, origin_y, width, height) = loop {
            let ox = min_x.div_euclid(cell_size) * cell_size;
            let oy = min_y.div_euclid(cell_size) * cell_size;
            let w = ((max_x - ox) / cell_size + 1).max(1) as usize;
            let h = ((max_y - oy) / cell_size + 1).max(1) as usize;
            if w * h <= MAX_GRID_CELLS || cell_size >= 8 {
                break (ox, oy, w, h);
            }
            cell_size += 1;
        };

        let mut field = Self {
            origin_x,
            origin_y,
            width,
            height,
            cell_size,
            cells: vec![FieldCell::default(); width * height],
        };
        for splat in splats {
            field.splat(*splat);
        }
        Some(field)
    }

    fn splat(&mut self, splat: FieldSplat) {
        let r = splat.radius.max(self.cell_size as f32 * 0.85);
        let inv_r_sq = 1.0 / (r * r);
        let cs = self.cell_size as f32;
        let gx0 = ((splat.center.x - r - self.origin_x as f32) / cs)
            .floor()
            .max(0.0) as usize;
        let gy0 = ((splat.center.y - r - self.origin_y as f32) / cs)
            .floor()
            .max(0.0) as usize;
        let gx1 = (((splat.center.x + r - self.origin_x as f32) / cs).ceil() as isize)
            .clamp(0, self.width as isize - 1) as usize;
        let gy1 = (((splat.center.y + r - self.origin_y as f32) / cs).ceil() as isize)
            .clamp(0, self.height as isize - 1) as usize;
        if gx0 > gx1 || gy0 > gy1 || gx0 >= self.width || gy0 >= self.height {
            return;
        }

        let half = cs * 0.5;
        for gy in gy0..=gy1 {
            let cy = self.origin_y as f32 + gy as f32 * cs + half;
            let dy = cy - splat.center.y;
            let dy_sq = dy * dy;
            let row = gy * self.width;
            for gx in gx0..=gx1 {
                let cx = self.origin_x as f32 + gx as f32 * cs + half;
                let dx = cx - splat.center.x;
                let norm_sq = (dx * dx + dy_sq) * inv_r_sq;
                if norm_sq >= 1.0 {
                    continue;
                }
                let one_minus = 1.0 - norm_sq;
                let weight = one_minus * one_minus;
                let cell = &mut self.cells[row + gx];
                if splat.heat > 0.0 {
                    cell.heat += splat.heat * weight;
                    if weight > 0.08 {
                        cell.flame_depth = cell.flame_depth.min(splat.depth);
                    }
                }
                if splat.core_heat > 0.0 {
                    cell.core_heat += splat.core_heat * weight;
                }
                if splat.smoke > 0.0 {
                    cell.smoke += splat.smoke * weight;
                    if weight > 0.08 {
                        cell.smoke_depth = cell.smoke_depth.min(splat.depth);
                    }
                }
            }
        }
    }

    fn cell(&self, gx: usize, gy: usize) -> FieldCell {
        self.cells[gy * self.width + gx]
    }

    fn smoke_at(&self, gx: isize, gy: isize) -> f32 {
        if gx < 0 || gy < 0 || gx >= self.width as isize || gy >= self.height as isize {
            0.0
        } else {
            self.cells[gy as usize * self.width + gx as usize].smoke
        }
    }

    fn heat_at(&self, gx: isize, gy: isize) -> f32 {
        if gx < 0 || gy < 0 || gx >= self.width as isize || gy >= self.height as isize {
            0.0
        } else {
            self.cells[gy as usize * self.width + gx as usize].heat
        }
    }

    fn rasterize(&self, fb: &mut Framebuffer, exhaust_time: f32, alien: bool) {
        let fb_w = fb.width as i32;
        let fb_h = fb.height as i32;
        let anim_tick = (exhaust_time * 18.0).floor() as i32;
        let glow_color = if alien {
            Vector3::new(0.18, 1.0, 0.36)
        } else {
            Vector3::new(1.0, 0.42, 0.08)
        };

        for gy in 0..self.height {
            let sy = self.origin_y + gy as i32 * self.cell_size;
            let by = sy.div_euclid(self.cell_size).rem_euclid(4) as usize;
            let cy_idx = sy.div_euclid(self.cell_size);
            for gx in 0..self.width {
                let cell = self.cell(gx, gy);
                if cell.heat < GLOW_THRESHOLD && cell.smoke < SMOKE_THRESHOLD * 0.6 {
                    continue;
                }
                let sx = self.origin_x + gx as i32 * self.cell_size;
                let bx = sx.div_euclid(self.cell_size).rem_euclid(4) as usize;
                let cx_idx = sx.div_euclid(self.cell_size);
                let bayer = BAYER_4X4[by][bx] - 0.5;
                let flicker = cell_flicker_noise(cx_idx, cy_idx, anim_tick);

                let flame_val =
                    cell.heat + cell.core_heat * 0.65 + (flicker - 0.5) * 0.24 * cell.heat.min(1.0);
                let smoke_val = cell.smoke + (flicker - 0.5) * 0.14 * cell.smoke.min(1.0);

                let flame_active =
                    cell.flame_depth.is_finite() && flame_val + bayer * 0.14 >= FLAME_THRESHOLD;
                let smoke_active =
                    cell.smoke_depth.is_finite() && smoke_val + bayer * 0.12 >= SMOKE_THRESHOLD;

                if flame_active && (!smoke_active || cell.flame_depth <= cell.smoke_depth + 0.12) {
                    let color = shade_flame_cell(flame_val, cell.core_heat, bayer, alien);
                    fill_cell_opaque(
                        fb,
                        sx,
                        sy,
                        self.cell_size,
                        fb_w,
                        fb_h,
                        cell.flame_depth,
                        color,
                    );
                } else if smoke_active {
                    let gx_i = gx as isize;
                    let gy_i = gy as isize;
                    let grad_x = self.smoke_at(gx_i + 1, gy_i) - self.smoke_at(gx_i - 1, gy_i);
                    let grad_y = self.smoke_at(gx_i, gy_i + 1) - self.smoke_at(gx_i, gy_i - 1);
                    let nearby_heat = cell.heat
                        + 0.25
                            * (self.heat_at(gx_i + 1, gy_i)
                                + self.heat_at(gx_i - 1, gy_i)
                                + self.heat_at(gx_i, gy_i + 1)
                                + self.heat_at(gx_i, gy_i - 1));
                    let color =
                        shade_smoke_cell(smoke_val, grad_x, grad_y, nearby_heat, bayer, alien);
                    fill_cell_opaque(
                        fb,
                        sx,
                        sy,
                        self.cell_size,
                        fb_w,
                        fb_h,
                        cell.smoke_depth,
                        color,
                    );
                } else if cell.heat >= GLOW_THRESHOLD && cell.flame_depth.is_finite() {
                    let glow_strength = ((cell.heat - GLOW_THRESHOLD)
                        / (FLAME_THRESHOLD - GLOW_THRESHOLD))
                        .clamp(0.0, 1.0);
                    let intensity = glow_strength * PARTICLE_INTENSITY * 0.85;
                    fill_cell_light(
                        fb,
                        sx,
                        sy,
                        self.cell_size,
                        fb_w,
                        fb_h,
                        cell.flame_depth,
                        glow_color,
                        intensity,
                    );
                }
            }
        }
    }
}

impl ShipEffects {
    pub fn new(wing_tips: [Vector3; 2], rear_point: Vector3, ship_radius: f32) -> Self {
        let safe_radius = ship_radius.max(0.01);
        Self {
            alien: false,
            wing_tips,
            rear_point,
            ship_radius: safe_radius,
            trail_speed: safe_radius * TRAIL_SPEED_FACTOR,
            trail: Vec::with_capacity(TRAIL_CAPACITY),
            gas: Vec::with_capacity(GAS_CAPACITY),
            sparks: Vec::with_capacity(SPARK_CAPACITY),
            prev_nozzle_origin: None,
            engine_power: 0.0,
            exhaust_time: 0.0,
            trail_timer: 0.0,
            core_timer: 0.0,
            plume_timer: 0.0,
            spark_timer: 0.0,
            rng_state: 0x8542_5987,
        }
    }

    pub fn set_alien(&mut self, alien: bool) {
        self.alien = alien;
    }

    pub fn clear(&mut self) {
        self.trail.clear();
        self.gas.clear();
        self.sparks.clear();
        self.prev_nozzle_origin = None;
        self.engine_power = 0.0;
        self.exhaust_time = 0.0;
        self.trail_timer = 0.0;
        self.core_timer = 0.0;
        self.plume_timer = 0.0;
        self.spark_timer = 0.0;
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
        let dt = frame_time.max(0.0);
        if dt == 0.0 {
            return;
        }
        self.update_trail(trail_emitting, dt, ship_velocity, uniforms);
        self.update_exhaust(
            exhaust_emitting,
            dt,
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
            let backward = safe_normalize(transform_direction(
                &uniforms.model_matrix,
                Vector3::new(0.0, 0.0, -1.0),
            ));
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
            let color = if self.alien {
                Vector3::new(0.2, 1.0, 0.38) * intensity
            } else {
                Vector3::new(TRAIL_COLOR[0], TRAIL_COLOR[1], TRAIL_COLOR[2]) * intensity
            };
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
        self.exhaust_time += frame_time;
        let target_power = if emitting {
            0.55 + speed_ratio.clamp(0.0, 1.0) * 0.45
        } else {
            0.0
        };
        let power_rate = if emitting { 18.0 } else { 11.0 };
        self.engine_power +=
            (target_power - self.engine_power) * (1.0 - (-frame_time * power_rate).exp());

        let exhaust_time = self.exhaust_time;
        let ship_radius = self.ship_radius;
        for parcel in &mut self.gas {
            step_gas_parcel(parcel, frame_time, exhaust_time, ship_radius);
        }
        self.gas.retain(|parcel| parcel.age < parcel.max_lifetime);

        for spark in &mut self.sparks {
            step_spark(spark, frame_time, exhaust_time, ship_radius);
        }
        self.sparks.retain(|spark| spark.lifetime > 0.0);

        if !emitting {
            self.prev_nozzle_origin = None;
            self.core_timer = 0.0;
            self.plume_timer = 0.0;
            self.spark_timer = 0.0;
            return;
        }

        let origin = transform_point(&uniforms.model_matrix, self.rear_point);
        let prev_origin = self.prev_nozzle_origin.unwrap_or(origin);
        self.prev_nozzle_origin = Some(origin);

        let backward = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(0.0, 0.0, -1.0),
        ));
        let right = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(1.0, 0.0, 0.0),
        ));
        let up = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(0.0, 1.0, 0.0),
        ));
        let nozzle_radius = self.ship_radius * 0.085;

        let thrust_power = (speed_ratio / EXHAUST_SPEED_REFERENCE_RATIO).clamp(0.55, 2.2);
        let zoom_ratio = reference_distance / camera_distance.max(0.001);
        let density = if zoom_ratio > 1.0 {
            (zoom_ratio * EXHAUST_CLOSE_DENSITY_BOOST).min(EXHAUST_CLOSE_MAX_DENSITY)
        } else {
            zoom_ratio.clamp(0.75, 1.0)
        };

        self.core_timer = (self.core_timer + frame_time * thrust_power).min(0.12);
        self.plume_timer = (self.plume_timer + frame_time * thrust_power).min(0.12);
        self.spark_timer = (self.spark_timer + frame_time * thrust_power).min(0.12);

        self.emit_gas_stream(
            true,
            CORE_EMISSION_INTERVAL / density,
            frame_time,
            thrust_power,
            prev_origin,
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
        self.emit_gas_stream(
            false,
            PLUME_EMISSION_INTERVAL / density,
            frame_time,
            thrust_power,
            prev_origin,
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
        self.emit_sparks(
            SPARK_EMISSION_INTERVAL / density,
            frame_time,
            thrust_power,
            prev_origin,
            origin,
            backward,
            right,
            up,
            ship_velocity,
            nozzle_radius,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_gas_stream(
        &mut self,
        is_core: bool,
        interval: f32,
        frame_time: f32,
        thrust_power: f32,
        prev_origin: Vector3,
        origin: Vector3,
        backward: Vector3,
        right: Vector3,
        up: Vector3,
        ship_velocity: Vector3,
        nozzle_radius: f32,
    ) {
        let timer = if is_core {
            self.core_timer
        } else {
            self.plume_timer
        };
        let count = (timer / interval).floor() as usize;
        if count == 0 {
            return;
        }
        let remainder = timer - count as f32 * interval;
        if is_core {
            self.core_timer = remainder;
        } else {
            self.plume_timer = remainder;
        }

        let thrust_vel_scale = 0.72 + thrust_power.min(1.8) * 0.28;
        for i in 0..count {
            if self.gas.len() >= GAS_CAPACITY {
                break;
            }
            // Interpolate nozzle origin across the frame so fast motion leaves no discrete beads.
            let frac = (i as f32 + self.next_rand_f32()) / count as f32;
            let spawn_origin = prev_origin + (origin - prev_origin) * frac;
            let sub_dt = (1.0 - frac) * frame_time;

            let (disk_x, disk_y) = self.next_disk_sample();
            let radial_dist = (disk_x * disk_x + disk_y * disk_y).sqrt();
            let radial_scale = if is_core { 0.45 } else { 0.92 };
            let nozzle_offset = right * (disk_x * nozzle_radius * radial_scale)
                + up * (disk_y * nozzle_radius * radial_scale);

            let spread = if is_core { 0.12 } else { 0.34 };
            let exit_base = if is_core {
                CORE_EXIT_SPEED
            } else {
                PLUME_EXIT_SPEED
            };
            let exit_speed = self.ship_radius
                * exit_base
                * thrust_vel_scale
                * (0.82 + self.next_rand_f32() * 0.36)
                * (1.0 - radial_dist * 0.18);
            let lateral_x = (disk_x * 0.55 + self.next_rand_bipolar() * 0.45) * spread;
            let lateral_y = (disk_y * 0.55 + self.next_rand_bipolar() * 0.45) * spread;
            let velocity = ship_velocity * WAKE_VELOCITY_INHERITANCE
                + backward * exit_speed
                + right * (lateral_x * self.ship_radius)
                + up * (lateral_y * self.ship_radius);

            let initial_radius = if is_core {
                nozzle_radius * (0.72 + self.next_rand_f32() * 0.25)
            } else {
                nozzle_radius * (0.85 + self.next_rand_f32() * 0.40)
            };
            let max_lifetime = if is_core {
                CORE_LIFETIME * (0.8 + self.next_rand_f32() * 0.4)
            } else {
                PLUME_LIFETIME * (0.75 + self.next_rand_f32() * 0.5)
            };
            // Outer shear-layer parcels cool faster than the insulated hot inner jet.
            let cooling_rate = if is_core {
                2.4 + self.next_rand_f32() * 0.7
            } else {
                1.25 + radial_dist * 0.65 + self.next_rand_f32() * 0.45
            };
            let temperature = if is_core {
                1.05 + self.next_rand_f32() * 0.15
            } else {
                0.88 - radial_dist * 0.18 + self.next_rand_f32() * 0.16
            };
            let expansion_rate = if is_core {
                self.ship_radius * 0.08
            } else {
                self.ship_radius * (0.14 + self.next_rand_f32() * 0.08)
            };

            let mut parcel = GasParcel {
                position: spawn_origin + nozzle_offset,
                velocity,
                temperature,
                cooling_rate,
                radius: initial_radius,
                initial_radius,
                expansion_rate,
                age: 0.0,
                max_lifetime,
                seed: self.rng_state,
                is_core,
            };
            if sub_dt > 0.0 {
                step_gas_parcel(&mut parcel, sub_dt, self.exhaust_time, self.ship_radius);
            }
            self.gas.push(parcel);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_sparks(
        &mut self,
        interval: f32,
        frame_time: f32,
        thrust_power: f32,
        prev_origin: Vector3,
        origin: Vector3,
        backward: Vector3,
        right: Vector3,
        up: Vector3,
        ship_velocity: Vector3,
        nozzle_radius: f32,
    ) {
        let count = (self.spark_timer / interval).floor() as usize;
        if count == 0 {
            return;
        }
        self.spark_timer -= count as f32 * interval;

        let thrust_vel_scale = 0.75 + thrust_power.min(1.8) * 0.30;
        for i in 0..count {
            if self.sparks.len() >= SPARK_CAPACITY {
                break;
            }
            let frac = (i as f32 + self.next_rand_f32()) / count as f32;
            let spawn_origin = prev_origin + (origin - prev_origin) * frac;
            let sub_dt = (1.0 - frac) * frame_time;

            let (disk_x, disk_y) = self.next_disk_sample();
            let pos = spawn_origin
                + right * (disk_x * nozzle_radius * 0.85)
                + up * (disk_y * nozzle_radius * 0.85);
            let speed = self.ship_radius
                * SPARK_EXIT_SPEED
                * thrust_vel_scale
                * (0.7 + self.next_rand_f32() * 0.65);
            let velocity = ship_velocity * WAKE_VELOCITY_INHERITANCE
                + backward * speed
                + right * (self.next_rand_bipolar() * self.ship_radius * 1.15)
                + up * (self.next_rand_bipolar() * self.ship_radius * 1.15);
            let max_lifetime = SPARK_LIFETIME * (0.6 + self.next_rand_f32() * 0.75);
            let mut spark = SparkParticle {
                position: pos,
                prev_position: pos,
                velocity,
                lifetime: max_lifetime,
                max_lifetime,
                seed: self.rng_state,
            };
            if sub_dt > 0.0 {
                step_spark(&mut spark, sub_dt, self.exhaust_time, self.ship_radius);
            }
            self.sparks.push(spark);
        }
    }

    pub fn draw_exhaust(
        &self,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
        _reference_distance: f32,
    ) {
        if self.engine_power < 0.01 && self.gas.is_empty() && self.sparks.is_empty() {
            return;
        }
        self.draw_engine_light(framebuffer, uniforms);

        let mut splats = Vec::with_capacity(self.gas.len() + NOZZLE_SEGMENTS);
        self.collect_nozzle_splats(framebuffer, uniforms, &mut splats);
        self.collect_gas_splats(framebuffer, uniforms, &mut splats);

        if let Some(field) =
            ExhaustField::build(&splats, framebuffer.width as i32, framebuffer.height as i32)
        {
            field.rasterize(framebuffer, self.exhaust_time, self.alien);
        }

        self.draw_sparks(framebuffer, uniforms);
    }

    fn collect_nozzle_splats(
        &self,
        fb: &Framebuffer,
        uniforms: &Uniforms,
        splats: &mut Vec<FieldSplat>,
    ) {
        if self.engine_power < 0.02 {
            return;
        }
        let backward = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(0.0, 0.0, -1.0),
        ));
        let right = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(1.0, 0.0, 0.0),
        ));
        let up = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(0.0, 1.0, 0.0),
        ));
        let origin = transform_point(&uniforms.model_matrix, self.rear_point)
            + backward * (self.ship_radius * 0.02);
        let nozzle_radius = self.ship_radius * 0.092;
        let jet_length = self.ship_radius * (0.18 + self.engine_power * 0.42);

        for i in 0..NOZZLE_SEGMENTS {
            let u = i as f32 / (NOZZLE_SEGMENTS - 1) as f32;
            // Standing supersonic pressure wave creates subtle pulsing Mach diamonds.
            let mach_wave = 0.5 + 0.5 * (u * TAU * 2.4 - self.exhaust_time * 30.0).sin();
            let wobble_x = (self.exhaust_time * 34.0 + u * 8.5).sin() * u * nozzle_radius * 0.14;
            let wobble_y = (self.exhaust_time * 27.0 + u * 11.0).cos() * u * nozzle_radius * 0.14;
            let world_pos = origin + backward * (jet_length * u) + right * wobble_x + up * wobble_y;
            let Some((center, depth)) = project(fb, uniforms, world_pos) else {
                continue;
            };
            let world_radius = nozzle_radius
                * (1.08 - u * 0.58)
                * (0.86 + 0.24 * mach_wave)
                * (0.45 + 0.55 * self.engine_power);
            let screen_radius =
                projected_sphere_radius(world_pos, world_radius, uniforms, fb.height)
                    .clamp(1.5, 42.0);
            let heat = (1.35 - u * 0.72) * (0.84 + 0.30 * mach_wave) * self.engine_power;
            let core_heat =
                (1.25 - u * 1.08).max(0.0) * (0.78 + 0.38 * mach_wave) * self.engine_power;
            splats.push(FieldSplat {
                center,
                depth,
                radius: screen_radius,
                heat,
                core_heat,
                smoke: 0.0,
            });
        }
    }

    fn collect_gas_splats(
        &self,
        fb: &Framebuffer,
        uniforms: &Uniforms,
        splats: &mut Vec<FieldSplat>,
    ) {
        for parcel in &self.gas {
            let Some((center, depth)) = project(fb, uniforms, parcel.position) else {
                continue;
            };
            let screen_radius =
                projected_sphere_radius(parcel.position, parcel.radius, uniforms, fb.height)
                    .clamp(1.2, 48.0);
            let (heat, core_heat, smoke) = parcel_field_contributions(parcel);
            if heat <= 0.005 && smoke <= 0.005 {
                continue;
            }
            splats.push(FieldSplat {
                center,
                depth,
                radius: screen_radius,
                heat,
                core_heat,
                smoke,
            });
        }
    }

    fn draw_sparks(&self, fb: &mut Framebuffer, uniforms: &Uniforms) {
        let fb_w = fb.width as i32;
        let fb_h = fb.height as i32;
        for spark in &self.sparks {
            let Some((head, head_depth)) = project(fb, uniforms, spark.position) else {
                continue;
            };
            let t = (spark.lifetime / spark.max_lifetime).clamp(0.0, 1.0);
            let level = (3.0 + t * 3.0).round().clamp(2.0, 6.0) as usize;
            let color = fire_palette_color(level, self.alien);
            let snapped_head = pixel_position(head);
            fill_cell_opaque(
                fb,
                snapped_head.x as i32,
                snapped_head.y as i32,
                PIXEL_CELL_SIZE,
                fb_w,
                fb_h,
                head_depth,
                color,
            );
            if let Some((tail, tail_depth)) = project(fb, uniforms, spark.prev_position) {
                let snapped_tail = pixel_position(tail);
                if (snapped_tail - snapped_head).length() >= PIXEL_CELL_SIZE as f32 {
                    let mid = pixel_position((head + tail) * 0.5);
                    let tail_color = fire_palette_color(level.saturating_sub(2), self.alien);
                    fill_cell_opaque(
                        fb,
                        mid.x as i32,
                        mid.y as i32,
                        PIXEL_CELL_SIZE,
                        fb_w,
                        fb_h,
                        (head_depth + tail_depth) * 0.5,
                        tail_color,
                    );
                }
            }
        }
    }

    fn draw_engine_light(&self, fb: &mut Framebuffer, uniforms: &Uniforms) {
        if self.engine_power < 0.01 {
            return;
        }
        let backward = safe_normalize(transform_direction(
            &uniforms.model_matrix,
            Vector3::new(0.0, 0.0, -1.0),
        ));
        let origin = transform_point(&uniforms.model_matrix, self.rear_point)
            + backward * (self.ship_radius * 0.025);
        let facing = backward
            .dot(safe_normalize(uniforms.camera_pos - origin))
            .max(0.0);
        if facing <= 0.0 {
            return;
        }
        let Some((position, depth)) = project(fb, uniforms, origin) else {
            return;
        };
        let radius = projected_sphere_radius(origin, self.ship_radius * 0.075, uniforms, fb.height)
            .round()
            .clamp(1.0, 8.0) as i32;
        let color = if self.alien {
            Vector3::new(0.20, 1.0, 0.40)
        } else {
            Vector3::new(1.0, 0.55, 0.12)
        };
        let power = self.engine_power * facing;
        draw_pixel_particle(fb, position, depth, radius * 2, color * (power * 0.6));
        draw_square(
            fb,
            pixel_position(position),
            depth,
            radius,
            color * (power * 2.0),
        );
        draw_square(
            fb,
            pixel_position(position),
            depth,
            (radius / 2).max(1),
            if self.alien {
                Vector3::new(0.70, 1.0, 0.75)
            } else {
                Vector3::new(1.0, 0.95, 0.70)
            } * (power * 4.0),
        );
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

    fn next_disk_sample(&mut self) -> (f32, f32) {
        let angle = self.next_rand_f32() * TAU;
        let r = self.next_rand_f32().sqrt();
        (angle.cos() * r, angle.sin() * r)
    }
}

fn step_gas_parcel(parcel: &mut GasParcel, dt: f32, exhaust_time: f32, ship_radius: f32) {
    parcel.age += dt;

    // Radiative + convective thermal decay: hotter plasma radiates heat faster.
    let cooling = parcel.cooling_rate * (0.55 + 0.95 * parcel.temperature) * dt;
    parcel.temperature = (parcel.temperature - cooling).max(0.0);

    // Exponential aerodynamic drag slows the jet and curves the wake during maneuvers.
    let drag = if parcel.temperature > SMOKE_TRANSITION_TEMP {
        FLAME_DRAG
    } else {
        SMOKE_DRAG
    };
    parcel.velocity *= (-drag * dt).exp();

    // Kelvin-Helmholtz shear curl grows once the parcel leaves the pressurized nozzle core.
    let laminar_release = (parcel.age / 0.085).clamp(0.0, 1.0);
    let swirl = curl_turbulence(
        parcel.position * (1.0 / ship_radius),
        exhaust_time,
        parcel.seed,
    );
    let turb_strength = ship_radius * if parcel.is_core { 1.35 } else { 3.25 } * laminar_release;
    parcel.velocity += swirl * (turb_strength * dt);

    // Thermal buoyancy gently lifts warm exhaust as it slows and transitions to smoke.
    let age_ratio = (parcel.age / parcel.max_lifetime).clamp(0.0, 1.0);
    let buoyancy_factor = (1.0 - parcel.temperature).clamp(0.0, 1.0) * (1.0 - age_ratio * 0.45);
    parcel.velocity.y += ship_radius * SMOKE_BUOYANCY * buoyancy_factor * dt;

    parcel.position += parcel.velocity * dt;

    // Gas expands radially as pressure equalizes and ambient fluid is entrained.
    let expand_boost = 1.0 + (1.0 - parcel.temperature) * 0.85;
    parcel.radius += parcel.expansion_rate * expand_boost * dt;
}

fn step_spark(spark: &mut SparkParticle, dt: f32, exhaust_time: f32, ship_radius: f32) {
    spark.prev_position = spark.position;
    spark.velocity *= (-SPARK_DRAG * dt).exp();
    let swirl = curl_turbulence(
        spark.position * (1.5 / ship_radius),
        exhaust_time,
        spark.seed,
    );
    spark.velocity += swirl * (ship_radius * 1.4 * dt);
    spark.position += spark.velocity * dt;
    spark.lifetime -= dt;
}

fn parcel_field_contributions(parcel: &GasParcel) -> (f32, f32, f32) {
    let life_remaining = (1.0 - parcel.age / parcel.max_lifetime).clamp(0.0, 1.0);
    let fade_out = (life_remaining / 0.25).clamp(0.0, 1.0);
    let density = (parcel.initial_radius / parcel.radius.max(0.0001))
        .powf(1.1)
        .clamp(0.24, 1.0);

    let heat = if parcel.temperature > 0.16 {
        let normalized = ((parcel.temperature - 0.16) / 0.84).clamp(0.0, 1.25);
        let boost = if parcel.is_core { 1.35 } else { 1.05 };
        normalized * density * fade_out * boost
    } else {
        0.0
    };
    let core_heat = if parcel.temperature > 0.68 {
        ((parcel.temperature - 0.68) / 0.32).clamp(0.0, 1.4) * density * 1.1
    } else {
        0.0
    };
    let smoke = if !parcel.is_core && parcel.temperature < SMOKE_TRANSITION_TEMP {
        let smoke_in = ((SMOKE_TRANSITION_TEMP - parcel.temperature) / 0.26).clamp(0.0, 1.0);
        smoke_in * density * fade_out * 0.88
    } else {
        0.0
    };
    (heat, core_heat, smoke)
}

fn curl_turbulence(scaled_pos: Vector3, time: f32, seed: u32) -> Vector3 {
    let p = Vector3::new(
        scaled_pos.x * 2.1,
        scaled_pos.y * 2.1 - time * 1.1,
        scaled_pos.z * 2.1 + time * 2.3,
    );
    let eps = 0.08;
    let s1 = seed ^ 0x9E37_79B9;
    let s2 = seed ^ 0x85EB_CA6B;
    let s3 = seed ^ 0xC2B2_AE35;

    let dpz_dy =
        (noise_3d(p.x, p.y + eps, p.z, s3) - noise_3d(p.x, p.y - eps, p.z, s3)) / (2.0 * eps);
    let dpy_dz =
        (noise_3d(p.x, p.y, p.z + eps, s2) - noise_3d(p.x, p.y, p.z - eps, s2)) / (2.0 * eps);
    let dpx_dz =
        (noise_3d(p.x, p.y, p.z + eps, s1) - noise_3d(p.x, p.y, p.z - eps, s1)) / (2.0 * eps);
    let dpz_dx =
        (noise_3d(p.x + eps, p.y, p.z, s3) - noise_3d(p.x - eps, p.y, p.z, s3)) / (2.0 * eps);
    let dpy_dx =
        (noise_3d(p.x + eps, p.y, p.z, s2) - noise_3d(p.x - eps, p.y, p.z, s2)) / (2.0 * eps);
    let dpx_dy =
        (noise_3d(p.x, p.y + eps, p.z, s1) - noise_3d(p.x, p.y - eps, p.z, s1)) / (2.0 * eps);

    Vector3::new(dpz_dy - dpy_dz, dpx_dz - dpz_dx, dpy_dx - dpx_dy)
}

fn hash_3d(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (z as u32).wrapping_mul(2_147_483_647)
        ^ seed;
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xFFFF) as f32 / 65535.0
}

fn noise_3d(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let fz = z - iz as f32;
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let uz = fz * fz * (3.0 - 2.0 * fz);

    let c000 = hash_3d(ix, iy, iz, seed);
    let c100 = hash_3d(ix + 1, iy, iz, seed);
    let c010 = hash_3d(ix, iy + 1, iz, seed);
    let c110 = hash_3d(ix + 1, iy + 1, iz, seed);
    let c001 = hash_3d(ix, iy, iz + 1, seed);
    let c101 = hash_3d(ix + 1, iy, iz + 1, seed);
    let c011 = hash_3d(ix, iy + 1, iz + 1, seed);
    let c111 = hash_3d(ix + 1, iy + 1, iz + 1, seed);

    let x00 = c000 + (c100 - c000) * ux;
    let x10 = c010 + (c110 - c010) * ux;
    let x01 = c001 + (c101 - c001) * ux;
    let x11 = c011 + (c111 - c011) * ux;
    let y0 = x00 + (x10 - x00) * uy;
    let y1 = x01 + (x11 - x01) * uy;
    y0 + (y1 - y0) * uz
}

fn cell_flicker_noise(cx: i32, cy: i32, tick: i32) -> f32 {
    hash_3d(cx, cy, tick, 0x51A7_9D31)
}

fn fire_palette_color(level: usize, alien: bool) -> Vector3 {
    let idx = level.min(FIRE_PALETTE.len() - 1);
    let [r, g, b] = if alien {
        ALIEN_FIRE_PALETTE[idx]
    } else {
        FIRE_PALETTE[idx]
    };
    Vector3::new(r as f32, g as f32, b as f32) * (1.0 / 255.0)
}

fn shade_flame_cell(flame_value: f32, core_heat: f32, bayer: f32, alien: bool) -> Vector3 {
    let norm = ((flame_value - FLAME_THRESHOLD) / 2.15).clamp(0.0, 1.0);
    let boosted = (norm.powf(0.68) + (core_heat * 0.28).min(0.45)).clamp(0.0, 1.0);
    let level = ((boosted * 6.0) + bayer * 0.45).round().clamp(0.0, 6.0) as usize;
    fire_palette_color(level, alien)
}

fn shade_smoke_cell(
    smoke_value: f32,
    grad_x: f32,
    grad_y: f32,
    nearby_heat: f32,
    bayer: f32,
    alien: bool,
) -> Vector3 {
    // Volumetric top-left key shading from the scalar field gradient.
    let rim_light = (grad_y * 0.9 - grad_x * 0.45).clamp(-0.6, 0.8);
    let density_tone = ((smoke_value - SMOKE_THRESHOLD) * 0.55).clamp(0.0, 0.5);
    let shade_score =
        (0.38 + rim_light * 0.65 + density_tone * 0.35 + bayer * 0.22).clamp(0.0, 1.0);
    let level = (shade_score * 3.0).round().clamp(0.0, 3.0) as usize;
    let [r, g, b] = SMOKE_PALETTE[level];
    let base = Vector3::new(r as f32, g as f32, b as f32) * (1.0 / 255.0);

    // Nearby hot flame bounces warm orange (or alien green) underlight onto adjacent smoke.
    let bounce = ((nearby_heat - 0.12) * 0.85).clamp(0.0, 0.65);
    if bounce > 0.05 {
        let bounce_tint = if alien {
            Vector3::new(0.14, 0.68, 0.28)
        } else {
            Vector3::new(0.82, 0.30, 0.08)
        };
        let quantized_bounce = (bounce * 3.0).round() / 3.0;
        base + (bounce_tint - base) * quantized_bounce
    } else {
        base
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_cell_opaque(
    fb: &mut Framebuffer,
    sx: i32,
    sy: i32,
    cell_size: i32,
    fb_w: i32,
    fb_h: i32,
    depth: f32,
    color: Vector3,
) {
    for py in 0..cell_size {
        let y = sy + py;
        if y < 0 || y >= fb_h {
            continue;
        }
        for px in 0..cell_size {
            let x = sx + px;
            if x >= 0 && x < fb_w {
                fb.point(x, y, depth, color);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_cell_light(
    fb: &mut Framebuffer,
    sx: i32,
    sy: i32,
    cell_size: i32,
    fb_w: i32,
    fb_h: i32,
    depth: f32,
    color: Vector3,
    intensity: f32,
) {
    for py in 0..cell_size {
        let y = sy + py;
        if y < 0 || y >= fb_h {
            continue;
        }
        for px in 0..cell_size {
            let x = sx + px;
            if x >= 0 && x < fb_w {
                fb.add_light(x, y, depth, color, intensity);
            }
        }
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

fn pixel_position(position: Vector2) -> Vector2 {
    let step = PIXEL_CELL_SIZE as f32;
    Vector2::new(
        (position.x / step).floor() * step,
        (position.y / step).floor() * step,
    )
}

fn draw_pixel_particle(
    fb: &mut Framebuffer,
    position: Vector2,
    depth: f32,
    radius: i32,
    color: Vector3,
) {
    let position = pixel_position(position);
    let radius = (radius.clamp(1, 9) / 2) * 2 + 1;
    for y in -radius..=radius {
        for x in -radius..=radius {
            if x.abs() > radius / 2 && y.abs() > radius / 2 {
                continue;
            }
            let intensity = if x.abs().max(y.abs()) <= radius / 2 {
                1.0
            } else {
                0.45
            };
            fb.add_light(
                position.x as i32 + x,
                position.y as i32 + y,
                depth,
                color,
                PARTICLE_INTENSITY * intensity,
            );
        }
    }
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
            framebuffer.add_light(
                position.x as i32 + x,
                position.y as i32 + y,
                depth,
                color,
                PARTICLE_INTENSITY,
            );
        }
    }
}

fn safe_normalize(v: Vector3) -> Vector3 {
    let len = v.length();
    if len > 1e-6 {
        v * (1.0 / len)
    } else {
        Vector3::new(0.0, 0.0, -1.0)
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
