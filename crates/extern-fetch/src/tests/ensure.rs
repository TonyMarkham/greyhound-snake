use crate::{
    ExternFetchError, ExternFetchResult,
    artifact::Artifact,
    config::Config,
    digest::sha256_hex,
    ensure::{Outcome, ensure_with},
    group::Group,
    host_os::HostOs,
    provenance::Provenance,
    staged_archive::StagedArchive,
    tests::archive::zip_bytes,
};

use std::{cell::Cell, fs};
use tempfile::TempDir;

fn config_text(sha256: &str) -> String {
    format!(
        "[occt]\nrepo = \"example/occt-dev\"\ntag = \"v9\"\n\n[occt.windows.dev]\nurl = \"https://github.com/example/occt-dev/releases/download/v9/occt-dev.zip\"\nsha256 = \"{sha256}\"\ndest = \"extern/x/dev\"\n"
    )
}

fn parts(config: &Config) -> (&Group, &Artifact) {
    let group = config.group("occt").unwrap();
    let (_, artifact) = group
        .section(HostOs::Windows)
        .unwrap()
        .artifacts()
        .next()
        .unwrap();
    (group, artifact)
}

fn counting_fetch<'a>(
    bytes: &'a [u8],
    calls: &'a Cell<u32>,
) -> impl Fn(&str) -> ExternFetchResult<StagedArchive> + 'a {
    move |_url| {
        calls.set(calls.get() + 1);
        Ok(StagedArchive::Bytes(bytes.to_vec()))
    }
}

#[test]
fn fresh_download_verifies_extracts_then_skips() {
    let bytes = zip_bytes(&[("a.txt", "hello"), ("dir/b.txt", "world")]);
    let sha = sha256_hex(&bytes);
    let config = Config::parse(&config_text(&sha)).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let calls = Cell::new(0u32);
    let fetch = counting_fetch(&bytes, &calls);

    let outcome = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    assert_eq!(outcome, Outcome::Extracted { files: 2 });
    assert_eq!(
        fs::read_to_string(temp.path().join("extern/x/dev/dir/b.txt")).unwrap(),
        "world"
    );
    let marker = Provenance::read(&temp.path().join("extern/x/dev.provenance.toml")).unwrap();
    assert_eq!(marker.sha256(), sha);

    // Second run: the marker matches the config, so nothing downloads.
    let outcome = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    assert_eq!(outcome, Outcome::Skipped);
    assert_eq!(calls.get(), 1, "the second run must not re-download");
}

#[test]
fn unpinned_entry_extracts_records_and_reports_the_hash() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let sha = sha256_hex(&bytes);
    let config = Config::parse(&config_text("")).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let calls = Cell::new(0u32);
    let fetch = counting_fetch(&bytes, &calls);

    let outcome = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    assert_eq!(
        outcome,
        Outcome::NeedsPin {
            sha256: sha.clone()
        }
    );

    // Pin the reported hash: the marker already recorded exactly that
    // hash, so the next run skips without another download.
    let config = Config::parse(&config_text(&sha)).unwrap();
    let (group, artifact) = parts(&config);
    let outcome = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    assert_eq!(outcome, Outcome::Skipped);
    assert_eq!(
        calls.get(),
        1,
        "pinning an already-cached tree must not re-download"
    );
}

#[test]
fn pin_change_fails_loudly_naming_both_hashes() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let sha = sha256_hex(&bytes);
    let config = Config::parse(&config_text(&sha)).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let calls = Cell::new(0);
    let fetch = counting_fetch(&bytes, &calls);
    ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();

    let wrong = "f".repeat(64);
    let config = Config::parse(&config_text(&wrong)).unwrap();
    let (group, artifact) = parts(&config);
    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("certifies a different artifact"),
        "{message}"
    );
    assert!(message.contains(&sha), "{message}");
    assert!(message.contains(&wrong), "{message}");
}

