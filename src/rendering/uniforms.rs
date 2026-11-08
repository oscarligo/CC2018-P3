use raylib::prelude::*;

#[derive(Clone, Copy)]
pub struct Uniforms {
    pub model_matrix: Matrix,
    pub camera_pos: Vector3,
    pub camera_target: Vector3,
    pub camera_up: Vector3,
    pub light_position: Option<Vector3>,
}
