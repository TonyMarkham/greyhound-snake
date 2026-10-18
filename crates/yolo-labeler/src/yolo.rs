use std::path::{Path, PathBuf};

/// One YOLO detection: class id plus center-relative, image-normalized
/// geometry. Every coordinate is in `0.0..=1.0`, matching the YOLO txt
/// format `class cx cy w h`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct YoloBox {
    pub(crate) class: u32,
    pub(crate) cx: f32,
    pub(crate) cy: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl YoloBox {
    /// Builds a box from two drag corners in either order; the result is
    /// clamped into the image.
    pub(crate) fn from_corners(class: u32, x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        let (left, right) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (top, bottom) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        Self {
            class,
            cx: ((left + right) / 2.0).clamp(0.0, 1.0),
            cy: ((top + bottom) / 2.0).clamp(0.0, 1.0),
            w: (right - left).clamp(0.0, 1.0),
            h: (bottom - top).clamp(0.0, 1.0),
        }
    }

    /// Hit test in normalized coordinates: inside the axis-aligned rect.
    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        (x - self.cx).abs() <= self.w / 2.0 && (y - self.cy).abs() <= self.h / 2.0
    }

    pub(crate) fn x0(&self) -> f32 {
        self.cx - self.w / 2.0
    }

    pub(crate) fn x1(&self) -> f32 {
        self.cx + self.w / 2.0
    }

    pub(crate) fn y0(&self) -> f32 {
        self.cy - self.h / 2.0
    }

    pub(crate) fn y1(&self) -> f32 {
        self.cy + self.h / 2.0
    }
}

/// YOLO convention: the label file for `images/foo.jpg` is `foo.txt`
/// beside it — the extension is replaced, not appended.
pub(crate) fn label_path_for(image: &Path) -> PathBuf {
    image.with_extension("txt")
}

/// Parses a label file body. Blank lines and `#` comments are skipped;
/// malformed lines are dropped so one bad line cannot hide the rest of
/// the file. Out-of-range values are clamped into the image.
pub(crate) fn parse_labels(text: &str) -> Vec<YoloBox> {
    text.lines().filter_map(parse_line).collect()
}

/// Formats boxes as YOLO txt lines (`class cx cy w h`, six decimals),
/// one detection per line, newline-terminated.
pub(crate) fn format_labels(boxes: &[YoloBox]) -> String {
    let mut out = String::new();
    for label in boxes {
        out.push_str(&format!(
            "{} {:.6} {:.6} {:.6} {:.6}\n",
            label.class, label.cx, label.cy, label.w, label.h
        ));
    }
    out
}

/// The dataset format's never-wipe rule: a writer refuses to rewrite a
/// label file to empty when it currently holds lines (empty files are
/// only ever legitimate from their writer, never from an edit). Pass
/// the raw existing file text (missing file = empty string) and the
/// would-be write.
pub(crate) fn wipes_labels(existing: &str, updated: &str) -> bool {
    !existing.trim().is_empty() && updated.trim().is_empty()
}

fn parse_line(line: &str) -> Option<YoloBox> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fields = line.split_whitespace();
    let class = fields.next()?.parse::<u32>().ok()?;
    let cx = fields.next()?.parse::<f32>().ok()?;
    let cy = fields.next()?.parse::<f32>().ok()?;
    let w = fields.next()?.parse::<f32>().ok()?;
    let h = fields.next()?.parse::<f32>().ok()?;
    if fields.next().is_some() {
        return None;
    }
    Some(YoloBox {
        class,
        cx: cx.clamp(0.0, 1.0),
        cy: cy.clamp(0.0, 1.0),
        w: w.clamp(0.0, 1.0),
        h: h.clamp(0.0, 1.0),
    })
}
