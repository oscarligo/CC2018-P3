use super::local_flight::LOCAL_PITCH_LIMIT;
use super::*;
use crate::ship::vehicle::Vehicle;

#[test]
fn local_cruise_approaches_the_surface_with_a_close_chase_camera() {
    use crate::{
        celestial_shaders::CelestialKind, objects::celestial::CelestialBody, scene::world::World,
    };
    let car = Vehicle::car();
    let center = Vector3::new(600., -200., 1500.);
    for radius in [100., 500., 1500.] {
        for distance_factor in [1.2, 2.0, 4.0] {
            let mut world = World::new();
            world.add(
                CelestialBody::new(CelestialKind::Rocky, center)
                    .named("Prueba")
                    .with_scale(radius),
            );
            let mut ship = ShipMovement::new(&car.mesh, 2.);
            ship.target = center + Vector3::new(radius * distance_factor, 0., 0.);
            let mut uniforms = car.preview_uniforms(0.);
            ship.update(
                &InputState {
                    local_pressed: true,
                    ..Default::default()
                },
                1. / 60.,
                &mut uniforms,
                &world,
            );
            for _ in 0..1200 {
                let previous = ship.target;
                ship.update(&InputState::default(), 1. / 60., &mut uniforms, &world);
                assert!(
                    (ship.target - previous).length() < radius * 0.015,
                    "Approach must not teleport"
                );
                assert!(
                    (uniforms.camera_target - ship.target).length() < 0.001,
                    "Keep the usual ship-centered view"
                );
                assert!(
                    (uniforms.camera_pos - ship.target).length() < car.radius * 3.5,
                    "Do not zoom out with planetary altitude"
                );
            }
            let altitude = (ship.target - center).length() - radius;
            let desired = (radius * 0.05).max(car.radius * 3.5);
            assert!(
                (altitude - desired).abs() < 0.25,
                "Approach must settle into a low overflight"
            );
            assert!(ship.local.as_ref().unwrap().approach_altitude.is_none());
            // A ray through the lower part of the view must see the nearby ground.
            let forward = (uniforms.camera_target - uniforms.camera_pos).normalize();
            let right = uniforms.camera_up.cross(forward).normalize();
            let up = forward.cross(right);
            let ray = (forward - up * (0.9 * 30_f32.to_radians().tan())).normalize();
            let to_center = center - uniforms.camera_pos;
            let along = ray.dot(to_center);
            assert!(along > 0. && to_center.length_sqr() - along * along < radius * radius);
            assert!(ship.speed > 0.);
            assert!(!ship.take_impact());
        }
    }
}

#[test]
fn local_climb_levels_on_release_with_a_surface_camera_at_different_frame_rates() {
    use crate::{
        celestial_shaders::CelestialKind, objects::celestial::CelestialBody, scene::world::World,
    };
    let car = Vehicle::car();
    let mut world = World::new();
    world.add(
        CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
            .named("Prueba")
            .with_scale(100.),
    );
    let simulate = |fps: usize| {
        let mut ship = ShipMovement::new(&car.mesh, 2.);
        ship.target = Vector3::new(100. + ship.radius * 5., 0., 0.);
        let mut u = car.preview_uniforms(0.);
        ship.update(
            &InputState {
                local_pressed: true,
                ..Default::default()
            },
            1. / fps as f32,
            &mut u,
            &world,
        );
        let start = ship.target.length();
        for _ in 0..fps * 2 {
            ship.update(
                &InputState {
                    thrust: true,
                    pitch_up: true,
                    turn_right: true,
                    ..Default::default()
                },
                1. / fps as f32,
                &mut u,
                &world,
            );
            let up = ship.target.normalize();
            let forward =
                transform_direction(&ship.orientation.to_matrix(), Vector3::new(0., 0., 1.));
            assert!(forward.dot(up) <= LOCAL_PITCH_LIMIT.sin() + 0.001);
        }
        assert!(
            ship.target.length() > start + ship.radius * 0.1,
            "S debe permitir ascender"
        );
        for _ in 0..fps * 4 {
            ship.update(
                &InputState {
                    thrust: true,
                    ..Default::default()
                },
                1. / fps as f32,
                &mut u,
                &world,
            );
        }
        let settled = ship.target.length();
        for _ in 0..fps * 2 {
            ship.update(
                &InputState {
                    thrust: true,
                    ..Default::default()
                },
                1. / fps as f32,
                &mut u,
                &world,
            );
        }
        assert!(
            (ship.target.length() - settled).abs() < ship.radius * 0.01,
            "La altura debe estabilizarse al soltar W/S"
        );
        assert!(
            u.camera_up.dot(ship.target.normalize()) > 0.995,
            "La cámara debe conservar el horizonte local"
        );
        assert!(u.camera_pos.length() > 100. + ship.radius * 0.1);
        assert!(!ship.take_impact());
        (ship.target, u.camera_pos)
    };
    let reference = simulate(60);
    for fps in [30, 144] {
        let result = simulate(fps);
        assert!(
            (result.0 - reference.0).length() < car.radius * 0.12,
            "Vuelo local a {fps} FPS"
        );
        assert!(
            (result.1 - reference.1).length() < car.radius * 0.2,
            "Cámara local a {fps} FPS"
        );
    }
}

