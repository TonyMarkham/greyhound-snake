use crate::dictionary::ArucoDictionary;
use crate::error::Result;
use crate::hamming::{TAU_TARGET, hamming, min_rotation_hamming, self_distance};
use crate::marker::Marker;

fn collected(dict: ArucoDictionary) -> Vec<Marker> {
    dict.markers()
        .collect::<Result<Vec<_>>>()
        .expect("dense ids cannot fail")
}

#[test]
fn hamming_counts_differing_bits() {
    assert_eq!(hamming(0x0000, 0xffff), 16);
    assert_eq!(hamming(0xb532, 0xb532), 0);
    assert_eq!(hamming(0x0000, 0x0001), 1);
}

#[test]
fn self_distance_is_zero_only_for_symmetric_fields() {
    assert_eq!(self_distance(0xffff), 0);
    assert_eq!(self_distance(0x0000), 0);
    // 0x8000 (one corner bit) versus each turned copy: the bit leaves
    // one cell and enters another, so every rotation differs by
    // exactly 2 bits.
    assert_eq!(self_distance(0x8000), 2);
}

#[test]
fn min_rotation_hamming_never_exceeds_the_plain_distance() {
    // The identity rotation is part of the minimum, so the
    // rotation-aware distance can only shrink.
    for pair in [(0xb532, 0xeb48), (0x0000, 0xffff), (0x1234, 0x4321)] {
        assert!(min_rotation_hamming(pair.0, pair.1) <= hamming(pair.0, pair.1));
    }
    // A marker against its own rotations is distance 0.
    assert_eq!(min_rotation_hamming(0x8000, 0x0008), 0);
    assert_eq!(min_rotation_hamming(0xb532, 0xb532), 0);
}

#[test]
fn tau_target_matches_the_paper_formula() {
    // C = floor(markerSize²/4) = 4, tau = 2·floor(4C/3) = 10 (OpenCV
    // extendDictionary, Garrido-Jurado et al. 2014).
    assert_eq!(TAU_TARGET, 10);
}

#[test]
fn dictionary_minimum_distances_match_the_documented_values() {
    // Documented per dictionary in the OpenCV enum comments; the
    // measured minimum over all pairs, all rotations, must equal it.
    // This also proves no two markers share any rotation.
    let expected: [(ArucoDictionary, u32); 4] = [
        (ArucoDictionary::Dict4x4_50, 4),
        (ArucoDictionary::Dict4x4_100, 3),
        (ArucoDictionary::Dict4x4_250, 3),
        (ArucoDictionary::Dict4x4_1000, 2),
    ];
    for (dict, expected_minimum) in expected {
        let markers = collected(dict);
        let mut minimum = u32::MAX;
        for (index, marker) in markers.iter().enumerate() {
            for other in markers.iter().skip(index + 1) {
                minimum = minimum.min(min_rotation_hamming(marker.bits, other.bits));
            }
        }
        assert_eq!(
            minimum, expected_minimum,
            "dict {dict:?} measured minimum distance"
        );
    }
}

#[test]
fn min_rotation_hamming_is_symmetric_over_relative_rotations() {
    // The distance over all rotations of both markers must be
    // symmetric: min over rotations of b against a equals min over
    // rotations of a against b (the relative rotations cover the same
    // four offsets either way).
    for pair in [(0xb532, 0xeb48), (0x1234, 0x4321), (0x8000, 0x0800)] {
        let forward = min_rotation_hamming(pair.0, pair.1);
        let backward = min_rotation_hamming(pair.1, pair.0);
        assert_eq!(forward, backward);
    }
}
