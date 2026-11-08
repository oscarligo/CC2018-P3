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

#[derive(Clone, Copy, Debug)]
pub struct CelestialStyle {
    pub noise_offset: Vector3,
    pub tint: Vector3,
    pub cloud_threshold: f32,
    pub animation_speed: f32,
    pub rotation_speed: f32,
}

impl Default for CelestialStyle {
    fn default() -> Self {
        Self {
            noise_offset: Vector3::zero(),
            tint: Vector3::new(1.0, 1.0, 1.0),
            cloud_threshold: 0.59,
            animation_speed: 1.0,
            rotation_speed: 0.08,
        }
    }
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
    light_direction: Vector3,
    style: CelestialStyle,
) -> Vector3 {
    let color = match kind {
        CelestialKind::Star => star::shade(point, time, style.noise_offset),
        CelestialKind::Rocky => rocky::shade(
            base_color.expect("rocky bodies require a precalculated base color"),
            point,
            normal,
            time,
            light_direction,
            style.noise_offset,
            style.cloud_threshold,
        ),
        CelestialKind::GasGiant => {
            gas_giant::shade(point, normal, time, light_direction, style.noise_offset)
        }
    };
    Vector3::new(
        (color.x * style.tint.x).clamp(0.0, 1.0),
        (color.y * style.tint.y).clamp(0.0, 1.0),
        (color.z * style.tint.z).clamp(0.0, 1.0),
    )
}

pub(super) fn color_mix(from: Vector3, to: Vector3, amount: f32) -> Vector3 {
    from + (to - from) * amount.clamp(0.0, 1.0)
}

pub(super) fn lit(color: Vector3, normal: Vector3, light_direction: Vector3) -> Vector3 {
    const AMBIENT_LIGHT: f32 = 0.06;
    let diffuse = normal.dot(light_direction).max(0.0).powf(0.65);
    color * (AMBIENT_LIGHT + diffuse * (1.0 - AMBIENT_LIGHT))
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
            let color = shade(
                kind,
                point,
                point,
                1.25,
                base_color(kind, point),
                Vector3::new(0.0, 1.0, 0.0),
                CelestialStyle::default(),
            );
            for channel in [color.x, color.y, color.z] {
                assert!(channel.is_finite() && (0.0..=1.0).contains(&channel));
            }
        }
    }

    #[test]
    fn lighting_uses_the_real_source_direction() {
        let white = Vector3::new(1.0, 1.0, 1.0);
        let normal = Vector3::new(0.0, 1.0, 0.0);
        let lit_side = lit(white, normal, normal);
        let dark_side = lit(white, normal, -normal);
        assert!(lit_side.x > dark_side.x * 10.0);
    }

    #[test]
    fn style_changes_procedural_appearance() {
        let point = Vector3::new(0.3, 0.8, -0.4).normalize();
        let default = shade(
            CelestialKind::Star,
            point,
            point,
            1.0,
            None,
            point,
            CelestialStyle::default(),
        );
        let styled = shade(
            CelestialKind::Star,
            point,
            point,
            1.0,
            None,
            point,
            CelestialStyle {
                noise_offset: Vector3::new(3.0, 2.0, 1.0),
                tint: Vector3::new(0.5, 1.0, 1.0),
                ..CelestialStyle::default()
            },
        );
        assert!((default - styled).length() > 0.01);
    }
}
