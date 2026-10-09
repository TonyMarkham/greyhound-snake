use crate::model::BodyKind;

/// One `<body>` element: a scene node in depth-first pre-order, positioned
/// in its parent frame. `depth` counts its ancestors; the serializer uses it
/// to open and close the nested tags.
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub comment: String,
    pub depth: usize,
    pub pos: [f64; 3],
    pub quat: [f64; 4],
    pub kind: BodyKind,
}
