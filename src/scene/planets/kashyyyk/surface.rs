//! Superficie CPU de Kashyyyk. Las coordenadas locales mantienen el terreno
//! unido al planeta; solo el campo de nubes se desplaza con el tiempo.
use crate::noise::{fbm, smoothstep};
use raylib::prelude::Vector3;
use std::sync::OnceLock;

const SEA_LEVEL: f32 = 0.45;
const CLOUD_SPEED: f32 = 0.018;
const GROUND_GRID: usize = 65;
const CLOUD_GRID: usize = 32;

struct NoiseFields {
    ground: Vec<Vector3>,
    clouds: Vec<f32>,
}

fn fields() -> &'static NoiseFields {
    static FIELDS: OnceLock<NoiseFields> = OnceLock::new();
    FIELDS.get_or_init(|| {
        let mut ground = Vec::with_capacity(GROUND_GRID.pow(3));
        for z in 0..GROUND_GRID {
            for y in 0..GROUND_GRID {
                for x in 0..GROUND_GRID {
                    ground.push(ground_color(
                        Vector3::new(x as f32, y as f32, z as f32) * (1.0 / 32.0)
                            - Vector3::new(1.0, 1.0, 1.0),
                    ));
                }
            }
        }
        let mut clouds = Vec::with_capacity(CLOUD_GRID.pow(3));
        for z in 0..CLOUD_GRID {
            for y in 0..CLOUD_GRID {
                for x in 0..CLOUD_GRID {
                    clouds.push(fbm(Vector3::new(x as f32, y as f32, z as f32) * 0.25, 2));
                }
            }
        }
        NoiseFields { ground, clouds }
    })
}

pub(super) fn prepare() {
    fields();
}

/// Interpolación trilineal de campos CPU. Las nubes envuelven sus índices;
/// el terreno usa un volumen continuo que cubre la esfera sin costuras UV.
trait FieldValue: Copy {
    fn blend(a: Self, b: Self, t: f32) -> Self;
}

