use crate::dictionary::ArucoDictionary;
use crate::sheet::SheetSvg;
use crate::units::Units;

fn sheet() -> SheetSvg {
    SheetSvg::new(ArucoDictionary::Dict4x4_50).expect("dictionary ids are dense")
}

#[test]
fn sheet_of_fifty_cells_lays_out_ten_by_five() {
    // 50 markers, 10 cols -> 5 rows; slot = 6 cells (margin 0 — the
    // border ring sits flush to the slot edge, OpenCV parity),
    // 1-cell gutters, 1-cell quiet zone: 1 + 10*7 = 71 wide,
    // 1 + 5*7 = 36 tall.
    let svg = sheet().to_svg().expect("valid sheet");
    assert!(svg.contains("viewBox=\"0 0 71 36\""), "{svg}");
    assert_eq!(svg.matches("<g transform=\"translate(").count(), 50);
    assert!(svg.contains("<g transform=\"translate(1, 1)\">"), "{svg}");
    // Last slot: column 9 row 4 -> x = 1+9*7 = 64, y = 1+4*7 = 29.
    assert!(svg.contains("<g transform=\"translate(64, 29)\">"), "{svg}");
}

#[test]
fn sheet_aspect_follows_the_view_box() {
    // width 200 px, aspect 36/71 -> height 101.408... -> "101".
    let svg = sheet()
        .cols(10)
        .size(200, Units::Px)
        .to_svg()
        .expect("valid sheet");
    assert!(svg.contains("width=\"200\" height=\"101\""), "{svg}");
    // mm: 200 mm wide -> 101.408 mm tall (3-decimal micron rounding).
    let svg = sheet()
        .cols(10)
        .size(200, Units::Mm)
        .to_svg()
        .expect("valid sheet");
    assert!(
        svg.contains("width=\"200mm\" height=\"101.408mm\""),
        "{svg}"
    );
}

#[test]
fn sheet_with_labels_adds_a_caption_row_per_slot() {
    // Slot height grows to 7: 1 + 5*8 = 41 tall.
    let svg = sheet().label(true).to_svg().expect("valid sheet");
    assert!(svg.contains("viewBox=\"0 0 71 41\""), "{svg}");
    assert!(svg.contains(">49</text>"), "{svg}");
}

#[test]
fn sheet_holds_one_cell_gutters() {
    // Slots span [1, 7) for column 0 (origin 1, slot 6 wide); the
    // gutter column is x=7 — no black rect may start there. (Slot 1
    // starts at x=8, so x=9 legitimately carries slot-1 geometry.)
    let svg = sheet().to_svg().expect("valid sheet");
    let gutter = "<rect x=\"7\" y=\"";
    assert!(
        !svg.contains(gutter),
        "gutter column must not carry a black rect"
    );
}

#[test]
fn zero_columns_is_an_error() {
    let error = sheet().cols(0).to_svg().expect_err("cols=0 must fail");
    assert!(error.to_string().contains("at least one column"));
}

#[test]
fn mm_unit_suffix_is_present() {
    let svg = sheet().size(100, Units::Mm).to_svg().expect("valid sheet");
    assert!(svg.contains("width=\"100mm\""), "{svg}");
}
