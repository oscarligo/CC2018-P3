mod fragment;
mod framebuffer;
mod line;
mod load_objects;
mod rasterizer;
mod render;
mod shaders;
mod triangle;
mod uniforms;
mod vertex;

use framebuffer::Framebuffer;
use rasterizer::{rasterize, TriangleMode};
use raylib::prelude::*;
use render::render;
use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};
use uniforms::Uniforms;

fn main() {
    const WINDOW_WIDTH: i32 = 600;
    const WINDOW_HEIGHT: i32 = 650;

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
        .title("Conway's Game of Life")
        .build();

    // Load the 3D object from an .obj file
    let mesh = load_objects::load_obj("assets/ship.obj").expect("Failed to load .obj file");
    let mut transformed_vertices = Vec::with_capacity(mesh.vertices.len());
    let mut fragments = Vec::new();
    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);

    // moves the camera position to view the object from a different angle
    let camera_pos = Vector3::new(10.0, 2.0, -2.0);
    let mut uniforms = Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos,
    };
    let mut rotation_x = 0.0;
    let mut rotation_y = 0.0;

    while !window.window_should_close() {
        framebuffer.clear();
        let rotation_step = 1.5 * window.get_frame_time();

        // Handle camera movement based on user input
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_W) {
            // Move camera forward
            uniforms.camera_pos += Vector3::new(0.0, 0.1, 0.0);
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_S) {
            // Move camera backward
            uniforms.camera_pos -= Vector3::new(0.0, 0.1, 0.0);
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_A) {
            // Move camera left
            uniforms.camera_pos -= Vector3::new(0.1, 0.0, 0.0);
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_D) {
            // Move camera right
            uniforms.camera_pos += Vector3::new(0.1, 0.0, 0.0);
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_LEFT) {
            rotation_y -= rotation_step;
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_RIGHT) {
            rotation_y += rotation_step;
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_UP) {
            rotation_x -= rotation_step;
        }
        if window.is_key_down(raylib::consts::KeyboardKey::KEY_DOWN) {
            rotation_x += rotation_step;
        }
        uniforms.model_matrix = Matrix::rotate_xyz(Vector3::new(rotation_x, rotation_y, 0.0));

        // Render the 3D object to the framebuffer
        render(
            &mut framebuffer,
            &uniforms,
            &mesh,
            triangle_mode,
            &mut transformed_vertices,
            &mut fragments,
        );

        framebuffer.swap_buffers(&mut window, &thread);
        thread::sleep(Duration::from_millis(16));
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
    }
    let triangles = vec![vertices];

    for mode in [
        TriangleMode::FilledBarycentric,
        TriangleMode::FilledScanline,
    ] {
        let mut measurements = Vec::with_capacity(7);
        let mut fragment_count = 0;
        for _ in 0..7 {
            let start = Instant::now();
            for _ in 0..100 {
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
