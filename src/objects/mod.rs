use crate::world::World;

pub mod celestial;

/// Register all objects in the world.
pub fn register_all(_world: &mut World) {
    _world.add(celestial::CelestialBody::new(
        crate::celestial_shaders::CelestialKind::Star,
        raylib::prelude::Vector3::new(0.0, 0.0, 500.0),
    ));
    _world.add(celestial::CelestialBody::new(
        crate::celestial_shaders::CelestialKind::Rocky,
        raylib::prelude::Vector3::new(-300.0, 100.0, 800.0),
    ));
    _world.add(celestial::CelestialBody::new(
        crate::celestial_shaders::CelestialKind::GasGiant,
        raylib::prelude::Vector3::new(350.0, -50.0, 1_000.0),
    ));
}
