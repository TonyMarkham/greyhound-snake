use unity_projection::UnityBounds;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct HostSceneCounts {
    pub node_count: u32,
    pub mesh_count: u32,
    pub color_count: u32,
    pub name_bytes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct HostMeshCounts {
    pub vertex_count: u32,
    pub index_count: u32,
    pub submesh_count: u32,
    pub color_count: u32,
    pub bounds: UnityBounds,
}
