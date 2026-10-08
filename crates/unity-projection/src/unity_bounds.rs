#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct UnityBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}
