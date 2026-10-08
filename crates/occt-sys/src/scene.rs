use mesh::Mesh;

/// The assembly forest of a STEP document plus its unique tessellated
/// meshes. Nodes address meshes by index (deduplicated across instances);
/// every mesh carries the document's global color palette, so submesh
/// color attribution is consistent across all of them.
pub struct Scene {
    forest: mesh::Scene,
    meshes: Vec<Mesh>,
}

impl Scene {
    pub(crate) fn new(forest: mesh::Scene, meshes: Vec<Mesh>) -> Self {
        Self { forest, meshes }
    }

    pub fn forest(&self) -> &mesh::Scene {
        &self.forest
    }

    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }
}
