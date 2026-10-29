use crate::vertex::Vertex;

pub fn primitive_assembly(transformed_vertices: &[Vertex]) -> Vec<[Vertex; 3]> {
    assert!(
        transformed_vertices.len() % 3 == 0,
        "The number of transformed vertices must be a multiple of 3 for triangle assembly.",
    );

    transformed_vertices
        .chunks_exact(3)
        .map(|group| [group[0].clone(), group[1].clone(), group[2].clone()])
        .collect()
}
