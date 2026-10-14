use crate::provenance::Provenance;

use std::fs;
use tempfile::TempDir;

const SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const URL: &str = "https://github.com/example/occt-dev/releases/download/v9/occt-dev.zip";

#[test]
fn write_read_roundtrip() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("dev.provenance.toml");
    let marker = Provenance::new("occt", "dev", "example/occt-dev", "v9", URL, SHA);
    marker.write(&path).unwrap();
    assert_eq!(Provenance::read(&path).unwrap(), marker);
}

#[test]
fn matches_requires_equal_coordinates() {
    let marker = Provenance::new("occt", "dev", "example/occt-dev", "v9", URL, SHA);
    assert!(marker.matches("occt", "dev", "example/occt-dev", "v9", URL, Some(SHA)));
    assert!(marker.matches("occt", "dev", "example/occt-dev", "v9", URL, None));
    // Hash comparison is case-insensitive: release pages print lowercase,
    // but a hand-pinned uppercase hash must still match.
    assert!(marker.matches(
        "occt",
        "dev",
        "example/occt-dev",
        "v9",
        URL,
        Some(&SHA.to_uppercase())
    ));
    assert!(!marker.matches("opencv", "dev", "example/occt-dev", "v9", URL, None));
    assert!(!marker.matches("occt", "libs", "example/occt-dev", "v9", URL, None));
    assert!(!marker.matches(
        "occt",
        "dev",
        "example/occt-dev",
        "v9",
        "https://github.com/example/occt-dev/releases/download/v9/other.zip",
        None
    ));
    assert!(!marker.matches(
        "occt",
        "dev",
        "example/occt-dev",
        "v9",
        URL,
        Some("1111111111111111111111111111111111111111111111111111111111111111")
    ));
}

#[test]
fn read_reports_an_invalid_marker() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("dev.provenance.toml");
    fs::write(&path, "garbage").unwrap();
    let error = Provenance::read(&path).unwrap_err();
    assert!(
        error.to_string().contains("not a valid provenance marker"),
        "{error}"
    );
}
