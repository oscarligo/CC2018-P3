use raylib::prelude::{Vector2, Vector3};

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vector3,
    pub transformed_position: Vector3,
    pub screen_position: Option<Vector2>,
    pub camera_depth: f32,
    pub color: Vector3,
    pub tex_coord: Option<Vector2>,
    pub material_index: usize,
}

impl Vertex {
    pub fn new(position: Vector3) -> Self {
        Self {
            position,
            transformed_position: position,
            screen_position: None,
            camera_depth: f32::INFINITY,
            color: Vector3::new(1.0, 1.0, 1.0),
            tex_coord: None,
            material_index: 0,
        }
    }
}
