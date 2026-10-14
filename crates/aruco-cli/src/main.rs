mod cli;
mod target;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use crate::{cli::Cli, target::Target};

use aruco::{
    ArucoError, ArucoResult, MarkerRaster, MarkerSvg, SheetSvg, Units, filter_self_symmetric,
    select,
};
use clap::Parser;
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};

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
    let as_png = is_png(&cli.output);
    if as_png {
        // PNG rasterization is pixel-based: the physical units and the
        // text caption are SVG-only, and the contact sheet has no PNG
        // layout.
        if cli.label {
            return Err(ArucoError::aruco(
                "--label is SVG-only: PNG output has no caption row",
            ));
        }
        if !matches!(cli.units, Units::Px) {
            return Err(ArucoError::aruco(
                "PNG output is pixel-based: --units must be px",
            ));
        }
        if cli.sheet {
            return Err(ArucoError::aruco("--sheet is SVG-only"));
        }
    }
    let (target, count) = (cli.target, cli.count);
    match (target, count) {
        (None, None) | (Some(_), Some(_)) => Err(ArucoError::aruco(
            "either a marker id or --count is required (clap enforces this; this arm is defensive)",
        )),
        (Some(Target::Id(id)), None) => {
            let marker = cli.dict.marker(id)?;
            if as_png {
                let raster = MarkerRaster::new(marker, cli.margin, cli.size)?;
                write_png(&raster, &cli.output)
            } else {
                let svg = MarkerSvg::new(marker)
                    .margin(cli.margin)
                    .size(cli.size, cli.units)
                    .label(cli.label);
                write_document(&svg.to_svg(), &cli.output)
            }
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
                write_dictionary(&cli, as_png, cli.dict.markers())
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
                write_dictionary(&cli, as_png, markers)
            }
        }
    }
}

/// The output format follows the `--output` extension: `.png`
/// rasterizes, anything else writes SVG.
fn is_png(output: &Path) -> bool {
    output
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
}

/// Encodes the raster as an 8-bit grayscale PNG.
fn write_png(raster: &MarkerRaster, output: &Path) -> ArucoResult<()> {
    let file = fs::File::create(output).map_err(|error| {
        ArucoError::aruco(format!("cannot create {}: {error}", output.display()))
    })?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), raster.size(), raster.size());
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| {
        ArucoError::aruco(format!(
            "png header failed for {}: {error}",
            output.display()
        ))
    })?;
    writer.write_image_data(raster.pixels()).map_err(|error| {
        ArucoError::aruco(format!(
            "png write failed for {}: {error}",
            output.display()
        ))
    })?;
    Ok(())
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

/// Batch mode: one `marker_{id:04}.{svg|png}` per marker inside the
/// output directory, created if missing. Consumes a fallible marker
/// stream so both the full dictionary and a selection share the writer.
fn write_dictionary(
    cli: &Cli,
    as_png: bool,
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
        let path = file_name(marker.id, &cli.output, as_png);
        if as_png {
            let raster = MarkerRaster::new(marker, cli.margin, cli.size)?;
            write_png(&raster, &path)?;
        } else {
            let svg = MarkerSvg::new(marker)
                .margin(cli.margin)
                .size(cli.size, cli.units)
                .label(cli.label);
            write_document(&svg.to_svg(), &path)?;
        }
    }
    Ok(())
}

/// `marker_{id:04}.{ext}` inside `directory` — zero-padded so directory
/// listings sort numerically.
fn file_name(id: u32, directory: &Path, as_png: bool) -> std::path::PathBuf {
    let extension = if as_png { "png" } else { "svg" };
    directory.join(format!("marker_{id:04}.{extension}"))
}
