use std::fs;
use std::path::PathBuf;

use crate::classes::{self, DEFAULT_CLASS};
use crate::data_yaml;
use crate::image_set::ImageSet;

use gpui::SharedString;

fn names(list: &[&str]) -> Vec<SharedString> {
    list.iter()
        .map(|name| SharedString::from((*name).to_string()))
        .collect()
}

#[test]
fn save_writes_one_name_per_line() {
    let dir = tempfile::tempdir().unwrap();
    classes::save(dir.path(), &names(&["person", "car"])).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("classes.txt")).unwrap(),
        "person\ncar\n"
    );
}

#[test]
fn extend_to_cover_reports_growth_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.png"), b"png").unwrap();
    fs::write(dir.path().join("a.txt"), "3 0.5 0.5 0.2 0.2\n").unwrap();
    let set = ImageSet::scan(dir.path()).unwrap();
    let mut list = names(&[DEFAULT_CLASS]);

    let changed = classes::extend_to_cover(&mut list, &set);

    // The list grew with placeholders; persisting is the caller's call
    // (it knows whether a tree data.yaml forbids writing classes.txt).
    assert!(changed);
    assert_eq!(list.len(), 4);
    assert_eq!(list[3], "class 3");
    assert!(!dir.path().join("classes.txt").exists());
}

#[test]
fn extend_to_cover_keeps_longer_list_and_skips_missing_labels() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.png"), b"png").unwrap();
    fs::write(dir.path().join("a.txt"), "1 0.5 0.5 0.2 0.2\n").unwrap();
    // An image with no label file at all must not break the scan.
    fs::write(dir.path().join("b.jpg"), b"jpg").unwrap();
    let set = ImageSet::scan(dir.path()).unwrap();
    let mut list = names(&["person", "car", "bike"]);

    let changed = classes::extend_to_cover(&mut list, &set);

    assert!(!changed);
    assert_eq!(list.len(), 3);
}

#[test]
fn extend_to_cover_ignores_folders_without_labels() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.png"), b"png").unwrap();
    let set = ImageSet::scan(dir.path()).unwrap();
    let mut list = names(&[DEFAULT_CLASS]);

    let changed = classes::extend_to_cover(&mut list, &set);

    assert!(!changed);
    assert_eq!(list.len(), 1);
}

#[test]
fn extend_to_cover_reads_layout_labels_not_images() {
    // Ultralytics layout: labels live in the sibling labels/ folder. If
    // the scan read the image files themselves (the bug this guards
    // against), the JPEG bytes would fail to decode and no class ids
    // would ever be found.
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("images")).unwrap();
    fs::create_dir(dir.path().join("labels")).unwrap();
    fs::write(dir.path().join("images").join("a.png"), b"png").unwrap();
    fs::write(
        dir.path().join("labels").join("a.txt"),
        "3 0.5 0.5 0.2 0.2\n",
    )
    .unwrap();
    let set = ImageSet::scan(dir.path()).unwrap();
    let mut list = names(&[DEFAULT_CLASS]);

    let changed = classes::extend_to_cover(&mut list, &set);

    assert!(changed);
    assert_eq!(list.len(), 4);
}

#[test]
fn yaml_lookup_is_one_directory_up() {
    let tree = tempfile::tempdir().unwrap();
    let run = tree.path().join("run_1");
    fs::create_dir(&run).unwrap();

    assert!(data_yaml::find_yaml(&run).is_none());

    fs::write(tree.path().join("data.yaml"), "names: [Left]\n").unwrap();
    assert!(data_yaml::find_yaml(&run).is_some());
}
