use raylib::prelude::*;

pub struct Uniforms {
    pub model_matrix: Matrix,
    pub camera_pos: Vector3,
    pub camera_target: Vector3,
}
