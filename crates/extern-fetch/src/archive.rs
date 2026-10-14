use crate::{ExternFetchError, ExternFetchResult};

use std::{
    fs,
    io::{self, Cursor, Write},
    path::Path,
};
use zip::ZipArchive;

/// Extracts a zip held in memory into `dest` (created if missing),
/// refusing entries that escape the extraction root (zip-slip). Returns
/// the number of files written.
pub(crate) fn extract(zip_bytes: &[u8], dest: &Path) -> ExternFetchResult<usize> {
    let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|error| ExternFetchError::archive(format!("not a readable zip: {error}")))?;
    fs::create_dir_all(dest).map_err(|error| {
        ExternFetchError::io(format!("cannot create {}: {error}", dest.display()))
    })?;
    let mut files = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            ExternFetchError::archive(format!("cannot read zip entry {index}: {error}"))
        })?;
        // enclosed_name() is the zip-slip guard: None = the entry's path
        // escapes the extraction root.
        let Some(relative) = entry.enclosed_name() else {
            return Err(ExternFetchError::archive(format!(
                "zip entry {} escapes the extraction root",
                entry.name().unwrap_or_default()
            )));
        };
        let target = dest.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|error| {
                ExternFetchError::io(format!("cannot create {}: {error}", target.display()))
            })?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    ExternFetchError::io(format!("cannot create {}: {error}", parent.display()))
                })?;
            }
            let mut out = fs::File::create(&target).map_err(|error| {
                ExternFetchError::io(format!("cannot create {}: {error}", target.display()))
            })?;
            io::copy(&mut entry, &mut out).map_err(|error| {
                ExternFetchError::io(format!("cannot extract {}: {error}", target.display()))
            })?;
            out.flush().map_err(|error| {
                ExternFetchError::io(format!("cannot flush {}: {error}", target.display()))
            })?;
            files += 1;
        }
    }
    Ok(files)
}
