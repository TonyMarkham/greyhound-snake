use crate::{OcctBounds, ProjectionSettings, UnityVertex, project};

use mesh::{FaceAttrib, FaceRange, Mesh, MeshBuilder};

fn attribs(solid: u32, color: u32, count: usize) -> Vec<FaceAttrib> {
    vec![FaceAttrib { solid, color }; count]
}

fn mesh_with_two_faces() -> Mesh {
    MeshBuilder::default()
        .with_vertices(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 3.0],
            [3.0, 0.0, 3.0],
            [2.0, 1.0, 3.0],
        ])
        .with_normals(vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
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
        .with_face_attribs(attribs(0, 0, 2))
        .with_colors(vec![[0.976, 0.678, 0.122, 1.0]])
        .build()
        .unwrap()
}

fn bounds() -> OcctBounds {
    OcctBounds {
        min: [-10.0, -25.0, -0.5],
        max: [10.0, 25.0, 11.0],
    }
}

#[test]
fn given_mesh_when_projected_then_positions_are_permuted_and_scaled() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 0.001 }).unwrap();

    // then
    assert_eq!(projected.vertices()[0].position, [0.0, 0.0, 0.0]);
    assert_eq!(projected.vertices()[1].position, [0.001, 0.0, 0.0]);
    assert_eq!(projected.vertices()[2].position, [0.0, 0.0, 0.001]);
    assert_eq!(projected.vertices()[3].position, [0.002, 0.003, 0.0]);
}

#[test]
fn given_mesh_when_projected_then_normals_are_permuted_but_never_negated() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 }).unwrap();

    // then
    assert_eq!(projected.vertices()[0].normal, [0.0, 1.0, 0.0]);
    assert_eq!(projected.vertices()[3].normal, [-1.0, 0.0, 0.0]);
}

#[test]
fn given_mesh_when_projected_then_every_triangle_has_two_indices_swapped() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 }).unwrap();

    // then
    assert_eq!(projected.indices(), &[0, 2, 1, 3, 5, 4]);
}

#[test]
fn given_faces_sharing_solid_and_color_when_projected_then_one_submesh_per_group() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 }).unwrap();

    // then
    let submeshes = projected.submeshes();
    assert_eq!(submeshes.len(), 1);
    assert_eq!(submeshes[0].index_start, 0);
    assert_eq!(submeshes[0].index_count, 6);
    assert_eq!(submeshes[0].first_vertex, 0);
    assert_eq!(submeshes[0].vertex_count, 6);
    assert_eq!(projected.submesh_colors(), &[0]);
}

#[test]
fn given_faces_with_distinct_solids_when_projected_then_one_submesh_per_solid() {
    // given
    let mesh = MeshBuilder::default()
        .with_vertices(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 3.0],
            [3.0, 0.0, 3.0],
            [2.0, 1.0, 3.0],
        ])
        .with_normals(vec![[0.0, 0.0, 1.0]; 6])
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
            FaceAttrib { solid: 0, color: 0 },
            FaceAttrib { solid: 1, color: 0 },
        ])
        .with_colors(vec![[0.976, 0.678, 0.122, 1.0]])
        .build()
        .unwrap();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 }).unwrap();

    // then
    let submeshes = projected.submeshes();
    assert_eq!(submeshes.len(), 2);
    assert_eq!(submeshes[0].index_start, 0);
    assert_eq!(submeshes[0].index_count, 3);
    assert_eq!(submeshes[1].index_start, 3);
    assert_eq!(submeshes[1].index_count, 3);
    assert_eq!(projected.submesh_colors(), &[0, 0]);
}

