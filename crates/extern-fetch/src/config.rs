use crate::{ExternFetchError, ExternFetchResult, group::Group};

use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// The whole fetch configuration: dependency groups keyed by name
/// (`occt`, `opencv`, ...), flattened from the top-level tables.
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    #[serde(flatten)]
    groups: BTreeMap<String, Group>,
}

impl Config {
    /// Reads, parses, and validates. Parse and validate are separate
    /// steps so tests can feed in-memory TOML.
    pub(crate) fn load(path: &Path) -> ExternFetchResult<Config> {
        let text = fs::read_to_string(path).map_err(|error| {
            ExternFetchError::io(format!(
                "cannot read config {}: {error} (the default path assumes the tool runs from the repository root; otherwise pass --config)",
                path.display()
            ))
        })?;
        let config = Self::parse(&text)?;
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn parse(text: &str) -> ExternFetchResult<Config> {
        toml::from_str(text).map_err(|error| {
            ExternFetchError::config(format!("config is not valid TOML for the schema: {error}"))
        })
    }

    pub(crate) fn validate(&self) -> ExternFetchResult<()> {
        for (name, group) in &self.groups {
            group.validate(name)?;
        }
        Ok(())
    }

    pub(crate) fn group(&self, name: &str) -> Option<&Group> {
        self.groups.get(name)
    }

    pub(crate) fn group_names(&self) -> Vec<&str> {
        self.groups.keys().map(String::as_str).collect()
    }

    /// Every group, or only those named in `filter` when non-empty.
    pub(crate) fn selected_groups<'a>(
        &'a self,
        filter: &'a [String],
    ) -> impl Iterator<Item = (&'a String, &'a Group)> {
        self.groups
            .iter()
            .filter(move |(name, _)| filter.is_empty() || filter.contains(name))
    }

    /// Destination paths resolve against the repository root, taken as
    /// two levels above the config file's directory (the crate dir for
    /// the in-repo config `crates/extern-fetch/config.toml`) — the same
    /// rule `crates/occt-sys/config.toml` states for its own paths. The
    /// config path is made absolute against the current directory first
    /// (the default path is repo-root-relative), so the rule holds
    /// regardless of how the tool was invoked.
    pub(crate) fn repo_root(config_path: &Path) -> ExternFetchResult<PathBuf> {
        let absolute = if config_path.is_absolute() {
            config_path.to_path_buf()
        } else {
            let cwd = std::env::current_dir().map_err(|error| {
                ExternFetchError::io(format!("cannot determine the current directory: {error}"))
            })?;
            cwd.join(config_path)
        };
        let dir = absolute.parent().ok_or_else(|| {
            ExternFetchError::config(format!(
                "config path {} has no directory component",
                absolute.display()
            ))
        })?;
        let root = dir
            .parent()
            .and_then(|parent| parent.parent())
            .ok_or_else(|| {
                ExternFetchError::config(format!(
                    "config {} must live two directory levels below the repository root (e.g. crates/extern-fetch/config.toml)",
                    absolute.display()
                ))
            })?;
        Ok(root.to_path_buf())
    }
}
