use super::{INITIAL_OFFSET, ORBITAL_SPEED, RADIUS, ROTATION_SPEED, district, registrar};
use crate::objects::celestial::CelestialBody;
use crate::rendering::{framebuffer::Framebuffer, render::Renderer, uniforms::Uniforms};
use crate::scene::{
    Starfield,
    world::{World, WorldUpdate},
};
use raylib::prelude::{Color, Matrix, Vector3};
use std::time::Instant;

const FRAMES: usize = 30;
const DT: f32 = 1.0 / 60.0;
const CAPTURE_DIR: &str = "target/coruscant";

pub(in crate::scene::planets) fn run(captures: bool) -> Result<(), Box<dyn std::error::Error>> {
    if captures {
        std::fs::create_dir_all(CAPTURE_DIR)?;
    }
    println!("fase,triangulos_media,ms_media,ms_p95");
    let sun = Vector3::new(0.0, 0.0, 1500.0);
    let stars = Starfield::new();
    let mut frame = Framebuffer::new(800, 600, Color::BLACK);
    let mut renderer = Renderer::default();
    let mut world = World::new();
    world.add(CelestialBody::new_primary_star(sun).with_scale(363.0));
    registrar(&mut world);
    let axis = Vector3::new(0.0, 1.0, 0.0);
    let site = district::site();
    let mut time = 34.0;
    world.update(&WorldUpdate { frame_time: time });

    for (phase, altitude, lateral) in [
        ("orbita", 680.0, 40.0),
        ("acercamiento", 170.0, 52.0),
        ("sobrevuelo", 24.0, 42.0),
        ("salida", 510.0, -75.0),
    ] {
        let mut times = Vec::with_capacity(FRAMES);
        let mut triangles = 0;
        // Asienta las transiciones de la esfera antes de cronometrar.
        for frame_index in 0..90 + FRAMES {
            let start = Instant::now();
            time += DT;
            world.update(&WorldUpdate { frame_time: DT });
            let center = sun + INITIAL_OFFSET.rotate_by_axis_angle(axis, ORBITAL_SPEED * time);
            let up = site
                .center
                .rotate_by_axis_angle(axis, ROTATION_SPEED * time);
            let east = site.east.rotate_by_axis_angle(axis, ROTATION_SPEED * time);
            let ground = center + up * RADIUS;
            let camera = Uniforms {
                model_matrix: Matrix::identity(),
                camera_pos: ground + up * altitude + east * lateral,
                camera_target: ground + up * 5.0,
                camera_up: if phase == "sobrevuelo" { up } else { axis },
                light_position: Some(sun),
            };
            renderer.begin_frame();
            frame.clear();
            stars.draw(&mut frame, &camera);
            world.draw(&mut renderer, &mut frame, &camera);
            if frame_index >= 90 {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                triangles += renderer.metrics().triangles;
            }
        }
        let mean = times.iter().sum::<f64>() / FRAMES as f64;
        times.sort_unstable_by(f64::total_cmp);
        let p95 = times[(FRAMES as f64 * 0.95).ceil() as usize - 1];
        println!(
            "{phase},{:.0},{mean:.2},{p95:.2}",
            triangles as f64 / FRAMES as f64
        );
        if captures {
            frame.save_png(format!("{CAPTURE_DIR}/{phase}.png"))?;
        }
    }
    Ok(())
}
