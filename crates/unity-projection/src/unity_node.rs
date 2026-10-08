/// One node of the projected assembly forest. Names are shared through
/// `UnityScene::names()`; transforms are Unity-space row-major 3x4 matrices
/// (rotation conjugated by the axis permutation, translation permuted and
/// scaled).
#[derive(Debug, Clone, PartialEq)]
pub struct UnityNode {
    pub parent: u32,
    pub mesh: u32,
    pub name_offset: u32,
    pub name_length: u32,
    pub transform: [f32; 12],
}

impl UnityNode {
    /// Sentinel parent index for root nodes of the scene forest.
    pub const NO_PARENT: u32 = u32::MAX;
    /// Sentinel mesh index for geometry-free assembly grouping nodes.
    pub const NO_MESH: u32 = u32::MAX;
}
