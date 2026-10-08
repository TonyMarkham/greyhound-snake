#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct UnityVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}
