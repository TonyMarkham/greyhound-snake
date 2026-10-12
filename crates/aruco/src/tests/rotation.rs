use crate::rotation::rotations;

#[test]
fn rotation_index_zero_is_the_identity() {
    for bits in [0u16, 0xb532, 0x8000, 0x0001, u16::MAX] {
        assert_eq!(rotations(bits)[0], bits);
    }
}

#[test]
fn one_turn_moves_the_top_left_bit_to_the_bottom_left() {
    // out(row, column) = bits(column, 3 - row): the bit at (0, 0)
    // lands at (3, 0) — the corner cycles anticlockwise
    // (0,0) → (3,0) → (3,3) → (0,3).
    assert_eq!(rotations(0x8000)[1], 0x0008);
    assert_eq!(rotations(0x8000)[2], 0x0001);
    assert_eq!(rotations(0x8000)[3], 0x1000);
    assert_eq!(rotations(0x8000)[0], 0x8000, "four turns restore the field");
}

#[test]
fn all_white_and_all_black_are_rotation_invariant() {
    assert_eq!(rotations(0xffff), [0xffff; 4]);
    assert_eq!(rotations(0x0000), [0x0000; 4]);
}

#[test]
fn rotating_four_times_restores_any_field() {
    // Exhaustive over the 2^16 space: turn four times, land home.
    for bits in 0..=u16::MAX {
        let mut turned = bits;
        for _ in 0..4 {
            turned = rotations(turned)[1];
        }
        assert_eq!(turned, bits);
    }
}
