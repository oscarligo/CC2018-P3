use super::celestial::BodyShape;
use crate::celestial_shaders::{CelestialKind, CelestialStyle};
use raylib::prelude::Vector3;

pub(super) const BODY_SCALE: f32 = 165.0;
// Multiplicador global de distancia orbital (1.0 = normal, 0.5 = más juntos, 2.0 = más separados)
pub(super) const ORBIT_SPREAD: f32 = 1.0;

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

/// Calcula una posición inicial en 360° escalada por ORBIT_SPREAD.
fn orbit_pos(center: Vector3, radius: f32, angle_deg: f32, elevation: f32) -> Vector3 {
    let rad = angle_deg.to_radians();
    let r = radius * ORBIT_SPREAD;
    Vector3::new(
        center.x + r * rad.cos(),
        center.y + (elevation * ORBIT_SPREAD),
        center.z + r * rad.sin(),
    )
}

/// Calcula el desplazamiento inicial de una luna.
fn moon_offset(radius: f32, angle_deg: f32, elevation: f32) -> Vector3 {
    let rad = angle_deg.to_radians();
    Vector3::new(radius * rad.cos(), elevation, radius * rad.sin())
}

fn style(
    noise_offset: Vector3,
    tint: Vector3,
    cloud_threshold: f32,
    animation_speed: f32,
    rotation_speed: f32,
) -> CelestialStyle {
    CelestialStyle {
        noise_offset,
        tint,
        cloud_threshold,
        animation_speed,
        rotation_speed,
    }
}

fn atmo(
    shell_scale: f32,
    color: Vector3,
    strength: f32,
    shape: BodyShape,
) -> Option<AtmosphereDefinition> {
    Some(AtmosphereDefinition {
        shell_scale,
        color,
        strength,
        shape,
    })
}

fn ring(outer_scale: f32, tilt_x: f32, tilt_z: f32) -> Option<RingDefinition> {
    Some(RingDefinition {
        outer_scale,
        tilt_x,
        tilt_z,
    })
}

fn field(seed: u32, count: usize) -> Option<RockFieldDefinition> {
    Some(RockFieldDefinition { seed, count })
}

