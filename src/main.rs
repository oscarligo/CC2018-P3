mod celestial_shaders;
mod interaction;
mod noise;
pub mod objects;
mod performance;
mod physics;
mod rendering;
mod scene;
mod ship;

use interaction::InteractionSystem;
use objects::celestial::CelestialBody;
use performance::PerformanceMonitor;
use raylib::prelude::*;
use rendering::framebuffer::Framebuffer;
use rendering::render::Renderer;
use rendering::uniforms::Uniforms;
use scene::world::{World, WorldUpdate};
use scene::{Starfield, build_solar_system};
use ship::{InputState, RocketAudio, ShipMovement};
use ship::{
    alien_audio::AlienAudio,
    commands::{Command, CommandReader},
    menu::{TravelMenu, choose_vehicle},
    radio::{CarRadio, PANEL_HEIGHT},
    vehicle::{Vehicle, VehicleKind},
};
use std::thread;
use std::time::{Duration, Instant};

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const WINDOW_TITLE: &str = "Software Renderer";
const FRAME_DELAY: Duration = Duration::from_millis(16);

fn main() {
    if std::env::args().any(|arg| arg == "--preview-persona3") {
        if let Err(error) = scene::planets::exploration::preview() {
            eprintln!("No se pudo capturar los planetas: {error}");
            std::process::exit(2);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--benchmark-radio") {
        ship::radio::benchmark();
        return;
    }
    if std::env::args().any(|arg| arg == "--preview-radio") {
        ship::radio::preview();
        return;
    }
    if std::env::args().any(|arg| arg == "--preview-dyson") {
        preview_dyson();
        return;
    }
    if std::env::args().any(|arg| arg == "--preview-alien") {
        preview_alien();
        return;
    }
    if std::env::args().any(|arg| arg == "--preview-ships") {
        preview_ships();
        return;
    }
    if std::env::args().any(|arg| arg == "--benchmark-coruscant") {
        let captures = std::env::args().any(|arg| arg == "--capturas");
        if let Err(error) = scene::planets::benchmark_coruscant(captures) {
            eprintln!("No se pudo medir Coruscant: {error}");
            std::process::exit(2);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--benchmark-kashyyyk") {
        let captures = std::env::args().any(|arg| arg == "--capturas");
        if let Err(error) = scene::planets::benchmark_kashyyyk(captures) {
            eprintln!("No se pudo medir Kashyyyk: {error}");
            std::process::exit(2);
        }
        return;
    }
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
    window.set_exit_key(None);
    let audio = RaylibAudio::init_audio_device().expect("Failed to initialize audio");
    let mut rocket_audio = RocketAudio::new(&audio);
    let mut alien_audio = AlienAudio::new(&audio);

    thread::scope(|scope| {
        let mut renderer = Renderer::default();
        let mut performance = PerformanceMonitor::default();
        let mut interaction = InteractionSystem::default();
        let starfield = Starfield::new();
        let mut framebuffer =
            Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);
        let car = Vehicle::car();
        let car_radius = car.radius;
        let mut vehicles = [car, Vehicle::saucer(car_radius)];
        let Some(selected) = choose_vehicle(
            &mut window,
            &thread,
            &mut framebuffer,
            &mut renderer,
            &mut vehicles,
        ) else {
            return;
        };
        let mut vehicle = if selected == 0 {
            let [car, _] = vehicles;
            car
        } else {
            let [_, saucer] = vehicles;
            saucer
        };
        let mut travel = TravelMenu::default();
        let mut commands = CommandReader::default();
        let mut radio = (vehicle.kind == VehicleKind::Car).then(|| CarRadio::new(&audio, scope));
        let mut inspection_delay: Option<f32> = None;
        let mut jump_flash = 0.0_f32;
        let mut elapsed = 0.0;

        let mut uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::zero(),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        let mut movement = ShipMovement::new(&vehicle.mesh, rocket_audio.moving_duration());
        movement.set_muzzle(vehicle.muzzle);
        movement.configure_vehicle(
            car_radius,
            rocket_audio.moving_duration(),
            vehicle.kind == VehicleKind::Saucer,
        );
        let mut world = build_solar_system();
        movement.update(&InputState::read(&window), 0.0, &mut uniforms, &world);

        while !window.window_should_close() {
            let frame_start = Instant::now();
            renderer.begin_frame();
            framebuffer.clear();
            let frame_time = window.get_frame_time().min(0.1);
            elapsed += frame_time;
            jump_flash = (jump_flash - frame_time * 2.0).max(0.0);
            let update_start = Instant::now();
            world.update(&WorldUpdate { frame_time });
            let targets = world.travel_targets();
            let was_open = travel.open;
            if !was_open && window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                break;
            }
            commands.tick(frame_time);
            let mut open_jump = false;
            let mut radio_toggled = false;
            while let Some(ch) = window.get_char_pressed() {
                if !travel.open
                    && let Some(command) = commands.push(ch)
                {
                    match command {
                        Command::Radio if vehicle.kind == VehicleKind::Car => {
                            if let Some(radio) = &mut radio {
                                radio.toggle();
                                radio_toggled = true;
                            }
                        }
                        Command::Jump if vehicle.kind == VehicleKind::Saucer => open_jump = true,
                        Command::Sneeze | Command::Chase if vehicle.kind == VehicleKind::Saucer => {
                            movement.command(command)
                        }
                        _ => {}
                    }
                }
            }
            let radio_mouse = radio.as_mut().is_some_and(|r| r.update(&window));
            let jump = travel.update(&mut window, vehicle.kind, open_jump, &targets);
            if let Some(index) = jump
                && movement.teleport(&targets[index], &world)
            {
                jump_flash = 1.0;
                alien_audio.stop();
            }
            let blocked = was_open || travel.open || jump.is_some();
            let mut input = if blocked {
                InputState::default()
            } else {
                InputState::for_vehicle(&window, vehicle.kind == VehicleKind::Saucer)
            };
            if vehicle.kind == VehicleKind::Car {
                // Give the second letter of "fm" a short grace period before treating F as inspection.
                if radio_toggled {
                    inspection_delay = None;
                    input.interact_pressed = false;
                } else {
                    if input.interact_pressed {
                        inspection_delay = Some(0.28);
                    }
                    input.interact_pressed = false;
                    if let Some(delay) = &mut inspection_delay {
                        *delay -= frame_time;
                        if *delay <= 0.0 {
                            input.interact_pressed = true;
                            inspection_delay = None;
                        }
                    }
                }
            }
            if radio_mouse {
                input.orbiting = false;
                input.zoom_delta = 0.0;
                input.firing = false;
            }
            movement.update(
                &input,
                if travel.open { 0.0 } else { frame_time },
                &mut uniforms,
                &world,
            );
            let radio_active = radio.as_ref().is_some_and(CarRadio::is_playing);
            let effects_gain = rocket_audio.set_radio_active(radio_active, frame_time);
            alien_audio.set_impact_gain(effects_gain);
            alien_audio.set_paused(travel.open);
            if input.braking {
                alien_audio.stop();
            }
            if movement.take_impact() {
                alien_audio.stop();
                alien_audio.play(&[ship::alien::Cue::Impact]);
            }
            if !travel.open
                && let Some(alien) = &mut movement.alien
            {
                alien_audio.play(&alien.take_cues());
            }
            uniforms.light_position = world.primary_light_position();
            interaction.update(&mut world, &uniforms, input.interact_pressed, frame_time);
            rocket_audio.update(
                input.thrust,
                input.braking,
                movement.is_moving() && !travel.open,
                movement.can_accelerate(),
                movement.speed_ratio(),
                frame_time,
            );
            let update_time = update_start.elapsed();

            starfield.draw(&mut framebuffer, &uniforms);
            world.draw(&mut renderer, &mut framebuffer, &uniforms);

            vehicle.draw(&mut renderer, &mut framebuffer, &uniforms, elapsed);
            movement.draw_trail(&mut framebuffer, &uniforms);
            movement.draw_exhaust(&mut framebuffer, &uniforms);

            let radio_visible = radio.as_ref().is_some_and(|r| r.visible());
            framebuffer.present_with_overlay_bottom(
                &mut window,
                &thread,
                if travel.open {
                    None
                } else {
                    interaction.overlay()
                },
                if radio_visible { PANEL_HEIGHT + 32 } else { 14 },
                |d| {
                    travel.draw(d, vehicle.kind, &targets, jump_flash);
                    if !travel.open {
                        movement.draw_navigation(d);
                    }
                    if !travel.open
                        && let Some(alien) = &movement.alien
                    {
                        alien.draw_hud(d);
                    }
                    if let Some(radio) = &radio {
                        radio.draw(d);
                    }
                },
            );
            let mut metrics = renderer.metrics();
            metrics.update = update_time;
            metrics.frame = frame_start.elapsed();
            performance.record(metrics);
            thread::sleep(FRAME_DELAY);
        }
    });
}

