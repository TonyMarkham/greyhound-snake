use crate::data;
use crate::dictionary::ArucoDictionary;
use crate::error::Result;
use crate::marker::Marker;
use crate::rotation::rotations;

/// Markers of the dictionary, collected (ids are dense, so all
/// `Result`s are `Ok`).
fn collected(dict: ArucoDictionary) -> Vec<Marker> {
    dict.markers()
        .collect::<Result<Vec<_>>>()
        .expect("dense ids cannot fail")
}

#[test]
fn marker_counts_match_the_predefined_sizes() {
    assert_eq!(ArucoDictionary::Dict4x4_50.marker_count(), 50);
    assert_eq!(ArucoDictionary::Dict4x4_100.marker_count(), 100);
    assert_eq!(ArucoDictionary::Dict4x4_250.marker_count(), 250);
    assert_eq!(ArucoDictionary::Dict4x4_1000.marker_count(), 1000);
    assert_eq!(ArucoDictionary::ALL.len(), 4);
}

#[test]
fn marker_zero_is_the_stored_rotation0_bytes() {
    // hpp:4121: marker 0 rotation 0 = {181, 50} = rows
    // 1011 / 0101 / 0011 / 0010 = 0xb532.
    let marker = ArucoDictionary::Dict4x4_50
        .marker(0)
        .expect("id 0 is valid");
    assert_eq!(marker.id, 0);
    assert_eq!(marker.bits, 0xb532);
    assert_eq!(marker.bits & 0x8000, 0x8000, "row 0 column 0 is black");
    assert_eq!(marker.bits & 0x0001, 0, "row 3 column 3 is white");
}

#[test]
fn marker_lookup_is_dense_and_bounds_checked() {
    let dict = ArucoDictionary::Dict4x4_50;
    for id in 0..50u32 {
        assert_eq!(dict.marker(id).expect("dense id").id, id);
    }
    let error = dict.marker(50).expect_err("50 is past the end");
    assert!(error.to_string().contains("out of range"));
}

#[test]
fn dictionaries_are_nested_prefixes_of_the_shared_table() {
    let all = collected(ArucoDictionary::Dict4x4_1000);
    for (dict, size) in [
        (ArucoDictionary::Dict4x4_50, 50usize),
        (ArucoDictionary::Dict4x4_100, 100),
        (ArucoDictionary::Dict4x4_250, 250),
    ] {
        let prefix = collected(dict);
        assert_eq!(prefix.len(), size);
        for (index, marker) in prefix.iter().enumerate() {
            assert_eq!(
                marker.bits, all[index].bits,
                "dict {dict:?} marker {index} must equal the 1000-dict marker"
            );
        }
    }
}

#[test]
fn no_dictionary_marker_is_self_rotation_symmetric() {
    // Computed over the transcribed table: 0 in every predefined 4x4
    // dictionary (the 2^16 space holds up to 256 patterns fixed by
    // 180 degree rotation; OpenCV's dicts contain none — recorded as
    // this test constant).
    for dict in ArucoDictionary::ALL {
        let count = collected(dict)
            .into_iter()
            .filter(|marker| crate::hamming::self_distance(marker.bits) == 0)
            .count();
        assert_eq!(count, 0, "dict {dict:?} holds self-symmetric markers");
    }
}

#[test]
fn stored_rotations_are_distinct_within_every_marker() {
    // Distinctness of the four stored rotations per marker, straight
    // from the table (independent of our rotation code).
    for (index, bytes) in data::DICT_4X4_1000_BYTES.iter().enumerate() {
        assert!(
            bytes[0] != bytes[1] && bytes[1] != bytes[2] && bytes[2] != bytes[3],
            "marker {index} stores duplicate rotations"
        );
    }
}

#[test]
fn our_rotation_code_reproduces_the_stored_rotations() {
    // The ground-truth check: OpenCV precomputed and stored all four
    // rotations; our 90-degree-anticlockwise permutation must
    // reproduce rotations 1-3 from rotation 0 for all 1000 markers.
    for (index, bytes) in data::DICT_4X4_1000_BYTES.iter().enumerate() {
        let canonical = (u16::from(bytes[0][0]) << 8) | u16::from(bytes[0][1]);
        let ours = rotations(canonical);
        for rotation in 1..4usize {
            let stored = (u16::from(bytes[rotation][0]) << 8) | u16::from(bytes[rotation][1]);
            assert_eq!(
                ours[rotation], stored,
                "marker {index}: our rotation {rotation} must equal the stored bytes"
            );
        }
    }
}