impl FieldValue for f32 {
    fn blend(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl FieldValue for Vector3 {
    fn blend(a: Self, b: Self, t: f32) -> Self {
        Vector3::new(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t,
        )
    }
}

fn sample<T: FieldValue>(data: &[T], size: usize, point: Vector3, periodic: bool) -> T {
    let axes = [point.x, point.y, point.z].map(|value| {
        let value = if periodic {
            value.rem_euclid(size as f32)
        } else {
            value.clamp(0.0, (size - 1) as f32)
        };
        let first = value.floor() as usize;
        let next = if periodic {
            (first + 1) % size
        } else {
            (first + 1).min(size - 1)
        };
        (first, next, value - first as f32)
    });
    let at = |x: usize, y: usize, z: usize| data[z * size * size + y * size + x];
    let lerp = T::blend;
    let [(x0, x1, x), (y0, y1, y), (z0, z1, z)] = axes;
    lerp(
        lerp(
            lerp(at(x0, y0, z0), at(x1, y0, z0), x),
            lerp(at(x0, y1, z0), at(x1, y1, z0), x),
            y,
        ),
        lerp(
            lerp(at(x0, y0, z1), at(x1, y0, z1), x),
            lerp(at(x0, y1, z1), at(x1, y1, z1), x),
            y,
        ),
        z,
    )
}

fn mix(a: Vector3, b: Vector3, amount: f32) -> Vector3 {
    Vector3::blend(a, b, amount.clamp(0.0, 1.0))
}

fn elevation(point: Vector3) -> f32 {
    fbm(point * 3.3 + Vector3::new(2.4, -1.7, 0.8), 3)
}

fn vegetation_density(point: Vector3) -> f32 {
    fbm(point * 8.5 + Vector3::new(-3.0, 0.5, 1.2), 2)
}

/// Mismo terreno que el shader: sin árboles en agua, costas ni roca alta.
pub(super) fn forest_weight(point: Vector3) -> f32 {
    let height = elevation(point);
    if !(SEA_LEVEL + 0.055..0.62).contains(&height) {
        return 0.0;
    }
    smoothstep(0.25, 0.72, vegetation_density(point))
}

fn cloud_cover(point: Vector3, time: f32) -> f32 {
    let noise = sample(
        &fields().clouds,
        CLOUD_GRID,
        Vector3::new(
            (point.x * 5.2 + time * CLOUD_SPEED) * 4.0,
            (point.y * 5.2 + time * 0.004) * 4.0,
            (point.z * 5.2 - time * 0.011) * 4.0,
        ),
        true,
    );
    smoothstep(0.59, 0.76, noise) * 0.72
}

fn ground_color(point: Vector3) -> Vector3 {
    let height = elevation(point);
    let detail = fbm(point * 23.0, 2);
    let density = vegetation_density(point);
    let forest = mix(
        Vector3::new(0.045, 0.16, 0.035),
        Vector3::new(0.20, 0.43, 0.075),
        smoothstep(0.25, 0.72, density),
    ) * (0.8 + detail * 0.4);
    let rock = mix(
        Vector3::new(0.28, 0.25, 0.17),
        Vector3::new(0.58, 0.57, 0.48),
        detail,
    );
    let land = mix(forest, rock, smoothstep(0.62, 0.76, height));
    let coast = mix(
        Vector3::new(0.46, 0.43, 0.22),
        land,
        smoothstep(SEA_LEVEL + 0.015, SEA_LEVEL + 0.055, height),
    );
    let water = mix(
        Vector3::new(0.012, 0.065, 0.13),
        Vector3::new(0.025, 0.27, 0.29),
        smoothstep(SEA_LEVEL - 0.10, SEA_LEVEL, height),
    );
    mix(
        water,
        coast,
        smoothstep(SEA_LEVEL - 0.008, SEA_LEVEL + 0.012, height),
    )
}

pub(super) fn shade(
    point: Vector3,
    normal: Vector3,
    time: f32,
    light_direction: Vector3,
) -> Vector3 {
    let ground = sample(
        &fields().ground,
        GROUND_GRID,
        Vector3::new(
            point.x * 32.0 + 32.0,
            point.y * 32.0 + 32.0,
            point.z * 32.0 + 32.0,
        ),
        false,
    );
    let color = mix(
        ground,
        Vector3::new(0.83, 0.89, 0.86),
        cloud_cover(point, time),
    );
    let diffuse = (normal.x * light_direction.x
        + normal.y * light_direction.y
        + normal.z * light_direction.z)
        .max(0.0)
        .sqrt();
    let intensity = 0.08 + diffuse * 0.92;
    Vector3::new(
        color.x * intensity,
        color.y * intensity,
        color.z * intensity,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_fields_interpolate_and_clouds_wrap_continuously() {
        let data = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        assert_eq!(sample(&data, 2, Vector3::new(0.5, 0.5, 0.5), false), 3.5);
        let a = sample(
            &fields().clouds,
            CLOUD_GRID,
            Vector3::new(-0.2, 3.4, 2.1),
            true,
        );
        let b = sample(
            &fields().clouds,
            CLOUD_GRID,
            Vector3::new(31.8, 3.4, 2.1),
            true,
        );
        assert!((a - b).abs() < 0.00001);
        for point in [
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(-0.5, 0.25, 0.75),
            Vector3::new(1.0, 0.0, 0.0),
        ] {
            let cached = sample(
                &fields().ground,
                GROUND_GRID,
                point * 32.0 + Vector3::new(32.0, 32.0, 32.0),
                false,
            );
            assert!((cached - ground_color(point)).length() < 0.00001);
        }
    }

    #[test]
    fn terrain_is_reproducible_and_contains_land_and_water() {
        let mut land = 0;
        let mut water = 0;
        for i in 0..256 {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 256.0;
            let angle = i as f32 * 2.399963;
            let radius = (1.0 - y * y).sqrt();
            let point = Vector3::new(radius * angle.cos(), y, radius * angle.sin());
            let height = elevation(point);
            assert_eq!(height, elevation(point));
            if height > SEA_LEVEL {
                land += 1;
            } else {
                water += 1;
            }
            let color = shade(point, point, 5.0, point);
            for channel in [color.x, color.y, color.z] {
                assert!(channel.is_finite() && (0.0..=1.0).contains(&channel));
            }
        }
        assert!(land > 20 && water > 20);
    }

    #[test]
    fn clouds_move_and_lighting_uses_the_sun() {
        let mut changed = 0;
        for i in 0..64 {
            let point =
                Vector3::new((i as f32 * 0.7).sin(), 0.3, (i as f32 * 0.7).cos()).normalize();
            if (cloud_cover(point, 0.0) - cloud_cover(point, 30.0)).abs() > 0.03 {
                changed += 1;
            }
            assert!(
                shade(point, point, 0.0, point).length()
                    > shade(point, point, 0.0, -point).length() * 5.0
            );
        }
        assert!(changed > 5);
    }
}
