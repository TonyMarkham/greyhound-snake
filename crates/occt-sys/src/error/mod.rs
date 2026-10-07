pub(crate) mod result;
pub use result::Result;

// ---------------------------------------------------------------------------------------------- //

use error_location::ErrorLocation;
use std::{
    panic::Location,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message} {location}")]
    Step {
        message: String,
        location: ErrorLocation,
    },
    #[error("OCCT load failed: library_dir={library_dir:?}; shim={shim:?}; {message} {location}")]
    Load {
        library_dir: PathBuf,
        shim: PathBuf,
        message: String,
        location: ErrorLocation,
    },
    #[error("OCCT symbol lookup failed: shim={shim:?}; symbol={symbol}; {message} {location}")]
    Symbol {
        shim: PathBuf,
        symbol: String,
        message: String,
        location: ErrorLocation,
    },
    #[error("OCCT ABI mismatch: shim={shim:?}; expected={expected}; actual={actual} {location}")]
    Abi {
        shim: PathBuf,
        expected: u32,
        actual: u32,
        location: ErrorLocation,
    },
}

impl Error {
    #[track_caller]
    pub fn step(message: impl Into<String>) -> Self {
        Self::Step {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn load(directory: &Path, shim: &Path, message: impl Into<String>) -> Self {
        Self::Load {
            library_dir: directory.to_owned(),
            shim: shim.to_owned(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn symbol(shim: &Path, name: &[u8], message: impl Into<String>) -> Self {
        Self::Symbol {
            shim: shim.to_owned(),
            symbol: String::from_utf8_lossy(name.strip_suffix(&[0]).unwrap_or(name)).into_owned(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn abi(shim: &Path, expected: u32, actual: u32) -> Self {
        Self::Abi {
            shim: shim.to_owned(),
            expected,
            actual,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
