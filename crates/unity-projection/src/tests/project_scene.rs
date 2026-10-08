use crate::{ProjectionSettings, UnityNode, project::project_transform, project_scene};

use mesh::{MeshBuilder, Node, Scene};

fn settings() -> ProjectionSettings {
    ProjectionSettings { scale: 0.001 }
}

fn forest(nodes: Vec<Node>) -> Scene {
    Scene::try_new(nodes, 1, b"rootpart".to_vec()).unwrap()
}

fn root() -> Node {
    Node {
        parent: Node::NO_PARENT,
        mesh: Node::NO_MESH,
        name_offset: 0,
        name_length: 4,
        transform: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ],
    }
}

fn placed(parent: u32, transform: [f32; 12]) -> Node {
    Node {
        parent,
        mesh: 0,
        name_offset: 4,
        name_length: 4,
        transform,
    }
}

fn triangle_mesh() -> mesh::Mesh {
    MeshBuilder::default()
        .with_vertices(vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        .with_normals(vec![[0.0, 0.0, 1.0]; 3])
        .with_triangles(vec![[0, 1, 2]])
        .with_faces(vec![mesh::FaceRange {
            vertex_start: 0,
            vertex_count: 3,
            index_start: 0,
            index_count: 3,
        }])
        .with_face_attribs(vec![mesh::FaceAttrib { solid: 0, color: 0 }])
        .with_colors(vec![[1.0, 0.5, 0.0, 1.0]])
        .build()
        .unwrap()
}

#[test]
fn given_a_translated_placement_when_projected_then_the_translation_is_permuted_and_scaled() {
    // given: OCCT translation (17, 18, -19) on an identity rotation
    let placed = placed(
        0,
        [
            1.0, 0.0, 0.0, 17.0, //
            0.0, 1.0, 0.0, 18.0, //
            0.0, 0.0, 1.0, -19.0,
        ],
    );

    // when
    let [row0, row1, row2] = {
        let projected = project_transform(&placed.transform, 0.001);
        [
            [projected[0], projected[1], projected[2], projected[3]],
            [projected[4], projected[5], projected[6], projected[7]],
            [projected[8], projected[9], projected[10], projected[11]],
        ]
    };

    // then: Unity = (x, z, y), so the translation becomes (17, -19, 18) x 0.001
    assert_eq!(row0, [1.0, 0.0, 0.0, 0.017]);
    assert_eq!(row1, [0.0, 1.0, 0.0, -0.019]);
    assert_eq!(row2, [0.0, 0.0, 1.0, 0.018]);
}

#[test]
fn given_a_rotated_placement_when_projected_then_the_rotation_is_conjugated_by_the_permutation() {
    // given: 90-degree CCW rotation about OCCT +Z (maps x->y, y->-x),
    // translation (0, 0, 5)
    let rotation = [
        0.0, -1.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 5.0,
    ];

    // when
    let projected = project_transform(&rotation, 1.0);

    // then: M R M with M = (x, z, y). Conjugating by a reflection reverses
    // the rotation sense, so the OCCT +Z rotation appears as -90 degrees
    // about Unity's mapped axis (+Y): rows [(0,0,-1),(0,1,0),(1,0,0)].
    let rows: [&[f32; 3]; 3] = [
        &[projected[0], projected[1], projected[2]],
        &[projected[4], projected[5], projected[6]],
        &[projected[8], projected[9], projected[10]],
    ];
    assert_eq!(rows[0], &[0.0, 0.0, -1.0]);
    assert_eq!(rows[1], &[0.0, 1.0, 0.0]);
    assert_eq!(rows[2], &[1.0, 0.0, 0.0]);
    // translation (0, 0, 5) -> (0, 5, 0) at scale 1
    assert_eq!(&projected[3..4], &[0.0]);
    assert_eq!(&projected[7..8], &[5.0]);
    assert_eq!(&projected[11..12], &[0.0]);
}

#[test]
fn given_a_scene_when_projected_then_nodes_keep_hierarchy_and_names() {
    // given
    let forest = forest(vec![
        root(),
        placed(
            0,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0],
        ),
    ]);
    let meshes = vec![triangle_mesh()];

    // when
    let scene = project_scene(&forest, &meshes, settings()).unwrap();

    // then
    assert_eq!(scene.nodes().len(), 2);
    assert_eq!(scene.names(), b"rootpart");
    let part = &scene.nodes()[1];
    assert_eq!(part.parent, 0);
    assert_eq!(part.mesh, 0);
    assert_eq!(
        &scene.names()[part.name_offset as usize..][..part.name_length as usize],
        b"part"
    );
}

