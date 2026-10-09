use crate::model::Inertial;

/// What a body carries: assembly grouping nodes are geometry-free, meshed
/// bodies reference their unique mesh asset and carry the exact-BRep
/// inertial in MJCF units.
#[derive(Clone, Debug)]
pub enum BodyKind {
    Grouping,
    Meshed {
        /// The resolved `<mesh>` asset name the `<geom>` references.
        mesh: String,
        inertial: Inertial,
    },
}
