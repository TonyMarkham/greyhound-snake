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
    /// Where downloads may stage archive bytes on disk before
    /// extraction. Tracked default: empty = in-memory. Both consumers
    /// honor it: this tool (`ensure`) and the Unity package's C#
    /// provisioner via the staged copy of this config (see the key's
    /// comment in config.toml, refactor-repo.md D8).
    #[serde(default)]
    pub(crate) download_location: String,
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

    /// The validated `download_location`: None = in-memory (the tracked
    /// default); Some = an absolute path to an EXISTING directory the
    /// archive may be staged in. Relative paths are rejected (no honest
    /// root to resolve against; the key's meaning is machine-local) and
    /// a missing directory fails rather than being created — neither
    /// consumer creates directories on a machine whose policies govern
    /// exactly which paths are approved. This tool consumes it only for
    /// its own download staging; the C# provisioner applies the same
    /// rule to the staged copy of this config.
    pub(crate) fn download_dir(&self) -> ExternFetchResult<Option<PathBuf>> {
        if self.download_location.is_empty() {
            return Ok(None);
        }
        let path = PathBuf::from(&self.download_location);
        if !path.is_absolute() {
            return Err(ExternFetchError::config(format!(
                "download_location must be an absolute path with forward \
                 slashes, got {:?}",
                self.download_location
            )));
        }
        if !path.is_dir() {
            return Err(ExternFetchError::config(format!(
                "download_location {} does not exist; the tool never \
                 creates directories",
                path.display()
            )));
        }
        Ok(Some(path))
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
