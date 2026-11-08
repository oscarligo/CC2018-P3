use crate::fragment::Fragment;
use crate::triangle::{is_back_facing, triangle_scanline};
use crate::vertex::Vertex;

pub fn rasterize_indexed(
    vertices: &[Vertex],
    indices: &[u32],
    width: u32,
    height: u32,
    fragments: &mut Vec<Fragment>,
) -> usize {
    fragments.clear();
    let mut culled_triangles = 0;

    for indices in indices.chunks_exact(3) {
        let a = &vertices[indices[0] as usize];
        let b = &vertices[indices[1] as usize];
        let c = &vertices[indices[2] as usize];
        if is_back_facing(a, b, c) {
            culled_triangles += 1;
            continue;
        }
        fragments.extend(triangle_scanline(a, b, c, width, height));
    }

    culled_triangles
}
