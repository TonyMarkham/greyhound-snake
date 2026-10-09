use crate::{
    assemble::{AssembleOptions, assemble},
    error::Error,
    mjcf::write_model,
    stl::write_mesh,
};

use mesh::{FaceRange, MeshBuilder, MeshProperties, Node, Scene};

const TOLERANCE: f64 = 1e-12;

fn identity() -> [f32; 12] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0,
    ]
}

fn translated(translation: [f32; 3]) -> [f32; 12] {
    [
        1.0,
        0.0,
        0.0,
        translation[0], //
        0.0,
        1.0,
        0.0,
        translation[1], //
        0.0,
        0.0,
        1.0,
        translation[2],
    ]
}

fn rotated_z_quarter() -> [f32; 12] {
    [
        0.0, -1.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0,
    ]
}

fn scaled_twice() -> [f32; 12] {
    [
        2.0, 0.0, 0.0, 0.0, //
        0.0, 2.0, 0.0, 0.0, //
        0.0, 0.0, 2.0, 0.0,
    ]
}

fn mirrored_x() -> [f32; 12] {
    [
        -1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0,
    ]
}

fn nan_translation() -> [f32; 12] {
    [
        1.0,
        0.0,
        0.0,
        f32::NAN, //
        0.0,
        1.0,
        0.0,
        0.0, //
        0.0,
        0.0,
        1.0,
        0.0,
    ]
}

fn node(parent: u32, mesh: u32, name: (&str, u32, u32), transform: [f32; 12]) -> Node {
    Node {
        parent,
        mesh,
        name_offset: name.1,
        name_length: name.2,
        transform,
    }
}

fn single_root_scene(transform: [f32; 12]) -> Scene {
    Scene::try_new(
        vec![node(Node::NO_PARENT, 0, ("box", 0, 3), transform)],
        1,
        b"box".to_vec(),
    )
    .unwrap()
}

fn grouped_scene() -> Scene {
    Scene::try_new(
        vec![
            node(Node::NO_PARENT, Node::NO_MESH, ("asm", 0, 3), identity()),
            node(0, 0, ("part", 3, 4), translated([17.0, 18.0, -19.0])),
            node(0, 0, ("part", 7, 4), rotated_z_quarter()),
        ],
        1,
        b"asmpartpart".to_vec(),
    )
    .unwrap()
}

fn two_root_scene() -> Scene {
    Scene::try_new(
        vec![
            node(Node::NO_PARENT, Node::NO_MESH, ("a", 0, 1), identity()),
            node(Node::NO_PARENT, Node::NO_MESH, ("b", 1, 1), identity()),
        ],
        0,
        b"ab".to_vec(),
    )
    .unwrap()
}

/// The 2x1x1 mm fixture box spanning [0,2]x[0,1]x[0,1]: 8 corners, 12
/// outward-CCW triangles (right-hand rule points outward in OCCT algebra).
fn box_mesh() -> mesh::Mesh {
    let vertices = vec![
        [0.0, 0.0, 0.0], //
        [2.0, 0.0, 0.0], //
        [2.0, 1.0, 0.0], //
        [0.0, 1.0, 0.0], //
        [0.0, 0.0, 1.0], //
        [2.0, 0.0, 1.0], //
        [2.0, 1.0, 1.0], //
        [0.0, 1.0, 1.0],
    ];
    let triangles = vec![
        [0, 3, 2], //
        [0, 2, 1], // bottom, outward -Z
        [4, 5, 6], //
        [4, 6, 7], // top, outward +Z
        [0, 1, 5], //
        [0, 5, 4], // front y=0, outward -Y
        [3, 6, 2], //
        [3, 7, 6], // back y=1, outward +Y
        [0, 7, 3], //
        [0, 4, 7], // left x=0, outward -X
        [1, 2, 6], //
        [1, 6, 5], // right x=2, outward +X
    ];
    MeshBuilder::default()
        .with_vertices(vertices)
        .with_triangles(triangles)
        .with_faces(vec![FaceRange {
            vertex_start: 0,
            vertex_count: 8,
            index_start: 0,
            index_count: 36,
        }])
        .build()
        .unwrap()
}

