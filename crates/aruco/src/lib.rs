mod data;
mod dictionary;
mod error;
mod hamming;
mod marker;
mod prng;
mod render;
mod rotation;
mod select;
mod sheet;
mod units;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use dictionary::ArucoDictionary;
pub use error::{Error as ArucoError, result::Result as ArucoResult};
pub use hamming::{TAU_TARGET, hamming, min_rotation_hamming, self_distance};
pub use marker::Marker;
pub use prng::SplitMix64;
pub use render::MarkerSvg;
pub use rotation::rotations;
pub use select::{Selection, filter_self_symmetric, select};
pub use sheet::SheetSvg;
pub use units::Units;
