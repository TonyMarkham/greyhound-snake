use crate::archive::extract;

use std::{
    fs,
    io::{Cursor, Write},
};
use tempfile::TempDir;
use zip::{ZipWriter, write::SimpleFileOptions};

pub(super) fn zip_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn extracts_files_and_creates_parent_directories() {
    let temp = TempDir::new().unwrap();
    let dest = temp.path().join("out").join("deep");
    let files = extract(
        &zip_bytes(&[("a.txt", "hello"), ("dir/b.txt", "world")]),
        &dest,
    )
    .unwrap();
    assert_eq!(files, 2);
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "hello");
    assert_eq!(fs::read_to_string(dest.join("dir/b.txt")).unwrap(), "world");
}

#[test]
fn rejects_entries_escaping_the_extraction_root() {
    let temp = TempDir::new().unwrap();
    let error = extract(
        &zip_bytes(&[("../evil.txt", "boom")]),
        &temp.path().join("out"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("escapes"), "{error}");
    assert!(!temp.path().join("evil.txt").exists());
}

#[test]
fn reports_a_non_zip_payload() {
    let temp = TempDir::new().unwrap();
    let error = extract(b"definitely not a zip", &temp.path().join("out")).unwrap_err();
    assert!(error.to_string().contains("not a readable zip"), "{error}");
}
