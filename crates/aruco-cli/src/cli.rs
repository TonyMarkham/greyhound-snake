use crate::target::Target;

use aruco::{ArucoDictionary, ArucoError, ArucoResult, Selection, Units};
use clap::Parser;
use std::path::PathBuf;

/// Generates 4x4 ArUco marker SVGs from OpenCV's predefined
/// dictionaries.
#[derive(Parser, Debug)]
pub(crate) struct Cli {
    /// A marker id (0-based, id 0 is valid) or `all` for the whole
    /// dictionary. Required unless `--count` is given.
    #[arg(value_name = "ID|ALL", required_unless_present = "count")]
    pub(crate) target: Option<Target>,

    /// Pick `count` rotationally-unambiguous markers out of the
    /// dictionary instead of using the positional argument.
    #[arg(long, conflicts_with = "target")]
    pub(crate) count: Option<u32>,

    /// Selection reproducibility seed (requires `--count`). With
    /// `--min-distance`, selects greedily for robustness.
    #[arg(long, requires = "count")]
    pub(crate) seed: Option<u64>,

    /// Minimum pairwise Hamming distance floor for the selection,
    /// over all rotations (requires `--count`; needs `--seed`).
    #[arg(long, requires = "count")]
    pub(crate) min_distance: Option<u32>,

    /// Predefined dictionary.
    #[arg(short, long, default_value = "4x4_50")]
    pub(crate) dict: ArucoDictionary,

    /// Output: a file path in single-id and `--sheet` mode, a
    /// directory (created if missing) in `all`/`--count` mode.
    #[arg(short, long)]
    pub(crate) output: PathBuf,

    /// Rendered width: the marker's side length for individual
    /// files, the sheet's width in `--sheet` mode.
    #[arg(long, default_value_t = 256)]
    pub(crate) size: u32,

    /// Unit for `--size`.
    #[arg(long, default_value = "px")]
    pub(crate) units: Units,

    /// Quiet-zone margin around the border, in cells (default 0:
    /// OpenCV `generateImageMarker` parity — the black ring sits
    /// flush to the SVG edge; add white surroundings explicitly).
    #[arg(long, default_value_t = 0)]
    pub(crate) margin: u32,

    /// Caption row with the marker id below each marker.
    #[arg(long)]
    pub(crate) label: bool,

    /// Emit one contact-sheet SVG instead of individual files
    /// (requires `all` or `--count`).
    #[arg(long)]
    pub(crate) sheet: bool,

    /// Contact-sheet columns (default 10; requires `--sheet`).
    #[arg(long, requires = "sheet", default_value_t = 10)]
    pub(crate) cols: u32,
}

impl Cli {
    /// Cross-flag rules clap cannot express: a sheet needs a marker
    /// *set* — `all` (whole dict) or `--count` (a selection) — never
    /// a single id.
    pub(crate) fn validate(&self) -> ArucoResult<()> {
        if self.sheet && matches!(self.target, Some(Target::Id(_))) {
            return Err(ArucoError::aruco("--sheet requires `all` or `--count`"));
        }
        Ok(())
    }

    /// The selection strategy implied by the flag combination:
    /// `FirstN` without `--seed`, `Seeded` with it, `MaxDistance`
    /// with seed + floor.
    pub(crate) fn selection(&self) -> Selection {
        match self.seed {
            Some(seed) => match self.min_distance {
                Some(min_distance) => Selection::MaxDistance { seed, min_distance },
                None => Selection::Seeded { seed },
            },
            None => Selection::FirstN,
        }
    }
}
