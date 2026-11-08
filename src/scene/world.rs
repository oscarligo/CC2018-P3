use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::projection::sphere_is_visible;
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use raylib::prelude::Vector3;

/// Estado espacial común para cualquier entidad del mundo.
pub struct WorldTransform {
    pub position: Vector3,
    pub velocity: Vector3,
    pub scale: Vector3,
    orbit: Option<Orbit>,
    orbit_time: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    center: Vector3,
    initial_offset: Vector3,
    axis: Vector3,
    angular_speed: f32,
}

impl Orbit {
    pub fn new(
        center: Vector3,
        initial_position: Vector3,
        axis: Vector3,
        angular_speed: f32,
    ) -> Self {
        Self {
            center,
            initial_offset: initial_position - center,
            axis: if axis.length() > f32::EPSILON {
                axis.normalize()
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            },
            angular_speed,
        }
    }

    fn position(&self, time: f32) -> Vector3 {
        let angle = self.angular_speed * time;
        let (sin, cos) = angle.sin_cos();
        let offset = self.initial_offset * cos
            + self.axis.cross(self.initial_offset) * sin
            + self.axis * self.axis.dot(self.initial_offset) * (1.0 - cos);
        self.center + offset
    }
}

impl WorldTransform {
    pub fn new(position: Vector3, scale: Vector3) -> Self {
        Self {
            position,
            velocity: Vector3::zero(),
            scale,
            orbit: None,
            orbit_time: 0.0,
        }
    }

    pub fn set_orbit(&mut self, orbit: Orbit) {
        self.orbit = Some(orbit);
        self.orbit_time = 0.0;
        self.position = orbit.position(0.0);
    }

    fn advance(&mut self, update: &WorldUpdate) {
        if let Some(orbit) = self.orbit {
            self.orbit_time += update.frame_time;
            self.position = orbit.position(self.orbit_time);
        }
        self.position += self.velocity * update.frame_time;
    }
}

/// Shared data for updating the world state.
pub struct WorldUpdate {
    pub frame_time: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderPass {
    Opaque,
    Transparent,
}

// The `WorldObject` trait defines the interface for objects that can be updated and drawn in the world. Each object must implement the `update` method to handle state changes based on the provided `WorldUpdate`, and the `draw` method to render itself onto the framebuffer using the given uniforms.
pub trait WorldObject {
    fn transform(&self) -> &WorldTransform;
    fn transform_mut(&mut self) -> &mut WorldTransform;
    fn update(&mut self, _update: &WorldUpdate) {}
    fn bounding_radius(&self) -> Option<f32> {
        None
    }
    fn light_position(&self) -> Option<Vector3> {
        None
    }
    fn render_pass(&self) -> RenderPass {
        RenderPass::Opaque
    }
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

    pub(crate) fn primary_light_position(&self) -> Option<Vector3> {
        self.objects
            .iter()
            .find_map(|object| object.light_position())
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
        let light_position = self.primary_light_position().or(uniforms.light_position);
        let scene_uniforms = Uniforms {
            light_position,
            ..*uniforms
        };
        self.sort_front_to_back(uniforms.camera_pos);
        draw_pass(
            &mut self.objects,
            RenderPass::Opaque,
            false,
            renderer,
            framebuffer,
            &scene_uniforms,
        );
        draw_pass(
            &mut self.objects,
            RenderPass::Transparent,
            true,
            renderer,
            framebuffer,
            &scene_uniforms,
        );
    }

