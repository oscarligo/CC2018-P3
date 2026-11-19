use std::sync::OnceLock;
const MAP_SIZE: usize = 96;
static URESKI_MAP: OnceLock<Vec<Vector3>> = OnceLock::new();
fn cube_point(face: usize, u: f32, w: f32) -> Vector3 {
    match face {
        0 => v(1., w, u),
        1 => v(-1., w, u),
        2 => v(u, 1., w),
        3 => v(u, -1., w),
        4 => v(u, w, 1.),
        _ => v(u, w, -1.),
    }
    .normalize()
}
pub(super) fn prepare() {
    URESKI_MAP.get_or_init(|| {
        let mut colors = Vec::with_capacity(MAP_SIZE * MAP_SIZE * 6);
        for face in 0..6 {
            for y in 0..MAP_SIZE {
                for x in 0..MAP_SIZE {
                    let p = cube_point(
                        face,
                        x as f32 / (MAP_SIZE - 1) as f32 * 2. - 1.,
                        y as f32 / (MAP_SIZE - 1) as f32 * 2. - 1.,
                    );
                    colors.push(ureski_uncached(p, p, 0., v(1., 0., 0.)));
                }
            }
        }
        colors
    });
}
pub(super) fn ureski(p: Vector3, _normal: Vector3, _time: f32, _light: Vector3) -> Vector3 {
    prepare();
    let a = v(p.x.abs(), p.y.abs(), p.z.abs());
    let (face, u, w) = if a.x >= a.y && a.x >= a.z {
        (usize::from(p.x < 0.), p.z / a.x, p.y / a.x)
    } else if a.y >= a.z {
        (2 + usize::from(p.y < 0.), p.x / a.y, p.z / a.y)
    } else {
        (4 + usize::from(p.z < 0.), p.x / a.z, p.y / a.z)
    };
    let x = ((u * 0.5 + 0.5) * (MAP_SIZE - 1) as f32).clamp(0., (MAP_SIZE - 1) as f32 - 0.0001);
    let y = ((w * 0.5 + 0.5) * (MAP_SIZE - 1) as f32).clamp(0., (MAP_SIZE - 1) as f32 - 0.0001);
    let ix = x as usize;
    let iy = y as usize;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let map = URESKI_MAP.get().unwrap();
    let base = face * MAP_SIZE * MAP_SIZE;
    let a = map[base + iy * MAP_SIZE + ix] * (1. - fx) + map[base + iy * MAP_SIZE + ix + 1] * fx;
    let b = map[base + (iy + 1) * MAP_SIZE + ix] * (1. - fx)
        + map[base + (iy + 1) * MAP_SIZE + ix + 1] * fx;
    a * (1. - fy) + b * fy
}
use crate::noise::{smoothstep, value_noise_3d};
use raylib::prelude::Vector3;
fn v(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}
fn mix(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    a * (1. - t) + b * t
}

/// Local +X is permanently subsolar; the biome boundaries move with the actual body.
fn ureski_uncached(p: Vector3, _normal: Vector3, time: f32, _light: Vector3) -> Vector3 {
    let n = value_noise_3d(p * 12. + v(3., 1., 8.));
    let fine = value_noise_3d(p * 42.);
    let dunes = (p.y * 45. + (p.z * 11.).sin() * 2. + n * 3.).sin() * 0.5 + 0.5;
    let hot = mix(
        v(0.45, 0.14, 0.035),
        v(0.98, 0.67, 0.26),
        0.3 + n * 0.5 + dunes * 0.2,
    );
    let hot = mix(hot, v(1., 0.84, 0.45), smoothstep(0.77, 0.99, p.x) * 0.55);
    let frost = (p.y * 23. + (p.z * 19.).cos() * 2.).sin().abs();
    let cold = mix(
        v(0.09, 0.18, 0.34),
        v(0.55, 0.78, 0.89),
        0.25 + n * 0.45 + fine * 0.20,
    );
    let cold = cold + v(0.02, 0.12, 0.19) * (1. - smoothstep(0.01, 0.06, frost));
    let land = mix(v(0.035, 0.27, 0.08), v(0.30, 0.69, 0.13), n);
    let river = 1. - smoothstep(0.04, 0.10, (n - 0.48).abs());
    let green = mix(land, v(0.025, 0.34, 0.41), river * 0.72) + v(0.03, 0.07, 0.01) * fine;
    let edge = 0.27 + (p.y * 9. + (p.z * 8.).sin()).sin() * 0.025;
    let day = smoothstep(edge, edge + 0.10, p.x);
    let night = smoothstep(edge, edge + 0.10, -p.x);
    let result = mix(mix(green, hot, day), cold, night);
    let aurora = (1. - smoothstep(0.012, 0.045, (p.x + 0.34).abs()))
        * (0.5 + 0.5 * (p.z * 31. + time * 0.6).sin());
    result * (0.70 + 0.30 * p.x.max(0.)) + v(0.04, 0.23, 0.20) * aurora
}

pub(super) fn apparitions(time: f32) -> [Vector3; 3] {
    [
        v(
            (time * 0.37).sin() * 0.11,
            (time * 0.53).cos() * 0.07,
            (time * 0.29).cos() * 0.08,
        ),
        v(
            0.32 * (time * 0.41 + 1.).sin(),
            0.23 * (time * 0.32).cos(),
            0.15 * (time * 0.51).sin(),
        ),
        v(
            0.30 * (time * 0.31 + 3.).cos(),
            0.24 * (time * 0.49 + 1.).sin(),
            0.16 * (time * 0.23 + 2.).cos(),
        ),
    ]
}

