use crate::{ExternFetchError, ExternFetchResult};

use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

/// Lowercase hex SHA-256 of `bytes` — the form the release pages and the
/// provenance markers use.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Lowercase hex SHA-256 of the file at `path`, streamed in 1 MiB
/// chunks — a staged archive is tens to hundreds of MB and must not be
/// buffered whole just to hash it.
pub(crate) fn sha256_hex_file(path: &Path) -> ExternFetchResult<String> {
    let mut file = fs::File::open(path).map_err(|error| {
        ExternFetchError::io(format!("cannot open {}: {error}", path.display()))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            ExternFetchError::io(format!("cannot read {}: {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(hasher.finalize().as_slice()))
}

fn hex(digest: &[u8]) -> String {
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
