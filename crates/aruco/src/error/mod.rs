pub(crate) mod result;

pub use result::Result;

// ---------------------------------------------------------------------------------------------- //

use error_location::ErrorLocation;
use std::panic::Location;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message} {location}")]
    Aruco {
        message: String,
        location: ErrorLocation,
    },
}

impl Error {
    #[track_caller]
    pub fn aruco(message: impl Into<String>) -> Self {
        Self::Aruco {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
