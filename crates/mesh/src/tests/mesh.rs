use crate::{FaceRange, Mesh, MeshBuilder};

fn vertices() -> Vec<[f32; 3]> {
    vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
}

fn triangles() -> Vec<[u32; 3]> {
    vec![[0, 1, 2]]
}

fn faces() -> Vec<FaceRange> {
    vec![FaceRange {
        vertex_start: 0,
        vertex_count: 3,
        index_start: 0,
        index_count: 3,
    }]
}

fn builder() -> MeshBuilder {
    MeshBuilder::default()
        .with_vertices(vertices())
        .with_triangles(triangles())
        .with_faces(faces())
}

#[test]
fn given_complete_builder_when_build_then_returns_mesh() {
    // given
    let builder = builder();

    // when
    let mesh = builder.build().unwrap();

    // then
    assert_eq!(mesh.vertices(), vertices());
    assert_eq!(mesh.triangles(), triangles());
}

#[test]
fn given_builder_without_optionals_when_build_then_optionals_are_none() {
    // given
    let builder = builder();

    // when
    let mesh = builder.build().unwrap();

    // then
    assert_eq!(mesh.uvs(), None);
    assert_eq!(mesh.normals(), None);
}

#[test]
fn given_builder_with_uvs_when_build_then_uvs_are_preserved() {
    // given
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];

    // when
    let mesh = builder().with_uvs(uvs.clone()).build().unwrap();

    // then
    assert_eq!(mesh.uvs(), Some(&uvs[..]));
}

#[test]
fn given_repeated_with_vertices_when_build_then_last_value_wins() {
    // given
    let replacement = vec![[1.0, 1.0, 1.0], [2.0, 1.0, 1.0], [1.0, 2.0, 1.0]];

    // when
    let mesh = builder()
        .with_vertices(replacement.clone())
        .build()
        .unwrap();

    // then
    assert_eq!(mesh.vertices(), replacement);
}

#[test]
fn given_equivalent_builders_when_comparing_build_and_try_from_then_results_match() {
    // given
    let built = builder().build().unwrap();

    // when
    let tried = Mesh::try_from(builder()).unwrap();

    // then
    assert_eq!(built.vertices(), tried.vertices());
    assert_eq!(built.triangles(), tried.triangles());
}

#[test]
fn given_missing_vertices_when_build_then_error_reports_them() {
    // given
    let builder = MeshBuilder::default().with_triangles(triangles());

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("no vertices defined"));
}

#[test]
fn given_fewer_than_three_vertices_when_build_then_error_states_the_minimum() {
    // given
    let builder = MeshBuilder::default()
        .with_vertices(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]])
        .with_triangles(triangles());

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("at least 3 vertices"));
}

#[test]
fn given_missing_triangles_when_build_then_error_reports_them() {
    // given
    let builder = MeshBuilder::default().with_vertices(vertices());

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("no triangles defined"));
}

#[test]
fn given_empty_triangles_when_build_then_error_states_the_minimum() {
    // given
    let builder = builder().with_triangles(vec![]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("at least 1 triangle"));
}

#[test]
fn given_triangle_index_beyond_vertices_when_build_then_error_names_the_index() {
    // given
    let builder = builder().with_triangles(vec![[0, 1, 99]]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("99"));
}

#[test]
fn given_normal_count_mismatch_when_build_then_error_reports_normals() {
    // given
    let builder = builder().with_normals(vec![[0.0, 0.0, 1.0]; 2]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("normals"));
}

#[test]
fn given_uv_count_mismatch_when_build_then_error_reports_uvs() {
    // given
    let builder = builder().with_uvs(vec![[0.0, 0.0]]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("uv"));
}

fn two_face_builder() -> MeshBuilder {
    MeshBuilder::default()
        .with_vertices(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
        ])
        .with_triangles(vec![[0, 1, 2], [3, 4, 5]])
}

#[test]
fn given_two_faces_when_build_then_faces_are_preserved() {
    // given
    let builder = two_face_builder().with_faces(vec![
        FaceRange {
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        },
        FaceRange {
            vertex_start: 3,
            vertex_count: 3,
            index_start: 3,
            index_count: 3,
        },
    ]);

    // when
    let mesh = builder.build().unwrap();

    // then
    assert_eq!(mesh.faces().len(), 2);
    assert_eq!(mesh.faces()[1].vertex_start, 3);
}

#[test]
fn given_missing_faces_when_build_then_error_reports_them() {
    // given
    let builder = MeshBuilder::default()
        .with_vertices(vertices())
        .with_triangles(triangles());

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("no face ranges defined"));
}

#[test]
fn given_empty_faces_when_build_then_error_reports_coverage() {
    // given
    let builder = builder().with_faces(vec![]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("cover 0 of 3 vertices"));
}

#[test]
fn given_face_index_count_not_multiple_of_three_when_build_then_error_reports_the_face() {
    // given
    let builder = builder().with_faces(vec![FaceRange {
        vertex_start: 0,
        vertex_count: 3,
        index_start: 0,
        index_count: 4,
    }]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("not a multiple of 3"));
}

#[test]
fn given_face_beyond_vertex_buffer_when_build_then_error_reports_the_range() {
    // given
    let builder = builder().with_faces(vec![FaceRange {
        vertex_start: 0,
        vertex_count: 4,
        index_start: 0,
        index_count: 3,
    }]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("exceed 3 vertices"));
}

#[test]
fn given_face_beyond_index_buffer_when_build_then_error_reports_the_range() {
    // given
    let builder = builder().with_faces(vec![FaceRange {
        vertex_start: 0,
        vertex_count: 3,
        index_start: 0,
        index_count: 6,
    }]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("exceed 3 indices"));
}

#[test]
fn given_unchained_face_ranges_when_build_then_error_reports_the_expected_start() {
    // given
    let builder = two_face_builder().with_faces(vec![
        FaceRange {
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        },
        FaceRange {
            vertex_start: 3,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        },
    ]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(
        error
            .to_string()
            .contains("starts at index 0 but 3 was expected")
    );
}

#[test]
fn given_partial_index_coverage_when_build_then_error_reports_the_shortfall() {
    // given
    let builder = two_face_builder().with_faces(vec![FaceRange {
        vertex_start: 0,
        vertex_count: 6,
        index_start: 0,
        index_count: 3,
    }]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("cover 3 of 6 indices"));
}

#[test]
fn given_partial_vertex_coverage_when_build_then_error_reports_the_shortfall() {
    // given
    let builder = MeshBuilder::default()
        .with_vertices(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ])
        .with_triangles(vec![[0, 1, 2]])
        .with_faces(faces());

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("cover 3 of 4 vertices"));
}

#[test]
fn given_vertex_gap_between_faces_when_build_then_error_reports_the_expected_start() {
    // given
    let builder = two_face_builder().with_faces(vec![
        FaceRange {
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        },
        FaceRange {
            vertex_start: 4,
            vertex_count: 2,
            index_start: 3,
            index_count: 3,
        },
    ]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(
        error
            .to_string()
            .contains("starts at vertex 4 but 3 was expected")
    );
}
