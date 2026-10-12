use crate::rotation::rotations;

/// The Garrido-Jurado et al. (2014) target inter-marker distance that
/// OpenCV's generator aims for: `C = floor(markerSize²/4)`,
/// `tau = 2·floor(4C/3)` — 10 for 4x4 markers (OpenCV
/// `extendDictionary`). The predefined dictionaries fall short of it
/// (documented minimums 4/3/3/2); selection can use it as the ideal
/// target.
pub const TAU_TARGET: u32 = 10;

/// Hamming distance between two 4x4 bit fields (16 bits, popcount of
/// the XOR).
pub fn hamming(a: u16, b: u16) -> u32 {
    (a ^ b).count_ones()
}

/// Minimum Hamming distance between `a` and any of the four rotations
/// of `b` — OpenCV's inter-marker criterion, computed over all
/// rotations of both markers (`getDistanceToId` with `allRotations`;
/// the relative-rotation minimum over one side covers both).
pub fn min_rotation_hamming(a: u16, b: u16) -> u32 {
    let distances = rotations(b);
    hamming(a, distances[0])
        .min(hamming(a, distances[1]))
        .min(hamming(a, distances[2]))
        .min(hamming(a, distances[3]))
}

/// Distance from a marker to its own rotations — 0 means the marker
/// is self-rotation-symmetric and its orientation cannot be resolved
/// from an image (OpenCV `_getSelfDistance`).
pub fn self_distance(bits: u16) -> u32 {
    let distances = rotations(bits);
    hamming(bits, distances[1])
        .min(hamming(bits, distances[2]))
        .min(hamming(bits, distances[3]))
}
