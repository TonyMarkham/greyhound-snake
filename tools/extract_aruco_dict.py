#!/usr/bin/env python3
"""Extract OpenCV's DICT_4X4_1000_BYTES table as a Rust constant.

Source: modules/objdetect/src/aruco/predefined_dictionaries.hpp in
opencv/opencv, branch 4.x, commit 802cf95aa3d42816995695bf68a86b3532f12760
(2026-10-10). OpenCV is Apache-2.0; the emitted module carries the
attribution required by license section 4.

Usage:
    python3 tools/extract_aruco_dict.py --hpp <path-to-hpp> --out <out.rs>
"""

import argparse
import re
import sys

OPENCV_COMMIT = "802cf95aa3d42816995695bf68a86b3532f12760"
OPENCV_BRANCH = "4.x"
OPENCV_SOURCE = "modules/objdetect/src/aruco/predefined_dictionaries.hpp"
MARKERS = 1000
ROTATIONS = 4
BYTES_PER_ROTATION = 2

HEADER = f"""//! OpenCV's predefined 4x4 ArUco dictionary, transcribed verbatim.
//!
//! Reading the table: a marker is a 4x4 field of cells (1 = black),
//! stored as bytes whose **binary representation is the cell
//! pattern**. Each rotation's two bytes split the field in half:
//! byte 0 = rows 0-1, byte 1 = rows 2-3; within a byte, cells fill
//! MSB-first down the rows, so bit index `b = row*4 + column` sits at
//! position `15 - b` of the pair read as one 16-bit integer.
//!
//! Example (marker 0, rotation 0) — bytes `[181, 50]`:
//!
//! ```text
//!     181 = 0b1011_0101 → row 0 = 1011, row 1 = 0101
//!      50 = 0b0011_0010 → row 2 = 0011, row 3 = 0010
//!
//!     1 0 1 1      row 0  <- 181 high nibble
//!     0 1 0 1      row 1  <- 181 low nibble
//!     0 0 1 1      row 2  <-  50 high nibble
//!     0 0 1 0      row 3  <-  50 low nibble
//! ```
//!
//! The next pair, `[235, 72]`, is the same field turned 90 degrees
//! anticlockwise — the old right column swings into the top row
//! (top-right corner lands at top-left) — decoded by the same rule.
//! The turn rearranges every cell: a cell at `(row, column)` moves to
//! `(3 - column, row)`, so the four decoded grids look unrelated to
//! the eye; the identity is the cell mapping, not the byte values.
//!
//! ```text
//!     235 = 0b1110_1011 → row 0 = 1110, row 1 = 1011
//!      72 = 0b0100_1000 → row 2 = 0100, row 3 = 1000
//!
//!     1 1 1 0      turned row 0  <- 235 high nibble
//!     1 0 1 1      turned row 1  <- 235 low nibble
//!     0 1 0 0      turned row 2  <-  72 high nibble
//!     1 0 0 0      turned row 3  <-  72 low nibble
//! ```
//!
//! Source: `{OPENCV_SOURCE}` in opencv/opencv, branch `{OPENCV_BRANCH}`,
//! commit `{OPENCV_COMMIT}` (<https://github.com/opencv/opencv>),
//! extracted by `tools/extract_aruco_dict.py`. Byte-level layout is
//! documented in OpenCV's official `cv::aruco::Dictionary` class docs
//! (<https://docs.opencv.org/4.13.0/d5/d0b/classcv_1_1aruco_1_1Dictionary.html>):
//! `[marker][rotation][byte]`,
//! `ceil(markerSize^2/8) = {BYTES_PER_ROTATION}` bytes per rotation,
//! rotations precomputed, each rotation a 90 degree anticlockwise turn
//! of the row-major, MSB-first bit field (the MSB-first bit order is
//! documented only in the source: `getBitsFromByteList`).
//!
//! All four predefined 4x4 dictionaries (`DICT_4X4_50/100/250/1000`)
//! are row-count-limited views of this one table (OpenCV
//! `getPredefinedDictionary` constructs them all from this pointer).
//!
//! OpenCV is licensed under the Apache License, Version 2.0
//! (<http://www.apache.org/licenses/LICENSE-2.0>); this transcription
//! is redistributed source data and carries that license's attribution
//! obligations.
"""


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hpp", required=True, help="path to predefined_dictionaries.hpp")
    parser.add_argument("--out", required=True, help="path of the Rust module to write")
    args = parser.parse_args()

    with open(args.hpp, encoding="utf-8") as handle:
        text = handle.read()

    marker = "DICT_4X4_1000_BYTES"
    declaration = text.index(marker)
    body = text.index("{", declaration)
    end = text.index("};", body)
    numbers = [int(token) for token in re.findall(r"\d+", text[body:end])]

    expected = MARKERS * ROTATIONS * BYTES_PER_ROTATION
    if len(numbers) != expected:
        fail(f"parsed {len(numbers)} numbers, expected {expected}")

    const_type = f"[[[u8; {BYTES_PER_ROTATION}]; {ROTATIONS}]; {MARKERS}]"
    lines = HEADER.splitlines()
    lines.append("")
    lines.append(f"pub(crate) const DICT_4X4_1000_BYTES: {const_type} = [")
    for m in range(MARKERS):
        rows = numbers[m * ROTATIONS * BYTES_PER_ROTATION:(m + 1) * ROTATIONS * BYTES_PER_ROTATION]
        rotations = ", ".join(
            f"[{rows[r * BYTES_PER_ROTATION]}, {rows[r * BYTES_PER_ROTATION + 1]}]"
            for r in range(ROTATIONS)
        )
        lines.append(f"    [{rotations}],")
    lines.append("];")
    lines.append("")

    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(
        f"wrote {args.out}: {MARKERS} markers x {ROTATIONS} rotations"
        f" x {BYTES_PER_ROTATION} bytes"
    )


if __name__ == "__main__":
    main()
