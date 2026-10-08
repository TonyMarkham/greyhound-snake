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
pub struct HostMeshProperties {
    pub volume_mm3: f32,
    pub file_density: f32,
    pub centre_of_gravity: [f32; 3],
    pub gyration_radii: [f32; 3],
    pub principal_axes: [f32; 9],
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
