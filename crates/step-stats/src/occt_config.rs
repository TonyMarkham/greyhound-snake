use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcctConfig {
    pub library_dir: PathBuf,
    pub shim_path: PathBuf,
}