/// Analytic internal volume, composed along each camera ray before the gas envelope.
/// The inner spheres can overlap: they are apparitions, not solid surface decorations.
pub(super) fn riddertal(p: Vector3, ray: Vector3, time: f32, light: Vector3) -> Vector3 {
    let view = -ray;
    let mut right = v(0., 1., 0.).cross(view);
    if right.length_sqr() < 0.001 {
        right = v(1., 0., 0.).cross(view);
    }
    let right = right.normalize();
    let up = view.cross(right).normalize();
    let mut inner = v(0.16, 0.025, 0.12);
    for (i, center) in apparitions(time).into_iter().enumerate().rev() {
        let t = (center - p).dot(ray).max(0.);
        let near = p + ray * t - center;
        let radius = if i == 0 { 0.38 } else { 0.35 };
        let distance = near.length();
        let halo = (-distance * distance * 5.).exp();
        inner += v(0.15, 0.035, 0.18) * halo;
        if distance < radius {
            let front = (radius * radius - distance * distance).sqrt();
            let hit = (near - ray * front) * (1. / radius);
            let a = hit.y.atan2(hit.x);
            let ribbons = (hit.y * 24. + a * 3. + time * (1. + i as f32 * 0.2) + hit.z * 11.)
                .sin()
                .abs();
            let wire = 1. - smoothstep(0.025, 0.11, ribbons);
            let energy = (time * 0.9 + hit.x * 12. + hit.z * 8.).sin() * 0.5 + 0.5;
            let mut core =
                v(0.12, 0.035, 0.23) + v(0.28, 0.13, 0.35) * energy + v(0.48, 0.77, 0.79) * wire;
            let uv = v(near.dot(right), near.dot(up), 0.);
            for sign in [-1., 1.] {
                let x = uv.x - sign * 0.145;
                let y = uv.y - 0.045;
                let almond = (x / 0.114).powi(2) + (y / 0.047).powi(2);
                let eyelid = 1. - smoothstep(0.78, 1.08, almond);
                if eyelid > 0.001 {
                    let iris = (x / 0.039).powi(2) + (y / 0.037).powi(2);
                    let pupil = (x / 0.011).powi(2) + (y / 0.036).powi(2);
                    let eye = mix(
                        v(0.93, 0.84, 0.66),
                        v(0.27, 0.92, 0.86),
                        1. - smoothstep(0.8, 1.05, iris),
                    );
                    let eye = mix(
                        eye,
                        v(0.008, 0.004, 0.025),
                        1. - smoothstep(0.72, 1.0, pupil),
                    );
                    core = mix(core, eye, eyelid);
                }
            }
            let opacity = if i == 0 {
                0.92
            } else {
                0.32 + 0.12 * (time * 0.8 + i as f32).sin()
            };
            inner = mix(inner, core, opacity);
        }
    }
    let flow = v(
        p.x + (p.y * 7. + time * 0.35).sin() * 0.12,
        p.y,
        p.z + (p.x * 5. - time * 0.28).cos() * 0.13,
    );
    let cloud = value_noise_3d(flow * 7. + v(time * 0.035, 0., 0.));
    let band = (p.y * 24. + cloud * 5. + time * 0.28).sin() * 0.5 + 0.5;
    let gas = mix(
        v(0.52, 0.14, 0.32),
        v(0.99, 0.58, 0.74),
        cloud * 0.65 + band * 0.35,
    );
    let facing = p.dot(view).abs();
    let opacity = 0.26 + cloud * 0.17 + (1. - facing).powf(2.) * 0.45;
    let result = mix(inner, gas * (0.80 + 0.20 * p.dot(light).max(0.)), opacity);
    let shimmer = (1. - facing).powf(5.) * (0.65 + 0.35 * (p.y * 21. + time * 2.).sin());
    result + v(0.45, 0.14, 0.32) * shimmer
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ureski_has_three_permanent_visibly_different_climates() {
        let hot = ureski(v(1., 0., 0.), v(1., 0., 0.), 0., v(1., 0., 0.));
        let cold = ureski(v(-1., 0., 0.), v(-1., 0., 0.), 0., v(1., 0., 0.));
        let spring = ureski(v(0., 0., 1.), v(0., 0., 1.), 0., v(1., 0., 0.));
        assert!(hot.x > hot.z * 2.);
        assert!(cold.z > cold.x * 1.5);
        assert!(spring.y > spring.x * 2.);
    }
    #[test]
    fn the_entity_is_animated_view_dependent_and_finite() {
        assert_ne!(apparitions(0.), apparitions(3.));
        let mut change = 0.;
        for i in 0..100 {
            let p = v(i as f32 * 0.006 - 0.3, 0.01, -0.95).normalize();
            let ray = (p - v(0., 0., -3.)).normalize();
            let a = riddertal(p, ray, 0., v(1., 0., 0.));
            let b = riddertal(p, ray, 3., v(1., 0., 0.));
            assert!(a.x.is_finite() && a.y.is_finite() && a.z.is_finite());
            change += (a - b).length();
        }
        assert!(change > 2.);
    }
}
