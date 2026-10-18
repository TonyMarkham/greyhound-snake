use std::path::Path;

use crate::yolo::{self, YoloBox};

#[test]
fn parse_and_format_round_trip() {
    let text = "0 0.500000 0.500000 0.200000 0.400000\n1 0.100000 0.900000 0.300000 0.200000\n";
    let parsed = yolo::parse_labels(text);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].class, 0);
    assert_eq!(parsed[1].cx, 0.1);
    assert_eq!(yolo::format_labels(&parsed), text);
}

#[test]
fn parse_skips_blank_comment_and_malformed_lines() {
    let text = "\n# annotation v2\n1 0.25 0.25 0.5 0.5\nnot a box\n2 0.5 0.5\n9 x y w h\n";
    let parsed = yolo::parse_labels(text);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].class, 1);
}

#[test]
fn parse_clamps_out_of_range_values() {
    let parsed = yolo::parse_labels("0 1.5 -0.5 2.0 0.5\n");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].cx, 1.0);
    assert_eq!(parsed[0].cy, 0.0);
    assert_eq!(parsed[0].w, 1.0);
    assert_eq!(parsed[0].h, 0.5);
}

#[test]
fn parse_rejects_extra_fields() {
    let parsed = yolo::parse_labels("0 0.5 0.5 0.2 0.4 0.9\n");
    assert!(parsed.is_empty());
}

#[test]
fn from_corners_orders_swapped_corners() {
    let label = YoloBox::from_corners(2, 0.8, 0.9, 0.2, 0.1);
    assert!((label.cx - 0.5).abs() < 1e-6);
    assert!((label.cy - 0.5).abs() < 1e-6);
    assert!((label.w - 0.6).abs() < 1e-6);
    assert!((label.h - 0.8).abs() < 1e-6);
}

#[test]
fn contains_matches_center_box() {
    let label = YoloBox::from_corners(0, 0.4, 0.4, 0.6, 0.6);
    assert!(label.contains(0.5, 0.5));
    assert!(label.contains(0.4, 0.4));
    assert!(!label.contains(0.15, 0.5));
}

#[test]
fn never_wipe_refuses_emptying_a_populated_file() {
    assert!(yolo::wipes_labels("0 0.5 0.5 0.2 0.4\n", ""));
    // A file that was already empty (a background frame) may stay empty.
    assert!(!yolo::wipes_labels("", ""));
    assert!(!yolo::wipes_labels("\n", " \n"));
    // Writing lines is never a wipe.
    assert!(!yolo::wipes_labels(
        "0 0.5 0.5 0.2 0.4\n",
        "1 0.1 0.1 0.1 0.1\n"
    ));
}

#[test]
fn label_path_replaces_extension() {
    assert_eq!(
        yolo::label_path_for(Path::new("images/frame_01.PNG")),
        Path::new("images/frame_01.txt")
    );
}
