use crate::{ExternFetchError, ExternFetchResult};

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// The sibling `<dest>.provenance.toml` file: the provisioner's ownership
/// claim on a destination directory. A directory is trusted (and left
/// untouched) only when this marker exists and matches the config entry;
/// anything else fails loudly and must be deleted by hand — the tool
/// never deletes content it cannot prove it created.
///
/// Ownership is the coordinates plus hash, not the writing tool: the
/// `tool` field is informational. Two provisioners share the package
/// payload destination — this tool via `just
/// fetch-occt`, and the Unity package's C# provisioner (tool
/// `com.greyhound.step`) via the staged fetch config — and because both
/// verify the same pinned sha256, a marker either wrote certifies the
/// same bytes.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Provenance {
    tool: String,
    group: String,
    role: String,
    repo: String,
    tag: String,
    url: String,
    sha256: String,
}

impl Provenance {
    pub(crate) fn new(
        group: &str,
        role: &str,
        repo: &str,
        tag: &str,
        url: &str,
        sha256: &str,
    ) -> Self {
        Self {
            tool: "extern-fetch".to_string(),
            group: group.to_string(),
            role: role.to_string(),
            repo: repo.to_string(),
            tag: tag.to_string(),
            url: url.to_string(),
            sha256: sha256.to_string(),
        }
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    /// True when this marker certifies exactly the config entry: same
    /// coordinates and — once the entry pins a hash — the same hash
    /// (case-insensitive; release pages print lowercase). The `tool`
    /// field is deliberately not compared: the package payload
    /// destination is shared with the Unity C# provisioner, which records
    /// its own tool name (see the type's doc comment).
    pub(crate) fn matches(
        &self,
        group: &str,
        role: &str,
        repo: &str,
        tag: &str,
        url: &str,
        pinned_sha256: Option<&str>,
    ) -> bool {
        if self.group != group
            || self.role != role
            || self.repo != repo
            || self.tag != tag
            || self.url != url
        {
            return false;
        }
        match pinned_sha256 {
            Some(pinned) => self.sha256.eq_ignore_ascii_case(pinned),
            None => true,
        }
    }

    pub(crate) fn read(path: &Path) -> ExternFetchResult<Self> {
        let text = fs::read_to_string(path).map_err(|error| {
            ExternFetchError::provenance(format!("cannot read {}: {error}", path.display()))
        })?;
        toml::from_str(&text).map_err(|error| {
            ExternFetchError::provenance(format!(
                "{} is not a valid provenance marker: {error}",
                path.display()
            ))
        })
    }

    pub(crate) fn write(&self, path: &Path) -> ExternFetchResult<()> {
        let text = toml::to_string(self).map_err(|error| {
            ExternFetchError::provenance(format!("cannot serialize provenance: {error}"))
        })?;
        fs::write(path, text).map_err(|error| {
            ExternFetchError::provenance(format!("cannot write {}: {error}", path.display()))
        })
    }
}
