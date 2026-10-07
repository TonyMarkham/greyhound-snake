mod builder;
mod error;
mod face_range;
mod mesh;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use builder::Builder as MeshBuilder;
pub use error::{Error as MeshError, result::Result as MeshResult};
pub use face_range::FaceRange;
pub use mesh::Mesh;
