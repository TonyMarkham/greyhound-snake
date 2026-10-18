use std::fs;

use crate::image_set::ImageSet;

#[test]
fn scan_picks_images_and_ignores_other_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("b.png"), b"png").unwrap();
    fs::write(dir.path().join("a.JPG"), b"jpg").unwrap();
    fs::write(dir.path().join("notes.txt"), b"txt").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let set = ImageSet::scan(dir.path()).unwrap();
    assert_eq!(set.len(), 2);
    assert_eq!(
        set.current().unwrap().file_name().unwrap().to_str(),
        Some("a.JPG")
    );
}

#[test]
fn navigation_wraps_around() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.png"), b"").unwrap();
    fs::write(dir.path().join("b.png"), b"").unwrap();
    let mut set = ImageSet::scan(dir.path()).unwrap();
    set.next();
    assert_eq!(set.position(), 1);
    set.next();
    assert_eq!(set.position(), 0);
    set.prev();
    assert_eq!(set.position(), 1);
}

#[test]
fn empty_folder_has_no_current_image() {
    let dir = tempfile::tempdir().unwrap();
    let set = ImageSet::scan(dir.path()).unwrap();
    assert!(set.is_empty());
    assert!(set.current().is_none());
}

#[test]
fn flat_layout_resolves_labels_beside_images() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.png"), b"").unwrap();
    let set = ImageSet::scan(dir.path()).unwrap();
    assert_eq!(
        set.label_path_for(&dir.path().join("a.png")),
        dir.path().join("a.txt")
    );
}

#[test]
fn ultralytics_layout_scans_images_and_resolves_labels_folder() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("images")).unwrap();
    fs::create_dir(dir.path().join("labels")).unwrap();
    fs::write(dir.path().join("images").join("img_0.jpg"), b"").unwrap();
    fs::write(dir.path().join("images").join("img_1.jpg"), b"").unwrap();
    fs::write(dir.path().join("labels").join("img_0.txt"), b"").unwrap();
    // A stray file directly in the run folder is not a frame.
    fs::write(dir.path().join("notes.json"), b"{}").unwrap();

    let set = ImageSet::scan(dir.path()).unwrap();
    assert_eq!(set.len(), 2);
    assert_eq!(
        set.label_path_for(&dir.path().join("images").join("img_0.jpg")),
        dir.path().join("labels").join("img_0.txt")
    );
}
