use crate::{
    error::{Error, Result},
    marker::Marker,
    render,
};

use std::fmt;

/// One marker rasterized into a grayscale pixel buffer.
///
/// The layout is the SVG's, at integer pixels per module: `size` must
/// be a nonzero multiple of the module grid (6 + 2*margin cells per
/// side), so every module boundary lands on a pixel edge and the black
/// regions are seamless. A fractional module size leaves anti-aliased
/// hairlines at the cell boundaries, which standard dictionaries
/// refuse to decode as the intended id (measured through the OpenCV
/// harness, 2026-10-14: the 1024px 6x6 textures at 170.67px per module
/// carry a 1px ~56/255 seam at every boundary).
pub struct MarkerRaster {
    pixels: Vec<u8>,
    size: u32,
}

impl MarkerRaster {
    /// Renders `marker` at `size` px square with `margin` quiet-zone
    /// cells. White base, black ink — the same geometry as
    /// [`MarkerSvg`](crate::MarkerSvg) (parity-tested in `tests::raster`).
    pub fn new(marker: Marker, margin: u32, size: u32) -> Result<Self> {
        let cells = 6 + 2 * margin;
        if size == 0 || !size.is_multiple_of(cells) {
            return Err(Error::aruco(format!(
                "size {size} must be a nonzero multiple of the module \
                 grid ({cells} cells per side)"
            )));
        }
        let module = size / cells;
        // u32 -> usize is infallible on every real target; the error arm
        // is the numeric-conversion rule's real-error requirement, never
        // a reachable case. All index arithmetic runs in usize from here.
        let size_us = usize::try_from(size).map_err(|_| Error::aruco("size exceeds usize"))?;
        let mut pixels = vec![255u8; size_us * size_us];
        for [x, y, w, h] in render::black_rects(marker, margin) {
            // The products are bounded by `size` (each rect lies inside
            // the grid), so they too are lossless u32 -> usize.
            let start_row =
                usize::try_from(y * module).map_err(|_| Error::aruco("rect row exceeds usize"))?;
            let start_col =
                usize::try_from(x * module).map_err(|_| Error::aruco("rect col exceeds usize"))?;
            let height = usize::try_from(h * module)
                .map_err(|_| Error::aruco("rect height exceeds usize"))?;
            let width = usize::try_from(w * module)
                .map_err(|_| Error::aruco("rect width exceeds usize"))?;
            for dy in 0..height {
                let start = (start_row + dy) * size_us + start_col;
                pixels[start..start + width].fill(0);
            }
        }
        Ok(Self { pixels, size })
    }

    /// The pixel data, row-major, top-down, 8-bit grayscale.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The side length in pixels (the raster is square).
    pub fn size(&self) -> u32 {
        self.size
    }
}

impl fmt::Debug for MarkerRaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarkerRaster")
            .field("size", &self.size)
            .field("pixels", &self.pixels.len())
            .finish()
    }
}