/// Exact-BRep properties of the 2x1x1 box at unit density: moments are the
/// density-1 volume integrals V/12*(b^2+c^2) etc. for extents (a,b,c)=(2,1,1),
/// matched to the axis-aligned principal axes.
fn box_properties() -> MeshProperties {
    MeshProperties {
        volume_mm3: 2.0,
        centre_of_gravity: [1.0, 0.5, 0.5],
        principal_axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        principal_moments: [1.0 / 3.0, 5.0 / 6.0, 5.0 / 6.0],
        file_density: 0.0,
    }
}

fn options(free_root: bool) -> AssembleOptions {
    AssembleOptions {
        model_name: "box-2x1x1".to_owned(),
        scale: 0.001,
        free_root,
        density_parameter: 1.0,
        deflection: 0.01,
    }
}

fn assert_quat_close(actual: &[f64; 4], expected: &[f64; 4]) {
    for (axis, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < TOLERANCE,
            "axis {axis}: {actual} vs {expected}"
        );
    }
}

#[test]
fn given_the_fixture_box_when_written_then_the_mjcf_matches_the_reference() {
    let forest = single_root_scene(identity());
    let model = assemble(&forest, &[box_properties()], &options(false)).unwrap();

    let mut text = Vec::new();
    write_model(&mut text, &model).unwrap();
    let text = String::from_utf8(text).unwrap();

    assert_eq!(
        text,
        r#"<?xml version="1.0" encoding="utf-8"?>
<!-- exported by greyhound-export-mjc from box-2x1x1; density 1.0 g/cm3, scale 0.001, free-root off, deflection 0.01 -->
<mujoco model="box-2x1x1">
  <compiler angle="radian" meshdir="meshes" autolimits="true" inertiafromgeom="auto" balanceinertia="false"/>
  <asset>
    <!-- first used by: box -->
    <mesh name="mesh_000" file="mesh_000.stl"/>
  </asset>
  <worldbody>
    <!-- part: box -->
    <body name="box" pos="0 0 0" quat="1 0 0 0">
      <geom type="mesh" mesh="mesh_000"/>
      <inertial pos="0.001 0.0005 0.0005" quat="1 0 0 0" mass="2e-6" diaginertia="3.333333333333333e-13 8.333333333333334e-13 8.333333333333334e-13"/>
    </body>
  </worldbody>
</mujoco>
"#
    );
}

#[test]
fn given_the_fixture_box_when_written_then_the_stl_bytes_match_the_layout() {
    let mut bytes = Vec::new();
    write_mesh(&mut bytes, &box_mesh(), 0.001).unwrap();

    assert_eq!(bytes.len(), 84 + 50 * 12);
    assert!(bytes[..80].iter().all(|byte| *byte == 0));
    assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 12);

    let f32_at = |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let scaled = |vertex: f32| (f64::from(vertex) * 0.001) as f32;

    // First triangle (0, 3, 2) on the bottom face: outward normal -Z; the
    // scaled vertices keep the winding.
    let normal = [f32_at(84), f32_at(88), f32_at(92)];
    assert!((normal[0]).abs() < 1e-9 && (normal[1]).abs() < 1e-9);
    assert!((normal[2] - -1.0).abs() < 1e-9, "{normal:?}");
    let v0 = [f32_at(96), f32_at(100), f32_at(104)];
    let v1 = [f32_at(108), f32_at(112), f32_at(116)];
    let v2 = [f32_at(120), f32_at(124), f32_at(128)];
    for (component, expected) in v0.iter().zip([0.0f32, 0.0, 0.0]) {
        assert!((component - expected).abs() < 1e-9);
    }
    for (component, expected) in v1.iter().zip([scaled(0.0), scaled(1.0), scaled(0.0)]) {
        assert!((component - expected).abs() < 1e-9);
    }
    for (component, expected) in v2.iter().zip([scaled(2.0), scaled(1.0), scaled(0.0)]) {
        assert!((component - expected).abs() < 1e-9);
    }
    assert!(bytes[132..134].iter().all(|byte| *byte == 0));
    assert!(bytes[682..684].iter().all(|byte| *byte == 0));
}

