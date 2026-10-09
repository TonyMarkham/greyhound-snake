pub(crate) mod result;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use result::Result;

use error_location::ErrorLocation;
use std::{io, panic::Location, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot write {path:?}: {source} {location}")]
    Io {
        path: PathBuf,
        source: io::Error,
        location: ErrorLocation,
    },
    #[error("{message} {location}")]
    Invalid {
        message: String,
        location: ErrorLocation,
    },
    #[error("node {node}: {message} {location}")]
    Transform {
        node: u32,
        message: String,
        location: ErrorLocation,
    },
    #[error("mesh {mesh}: {message} {location}")]
    Properties {
        mesh: u32,
        message: String,
        location: ErrorLocation,
    },
    #[error("config: {source} {location}")]
    Config {
        // Boxed: ConfigError carries path buffers and parser payloads that
        // would push this enum past clippy's result_large_err threshold.
        source: Box<crate::config_error::ConfigError>,
        location: ErrorLocation,
    },
    #[error("step: {source} {location}")]
    Step {
        source: Box<occt_sys::OcctError>,
        location: ErrorLocation,
    },
}

impl Error {
    #[track_caller]
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn transform(node: u32, message: impl Into<String>) -> Self {
        Self::Transform {
            node,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn properties(mesh: u32, message: impl Into<String>) -> Self {
        Self::Properties {
            mesh,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn config(source: crate::config_error::ConfigError) -> Self {
        Self::Config {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn step(source: occt_sys::OcctError) -> Self {
        Self::Step {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
