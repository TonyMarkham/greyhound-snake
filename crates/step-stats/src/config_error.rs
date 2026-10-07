use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config {path:?}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("invalid TOML in config {path:?}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("invalid config {path:?}: {message}")]
    Invalid { path: PathBuf, message: String },
}
