use super::*;

/// Exercise the actual flight, weapon and HUD without manual input or audible playback.
/// All checkpoints use reachable states, not pre-filled charge values.
pub fn preview() {
    use crate::ship::{InputState, ShipMovement, vehicle::Vehicle};
    use crate::{
        rendering::render::Renderer,
        scene::{Starfield, build_solar_system, world::WorldUpdate},
    };
    use std::{fs, time::Instant};
    let (mut window, thread) = raylib::init()
        .size(800, 600)
        .title("Destructor / verificación")
        .hidden()
        .build();
    window.set_exit_key(None);
    fs::create_dir_all("target/previews").expect("crear capturas");
    let mut ship = Vehicle::destroyer(Vehicle::car().radius);
    let mut renderer = Renderer::default();
    let mut fb = Framebuffer::new(800, 600, Color::new(5, 9, 16, 255));
    let stars = Starfield::new();
    let hangar = ship.preview_uniforms(0.18);
    stars.draw(&mut fb, &hangar);
    ship.draw(&mut renderer, &mut fb, &hangar, 0.);
    for _ in 0..2 {
        fb.present(&mut window, &thread, None, |d| {
            crate::ship::menu::draw_hangar(d, 2)
        });
    }
    window.take_screenshot(&thread, "target/previews/destroyer-hangar.png");
    // Also capture the open bay from below to verify its independent moving geometry.
    let under = Uniforms {
        camera_pos: Vector3::new(ship.radius * 1.4, -ship.radius * 0.9, ship.radius * 1.8),
        ..hangar
    };
    ship.weapon_open = 1.;
    ship.weapon_charge = 0.9;
    fb.clear();
    stars.draw(&mut fb, &under);
    ship.draw(&mut renderer, &mut fb, &under, 1.);
    fb.save_png("target/previews/destroyer-bay.png").unwrap();
    let mut world = build_solar_system();
    world.update(&WorldUpdate { frame_time: 0. });
    let count = world.travel_targets().len();
    let target = world
        .travel_targets()
        .into_iter()
        .find(|t| t.name == "Kashyyyk")
        .unwrap();
    let mut movement = ShipMovement::new(&ship.mesh, 1.);
    movement.configure_vehicle(ship.radius, 1., false);
    assert!(movement.teleport(&target, &world));
    let mut u = hangar;
    movement.update(&InputState::default(), 0., &mut u, &world);
    let mut weapon = Doomsday::default();
    fn frame(
        w: &mut Doomsday,
        world: &mut World,
        movement: &mut ShipMovement,
        u: &mut Uniforms,
        input: Controls,
        dt: f32,
        ship: &Vehicle,
    ) {
        let previous = w.target;
        world.update(&WorldUpdate { frame_time: dt });
        let origin = crate::model_point(&u.model_matrix, ship.muzzle.unwrap());
        let m = u.model_matrix;
        let aim = world.weapon_target(movement.position(), Vector3::new(m.m8, m.m9, m.m10));
        w.update(dt, &input, aim, world, origin);
        if let Some(target) = w.target.or(previous) {
            movement.weapon_station(&previous.unwrap_or(target), &target, dt, u, world);
        } else {
            movement.update(&InputState::default(), dt, u, world);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn capture(
        name: &str,
        w: &Doomsday,
        world: &mut World,
        ship: &mut Vehicle,
        u: &Uniforms,
        window: &mut RaylibHandle,
        thread: &RaylibThread,
    ) {
        let mut fb = Framebuffer::new(800, 600, Color::new(5, 9, 16, 255));
        let mut r = Renderer::default();
        let u = Uniforms {
            light_position: world.primary_light_position(),
            ..*u
        };
        let now = Instant::now();
        Starfield::new().draw(&mut fb, &u);
        world.draw(&mut r, &mut fb, &u);
        ship.weapon_charge = w.charge;
        ship.weapon_open = w.bay_open;
        ship.bomb_visible = w.bomb_visible();
        ship.draw(&mut r, &mut fb, &u, w.clock);
        w.draw_effects(
            &mut fb,
            &u,
            crate::model_point(&u.model_matrix, ship.muzzle.unwrap()),
        );
        let elapsed = now.elapsed();
        let m = u.model_matrix;
        let aim = world.weapon_target(
            crate::model_point(&m, Vector3::zero()),
            Vector3::new(m.m8, m.m9, m.m10),
        );
        let total_planets = build_solar_system().travel_targets().len();
        for _ in 0..2 {
            fb.present(window, thread, None, |d| {
                crate::ship::menu::TravelMenu::default().draw(
                    d,
                    crate::ship::vehicle::VehicleKind::Destroyer,
                    &world.travel_targets(),
                    0.,
                );
                w.draw_hud(d, aim, &u);
                if !w.busy() {
                    crate::draw_flight_status(
                        d,
                        crate::ship::vehicle::VehicleKind::Destroyer,
                        world.travel_targets().len(),
                        total_planets,
                        0.,
                        0.,
                    );
                }
            });
        }
        let path = format!("target/previews/destroyer-{name}.png");
        window.take_screenshot(thread, &path);
        println!(
            "{path} / CPU {:.2} ms / {} triángulos",
            elapsed.as_secs_f64() * 1000.,
            r.metrics().triangles
        );
    }
    capture(
        "idle",
        &weapon,
        &mut world,
        &mut ship,
        &u,
        &mut window,
        &thread,
    );
    frame(
        &mut weapon,
        &mut world,
        &mut movement,
        &mut u,
        Controls {
            activate: true,
            ..Default::default()
        },
        0.,
        &ship,
    );
    assert_eq!(
        weapon.phase,
        Phase::Charging,
        "El planeta debe poder fijarse después de viajar"
    );
    let mut saved = false;
    for i in 0..1800 {
        frame(
            &mut weapon,
            &mut world,
            &mut movement,
            &mut u,
            Controls {
                tap_count: u8::from(i % 7 == 0),
                ..Default::default()
            },
            1. / 60.,
            &ship,
        );
        if !saved && weapon.charge > 0.72 {
            capture(
                "charge",
                &weapon,
                &mut world,
                &mut ship,
                &u,
                &mut window,
                &thread,
            );
            saved = true;
        }
        if weapon.phase == Phase::Confirm {
            break;
        }
    }
    assert_eq!(weapon.phase, Phase::Confirm);
    capture(
        "confirm",
        &weapon,
        &mut world,
        &mut ship,
        &u,
        &mut window,
        &thread,
    );
    frame(
        &mut weapon,
        &mut world,
        &mut movement,
        &mut u,
        Controls {
            left: true,
            confirm: true,
            ..Default::default()
        },
        0.,
        &ship,
    );
    for i in 0..600 {
        frame(
            &mut weapon,
            &mut world,
            &mut movement,
            &mut u,
            Controls::default(),
            1. / 60.,
            &ship,
        );
        match i {
            60 => capture(
                "flight",
                &weapon,
                &mut world,
                &mut ship,
                &u,
                &mut window,
                &thread,
            ),
            185 => capture(
                "rupture",
                &weapon,
                &mut world,
                &mut ship,
                &u,
                &mut window,
                &thread,
            ),
            234 => capture(
                "explosion",
                &weapon,
                &mut world,
                &mut ship,
                &u,
                &mut window,
                &thread,
            ),
            430 => capture(
                "debris",
                &weapon,
                &mut world,
                &mut ship,
                &u,
                &mut window,
                &thread,
            ),
            _ => {}
        }
    }
    assert_eq!(world.travel_targets().len(), count - 1);
    assert!(world.target_by_id(target.id).is_none());
    world.restore_planets();
    assert_eq!(world.travel_targets().len(), count);
    let restored = world.target_by_id(target.id).unwrap();
    assert!(movement.teleport(&restored, &world));
    movement.update(&InputState::default(), 0., &mut u, &world);
    frame(
        &mut weapon,
        &mut world,
        &mut movement,
        &mut u,
        Controls {
            activate: true,
            ..Default::default()
        },
        0.,
        &ship,
    );
    assert_eq!(weapon.phase, Phase::Charging);
    for i in 0..60 {
        frame(
            &mut weapon,
            &mut world,
            &mut movement,
            &mut u,
            Controls {
                tap_count: u8::from(i % 7 == 0),
                ..Default::default()
            },
            1. / 60.,
            &ship,
        );
    }
    frame(
        &mut weapon,
        &mut world,
        &mut movement,
        &mut u,
        Controls {
            cancel: true,
            ..Default::default()
        },
        0.,
        &ship,
    );
    capture(
        "cancel",
        &weapon,
        &mut world,
        &mut ship,
        &u,
        &mut window,
        &thread,
    );
    for (name, cue) in [
        ("reactor", None),
        ("tap", Some(Cue::Tap)),
        ("ready", Some(Cue::Ready)),
        ("abort", Some(Cue::Abort)),
        ("launch", Some(Cue::Launch)),
        ("arrival", Some(Cue::Arrival)),
        ("explosion", Some(Cue::Explosion)),
    ] {
        fs::write(
            format!("target/previews/destroyer-{name}.wav"),
            crate::ship::doomsday_audio::synthesize(cue),
        )
        .unwrap();
    }
    println!(
        "Secuencia completa, cancelación y restauración verificadas. Capturas y WAV en target/previews."
    );
}
