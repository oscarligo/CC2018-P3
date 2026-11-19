mod benchmark;
mod detail;
mod forest;
mod surface;
mod village;

use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use crate::objects::celestial::CelestialBody;
use crate::scene::world::{Orbit, World};
use raylib::prelude::Vector3;

const RADIUS: f32 = 120.0;
// Radianes por segundo; ambos movimientos usan WorldUpdate::frame_time.
const ORBITAL_SPEED: f32 = 0.025;
const ROTATION_SPEED: f32 = 0.08;
// Delante y a la izquierda de la nave, con luz lateral visible al iniciar.
const INITIAL_OFFSET: Vector3 = Vector3 {
    x: -600.0,
    y: 180.0,
    z: -300.0,
};

pub(super) fn benchmark(captures: bool) -> Result<(), Box<dyn std::error::Error>> {
    benchmark::run(captures)
}

pub(super) fn registrar(world: &mut World) {
    register_with_lod(world, true);
}

fn register_with_lod(world: &mut World, lod: bool) {
    surface::prepare();
    let sun_position = world
        .primary_light_position()
        .expect("Kashyyyk requiere registrar primero el sol del sistema");
    let position = sun_position + INITIAL_OFFSET;
    let planet = CelestialBody::styled(
        CelestialKind::Rocky,
        position,
        CelestialStyle {
            rotation_speed: ROTATION_SPEED,
            ..CelestialStyle::default()
        },
    )
    .named("Kashyyyk")
    .with_fragment_shader(surface::shade)
    .with_scale(RADIUS)
    .with_orbit(Orbit::new(
        sun_position,
        position,
        Vector3::new(0.0, 1.0, 0.0),
        ORBITAL_SPEED,
    ));

    let anchor = planet.transform.anchor();
    world.add(planet);
    forest::register(world, anchor.clone(), RADIUS, lod);
    village::register(world, anchor, RADIUS, lod);
}
