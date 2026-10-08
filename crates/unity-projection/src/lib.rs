mod bounds;
mod error;
mod project;
mod settings;
mod sub_mesh;
mod unity_bounds;
mod unity_mesh;
mod unity_node;
mod unity_scene;
mod vertex;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use bounds::OcctBounds;
pub use error::{Error as ProjectionError, result::Result as ProjectionResult};
pub use project::{project, project_scene};
pub use settings::ProjectionSettings;
pub use sub_mesh::UnitySubMesh;
pub use unity_bounds::UnityBounds;
pub use unity_mesh::UnityMesh;
pub use unity_node::UnityNode;
pub use unity_scene::UnityScene;
pub use vertex::UnityVertex;
