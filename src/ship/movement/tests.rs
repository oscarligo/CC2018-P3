use super::*;
use crate::ship::vehicle::Vehicle;

#[test]
fn flight_response_matches_at_30_60_and_144_fps() {
    let car = Vehicle::car();
    let world = crate::scene::world::World::new();
    let simulate = |fps: usize| {
        let mut ship = ShipMovement::new(&car.mesh, 2.);
        let mut u = car.preview_uniforms(0.);
        for stage in 0..3 {
            for _ in 0..fps * 2 {
                ship.update(
                    &InputState {
                        thrust: stage < 2,
                        boost: stage < 2,
                        braking: stage == 2,
                        turn_right: stage == 1,
                        pitch_up: stage == 1,
                        ..Default::default()
                    },
                    1. / fps as f32,
                    &mut u,
                    &world,
                );
            }
        }
        (ship.target, ship.orientation, ship.speed, u.camera_pos)
    };
    let baseline = simulate(60);
    for fps in [30, 144] {
        let result = simulate(fps);
        assert!(
            (result.0 - baseline.0).length() < car.radius * 0.08,
            "FPS {fps}: {:?} / {:?}",
            result.0,
            baseline.0
        );
        let a = transform_direction(&result.1.to_matrix(), Vector3::new(0., 0., 1.));
        let b = transform_direction(&baseline.1.to_matrix(), Vector3::new(0., 0., 1.));
        assert!(a.dot(b) > 0.9999);
        assert!((result.2 - baseline.2).abs() < 0.001);
        assert!((result.3 - baseline.3).length() < car.radius * 0.15);
    }
}