#[test]
fn given_a_scene_when_projected_then_each_mesh_is_a_unity_mesh() {
    // given
    let forest = forest(vec![
        root(),
        placed(
            0,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0],
        ),
    ]);
    let meshes = vec![triangle_mesh()];

    // when
    let scene = project_scene(&forest, &meshes, settings()).unwrap();

    // then: the winding flips, positions permute and scale
    let mesh = &scene.meshes()[0];
    assert_eq!(mesh.vertices()[0].position, [0.0, 0.0, 0.0]);
    assert_eq!(mesh.vertices()[1].position, [0.001, 0.0, 0.0]);
    assert_eq!(mesh.vertices()[2].position, [0.0, 0.0, 0.001]);
    assert_eq!(mesh.indices(), &[0, 2, 1]);
    assert_eq!(mesh.submeshes().len(), 1);
    assert_eq!(mesh.submesh_colors(), &[0]);
    assert_eq!(mesh.colors(), &[[1.0, 0.5, 0.0, 1.0]]);
}

#[test]
fn given_mismatched_mesh_count_when_projected_then_the_scene_is_rejected() {
    // given
    let forest = forest(vec![
        root(),
        placed(
            0,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        ),
    ]);

    // when
    let error = project_scene(&forest, &[], settings()).unwrap_err();

    // then
    assert!(error.to_string().contains("do not match"), "{error}");
}

#[test]
fn given_a_nonpositive_scale_when_projected_then_the_scene_is_rejected() {
    // given
    let forest = forest(vec![root()]);
    let meshes = Vec::new();

    // when
    let error = project_scene(&forest, &meshes, ProjectionSettings { scale: 0.0 }).unwrap_err();

    // then
    assert!(error.to_string().contains("finite and positive"), "{error}");
}

#[test]
fn given_a_placed_scene_when_projected_then_node_and_vertex_transforms_compose() {
    // given: node rotates 90 degrees about OCCT +Z and translates (0, 0, 5)
    let rotation = [
        0.0, -1.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 5.0,
    ];
    let forest = forest(vec![root(), placed(0, rotation)]);
    let meshes = vec![triangle_mesh()];
    let scene = project_scene(&forest, &meshes, settings()).unwrap();

    // when: compose the projected transform with the projected vertex
    let projected_rotation = scene.nodes()[1].transform;
    let vertex = scene.meshes()[0].vertices()[1].position; // OCCT (1,0,0) * 0.001
    let composed = [
        projected_rotation[0] * vertex[0]
            + projected_rotation[1] * vertex[1]
            + projected_rotation[2] * vertex[2]
            + projected_rotation[3],
        projected_rotation[4] * vertex[0]
            + projected_rotation[5] * vertex[1]
            + projected_rotation[6] * vertex[2]
            + projected_rotation[7],
        projected_rotation[8] * vertex[0]
            + projected_rotation[9] * vertex[1]
            + projected_rotation[10] * vertex[2]
            + projected_rotation[11],
    ];

    // then: OCCT world point = R (1,0,0) + (0,0,5) = (0,1,5); Unity of that
    // is (0, 5, 1) * 0.001
    let expected = [0.0, 0.005, 0.001];
    for (axis, (actual, expected)) in composed.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < 1e-6,
            "axis {axis}: {composed:?}"
        );
    }
}

#[test]
fn given_a_scene_with_unity_node_consts_when_projected_then_consts_match_the_mesh_model() {
    assert_eq!(UnityNode::NO_PARENT, mesh::Node::NO_PARENT);
    assert_eq!(UnityNode::NO_MESH, mesh::Node::NO_MESH);
}
