mod gas_giant;
mod rocky;
mod star;

use crate::fragment::Fragment;
use raylib::prelude::Vector3;

#[derive(Clone, Copy, Debug, Default)]
pub enum CelestialKind {
    #[default]
    Star,
    Rocky,
    GasGiant,
}

pub fn shade(kind: CelestialKind, fragment: &Fragment, time: f32) -> Vector3 {
    match kind {
        CelestialKind::Star => star::shade(fragment, time),
        CelestialKind::Rocky => rocky::shade(fragment, time),
        CelestialKind::GasGiant => gas_giant::shade(fragment, time),
    }
}

pub(super) fn color_mix(from: Vector3, to: Vector3, amount: f32) -> Vector3 {
    from + (to - from) * amount.clamp(0.0, 1.0)
}

pub(super) fn lit(color: Vector3, normal: Vector3) -> Vector3 {
    let light_direction = Vector3::new(-0.45, 0.65, -0.6).normalize();
    color * (0.18 + normal.dot(light_direction).max(0.0) * 0.82)
}
