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

// Trail constants
const TRAIL_CAPACITY: usize = 64;
const TRAIL_SPEED_FACTOR: f32 = 2.0;
const TRAIL_PARTICLE_LIFETIME: f32 = 2.0;
const TRAIL_EMISSION_INTERVAL: f32 = 1.0 / 30.0;
const TRAIL_PROJECTION_SCALE: f32 = 90.0;
const TRAIL_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const TRAIL_POINT_RADIUS: i32 = 1;
const LOCAL_BACKWARD: [f32; 3] = [0.0, 0.0, -1.0];

pub struct ShipMovement {
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
}

struct TrailParticle {
    position: Vector3,
    velocity: Vector3,
    lifetime: f32,
}

impl ShipMovement {
    pub fn new(mesh: &Mesh) -> Self {
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

        Self {
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
        }
    }

    pub fn update(&mut self, input: &InputState, frame_time: f32, uniforms: &mut Uniforms) {
        self.update_camera(input);
        self.update_ship(input, frame_time);
        self.apply(uniforms);
        self.update_trail(input.trail, frame_time, uniforms);
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

    fn apply(&self, uniforms: &mut Uniforms) {
        uniforms.model_matrix = Matrix::translate(self.target.x, self.target.y, self.target.z)
            * Matrix::rotate_xyz(Vector3::new(self.ship_pitch, self.ship_yaw, self.ship_roll))
            * Matrix::translate(-self.target.x, -self.target.y, -self.target.z);

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
            for wing_tip in self.wing_tips {
                self.trail.push(TrailParticle {
                    position: transform_point(&uniforms.model_matrix, wing_tip),
                    velocity: backward * self.trail_speed,
                    lifetime: TRAIL_PARTICLE_LIFETIME,
                });
            }
        }
    }

    pub fn draw_trail(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
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
            for y in -TRAIL_POINT_RADIUS..=TRAIL_POINT_RADIUS {
                for x in -TRAIL_POINT_RADIUS..=TRAIL_POINT_RADIUS {
                    framebuffer.point(position.x as i32 + x, position.y as i32 + y, depth, color);
                }
            }
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
