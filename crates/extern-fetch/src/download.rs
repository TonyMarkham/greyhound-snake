use crate::{ExternFetchError, ExternFetchResult};

/// Blocking download of one release asset into memory. The archives are
/// tens of MB — no streaming temp file is needed, and hashing + extraction
/// both want the whole byte range anyway. GitHub release downloads
/// redirect to a CDN; ureq follows redirects by default.
pub(crate) fn download(url: &str) -> ExternFetchResult<Vec<u8>> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|error| ExternFetchError::download(url, format!("{error}")))?;
    response
        .body_mut()
        .with_config()
        // ureq caps read_to_vec at 10 MiB by default to prevent memory
        // exhaustion; release assets are tens to a few hundred MB.
        .limit(1_073_741_824)
        .read_to_vec()
        .map_err(|error| ExternFetchError::download(url, format!("body read failed: {error}")))
}