#[test]
fn sha_mismatch_downloads_but_extracts_nothing() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let config = Config::parse(&config_text(&"f".repeat(64))).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let calls = Cell::new(0);
    let fetch = counting_fetch(&bytes, &calls);
    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("sha256 mismatch"), "{message}");
    assert!(message.contains("nothing was extracted"), "{message}");
    assert!(!temp.path().join("extern/x/dev").exists());
}

#[test]
fn existing_destination_without_marker_fails_loudly() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let config = Config::parse(&config_text(&sha256_hex(&bytes))).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("extern/x/dev")).unwrap();
    let calls = Cell::new(0);
    let fetch = counting_fetch(&bytes, &calls);
    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    assert!(
        error.to_string().contains("without a provenance marker"),
        "{error}"
    );
}

#[test]
fn marker_without_destination_fails_loudly() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let config = Config::parse(&config_text(&sha256_hex(&bytes))).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let calls = Cell::new(0);
    let fetch = counting_fetch(&bytes, &calls);
    ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    fs::remove_dir_all(temp.path().join("extern/x/dev")).unwrap();
    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    assert!(
        error.to_string().contains("the directory is missing"),
        "{error}"
    );
}

#[test]
fn corrupt_marker_fails_loudly() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let config = Config::parse(&config_text(&sha256_hex(&bytes))).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let calls = Cell::new(0);
    let fetch = counting_fetch(&bytes, &calls);
    ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    fs::write(temp.path().join("extern/x/dev.provenance.toml"), "garbage").unwrap();
    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    assert!(
        error.to_string().contains("not a valid provenance marker"),
        "{error}"
    );
}

#[test]
fn download_failure_passes_through() {
    let config = Config::parse(&config_text(&"f".repeat(64))).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let fetch = |_url: &str| -> ExternFetchResult<StagedArchive> {
        Err(ExternFetchError::download(_url, "offline"))
    };
    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    assert!(error.to_string().contains("download failed"), "{error}");
}

#[test]
fn staged_file_mode_extracts_then_deletes_the_zip() {
    let bytes = zip_bytes(&[("a.txt", "hello"), ("dir/b.txt", "world")]);
    let sha = sha256_hex(&bytes);
    let config = Config::parse(&config_text(&sha)).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let staged_dir = temp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let fetch = |_url: &str| -> ExternFetchResult<StagedArchive> {
        let path = staged_dir.join("occt-dev.zip");
        fs::write(&path, &bytes).unwrap();
        Ok(StagedArchive::File(path))
    };

    let outcome = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap();
    assert_eq!(outcome, Outcome::Extracted { files: 2 });
    assert_eq!(
        fs::read_to_string(temp.path().join("extern/x/dev/dir/b.txt")).unwrap(),
        "world"
    );
    assert!(
        !staged_dir.join("occt-dev.zip").exists(),
        "the staged zip is deleted after install"
    );
}

#[test]
fn staged_file_hash_mismatch_extracts_nothing_and_deletes_the_zip() {
    let bytes = zip_bytes(&[("a.txt", "hello")]);
    let config = Config::parse(&config_text(&"f".repeat(64))).unwrap();
    let (group, artifact) = parts(&config);
    let temp = TempDir::new().unwrap();
    let staged_dir = temp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let fetch = |_url: &str| -> ExternFetchResult<StagedArchive> {
        let path = staged_dir.join("occt-dev.zip");
        fs::write(&path, &bytes).unwrap();
        Ok(StagedArchive::File(path))
    };

    let error = ensure_with(&fetch, "occt", "dev", group, artifact, temp.path()).unwrap_err();
    assert!(error.to_string().contains("sha256 mismatch"), "{error}");
    assert!(
        !temp.path().join("extern/x/dev").exists(),
        "nothing was extracted"
    );
    assert!(
        !staged_dir.join("occt-dev.zip").exists(),
        "the staged zip is deleted even on failure"
    );
}
