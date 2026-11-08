use super::{color_mix, lit};
use crate::noise::{fbm, smoothstep};
use raylib::prelude::Vector3;

pub fn shade(point: Vector3, normal: Vector3, time: f32, light_direction: Vector3) -> Vector3 {
    let turbulence = fbm(point * 4.5 + Vector3::new(time * 0.025, 0.0, 0.0), 4);
    let latitude = point.y + (turbulence - 0.5) * 0.24;
    let wave = (latitude * 25.0 + (point.x * 5.0 + time * 0.18).sin() * 0.55).sin();
    let band = smoothstep(-0.78, 0.78, wave);
    let cream = Vector3::new(0.86, 0.68, 0.43);
    let amber = Vector3::new(0.62, 0.24, 0.075);
    let pale = Vector3::new(0.95, 0.84, 0.64);
    let mut color = color_mix(amber, cream, band);
    color = color_mix(color, pale, smoothstep(0.63, 0.82, turbulence) * 0.55);

    let storm_center = Vector3::new(0.58, -0.18, -0.78).normalize();
    let delta = point - storm_center;
    let storm_distance = (delta.y * delta.y * 5.5 + delta.x * delta.x + delta.z * delta.z).sqrt();
    let storm = 1.0 - smoothstep(0.12, 0.34, storm_distance + (turbulence - 0.5) * 0.08);
    let swirl = ((delta.z.atan2(delta.x) * 5.0 - storm_distance * 28.0 + time * 0.4).sin() * 0.5
        + 0.5)
        * storm;
    color = color_mix(
        color,
        color_mix(Vector3::new(0.58, 0.12, 0.045), pale, swirl),
        storm,
    );
    lit(color, normal, light_direction)
}
