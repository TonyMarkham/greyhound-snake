use crate::{ExternFetchError, ExternFetchResult};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// What one download produced: either the archive bytes held in memory
/// (the tracked default — `download_location` empty) or the path of the
/// zip staged inside the configured `download_location` directory (the
/// per-machine override; the same key the Unity package's C#
/// provisioner consumes from the staged config, refactor-repo.md D8).
/// The staged file is disposable: the caller deletes it once the hash
/// is verified and the payload extracted — success or failure.
pub(crate) enum StagedArchive {
    Bytes(Vec<u8>),
    File(PathBuf),
}

impl StagedArchive {
    /// The staged zip's name: the release asset's own file name (the
    /// last url segment). A release asset url always carries one; a
    /// url that does not fails loudly instead of colliding on a
    /// made-up name.
    pub(crate) fn zip_name(url: &str) -> ExternFetchResult<String> {
        let name = url.rsplit('/').next().unwrap_or("");
        if name.is_empty() {
            return Err(ExternFetchError::download(
                url,
                "cannot derive a staged zip name from this url",
            ));
        }
        Ok(name.to_owned())
    }
}

/// Blocking download of one release asset into
/// `<dir>/<asset file name>`, streamed straight from the response into
/// the file: the archive never sits whole in memory (the file mode's
/// reason to exist). The caller owns deletion of the staged file.
pub(crate) fn download_to_staged(url: &str, dir: &Path) -> ExternFetchResult<PathBuf> {
    let path = dir.join(StagedArchive::zip_name(url)?);
    let mut response = ureq::get(url)
        .call()
        .map_err(|error| ExternFetchError::download(url, format!("{error}")))?;
    let mut reader = response
        .body_mut()
        .with_config()
        // Same ceiling as the in-memory download: release assets are
        // tens to a few hundred MB.
        .limit(1_073_741_824)
        .reader();
    let mut file = fs::File::create(&path).map_err(|error| {
        ExternFetchError::io(format!("cannot create {}: {error}", path.display()))
    })?;
    io::copy(&mut reader, &mut file).map_err(|error| {
        // A partially written zip is worthless: remove it so the next
        // run starts clean.
        let _ = fs::remove_file(&path);
        ExternFetchError::download(url, format!("body read failed: {error}"))
    })?;
    Ok(path)
}
