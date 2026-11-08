use crate::world::World;
use crate::celestial_shaders::CelestialKind;
use raylib::prelude::Vector3;

pub mod celestial;

/// Register all objects in the world.
pub fn register_all(_world: &mut World) {
    let celestial_data = [
        // 1 - 5
        (CelestialKind::Star, Vector3::new(0.0, 0.0, 1_000.0)),
        (CelestialKind::Rocky, Vector3::new(-700.0, 300.0, 1_700.0)),
        (CelestialKind::GasGiant, Vector3::new(800.0, -200.0, 2_400.0)),
        (CelestialKind::Star, Vector3::new(-600.0, -500.0, 3_100.0)),
        (CelestialKind::Rocky, Vector3::new(750.0, 400.0, 3_800.0)),

        // 6 - 10
        (CelestialKind::GasGiant, Vector3::new(-850.0, 200.0, 4_500.0)),
        (CelestialKind::Star, Vector3::new(900.0, -450.0, 5_200.0)),
        (CelestialKind::Rocky, Vector3::new(-750.0, -350.0, 5_900.0)),
        (CelestialKind::GasGiant, Vector3::new(650.0, 500.0, 6_600.0)),
        (CelestialKind::Star, Vector3::new(-900.0, 100.0, 7_300.0)),

        // 11 - 15
        (CelestialKind::Rocky, Vector3::new(800.0, -600.0, 8_000.0)),
        (CelestialKind::GasGiant, Vector3::new(-700.0, 550.0, 8_700.0)),
        (CelestialKind::Star, Vector3::new(850.0, 250.0, 9_400.0)),
        (CelestialKind::Rocky, Vector3::new(-800.0, -400.0, 10_100.0)),
        (CelestialKind::GasGiant, Vector3::new(700.0, -300.0, 10_800.0)),

        // 16 - 20
        (CelestialKind::Star, Vector3::new(-950.0, 350.0, 11_500.0)),
        (CelestialKind::Rocky, Vector3::new(900.0, 600.0, 12_200.0)),
        (CelestialKind::GasGiant, Vector3::new(-650.0, -550.0, 12_900.0)),
        (CelestialKind::Star, Vector3::new(750.0, -150.0, 13_600.0)),
        (CelestialKind::Rocky, Vector3::new(-850.0, 450.0, 14_300.0)),

        // 21 - 25
        (CelestialKind::GasGiant, Vector3::new(950.0, 150.0, 15_000.0)),
        (CelestialKind::Star, Vector3::new(-700.0, -250.0, 15_700.0)),
        (CelestialKind::Rocky, Vector3::new(600.0, -650.0, 16_400.0)),
        (CelestialKind::GasGiant, Vector3::new(-900.0, 500.0, 17_100.0)),
        (CelestialKind::Star, Vector3::new(850.0, 350.0, 17_800.0)),

        // 26 - 30
        (CelestialKind::Rocky, Vector3::new(-600.0, -450.0, 18_500.0)),
        (CelestialKind::GasGiant, Vector3::new(700.0, 650.0, 19_200.0)),
        (CelestialKind::Star, Vector3::new(-800.0, 200.0, 19_900.0)),
        (CelestialKind::Rocky, Vector3::new(950.0, -350.0, 20_600.0)),
        (CelestialKind::GasGiant, Vector3::new(-750.0, -600.0, 21_300.0)),
    ];

    for (kind, position) in celestial_data {
        _world.add(celestial::CelestialBody::new(kind, position));
    }
}