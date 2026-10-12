mod cli;
mod target;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::cli::Cli;
use crate::target::Target;

use aruco::{ArucoError, ArucoResult, MarkerSvg, SheetSvg, filter_self_symmetric, select};
use clap::Parser;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> ArucoResult<()> {
    cli.validate()?;
    let (target, count) = (cli.target, cli.count);
    match (target, count) {
        (None, None) | (Some(_), Some(_)) => Err(ArucoError::aruco(
            "either a marker id or --count is required (clap enforces this; this arm is defensive)",
        )),
        (Some(Target::Id(id)), None) => {
            let marker = cli.dict.marker(id)?;
            let svg = MarkerSvg::new(marker)
                .margin(cli.margin)
                .size(cli.size, cli.units)
                .label(cli.label);
            write_document(&svg.to_svg(), &cli.output)
        }
        (Some(Target::All), None) => {
            if cli.sheet {
                let sheet = SheetSvg::new(cli.dict)?
                    .cols(cli.cols)
                    .margin(cli.margin)
                    .size(cli.size, cli.units)
                    .label(cli.label);
                write_document(&sheet.to_svg()?, &cli.output)
            } else {
                write_dictionary(&cli, cli.dict.markers())
            }
        }
        (None, Some(count)) => {
            // Selection mode: rotationally-unambiguous markers only,
            // then the chosen strategy.
            let pool = filter_self_symmetric(cli.dict)?;
            let picked = select(&pool, count, cli.selection())?;
            let ids: Vec<String> = picked.iter().map(|marker| marker.id.to_string()).collect();
            println!("selected {} markers: {}", picked.len(), ids.join(" "));
            if cli.sheet {
                let sheet = SheetSvg::from_markers(picked)
                    .cols(cli.cols)
                    .margin(cli.margin)
                    .size(cli.size, cli.units)
                    .label(cli.label);
                write_document(&sheet.to_svg()?, &cli.output)
            } else {
                let markers = picked.into_iter().map(Ok);
                write_dictionary(&cli, markers)
            }
        }
    }
}

fn write_document(svg: &str, output: &Path) -> ArucoResult<()> {
    let file = fs::File::create(output).map_err(|error| {
        ArucoError::aruco(format!("cannot create {}: {error}", output.display()))
    })?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(svg.as_bytes())
        .map_err(|error| ArucoError::aruco(format!("write failed: {error}")))
}

/// Batch mode: one `marker_{id:04}.svg` per marker inside the output
/// directory, created if missing. Consumes a fallible marker stream
/// so both the full dictionary and a selection share the writer.
fn write_dictionary(
    cli: &Cli,
    markers: impl Iterator<Item = ArucoResult<aruco::Marker>>,
) -> ArucoResult<()> {
    fs::create_dir_all(&cli.output).map_err(|error| {
        ArucoError::aruco(format!(
            "cannot create directory {}: {error}",
            cli.output.display()
        ))
    })?;
    for marker in markers {
        let marker = marker?;
        let svg = MarkerSvg::new(marker)
            .margin(cli.margin)
            .size(cli.size, cli.units)
            .label(cli.label);
        write_document(&svg.to_svg(), &file_name(marker.id, &cli.output))?;
    }
    Ok(())
}

/// `marker_{id:04}.svg` inside `directory` — zero-padded so directory
/// listings sort numerically.
fn file_name(id: u32, directory: &Path) -> std::path::PathBuf {
    directory.join(format!("marker_{id:04}.svg"))
}
