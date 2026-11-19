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
    .with_fragment_shader(shade_placeholder);

    world.add(planet);
}

fn shade_placeholder(point: Vector3, normal: Vector3, _time: f32, light: Vector3) -> Vector3 {
    let daylight = normal.dot(light).max(0.0);
    let surface = Vector3::new(0.16, 0.22, 0.32) * (0.32 + 0.68 * daylight);

    // La marca temporal usa coordenadas locales para que se vea girar con el planeta.
    let marker_direction = Vector3::new(-0.48, 0.16, -0.86);
    let marker = ((point.dot(marker_direction) - 0.97) / 0.012).clamp(0.0, 1.0);
    let amber = Vector3::new(0.96, 0.63, 0.27);
    surface + (amber - surface) * marker
}
