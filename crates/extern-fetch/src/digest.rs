use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes` — the form the release pages and the
/// provenance markers use.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}
