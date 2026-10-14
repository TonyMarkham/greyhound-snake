use crate::digest::{sha256_hex, sha256_hex_file};

use std::fs;

#[test]
fn empty_input_hash() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn abc_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn file_hash_matches_the_memory_hash() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("archive.zip");
    fs::write(&path, b"abc").unwrap();
    assert_eq!(
        sha256_hex_file(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
