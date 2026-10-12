/// The four rotations of a 4x4 bit field.
///
/// Index 0 is the identity; index k is the field turned 90 degrees
/// anticlockwise k times — the same rotation order OpenCV precomputes
/// and stores per marker (`predefined_dictionaries.hpp`, hpp:18).
pub fn rotations(bits: u16) -> [u16; 4] {
    let one = rotate90_anticlockwise(bits);
    let two = rotate90_anticlockwise(one);
    let three = rotate90_anticlockwise(two);
    [bits, one, two, three]
}

/// One 90 degree anticlockwise turn.
///
/// Forward rule for tracing a cell: the cell at `(row, column)` moves
/// to `(3 - column, row)` — the right column swings into the top row.
/// The implementation states the same turn in fill direction: the
/// value at `out(row, column)` comes from `bits(column, 3 - row)`.
fn rotate90_anticlockwise(bits: u16) -> u16 {
    let mut out = 0u16;
    for row in 0..4u32 {
        for column in 0..4u32 {
            if bit_at(bits, column, 3 - row) {
                out |= 1u16 << (15 - row * 4 - column);
            }
        }
    }
    out
}

/// The bit at `(row, column)` of the 4x4 field.
fn bit_at(bits: u16, row: u32, column: u32) -> bool {
    bits & (1u16 << (15 - row * 4 - column)) != 0
}
