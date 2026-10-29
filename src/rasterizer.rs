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

pub fn rasterize_indexed(
    vertices: &[Vertex],
    indices: &[u32],
    mode: TriangleMode,
    fragments: &mut Vec<Fragment>,
) {
    fragments.clear();

    for indices in indices.chunks_exact(3) {
        let a = &vertices[indices[0] as usize];
        let b = &vertices[indices[1] as usize];
        let c = &vertices[indices[2] as usize];
        fragments.extend(match mode {
            TriangleMode::Wireframe => triangle(a, b, c),
            TriangleMode::FilledBarycentric => triangle_barycentric(a, b, c),
            TriangleMode::FilledScanline => triangle_scanline(a, b, c),
        });
    }
}
