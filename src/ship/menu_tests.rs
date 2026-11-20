use super::*;

#[test]
#[ignore = "Verifica el regreso al hangar con una ventana oculta y entradas de raylib"]
fn return_to_hangar_and_reselect_without_reopening_the_window() {
    let (mut window, thread) = raylib::init()
        .size(800, 600)
        .title("Hangar / verificación")
        .hidden()
        .build();
    window.set_exit_key(None);
    std::fs::create_dir_all("target/previews").unwrap();
    // raylib's own automation API changes its input state without controlling the desktop.
    let path = "target/previews/hangar-inputs.txt";
    std::fs::write(
        path,
        concat!(
            "c 12\n",
            "e 0 2 290 0 0 0\n", // F1 down
            "e 0 2 262 0 0 0\n", // right down
            "e 0 2 257 0 0 0\n", // enter down
            "e 0 2 256 0 0 0\n", // escape down
            "e 0 1 290 0 0 0\n",
            "e 0 1 262 0 0 0\n",
            "e 0 1 257 0 0 0\n",
            "e 0 1 256 0 0 0\n",
            "e 0 7 680 30 0 0\n", // pointer inside return button
            "e 0 6 0 0 0 0\n",    // left mouse down
            "e 0 5 0 0 0 0\n",
            "e 0 7 400 300 0 0\n", // pointer outside return button
        ),
    )
    .unwrap();
    let list = window.load_automation_event_list(Some(path.into()));
    assert_eq!(list.count(), 12);
    let events = list.events();
    let mut fb = Framebuffer::new(800, 600, Color::BLACK);
    let mut renderer = Renderer::default();
    let car = Vehicle::car();
    let radius = car.radius;
    let mut vehicles = [car, Vehicle::saucer(radius), Vehicle::destroyer(radius)];

    // Start in flight and verify that both controls request the same transition.
    let mut world = crate::scene::build_solar_system();
    let targets = world.travel_targets();
    let mut movement = crate::ship::ShipMovement::new(&vehicles[0].mesh, 2.);
    let mut uniforms = vehicles[0].preview_uniforms(0.);
    movement.update(
        &crate::ship::InputState::default(),
        0.,
        &mut uniforms,
        &world,
    );
    world.draw(&mut renderer, &mut fb, &uniforms);
    vehicles[0].draw(&mut renderer, &mut fb, &uniforms, 0.);
    for _ in 0..2 {
        fb.present(&mut window, &thread, None, |d| {
            TravelMenu::default().draw(d, VehicleKind::Car, &targets, 0.);
            draw_hangar_return(d);
        });
    }
    window.take_screenshot(&thread, "target/previews/hangar-return-flight.png");
    events[0].play();
    assert!(hangar_requested(&window));
    events[4].play();
    events[8].play();
    events[9].play();
    assert!(hangar_requested(&window));
    events[10].play();
    events[11].play();
    events[9].play();
    assert!(!hangar_requested(&window));
    events[10].play();

    for initial in [0, 1, 2, 0] {
        let selected = choose_vehicle_with_input(
            &mut window,
            &thread,
            &mut fb,
            &mut renderer,
            &mut vehicles,
            initial,
            false,
            |_, frame| {
                assert!(frame < 5, "El hangar debe aceptar las entradas simuladas");
                events[6].play();
                events[5].play();
                match frame {
                    0 => events[2].play(), // A stale confirmation must not skip the hangar.
                    1 => events[1].play(),
                    2 => events[2].play(),
                    _ => unreachable!(),
                }
            },
        );
        assert_eq!(selected, Some((initial + 1) % 3));
        assert_eq!(world.travel_targets().len(), targets.len());
    }
    window.take_screenshot(&thread, "target/previews/hangar-return-menu.png");
    let result = choose_vehicle_with_input(
        &mut window,
        &thread,
        &mut fb,
        &mut renderer,
        &mut vehicles,
        0,
        false,
        |_, frame| {
            assert!(frame < 3);
            events[6].play();
            if frame == 1 {
                events[3].play();
            }
        },
    );
    assert_eq!(result, None);
}