#[test]
fn given_interleaved_groups_when_projected_then_indices_reorder_into_contiguous_submeshes() {
    // given: face A (solid 0, red), face B (solid 1, green), face C (solid 0, red)
    let mesh = MeshBuilder::default()
        .with_vertices(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [4.0, 0.0, 0.0],
            [5.0, 0.0, 0.0],
            [4.0, 1.0, 0.0],
        ])
        .with_normals(vec![[0.0, 0.0, 1.0]; 9])
        .with_triangles(vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]])
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
            FaceRange {
                vertex_start: 6,
                vertex_count: 3,
                index_start: 6,
                index_count: 3,
            },
        ])
        .with_face_attribs(vec![
            FaceAttrib { solid: 0, color: 0 },
            FaceAttrib { solid: 1, color: 1 },
            FaceAttrib { solid: 0, color: 0 },
        ])
        .with_colors(vec![[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]])
        .build()
        .unwrap();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 }).unwrap();

    // then: faces A and C regroup into the first submesh; winding swaps hold
    assert_eq!(projected.submeshes().len(), 2);
    assert_eq!(projected.submesh_colors(), &[0, 1]);
    assert_eq!(projected.indices(), &[0, 2, 1, 6, 8, 7, 3, 5, 4]);
    assert_eq!(projected.submeshes()[0].index_start, 0);
    assert_eq!(projected.submeshes()[0].index_count, 6);
    assert_eq!(projected.submeshes()[1].index_start, 6);
    assert_eq!(projected.submeshes()[1].index_count, 3);
    assert_eq!(
        projected.colors(),
        &[[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]]
    );
}

#[test]
fn given_mesh_without_face_attribs_when_projected_then_error_requires_attribs() {
    // given
    let mesh = MeshBuilder::default()
        .with_vertices(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        .with_normals(vec![[0.0, 0.0, 1.0]; 3])
        .with_triangles(vec![[0, 1, 2]])
        .with_faces(vec![FaceRange {
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        }])
        .with_colors(vec![[1.0, 1.0, 1.0, 1.0]])
        .build()
        .unwrap();

    // when
    let result = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 });

    // then
    let error = result.unwrap_err().to_string();
    assert!(error.contains("no face attribs"), "{error}");
}

#[test]
fn given_occt_bounds_when_projected_then_unity_bounds_are_permuted_and_scaled() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 0.001 }).unwrap();

    // then
    let unity_bounds = projected.bounds();
    assert_eq!(unity_bounds.min, [-0.010, -0.000_5, -0.025]);
    assert_eq!(unity_bounds.max, [0.010, 0.011, 0.025]);
}

#[test]
fn given_mesh_when_projected_then_vertex_buffer_is_interleaved_24_byte_layout() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let projected = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 }).unwrap();

    // then
    assert_eq!(projected.vertices().len(), mesh.vertices().len());
    assert_eq!(std::mem::size_of::<UnityVertex>(), 24);
    assert_eq!(projected.indices().len(), mesh.triangles().len() * 3);
}

#[test]
fn given_default_settings_when_inspected_then_scale_encodes_millimetres_to_metres() {
    // given
    let settings = ProjectionSettings::default();

    // when
    let scale = settings.scale;

    // then
    assert_eq!(scale, 0.001);
}

#[test]
fn given_non_positive_scale_when_projected_then_error_reports_the_scale() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let result = project(&mesh, bounds(), ProjectionSettings { scale: 0.0 });

    // then
    let error = result.unwrap_err().to_string();
    assert!(error.contains("must be finite and positive"), "{error}");
}

#[test]
fn given_negative_scale_when_projected_then_error_reports_the_scale() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let result = project(&mesh, bounds(), ProjectionSettings { scale: -1.0 });

    // then
    assert!(result.is_err());
}

#[test]
fn given_nan_scale_when_projected_then_error_reports_the_scale() {
    // given
    let mesh = mesh_with_two_faces();

    // when
    let result = project(&mesh, bounds(), ProjectionSettings { scale: f64::NAN });

    // then
    assert!(result.is_err());
}

#[test]
fn given_mesh_without_normals_when_projected_then_error_requires_normals() {
    // given
    let mesh = MeshBuilder::default()
        .with_vertices(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        .with_triangles(vec![[0, 1, 2]])
        .with_faces(vec![FaceRange {
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        }])
        .build()
        .unwrap();

    // when
    let result = project(&mesh, bounds(), ProjectionSettings { scale: 1.0 });

    // then
    let error = result.unwrap_err().to_string();
    assert!(error.contains("no normals"), "{error}");
}
