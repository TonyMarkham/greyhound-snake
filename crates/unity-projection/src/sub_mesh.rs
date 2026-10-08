#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct UnitySubMesh {
    pub index_start: u32,
    pub index_count: u32,
    pub first_vertex: u32,
    pub vertex_count: u32,
}
