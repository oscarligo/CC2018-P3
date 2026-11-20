use super::*;
use crate::ship::vehicle::Vehicle;

#[test]
#[ignore = "Genera capturas del sobrevuelo local con el renderer CPU sin abrir ventanas"]
fn capture_local_overflight() {
    use crate::{
        rendering::render::Renderer,
        scene::{Starfield, build_solar_system, world::WorldUpdate},
    };
    std::fs::create_dir_all("target/previews").unwrap();
    for (label, altitude) in [("cerca", 0.18), ("lejos", 1.6)] {
        let mut world = build_solar_system();
        let mut vehicle = Vehicle::car();
        let mut ship = ShipMovement::new(&vehicle.mesh, 2.);
        let body = world
            .navigation_target(Some("Coruscant"), Vector3::zero())
            .unwrap();
        let radial = Vector3::new(-0.76, 0.16, -0.63).normalize();
        ship.target = body.position + radial * (body.radius * (1. + altitude) + ship.radius);
        let local = super::super::local::LocalFlight::new(body);
        ship.orientation = local.tangent_orientation(ship.target, Quaternion::identity());
        ship.camera.snap(ship.orientation);
        let mut u = vehicle.preview_uniforms(0.);
        ship.update(
            &InputState {
                local_pressed: true,
                ..Default::default()
            },
            1. / 60.,
            &mut u,
            &world,
        );
        let mut renderer = Renderer::default();
        let mut fb = Framebuffer::new(800, 600, Color::BLACK);
        let stars = Starfield::new();
        for frame in 0..=360 {
            world.update(&WorldUpdate {
                frame_time: 1. / 60.,
            });
            ship.update(
                &InputState {
                    thrust: false,
                    turn_right: (210..240).contains(&frame),
                    ..Default::default()
                },
                1. / 60.,
                &mut u,
                &world,
            );
            assert!(ship.local.is_some());
            assert!(!ship.take_impact());
            // Draw periodically so the actual distance-based LOD settles during the flight.
            if [0, 30, 90, 150, 360].contains(&frame) {
                fb.clear();
                renderer.begin_frame();
                stars.draw(&mut fb, &u);
                world.draw(&mut renderer, &mut fb, &u);
                vehicle.draw(&mut renderer, &mut fb, &u, frame as f32 / 60.);
                if [90, 150, 360].contains(&frame) {
                    let path = format!("target/previews/local-overflight-{label}-{frame}.png");
                    fb.save_png(&path).unwrap();
                    println!("{path}");
                }
            }
        }
    }
}
