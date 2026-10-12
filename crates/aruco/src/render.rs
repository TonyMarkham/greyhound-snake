use crate::marker::Marker;
use crate::units::Units;

use std::io;

/// Renders one marker as a standalone SVG document.
///
/// Geometry is a cell grid (viewBox in cells): a 1-cell black border
/// ring (OpenCV's `borderBits = 1` default), the 4x4 data field, and
/// a quiet zone of `margin` cells on every side — `6 + 2*margin`
/// cells per side, plus one caption row below when labeled.
/// `size`/`units` only set the `width`/`height` attributes; they
/// never change the geometry.
#[derive(Debug, Clone)]
pub struct MarkerSvg {
    marker: Marker,
    margin: u32,
    size: u32,
    units: Units,
    label: bool,
}

impl MarkerSvg {
    /// Defaults: no quiet zone (OpenCV `generateImageMarker` parity —
    /// the black border ring sits flush to the SVG edge), 256 px
    /// square, no caption. A quiet zone is opt-in via `margin`.
    pub fn new(marker: Marker) -> Self {
        Self {
            marker,
            margin: 0,
            size: 256,
            units: Units::Px,
            label: false,
        }
    }

    /// Quiet-zone margin in cells (>= 0), drawn on all four sides.
    pub fn margin(self, margin: u32) -> Self {
        Self { margin, ..self }
    }

    /// Physical side length and unit for `width`/`height`.
    pub fn size(self, size: u32, units: Units) -> Self {
        Self {
            size,
            units,
            ..self
        }
    }

    /// Adds a one-cell caption row below the marker holding the id.
    pub fn label(self, label: bool) -> Self {
        Self { label, ..self }
    }

    /// The marker's width in viewBox cells (6 + 2*margin).
    fn cells_wide(&self) -> u32 {
        6 + 2 * self.margin
    }

    /// The marker's height in viewBox cells (adds the caption row).
    fn cells_tall(&self) -> u32 {
        self.cells_wide() + u32::from(self.label)
    }

    /// Serializes the document to a string.
    pub fn to_svg(&self) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}{}\" height=\"{}{}\">\n",
            self.cells_wide(),
            self.cells_tall(),
            self.size,
            self.units.suffix(),
            self.size,
            self.units.suffix(),
        ));
        out.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n",
            self.cells_wide(),
            self.cells_tall(),
        ));
        write_content(&self.marker, self.margin, self.label, &mut out);
        out.push_str("</svg>\n");
        out
    }

    /// Writes the document to any byte sink.
    pub fn write(&self, out: &mut impl io::Write) -> io::Result<()> {
        out.write_all(self.to_svg().as_bytes())
    }
}

/// The black geometry of one marker: the border ring, one `<rect>`
/// per black data cell (row-major bit order, matching the table's
/// `b = row*4 + column` at position `15 - b`), and the optional
/// caption. Positions are in viewBox cells relative to the marker's
/// own origin; a sheet wraps calls in `<g transform>` translations.
pub(crate) fn write_content(marker: &Marker, margin: u32, label: bool, out: &mut String) {
    let m = margin;
    out.push_str(&format!(
        "<rect x=\"{m}\" y=\"{m}\" width=\"6\" height=\"1\" fill=\"#000000\"/>\n"
    ));
    out.push_str(&format!(
        "<rect x=\"{m}\" y=\"{}\" width=\"1\" height=\"4\" fill=\"#000000\"/>\n",
        m + 1,
    ));
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"4\" fill=\"#000000\"/>\n",
        m + 5,
        m + 1,
    ));
    out.push_str(&format!(
        "<rect x=\"{m}\" y=\"{}\" width=\"6\" height=\"1\" fill=\"#000000\"/>\n",
        m + 5,
    ));
    for bit in 0..16u32 {
        if marker.bits & (1u16 << (15 - bit)) != 0 {
            // The data field hugs the border ring: ring occupies cell
            // m, data starts at m+1 — a data rect at m+2 would leave a
            // one-cell white gap inside the border.
            let x = m + 1 + bit % 4;
            let y = m + 1 + bit / 4;
            out.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"1\" height=\"1\" fill=\"#000000\"/>\n"
            ));
        }
    }
    if label {
        // Caption row: cells [m+6, m+7); the id is centered
        // (width 6 + 2*margin is always even, so the center is an
        // exact cell boundary) and the baseline sits 0.8 into the
        // row, leaving 0.2 cells of padding below.
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}.8\" font-family=\"monospace\" font-size=\"0.8\" text-anchor=\"middle\">{}</text>\n",
            (6 + 2 * m) / 2,
            m + 6,
            marker.id,
        ));
    }
}