fn preview_ships() {
    let car = Vehicle::car();
    let radius = car.radius;
    let mut vehicles = [car, Vehicle::saucer(radius)];
    std::fs::create_dir_all("target/previews").expect("crear directorio de capturas");
    let mut renderer = Renderer::default();
    for vehicle in &mut vehicles {
        let mut fb = Framebuffer::new(800, 600, Color::new(9, 16, 26, 255));
        let u = vehicle.preview_uniforms(0.25);
        renderer.begin_frame();
        vehicle.draw(&mut renderer, &mut fb, &u, 0.0);
        let name = if vehicle.kind == VehicleKind::Car {
            "car"
        } else {
            "ufo"
        };
        fb.save_png(format!("target/previews/{name}.png"))
            .expect("guardar captura");
        println!("target/previews/{name}.png");
    }
}

fn preview_alien() {
    use ship::alien::Cue;
    let car = Vehicle::car();
    let mut vehicle = Vehicle::saucer(car.radius);
    let mut world = build_solar_system();
    world.update(&WorldUpdate { frame_time: 0.0 });
    let stars = Starfield::new();
    std::fs::create_dir_all("target/previews").expect("crear capturas");
    for (name, command, frames) in [
        ("achu-charge", Command::Sneeze, 45),
        ("achu-burst", Command::Sneeze, 68),
        ("mish-laser", Command::Chase, 10),
        ("mish-orbit", Command::Chase, 90),
    ] {
        let mut movement = ShipMovement::new(&vehicle.mesh, 2.0);
        movement.set_muzzle(vehicle.muzzle);
        movement.configure_vehicle(car.radius, 2.0, true);
        let mut u = vehicle.preview_uniforms(0.0);
        movement.update(&InputState::default(), 0., &mut u, &world);
        movement.command(command);
        for frame in 0..frames {
            let input = InputState {
                mouse_position: Vector2::new(620., 230.),
                firing: name == "mish-laser" && frame == frames - 1,
                ..Default::default()
            };
            movement.update(&input, 1. / 60., &mut u, &world);
        }
        let mut fb = Framebuffer::new(800, 600, Color::new(5, 10, 20, 255));
        let mut renderer = Renderer::default();
        stars.draw(&mut fb, &u);
        world.draw(&mut renderer, &mut fb, &u);
        vehicle.draw(&mut renderer, &mut fb, &u, frames as f32 / 60.);
        movement.draw_trail(&mut fb, &u);
        movement.draw_exhaust(&mut fb, &u);
        fb.save_png(format!("target/previews/{name}.png"))
            .expect("guardar captura");
    }
    for (name, cue) in [
        ("charge", Cue::Charge),
        ("sneeze-stereo", Cue::Sneeze),
        ("laser-stereo", Cue::Laser),
        ("mish", Cue::Chase),
        ("impact", Cue::Impact),
    ] {
        std::fs::write(
            format!("target/previews/{name}.wav"),
            ship::alien_audio::synthesize(cue),
        )
        .expect("guardar audio");
    }
    println!("Capturas y efectos estereo: target/previews/");
    preview_impacts();
}

