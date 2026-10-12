use crate::dictionary::ArucoDictionary;
use crate::error::{Error, Result};
use crate::marker::Marker;
use crate::render::write_content;
use crate::units::Units;

use std::io;

/// Renders a set of markers as one contact-sheet SVG: markers in a
/// `cols`-wide grid, one cell of gutter between neighbors and a
/// one-cell quiet zone around the edge.
#[derive(Debug, Clone)]
pub struct SheetSvg {
    markers: Vec<Marker>,
    cols: u32,
    margin: u32,
    size: u32,
    units: Units,
    label: bool,
}

impl SheetSvg {
    /// A sheet of every marker in the dictionary, in id order.
    pub fn new(dict: ArucoDictionary) -> Result<Self> {
        let mut markers = Vec::new();
        for marker in dict.markers() {
            markers.push(marker?);
        }
        Ok(Self {
            markers,
            cols: 10,
            margin: 0,
            size: 256,
            units: Units::Px,
            label: false,
        })
    }

    /// A sheet of an explicit marker set (e.g. a `--count` selection),
    /// in the given order.
    pub fn from_markers(markers: Vec<Marker>) -> Self {
        Self {
            markers,
            cols: 10,
            margin: 0,
            size: 256,
            units: Units::Px,
            label: false,
        }
    }

    /// Columns of the grid (>= 1); rows are `ceil(count / cols)`.
    pub fn cols(self, cols: u32) -> Self {
        Self { cols, ..self }
    }

    /// Quiet-zone margin around each marker, in cells.
    pub fn margin(self, margin: u32) -> Self {
        Self { margin, ..self }
    }

    /// Physical sheet width for the `width` attribute; `height`
    /// follows the viewBox aspect (px rounds to whole pixels, mm to
    /// microns — 3 decimals), so a sheet is never stretched.
    pub fn size(self, size: u32, units: Units) -> Self {
        Self {
            size,
            units,
            ..self
        }
    }

    /// Caption row (the id) under every marker.
    pub fn label(self, label: bool) -> Self {
        Self { label, ..self }
    }

    /// One marker's slot: width/height in cells, including its own
    /// quiet zone and caption row.
    fn slot(&self) -> (u32, u32) {
        let side = 6 + 2 * self.margin;
        (side, side + u32::from(self.label))
    }

    /// Columns actually used: never more than the marker count (a
    /// 6-marker sheet at 10 columns would carry four empty slots).
    /// `cells()` and `to_svg()` must agree on this — positioning and
    /// sizing derive from the same value.
    fn effective_cols(&self) -> Result<u32> {
        if self.cols == 0 {
            return Err(Error::aruco("sheet needs at least one column"));
        }
        let count = u32::try_from(self.markers.len())
            .map_err(|_| Error::aruco("marker count exceeds u32"))?;
        Ok(self.cols.min(count.max(1)))
    }

    /// The sheet's total size in cells.
    fn cells(&self) -> Result<(u32, u32)> {
        let count = u32::try_from(self.markers.len())
            .map_err(|_| Error::aruco("marker count exceeds u32"))?;
        let cols = self.effective_cols()?;
        let rows = count.div_ceil(cols);
        let (slot_w, slot_h) = self.slot();
        // 1-cell gutter between slots; 1-cell quiet zone around the
        // edge: total = 1 + cols*(slot + 1).
        let width = 1 + cols * (slot_w + 1);
        let height = 1 + rows * (slot_h + 1);
        Ok((width, height))
    }

    /// Serializes the document to a string.
    pub fn to_svg(&self) -> Result<String> {
        let (width, height) = self.cells()?;
        let (slot_w, slot_h) = self.slot();
        let cols = self.effective_cols()?;
        let aspect = f64::from(height) / f64::from(width);
        let height_attr = match self.units {
            Units::Px => format!("{:.0}", f64::from(self.size) * aspect),
            Units::Mm => format!("{:.3}", f64::from(self.size) * aspect),
        };
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{}{}\" height=\"{}{}\">\n",
            self.size,
            self.units.suffix(),
            height_attr,
            self.units.suffix(),
        ));
        out.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#ffffff\"/>\n"
        ));
        for (index, marker) in self.markers.iter().enumerate() {
            // Slots follow enumeration order, not marker id: a
            // `--count` selection holds non-dense ids, and the grid
            // must have no holes. The caption still shows the id.
            let index =
                u32::try_from(index).map_err(|_| Error::aruco("marker count exceeds u32"))?;
            let column = index % cols;
            let row = index / cols;
            out.push_str(&format!(
                "<g transform=\"translate({}, {})\">\n",
                1 + column * (slot_w + 1),
                1 + row * (slot_h + 1),
            ));
            write_content(marker, self.margin, self.label, &mut out);
            out.push_str("</g>\n");
        }
        out.push_str("</svg>\n");
        Ok(out)
    }

    /// Writes the document to any byte sink.
    pub fn write(&self, out: &mut impl io::Write) -> Result<()> {
        let svg = self.to_svg()?;
        out.write_all(svg.as_bytes())
            .map_err(|error| Error::aruco(format!("sheet write failed: {error}")))
    }
}
