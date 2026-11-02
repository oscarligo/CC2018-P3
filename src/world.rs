use crate::framebuffer::Framebuffer;
use crate::uniforms::Uniforms;
use raylib::prelude::Vector3;

/// Estado espacial común para cualquier entidad del mundo.
pub struct WorldTransform {
    pub position: Vector3,
    pub velocity: Vector3,
    pub scale: Vector3,
}

impl WorldTransform {
    pub fn new(position: Vector3, scale: Vector3) -> Self {
        Self {
            position,
            velocity: Vector3::zero(),
            scale,
        }
    }

    fn advance(&mut self, update: &WorldUpdate) {
        self.position += self.velocity * update.frame_time;
    }
}

/// Shared data for updating the world state.
pub struct WorldUpdate {
    pub frame_time: f32,
}

// The `WorldObject` trait defines the interface for objects that can be updated and drawn in the world. Each object must implement the `update` method to handle state changes based on the provided `WorldUpdate`, and the `draw` method to render itself onto the framebuffer using the given uniforms.
pub trait WorldObject {
    fn transform(&self) -> &WorldTransform;
    fn transform_mut(&mut self) -> &mut WorldTransform;
    fn update(&mut self, _update: &WorldUpdate) {}
    fn draw(&mut self, framebuffer: &mut Framebuffer, uniforms: &Uniforms);
}

#[derive(Default)]
pub struct World {
    objects: Vec<Box<dyn WorldObject>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<T: WorldObject + 'static>(&mut self, object: T) {
        self.objects.push(Box::new(object));
    }

    pub fn update(&mut self, update: &WorldUpdate) {
        for object in &mut self.objects {
            object.transform_mut().advance(update);
            object.update(update);
        }
    }

    pub fn draw(&mut self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        for object in &mut self.objects {
            object.draw(framebuffer, uniforms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{World, WorldUpdate};
    use crate::objects::sun::Sun;
    use raylib::prelude::Vector3;

    #[test]
    fn long_world_updates_move_only_entities_with_their_own_velocity() {
        let stationary_position = Vector3::new(10.0, 20.0, 30.0);
        let moving_position = Vector3::new(-100.0, 25.0, 300.0);
        let velocity = Vector3::new(4.0, -2.0, 8.0);
        let mut moving = Sun::new(moving_position, 1.0);
        moving.set_velocity(velocity);
        let mut world = World::new();
        world.add(Sun::new(stationary_position, 1.0));
        world.add(moving);

        for _ in 0..1000 {
            world.update(&WorldUpdate { frame_time: 0.125 });
        }

        assert_eq!(world.objects[0].transform().position, stationary_position);
        assert_eq!(
            world.objects[1].transform().position,
            moving_position + velocity * 125.0
        );
    }
}