pub(super) fn solar_system_definitions() -> Vec<BodyDefinition> {
    let sun_pos = Vector3::new(0.0, 0.0, 1500.0);

    vec![
        // 0: Estrella Amarilla Principal
        BodyDefinition {
            kind: CelestialKind::Star,
            shape: BodyShape::Sphere,
            position: sun_pos,
            scale: BODY_SCALE * 2.2,
            style: style(
                Vector3::new(2.1, -0.7, 1.4),
                Vector3::new(1.05, 0.96, 0.82),
                1.0,
                1.15,
                0.02,
            ),
            primary_light: true,
            motion: MotionDefinition::Static,
            atmosphere: atmo(1.18, Vector3::new(1.0, 0.34, 0.04), 0.14, BodyShape::Sphere),
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        // 1: Protoplaneta (Arriba y a la derecha)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: orbit_pos(sun_pos, 700.0, 18.0, 600.0),
            scale: BODY_SCALE * 0.55,
            style: style(
                Vector3::new(1.1, 3.2, -0.8),
                Vector3::new(0.75, 0.70, 0.68),
                1.0,
                0.2,
                0.025,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.8, 0.6, -0.4),
                angular_speed: 0.045,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(0),
        },
        // 2: Magmático (Abajo, muy sumergido)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: orbit_pos(sun_pos, 900.0, 142.0, -1100.0),
            scale: BODY_SCALE * 0.78,
            style: style(
                Vector3::new(-2.8, 0.9, 4.1),
                Vector3::new(1.25, 0.48, 0.22),
                0.48,
                0.95,
                0.04,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-0.5, 0.7, 0.9),
                angular_speed: 0.036,
            },
            atmosphere: atmo(1.09, Vector3::new(1.0, 0.25, 0.05), 0.16, BodyShape::Sphere),
            ring: None,
            rock_field: field(0x11A4_88C1, 24),
            rocks: surface_rocks_preset(1),
        },
        // 3: Invernadero (Alto en Y)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: orbit_pos(sun_pos, 1200.0, 255.0, 1400.0),
            scale: BODY_SCALE * 0.92,
            style: style(
                Vector3::new(0.4, -4.2, 2.7),
                Vector3::new(1.12, 1.02, 0.62),
                0.32,
                0.65,
                -0.018,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(1.0, 0.2, 0.5),
                angular_speed: 0.029,
            },
            atmosphere: atmo(
                1.13,
                Vector3::new(0.95, 0.82, 0.30),
                0.15,
                BodyShape::Sphere,
            ),
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        // 4: Oceánico (Profundo en -Y)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: orbit_pos(sun_pos, 1800.0, 320.0, -1600.0),
            scale: BODY_SCALE,
            style: style(
                Vector3::new(-3.4, 1.8, 0.6),
                Vector3::new(0.82, 1.0, 1.08),
                0.64,
                0.5,
                0.055,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-0.7, 0.8, -0.6),
                angular_speed: 0.024,
            },
            atmosphere: atmo(1.07, Vector3::new(0.16, 0.52, 1.0), 0.11, BodyShape::Sphere),
            ring: None,
            rock_field: field(0x52A7_19D3, 36),
            rocks: surface_rocks_preset(0),
        },
        // 5: Luna de #4
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: Vector3::zero(),
            scale: 52.0,
            style: style(
                Vector3::new(-1.6, 4.1, 2.3),
                Vector3::new(0.76, 0.74, 0.70),
                1.0,
                0.0,
                0.14,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 4,
                initial_offset: moon_offset(220.0, 45.0, -80.0),
                axis: Vector3::new(0.15, 1.0, 0.08),
                angular_speed: 0.085,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(2),
        },
        // 6: Desierto con Anillo (Zenit)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: orbit_pos(sun_pos, 2200.0, 78.0, 2400.0),
            scale: BODY_SCALE * 0.74,
            style: style(
                Vector3::new(5.1, -1.3, -2.0),
                Vector3::new(1.18, 0.62, 0.42),
                0.78,
                0.35,
                0.052,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.4, 0.9, -0.8),
                angular_speed: 0.020,
            },
            atmosphere: atmo(
                1.05,
                Vector3::new(0.92, 0.45, 0.24),
                0.08,
                BodyShape::Sphere,
            ),
            ring: ring(1.55, 0.22, -0.08),
            rock_field: None,
            rocks: surface_rocks_preset(1),
        },
        // 7 & 8: Lunas de #6
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: Vector3::zero(),
            scale: 34.0,
            style: style(
                Vector3::new(2.2, 1.1, -3.5),
                Vector3::new(0.62, 0.54, 0.48),
                1.0,
                0.0,
                0.22,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 6,
                initial_offset: moon_offset(150.0, 130.0, 45.0),
                axis: Vector3::new(0.08, 1.0, -0.12),
                angular_speed: 0.12,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: Vector3::zero(),
            scale: 24.0,
            style: style(
                Vector3::new(-4.0, 2.5, 1.7),
                Vector3::new(0.68, 0.64, 0.58),
                1.0,
                0.0,
                0.18,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 6,
                initial_offset: moon_offset(210.0, 295.0, -65.0),
                axis: Vector3::new(-0.10, 1.0, 0.05),
                angular_speed: 0.075,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        // 9: Planeta Enano (Nadir)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: orbit_pos(sun_pos, 2800.0, 198.0, -2900.0),
            scale: BODY_SCALE * 0.48,
            style: style(
                Vector3::new(-0.9, -3.1, 4.4),
                Vector3::new(0.58, 0.60, 0.65),
                0.92,
                0.15,
                0.12,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.9, 0.2, 0.4),
                angular_speed: 0.017,
            },
            atmosphere: None,
            ring: None,
            rock_field: field(0x88F1_309A, 64),
            rocks: surface_rocks_preset(2),
        },
        // 10: Supertierra (Elevado)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Oblate,
            position: orbit_pos(sun_pos, 3500.0, 348.0, 3200.0),
            scale: BODY_SCALE * 1.32,
            style: style(
                Vector3::new(3.7, 2.4, -1.9),
                Vector3::new(0.56, 1.15, 0.72),
                0.52,
                0.60,
                0.085,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-0.3, 0.8, 1.0),
                angular_speed: 0.015,
            },
            atmosphere: atmo(
                1.08,
                Vector3::new(0.22, 0.92, 0.58),
                0.13,
                BodyShape::Oblate,
            ),
            ring: ring(1.68, -0.18, 0.24),
            rock_field: None,
            rocks: surface_rocks_preset(0),
        },
        // 11: Luna de #10
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: Vector3::zero(),
            scale: 64.0,
            style: style(
                Vector3::new(1.5, -2.2, 3.0),
                Vector3::new(0.65, 0.95, 1.15),
                0.58,
                0.45,
                0.07,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 10,
                initial_offset: moon_offset(300.0, 210.0, 90.0),
                axis: Vector3::new(0.06, 1.0, 0.12),
                angular_speed: 0.065,
            },
            atmosphere: atmo(1.06, Vector3::new(0.30, 0.75, 1.0), 0.09, BodyShape::Sphere),
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        // 12: Mundo Amatista (Muy hundido)
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: orbit_pos(sun_pos, 4200.0, 112.0, -4500.0),
            scale: BODY_SCALE * 0.88,
            style: style(
                Vector3::new(-5.2, 0.7, -3.3),
                Vector3::new(1.05, 0.62, 1.22),
                0.74,
                0.40,
                0.065,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(1.0, 0.1, -0.5),
                angular_speed: 0.013,
            },
            atmosphere: atmo(1.06, Vector3::new(0.78, 0.28, 1.0), 0.10, BodyShape::Sphere),
            ring: None,
            rock_field: field(0x3C90_7B2E, 30),
            rocks: surface_rocks_preset(3),
        },
        // 13: Planeta Árido
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: orbit_pos(sun_pos, 5000.0, 232.0, 5200.0),
            scale: BODY_SCALE * 0.84,
            style: style(
                Vector3::new(2.9, -3.8, 1.2),
                Vector3::new(1.12, 1.14, 1.20),
                0.84,
                0.25,
                0.048,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-0.8, 0.5, 0.3),
                angular_speed: 0.0115,
            },
            atmosphere: atmo(1.04, Vector3::new(0.75, 0.88, 1.0), 0.07, BodyShape::Sphere),
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(1),
        },
        // 14: Coloso Joviano (Hacia abajo)
        BodyDefinition {
            kind: CelestialKind::GasGiant,
            shape: BodyShape::Oblate,
            position: orbit_pos(sun_pos, 6000.0, 42.0, -5800.0),
            scale: BODY_SCALE * 1.85,
            style: style(
                Vector3::new(4.2, 0.9, -2.7),
                Vector3::new(1.08, 0.88, 0.72),
                0.50,
                0.85,
                0.12,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.5, 0.6, 0.9),
                angular_speed: 0.0095,
            },
            atmosphere: atmo(1.05, Vector3::new(1.0, 0.58, 0.18), 0.11, BodyShape::Oblate),
            ring: ring(1.75, 0.25, -0.10),
            rock_field: field(0x9A44_E102, 45),
            rocks: Vec::new(),
        },
        // 15, 16, 17: Lunas del Coloso
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: Vector3::zero(),
            scale: 56.0,
            style: style(
                Vector3::new(-2.1, 3.4, -1.5),
                Vector3::new(1.22, 0.96, 0.28),
                0.68,
                0.80,
                0.11,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 14,
                initial_offset: moon_offset(320.0, 25.0, -45.0),
                axis: Vector3::new(-0.05, 1.0, 0.12),
                angular_speed: 0.095,
            },
            atmosphere: atmo(1.06, Vector3::new(1.0, 0.78, 0.12), 0.08, BodyShape::Sphere),
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(1),
        },
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: Vector3::zero(),
            scale: 52.0,
            style: style(
                Vector3::new(3.3, -1.7, 2.8),
                Vector3::new(0.86, 1.05, 1.18),
                0.90,
                0.20,
                0.08,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 14,
                initial_offset: moon_offset(450.0, 155.0, 75.0),
                axis: Vector3::new(-0.08, 1.0, 0.15),
                angular_speed: 0.068,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: Vector3::zero(),
            scale: 76.0,
            style: style(
                Vector3::new(0.8, 2.6, -4.0),
                Vector3::new(0.82, 0.78, 0.74),
                0.85,
                0.15,
                0.06,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 14,
                initial_offset: moon_offset(580.0, 275.0, -110.0),
                axis: Vector3::new(-0.06, 1.0, 0.14),
                angular_speed: 0.045,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(2),
        },
        // 18: Gigante Anillado Imperial (Hacia arriba)
        BodyDefinition {
            kind: CelestialKind::GasGiant,
            shape: BodyShape::Oblate,
            position: orbit_pos(sun_pos, 7200.0, 168.0, 7000.0),
            scale: BODY_SCALE * 1.65,
            style: style(
                Vector3::new(-3.8, -1.4, 3.2),
                Vector3::new(1.14, 1.04, 0.80),
                0.42,
                0.70,
                0.105,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-0.6, 0.7, -0.4),
                angular_speed: 0.008,
            },
            atmosphere: atmo(1.05, Vector3::new(1.0, 0.84, 0.45), 0.10, BodyShape::Oblate),
            ring: ring(2.65, 0.42, -0.18),
            rock_field: None,
            rocks: Vec::new(),
        },
        // 19, 20: Lunas de #18
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: Vector3::zero(),
            scale: 72.0,
            style: style(
                Vector3::new(1.9, -3.5, -0.7),
                Vector3::new(1.10, 0.78, 0.38),
                0.30,
                0.55,
                0.05,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 18,
                initial_offset: moon_offset(480.0, 85.0, -120.0),
                axis: Vector3::new(0.14, 1.0, -0.08),
                angular_speed: 0.055,
            },
            atmosphere: atmo(1.14, Vector3::new(1.0, 0.62, 0.16), 0.16, BodyShape::Sphere),
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: Vector3::zero(),
            scale: 44.0,
            style: style(
                Vector3::new(-4.5, 1.2, 2.1),
                Vector3::new(0.95, 1.08, 1.22),
                0.75,
                0.40,
                0.09,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 18,
                initial_offset: moon_offset(360.0, 240.0, 80.0),
                axis: Vector3::new(0.10, 1.0, -0.12),
                angular_speed: 0.082,
            },
            atmosphere: atmo(1.08, Vector3::new(0.65, 0.90, 1.0), 0.09, BodyShape::Sphere),
            ring: None,
            rock_field: field(0x77D2_4F19, 28),
            rocks: Vec::new(),
        },
        // 21: Enana Marrón (Sur Profundo)
        BodyDefinition {
            kind: CelestialKind::Star,
            shape: BodyShape::Oblate,
            position: orbit_pos(sun_pos, 8500.0, 288.0, -8200.0),
            scale: BODY_SCALE * 1.38,
            style: style(
                Vector3::new(5.4, 2.8, -3.9),
                Vector3::new(1.22, 0.38, 0.45),
                0.40,
                1.40,
                0.15,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.8, 0.3, 1.0),
                angular_speed: 0.0068,
            },
            atmosphere: atmo(1.12, Vector3::new(1.0, 0.18, 0.28), 0.14, BodyShape::Oblate),
            ring: ring(1.90, -0.35, 0.28),
            rock_field: None,
            rocks: Vec::new(),
        },
        // 22: Luna de #21
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: Vector3::zero(),
            scale: 48.0,
            style: style(
                Vector3::new(-1.2, -2.9, 3.8),
                Vector3::new(0.55, 0.35, 0.32),
                1.0,
                0.30,
                0.13,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 21,
                initial_offset: moon_offset(280.0, 330.0, 45.0),
                axis: Vector3::new(-0.12, 1.0, 0.09),
                angular_speed: 0.105,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(3),
        },
        // 23: Gigante Cian (Norte Profundo)
        BodyDefinition {
            kind: CelestialKind::GasGiant,
            shape: BodyShape::Oblate,
            position: orbit_pos(sun_pos, 9800.0, 6.0, 9500.0),
            scale: BODY_SCALE * 1.28,
            style: style(
                Vector3::new(-2.4, 4.0, 1.5),
                Vector3::new(0.64, 1.08, 1.18),
                0.55,
                0.60,
                0.095,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-0.9, 0.4, -0.6),
                angular_speed: 0.0055,
            },
            atmosphere: atmo(1.06, Vector3::new(0.32, 0.90, 1.0), 0.11, BodyShape::Oblate),
            ring: ring(1.95, 1.35, 0.25),
            rock_field: None,
            rocks: Vec::new(),
        },
        // 24: Luna de #23
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Asteroid,
            position: Vector3::zero(),
            scale: 44.0,
            style: style(
                Vector3::new(3.1, 0.5, -2.6),
                Vector3::new(0.78, 0.86, 0.92),
                1.0,
                0.0,
                0.16,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 23,
                initial_offset: moon_offset(280.0, 115.0, -110.0),
                axis: Vector3::new(0.25, 1.0, -0.15),
                angular_speed: 0.078,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(3),
        },
        // 25: Gigante Cobalto
        BodyDefinition {
            kind: CelestialKind::GasGiant,
            shape: BodyShape::Oblate,
            position: orbit_pos(sun_pos, 11200.0, 126.0, -10500.0),
            scale: BODY_SCALE * 1.24,
            style: style(
                Vector3::new(1.7, -4.6, -1.8),
                Vector3::new(0.38, 0.62, 1.25),
                0.46,
                1.35,
                0.14,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.7, 0.8, -0.2),
                angular_speed: 0.0046,
            },
            atmosphere: atmo(1.07, Vector3::new(0.18, 0.42, 1.0), 0.13, BodyShape::Oblate),
            ring: None,
            rock_field: field(0x61B8_C407, 32),
            rocks: Vec::new(),
        },
        // 26: Luna Retrógrada de #25
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: Vector3::zero(),
            scale: 60.0,
            style: style(
                Vector3::new(-3.6, 2.1, 3.9),
                Vector3::new(0.92, 0.84, 0.90),
                0.72,
                0.30,
                -0.065,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 25,
                initial_offset: moon_offset(320.0, 205.0, 85.0),
                axis: Vector3::new(-0.14, 1.0, 0.10),
                angular_speed: -0.058,
            },
            atmosphere: atmo(1.05, Vector3::new(0.70, 0.82, 1.0), 0.07, BodyShape::Sphere),
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(0),
        },
        // 27: Helado Exterior
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Sphere,
            position: orbit_pos(sun_pos, 12600.0, 216.0, 11500.0),
            scale: BODY_SCALE * 0.58,
            style: style(
                Vector3::new(4.8, -0.6, 2.2),
                Vector3::new(0.96, 0.82, 0.76),
                0.80,
                0.20,
                0.032,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(-1.0, 0.3, 0.8),
                angular_speed: 0.0038,
            },
            atmosphere: atmo(
                1.08,
                Vector3::new(0.62, 0.78, 0.95),
                0.06,
                BodyShape::Sphere,
            ),
            ring: None,
            rock_field: None,
            rocks: surface_rocks_preset(1),
        },
        // 28: Luna Binaria de #27
        BodyDefinition {
            kind: CelestialKind::Rocky,
            shape: BodyShape::Rocky,
            position: Vector3::zero(),
            scale: 50.0,
            style: style(
                Vector3::new(-2.0, -1.5, -3.1),
                Vector3::new(0.72, 0.70, 0.74),
                1.0,
                0.0,
                0.045,
            ),
            primary_light: false,
            motion: MotionDefinition::ParentOrbit {
                parent: 27,
                initial_offset: moon_offset(180.0, 60.0, 45.0),
                axis: Vector3::new(0.28, 1.0, -0.18),
                angular_speed: 0.048,
            },
            atmosphere: None,
            ring: None,
            rock_field: None,
            rocks: Vec::new(),
        },
        // 29: Gigante Lejano
        BodyDefinition {
            kind: CelestialKind::GasGiant,
            shape: BodyShape::Sphere,
            position: orbit_pos(sun_pos, 14000.0, 306.0, -12500.0),
            scale: BODY_SCALE * 1.12,
            style: style(
                Vector3::new(-5.5, 3.7, -4.2),
                Vector3::new(0.52, 0.44, 0.98),
                0.38,
                0.90,
                0.075,
            ),
            primary_light: false,
            motion: MotionDefinition::Orbit {
                center: sun_pos,
                axis: Vector3::new(0.6, 0.9, -0.7),
                angular_speed: 0.0030,
            },
            atmosphere: atmo(
                1.09,
                Vector3::new(0.38, 0.25, 0.95),
                0.12,
                BodyShape::Sphere,
            ),
            ring: ring(2.30, -0.52, 0.34),
            rock_field: field(0xE409_55B2, 52),
            rocks: Vec::new(),
        },
    ]
}

