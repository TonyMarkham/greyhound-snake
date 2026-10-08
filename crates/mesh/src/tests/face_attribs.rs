use crate::{FaceAttrib, FaceRange, MeshBuilder};

use super::mesh::{faces, triangles, vertices};

fn builder() -> MeshBuilder {
    MeshBuilder::default()
        .with_vertices(vertices())
        .with_triangles(triangles())
        .with_faces(faces())
}

fn attribs() -> Vec<FaceAttrib> {
    vec![FaceAttrib { solid: 0, color: 0 }]
}

#[test]
fn given_attribs_and_colors_when_build_then_both_are_preserved() {
    // given
    let colors = vec![[0.976, 0.678, 0.122, 1.0]];

    // when
    let mesh = builder()
        .with_face_attribs(attribs())
        .with_colors(colors.clone())
        .build()
        .unwrap();

    // then
    assert_eq!(mesh.face_attribs(), Some(&attribs()[..]));
    assert_eq!(mesh.colors(), Some(&colors[..]));
}

#[test]
fn given_builder_without_attribs_when_build_then_attribs_are_none() {
    // given
    let builder = builder();

    // when
    let mesh = builder.build().unwrap();

    // then
    assert_eq!(mesh.face_attribs(), None);
    assert_eq!(mesh.colors(), None);
}

#[test]
fn given_attrib_count_mismatch_when_build_then_error_reports_the_mismatch() {
    // given
    let builder = builder()
        .with_face_attribs(vec![
            FaceAttrib { solid: 0, color: 0 },
            FaceAttrib { solid: 0, color: 0 },
        ])
        .with_colors(vec![[1.0, 1.0, 1.0, 1.0]]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(
        error
            .to_string()
            .contains("face attrib count 2 does not match 1 faces")
    );
}

#[test]
fn given_color_index_beyond_table_when_build_then_error_reports_the_color() {
    // given
    let builder = builder()
        .with_face_attribs(vec![FaceAttrib { solid: 0, color: 1 }])
        .with_colors(vec![[1.0, 1.0, 1.0, 1.0]]);

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("references color 1 of 1"));
}

#[test]
fn given_attribs_without_colors_when_build_then_error_reports_the_missing_color() {
    // given
    let builder = builder().with_face_attribs(attribs());

    // when
    let error = builder.build().unwrap_err();

    // then
    assert!(error.to_string().contains("references color 0 of 0"));
}

#[test]
fn given_no_solid_sentinel_when_read_then_it_is_u32_max() {
    // given / when / then
    assert_eq!(FaceAttrib::NO_SOLID, u32::MAX);
}

#[test]
fn given_two_faces_with_attribs_when_build_then_attribs_follow_face_order() {
    // given
    let mesh = MeshBuilder::default()
        .with_vertices(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ])
        .with_triangles(vec![[0, 1, 2], [3, 4, 5]])
        .with_faces(vec![
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
        ])
        .with_face_attribs(vec![
            FaceAttrib { solid: 0, color: 1 },
            FaceAttrib {
                solid: FaceAttrib::NO_SOLID,
                color: 0,
            },
        ])
        .with_colors(vec![[1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 1.0, 1.0]])
        .build()
        .unwrap();

    // when
    let attribs = mesh.face_attribs().unwrap();

    // then
    assert_eq!(attribs[0].solid, 0);
    assert_eq!(attribs[0].color, 1);
    assert_eq!(attribs[1].solid, FaceAttrib::NO_SOLID);
    assert_eq!(attribs[1].color, 0);
}