#[test]
fn thrust_and_spin_are_independent_without_steering_the_camera() {
    let car = Vehicle::car();
    let saucer = Vehicle::saucer(car.radius);
    let mut ship = ShipMovement::new(&saucer.mesh, 2.0);
    ship.configure_vehicle(car.radius, 2.0, true);
    let world = crate::scene::world::World::new();
    let mut u = saucer.preview_uniforms(0.0);
    for _ in 0..60 {
        ship.update(
            &InputState {
                thrust: true,
                boost: true,
                ..Default::default()
            },
            1.0 / 60.0,
            &mut u,
            &world,
        );
    }
    assert!(ship.speed > 0.0);
    assert_eq!(ship.alien.as_ref().unwrap().visual_rotation().y, 0.0);
    let before = ship.speed;
    for _ in 0..30 {
        ship.update(
            &InputState {
                spin_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
            &mut u,
            &world,
        );
    }
    assert!(ship.speed < before);
    assert!(ship.alien.as_ref().unwrap().visual_rotation().y > 0.1);
    assert_eq!(ship.orientation, Quaternion::identity());
}

#[test]
fn all_vehicles_bounce_instead_of_tunneling_through_a_planet() {
    use crate::{celestial_shaders::CelestialKind, objects::celestial::CelestialBody};
    let car = Vehicle::car();
    let saucer = Vehicle::saucer(car.radius);
    let destroyer = Vehicle::destroyer(car.radius);
    for vehicle in [&car, &saucer, &destroyer] {
        let mut ship = ShipMovement::new(&vehicle.mesh, 2.0);
        ship.configure_vehicle(
            car.radius,
            2.0,
            vehicle.kind == super::super::vehicle::VehicleKind::Saucer,
        );
        ship.target = Vector3::zero();
        ship.speed = 10_000.0;
        let center = Vector3::new(0., 0., 100.);
        let mut world = crate::scene::world::World::new();
        world.add(CelestialBody::new(CelestialKind::Rocky, center).with_scale(20.0));
        ship.advance_with_collisions(0.1, &world);
        assert!(ship.target.z < 100.0 - 20.0 - ship.radius);
        assert!(ship.recoil.z < 0.0);
        assert_eq!(ship.speed, 0.0);
        assert!(ship.take_impact());
        assert!(!ship.take_impact());
        for _ in 0..120 {
            ship.advance_with_collisions(1.0 / 60.0, &world);
            assert!((ship.target - center).length() > 20.0 + ship.radius);
        }
        ship.target = center + Vector3::new(1., 0., 0.);
        ship.recoil = Vector3::zero();
        ship.advance_with_collisions(1.0 / 60.0, &world);
        assert!((ship.target - center).length() > 20.0 + ship.radius);
    }
}

#[test]
fn aiming_does_not_move_the_vehicle_or_change_its_heading() {
    let car = Vehicle::car();
    let saucer = Vehicle::saucer(car.radius);
    let mut ship = ShipMovement::new(&saucer.mesh, 2.0);
    ship.configure_vehicle(car.radius, 2.0, true);
    let world = crate::scene::world::World::new();
    let mut u = saucer.preview_uniforms(0.0);
    let position = ship.target;
    let heading = ship.orientation;
    ship.command(super::super::commands::Command::Chase);
    for i in 0..120 {
        ship.update(
            &InputState {
                mouse_position: Vector2::new((i * 5) as f32, 250.0),
                ..Default::default()
            },
            1.0 / 60.0,
            &mut u,
            &world,
        );
    }
    assert_eq!(ship.target, position);
    assert_eq!(ship.orientation, heading);
}

#[test]
fn sneeze_covers_a_large_distance_and_returns_to_normal_speed() {
    let car = Vehicle::car();
    let vehicle = Vehicle::saucer(car.radius);
    let mut ship = ShipMovement::new(&vehicle.mesh, 2.0);
    ship.configure_vehicle(car.radius, 2.0, true);
    let world = crate::scene::world::World::new();
    let mut u = vehicle.preview_uniforms(0.0);
    ship.update(&InputState::default(), 0.0, &mut u, &world);
    ship.command(super::super::commands::Command::Sneeze);
    let mut distance = 0.0;
    let mut previous = ship.target;
    for _ in 0..330 {
        ship.update(&InputState::default(), 1.0 / 60.0, &mut u, &world);
        distance += (ship.target - previous).length();
        previous = ship.target;
    }
    assert!(
        distance > car.radius * 350.0,
        "distancia {distance}, radio {}",
        car.radius
    );
    assert!(ship.speed <= ship.max_speed * BOOST_SPEED_MULTIPLIER);
    assert!(!ship.alien.as_ref().unwrap().active());
}

#[test]
fn saucer_acceleration_and_top_speed_are_three_times_the_car() {
    let car = Vehicle::car();
    let saucer = Vehicle::saucer(car.radius);
    let mut normal = ShipMovement::new(&car.mesh, 2.0);
    let mut alien = ShipMovement::new(&saucer.mesh, 2.0);
    normal.configure_vehicle(car.radius, 2.0, false);
    alien.configure_vehicle(car.radius, 2.0, true);
    for _ in 0..300 {
        normal.update_motion(true, true, false, 1.0 / 60.0);
        alien.update_motion(true, true, false, 1.0 / 60.0);
        assert!((alien.speed - normal.speed * 3.0).abs() < 0.001);
    }
    for _ in 0..300 {
        normal.update_motion(false, false, true, 1.0 / 60.0);
        alien.update_motion(false, false, true, 1.0 / 60.0);
    }
    assert_eq!(normal.speed, 0.0);
    assert_eq!(alien.speed, 0.0);
}

#[test]
fn every_planet_has_a_safe_jump_after_orbiting() {
    let mut world = crate::scene::build_solar_system();
    world.update(&crate::scene::world::WorldUpdate { frame_time: 51.0 });
    let car = Vehicle::car();
    let saucer = Vehicle::saucer(car.radius);
    let mut ship = ShipMovement::new(&saucer.mesh, 2.0);
    ship.configure_vehicle(car.radius, 2.0, true);
    for target in world.travel_targets() {
        ship.speed = 100.0;
        assert!(ship.teleport(&target, &world), "{}", target.name);
        assert_eq!(ship.speed, 0.0);
        let offset = target.position - ship.target;
        assert!(offset.length() > target.radius * 3.0);
        let forward = transform_direction(&ship.orientation.to_matrix(), Vector3::new(0., 0., 1.));
        assert!(forward.dot(offset.normalize()) > 0.999);
        let mut hits = Vec::new();
        world.query_sphere(
            ship.target,
            ship.camera.reference_distance() * 1.5,
            &mut hits,
        );
        assert!(hits.is_empty(), "{}", target.name);
    }
}
