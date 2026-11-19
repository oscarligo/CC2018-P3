mod surface;

use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use crate::objects::celestial::CelestialBody;
use crate::scene::world::{Orbit, World};
use raylib::prelude::Vector3;

const RADIUS: f32 = 140.0;
const ORBITAL_SPEED: f32 = 0.022;
const ROTATION_SPEED: f32 = 0.12;
// A la derecha de la vista inicial, en un plano separado del de Kashyyyk.
const INITIAL_OFFSET: Vector3 = Vector3 {
    x: 630.0,
    y: -220.0,
    z: -370.0,
};

pub(super) fn registrar(world: &mut World) {
    let sun_position = world
        .primary_light_position()
        .expect("Coruscant requiere registrar primero el sol del sistema");
    surface::prepare();
    let position = sun_position + INITIAL_OFFSET;
    let planet = CelestialBody::styled(
        CelestialKind::Rocky,
        position,
        CelestialStyle {
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
    ))
    .with_fragment_shader(surface::shade);

    world.add(planet);
}
