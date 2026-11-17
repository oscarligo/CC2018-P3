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

pub(super) fn registrar(world: &mut World) {
    let sun_position = world
        .primary_light_position()
        .expect("Kashyyyk requiere registrar primero el sol del sistema");
    let position = sun_position + INITIAL_OFFSET;
    let planet = CelestialBody::styled(
        CelestialKind::Rocky,
        position,
        CelestialStyle {
            // Apariencia verde provisional. El shader forestal va en el bloque 2.
            tint: Vector3::new(0.28, 1.65, 0.22),
            cloud_threshold: 1.1,
            animation_speed: 0.0,
            rotation_speed: ROTATION_SPEED,
            ..CelestialStyle::default()
        },
    )
    .with_scale(RADIUS)
    .with_orbit(Orbit::new(
        sun_position,
        position,
        Vector3::new(0.0, 1.0, 0.0),
        ORBITAL_SPEED,
    ));

    world.add(planet);
}
