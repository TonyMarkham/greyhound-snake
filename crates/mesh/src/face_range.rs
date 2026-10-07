#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceRange {
    pub vertex_start: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub index_count: u32,
}
