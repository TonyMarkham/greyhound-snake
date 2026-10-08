use crate::{MeshError, MeshResult, node::Node};

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    nodes: Vec<Node>,
    mesh_count: u32,
    names: Vec<u8>,
}

impl Scene {
    /// Builds a scene from node data and validates its invariants: parents
    /// precede their children (depth-first pre-order), mesh references stay
    /// inside `mesh_count`, and name ranges address the UTF-8 name blob.
    pub fn try_new(nodes: Vec<Node>, mesh_count: u32, names: Vec<u8>) -> MeshResult<Self> {
        if nodes.is_empty() {
            return Err(MeshError::mesh("scene has no nodes"));
        }
        for (index, node) in nodes.iter().enumerate() {
            let index = u32::try_from(index)
                .map_err(|_| MeshError::mesh("node count exceeds the u32 ABI range"))?;
            if node.parent != Node::NO_PARENT && node.parent >= index {
                return Err(MeshError::mesh(format!(
                    "node {index} references parent {} that does not precede it",
                    node.parent
                )));
            }
            if node.mesh != Node::NO_MESH && node.mesh >= mesh_count {
                return Err(MeshError::mesh(format!(
                    "node {index} references mesh {} beyond mesh_count {mesh_count}",
                    node.mesh
                )));
            }
            let offset = u64::from(node.name_offset);
            let end = offset + u64::from(node.name_length);
            let blob_length = u64::try_from(names.len())
                .map_err(|_| MeshError::mesh("name blob exceeds the u64 range"))?;
            if end > blob_length {
                return Err(MeshError::mesh(format!(
                    "node {index} name range {offset}..{end} exceeds the name blob"
                )));
            }
        }
        Ok(Self {
            nodes,
            mesh_count,
            names,
        })
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn mesh_count(&self) -> u32 {
        self.mesh_count
    }

    pub fn names(&self) -> &[u8] {
        &self.names
    }

    /// The UTF-8 name addressed by a node's name range. Validated scenes
    /// always address the blob; malformed ranges yield an empty slice.
    pub fn node_name(&self, node: &Node) -> &[u8] {
        let offset = usize::try_from(node.name_offset).unwrap_or(self.names.len());
        if offset >= self.names.len() {
            return &[];
        }
        let end = usize::try_from(node.name_length)
            .unwrap_or(0)
            .saturating_add(offset)
            .min(self.names.len());
        &self.names[offset..end]
    }
}
