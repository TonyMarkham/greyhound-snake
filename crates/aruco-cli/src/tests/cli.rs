use crate::cli::Cli;
use crate::target::Target;

use aruco::{ArucoDictionary, Selection};
use clap::{CommandFactory, Parser};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("aruco").chain(args.iter().copied()))
}

#[test]
fn single_id_parses_with_defaults() {
    let cli = parse(&["7", "-o", "marker_7.svg"]).expect("valid arguments");
    assert_eq!(cli.target, Some(Target::Id(7)));
    assert_eq!(cli.dict, ArucoDictionary::Dict4x4_50);
    assert_eq!(cli.size, 256);
    assert_eq!(cli.margin, 0);
    assert!(!cli.label);
    assert!(!cli.sheet);
}

#[test]
fn id_zero_is_valid() {
    let cli = parse(&["0", "-o", "out.svg"]).expect("id 0 is valid");
    assert_eq!(cli.target, Some(Target::Id(0)));
}

#[test]
fn all_parses_and_sheet_takes_cols() {
    let cli = parse(&["all", "-o", "dir", "--sheet", "--cols", "8"]).expect("valid arguments");
    assert_eq!(cli.target, Some(Target::All));
    assert!(cli.sheet);
    assert_eq!(cli.cols, 8);
}

#[test]
fn unknown_dictionary_is_rejected() {
    let error = parse(&["7", "-o", "x.svg", "--dict", "5x5_50"])
        .expect_err("5x5 is not a supported dictionary");
    assert!(error.to_string().contains("unknown dictionary"), "{error}");
}

#[test]
fn non_numeric_positional_is_rejected() {
    let error = parse(&["some-file", "-o", "x.svg"]).expect_err("not a number");
    assert!(
        error.to_string().contains("expected a marker id"),
        "{error}"
    );
}

#[test]
fn cols_requires_sheet_at_parse_time() {
    let error =
        parse(&["all", "-o", "dir", "--cols", "10"]).expect_err("--cols without --sheet must fail");
    assert!(error.to_string().contains("--sheet"), "{error}");
}

#[test]
fn sheet_with_a_single_id_is_rejected() {
    let cli = parse(&["7", "-o", "x.svg", "--sheet"]).expect("parses fine");
    let error = cli.validate().expect_err("--sheet needs a marker set");
    assert!(error.to_string().contains("`all` or `--count`"), "{error}");
}

#[test]
fn count_parses_and_conflicts_with_the_positional() {
    let cli = parse(&["--count", "12", "-o", "dir"]).expect("valid arguments");
    assert_eq!(cli.count, Some(12));
    assert_eq!(cli.target, None);
    assert_eq!(cli.selection(), Selection::FirstN);

    let error = parse(&["7", "--count", "12", "-o", "dir"])
        .expect_err("id and --count are mutually exclusive");
    assert!(error.to_string().contains("cannot be used with"), "{error}");
}

#[test]
fn seed_and_min_distance_require_count() {
    let error = parse(&["--seed", "7", "-o", "dir"]).expect_err("--seed without --count must fail");
    assert!(error.to_string().contains("--count"), "{error}");

    let error = parse(&["--min-distance", "6", "-o", "dir"])
        .expect_err("--min-distance without --count must fail");
    assert!(error.to_string().contains("--count"), "{error}");
}

#[test]
fn seed_plus_floor_selects_max_distance() {
    let cli = parse(&[
        "--count",
        "12",
        "--seed",
        "42",
        "--min-distance",
        "6",
        "-o",
        "dir",
    ])
    .expect("valid arguments");
    assert_eq!(
        cli.selection(),
        Selection::MaxDistance {
            seed: 42,
            min_distance: 6
        }
    );
}

#[test]
fn count_is_at_least_one_via_selection() {
    // --count 0 parses; selection rejects it with a clear error.
    let cli = parse(&["--count", "0", "-o", "dir"]).expect("parses fine");
    assert_eq!(cli.count, Some(0));
}

#[test]
fn help_documents_the_layout() {
    let mut command = Cli::command();
    let help = command.render_help().to_string();
    assert!(help.contains("ID|ALL"), "{help}");
    assert!(help.contains("4x4_50"), "{help}");
    assert!(help.contains("--count"), "{help}");
}
