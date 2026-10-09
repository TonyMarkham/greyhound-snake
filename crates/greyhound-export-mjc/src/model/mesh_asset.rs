/// A `<mesh>` asset: one per unique mesh, named `mesh_NNN`, with the STL
/// file living under the model's `meshes/` directory. The comment records
/// the raw part name of the first node using the asset.
#[derive(Clone, Debug)]
pub struct MeshAsset {
    pub name: String,
    pub file: String,
    pub comment: String,
}
