use crate::{ArucoDictionary, MarkerRaster, MarkerSvg, marker::Marker, render};

fn marker_zero() -> Marker {
    ArucoDictionary::Dict4x4_1000
        .marker(0)
        .expect("marker 0 exists")
}

// The doc example of the table's byte layout: marker 0, rotation 0,
// bytes [181, 50] -> rows 1011 / 0101 / 0011 / 0010 (1 = white).
const MARKER0_WHITE: [[u8; 4]; 4] = [[1, 0, 1, 1], [0, 1, 0, 1], [0, 0, 1, 1], [0, 0, 1, 0]];

#[test]
fn dimensions_match_size() {
    let raster = MarkerRaster::new(marker_zero(), 1, 512).expect("valid size");
    assert_eq!(raster.size(), 512);
    assert_eq!(raster.pixels().len(), 512 * 512);
}

#[test]
fn non_multiple_size_is_an_error() {
    // margin 1 -> 8 cells per side; 500 is not a multiple of 8.
    assert!(MarkerRaster::new(marker_zero(), 1, 500).is_err());
}

#[test]
fn pixels_are_binary_seam_free() {
    // A seam (an anti-aliased boundary pixel) would show as an
    // intermediate gray. The raster is pixel-exact, so every pixel is
    // pure black or pure white.
    let raster = MarkerRaster::new(marker_zero(), 1, 512).expect("valid size");
    assert!(raster.pixels().iter().all(|p| *p == 0 || *p == 255));
}

#[test]
fn data_cells_match_the_dictionary() {
    // margin 1 -> the data cell (r, c) sits at grid cell (2+c, 2+r);
    // size 512 / 8 cells = 64px per cell.
    let raster = MarkerRaster::new(marker_zero(), 1, 512).expect("valid size");
    let module = 64usize;
    for (r, row) in MARKER0_WHITE.iter().enumerate() {
        for (c, bit) in row.iter().enumerate() {
            let x = (2 + c) * module + module / 2;
            let y = (2 + r) * module + module / 2;
            let expected = if *bit == 1 { 255u8 } else { 0u8 };
            assert_eq!(
                raster.pixels()[y * 512 + x],
                expected,
                "data cell (r{r},c{c})"
            );
        }
    }
}

#[test]
fn quiet_zone_and_ring_sample_correctly() {
    let raster = MarkerRaster::new(marker_zero(), 1, 512).expect("valid size");
    let module = 64usize;
    // the quiet zone's center cell (grid 0,0)
    assert_eq!(raster.pixels()[32 * 512 + 32], 255);
    // the ring's corner cell (grid 1,1)
    assert_eq!(
        raster.pixels()[(module + module / 2) * 512 + module + module / 2],
        0
    );
}

#[test]
fn svg_geometry_parity() {
    // black_rects is the single geometry source; the SVG must carry
    // exactly one black <rect> per entry and one white base rect.
    let marker = marker_zero();
    let rects = render::black_rects(marker, 1);
    assert_eq!(rects.len(), 12); // 4 ring rects + 8 black data cells
    let svg = MarkerSvg::new(marker).margin(1).to_svg();
    assert_eq!(svg.matches("fill=\"#000000\"").count(), rects.len());
    assert_eq!(svg.matches("fill=\"#ffffff\"").count(), 1);
}