#[test]
fn local_speed_slows_near_the_ground_for_every_vehicle_and_brakes_to_a_stop() {
    use crate::{
        celestial_shaders::CelestialKind, objects::celestial::CelestialBody, scene::world::World,
    };
    let car = Vehicle::car();
    let base_radius = car.radius;
    let mut world = World::new();
    world.add(
        CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
            .named("Prueba")
            .with_scale(100.),
    );
    for vehicle in [
        car,
        Vehicle::saucer(base_radius),
        Vehicle::destroyer(base_radius),
    ] {
        let mut ship = ShipMovement::new(&vehicle.mesh, 2.);
        ship.configure_vehicle(
            vehicle.radius,
            2.,
            vehicle.kind == super::super::vehicle::VehicleKind::Saucer,
        );
        let mut u = vehicle.preview_uniforms(0.);
        ship.target = Vector3::new(100. + ship.radius * 5., 0., 0.);
        ship.update(
            &InputState {
                local_pressed: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        ship.local.as_mut().unwrap().approach_altitude = None;
        for _ in 0..600 {
            ship.update(
                &InputState {
                    thrust: true,
                    ..Default::default()
                },
                1. / 60.,
                &mut u,
                &world,
            );
        }
        let far_speed = ship.speed;
        ship.target = ship.target.normalize() * (100. + ship.radius * 1.25);
        for _ in 0..300 {
            ship.update(
                &InputState {
                    thrust: true,
                    ..Default::default()
                },
                1. / 60.,
                &mut u,
                &world,
            );
        }
        assert!(
            ship.speed < far_speed * 0.8,
            "Debe reducir la velocidad junto al terreno: {:?}",
            vehicle.kind
        );
        assert!(!ship.take_impact());
        for _ in 0..120 {
            ship.update(
                &InputState {
                    braking: true,
                    ..Default::default()
                },
                1. / 60.,
                &mut u,
                &world,
            );
        }
        assert_eq!(ship.speed, 0.);
        let stopped = ship.target;
        ship.update(&InputState::default(), 1., &mut u, &world);
        assert!((ship.target - stopped).length() < 0.001);
        ship.update(
            &InputState {
                thrust: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        for _ in 0..60 {
            ship.update(&InputState::default(), 1. / 60., &mut u, &world);
        }
        assert!(
            (ship.target - stopped).length() > ship.radius * 0.5,
            "Espacio debe reanudar el crucero"
        );
    }
}

#[test]
fn a_local_dive_still_bounces_and_the_camera_stays_outside() {
    use crate::{
        celestial_shaders::CelestialKind, objects::celestial::CelestialBody, scene::world::World,
    };
    let car = Vehicle::car();
    let mut world = World::new();
    world.add(
        CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
            .named("Prueba")
            .with_scale(100.),
    );
    let mut ship = ShipMovement::new(&car.mesh, 2.);
    ship.target = Vector3::new(100. + ship.radius * 3., 0., 0.);
    let body = world.navigation_target(None, ship.target).unwrap();
    ship.local = Some(super::super::local::LocalFlight::new(body));
    ship.orientation =
        Quaternion::from_vector3_to_vector3(Vector3::new(0., 0., 1.), Vector3::new(-1., 0., 0.));
    ship.speed = 10_000.;
    let mut u = car.preview_uniforms(0.);
    ship.update(
        &InputState {
            thrust: true,
            ..Default::default()
        },
        0.1,
        &mut u,
        &world,
    );
    assert!(ship.target.length() > 100. + ship.radius);
    assert!(u.camera_pos.length() > 100.);
    assert!(ship.take_impact());
}

#[test]
fn local_flight_circles_without_altitude_drift_and_exits_without_a_jump() {
    use crate::{
        celestial_shaders::CelestialKind, objects::celestial::CelestialBody, scene::world::World,
    };
    let car = Vehicle::car();
    let mut world = World::new();
    world.add(
        CelestialBody::new(CelestialKind::Rocky, Vector3::zero())
            .named("Prueba")
            .with_scale(100.),
    );
    let mut ship = ShipMovement::new(&car.mesh, 2.);
    let mut distance = 100. + ship.radius * 5.;
    ship.target = Vector3::new(distance, 0., 0.);
    let mut u = car.preview_uniforms(0.);
    ship.update(
        &InputState {
            local_pressed: true,
            ..Default::default()
        },
        1. / 60.,
        &mut u,
        &world,
    );
    assert!(ship.local.is_some());
    for _ in 0..600 {
        ship.update(&InputState::default(), 1. / 60., &mut u, &world);
    }
    distance = ship.target.length();
    let start = ship.target;
    let mut length = 0.;
    for _ in 0..12000 {
        let before = ship.target;
        ship.update(
            &InputState {
                thrust: true,
                boost: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        length += (ship.target - before).length();
        assert!(
            (ship.target.length() - distance).abs() < 0.15,
            "altitud {}",
            ship.target.length()
        );
        assert!(!ship.take_impact());
        if length > std::f32::consts::TAU * distance {
            break;
        }
    }
    assert!(
        length > std::f32::consts::TAU * distance,
        "Debe completar una vuelta"
    );
    assert!((ship.target - start).length() < distance * 0.08);
    let before = ship.target;
    let speed = ship.speed;
    ship.update(
        &InputState {
            local_pressed: true,
            thrust: true,
            ..Default::default()
        },
        1. / 60.,
        &mut u,
        &world,
    );
    assert!(ship.local.is_none());
    assert!((ship.target - before).length() < speed * 0.04);
    assert!((ship.speed - speed).abs() <= ship.acceleration / 60. + 0.001);
}

#[test]
fn local_reference_follows_the_orbit_and_ignores_a_paused_toggle() {
    use crate::{
        celestial_shaders::CelestialKind,
        objects::celestial::CelestialBody,
        scene::world::{Orbit, World, WorldUpdate},
    };
    let car = Vehicle::car();
    let mut world = World::new();
    let center = Vector3::new(500., 0., 0.);
    world.add(
        CelestialBody::new(CelestialKind::Rocky, center)
            .named("Prueba")
            .with_scale(100.)
            .with_orbit(Orbit::new(
                Vector3::zero(),
                center,
                Vector3::new(0., 1., 0.),
                0.1,
            )),
    );
    let mut ship = ShipMovement::new(&car.mesh, 2.);
    ship.target = center + Vector3::new(0., 150., 0.);
    let mut u = car.preview_uniforms(0.);
    ship.update(
        &InputState {
            local_pressed: true,
            ..Default::default()
        },
        0.,
        &mut u,
        &world,
    );
    assert!(ship.local.is_none());
    ship.update(
        &InputState {
            local_pressed: true,
            ..Default::default()
        },
        1. / 60.,
        &mut u,
        &world,
    );
    ship.update(
        &InputState {
            braking: true,
            ..Default::default()
        },
        1. / 60.,
        &mut u,
        &world,
    );
    let relative = ship.target - center;
    for _ in 0..120 {
        world.update(&WorldUpdate {
            frame_time: 1. / 60.,
        });
        ship.update(&InputState::default(), 1. / 60., &mut u, &world);
        let target = world
            .navigation_target(Some("Prueba"), ship.target)
            .unwrap();
        assert!((ship.target - target.position - relative).length() < 0.01);
    }
}
