use crate::{ExternFetchError, ExternFetchResult};

use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

/// One downloadable archive: where to get it, its pinned SHA-256, and
/// where it extracts. Role names are free-form config keys (`dev`,
/// `libs`, ...).
#[derive(Debug, Deserialize)]
pub(crate) struct Artifact {
    url: String,
    /// Empty (or omitted) = unpinned: the tool extracts anyway, records
    /// the computed hash in the provenance marker, reports it, and counts
    /// the run as failed until the hash is pinned here.
    #[serde(default)]
    sha256: String,
    /// Destination directory: repo-root-relative, forward slashes, no
    /// `..` — enforced by `validate`.
    dest: String,
}

impl Artifact {
    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn dest(&self) -> &str {
        &self.dest
    }

    /// The pin, when the entry carries one.
    pub(crate) fn pinned_sha256(&self) -> Option<&str> {
        if self.sha256.is_empty() {
            None
        } else {
            Some(&self.sha256)
        }
    }

    /// Destination directory resolved against the repository root.
    pub(crate) fn dest_path(&self, repo_root: &Path) -> PathBuf {
        repo_root.join(&self.dest)
    }

    /// The provenance marker is a SIBLING of the destination directory
    /// (`<last-component>.provenance.toml`), not a file inside it: the
    /// extracted tree stays exactly the archive's contents, which matters
    /// for destinations that end up inside the Unity package payload.
    pub(crate) fn marker_path(&self, repo_root: &Path) -> PathBuf {
        let dest = self.dest_path(repo_root);
        let name = dest
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dest".to_string());
        match dest.parent() {
            Some(parent) => parent.join(format!("{name}.provenance.toml")),
            None => PathBuf::from(format!("{name}.provenance.toml")),
        }
    }

    /// The config invariants this entry must satisfy before anything is
    /// downloaded. `ident` is the `group.os.role` dotted path used in
    /// error messages.
    pub(crate) fn validate(&self, ident: &str, repo: &str, tag: &str) -> ExternFetchResult<()> {
        // Same-tag invariant, enforced mechanically:
        // every URL of a group must be a release asset of that group's
        // own repo and tag, so the dev zip and the libs zip can never
        // drift onto different releases.
        let prefix = format!("https://github.com/{repo}/releases/download/{tag}/");
        if !self.url.starts_with(&prefix) {
            return Err(ExternFetchError::config(format!(
                "{ident}: url must start with {prefix} (same-tag invariant)"
            )));
        }
        if let Some(pinned) = self.pinned_sha256()
            && !is_sha256_hex(pinned)
        {
            return Err(ExternFetchError::config(format!(
                "{ident}: sha256 must be empty (unpinned) or 64 hex digits, got {:?}",
                self.sha256
            )));
        }
        self.validate_dest(ident)
    }

    fn validate_dest(&self, ident: &str) -> ExternFetchResult<()> {
        if self.dest.is_empty() {
            return Err(ExternFetchError::config(format!(
                "{ident}: dest must not be empty"
            )));
        }
        if self.dest.contains('\\') {
            return Err(ExternFetchError::config(format!(
                "{ident}: dest must use forward slashes, got {:?}",
                self.dest
            )));
        }
        if Path::new(&self.dest).is_absolute() {
            return Err(ExternFetchError::config(format!(
                "{ident}: dest must be relative to the repository root, got {:?}",
                self.dest
            )));
        }
        for component in Path::new(&self.dest).components() {
            let problem = match component {
                Component::Normal(name) => {
                    let name = name.to_string_lossy();
                    // A colon would be a drive/ADS marker on Windows and
                    // is rejected on every platform so the committed
                    // config means the same thing everywhere.
                    if name.contains(':') {
                        Some(format!("'{name}' contains ':'"))
                    } else {
                        None
                    }
                }
                Component::ParentDir => Some("'..'".to_string()),
                Component::CurDir => Some("'.'".to_string()),
                Component::RootDir | Component::Prefix(_) => {
                    Some("an absolute-path component".to_string())
                }
            };
            if let Some(problem) = problem {
                return Err(ExternFetchError::config(format!(
                    "{ident}: dest component {problem} is not allowed (got {:?})",
                    self.dest
                )));
            }
        }
        Ok(())
    }
}

fn is_sha256_hex(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}
