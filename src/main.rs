mod celestial_shaders;
mod display;
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
    ambient_audio::AmbientAudio,
    car_weapons::CarWeapons,
    commands::{Command, CommandReader},
    doomsday::{Controls as WeaponControls, Doomsday, DoomsdayAudio},
    menu::{TravelMenu, choose_vehicle, draw_hangar_return, hangar_requested},
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
    if std::env::args().any(|arg| arg == "--preview-doomsday") {
        ship::doomsday::preview();
        return;
    }
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

    thread::scope(|scope| {
        let mut renderer = Renderer::default();
        let starfield = Starfield::new();
        let mut framebuffer =
            Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, Color::BLACK);
        let mut world = build_solar_system();
        let total_planets = world.travel_targets().len();
        let mut selected = 0;
        let mut solar_return = false;
        'hangar: loop {
            let car = Vehicle::car();
            let car_radius = car.radius;
            let mut vehicles = [
                car,
                Vehicle::saucer(car_radius),
                Vehicle::destroyer(car_radius),
            ];
            let Some(choice) = choose_vehicle(
                &mut window,
                &thread,
                &mut framebuffer,
                &mut renderer,
                &mut vehicles,
                selected,
                solar_return,
            ) else {
                return;
            };
            selected = choice;
            solar_return = false;
            // Each departure owns its audio and transient state; returning drops them all.
            let mut rocket_audio = RocketAudio::new(&audio);
            let mut alien_audio = AlienAudio::new(&audio);
            let mut ambient_audio = AmbientAudio::new(&audio);
            let mut performance = PerformanceMonitor::default();
            let mut interaction = InteractionSystem::default();
            while window.get_char_pressed().is_some() {}
            while window.get_key_pressed().is_some() {}
            let mut vehicle = vehicles
                .into_iter()
                .nth(selected)
                .expect("nave seleccionada");
            let is_destroyer = vehicle.kind == VehicleKind::Destroyer;
            let mut weapon = Doomsday::default();
            let mut planet_finder = ship::planet_finder::PlanetFinder::default();
            let mut car_weapons =
                (vehicle.kind == VehicleKind::Car).then(|| CarWeapons::new(vehicle.radius));
            let car_muzzles = vehicle.car_laser_muzzles();
            let mut weapon_audio = is_destroyer.then(|| DoomsdayAudio::new(&audio));
            let mut travel = TravelMenu::default();
            let mut commands = CommandReader::default();
            let mut radio =
                (vehicle.kind == VehicleKind::Car).then(|| CarRadio::new(&audio, scope));
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
                if is_destroyer {
                    vehicle.radius
                } else {
                    car_radius
                },
                rocket_audio.moving_duration(),
                vehicle.kind == VehicleKind::Saucer,
            );
            if vehicle.kind == VehicleKind::Car {
                movement.configure_unbounded_car_thrust();
            }
            movement.update(&InputState::read(&window), 0.0, &mut uniforms, &world);

            let intro_destination = movement.position();
            let mut intro_time = (vehicle.kind == VehicleKind::Car).then_some(-6.0_f32);

            while !window.window_should_close() {
                display::update(&mut window);
                if hangar_requested(&window) {
                    continue 'hangar;
                }
                let frame_start = Instant::now();
                renderer.begin_frame();
                framebuffer.clear();
                let frame_time = window.get_frame_time().min(0.1);
                elapsed += frame_time;
                jump_flash = (jump_flash - frame_time * 2.0).max(0.0);
                let update_start = Instant::now();
                world.update(&WorldUpdate { frame_time });
                let targets = world.travel_targets();
                let intro_active = intro_time.is_some();
                if intro_active && let Some(radio) = &mut radio {
                    radio.poll_intro();
                    if let Some(time) = &mut intro_time {
                        if *time < 0.0 {
                            *time = (*time + window.get_frame_time()).min(0.0);
                            if *time == 0.0 {
                                radio.start_intro();
                            }
                        } else if radio.intro_ready() {
                            *time = (*time + window.get_frame_time()).min(27.0);
                        }
                    }
                }
                let intro_fade = intro_time.map_or(0, ship::intro_entry_fade);
                let was_open = travel.open;
                let was_weapon_busy = weapon.busy();
                if !was_open && !was_weapon_busy && window.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                    return;
                }
                commands.tick(frame_time);
                let mut open_jump = false;
                let mut radio_toggled = false;
                while let Some(ch) = window.get_char_pressed() {
                    if !intro_active
                        && !travel.open
                        && !weapon.busy()
                        && let Some(command) = commands.push(ch)
                    {
                        match command {
                            Command::Radio if vehicle.kind == VehicleKind::Car => {
                                if let Some(radio) = &mut radio {
                                    radio.toggle();
                                    radio_toggled = true;
                                }
                            }
                            Command::Jump if vehicle.kind != VehicleKind::Car => open_jump = true,
                            Command::Sneeze | Command::Chase
                                if vehicle.kind == VehicleKind::Saucer =>
                            {
                                movement.command(command)
                            }
                            _ => {}
                        }
                    }
                }
                let radio_mouse =
                    !intro_active && radio.as_mut().is_some_and(|r| r.update(&window));
                let jump = travel.update(&mut window, vehicle.kind, open_jump, &targets);
                if let Some(index) = jump
                    && movement.teleport(&targets[index], &world)
                {
                    jump_flash = 1.0;
                    alien_audio.stop();
                }
                let previous_target = weapon.target;
                let aim = if is_destroyer {
                    let m = uniforms.model_matrix;
                    world.weapon_target(movement.position(), Vector3::new(m.m8, m.m9, m.m10))
                } else {
                    None
                };
                let launch_origin = vehicle.muzzle.map_or(movement.position(), |p| {
                    model_point(&uniforms.model_matrix, p)
                });
                if is_destroyer && !was_open && !travel.open && jump.is_none() {
                    weapon.update(
                        window.get_frame_time(),
                        &WeaponControls::read(&mut window),
                        aim,
                        &mut world,
                        launch_origin,
                    );
                    if !was_weapon_busy
                        && !weapon.busy()
                        && window.is_key_pressed(KeyboardKey::KEY_F8)
                    {
                        world.restore_planets();
                        weapon.restored();
                        commands.clear();
                    }
                }
                let weapon_blocked = was_weapon_busy || weapon.busy();
                let blocked =
                    intro_active || was_open || travel.open || jump.is_some() || weapon_blocked;
                if vehicle.kind == VehicleKind::Car
                    && !blocked
                    && window.is_key_pressed(KeyboardKey::KEY_N)
                {
                    planet_finder.toggle(&targets);
                }
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
                    input.firing_held = false;
                }
                if let Some(time) = intro_time {
                    movement.intro_frame(
                        time.max(-3.0),
                        frame_time,
                        intro_destination,
                        &mut uniforms,
                    );
                    if time >= 27.0 {
                        intro_time = None;
                    }
                } else if weapon_blocked && let Some(target) = weapon.target.or(previous_target) {
                    movement.weapon_station(
                        &previous_target.unwrap_or(target),
                        &target,
                        frame_time,
                        &mut uniforms,
                        &mut world,
                    );
                } else {
                    movement.update(
                        &input,
                        if travel.open { 0.0 } else { frame_time },
                        &mut uniforms,
                        &world,
                    );
                }
                if let Some(car_weapons) = &mut car_weapons {
                    let matrix = uniforms.model_matrix;
                    let muzzles = car_muzzles.map(|point| model_point(&matrix, point));
                    let forward = Vector3::new(matrix.m8, matrix.m9, matrix.m10).normalize();
                    if car_weapons.update(
                        frame_time,
                        input.firing,
                        input.firing_held,
                        muzzles,
                        forward,
                        &mut world,
                    ) {
                        alien_audio.play(&[ship::alien::Cue::Laser]);
                    }
                }
                vehicle.weapon_charge = weapon.charge;
                vehicle.weapon_open = weapon.bay_open;
                vehicle.bomb_visible = weapon.bomb_visible();
                let weapon_cues = weapon.take_cues();
                if let Some(audio) = &mut weapon_audio {
                    audio.update(&weapon, &weapon_cues, frame_time);
                }
                let radio_active = radio.as_ref().is_some_and(CarRadio::is_playing);
                ambient_audio.update(
                    &world,
                    movement.position(),
                    radio.as_ref().is_some_and(CarRadio::visible),
                    frame_time,
                );
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
                    input.thrust || intro_active,
                    input.braking,
                    movement.is_moving() && !travel.open && !weapon_blocked,
                    movement.can_accelerate(),
                    movement.speed_ratio(),
                    frame_time,
                );
                let update_time = update_start.elapsed();

                let mut draw_uniforms = uniforms;
                let shake = weapon.shake() * vehicle.radius;
                draw_uniforms.camera_pos +=
                    Vector3::new((elapsed * 61.).sin(), (elapsed * 47.).cos(), 0.) * shake;
                starfield.draw(&mut framebuffer, &draw_uniforms);
                world.draw(&mut renderer, &mut framebuffer, &draw_uniforms);

                vehicle.braking = input.braking;
                vehicle.draw(&mut renderer, &mut framebuffer, &draw_uniforms, elapsed);
                movement.draw_trail(&mut framebuffer, &draw_uniforms);
                movement.draw_exhaust(&mut framebuffer, &draw_uniforms);
                if let Some(car_weapons) = &car_weapons {
                    car_weapons.draw(&mut framebuffer, &draw_uniforms, &world);
                }
                let launch_origin = vehicle.muzzle.map_or(movement.position(), |p| {
                    model_point(&uniforms.model_matrix, p)
                });
                if is_destroyer {
                    weapon.draw_effects(&mut framebuffer, &draw_uniforms, launch_origin);
                }

                let radio_visible = radio.as_ref().is_some_and(|r| r.visible());
                framebuffer.present_with_overlay_bottom(
                    &mut window,
                    &thread,
                    if intro_active || travel.open || weapon_blocked {
                        None
                    } else {
                        interaction.overlay()
                    },
                    if radio_visible { PANEL_HEIGHT + 32 } else { 14 },
                    |d| {
                        travel.draw(d, vehicle.kind, &targets, jump_flash);
                        if !intro_active && !travel.open && !weapon_blocked {
                            movement.draw_navigation(d);
                        }
                        if !travel.open
                            && let Some(alien) = &movement.alien
                        {
                            alien.draw_hud(d);
                        }
                        if !intro_active && let Some(radio) = &radio {
                            radio.draw(d);
                        }
                        if is_destroyer && !travel.open {
                            weapon.draw_hud(d, aim, &draw_uniforms);
                        }
                        if let Some(car_weapons) = &car_weapons
                            && !intro_active
                            && !travel.open
                        {
                            car_weapons.draw_hud(d, &draw_uniforms);
                            planet_finder.draw(d, &world, &draw_uniforms);
                        }
                        if !intro_active && !travel.open && !weapon_blocked {
                            draw_flight_status(
                                d,
                                world.travel_targets().len(),
                                total_planets,
                                movement.speed(),
                                movement.speed_multiple(),
                            );
                        }
                        draw_hangar_return(d);
                        if let Some(car_weapons) = &car_weapons {
                            car_weapons.draw_finale(d);
                        }
                        if intro_fade > 0 {
                            d.draw_rectangle(0, 0, 800, 600, Color::new(0, 0, 0, intro_fade));
                        }
                    },
                );
                if car_weapons
                    .as_ref()
                    .is_some_and(CarWeapons::finale_complete)
                {
                    world = build_solar_system();
                    solar_return = true;
                    continue 'hangar;
                }
                let mut metrics = renderer.metrics();
                metrics.update = update_time;
                metrics.frame = frame_start.elapsed();
                performance.record(metrics);
                thread::sleep(FRAME_DELAY);
            }
            return;
        }
    });
}

