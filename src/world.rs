use crate::framebuffer::Framebuffer;
use crate::uniforms::Uniforms;
use raylib::prelude::Vector3;

/// Shared data for updating the world state.
pub struct WorldUpdate {
    pub frame_time: f32,
    pub velocity: Vector3,
}

// The `WorldObject` trait defines the interface for objects that can be updated and drawn in the world. Each object must implement the `update` method to handle state changes based on the provided `WorldUpdate`, and the `draw` method to render itself onto the framebuffer using the given uniforms.
pub trait WorldObject {
    fn update(&mut self, update: &WorldUpdate);
    fn draw(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms);
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
            object.update(update);
        }
    }

    pub fn draw(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        for object in &self.objects {
            object.draw(framebuffer, uniforms);
        }
    }
}
