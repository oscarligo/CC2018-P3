mod fragment;
mod framebuffer;
mod input;
mod line;
mod load_objects;
mod rasterizer;
mod render;
mod rocket_audio;
mod shaders;
mod ship_movement;
mod triangle;
mod uniforms;
mod vertex;

use framebuffer::Framebuffer;
use input::InputState;
use rasterizer::{rasterize, TriangleMode};
use raylib::prelude::*;
use render::render;
use rocket_audio::RocketAudio;
use ship_movement::ShipMovement;
use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};
use uniforms::Uniforms;

const WINDOW_WIDTH: i32 = 600;
const WINDOW_HEIGHT: i32 = 650;
const WINDOW_TITLE: &str = "Software Renderer";
const MODEL_PATH: &str = "assets/red_car.obj";
const FRAME_DELAY: Duration = Duration::from_millis(16);
const BENCHMARK_REPETITIONS: usize = 7;
const BENCHMARK_ITERATIONS: usize = 100;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments
        .iter()
        .any(|argument| argument == "--benchmark-triangles")
    {
        benchmark_triangles();
        return;
    }
    let triangle_mode = if arguments.iter().any(|argument| argument == "--wireframe") {
        TriangleMode::Wireframe
    } else if arguments.iter().any(|argument| argument == "--scanline") {
        TriangleMode::FilledScanline
    } else {
        TriangleMode::FilledBarycentric
    };

    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(WINDOW_TITLE)
        .build();
    let audio = RaylibAudio::init_audio_device().expect("Failed to initialize audio");
    let mut rocket_audio = RocketAudio::new(&audio);

    // Load the 3D object from an .obj file
    let mesh = load_objects::load_obj(MODEL_PATH).expect("Failed to load .obj file");
    let mut transformed_vertices = Vec::with_capacity(mesh.vertices.len());
    let mut fragments = Vec::new();
    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);

    let mut uniforms = Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: Vector3::zero(),
        camera_target: Vector3::zero(),
    };
    let mut movement = ShipMovement::new(&mesh, rocket_audio.moving_duration());
    movement.update(&InputState::read(&window), 0.0, &mut uniforms);

    while !window.window_should_close() {
        framebuffer.clear();
        let input = InputState::read(&window);
        let frame_time = window.get_frame_time();
        movement.update(&input, frame_time, &mut uniforms);
        rocket_audio.update(
            input.thrust,
            input.braking,
            movement.is_moving(),
            movement.can_accelerate(),
            movement.speed_ratio(),
            frame_time,
        );

        // Render the 3D object to the framebuffer
        render(
            &mut framebuffer,
            &uniforms,
            &mesh,
            triangle_mode,
            &mut transformed_vertices,
            &mut fragments,
        );
        movement.draw_trail(&mut framebuffer, &uniforms);
        movement.draw_exhaust(&mut framebuffer, &uniforms);

        framebuffer.swap_buffers(&mut window, &thread);
        thread::sleep(FRAME_DELAY);
    }
}

fn benchmark_triangles() {
    let mut vertices = [
        vertex::Vertex::new(Vector3::new(20.0, 20.0, 1.0)),
        vertex::Vertex::new(Vector3::new(580.0, 80.0, 1.0)),
        vertex::Vertex::new(Vector3::new(250.0, 620.0, 1.0)),
    ];
    for vertex in &mut vertices {
        vertex.screen_position = Some(Vector2::new(vertex.position.x, vertex.position.y));
        vertex.camera_depth = 1.0;
    }
    let triangles = vec![vertices];

    for mode in [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ] {
        let mut measurements = Vec::with_capacity(BENCHMARK_REPETITIONS);
        let mut fragment_count = 0;
        for _ in 0..BENCHMARK_REPETITIONS {
            let start = Instant::now();
            for _ in 0..BENCHMARK_ITERATIONS {
                let fragments = rasterize(black_box(&triangles), mode);
                fragment_count = fragments.len();
                black_box(fragments);
            }
            measurements.push(start.elapsed());
        }
        measurements.sort();
        println!(
            "{mode:?}: mediana de 100 ejecuciones = {:?}, fragmentos = {fragment_count}",
            measurements[measurements.len() / 2],
        );
    }
}
