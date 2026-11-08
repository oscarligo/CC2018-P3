use self::catalog::{BodyDefinition, MotionDefinition};
use crate::scene::world::{LocalOrbit, Orbit, TransformAnchor, World};
use raylib::prelude::Vector3;

pub mod atmosphere;
mod catalog;
pub mod celestial;
pub mod ring;
pub mod rock;

/// Register all objects in the world.
pub fn register_all(world: &mut World) {
    let mut anchors: Vec<TransformAnchor> = Vec::new();
    for definition in catalog::solar_system_definitions() {
        register_body(world, definition, &mut anchors);
    }
}

fn register_body(
    world: &mut World,
    definition: BodyDefinition,
    anchors: &mut Vec<TransformAnchor>,
) {
    let mut body = if definition.primary_light {
        celestial::CelestialBody::new_primary_star_styled(definition.position, definition.style)
    } else {
        celestial::CelestialBody::styled(definition.kind, definition.position, definition.style)
    }
    .with_shape(definition.shape)
    .with_scale(definition.scale);
    body = match definition.motion {
        MotionDefinition::Static => body,
        MotionDefinition::Orbit {
            center,
            axis,
            angular_speed,
        } => body.with_orbit(Orbit::new(center, definition.position, axis, angular_speed)),
        MotionDefinition::ParentOrbit {
            parent,
            initial_offset,
            axis,
            angular_speed,
        } => body.with_parent_orbit(
            anchors
                .get(parent)
                .unwrap_or_else(|| panic!("parent body {parent} must be registered first"))
                .clone(),
            LocalOrbit::new(initial_offset, axis, angular_speed),
        ),
    };
    let anchor = body.transform.anchor();
    world.add(body);

    if let Some(field) = definition.rock_field {
        world.add(rock::RockField::attached_to(
            anchor.clone(),
            definition.scale,
            definition.shape,
            field.seed,
            field.count,
        ));
    }
    for rock in definition.rocks {
        world.add(rock::SurfaceRock::attached_to(
            anchor.clone(),
            rock.direction,
            rock.scale,
            rock.twist,
        ));
    }
    if let Some(atmosphere) = definition.atmosphere {
        world.add(
            atmosphere::Atmosphere::new(
                Vector3::zero(),
                definition.scale,
                atmosphere.shell_scale,
                atmosphere.color,
                atmosphere.strength,
            )
            .with_shape(atmosphere.shape)
            .attached_to(anchor.clone(), Vector3::zero()),
        );
    }
    if let Some(ring) = definition.ring {
        world.add(
            ring::PlanetRing::new(
                Vector3::zero(),
                definition.scale * ring.outer_scale,
                ring.tilt_x,
                ring.tilt_z,
            )
            .attached_to(anchor.clone(), Vector3::zero()),
        );
    }
    anchors.push(anchor);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_system_exposes_the_sun_as_primary_light() {
        let mut world = World::new();
        register_all(&mut world);
        assert_eq!(
            world.primary_light_position(),
            Some(Vector3::new(0.0, 0.0, 500.0))
        );
    }
}
