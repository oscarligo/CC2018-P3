use crate::physics::{Collider, ObjectId, Ray, RayHit, raycast_sphere, spheres_overlap};
use crate::rendering::framebuffer::Framebuffer;
use crate::rendering::projection::sphere_is_visible;
use crate::rendering::render::Renderer;
use crate::rendering::uniforms::Uniforms;
use raylib::prelude::{Matrix, Quaternion, Vector3};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct TransformAnchor(Rc<RefCell<AnchorState>>);

struct AnchorState {
    local_position: Vector3,
    local_rotation: Quaternion,
    local_scale: Vector3,
    parent: Option<TransformAnchor>,
}

#[derive(Clone, Copy)]
struct WorldPose {
    position: Vector3,
    rotation: Quaternion,
    scale: Vector3,
}

impl TransformAnchor {
    fn new(position: Vector3, scale: Vector3) -> Self {
        Self(Rc::new(RefCell::new(AnchorState {
            local_position: position,
            local_rotation: Quaternion::identity(),
            local_scale: scale,
            parent: None,
        })))
    }

    fn world_pose(&self) -> WorldPose {
        let state = self.0.borrow();
        let local_position = state.local_position;
        let local_rotation = state.local_rotation;
        let local_scale = state.local_scale;
        let parent = state.parent.clone();
        drop(state);
        parent.map_or(
            WorldPose {
                position: local_position,
                rotation: local_rotation,
                scale: local_scale,
            },
            |parent| {
                let parent = parent.world_pose();
                WorldPose {
                    position: parent.position
                        + transform_direction(
                            &parent.rotation.to_matrix(),
                            multiply_components(local_position, parent.scale),
                        ),
                    rotation: (parent.rotation * local_rotation).normalize(),
                    scale: multiply_components(parent.scale, local_scale),
                }
            },
        )
    }

    fn set_local_pose(&self, position: Vector3, rotation: Quaternion, scale: Vector3) {
        let mut state = self.0.borrow_mut();
        state.local_position = position;
        state.local_rotation = rotation;
        state.local_scale = scale;
    }

    fn set_local_rotation(&self, rotation: Quaternion) {
        self.0.borrow_mut().local_rotation = rotation;
    }

    fn set_local_scale(&self, scale: Vector3) {
        self.0.borrow_mut().local_scale = scale;
    }

    fn parent_pose(&self) -> Option<WorldPose> {
        self.0
            .borrow()
            .parent
            .clone()
            .map(|parent| parent.world_pose())
    }

    fn set_world_scale_preserving_parent(&self, scale: Vector3) {
        let local_scale = self
            .parent_pose()
            .map_or(scale, |parent| divide_components(scale, parent.scale));
        self.set_local_scale(local_scale);
    }

    fn set_world_rotation_preserving_parent(&self, rotation: Quaternion) {
        let local_rotation = self.parent_pose().map_or(rotation, |parent| {
            (parent.rotation.invert() * rotation).normalize()
        });
        self.set_local_rotation(local_rotation);
    }

    fn contains(&self, target: &Self) -> bool {
        if Rc::ptr_eq(&self.0, &target.0) {
            return true;
        }
        self.0
            .borrow()
            .parent
            .as_ref()
            .is_some_and(|parent| parent.contains(target))
    }

    fn set_local_position(&self, position: Vector3) {
        self.0.borrow_mut().local_position = position;
    }

    fn local_position(&self) -> Vector3 {
        self.0.borrow().local_position
    }

    fn set_parent(&self, parent: Option<Self>) {
        if let Some(parent) = &parent {
            assert!(
                !parent.contains(self),
                "a transform cannot be attached to itself or one of its children"
            );
        }
        self.0.borrow_mut().parent = parent;
    }

    fn has_parent(&self) -> bool {
        self.0.borrow().parent.is_some()
    }
}

