use crate::{
    archive::{extract, extract_file},
    staged_archive::StagedArchive,
};

use std::fs;
use tempfile::TempDir;

#[test]
fn zip_name_is_the_urls_last_segment() {
    assert_eq!(
        StagedArchive::zip_name(
            "https://github.com/example/occt-dev/releases/download/v9/occt-dev.zip"
        )
        .unwrap(),
        "occt-dev.zip"
    );
    let error =
        StagedArchive::zip_name("https://github.com/example/occt-dev/releases/download/v9/")
            .unwrap_err();
    assert!(error.to_string().contains("staged zip name"), "{error}");
}

#[test]
fn file_extract_matches_the_memory_extract() {
    let temp = TempDir::new().unwrap();
    let zip_path = temp.path().join("archive.zip");
    fs::write(
        &zip_path,
        crate::tests::archive::zip_bytes(&[("a.txt", "hello"), ("dir/b.txt", "world")]),
    )
    .unwrap();

    let dest = temp.path().join("out");
    let files = extract_file(&zip_path, &dest).unwrap();
    assert_eq!(files, 2);
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "hello");
    assert_eq!(fs::read_to_string(dest.join("dir/b.txt")).unwrap(), "world");

    // The zip-slip guard applies on the file path too.
    fs::write(
        &zip_path,
        crate::tests::archive::zip_bytes(&[("../evil.txt", "boom")]),
    )
    .unwrap();
    let error = extract_file(&zip_path, &temp.path().join("out2")).unwrap_err();
    assert!(error.to_string().contains("escapes"), "{error}");
    assert!(!temp.path().join("evil.txt").exists());

    // And the in-memory extract still behaves (regression guard for
    // the shared-loop refactor).
    let dest3 = temp.path().join("out3");
    assert_eq!(
        extract(
            &crate::tests::archive::zip_bytes(&[("a.txt", "hello")]),
            &dest3
        )
        .unwrap(),
        1
    );
}
