use super::celestial::BodyShape;
use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use raylib::prelude::Vector3;

pub(super) const BODY_SCALE: f32 = 165.0;

pub(super) struct BodyDefinition {
    pub kind: CelestialKind,
    pub shape: BodyShape,
    pub position: Vector3,
    pub scale: f32,
    pub style: CelestialStyle,
    pub primary_light: bool,
    pub motion: MotionDefinition,
    pub atmosphere: Option<AtmosphereDefinition>,
    pub ring: Option<RingDefinition>,
    pub rock_field: Option<RockFieldDefinition>,
    pub rocks: Vec<RockDefinition>,
}

pub(super) enum MotionDefinition {
    Static,
    Orbit {
        center: Vector3,
        axis: Vector3,
        angular_speed: f32,
    },
    ParentOrbit {
        parent: usize,
        initial_offset: Vector3,
        axis: Vector3,
        angular_speed: f32,
    },
}

pub(super) struct AtmosphereDefinition {
    pub shell_scale: f32,
    pub color: Vector3,
    pub strength: f32,
    pub shape: BodyShape,
}

pub(super) struct RingDefinition {
    pub outer_scale: f32,
    pub tilt_x: f32,
    pub tilt_z: f32,
}

pub(super) struct RockDefinition {
    pub direction: Vector3,
    pub scale: Vector3,
    pub twist: f32,
}

pub(super) struct RockFieldDefinition {
    pub seed: u32,
    pub count: usize,
}

pub(super) fn solar_system_definitions() -> Vec<BodyDefinition> {
    let sun_position = Vector3::new(0.0, 0.0, 500.0);
    let rocky_position = Vector3::new(-300.0, 100.0, 800.0);
    let gas_position = Vector3::new(1550.0, -50.0, 3000.0);

    vec![
        BodyDefinition {
            kind: CelestialKind::Star,
            shape: BodyShape::Sphere,
            position: sun_position,
            scale: BODY_SCALE,
            style: CelestialStyle {
                noise_offset: Vector3::new(2.1, -0.7, 1.4),
                tint: Vector3::new(1.05, 0.96, 0.82),
                animation_speed: 1.15,
                ..CelestialStyle::default()
            },
            primary_light: true,
            motion: MotionDefinition::Static,
            atmosphere: Some(AtmosphereDefinition {
                shell_scale: 1.16,
                color: Vector3::new(1.0, 0.34, 0.04),
                strength: 0.1,
                shape: BodyShape::Sphere,
            }),
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: rocky_position,
            scale: BODY_SCALE,
            style: CelestialStyle {
                noise_offset: Vector3::new(-3.4, 1.8, 0.6),
                tint: Vector3::new(0.82, 1.0, 1.08),
                cloud_threshold: 0.64,
                rotation_speed: 0.055,
                ..CelestialStyle::default()
            },
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_position,
                axis: Vector3::new(0.12, 1.0, 0.08),
                angular_speed: 0.035,
            },
            atmosphere: Some(AtmosphereDefinition {
                shell_scale: 1.07,
                color: Vector3::new(0.16, 0.52, 1.0),
                strength: 0.1,
                shape: BodyShape::Sphere,
            }),
            ring: None,
            rock_field: Some(RockFieldDefinition {
                seed: 0x52A7_19D3,
                count: 36,
            }),
            rocks: surface_rocks(),
        },
        BodyDefinition {
            kind: CelestialKind::GasGiant,
            shape: BodyShape::Oblate,
            position: gas_position,
            scale: BODY_SCALE,
            style: CelestialStyle {
                noise_offset: Vector3::new(4.2, 0.9, -2.7),
                tint: Vector3::new(1.08, 0.88, 0.72),
                animation_speed: 0.75,
                rotation_speed: 0.11,
                ..CelestialStyle::default()
            },
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_position,
                axis: Vector3::new(-0.08, 1.0, 0.16),
                angular_speed: 0.018,
            },
            atmosphere: Some(AtmosphereDefinition {
                shell_scale: 1.05,
                color: Vector3::new(1.0, 0.58, 0.18),
                strength: 0.1,
                shape: BodyShape::Oblate,
            }),
            ring: Some(RingDefinition {
                outer_scale: 2.1,
                tilt_x: 0.34,
                tilt_z: -0.12,
            }),
            rock_field: None,
            rocks: Vec::new(),
        },
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: Vector3::zero(),
            scale: 58.0,
            style: CelestialStyle {
                noise_offset: Vector3::new(-1.6, 4.1, 2.3),
                tint: Vector3::new(0.72, 0.68, 0.62),
                cloud_threshold: 1.0,
                rotation_speed: 0.16,
                ..CelestialStyle::default()
            },
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 1,
                initial_offset: Vector3::new(360.0, -80.0, 40.0),
                axis: Vector3::new(0.15, 1.0, 0.08),
                angular_speed: 0.09,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
    ]
}

fn surface_rocks() -> Vec<RockDefinition> {
    let directions = [
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(-0.55, 0.20, -0.81),
    ];
    directions
        .into_iter()
        .enumerate()
        .map(|(index, direction)| {
            let size = 9.0 + (index % 3) as f32 * 3.5;
            RockDefinition {
                direction,
                scale: Vector3::new(size, size, size),
                twist: index as f32 * 0.73,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_one_primary_light_and_orders_parent_dependencies() {
        let definitions = solar_system_definitions();
        assert_eq!(
            definitions
                .iter()
                .filter(|definition| definition.primary_light)
                .count(),
            1
        );
        for (index, definition) in definitions.iter().enumerate() {
            if let MotionDefinition::ParentOrbit { parent, .. } = &definition.motion {
                assert!(*parent < index);
            }
        }
        assert!(
            definitions
                .iter()
                .any(|definition| !definition.rocks.is_empty())
        );
        assert!(
            definitions
                .iter()
                .any(|definition| definition.rock_field.is_some())
        );
        assert!(
            definitions
                .iter()
                .any(|definition| definition.ring.is_some())
        );
    }
}
