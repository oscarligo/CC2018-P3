mod celestial_shaders;
mod noise;
pub mod objects;
mod performance;
mod physics;
mod rendering;
mod scene;
mod ship;

use objects::celestial::CelestialBody;
use performance::PerformanceMonitor;
use raylib::prelude::*;
use rendering::framebuffer::Framebuffer;
use rendering::render::Renderer;
use rendering::uniforms::Uniforms;
use scene::world::{World, WorldUpdate};
use scene::{Starfield, build_solar_system};
use ship::{InputState, RocketAudio, ShipMovement};
use std::thread;
use std::time::{Duration, Instant};

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const WINDOW_TITLE: &str = "Software Renderer";
const MODEL_PATH: &str = "assets/red_car.obj";
const FRAME_DELAY: Duration = Duration::from_millis(16);

fn main() {
    match benchmark_object_count(std::env::args()) {
        Ok(Some(object_count)) => {
            run_benchmark(object_count);
            return;
        }
        Ok(None) => {}
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    }

    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(WINDOW_TITLE)
        .build();
    let audio = RaylibAudio::init_audio_device().expect("Failed to initialize audio");
    let mut rocket_audio = RocketAudio::new(&audio);

    // Load the 3D object from an .obj file
    let mesh = rendering::load_objects::load_obj(MODEL_PATH).expect("Failed to load .obj file");
    let mut renderer = Renderer::default();
    let mut performance = PerformanceMonitor::default();
    let starfield = Starfield::new();
    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);

    let mut uniforms = Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: Vector3::zero(),
        camera_target: Vector3::zero(),
        camera_up: Vector3::new(0.0, 1.0, 0.0),
        light_position: None,
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
        uniforms.light_position = world.primary_light_position();
        rocket_audio.update(
            input.thrust,
            input.braking,
            movement.is_moving(),
            movement.can_accelerate(),
            movement.speed_ratio(),
            frame_time,
        );
        let update_time = update_start.elapsed();

        starfield.draw(&mut framebuffer, &uniforms);
        world.draw(&mut renderer, &mut framebuffer, &uniforms);

        // Render the 3D object to the framebuffer
        renderer.render_lit(&mut framebuffer, &uniforms, &mesh);
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

fn run_benchmark(object_count: usize) {
    const BENCHMARK_FRAMES: usize = 240;
    const BENCHMARK_FRAME_TIME: f32 = 1.0 / 60.0;

    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);
    let mut renderer = Renderer::default();
    let mut performance = PerformanceMonitor::default();
    let starfield = Starfield::new();
    let mut world = build_benchmark_world(object_count);
    let uniforms = Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: Vector3::zero(),
        camera_target: Vector3::new(0.0, 0.0, 1.0),
        camera_up: Vector3::new(0.0, 1.0, 0.0),
        light_position: None,
    };

    eprintln!("benchmark: {object_count} objects, {BENCHMARK_FRAMES} frames");
    for _ in 0..BENCHMARK_FRAMES {
        let frame_start = Instant::now();
        renderer.begin_frame();
        framebuffer.clear();
        let update_start = Instant::now();
        world.update(&WorldUpdate {
            frame_time: BENCHMARK_FRAME_TIME,
        });
        let update_time = update_start.elapsed();
        starfield.draw(&mut framebuffer, &uniforms);
        world.draw(&mut renderer, &mut framebuffer, &uniforms);

        let mut metrics = renderer.metrics();
        metrics.update = update_time;
        metrics.frame = frame_start.elapsed();
        performance.record(metrics);
    }
}

fn benchmark_object_count(args: impl IntoIterator<Item = String>) -> Result<Option<usize>, String> {
    let args: Vec<_> = args.into_iter().collect();
    if !args.iter().any(|argument| argument == "--benchmark") {
        return Ok(None);
    }
    let Some(index) = args.iter().position(|argument| argument == "--objects") else {
        return Ok(Some(3));
    };
    let value = args
        .get(index + 1)
        .ok_or_else(|| "--objects requires a positive integer".to_owned())?;
    let count = value
        .parse::<usize>()
        .map_err(|_| "--objects requires a positive integer".to_owned())?;
    if count == 0 {
        return Err("--objects must be greater than zero".to_owned());
    }
    Ok(Some(count))
}

fn build_benchmark_world(object_count: usize) -> World {
    use celestial_shaders::CelestialKind;

    let mut world = World::new();
    let columns = ((object_count as f32 * 4.0 / 3.0).sqrt().ceil() as usize).max(1);
    let rows = object_count.div_ceil(columns);

    for index in 0..object_count {
        let column = index % columns;
        let row = index / columns;
        let horizontal = (column as f32 + 0.5) / columns as f32 * 1.3 - 0.65;
        let vertical = (row as f32 + 0.5) / rows as f32 * 0.96 - 0.48;
        let depth = 900.0 + (index % 3) as f32 * 300.0;
        let kind = match index % 3 {
            0 => CelestialKind::Star,
            1 => CelestialKind::Rocky,
            _ => CelestialKind::GasGiant,
        };
        let position = Vector3::new(horizontal * depth, vertical * depth, depth);
        if index == 0 {
            world.add(CelestialBody::new_primary_star(position));
        } else {
            world.add(CelestialBody::new(kind, position));
        }
    }

    world
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn benchmark_object_count_is_validated() {
        assert_eq!(
            benchmark_object_count(args(&["app", "--benchmark"])).unwrap(),
            Some(3)
        );
        assert_eq!(
            benchmark_object_count(args(&["app", "--benchmark", "--objects", "30"])).unwrap(),
            Some(30)
        );
        assert!(benchmark_object_count(args(&["app", "--benchmark", "--objects", "0"])).is_err());
    }
}
