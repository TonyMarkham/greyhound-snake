use crate::{
    ExternFetchError, ExternFetchResult, archive, artifact::Artifact, digest::sha256_hex, download,
    group::Group, provenance::Provenance,
};

use std::path::Path;

/// What one ensure step did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Marker matched the config — nothing downloaded, nothing written.
    Skipped,
    /// Downloaded, hash verified against the pin, extracted, marker written.
    Extracted { files: usize },
    /// Entry unpinned: downloaded, extracted, computed hash recorded in
    /// the marker and reported. Counts as not-ok for the exit code —
    /// trust-on-first-use must be reviewed and pinned, not silently green.
    NeedsPin { sha256: String },
}

/// Brings one artifact's destination to the state the config describes.
/// `fetch` is the network boundary, injected so tests can run the whole
/// flow without HTTP.
pub(crate) fn ensure_with(
    fetch: &dyn Fn(&str) -> ExternFetchResult<Vec<u8>>,
    group_name: &str,
    role: &str,
    group: &Group,
    artifact: &Artifact,
    repo_root: &Path,
) -> ExternFetchResult<Outcome> {
    let dest = artifact.dest_path(repo_root);
    let marker_path = artifact.marker_path(repo_root);
    if marker_path.exists() {
        let marker = Provenance::read(&marker_path)?;
        if marker.matches(
            group_name,
            role,
            group.repo(),
            group.tag(),
            artifact.url(),
            artifact.pinned_sha256(),
        ) {
            if dest.exists() {
                return Ok(Outcome::Skipped);
            }
            return Err(ExternFetchError::provenance(format!(
                "{} certifies {} but the directory is missing; delete the marker and re-run",
                marker_path.display(),
                dest.display()
            )));
        }
        return Err(ExternFetchError::provenance(format!(
            "{} certifies a different artifact than the config (marker sha256 {}, url {}; config sha256 {}); delete {} and {} and re-run",
            marker_path.display(),
            marker.sha256(),
            marker.url(),
            artifact.pinned_sha256().unwrap_or("unpinned"),
            dest.display(),
            marker_path.display()
        )));
    }
    if dest.exists() {
        return Err(ExternFetchError::provenance(format!(
            "{} exists without a provenance marker; delete it and re-run",
            dest.display()
        )));
    }
    let bytes = fetch(artifact.url())?;
    let actual = sha256_hex(&bytes);
    if let Some(pinned) = artifact.pinned_sha256()
        && !actual.eq_ignore_ascii_case(pinned)
    {
        return Err(ExternFetchError::digest(format!(
            "sha256 mismatch for {group_name}.{role}: expected {pinned}, got {actual}; nothing was extracted"
        )));
    }
    let files = archive::extract(&bytes, &dest)?;
    let marker = Provenance::new(
        group_name,
        role,
        group.repo(),
        group.tag(),
        artifact.url(),
        &actual,
    );
    marker.write(&marker_path)?;
    match artifact.pinned_sha256() {
        Some(_) => Ok(Outcome::Extracted { files }),
        // Unpinned: extract and record the computed hash so the pin and
        // the cache agree once the hash is reviewed and written into the
        // config (the next run then only verifies against the marker).
        None => Ok(Outcome::NeedsPin { sha256: actual }),
    }
}

/// The real run: network fetch via `download`.
pub(crate) fn ensure(
    group_name: &str,
    role: &str,
    group: &Group,
    artifact: &Artifact,
    repo_root: &Path,
) -> ExternFetchResult<Outcome> {
    ensure_with(
        &|url| download::download(url),
        group_name,
        role,
        group,
        artifact,
        repo_root,
    )
}
