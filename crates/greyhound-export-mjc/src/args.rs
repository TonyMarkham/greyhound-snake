use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for the MJCF exporter CLI.
#[derive(Parser)]
#[command(about = "Export a STEP file as a MuJoCo MJCF model with binary STL meshes")]
pub struct Args {
    /// Runtime configuration file
    #[arg(long, default_value = "config.toml")]
    pub config: PathBuf,
    /// Output directory receiving the model XML and the meshes/ directory
    #[arg(long)]
    pub out: PathBuf,
    /// Fallback density in g/cm3 for parts whose STEP file carries none
    #[arg(long, default_value_t = 1.0)]
    pub density: f64,
    /// Give the root body a freejoint (floating) instead of leaving it static
    #[arg(long)]
    pub free_root: bool,
    /// Length scale baked into vertices and positions (meters per millimeter)
    #[arg(long, default_value_t = 0.001)]
    pub scale: f64,
    /// STEP file to export
    pub path: PathBuf,
}
