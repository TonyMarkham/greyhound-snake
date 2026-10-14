use crate::{
    ExternFetchError, ExternFetchResult, archive,
    artifact::Artifact,
    digest::{sha256_hex, sha256_hex_file},
    download,
    group::Group,
    provenance::Provenance,
    staged_archive::{StagedArchive, download_to_staged},
};

use std::{
    fs,
    path::{Path, PathBuf},
};

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
/// flow without HTTP: it hands back either the archive bytes (memory
/// mode) or the path of a zip staged inside the configured
/// `download_location` (file mode).
pub(crate) fn ensure_with(
    fetch: &dyn Fn(&str) -> ExternFetchResult<StagedArchive>,
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
    let staged = fetch(artifact.url())?;
    let outcome = install(
        &staged,
        group_name,
        role,
        group,
        artifact,
        &dest,
        &marker_path,
    );
    // The staged file is disposable: delete it once the payload is in
    // (or the attempt failed) — success or failure, never keep it.
    // Memory mode holds nothing to delete.
    if let StagedArchive::File(path) = &staged {
        let _ = fs::remove_file(path);
    }
    outcome
}

/// Hashes, verifies, extracts, and certifies — the tail shared by both
/// staging modes. Hash verification happens before any extraction, so
/// a mismatch leaves nothing behind ("nothing was extracted").
fn install(
    staged: &StagedArchive,
    group_name: &str,
    role: &str,
    group: &Group,
    artifact: &Artifact,
    dest: &Path,
    marker_path: &Path,
) -> ExternFetchResult<Outcome> {
    let actual = match staged {
        StagedArchive::Bytes(bytes) => sha256_hex(bytes),
        StagedArchive::File(path) => sha256_hex_file(path)?,
    };
    if let Some(pinned) = artifact.pinned_sha256()
        && !actual.eq_ignore_ascii_case(pinned)
    {
        return Err(ExternFetchError::digest(format!(
            "sha256 mismatch for {group_name}.{role}: expected {pinned}, got {actual}; nothing was extracted"
        )));
    }
    let files = match staged {
        StagedArchive::Bytes(bytes) => archive::extract(bytes, dest)?,
        StagedArchive::File(path) => archive::extract_file(path, dest)?,
    };
    let marker = Provenance::new(
        group_name,
        role,
        group.repo(),
        group.tag(),
        artifact.url(),
        &actual,
    );
    marker.write(marker_path)?;
    match artifact.pinned_sha256() {
        Some(_) => Ok(Outcome::Extracted { files }),
        // Unpinned: extract and record the computed hash so the pin and
        // the cache agree once the hash is reviewed and written into the
        // config (the next run then only verifies against the marker).
        None => Ok(Outcome::NeedsPin { sha256: actual }),
    }
}

/// The network boundary `ensure_with` receives: one url in, the staged
/// archive out (bytes in memory, or a zip path on disk). A type alias
/// keeps the Box construction below readable; the explicit HRTB is
/// what the inline `dyn Fn(&str)` elided.
type Fetch = dyn for<'url> Fn(&'url str) -> ExternFetchResult<StagedArchive>;

/// The real run: network fetch via `download`. `download_dir` selects
/// the staging mode — None (the tracked default) keeps the archive in
/// memory; Some(dir) streams it into `dir` under the asset's own file
/// name and hands the staged path back (the dir is validated up front
/// by Config::download_dir). Owned because the boxed 'static closure
/// captures it.
pub(crate) fn ensure(
    group_name: &str,
    role: &str,
    group: &Group,
    artifact: &Artifact,
    repo_root: &Path,
    download_dir: Option<PathBuf>,
) -> ExternFetchResult<Outcome> {
    let fetch: Box<Fetch> = match download_dir {
        None => Box::new(|url| download::download(url).map(StagedArchive::Bytes)),
        Some(dir) => Box::new(move |url| match download_to_staged(url, &dir) {
            Ok(path) => Ok(StagedArchive::File(path)),
            Err(error) => {
                // A partial staging file is worthless: remove it so
                // the next run starts clean. Best effort — the
                // download error itself is what matters.
                if let Ok(name) = StagedArchive::zip_name(url) {
                    let _ = fs::remove_file(dir.join(name));
                }
                Err(error)
            }
        }),
    };
    ensure_with(&*fetch, group_name, role, group, artifact, repo_root)
}