#[test]
fn given_a_grouping_root_with_meshed_children_when_assembled_then_names_dedupe_and_frames_map() {
    let forest = grouped_scene();
    let model = assemble(&forest, &[box_properties()], &options(false)).unwrap();

    assert_eq!(model.bodies.len(), 3);
    assert_eq!(model.assets.len(), 1);
    assert_eq!(model.assets[0].name, "mesh_000");
    assert_eq!(model.assets[0].comment, "first used by: part");

    let root = &model.bodies[0];
    assert_eq!(root.name, "asm");
    assert_eq!(root.depth, 0);
    assert!(matches!(root.kind, crate::model::BodyKind::Grouping));

    let first = &model.bodies[1];
    assert_eq!(first.name, "part");
    assert_eq!(first.depth, 1);
    for (axis, expected) in first.pos.iter().zip([0.017, 0.018, -0.019]) {
        assert!(
            (axis - expected).abs() < 1e-15,
            "{:?} vs {expected}",
            first.pos
        );
    }
    assert_quat_close(&first.quat, &[1.0, 0.0, 0.0, 0.0]);
    let crate::model::BodyKind::Meshed { mesh, inertial } = &first.kind else {
        panic!("first child must be meshed");
    };
    assert_eq!(mesh, "mesh_000");
    assert!((inertial.mass - 2.0e-6).abs() < 1e-18, "{}", inertial.mass);
    assert!((inertial.diaginertia[0] - 3.333333333333333e-13).abs() < 1e-25);

    let second = &model.bodies[2];
    assert_eq!(second.name, "part_2");
    let half = 0.5_f64.sqrt();
    assert_quat_close(&second.quat, &[half, 0.0, 0.0, half]);
    assert!(second.pos.iter().all(|value| value.abs() < 1e-15));
}

#[test]
fn given_a_grouped_scene_when_written_then_the_text_reflects_the_tree() {
    let forest = grouped_scene();
    let model = assemble(&forest, &[box_properties()], &options(false)).unwrap();
    let mut text = Vec::new();
    write_model(&mut text, &model).unwrap();
    let text = String::from_utf8(text).unwrap();

    // 18 * 0.001 round-trips as 0.018000000000000002; the f64 is the
    // correctly-scaled value.
    assert!(
        text.contains(r#"<body name="part" pos="0.017 0.018000000000000002 -0.019""#,),
        "{text}"
    );
    assert!(
        text.contains(r#"<body name="part_2" pos="0 0 0" quat=""#,),
        "{text}"
    );
    assert_eq!(text.matches("<!-- part: part -->").count(), 2);
    assert_eq!(text.matches("<mesh name=").count(), 1);
    assert!(!text.contains("<freejoint/>"));
}

#[test]
fn given_free_root_when_written_then_the_root_body_floats() {
    let forest = single_root_scene(identity());
    let model = assemble(&forest, &[box_properties()], &options(true)).unwrap();
    let mut text = Vec::new();
    write_model(&mut text, &model).unwrap();
    let text = String::from_utf8(text).unwrap();

    assert!(text.contains("free-root on"), "{text}");
    let freejoint = text.find("<freejoint/>").unwrap();
    let geom = text.find("<geom type=\"mesh\"").unwrap();
    assert!(freejoint < geom, "{text}");
}

#[test]
fn given_a_zero_scale_when_assembled_then_the_options_are_rejected() {
    let forest = single_root_scene(identity());
    let mut options = options(false);
    options.scale = 0.0;
    let error = assemble(&forest, &[box_properties()], &options).unwrap_err();
    assert!(error.to_string().contains("finite and positive"), "{error}");
}

#[test]
fn given_a_nonpositive_density_when_assembled_then_the_options_are_rejected() {
    let forest = single_root_scene(identity());
    let mut options = options(false);
    options.density_parameter = 0.0;
    let error = assemble(&forest, &[box_properties()], &options).unwrap_err();
    assert!(error.to_string().contains("finite and positive"), "{error}");
}

#[test]
fn given_two_roots_when_assembled_then_the_scene_is_rejected() {
    let forest = two_root_scene();
    let error = assemble(&forest, &[], &options(false)).unwrap_err();
    assert!(
        error.to_string().contains("exactly one scene root"),
        "{error}"
    );
}

#[test]
fn given_a_massless_root_with_free_root_when_assembled_then_the_scene_is_rejected() {
    let forest = grouped_scene();
    let error = assemble(&forest, &[box_properties()], &options(true)).unwrap_err();
    assert!(error.to_string().contains("carry mass"), "{error}");
}

#[test]
fn given_a_scaled_placement_when_assembled_then_the_node_is_rejected() {
    let forest = single_root_scene(scaled_twice());
    let error = assemble(&forest, &[box_properties()], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Transform { .. }), "{error}");
    assert!(error.to_string().contains("rigid"), "{error}");
}

#[test]
fn given_a_mirrored_placement_when_assembled_then_the_node_is_rejected() {
    let forest = single_root_scene(mirrored_x());
    let error = assemble(&forest, &[box_properties()], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Transform { .. }), "{error}");
    assert!(error.to_string().contains("mirrored"), "{error}");
}

