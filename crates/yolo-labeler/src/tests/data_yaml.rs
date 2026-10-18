use std::fs;

use crate::data_yaml;

const TREE_YAML: &str = "\
path: .
train:
  - run_A
val:
  - run_A
names:
  0: Left
  1: Right
  2: Pivot
  3: End
channels: 1
";

#[test]
fn find_and_load_one_directory_up() {
    let tree = tempfile::tempdir().unwrap();
    let run = tree.path().join("run_1");
    fs::create_dir(&run).unwrap();
    fs::write(tree.path().join("data.yaml"), TREE_YAML).unwrap();

    let yaml_path = data_yaml::find_yaml(&run).unwrap();
    assert_eq!(yaml_path, tree.path().join("data.yaml"));

    let names = data_yaml::load_names(&yaml_path).unwrap();
    assert_eq!(names.len(), 4);
    assert_eq!(names[0], "Left");
    assert_eq!(names[3], "End");
}

#[test]
fn the_target_folder_holds_no_yaml() {
    // The target folder holds only images/ and labels/: a data.yaml
    // sitting inside it is not the class source — only the one one
    // directory up counts.
    let tree = tempfile::tempdir().unwrap();
    let run = tree.path().join("run_1");
    fs::create_dir(&run).unwrap();
    fs::write(run.join("data.yaml"), "names: [Wrong]\n").unwrap();
    fs::write(tree.path().join("data.yaml"), TREE_YAML).unwrap();

    let yaml_path = data_yaml::find_yaml(&run).unwrap();
    assert_eq!(yaml_path, tree.path().join("data.yaml"));
    assert_eq!(data_yaml::load_names(&yaml_path).unwrap()[0], "Left");
}

#[test]
fn missing_yaml_is_none() {
    let tree = tempfile::tempdir().unwrap();
    let run = tree.path().join("run_1");
    fs::create_dir(&run).unwrap();

    assert!(data_yaml::find_yaml(&run).is_none());
}

#[test]
fn load_accepts_mapping_style_names() {
    let tree = tempfile::tempdir().unwrap();
    let run = tree.path().join("run_1");
    fs::create_dir(&run).unwrap();
    fs::write(
        tree.path().join("data.yaml"),
        "names:\n  0: Left\n  2: Pivot\n",
    )
    .unwrap();

    let names = data_yaml::load_names(&data_yaml::find_yaml(&run).unwrap()).unwrap();

    // Slot 1 is missing from the map: it must stay a placeholder rather
    // than shift the later names down.
    assert_eq!(names.len(), 3);
    assert_eq!(names[0], "Left");
    assert_eq!(names[1], "class 1");
    assert_eq!(names[2], "Pivot");
}

#[test]
fn update_names_preserves_other_keys() {
    let tree = tempfile::tempdir().unwrap();
    let run = tree.path().join("run_1");
    fs::create_dir(&run).unwrap();
    let yaml_path = tree.path().join("data.yaml");
    fs::write(&yaml_path, TREE_YAML).unwrap();

    data_yaml::update_names(&yaml_path, &["Left".into(), "Right".into()]).unwrap();

    let updated = fs::read_to_string(&yaml_path).unwrap();
    assert!(updated.contains("train:"));
    assert!(updated.contains("run_A"));
    assert!(updated.contains("channels: 1"));
    assert!(updated.contains("Left"));
    assert!(!updated.contains("Pivot"));
}
