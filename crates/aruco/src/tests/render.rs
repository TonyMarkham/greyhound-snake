use crate::marker::Marker;
use crate::render::MarkerSvg;
use crate::units::Units;

/// Marker 0 (bits 0xb532), defaults: margin 0 (the black ring sits
/// flush to the SVG edge — OpenCV `generateImageMarker` parity),
/// 256 px, no caption.
#[test]
fn marker_zero_document_matches_the_expected_svg() {
    let svg = MarkerSvg::new(Marker {
        id: 0,
        bits: 0xb532,
    })
    .to_svg();
    let expected = "\
<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 6 6\" width=\"256\" height=\"256\">
<rect x=\"0\" y=\"0\" width=\"6\" height=\"6\" fill=\"#ffffff\"/>
<rect x=\"0\" y=\"0\" width=\"6\" height=\"1\" fill=\"#000000\"/>
<rect x=\"0\" y=\"1\" width=\"1\" height=\"4\" fill=\"#000000\"/>
<rect x=\"5\" y=\"1\" width=\"1\" height=\"4\" fill=\"#000000\"/>
<rect x=\"0\" y=\"5\" width=\"6\" height=\"1\" fill=\"#000000\"/>
<rect x=\"1\" y=\"1\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"3\" y=\"1\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"4\" y=\"1\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"2\" y=\"2\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"4\" y=\"2\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"3\" y=\"3\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"4\" y=\"3\" width=\"1\" height=\"1\" fill=\"#000000\"/>
<rect x=\"3\" y=\"4\" width=\"1\" height=\"1\" fill=\"#000000\"/>
</svg>
";
    assert_eq!(svg, expected);
}

#[test]
fn margin_extends_the_view_box_on_every_side() {
    let svg = MarkerSvg::new(Marker {
        id: 3,
        bits: 0x1234,
    })
    .margin(2)
    .to_svg();
    assert!(svg.contains("viewBox=\"0 0 10 10\""), "{svg}");
    // Quiet zone: no black geometry may reach the outer 2 cells.
    for x in [0, 1, 8, 9] {
        for y in 0..10u32 {
            assert!(
                !has_black_at(&svg, x, y),
                "cell ({x}, {y}) must be quiet zone"
            );
        }
    }
}

#[test]
fn label_adds_one_caption_row_below_the_border() {
    let svg = MarkerSvg::new(Marker {
        id: 42,
        bits: 0x1234,
    })
    .label(true)
    .to_svg();
    assert!(svg.contains("viewBox=\"0 0 6 7\""), "{svg}");
    let expected = "<text x=\"3\" y=\"6.8\" font-family=\"monospace\" font-size=\"0.8\" text-anchor=\"middle\">42</text>\n";
    assert!(svg.contains(expected), "{svg}");
}

#[test]
fn units_carry_into_width_height_only() {
    let svg = MarkerSvg::new(Marker {
        id: 0,
        bits: 0xb532,
    })
    .size(50, Units::Mm)
    .to_svg();
    assert!(svg.contains("width=\"50mm\" height=\"50mm\""), "{svg}");
    assert!(svg.contains("viewBox=\"0 0 6 6\""), "{svg}");
}

#[test]
fn serialization_is_deterministic() {
    let build = || {
        MarkerSvg::new(Marker {
            id: 7,
            bits: 0x4321,
        })
        .to_svg()
    };
    assert_eq!(build(), build());
}

/// Reads the rendered grid back out of the SVG text: cell `(x, y)`
/// (in viewBox cells) is black only where a unit rect covers it.
fn has_black_at(svg: &str, x: u32, y: u32) -> bool {
    let pattern = format!("<rect x=\"{x}\" y=\"{y}\" width=\"1\" height=\"1\" fill=\"#000000\"/>");
    svg.contains(&pattern)
}
