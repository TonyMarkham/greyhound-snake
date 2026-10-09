use std::path::PathBuf;

/// What one export produced; `main` prints it and the tests assert it.
pub struct ExportSummary {
    pub model_path: PathBuf,
    pub body_count: usize,
    pub mesh_count: usize,
    /// Meshed occurrences — one `<geom>` per meshed body.
    pub geom_count: usize,
    /// Sum over meshed occurrences (not unique meshes) in kilograms.
    pub total_mass_kg: f64,
}
