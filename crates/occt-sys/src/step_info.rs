use crate::grey_box::GreyBbox;

/// What greyhound_step_info reported.
pub struct StepInfo {
    pub solids: i32,
    pub faces: i32,
    pub edges: i32,
    pub bbox: GreyBbox,
}