fn multiply_components(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

fn divide_components(value: Vector3, divisor: Vector3) -> Vector3 {
    fn divide(value: f32, divisor: f32) -> f32 {
        if divisor.abs() > f32::EPSILON {
            value / divisor
        } else {
            value
        }
    }
    Vector3::new(
        divide(value.x, divisor.x),
        divide(value.y, divisor.y),
        divide(value.z, divisor.z),
    )
}

fn transform_direction(matrix: &Matrix, direction: Vector3) -> Vector3 {
    Vector3::new(
        matrix.m0 * direction.x + matrix.m4 * direction.y + matrix.m8 * direction.z,
        matrix.m1 * direction.x + matrix.m5 * direction.y + matrix.m9 * direction.z,
        matrix.m2 * direction.x + matrix.m6 * direction.y + matrix.m10 * direction.z,
    )
}

/// Estado espacial común para cualquier entidad del mundo.
pub struct WorldTransform {
    pub position: Vector3,
    pub rotation: Quaternion,
    pub velocity: Vector3,
    pub scale: Vector3,
    orbit: Option<Orbit>,
    orbit_time: f32,
    anchor: TransformAnchor,
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
            rotation: Quaternion::identity(),
            velocity: Vector3::zero(),
            scale,
            orbit: None,
            orbit_time: 0.0,
            anchor: TransformAnchor::new(position, scale),
        }
    }

    pub(crate) fn anchor(&self) -> TransformAnchor {
        self.anchor.clone()
    }

    pub(crate) fn set_parent(&mut self, parent: TransformAnchor, local_position: Vector3) {
        self.orbit = None;
        self.orbit_time = 0.0;
        self.anchor.set_local_position(local_position);
        self.anchor.set_parent(Some(parent));
        self.anchor.set_world_scale_preserving_parent(self.scale);
        self.anchor
            .set_world_rotation_preserving_parent(self.rotation);
        self.resolve_world_transform();
    }

    pub(crate) fn set_local_rotation(&mut self, rotation: Quaternion) {
        self.anchor.set_local_rotation(rotation);
        self.resolve_world_transform();
    }

    pub fn set_local_scale(&mut self, scale: Vector3) {
        self.anchor.set_local_scale(scale);
        self.resolve_world_transform();
    }

    pub fn set_orbit(&mut self, orbit: Orbit) {
        self.anchor.set_parent(None);
        self.orbit = Some(orbit);
        self.orbit_time = 0.0;
        self.position = orbit.position(0.0);
        self.anchor
            .set_local_pose(self.position, self.rotation, self.scale);
    }

    fn advance(&mut self, update: &WorldUpdate) {
        if self.anchor.has_parent() {
            let local_position = self.anchor.local_position() + self.velocity * update.frame_time;
            self.anchor.set_local_position(local_position);
            return;
        }
        if let Some(orbit) = self.orbit {
            self.orbit_time += update.frame_time;
            self.position = orbit.position(self.orbit_time);
        }
        self.position += self.velocity * update.frame_time;
        self.anchor
            .set_local_pose(self.position, self.rotation, self.scale);
    }

    fn resolve_world_transform(&mut self) {
        let pose = self.anchor.world_pose();
        self.position = pose.position;
        self.rotation = pose.rotation;
        self.scale = pose.scale;
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
    fn collider(&self) -> Option<Collider> {
        None
    }
    fn render_pass(&self) -> RenderPass {
        RenderPass::Opaque
    }
    fn draw(&mut self, renderer: &mut Renderer, framebuffer: &mut Framebuffer, uniforms: &Uniforms);
}

struct WorldEntry {
    id: ObjectId,
    object: Box<dyn WorldObject>,
}

#[derive(Default)]
pub struct World {
    objects: Vec<WorldEntry>,
    next_object_id: u64,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<T: WorldObject + 'static>(&mut self, object: T) -> ObjectId {
        let id = ObjectId::new(self.next_object_id);
        self.next_object_id += 1;
        self.objects.push(WorldEntry {
            id,
            object: Box::new(object),
        });
        id
    }

    pub(crate) fn primary_light_position(&self) -> Option<Vector3> {
        self.objects
            .iter()
            .find_map(|entry| entry.object.light_position())
    }

    pub fn update(&mut self, update: &WorldUpdate) {
        for entry in &mut self.objects {
            entry.object.transform_mut().advance(update);
            entry.object.update(update);
        }
        for entry in &mut self.objects {
            entry.object.transform_mut().resolve_world_transform();
        }
    }

    pub fn raycast(&self, ray: Ray, max_distance: f32) -> Option<RayHit> {
        // shortcut: queries scan all colliders; add a spatial index when worlds contain hundreds of them.
        self.objects
            .iter()
            .filter_map(|entry| {
                let (center, radius) = collider_sphere(entry.object.as_ref())?;
                let distance = raycast_sphere(ray, center, radius, max_distance)?;
                Some(RayHit {
                    object_id: entry.id,
                    distance,
                    position: ray.origin + ray.direction * distance,
                })
            })
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    pub fn query_sphere(&self, center: Vector3, radius: f32, results: &mut Vec<ObjectId>) {
        results.clear();
        results.extend(self.objects.iter().filter_map(|entry| {
            let (object_center, object_radius) = collider_sphere(entry.object.as_ref())?;
            spheres_overlap(center, radius, object_center, object_radius).then_some(entry.id)
        }));
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
            distance_to_surface(a.object.as_ref(), camera_position)
                .total_cmp(&distance_to_surface(b.object.as_ref(), camera_position))
        });
    }
}

