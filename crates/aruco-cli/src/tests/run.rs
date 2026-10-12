use crate::cli::Cli;

use clap::Parser;
use std::path::Path;

fn cli_with(target: &str, output: &Path, extra: &[&str]) -> Cli {
    let out = output.to_string_lossy().into_owned();
    let mut args = vec!["aruco", target, "-o", out.as_str()];
    args.extend_from_slice(extra);
    Cli::parse_from(args)
}

fn cli(target: &str, output: &Path) -> Cli {
    cli_with(target, output, &[])
}

#[test]
fn single_id_writes_the_named_file() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("marker_0.svg");
    crate::run(cli("0", &output)).expect("single-id run succeeds");
    let contents = std::fs::read_to_string(&output).expect("file written");
    assert!(contents.starts_with("<?xml"));
    assert!(contents.contains("viewBox=\"0 0 6 6\""));
}

#[test]
fn all_writes_one_file_per_marker() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("dict");
    crate::run(cli("all", &output)).expect("batch run succeeds");
    for id in [0, 1, 25, 49] {
        let path = output.join(format!("marker_{id:04}.svg"));
        assert!(path.exists(), "{} missing", path.display());
    }
    assert_eq!(std::fs::read_dir(&output).unwrap().count(), 50);
}

#[test]
fn all_writes_the_whole_thousand_dictionary() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("dict");
    crate::run(cli_with("all", &output, &["--dict", "4x4_1000"])).expect("batch run succeeds");
    assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1000);
    assert!(output.join("marker_0999.svg").exists());
}

#[test]
fn sheet_writes_one_document() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("sheet.svg");
    crate::run(cli_with("all", &output, &["--sheet"])).expect("sheet run succeeds");
    let contents = std::fs::read_to_string(&output).expect("file written");
    assert!(contents.contains("viewBox=\"0 0 71 36\""), "{contents}");
}

#[test]
fn units_and_margin_reach_the_render() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("marker.svg");
    crate::run(cli_with(
        "0",
        &output,
        &["--size", "50", "--units", "mm", "--margin", "2", "--label"],
    ))
    .expect("run succeeds");
    let contents = std::fs::read_to_string(&output).expect("file written");
    assert!(contents.contains("viewBox=\"0 0 10 11\""), "{contents}");
    assert!(
        contents.contains("width=\"50mm\" height=\"50mm\""),
        "{contents}"
    );
    assert!(contents.contains(">0</text>"), "{contents}");
}

#[test]
fn dict_choice_reaches_the_files() {
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("dict");
    crate::run(cli_with("all", &output, &["--dict", "4x4_250"])).expect("batch run succeeds");
    assert_eq!(std::fs::read_dir(&output).unwrap().count(), 250);
}

#[test]
fn count_with_floor_writes_individual_files() {
    // The primary use case: 4 rotation-unique markers at a 6-bit
    // floor, one SVG each (measured: seed 6 reaches the floor).
    let directory = tempfile::tempdir().expect("temp dir");
    let output = directory.path().join("level6");
    let out = output.to_string_lossy().into_owned();
    let args = [
        "aruco",
        "--count",
        "4",
        "--dict",
        "4x4_1000",
        "--seed",
        "6",
        "--min-distance",
        "6",
        "-o",
        out.as_str(),
    ];
    crate::run(Cli::parse_from(args)).expect("selection run succeeds");
    assert_eq!(std::fs::read_dir(&output).unwrap().count(), 4);
    assert!(output.join("marker_0376.svg").exists());
    assert!(output.join("marker_0596.svg").exists());
    assert!(output.join("marker_0920.svg").exists());
    assert!(output.join("marker_0937.svg").exists());
}
