use gpui::Point;

/// Active drag on the image plane. Normalized coordinates are
/// `0.0..=1.0` image coordinates; pixel coordinates are view-local.
pub(crate) enum Drag {
    /// Left drag started on empty space: the two corners tracked so far.
    Creating {
        origin: Point<f32>,
        corner: Point<f32>,
    },
    /// Left drag started inside box `index`: cursor-to-center offset kept
    /// constant while the box follows the cursor.
    Moving { index: usize, offset: Point<f32> },
    /// Left drag on an edge or corner of the selected box: the opposite
    /// edge(s) stay fixed; a `None` axis keeps that axis's span.
    Resizing {
        index: usize,
        fixed_x: Option<f32>,
        fixed_y: Option<f32>,
    },
    /// Middle drag pans the zoomed plane.
    Panning {
        start_cursor: Point<f32>,
        start_pan: Point<f32>,
    },
}
