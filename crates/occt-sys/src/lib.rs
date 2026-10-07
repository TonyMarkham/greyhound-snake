//! Runtime loading and safe ownership of the OCCT-backed C++ shim.

mod dependencies;
pub mod error;
pub mod grey_box;
mod native_api;
pub mod occt;
pub mod step_doc;
pub mod step_info;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use error::{Error as OcctError, result::Result as OcctResult};
pub use occt::Occt;
