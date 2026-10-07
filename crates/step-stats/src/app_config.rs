use crate::{config_error::ConfigError, occt_config::OcctConfig};

use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub occt: OcctConfig,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let config_path = path.canonicalize().map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?;
        let text = fs::read_to_string(&config_path).map_err(|source| ConfigError::Read {
            path: config_path.clone(),
            source,
        })?;
        let mut config: Self = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: config_path.clone(),
            source,
        })?;
        if config.occt.library_dir.as_os_str().is_empty() {
            return Err(ConfigError::Invalid {
                path: config_path,
                message: "occt.library_dir must not be empty".to_owned(),
            });
        }
        let parent = config_path.parent().ok_or_else(|| ConfigError::Invalid {
            path: config_path.clone(),
            message: "config has no parent directory".to_owned(),
        })?;
        let requested = if config.occt.library_dir.is_absolute() {
            config.occt.library_dir.clone()
        } else {
            parent.join(&config.occt.library_dir)
        };
        let resolved = requested
            .canonicalize()
            .map_err(|error| ConfigError::Invalid {
                path: config_path.clone(),
                message: format!("occt.library_dir {}: {error}", requested.display()),
            })?;
        if !resolved.is_dir() {
            return Err(ConfigError::Invalid {
                path: config_path,
                message: format!("occt.library_dir {} is not a directory", resolved.display()),
            });
        }
        config.occt.library_dir = resolved;

        if config.occt.shim_path.as_os_str().is_empty() {
            return Err(ConfigError::Invalid {
                path: config_path,
                message: "occt.shim_path must not be empty".to_owned(),
            });
        }

        let requested_shim = if config.occt.shim_path.is_absolute() {
            config.occt.shim_path.clone()
        } else {
            parent.join(&config.occt.shim_path)
        };

        let resolved_shim =
            requested_shim
                .canonicalize()
                .map_err(|error| ConfigError::Invalid {
                    path: config_path.clone(),
                    message: format!("occt.shim_path {}: {error}", requested_shim.display(),),
                })?;

        if !resolved_shim.is_file() {
            return Err(ConfigError::Invalid {
                path: config_path,
                message: format!("occt.shim_path {} is not a file", resolved_shim.display(),),
            });
        }

        config.occt.shim_path = resolved_shim;

        Ok(config)
    }
}
