use crate::world::World;
use raylib::prelude::Vector3;

pub mod sun;

/// Register all objects in the world.
pub fn register_all(world: &mut World) {
    world.add(sun::Sun::new(Vector3::new(0.0, 0.0, 500.0), 1.0));
}
