use super::color_mix;
use crate::fragment::Fragment;
use crate::noise::{fbm, smoothstep};
use raylib::prelude::Vector3;

pub fn shade(fragment: &Fragment, time: f32) -> Vector3 {
    let point = fragment.object_position.normalize();
    let flow = Vector3::new(time * 0.08, -time * 0.035, time * 0.05);
    let cells = fbm(point * 3.4 + flow, 4);
    let granules = fbm(point * 14.0 - flow * 1.7, 3);
    let filaments = ((point.y * 18.0 + cells * 7.0 + time * 0.35).sin() * 0.5 + 0.5) * 0.18;
    let heat = (cells * 0.58 + granules * 0.30 + filaments).clamp(0.0, 1.0);

    let dark = Vector3::new(0.22, 0.015, 0.005);
    let orange = Vector3::new(1.0, 0.22, 0.015);
    let yellow = Vector3::new(1.0, 0.76, 0.10);
    let white = Vector3::new(1.0, 0.98, 0.78);
    let warm = color_mix(dark, orange, smoothstep(0.16, 0.48, heat));
    let hot = color_mix(warm, yellow, smoothstep(0.42, 0.73, heat));
    color_mix(hot, white, smoothstep(0.76, 0.94, heat))
}
