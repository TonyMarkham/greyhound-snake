use crate::model::{Body, MeshAsset};

/// The complete MJCF document, ready for serialization: one `<body>` per
/// scene node (depth-first pre-order), one `<mesh>` asset per unique mesh.
#[derive(Clone, Debug)]
pub struct MjcfModel {
    /// The `<mujoco model>` value — the STEP file stem.
    pub model_name: String,
    /// The leading settings comment; `--` already stripped.
    pub header: String,
    pub assets: Vec<MeshAsset>,
    pub bodies: Vec<Body>,
    pub free_root: bool,
}
