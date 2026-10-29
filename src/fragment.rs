use raylib::prelude::{Vector2, Vector3};

#[derive(Clone, Debug)]
pub struct Fragment {
    pub position: Vector2,
    pub color: Vector3,
    pub depth: f32,
}
