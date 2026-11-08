mod celestial_shaders;
mod follow_camera;
mod fragment;
mod framebuffer;
mod input;
mod load_objects;
mod noise;
mod performance;
pub mod objects;
pub mod planets;
mod projection;
mod rasterizer;
mod render;
mod rocket_audio;
pub mod solar_system;
mod shaders;
mod ship_effects;
mod ship_movement;
mod triangle;
mod uniforms;
mod vertex;
pub mod world;

use framebuffer::Framebuffer;
use input::InputState;
use performance::PerformanceMonitor;
use raylib::prelude::*;
use render::Renderer;
use rocket_audio::RocketAudio;
use ship_movement::ShipMovement;
use solar_system::build_solar_system;
use std::thread;
use std::time::{Duration, Instant};
use uniforms::Uniforms;
use world::WorldUpdate;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const WINDOW_TITLE: &str = "Software Renderer";
const MODEL_PATH: &str = "assets/red_car.obj";
const FRAME_DELAY: Duration = Duration::from_millis(16);

fn main() {
    if std::env::args().any(|argument| argument == "--benchmark") {
        run_benchmark();
        return;
    }

    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(WINDOW_TITLE)
        .build();
    let audio = RaylibAudio::init_audio_device().expect("Failed to initialize audio");
    let mut rocket_audio = RocketAudio::new(&audio);

    // Load the 3D object from an .obj file
    let mesh = load_objects::load_obj(MODEL_PATH).expect("Failed to load .obj file");
    let mut renderer = Renderer::default();
    let mut performance = PerformanceMonitor::default();
    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);

    let mut uniforms = Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: Vector3::zero(),
        camera_target: Vector3::zero(),
        camera_up: Vector3::new(0.0, 1.0, 0.0),
    };
    let mut movement = ShipMovement::new(&mesh, rocket_audio.moving_duration());
    let mut world = build_solar_system();
    movement.update(&InputState::read(&window), 0.0, &mut uniforms);

    while !window.window_should_close() {
        let frame_start = Instant::now();
        renderer.begin_frame();
        framebuffer.clear();
        let input = InputState::read(&window);
        let frame_time = window.get_frame_time();
        let update_start = Instant::now();
        movement.update(&input, frame_time, &mut uniforms);
        world.update(&WorldUpdate { frame_time });
        rocket_audio.update(
            input.thrust,
            input.braking,
            movement.is_moving(),
            movement.can_accelerate(),
            movement.speed_ratio(),
            frame_time,
        );
        let update_time = update_start.elapsed();

        world.draw(&mut renderer, &mut framebuffer, &uniforms);

        // Render the 3D object to the framebuffer
        renderer.render(&mut framebuffer, &uniforms, &mesh);
        movement.draw_trail(&mut framebuffer, &uniforms);
        movement.draw_exhaust(&mut framebuffer, &uniforms);

        framebuffer.swap_buffers(&mut window, &thread);
        let mut metrics = renderer.metrics();
        metrics.update = update_time;
        metrics.frame = frame_start.elapsed();
        performance.record(metrics);
        thread::sleep(FRAME_DELAY);
    }
}

fn run_benchmark() {
    const BENCHMARK_FRAMES: usize = 240;
    const BENCHMARK_FRAME_TIME: f32 = 1.0 / 60.0;

    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);
    let mut renderer = Renderer::default();
    let mut performance = PerformanceMonitor::default();
    let mut world = build_solar_system();
    let uniforms = Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: Vector3::zero(),
        camera_target: Vector3::new(0.0, 0.0, 1.0),
        camera_up: Vector3::new(0.0, 1.0, 0.0),
    };

    for _ in 0..BENCHMARK_FRAMES {
        let frame_start = Instant::now();
        renderer.begin_frame();
        framebuffer.clear();
        let update_start = Instant::now();
        world.update(&WorldUpdate {
            frame_time: BENCHMARK_FRAME_TIME,
        });
        let update_time = update_start.elapsed();
        world.draw(&mut renderer, &mut framebuffer, &uniforms);

        let mut metrics = renderer.metrics();
        metrics.update = update_time;
        metrics.frame = frame_start.elapsed();
        performance.record(metrics);
    }
}
