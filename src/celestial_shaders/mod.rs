mod gas_giant;
mod rocky;
mod star;

use raylib::prelude::Vector3;

#[derive(Clone, Copy, Debug, Default)]
pub enum CelestialKind {
    #[default]
    Star,
    Rocky,
    GasGiant,
}

pub fn base_color(kind: CelestialKind, point: Vector3) -> Option<Vector3> {
    match kind {
        CelestialKind::Rocky => Some(rocky::base_color(point)),
        CelestialKind::Star | CelestialKind::GasGiant => None,
    }
}

pub fn shade(
    kind: CelestialKind,
    point: Vector3,
    normal: Vector3,
    time: f32,
    base_color: Option<Vector3>,
) -> Vector3 {
    match kind {
        CelestialKind::Star => star::shade(point, time),
        CelestialKind::Rocky => rocky::shade(
            base_color.expect("rocky bodies require a precalculated base color"),
            point,
            normal,
            time,
        ),
        CelestialKind::GasGiant => gas_giant::shade(point, normal, time),
    }
}

pub(super) fn color_mix(from: Vector3, to: Vector3, amount: f32) -> Vector3 {
    from + (to - from) * amount.clamp(0.0, 1.0)
}

pub(super) fn lit(color: Vector3, normal: Vector3) -> Vector3 {
    let light_direction = Vector3::new(-0.4534, 0.6550, -0.6046);
    color * (0.18 + normal.dot(light_direction).max(0.0) * 0.82)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_colors_are_finite_and_in_display_range() {
        let point = Vector3::new(0.3, 0.8, -0.4).normalize();
        for kind in [
            CelestialKind::Star,
            CelestialKind::Rocky,
            CelestialKind::GasGiant,
        ] {
            let color = shade(kind, point, point, 1.25, base_color(kind, point));
            for channel in [color.x, color.y, color.z] {
                assert!(channel.is_finite() && (0.0..=1.0).contains(&channel));
            }
        }
    }
}
