use super::ObjectId;
use raylib::prelude::Vector3;

#[derive(Clone, Copy, Debug)]
pub enum Collider {
    Sphere { radius: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
}

impl Ray {
    pub fn new(origin: Vector3, direction: Vector3) -> Option<Self> {
        let length = direction.length();
        (length > f32::EPSILON).then(|| Self {
            origin,
            direction: direction * (1.0 / length),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub object_id: ObjectId,
    pub distance: f32,
    pub position: Vector3,
}

pub(crate) fn raycast_sphere(
    ray: Ray,
    center: Vector3,
    radius: f32,
    max_distance: f32,
) -> Option<f32> {
    let offset = ray.origin - center;
    let projection = offset.dot(ray.direction);
    let discriminant = projection * projection - (offset.dot(offset) - radius * radius);
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = -projection - root;
    let far = -projection + root;
    let distance = if near >= 0.0 { near } else { far };
    (distance >= 0.0 && distance <= max_distance).then_some(distance)
}

pub(crate) fn spheres_overlap(
    first_center: Vector3,
    first_radius: f32,
    second_center: Vector3,
    second_radius: f32,
) -> bool {
    let radius = first_radius + second_radius;
    (first_center - second_center).length_sqr() <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raycast_returns_the_nearest_surface() {
        let ray = Ray::new(Vector3::zero(), Vector3::new(0.0, 0.0, 2.0)).unwrap();
        assert_eq!(
            raycast_sphere(ray, Vector3::new(0.0, 0.0, 10.0), 2.0, 20.0),
            Some(8.0)
        );
        assert!(raycast_sphere(ray, Vector3::new(5.0, 0.0, 10.0), 2.0, 20.0).is_none());
    }

    #[test]
    fn raycast_from_inside_uses_the_exit_surface() {
        let ray = Ray::new(Vector3::zero(), Vector3::new(1.0, 0.0, 0.0)).unwrap();
        assert_eq!(raycast_sphere(ray, Vector3::zero(), 3.0, 10.0), Some(3.0));
    }
}
