use std::path::PathBuf;

/// Inputs for one export run; `main` builds it from CLI arguments.
pub struct ExportOptions {
    pub config_path: PathBuf,
    pub step_path: PathBuf,
    pub out_dir: PathBuf,
    /// Fallback density in g/cm³ for parts whose file carries none.
    pub density: f64,
    pub free_root: bool,
    /// Length scale baked into vertices and positions (meters per mm).
    pub scale: f64,
}
