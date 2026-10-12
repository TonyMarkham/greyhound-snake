/// One 4x4 ArUco marker: its dictionary id and canonical bit pattern.
///
/// `bits` is the 4x4 data field in OpenCV's rotation-0 orientation:
/// bit index `b = row*4 + column` sits at position `15 - b`
/// (row-major, MSB-first per byte — the mapping that reproduces the
/// stored rotation-0 bytes; see `aruco-svg.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marker {
    pub id: u32,
    pub bits: u16,
}

impl Marker {
    /// The marker whose canonical bits are the rotation-0 bytes of an
    /// OpenCV table row: first two bytes, MSB-first row-major.
    pub(crate) fn from_rotation0_bytes(id: u32, bytes: &[[u8; 2]; 4]) -> Self {
        let bits = (u16::from(bytes[0][0]) << 8) | u16::from(bytes[0][1]);
        Self { id, bits }
    }
}
