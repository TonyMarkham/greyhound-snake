pub(crate) mod result;

// ---------------------------------------------------------------------------------------------- //

use error_location::ErrorLocation;
use std::panic::Location;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message} {location}")]
    Mesh {
        message: String,
        location: ErrorLocation,
    },
}

impl Error {
    #[track_caller]
    pub fn mesh(message: impl Into<String>) -> Self {
        Self::Mesh {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
