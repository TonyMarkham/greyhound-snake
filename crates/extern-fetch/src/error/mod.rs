pub(crate) mod result;
pub(crate) use result::Result;

// ---------------------------------------------------------------------------------------------- //

use error_location::ErrorLocation;
use std::panic::Location;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("{message} {location}")]
    Config {
        message: String,
        location: ErrorLocation,
    },
    #[error("download failed: {url}; {message} {location}")]
    Download {
        url: String,
        message: String,
        location: ErrorLocation,
    },
    #[error("{message} {location}")]
    Digest {
        message: String,
        location: ErrorLocation,
    },
    #[error("{message} {location}")]
    Archive {
        message: String,
        location: ErrorLocation,
    },
    #[error("{message} {location}")]
    Provenance {
        message: String,
        location: ErrorLocation,
    },
    #[error("{message} {location}")]
    Io {
        message: String,
        location: ErrorLocation,
    },
}

impl Error {
    #[track_caller]
    pub(crate) fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn download(url: &str, message: impl Into<String>) -> Self {
        Self::Download {
            url: url.to_string(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn digest(message: impl Into<String>) -> Self {
        Self::Digest {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn archive(message: impl Into<String>) -> Self {
        Self::Archive {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn provenance(message: impl Into<String>) -> Self {
        Self::Provenance {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
