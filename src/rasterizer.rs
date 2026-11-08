use crate::fragment::Fragment;
use crate::triangle::{is_back_facing, triangle_scanline};
use crate::vertex::Vertex;

pub struct RasterizationStats {
    pub culled_triangles: usize,
    pub covered_fragments: usize,
}

pub fn rasterize_indexed(
    vertices: &[Vertex],
    indices: &[u32],
    width: u32,
    height: u32,
    mut emit: impl FnMut(Fragment),
) -> RasterizationStats {
    let mut culled_triangles = 0;
    let mut covered_fragments = 0;

    for indices in indices.chunks_exact(3) {
        let a = &vertices[indices[0] as usize];
        let b = &vertices[indices[1] as usize];
        let c = &vertices[indices[2] as usize];
        if is_back_facing(a, b, c) {
            culled_triangles += 1;
            continue;
        }
        covered_fragments += triangle_scanline(a, b, c, width, height, &mut emit);
    }

    RasterizationStats {
        culled_triangles,
        covered_fragments,
    }
}
