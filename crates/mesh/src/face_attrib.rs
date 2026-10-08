#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceAttrib {
    pub solid: u32,
    pub color: u32,
}

impl FaceAttrib {
    /// Sentinel solid index emitted for faces outside any solid; all such
    /// faces - open shells, sheet bodies, loose faces - merge into one
    /// group carrying this index.
    pub const NO_SOLID: u32 = u32::MAX;
}
