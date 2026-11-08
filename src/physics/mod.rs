mod collider;

pub(crate) use collider::{Collider, Ray, RayHit, raycast_sphere, spheres_overlap};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(u64);

impl ObjectId {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}
