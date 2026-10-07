/// Six doubles, contiguous — must match the C++ `double* bbox` layout
/// exactly (min xyz, then max xyz).
#[repr(C)]
pub struct GreyBbox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