fn speed_meter_fill(speed_multiple: f32) -> f32 {
    ((1.0 + 9.0 * speed_multiple.max(0.0)).ln() / 10_000.0_f32.ln()).clamp(0.0, 1.0)
}

fn draw_flight_status(
    d: &mut RaylibDrawHandle,
    remaining_planets: usize,
    total_planets: usize,
    speed: f32,
    speed_multiple: f32,
) {
    const X: i32 = 16;
    const Y: i32 = 558;
    const WIDTH: i32 = 400;
    const HEIGHT: i32 = 28;
    const BAR_X: i32 = 254;
    const BAR_WIDTH: i32 = 146;

    d.draw_rectangle(X, Y, WIDTH, HEIGHT, Color::new(5, 14, 30, 225));
    d.draw_rectangle_lines(X, Y, WIDTH, HEIGHT, Color::new(74, 125, 174, 230));
    d.draw_text(
        &format!("PLANETAS {remaining_planets}/{total_planets}"),
        X + 10,
        Y + 7,
        14,
        Color::new(224, 236, 246, 255),
    );
    d.draw_text(
        &format!("VEL {speed:.0}"),
        X + 132,
        Y + 7,
        14,
        Color::new(224, 236, 246, 255),
    );
    d.draw_rectangle(BAR_X, Y + 8, BAR_WIDTH, 12, Color::new(12, 29, 48, 255));
    let fill = (BAR_WIDTH as f32 * speed_meter_fill(speed_multiple).clamp(0.0, 1.0)) as i32;
    d.draw_rectangle(BAR_X, Y + 8, fill, 12, Color::new(100, 220, 255, 255));
}

