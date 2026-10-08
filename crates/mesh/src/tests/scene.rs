use crate::{Node, Scene};

fn identity() -> [f32; 12] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0,
    ]
}

fn named(offset: u32, length: u32) -> Node {
    Node {
        parent: Node::NO_PARENT,
        mesh: Node::NO_MESH,
        name_offset: offset,
        name_length: length,
        transform: identity(),
    }
}

#[test]
fn given_a_valid_forest_when_built_then_accessors_return_the_inputs() {
    // given
    let names = b"rootpart".to_vec();
    let nodes = vec![
        named(0, 4),
        Node {
            parent: 0,
            mesh: 0,
            name_offset: 4,
            name_length: 4,
            transform: identity(),
        },
    ];

    // when
    let scene = Scene::try_new(nodes, 1, names).unwrap();

    // then
    assert_eq!(scene.nodes().len(), 2);
    assert_eq!(scene.mesh_count(), 1);
    assert_eq!(scene.names(), b"rootpart");
    assert_eq!(scene.node_name(&scene.nodes()[1]), b"part");
}

#[test]
fn given_a_parent_that_does_not_precede_its_child_when_built_then_the_scene_is_rejected() {
    // given: the only node claims a later node as its parent
    let node = Node {
        parent: 1,
        ..named(0, 0)
    };

    // when
    let error = Scene::try_new(vec![node], 0, Vec::new()).unwrap_err();

    // then
    assert!(error.to_string().contains("does not precede it"), "{error}");
}

#[test]
fn given_a_mesh_reference_beyond_mesh_count_when_built_then_the_scene_is_rejected() {
    // given
    let node = Node {
        mesh: 3,
        ..named(0, 0)
    };

    // when
    let error = Scene::try_new(vec![node], 2, Vec::new()).unwrap_err();

    // then
    assert!(error.to_string().contains("beyond mesh_count"), "{error}");
}

#[test]
fn given_a_name_range_beyond_the_blob_when_built_then_the_scene_is_rejected() {
    // given
    let node = named(2, 5);

    // when
    let error = Scene::try_new(vec![node], 0, b"ab".to_vec()).unwrap_err();

    // then
    assert!(
        error.to_string().contains("exceeds the name blob"),
        "{error}"
    );
}

#[test]
fn given_no_nodes_when_built_then_the_scene_is_rejected() {
    // when
    let error = Scene::try_new(Vec::new(), 0, Vec::new()).unwrap_err();

    // then
    assert!(error.to_string().contains("no nodes"), "{error}");
}
