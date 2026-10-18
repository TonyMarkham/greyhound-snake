use gpui::Point;

/// Open right-click menu on a class chip: the class it targets and the
/// cursor position in window pixels (the menu renders above the
/// cursor, since the chip row sits at the window's bottom edge).
pub(crate) struct ContextMenu {
    pub(crate) class: usize,
    pub(crate) position: Point<f32>,
}
