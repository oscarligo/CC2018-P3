use raylib::prelude::{Vector2, Vector3};

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vector3,
    pub transformed_position: Vector3,
    pub screen_position: Option<Vector2>,
    pub color: Vector3,
}

impl Vertex {
    pub fn new(position: Vector3) -> Self {
        Self {
            position,
            transformed_position: position,
            screen_position: None,
            color: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
