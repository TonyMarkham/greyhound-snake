use crate::{UnityMesh, UnityNode};

/// The projected assembly forest: nodes with Unity-space transforms, the
/// unique projected meshes they reference, and the shared UTF-8 name blob.
#[derive(Debug, Clone)]
pub struct UnityScene {
    nodes: Vec<UnityNode>,
    meshes: Vec<UnityMesh>,
    names: Vec<u8>,
}

impl UnityScene {
    pub(crate) fn new(nodes: Vec<UnityNode>, meshes: Vec<UnityMesh>, names: Vec<u8>) -> Self {
        Self {
            nodes,
            meshes,
            names,
        }
    }

    pub fn nodes(&self) -> &[UnityNode] {
        &self.nodes
    }

    pub fn meshes(&self) -> &[UnityMesh] {
        &self.meshes
    }

    /// Shared UTF-8 name bytes addressed by the nodes' name ranges.
    pub fn names(&self) -> &[u8] {
        &self.names
    }
}