fn preview_impacts() {
    use celestial_shaders::CelestialKind;
    let car = Vehicle::car();
    let radius = car.radius;
    for mut vehicle in [car, Vehicle::saucer(radius)] {
        let mut world = World::new();
        world.add(
            CelestialBody::new(CelestialKind::Rocky, Vector3::new(0., 0., 150.)).with_scale(55.0),
        );
        let mut movement = ShipMovement::new(&vehicle.mesh, 0.3);
        movement.set_muzzle(vehicle.muzzle);
        movement.configure_vehicle(radius, 0.3, vehicle.kind == VehicleKind::Saucer);
        let mut u = vehicle.preview_uniforms(0.0);
        for _ in 0..1200 {
            movement.update(
                &InputState {
                    thrust: true,
                    boost: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                &mut u,
                &world,
            );
            if movement.take_impact() {
                break;
            }
        }
        for _ in 0..3 {
            movement.update(&InputState::default(), 1.0 / 60.0, &mut u, &world);
        }
        u.light_position = Some(Vector3::new(-200., 300., -100.));
        let mut fb = Framebuffer::new(800, 600, Color::new(5, 10, 20, 255));
        let mut renderer = Renderer::default();
        world.draw(&mut renderer, &mut fb, &u);
        vehicle.draw(&mut renderer, &mut fb, &u, 0.0);
        movement.draw_exhaust(&mut fb, &u);
        let name = if vehicle.kind == VehicleKind::Car {
            "impact-car"
        } else {
            "impact-ufo"
        };
        fb.save_png(format!("target/previews/{name}.png"))
            .expect("guardar choque");
    }
}

fn preview_dyson() {
    std::fs::create_dir_all("target/previews").expect("crear capturas");
    for (label, time, offset) in [
        ("dyson-00", 0.0, Vector3::new(120., 230., -1300.)),
        ("dyson-12", 12.0, Vector3::new(120., 230., -1300.)),
        ("dyson-28", 28.0, Vector3::new(120., 230., -1300.)),
        ("dyson-interior", 8.0, Vector3::new(120., 80., -480.)),
    ] {
        let mut world = build_solar_system();
        world.update(&WorldUpdate { frame_time: time });
        let sun = world.primary_light_position().unwrap();
        let mut u = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: sun + offset,
            camera_target: sun,
            camera_up: Vector3::new(0., 1., 0.),
            light_position: Some(sun),
        };
        if label == "dyson-interior" {
            u = objects::dyson::interior_view(sun, world.primary_light_bounds().unwrap().1, time);
        }
        let mut fb = Framebuffer::new(1200, 800, Color::new(3, 7, 15, 255));
        Starfield::new().draw(&mut fb, &u);
        world.draw(&mut Renderer::default(), &mut fb, &u);
        fb.save_png(format!("target/previews/{label}.png"))
            .expect("guardar Dyson");
    }
    println!("Capturas Dyson: target/previews/dyson-*.png");
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
    use objects::celestial::BodyShape;

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
            let body = CelestialBody::new(kind, position);
            world.add(match kind {
                CelestialKind::Rocky if index % 6 == 4 => body.with_shape(BodyShape::Asteroid),
                CelestialKind::Rocky => body.with_shape(BodyShape::Rocky),
                CelestialKind::GasGiant => body.with_shape(BodyShape::Oblate),
                CelestialKind::Star => body,
            });
        }
    }

    world
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;

    #[test]
    fn travel_catalog_stays_stable_when_rendering_reorders_the_world() {
        let mut world = build_solar_system();
        world.update(&WorldUpdate { frame_time: 0.0 });
        let before = world.travel_targets();
        assert_eq!(before.len(), 18);
        assert!(before.iter().any(|t| t.name == "Kashyyyk"));
        assert!(before.iter().any(|t| t.name == "Coruscant"));
        let uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::new(0., 0., 1.),
            camera_up: Vector3::new(0., 1., 0.),
            light_position: None,
        };
        world.draw(
            &mut Renderer::default(),
            &mut Framebuffer::new(32, 24, Color::BLACK),
            &uniforms,
        );
        world.update(&WorldUpdate { frame_time: 10.0 });
        let after = world.travel_targets();
        assert_eq!(
            before.iter().map(|t| t.name).collect::<Vec<_>>(),
            after.iter().map(|t| t.name).collect::<Vec<_>>()
        );
        assert!(
            before
                .iter()
                .zip(&after)
                .any(|(a, b)| (a.position - b.position).length() > 1.0)
        );
    }

    #[test]
    #[ignore = "Genera capturas de revision en target/previews sin abrir ventanas"]
    fn capture_vehicle_previews() {
        preview_ships();
        let ufo = image::open("target/previews/ufo.png").unwrap().to_rgb8();
        let green = ufo
            .pixels()
            .filter(|p| p[1] > p[0].saturating_add(30) && p[1] > p[2].saturating_add(20))
            .count();
        assert!(
            green > 1000,
            "El piloto y las luces verdes deben ser visibles"
        );
    }

    #[test]
    #[ignore = "Genera capturas de efectos y muestras WAV sin abrir ventanas"]
    fn capture_alien_effects() {
        preview_alien();
        let burst = image::open("target/previews/achu-burst.png")
            .unwrap()
            .to_rgb8();
        let glowing = burst
            .pixels()
            .filter(|p| p[1] > 150 && p[1] > p[0].saturating_add(35))
            .count();
        assert!(
            glowing > 3000,
            "La nube expansiva debe iluminar una zona amplia: {glowing}"
        );
    }

    #[test]
    #[ignore = "Capturas de revision de ambos aros Dyson"]
    fn capture_dyson() {
        preview_dyson();
    }

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