#[test]
fn given_a_nan_translation_when_assembled_then_the_node_is_rejected() {
    let forest = single_root_scene(nan_translation());
    let error = assemble(&forest, &[box_properties()], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Transform { .. }), "{error}");
    assert!(error.to_string().contains("non-finite"), "{error}");
}

#[test]
fn given_left_handed_principal_axes_when_assembled_then_one_axis_sign_is_flipped() {
    let mut properties = box_properties();
    properties.principal_axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
    let forest = single_root_scene(identity());
    let model = assemble(&forest, &[properties], &options(false)).unwrap();
    let crate::model::BodyKind::Meshed { inertial, .. } = &model.bodies[0].kind else {
        panic!("root must be meshed");
    };
    assert_quat_close(&inertial.quat, &[1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn given_non_unit_principal_axes_when_assembled_then_the_mesh_is_rejected() {
    let mut properties = box_properties();
    properties.principal_axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0]];
    let forest = single_root_scene(identity());
    let error = assemble(&forest, &[properties], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Properties { .. }), "{error}");
    assert!(error.to_string().contains("unit vectors"), "{error}");
}

#[test]
fn given_non_orthogonal_principal_axes_when_assembled_then_the_mesh_is_rejected() {
    let mut properties = box_properties();
    properties.principal_axes = [[0.6, 0.8, 0.0], [0.6, 0.8, 0.0], [0.0, 0.0, 1.0]];
    let forest = single_root_scene(identity());
    let error = assemble(&forest, &[properties], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Properties { .. }), "{error}");
    assert!(error.to_string().contains("orthogonal"), "{error}");
}

#[test]
fn given_a_zero_volume_when_assembled_then_the_mesh_is_rejected() {
    let mut properties = box_properties();
    properties.volume_mm3 = 0.0;
    let forest = single_root_scene(identity());
    let error = assemble(&forest, &[properties], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Properties { .. }), "{error}");
    assert!(error.to_string().contains("finite and positive"), "{error}");
}

#[test]
fn given_a_negative_moment_when_assembled_then_the_mesh_is_rejected() {
    let mut properties = box_properties();
    properties.principal_moments = [-1.0, 5.0 / 6.0, 5.0 / 6.0];
    let forest = single_root_scene(identity());
    let error = assemble(&forest, &[properties], &options(false)).unwrap_err();
    assert!(matches!(error, Error::Properties { .. }), "{error}");
    assert!(error.to_string().contains("non-negative"), "{error}");
}

#[test]
fn given_mismatched_property_sets_when_assembled_then_the_scene_is_rejected() {
    let forest = single_root_scene(identity());
    let error = assemble(&forest, &[], &options(false)).unwrap_err();
    assert!(error.to_string().contains("do not match"), "{error}");
}

#[test]
fn given_the_file_density_when_assembled_then_it_wins_over_the_parameter() {
    let mut properties = box_properties();
    properties.file_density = 7.85;
    let forest = single_root_scene(identity());
    let model = assemble(&forest, &[properties], &options(false)).unwrap();
    let crate::model::BodyKind::Meshed { inertial, .. } = &model.bodies[0].kind else {
        panic!("root must be meshed");
    };
    let expected = 2.0 * 7.85 * 1e-6;
    assert!(
        (inertial.mass - expected).abs() < 1e-18,
        "{}",
        inertial.mass
    );
}
