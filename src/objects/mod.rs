use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use crate::scene::world::{Orbit, World};
use raylib::prelude::Vector3;

pub mod atmosphere;
pub mod celestial;
pub mod ring;
pub mod rock;

/// Register all objects in the world.
pub fn register_all(_world: &mut World) {
    const BODY_SCALE: f32 = 165.0;
    let sun_position = Vector3::new(0.0, 0.0, 500.0);
    let rocky_position = Vector3::new(-300.0, 100.0, 800.0);
    let gas_position = Vector3::new(1550.0, -50.0, 3000.0);
    let rocky_orbit = Orbit::new(
        sun_position,
        rocky_position,
        Vector3::new(0.12, 1.0, 0.08),
        0.035,
    );
    let gas_orbit = Orbit::new(
        sun_position,
        gas_position,
        Vector3::new(-0.08, 1.0, 0.16),
        0.018,
    );

    let sun = celestial::CelestialBody::new_primary_star_styled(
        sun_position,
        CelestialStyle {
            noise_offset: Vector3::new(2.1, -0.7, 1.4),
            tint: Vector3::new(1.05, 0.96, 0.82),
            animation_speed: 1.15,
            ..CelestialStyle::default()
        },
    );
    let sun_anchor = sun.transform.anchor();
    let rocky = celestial::CelestialBody::styled(
        CelestialKind::Rocky,
        rocky_position,
        CelestialStyle {
            noise_offset: Vector3::new(-3.4, 1.8, 0.6),
            tint: Vector3::new(0.82, 1.0, 1.08),
            cloud_threshold: 0.64,
            rotation_speed: 0.055,
            ..CelestialStyle::default()
        },
    )
    .with_orbit(rocky_orbit);
    let rocky_anchor = rocky.transform.anchor();
    let gas = celestial::CelestialBody::styled(
        CelestialKind::GasGiant,
        gas_position,
        CelestialStyle {
            noise_offset: Vector3::new(4.2, 0.9, -2.7),
            tint: Vector3::new(1.08, 0.88, 0.72),
            animation_speed: 0.75,
            rotation_speed: 0.11,
            ..CelestialStyle::default()
        },
    )
    .with_orbit(gas_orbit);
    let gas_anchor = gas.transform.anchor();

    _world.add(sun);
    _world.add(rocky);
    _world.add(gas);
    let rock_directions = [
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(-0.55, 0.20, -0.81),
        Vector3::new(0.48, 0.42, -0.77),
        Vector3::new(-0.18, 0.90, -0.38),
        Vector3::new(-0.82, -0.30, -0.48),
        Vector3::new(0.74, -0.38, -0.55),
        Vector3::new(0.90, 0.12, 0.42),
        Vector3::new(-0.40, 0.55, 0.73),
    ];
    for (index, direction) in rock_directions.into_iter().enumerate() {
        let size = 9.0 + (index % 3) as f32 * 3.5;
        _world.add(rock::SurfaceRock::attached_to(
            rocky_anchor.clone(),
            direction,
            Vector3::new(size, size, size),
            index as f32 * 0.73,
        ));
    }
    _world.add(
        atmosphere::Atmosphere::new(
            sun_position,
            BODY_SCALE,
            1.16,
            Vector3::new(1.0, 0.34, 0.04),
            0.1,
        )
        .attached_to(sun_anchor, Vector3::zero()),
    );
    _world.add(
        atmosphere::Atmosphere::new(
            rocky_position,
            BODY_SCALE,
            1.07,
            Vector3::new(0.16, 0.52, 1.0),
            0.1,
        )
        .attached_to(rocky_anchor, Vector3::zero()),
    );
    _world.add(
        atmosphere::Atmosphere::new(
            gas_position,
            BODY_SCALE,
            1.05,
            Vector3::new(1.0, 0.58, 0.18),
            0.10,
        )
        .attached_to(gas_anchor.clone(), Vector3::zero()),
    );
    _world.add(
        ring::PlanetRing::new(gas_position, BODY_SCALE * 2.1, 0.34, -0.12)
            .attached_to(gas_anchor, Vector3::zero()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_system_exposes_the_sun_as_primary_light() {
        let mut world = World::new();
        register_all(&mut world);
        assert_eq!(
            world.primary_light_position(),
            Some(Vector3::new(0.0, 0.0, 500.0))
        );
    }
}