fn surface_rocks_preset(preset: usize) -> Vec<RockDefinition> {
    let layouts: [&[Vector3]; 4] = [
        &[
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(-0.55, 0.20, -0.81),
        ],
        &[
            Vector3::new(0.58, 0.58, 0.57),
            Vector3::new(-0.42, -0.71, 0.56),
            Vector3::new(0.18, -0.88, -0.44),
        ],
        &[
            Vector3::new(-0.85, 0.12, 0.51),
            Vector3::new(0.72, -0.38, -0.58),
            Vector3::new(0.10, 0.95, -0.29),
            Vector3::new(-0.33, -0.64, -0.69),
        ],
        &[
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(0.89, 0.0, 0.45),
        ],
    ];
    let directions = layouts[preset % layouts.len()];
    let base_size = 7.5 + (preset as f32 * 2.0);

    directions
        .iter()
        .copied()
        .enumerate()
        .map(|(index, direction)| {
            let size = base_size + (index % 3) as f32 * 3.5;
            RockDefinition {
                direction,
                scale: Vector3::new(size, size * 0.85, size * 1.15),
                twist: (index as f32 + preset as f32) * 0.73,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_one_primary_light_and_orders_parent_dependencies() {
        let definitions = solar_system_definitions();
        assert_eq!(definitions.len(), 30);
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

        let moon_parents: HashSet<_> = definitions
            .iter()
            .filter_map(|definition| match &definition.motion {
                MotionDefinition::ParentOrbit { parent, .. } => Some(*parent),
                MotionDefinition::Static | MotionDefinition::Orbit { .. } => None,
            })
            .collect();
        assert!(moon_parents.len() >= 5);

        let sun = definitions[0].position;
        let mut quadrants = [false; 4];
        for def in &definitions[1..] {
            if matches!(def.motion, MotionDefinition::Orbit { .. }) {
                let dx = def.position.x - sun.x;
                let dz = def.position.z - sun.z;
                let q = match (dx >= 0.0, dz >= 0.0) {
                    (true, true) => 0,
                    (false, true) => 1,
                    (false, false) => 2,
                    (true, false) => 3,
                };
                quadrants[q] = true;
            }
        }
        assert!(quadrants.into_iter().all(|present| present));
    }
}
