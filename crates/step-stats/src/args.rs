use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for the step-stats CLI.
#[derive(Parser)]
#[command(about = "Print geometry stats for a STEP file")]
pub struct Args {
    /// Runtime configuration file
    #[arg(long, default_value = "config.toml")]
    pub config: PathBuf,
    /// Also tessellate the STEP file and report mesh statistics
    #[arg(long)]
    pub mesh: bool,
    /// Also project the tessellated mesh into Unity buffer layout
    #[arg(long)]
    pub unity: bool,
    /// Also list the assembly forest node names
    #[arg(long)]
    pub names: bool,
    /// STEP file to inspect
    pub path: PathBuf,
}
