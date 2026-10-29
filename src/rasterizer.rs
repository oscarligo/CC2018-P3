use crate::fragment::Fragment;
use crate::triangle::{triangle, triangle_barycentric, triangle_scanline};
use crate::vertex::Vertex;

#[derive(Clone, Copy, Debug)]
pub enum TriangleMode {
    Wireframe,
    FilledBarycentric,
    FilledScanline,
}

pub fn rasterize(triangles: &[[Vertex; 3]], mode: TriangleMode) -> Vec<Fragment> {
    let mut fragments = Vec::new();

    for tri in triangles {
        fragments.extend(match mode {
            TriangleMode::Wireframe => triangle(&tri[0], &tri[1], &tri[2]),
            TriangleMode::FilledBarycentric => triangle_barycentric(&tri[0], &tri[1], &tri[2]),
            TriangleMode::FilledScanline => triangle_scanline(&tri[0], &tri[1], &tri[2]),
        });
    }

    fragments
}
