use super::{color_mix, lit};
use crate::fragment::Fragment;
use crate::noise::{fbm, smoothstep};
use raylib::prelude::Vector3;

pub fn shade(fragment: &Fragment, time: f32) -> Vector3 {
    let point = fragment.object_position.normalize();
    let terrain = fbm(point * 3.1, 5);
    let detail = fbm(point * 17.0, 3);
    let ocean = Vector3::new(0.015, 0.12, 0.30) * (0.72 + detail * 0.28);
    let vegetation = color_mix(
        Vector3::new(0.08, 0.20, 0.06),
        Vector3::new(0.34, 0.27, 0.10),
        smoothstep(0.50, 0.72, terrain),
    );
    let mountain = Vector3::new(0.52, 0.48, 0.42) * (0.7 + detail * 0.45);
    let land = color_mix(vegetation, mountain, smoothstep(0.66, 0.82, terrain));
    let mut surface = color_mix(ocean, land, smoothstep(0.43, 0.49, terrain));

    let ice = smoothstep(0.72, 0.92, point.y.abs() + fbm(point * 8.0, 2) * 0.16);
    surface = color_mix(surface, Vector3::new(0.82, 0.91, 0.94), ice);
    let cloud_flow = Vector3::new(time * 0.035, 0.0, -time * 0.02);
    let clouds = smoothstep(0.59, 0.70, fbm(point * 6.0 + cloud_flow, 4)) * 0.78;
    lit(
        color_mix(surface, Vector3::new(0.92, 0.94, 0.95), clouds),
        fragment.normal,
    )
}
