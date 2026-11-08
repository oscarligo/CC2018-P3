use crate::physics::{ObjectId, Ray};
use crate::rendering::uniforms::Uniforms;
use crate::scene::world::World;

const MAX_INTERACTION_DISTANCE: f32 = 1_200.0;
const FEEDBACK_SECONDS: f32 = 2.5;

#[derive(Clone, Copy)]
pub struct Interaction {
    pub(crate) prompt: &'static str,
    pub(crate) response: &'static str,
}

#[derive(Default)]
pub(crate) struct InteractionSystem {
    target: Option<ObjectId>,
    prompt: Option<&'static str>,
    feedback: Option<&'static str>,
    feedback_remaining: f32,
}

impl InteractionSystem {
    pub(crate) fn update(
        &mut self,
        world: &mut World,
        uniforms: &Uniforms,
        interact_pressed: bool,
        frame_time: f32,
    ) {
        self.feedback_remaining = (self.feedback_remaining - frame_time).max(0.0);
        if self.feedback_remaining == 0.0 {
            self.feedback = None;
        }
        let ray = Ray::new(
            uniforms.camera_pos,
            uniforms.camera_target - uniforms.camera_pos,
        );
        self.target = ray
            .and_then(|ray| world.raycast(ray, MAX_INTERACTION_DISTANCE))
            .map(|hit| hit.object_id)
            .filter(|id| world.interaction(*id).is_some());
        self.prompt = self
            .target
            .and_then(|id| world.interaction(id))
            .map(|interaction| interaction.prompt);

        if interact_pressed
            && let Some(response) = self
                .target
                .and_then(|id| world.interact(id))
                .map(|interaction| interaction.response)
        {
            self.feedback = Some(response);
            self.feedback_remaining = FEEDBACK_SECONDS;
            println!("{response}");
        }
    }

    #[cfg(test)]
    fn prompt(&self) -> Option<&str> {
        self.prompt
    }

    pub(crate) fn overlay(&self) -> Option<&str> {
        self.feedback.or(self.prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::Collider;
    use crate::rendering::framebuffer::Framebuffer;
    use crate::rendering::render::Renderer;
    use crate::scene::world::{WorldObject, WorldTransform};
    use raylib::prelude::{Matrix, Vector3};
    use std::cell::Cell;
    use std::rc::Rc;

    struct TestInteractable {
        transform: WorldTransform,
        interactions: Rc<Cell<u32>>,
    }

    impl WorldObject for TestInteractable {
        fn transform(&self) -> &WorldTransform {
            &self.transform
        }

        fn transform_mut(&mut self) -> &mut WorldTransform {
            &mut self.transform
        }

        fn collider(&self) -> Option<Collider> {
            Some(Collider::Sphere { radius: 1.0 })
        }

        fn interaction(&self) -> Option<Interaction> {
            Some(Interaction {
                prompt: "[F] Probar",
                response: "Probado",
            })
        }

        fn interact(&mut self) -> Option<Interaction> {
            self.interactions.set(self.interactions.get() + 1);
            self.interaction()
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
    fn selects_and_activates_the_object_at_the_center_of_view() {
        let interactions = Rc::new(Cell::new(0));
        let mut world = World::new();
        world.add(TestInteractable {
            transform: WorldTransform::new(
                Vector3::new(0.0, 0.0, 10.0),
                Vector3::new(1.0, 1.0, 1.0),
            ),
            interactions: Rc::clone(&interactions),
        });
        let uniforms = Uniforms {
            model_matrix: Matrix::identity(),
            camera_pos: Vector3::zero(),
            camera_target: Vector3::new(0.0, 0.0, 1.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),
            light_position: None,
        };
        let mut system = InteractionSystem::default();

        system.update(&mut world, &uniforms, true, 1.0 / 60.0);

        assert_eq!(system.prompt(), Some("[F] Probar"));
        assert_eq!(system.overlay(), Some("Probado"));
        assert_eq!(interactions.get(), 1);
    }
}
