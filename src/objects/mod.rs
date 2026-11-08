use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use crate::world::World;
use raylib::prelude::Vector3;

pub mod celestial;

/// Register all objects in the world.
pub fn register_all(_world: &mut World) {
    _world.add(celestial::CelestialBody::new_primary_star_styled(
        Vector3::new(0.0, 0.0, 500.0),
        CelestialStyle {
            noise_offset: Vector3::new(2.1, -0.7, 1.4),
            tint: Vector3::new(1.05, 0.96, 0.82),
            animation_speed: 1.15,
            ..CelestialStyle::default()
        },
    ));
    _world.add(celestial::CelestialBody::styled(
        CelestialKind::Rocky,
        Vector3::new(-300.0, 100.0, 800.0),
        CelestialStyle {
            noise_offset: Vector3::new(-3.4, 1.8, 0.6),
            tint: Vector3::new(0.82, 1.0, 1.08),
            cloud_threshold: 0.64,
            rotation_speed: 0.055,
            ..CelestialStyle::default()
        },
    ));
    _world.add(celestial::CelestialBody::styled(
        CelestialKind::GasGiant,
        Vector3::new(350.0, -50.0, 1_000.0),
        CelestialStyle {
            noise_offset: Vector3::new(4.2, 0.9, -2.7),
            tint: Vector3::new(1.08, 0.88, 0.72),
            animation_speed: 0.75,
            rotation_speed: 0.11,
            ..CelestialStyle::default()
        },
    ));
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
