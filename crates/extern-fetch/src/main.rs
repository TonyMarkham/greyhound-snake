mod archive;
mod artifact;
mod cli;
mod config;
mod digest;
mod download;
mod ensure;
mod error;
mod group;
mod host_os;
mod platform;
mod provenance;
mod staged_archive;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use error::{Error as ExternFetchError, Result as ExternFetchResult};

// ---------------------------------------------------------------------------------------------- //

use crate::{
    cli::Cli,
    config::Config,
    ensure::{Outcome, ensure},
    host_os::HostOs,
};

use clap::Parser;
use std::process::ExitCode;
fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `Ok(true)` = every selected artifact up to date or extracted+verified.
/// `Ok(false)` = one or more artifacts failed or await pinning; their
/// messages are already printed. `Err` = the run could not start.
fn run(cli: &Cli) -> ExternFetchResult<bool> {
    let config_path = &cli.config;
    let config = Config::load(config_path)?;
    let repo_root = Config::repo_root(config_path)?;
    // Validated up front so a bad download_location fails the run
    // before anything is fetched (the staging mode is run-global).
    let download_dir = config.download_dir()?;
    let os = HostOs::current()?;
    for name in &cli.group {
        if config.group(name).is_none() {
            return Err(ExternFetchError::config(format!(
                "--group {name}: no such group in {} (known: {})",
                config_path.display(),
                config.group_names().join(", ")
            )));
        }
    }
    let mut ok = true;
    let mut unpinned = false;
    for (name, group) in config.selected_groups(&cli.group) {
        let Some(platform) = group.section(os) else {
            eprintln!(
                "{name}: no [{}.{}] section — nothing configured for this OS",
                name,
                os.section_key()
            );
            ok = false;
            continue;
        };
        for (role, artifact) in platform.artifacts() {
            // The package payload is written exclusively by the package's
            // C# provisioner (only the C# fetcher puts anything in the
            // dist package). The
            // `payload` entries exist in this config as the pins source
            // that assemble-package stages into the package
            // (Editor/occt-fetch.toml); the CLI fetches only the extern/
            // dev trees and must never write into the package.
            if role == "payload" {
                continue;
            }
            match ensure(
                name,
                role,
                group,
                artifact,
                &repo_root,
                download_dir.clone(),
            ) {
                Ok(Outcome::Skipped) => {
                    println!("{name}/{role}: up to date ({})", artifact.dest());
                }
                Ok(Outcome::Extracted { files }) => {
                    println!(
                        "{name}/{role}: verified + extracted {files} files → {}",
                        artifact.dest()
                    );
                }
                Ok(Outcome::NeedsPin { sha256 }) => {
                    println!("{name}/{role}: UNPINNED — extracted anyway; sha256 = {sha256}");
                    unpinned = true;
                }
                Err(error) => {
                    eprintln!("{name}/{role}: error: {error}");
                    ok = false;
                }
            }
        }
    }
    if unpinned {
        eprintln!(
            "unpinned entries remain: pin the reported sha256 values into {} and re-run (the cache is already populated; the next run only verifies)",
            cli.config.display()
        );
    }
    Ok(ok && !unpinned)
}
