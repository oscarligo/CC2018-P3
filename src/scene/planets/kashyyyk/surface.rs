//! Superficie CPU de Kashyyyk. Las coordenadas locales mantienen el terreno
//! unido al planeta; solo el campo de nubes se desplaza con el tiempo.
use crate::noise::{fbm, smoothstep};
use raylib::prelude::Vector3;
use std::sync::OnceLock;

const SEA_LEVEL: f32 = 0.45;
const CLOUD_SPEED: f32 = 0.018;
const GROUND_GRID: usize = 97;
const CLOUD_GRID: usize = 64;
const GROUND_SCALE: f32 = (GROUND_GRID - 1) as f32 * 0.5;
const CLOUD_SCALE: f32 = CLOUD_GRID as f32 / 8.0;
const GRAIN_SCALE: f32 = 256.0;
const GRAIN_STRENGTH: f32 = 0.45;

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
                    // Guardar datos de terreno, no colores: clasificar después
                    // de interpolar mantiene nítidos los límites de cada bioma.
                    ground.push(terrain_fields(
                        Vector3::new(x as f32, y as f32, z as f32) * (1.0 / GROUND_SCALE)
                            - Vector3::new(1.0, 1.0, 1.0),
                    ));
                }
            }
        }
        let mut clouds = Vec::with_capacity(CLOUD_GRID.pow(3));
        for z in 0..CLOUD_GRID {
            for y in 0..CLOUD_GRID {
                for x in 0..CLOUD_GRID {
                    clouds.push(fbm(
                        Vector3::new(x as f32, y as f32, z as f32) * (1.0 / CLOUD_SCALE),
                        3,
                    ));
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

#[inline(always)]
fn sample<T: FieldValue>(data: &[T], size: usize, point: Vector3, periodic: bool) -> T {
    let axes = [point.x, point.y, point.z].map(|value| {
        let value = if periodic {
            // La conversión a entero sustituye floor/fmod en coordenadas normales.
            // Mantener el fallback para tiempos extraordinariamente grandes.
            if value.abs() > 1_000_000.0 {
                value.rem_euclid(size as f32)
            } else {
                value
            }
        } else {
            value.clamp(0.0, (size - 1) as f32)
        };
        let truncated = value as i32;
        let cell = truncated - i32::from(value < truncated as f32);
        let first = if periodic {
            cell.rem_euclid(size as i32) as usize
        } else {
            cell as usize
        };
        let next = if periodic {
            (first + 1) % size
        } else {
            (first + 1).min(size - 1)
        };
        (first, next, value - cell as f32)
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
            (point.x * 5.2 + time * CLOUD_SPEED) * CLOUD_SCALE,
            (point.y * 5.2 + time * 0.004) * CLOUD_SCALE,
            (point.z * 5.2 - time * 0.011) * CLOUD_SCALE,
        ),
        true,
    );
    smoothstep(0.62, 0.73, noise) * 0.46
}

fn terrain_fields(point: Vector3) -> Vector3 {
    Vector3::new(
        elevation(point),
        vegetation_density(point),
        fbm(point * 23.0, 2),
    )
}

fn water_color(height: f32) -> Vector3 {
    mix(
        Vector3::new(0.012, 0.065, 0.13),
        Vector3::new(0.025, 0.21, 0.25),
        smoothstep(SEA_LEVEL - 0.065, SEA_LEVEL, height),
    )
}

fn ground_color(terrain: Vector3) -> Vector3 {
    let height = terrain.x;
    if height <= SEA_LEVEL - 0.002 {
        return water_color(height);
    }
    let density = terrain.y;
    let detail = terrain.z;
    let forest = mix(
        Vector3::new(0.035, 0.12, 0.025),
        Vector3::new(0.18, 0.38, 0.065),
        smoothstep(0.30, 0.68, density),
    );
    let variation = 0.78 + detail * 0.44;
    let forest = Vector3::new(
        forest.x * variation,
        forest.y * variation,
        forest.z * variation,
    );
    let rock_amount = smoothstep(0.635, 0.70, height + (detail - 0.5) * 0.025);
    let land = if rock_amount > 0.0 {
        mix(
            forest,
            mix(
                Vector3::new(0.25, 0.26, 0.21),
                Vector3::new(0.49, 0.47, 0.38),
                detail,
            ),
            rock_amount,
        )
    } else {
        forest
    };
    if height >= SEA_LEVEL + 0.028 {
        return land;
    }
    let coast = mix(
        Vector3::new(0.40, 0.36, 0.22),
        land,
        smoothstep(SEA_LEVEL + 0.008, SEA_LEVEL + 0.028, height),
    );
    if height >= SEA_LEVEL + 0.003 {
        return coast;
    }
    mix(
        water_color(height),
        coast,
        smoothstep(SEA_LEVEL - 0.002, SEA_LEVEL + 0.003, height),
    )
}

pub(super) fn shade(
    point: Vector3,
    normal: Vector3,
    time: f32,
    light_direction: Vector3,
) -> Vector3 {
    let terrain = sample(
        &fields().ground,
        GROUND_GRID,
        Vector3::new(
            point.x * GROUND_SCALE + GROUND_SCALE,
            point.y * GROUND_SCALE + GROUND_SCALE,
            point.z * GROUND_SCALE + GROUND_SCALE,
        ),
        false,
    );
    let mut ground = ground_color(terrain);
    if terrain.x > SEA_LEVEL + 0.008 {
        // El mismo volumen sirve como detalle estático de materiales. No se
        // recalcula FBM por píxel ni se añaden texturas o mallas al planeta.
        let grain = sample(
            &fields().clouds,
            CLOUD_GRID,
            Vector3::new(
                point.x * GRAIN_SCALE + 11.0,
                point.y * GRAIN_SCALE + 27.0,
                point.z * GRAIN_SCALE + 5.0,
            ),
            true,
        );
        let amount = smoothstep(SEA_LEVEL + 0.008, SEA_LEVEL + 0.028, terrain.x);
        let variation = 1.0 + (grain - 0.5) * GRAIN_STRENGTH * amount;
        ground = Vector3::new(
            ground.x * variation,
            ground.y * variation,
            ground.z * variation,
        );
    }
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
    fn periodic_sampling_handles_negative_cells_and_wrap_boundaries() {
        let data = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        for x in [-2.25, -0.25, 0.25, 2.25, 1_000_000.25] {
            assert!((sample(&data, 2, Vector3::new(x, 0.0, 0.0), true) - 0.25_f32).abs() < 0.00001);
        }
        assert_eq!(sample(&data, 2, Vector3::new(-1.0, 0.0, 0.0), true), 1.0);
        assert_eq!(sample(&data, 2, Vector3::new(2.0, 0.0, 0.0), true), 0.0);
    }

    #[test]
    fn coast_stays_sharp_when_terrain_fields_are_interpolated() {
        let sea = Vector3::new(SEA_LEVEL - 0.02, 0.5, 0.5);
        let land = Vector3::new(SEA_LEVEL + 0.06, 0.5, 0.5);
        let data = [sea, land, sea, land, sea, land, sea, land];
        let terrain = sample(&data, 2, Vector3::new(0.5, 0.5, 0.5), false);
        let classified = ground_color(terrain);
        let blurred = mix(ground_color(sea), ground_color(land), 0.5);
        assert!((classified - blurred).length() > 0.06);
        let water = ground_color(Vector3::new(SEA_LEVEL - 0.004, 0.5, 0.5));
        let shore = ground_color(Vector3::new(SEA_LEVEL + 0.006, 0.5, 0.5));
        assert!((water - shore).length() > 0.25);
    }

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
            Vector3::new(CLOUD_GRID as f32 - 0.2, 3.4, 2.1),
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
                point * GROUND_SCALE + Vector3::new(GROUND_SCALE, GROUND_SCALE, GROUND_SCALE),
                false,
            );
            assert!((cached - terrain_fields(point)).length() < 0.00001);
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