    fn sort_front_to_back(&mut self, camera_position: Vector3) {
        self.objects.sort_by(|a, b| {
            distance_to_surface(a.as_ref(), camera_position)
                .total_cmp(&distance_to_surface(b.as_ref(), camera_position))
        });
    }
}

fn draw_pass(
    objects: &mut [Box<dyn WorldObject>],
    pass: RenderPass,
    reverse: bool,
    renderer: &mut Renderer,
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
) {
    let mut draw_object = |object: &mut Box<dyn WorldObject>| {
        if object.render_pass() != pass {
            return;
        }
        let culled = object.bounding_radius().is_some_and(|local_radius| {
            let transform = object.transform();
            let scale = transform
                .scale
                .x
                .abs()
                .max(transform.scale.y.abs())
                .max(transform.scale.z.abs());
            !sphere_is_visible(
                transform.position,
                local_radius * scale,
                uniforms,
                framebuffer.width,
                framebuffer.height,
            )
        });
        renderer.record_object(culled);
        if !culled {
            object.draw(renderer, framebuffer, uniforms);
        }
    };

    if reverse {
        for object in objects.iter_mut().rev() {
            draw_object(object);
        }
    } else {
        for object in objects.iter_mut() {
            draw_object(object);
        }
    }
}

fn distance_to_surface(object: &dyn WorldObject, camera_position: Vector3) -> f32 {
    let transform = object.transform();
    let distance = (transform.position - camera_position).length();
    let radius = object.bounding_radius().map_or(0.0, |local_radius| {
        let scale = transform
            .scale
            .x
            .abs()
            .max(transform.scale.y.abs())
            .max(transform.scale.z.abs());
        local_radius * scale
    });
    (distance - radius).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use raylib::prelude::{Color, Matrix};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct TestObject {
        transform: WorldTransform,
        radius: Option<f32>,
    }

    impl WorldObject for TestObject {
        fn transform(&self) -> &WorldTransform {
            &self.transform
        }

        fn transform_mut(&mut self) -> &mut WorldTransform {
            &mut self.transform
        }

        fn bounding_radius(&self) -> Option<f32> {
            self.radius
        }

        fn draw(
            &mut self,
            _renderer: &mut Renderer,
            _framebuffer: &mut Framebuffer,
            _uniforms: &Uniforms,
        ) {
        }
    }

    #[test]
    fn objects_are_sorted_nearest_first() {
        let mut world = World::new();
        for z in [30.0, 10.0, 20.0] {
            world.add(TestObject {
                transform: WorldTransform::new(
                    Vector3::new(0.0, 0.0, z),
                    Vector3::new(1.0, 1.0, 1.0),
                ),
                radius: None,
            });
        }

        world.sort_front_to_back(Vector3::zero());

        let positions: Vec<_> = world
            .objects
            .iter()
            .map(|object| object.transform().position.z)
            .collect();
        assert_eq!(positions, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn sorting_accounts_for_object_radius_and_scale() {
        let mut world = World::new();
        world.add(TestObject {
            transform: WorldTransform::new(
                Vector3::new(0.0, 0.0, 8.0),
                Vector3::new(1.0, 1.0, 1.0),
            ),
            radius: None,
        });
        world.add(TestObject {
            transform: WorldTransform::new(
                Vector3::new(0.0, 0.0, 10.0),
                Vector3::new(2.0, 2.0, 2.0),
            ),
            radius: Some(2.0),
        });

        world.sort_front_to_back(Vector3::zero());

        assert_eq!(world.objects[0].transform().position.z, 10.0);
    }

    struct TransparentTestObject {
        transform: WorldTransform,
        order: Rc<RefCell<Vec<f32>>>,
    }

    impl WorldObject for TransparentTestObject {
        fn transform(&self) -> &WorldTransform {
            &self.transform
        }

        fn transform_mut(&mut self) -> &mut WorldTransform {
            &mut self.transform
        }

        fn render_pass(&self) -> RenderPass {
            RenderPass::Transparent
        }

        fn draw(
            &mut self,
            _renderer: &mut Renderer,
            _framebuffer: &mut Framebuffer,
            _uniforms: &Uniforms,
        ) {
            self.order.borrow_mut().push(self.transform.position.z);
        }
    }

    #[test]
    fn transparent_objects_are_drawn_farthest_first() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut world = World::new();
        for z in [20.0, 30.0, 10.0] {
            world.add(TransparentTestObject {
                transform: WorldTransform::new(
                    Vector3::new(0.0, 0.0, z),
                    Vector3::new(1.0, 1.0, 1.0),
                ),
                order: Rc::clone(&order),
            });
        }
        let mut renderer = Renderer::default();
        let mut framebuffer = Framebuffer::new(1, 1, Color::BLACK);
        let uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::new(0.0, 0.0, 1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };

        world.draw(&mut renderer, &mut framebuffer, &uniforms);

        assert_eq!(*order.borrow(), [30.0, 20.0, 10.0]);
    }

    #[test]
    fn shared_orbit_parameters_keep_grouped_transforms_synchronized() {
        let center = Vector3::new(0.0, 0.0, 5.0);
        let initial = Vector3::new(10.0, 2.0, 5.0);
        let orbit = Orbit::new(center, initial, Vector3::new(0.2, 1.0, 0.1), 0.5);
        let mut body = WorldTransform::new(initial, Vector3::new(1.0, 1.0, 1.0));
        let mut atmosphere = WorldTransform::new(initial, Vector3::new(1.1, 1.1, 1.1));
        body.set_orbit(orbit);
        atmosphere.set_orbit(orbit);

        for _ in 0..120 {
            let update = WorldUpdate {
                frame_time: 1.0 / 60.0,
            };
            body.advance(&update);
            atmosphere.advance(&update);
        }

        assert!((body.position - atmosphere.position).length() < 0.0001);
        assert!(((body.position - center).length() - (initial - center).length()).abs() < 0.001);
    }
}
