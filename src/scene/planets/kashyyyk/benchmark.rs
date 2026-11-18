//! Comparación CPU reproducible. No abre ventana ni usa dibujo 3D de raylib.
use super::{INITIAL_OFFSET, ORBITAL_SPEED, RADIUS, ROTATION_SPEED, register_with_lod, village};
use crate::objects::celestial::CelestialBody;
use crate::rendering::{framebuffer::Framebuffer, render::Renderer, uniforms::Uniforms};
use crate::scene::world::{World, WorldUpdate};
use raylib::prelude::{Color, Matrix, Quaternion, Vector3};
use std::time::Instant;

const FRAMES: usize = 30;
const DT: f32 = 1.0 / 60.0;

fn rotate(rotation: Quaternion, point: Vector3) -> Vector3 {
    let m = rotation.to_matrix();
    Vector3::new(
        m.m0 * point.x + m.m4 * point.y + m.m8 * point.z,
        m.m1 * point.x + m.m5 * point.y + m.m9 * point.z,
        m.m2 * point.x + m.m6 * point.y + m.m10 * point.z,
    )
}

fn camera(name: &str, sun: Vector3, time: f32) -> Uniforms {
    let up = Vector3::new(0.0, 1.0, 0.0);
    let center = sun
        + rotate(
            Quaternion::from_axis_angle(up, ORBITAL_SPEED * time),
            INITIAL_OFFSET,
        );
    let radial = rotate(
        Quaternion::from_axis_angle(up, ROTATION_SPEED * time),
        village::site_direction(),
    );
    let tangent = radial.cross(up).normalize();
    let (position, target, camera_up) = match name {
        "espacio" => (center + radial * 1600.0, center, up),
        "acercamiento" => (center + radial * 360.0, center, up),
        "aldea" => {
            let target = center + radial * (RADIUS + 12.0);
            (target + radial * 28.0 + tangent * 50.0, target, radial)
        }
        "superficie" => {
            let target = center + radial * (RADIUS + 2.0);
            (target + radial * 2.0 + tangent * 8.0, target, radial)
        }
        _ => unreachable!(),
    };
    Uniforms {
        model_matrix: Matrix::identity(),
        camera_pos: position,
        camera_target: target,
        camera_up,
        light_position: Some(sun),
    }
}

pub(super) fn run(captures: bool) -> Result<(), Box<dyn std::error::Error>> {
    let sun = Vector3::new(0.0, 0.0, 1500.0);
    if captures {
        std::fs::create_dir_all("target/kashyyyk")?;
    }
    println!(
        "camara,culling,lod,triangulos_media,objetos_descartados_media,fragmentos_media,ms_media,ms_p95"
    );
    for name in ["espacio", "acercamiento", "aldea", "superficie"] {
        for (culling, lod) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut world = World::new();
            // Una escena de medición con un solo sol y la implementación real de Kashyyyk.
            world.add(CelestialBody::new_primary_star(sun).with_scale(363.0));
            register_with_lod(&mut world, lod);
            let mut renderer = Renderer::default();
            let mut framebuffer = Framebuffer::new(800, 600, Color::BLACK);
            world.draw_with_culling(
                &mut renderer,
                &mut framebuffer,
                &camera(name, sun, 0.0),
                culling,
            );
            for _ in 0..90 {
                world.update(&WorldUpdate { frame_time: DT });
            }
            let mut times = Vec::with_capacity(FRAMES);
            let mut triangles = 0;
            let mut culled = 0;
            let mut fragments = 0;
            let mut time = 90.0 * DT;
            for _ in 0..FRAMES {
                let start = Instant::now();
                world.update(&WorldUpdate { frame_time: DT });
                time += DT;
                renderer.begin_frame();
                framebuffer.clear();
                world.draw_with_culling(
                    &mut renderer,
                    &mut framebuffer,
                    &camera(name, sun, time),
                    culling,
                );
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                let metrics = renderer.metrics();
                triangles += metrics.triangles;
                culled += metrics.culled_objects;
                fragments += metrics.shaded_fragments;
            }
            let mean = times.iter().sum::<f64>() / FRAMES as f64;
            times.sort_by(f64::total_cmp);
            let p95 = times[(FRAMES as f64 * 0.95).ceil() as usize - 1];
            println!(
                "{name},{culling},{lod},{:.0},{:.1},{:.0},{mean:.2},{p95:.2}",
                triangles as f64 / FRAMES as f64,
                culled as f64 / FRAMES as f64,
                fragments as f64 / FRAMES as f64
            );
            if captures && culling == lod {
                let label = if lod { "optimizado" } else { "completo" };
                framebuffer.save_png(format!("target/kashyyyk/{name}-{label}.png"))?;
            }
        }
    }
    Ok(())
}
