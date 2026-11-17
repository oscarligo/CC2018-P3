use raylib::prelude::{Vector2, Vector3};

#[derive(Clone, Copy, Debug)]
pub struct SurfacePoint {
    pub position: Vector3,
    pub normal: Vector3,
}

#[derive(Clone, Debug)]
pub struct Fragment {
    pub surface: Option<SurfacePoint>,
    pub position: Vector2,
    pub color: Vector3,
    pub alpha: f32,
    pub depth: f32,
    pub tex_coord: Option<Vector2>,
    pub material_index: usize,
}
