//! Superficie CPU de Kashyyyk. Las coordenadas locales mantienen el terreno
//! unido al planeta; solo el campo de nubes se desplaza con el tiempo.
use crate::noise::{fbm, smoothstep};
use raylib::prelude::Vector3;

const SEA_LEVEL: f32 = 0.45;
const CLOUD_SPEED: f32 = 0.018;

fn mix(a: Vector3, b: Vector3, amount: f32) -> Vector3 {
    a + (b - a) * amount.clamp(0.0, 1.0)
}

fn elevation(point: Vector3) -> f32 {
    fbm(point * 3.3 + Vector3::new(2.4, -1.7, 0.8), 3)
}

fn cloud_cover(point: Vector3, time: f32) -> f32 {
    let flow = Vector3::new(time * CLOUD_SPEED, time * 0.004, -time * 0.011);
    smoothstep(0.59, 0.76, fbm(point * 5.2 + flow, 2)) * 0.72
}

pub(super) fn shade(
    point: Vector3,
    normal: Vector3,
    time: f32,
    light_direction: Vector3,
) -> Vector3 {
    let height = elevation(point);
    let detail = fbm(point * 23.0, 2);
    let density = fbm(point * 8.5 + Vector3::new(-3.0, 0.5, 1.2), 2);
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
    let ground = mix(
        water,
        coast,
        smoothstep(SEA_LEVEL - 0.008, SEA_LEVEL + 0.012, height),
    );
    let color = mix(
        ground,
        Vector3::new(0.83, 0.89, 0.86),
        cloud_cover(point, time),
    );
    let diffuse = normal.dot(light_direction).max(0.0).sqrt();
    color * (0.08 + diffuse * 0.92)
}

#[cfg(test)]
mod tests {
    use super::*;

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
