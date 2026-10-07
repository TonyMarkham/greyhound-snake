use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for the step-stats CLI.
#[derive(Parser)]
#[command(about = "Print geometry stats for a STEP file")]
pub struct Args {
    /// Runtime configuration file
    #[arg(long, default_value = "config.toml")]
    pub config: PathBuf,
    /// STEP file to inspect
    pub path: PathBuf,
}