fn model_point(m: &Matrix, p: Vector3) -> Vector3 {
    Vector3::new(
        m.m0 * p.x + m.m4 * p.y + m.m8 * p.z + m.m12,
        m.m1 * p.x + m.m5 * p.y + m.m9 * p.z + m.m13,
        m.m2 * p.x + m.m6 * p.y + m.m10 * p.z + m.m14,
    )
}

fn preview_ships() {
    let car = Vehicle::car();
    let radius = car.radius;
    let mut vehicles = [car, Vehicle::saucer(radius), Vehicle::destroyer(radius)];
    std::fs::create_dir_all("target/previews").expect("crear directorio de capturas");
    let mut renderer = Renderer::default();
    for vehicle in &mut vehicles {
        let mut fb = Framebuffer::new(800, 600, Color::new(9, 16, 26, 255));
        let u = vehicle.preview_uniforms(0.25);
        renderer.begin_frame();
        vehicle.draw(&mut renderer, &mut fb, &u, 0.0);
        let name = if vehicle.kind == VehicleKind::Car {
            "car"
        } else if vehicle.kind == VehicleKind::Saucer {
            "ufo"
        } else {
            "destroyer"
        };
        fb.save_png(format!("target/previews/{name}.png"))
            .expect("guardar captura");
        println!("target/previews/{name}.png");
        if vehicle.kind == VehicleKind::Car {
            for (name, angle, braking) in [
                ("car-lights-rear", 0.25, false),
                ("car-lights-braking", 0.25, true),
                ("car-lights-front", 2.65, false),
            ] {
                vehicle.braking = braking;
                fb.clear();
                renderer.begin_frame();
                vehicle.draw(
                    &mut renderer,
                    &mut fb,
                    &vehicle.preview_uniforms(angle),
                    0.0,
                );
                fb.save_png(format!("target/previews/{name}.png"))
                    .expect("guardar luces del carro");
            }
            vehicle.braking = false;
        }
        if vehicle.kind == VehicleKind::Destroyer {
            for (name, camera_pos, camera_target) in [
                (
                    "destroyer-engines",
                    Vector3::new(0., -vehicle.radius * 0.10, -vehicle.radius * 1.60),
                    Vector3::new(0., -vehicle.radius * 0.10, 0.),
                ),
                (
                    "destroyer-engines-angle",
                    Vector3::new(
                        vehicle.radius * 1.05,
                        vehicle.radius * 0.45,
                        -vehicle.radius * 2.1,
                    ),
                    Vector3::new(0., -vehicle.radius * 0.12, -vehicle.radius * 0.20),
                ),
            ] {
                let u = Uniforms {
                    camera_pos,
                    camera_target,
                    ..vehicle.preview_uniforms(0.)
                };
                fb.clear();
                renderer.begin_frame();
                vehicle.draw(&mut renderer, &mut fb, &u, 0.);
                fb.save_png(format!("target/previews/{name}.png"))
                    .expect("guardar motores");
                println!("target/previews/{name}.png");
            }
        }
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
    fn speed_meter_remains_useful_at_extreme_speeds() {
        assert_eq!(speed_meter_fill(0.0), 0.0);
        assert!(speed_meter_fill(1.0) > 0.2);
        assert!(speed_meter_fill(100.0) > speed_meter_fill(1.0));
        assert_eq!(speed_meter_fill(1_000_000.0), 1.0);
    }

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
