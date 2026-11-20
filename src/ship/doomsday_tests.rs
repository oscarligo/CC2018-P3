use super::*;
use crate::{
    celestial_shaders::CelestialKind,
    objects::celestial::CelestialBody,
    scene::world::{Orbit, WorldUpdate},
};
fn setup() -> (Doomsday, World, TravelTarget) {
    let mut world = World::new();
    world.add(
        CelestialBody::new(CelestialKind::Rocky, Vector3::new(0., 0., 100.))
            .with_scale(10.)
            .named("Prueba")
            .with_orbit(Orbit::new(
                Vector3::zero(),
                Vector3::new(0., 0., 100.),
                Vector3::new(0., 1., 0.),
                0.1,
            )),
    );
    let target = world.travel_targets()[0];
    let mut weapon = Doomsday::default();
    weapon.update(
        0.,
        &Controls {
            activate: true,
            ..Default::default()
        },
        Some(target),
        &mut world,
        Vector3::zero(),
    );
    (weapon, world, target)
}
fn mash(hz: f32, fps: f32, seconds: f32) -> (Doomsday, f32) {
    let (mut w, mut world, _) = setup();
    let mut next = 0.;
    let mut time = 0.;
    for _ in 0..(fps * seconds) as usize {
        let mut tap_count = 0;
        while time + 0.00001 >= next {
            tap_count += 1;
            next += 1. / hz;
        }
        w.update(
            1. / fps,
            &Controls {
                tap_count,
                ..Default::default()
            },
            None,
            &mut world,
            Vector3::zero(),
        );
        time += 1. / fps;
        if w.phase == Phase::Confirm {
            break;
        }
    }
    (w, time)
}
#[test]
fn charge_requires_sustained_fast_presses_at_different_frame_rates() {
    let mut times = Vec::new();
    for fps in [5., 10., 30., 60., 144.] {
        let (slow, _) = mash(6., fps, 30.);
        assert_eq!(slow.phase, Phase::Charging);
        assert!(slow.charge < 1.0);
        let (fast, time) = mash(9., fps, 18.);
        assert_eq!(fast.phase, Phase::Confirm, "{fps} FPS, q={}", fast.charge);
        assert!(
            (MIN_CHARGE_SECONDS..16.).contains(&time),
            "Carga a {fps} FPS: {time}s"
        );
        times.push(time);
    }
    let (w, time) = mash(7., 60., 25.);
    assert_eq!(w.phase, Phase::Confirm);
    assert!(time < 17.);
    println!("Carga a 7 pulsaciones/s: {time:.2}s (mínimo: {MIN_CHARGE_SECONDS}s)");
    let min = times.iter().copied().fold(f32::INFINITY, f32::min);
    let max = times.iter().copied().fold(0., f32::max);
    assert!(
        max - min < 0.45,
        "La cola debe conservar las pulsaciones aun con pocos cuadros: {times:?}"
    );
}
#[test]
fn one_press_or_a_pause_cannot_store_progress() {
    let (mut w, mut world, _) = setup();
    w.charge = 0.92;
    for _ in 0..300 {
        w.update(
            1. / 60.,
            &Controls::default(),
            None,
            &mut world,
            Vector3::zero(),
        );
    }
    assert_eq!(w.charge, 0.);
    assert_eq!(w.phase, Phase::Charging);
    w.charge = 0.;
    w.update(
        1. / 60.,
        &Controls {
            tap_count: 1,
            ..Default::default()
        },
        None,
        &mut world,
        Vector3::zero(),
    );
    for _ in 0..60 {
        w.update(
            1. / 60.,
            &Controls::default(),
            None,
            &mut world,
            Vector3::zero(),
        );
    }
    assert_eq!(w.charge, 0.);
}
#[test]
fn final_tap_never_authorizes_and_cancel_is_selected_by_default() {
    let (mut w, mut world, _) = setup();
    w.charge = 0.99;
    w.charge_time = MIN_CHARGE_SECONDS;
    w.update(
        0.,
        &Controls {
            tap_count: 1,
            confirm: true,
            left: true,
            ..Default::default()
        },
        None,
        &mut world,
        Vector3::zero(),
    );
    assert_eq!(w.phase, Phase::Confirm);
    assert!(!w.launch_selected);
    w.update(1., &Controls::default(), None, &mut world, Vector3::zero());
    assert_eq!(w.charge, 1.);
    w.update(
        0.,
        &Controls {
            confirm: true,
            ..Default::default()
        },
        None,
        &mut world,
        Vector3::zero(),
    );
    assert_eq!(w.phase, Phase::Aborting);
    assert_eq!(world.travel_targets().len(), 1);
}

#[test]
fn even_extreme_mashing_requires_fifteen_seconds_before_the_question() {
    let (mut w, mut world, _) = setup();
    for _ in 0..1400 {
        w.update(
            0.01,
            &Controls {
                tap_count: 16,
                confirm: true,
                left: true,
                ..Default::default()
            },
            None,
            &mut world,
            Vector3::zero(),
        );
        assert_eq!(w.phase, Phase::Charging);
        assert!(
            w.charge <= TAP_GAIN + (1. - TAP_GAIN) * w.charge_time / MIN_CHARGE_SECONDS + 0.0001
        );
    }
    for _ in 0..110 {
        w.update(
            0.01,
            &Controls {
                tap_count: 16,
                ..Default::default()
            },
            None,
            &mut world,
            Vector3::zero(),
        );
        if w.phase == Phase::Confirm {
            break;
        }
    }
    assert_eq!(w.phase, Phase::Confirm);
    assert!(!w.launch_selected);
    assert_eq!(world.travel_targets().len(), 1);
}

#[test]
fn waiting_without_pressing_does_not_satisfy_the_fifteen_seconds() {
    let (mut w, mut world, _) = setup();
    w.update(20., &Controls::default(), None, &mut world, Vector3::zero());
    assert_eq!(w.charge_time, 0.);
    for _ in 0..1400 {
        w.update(
            0.01,
            &Controls {
                tap_count: 1,
                ..Default::default()
            },
            None,
            &mut world,
            Vector3::zero(),
        );
        assert_eq!(w.phase, Phase::Charging);
    }
}
#[test]
fn launch_follows_orbit_and_only_destroys_on_impact_then_restores() {
    let (mut w, mut world, t) = setup();
    w.phase = Phase::Confirm;
    w.charge = 1.;
    let origin = Vector3::new(0., 0., 50.);
    w.update(
        0.,
        &Controls {
            left: true,
            confirm: true,
            ..Default::default()
        },
        None,
        &mut world,
        origin,
    );
    assert_eq!(w.phase, Phase::Flight);
    world.update(&WorldUpdate { frame_time: 1. });
    w.update(1., &Controls::default(), None, &mut world, origin);
    assert!((w.target.unwrap().position - t.position).length() > 0.1);
    assert!(world.target_by_id(t.id).is_some());
    w.update(FLIGHT_TIME, &Controls::default(), None, &mut world, origin);
    assert_eq!(w.phase, Phase::Rupture);
    w.update(RUPTURE_TIME, &Controls::default(), None, &mut world, origin);
    assert_eq!(w.phase, Phase::Aftermath);
    assert!(world.target_by_id(t.id).is_none());
    world.restore_planets();
    assert_eq!(world.travel_targets()[0].id, t.id);
}