fn draw_pass(
    objects: &mut [WorldEntry],
    pass: RenderPass,
    reverse: bool,
    renderer: &mut Renderer,
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
) {
    let mut draw_object = |entry: &mut WorldEntry| {
        let object = &mut entry.object;
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

fn collider_sphere(object: &dyn WorldObject) -> Option<(Vector3, f32)> {
    let Collider::Sphere { radius } = object.collider()?;
    let transform = object.transform();
    let scale = transform
        .scale
        .x
        .abs()
        .max(transform.scale.y.abs())
        .max(transform.scale.z.abs());
    Some((transform.position, radius.abs() * scale))
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
    use std::f32::consts::PI;
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

        fn collider(&self) -> Option<Collider> {
            self.radius.map(|radius| Collider::Sphere { radius })
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
            .map(|entry| entry.object.transform().position.z)
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

        assert_eq!(world.objects[0].object.transform().position.z, 10.0);
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
    fn child_transform_follows_its_orbiting_parent() {
        let center = Vector3::new(0.0, 0.0, 5.0);
        let initial = Vector3::new(10.0, 2.0, 5.0);
        let orbit = Orbit::new(center, initial, Vector3::new(0.2, 1.0, 0.1), 0.5);
        let mut parent = WorldTransform::new(initial, Vector3::new(1.0, 1.0, 1.0));
        let mut child = WorldTransform::new(Vector3::zero(), Vector3::new(1.1, 1.1, 1.1));
        let offset = Vector3::new(2.0, -1.0, 3.0);
        parent.set_orbit(orbit);
        child.set_parent(parent.anchor(), offset);

        for _ in 0..120 {
            let update = WorldUpdate {
                frame_time: 1.0 / 60.0,
            };
            child.advance(&update);
            parent.advance(&update);
            parent.resolve_world_transform();
            child.resolve_world_transform();
        }

        assert!((child.position - parent.position - offset).length() < 0.0001);
        assert!(((parent.position - center).length() - (initial - center).length()).abs() < 0.001);
    }

    #[test]
    fn nested_transforms_resolve_independently_of_update_order() {
        let scale = Vector3::new(1.0, 1.0, 1.0);
        let mut root = WorldTransform::new(Vector3::new(10.0, 0.0, 0.0), scale);
        let mut child = WorldTransform::new(Vector3::zero(), scale);
        let mut grandchild = WorldTransform::new(Vector3::zero(), scale);
        child.set_parent(root.anchor(), Vector3::new(2.0, 0.0, 0.0));
        grandchild.set_parent(child.anchor(), Vector3::new(0.0, 3.0, 0.0));
        root.velocity = Vector3::new(5.0, 0.0, 0.0);
        let update = WorldUpdate { frame_time: 1.0 };

        grandchild.advance(&update);
        child.advance(&update);
        root.advance(&update);
        grandchild.resolve_world_transform();

        assert_eq!(grandchild.position, Vector3::new(17.0, 3.0, 0.0));
    }

    #[test]
    fn children_inherit_parent_rotation_and_scale() {
        let scale = Vector3::new(1.0, 1.0, 1.0);
        let mut parent = WorldTransform::new(Vector3::new(5.0, 0.0, 0.0), scale);
        let mut child = WorldTransform::new(Vector3::zero(), scale);
        child.set_parent(parent.anchor(), Vector3::new(1.0, 0.0, 0.0));
        parent.set_local_rotation(Quaternion::from_axis_angle(
            Vector3::new(0.0, 1.0, 0.0),
            PI / 2.0,
        ));
        parent.set_local_scale(Vector3::new(2.0, 2.0, 2.0));
        parent.advance(&WorldUpdate { frame_time: 0.0 });
        child.resolve_world_transform();

        assert!(
            ((child.position - parent.position).length() - 2.0).abs() < 0.0001,
            "parent={:?} child={:?}",
            parent.position,
            child.position
        );
        assert_eq!(child.scale, Vector3::new(2.0, 2.0, 2.0));
        let child_forward =
            transform_direction(&child.rotation.to_matrix(), Vector3::new(0.0, 0.0, 1.0));
        assert!((child_forward - Vector3::new(1.0, 0.0, 0.0)).length() < 0.0001);
    }

    #[test]
    fn spatial_queries_return_stable_object_ids_after_sorting() {
        let mut world = World::new();
        let near = world.add(TestObject {
            transform: WorldTransform::new(
                Vector3::new(0.0, 0.0, 10.0),
                Vector3::new(1.0, 1.0, 1.0),
            ),
            radius: Some(2.0),
        });
        let far = world.add(TestObject {
            transform: WorldTransform::new(
                Vector3::new(0.0, 0.0, 20.0),
                Vector3::new(1.0, 1.0, 1.0),
            ),
            radius: Some(2.0),
        });
        world.sort_front_to_back(Vector3::new(0.0, 0.0, 30.0));

        let ray = Ray::new(Vector3::zero(), Vector3::new(0.0, 0.0, 1.0)).unwrap();
        let hit = world.raycast(ray, 100.0).unwrap();
        assert_eq!(hit.object_id, near);
        assert_eq!(hit.distance, 8.0);
        assert_eq!(hit.position, Vector3::new(0.0, 0.0, 8.0));

        let mut nearby = vec![near];
        world.query_sphere(Vector3::new(0.0, 0.0, 20.0), 1.0, &mut nearby);
        assert_eq!(nearby, [far]);
    }
}
