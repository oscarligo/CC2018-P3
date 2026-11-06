use crate::framebuffer::Framebuffer;
use crate::render::Renderer;
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
    fn draw(&mut self, renderer: &mut Renderer, framebuffer: &mut Framebuffer, uniforms: &Uniforms);
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

    pub fn draw(
        &mut self,
        renderer: &mut Renderer,
        framebuffer: &mut Framebuffer,
        uniforms: &Uniforms,
    ) {
        for object in &mut self.objects {
            object.draw(renderer, framebuffer, uniforms);
        }
    }
}
