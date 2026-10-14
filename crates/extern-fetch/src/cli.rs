use clap::Parser;
use std::path::PathBuf;

/// Downloads the pinned release archives this repo builds against and
/// extracts them into the `extern/` cache (refactor-repo.md D3/D4):
/// download → SHA-256 verify → extract → sibling provenance marker.
#[derive(Parser, Debug)]
pub(crate) struct Cli {
    /// Fetch config; the repo-root-relative default assumes the tool
    /// runs from the repository root (just / cargo run).
    #[arg(long, default_value = "crates/extern-fetch/config.toml")]
    pub(crate) config: PathBuf,

    /// Restrict the run to named groups (repeatable). Default: every
    /// group in the config with a section for this OS.
    #[arg(long)]
    pub(crate) group: Vec<String>,
}
