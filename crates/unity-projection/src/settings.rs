#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectionSettings {
    pub scale: f64,
}

impl Default for ProjectionSettings {
    fn default() -> Self {
        Self { scale: 0.001 }
    }
}
