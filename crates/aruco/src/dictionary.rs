use crate::data;
use crate::error::{Error, Result};
use crate::marker::Marker;

use std::str::FromStr;

/// A predefined OpenCV 4x4 ArUco dictionary.
///
/// The four sizes are nested: the first 50 markers of
/// [`ArucoDictionary::Dict4x4_1000`] are `Dict4x4_50`, the first 100
/// are `Dict4x4_100`, the first 250 are `Dict4x4_250` (OpenCV builds
/// all four from the same table pointer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArucoDictionary {
    Dict4x4_50,
    Dict4x4_100,
    Dict4x4_250,
    Dict4x4_1000,
}

impl ArucoDictionary {
    /// The four predefined dictionary sizes in nested order.
    pub const ALL: [ArucoDictionary; 4] = [
        ArucoDictionary::Dict4x4_50,
        ArucoDictionary::Dict4x4_100,
        ArucoDictionary::Dict4x4_250,
        ArucoDictionary::Dict4x4_1000,
    ];

    /// Number of markers in the dictionary (50/100/250/1000).
    pub fn marker_count(self) -> u32 {
        match self {
            ArucoDictionary::Dict4x4_50 => 50,
            ArucoDictionary::Dict4x4_100 => 100,
            ArucoDictionary::Dict4x4_250 => 250,
            ArucoDictionary::Dict4x4_1000 => 1000,
        }
    }

    /// The marker with the given id. Ids are 0-based and dense; id 0
    /// is valid.
    pub fn marker(self, id: u32) -> Result<Marker> {
        let count = self.marker_count();
        if id >= count {
            return Err(Error::aruco(format!(
                "marker id {id} is out of range for a dictionary of {count} markers"
            )));
        }
        let index = usize::try_from(id)
            .map_err(|_| Error::aruco(format!("marker id {id} does not fit usize")))?;
        let bytes = self.bytes()[index];
        Ok(Marker::from_rotation0_bytes(id, &bytes))
    }

    /// Every marker of the dictionary, in id order. Each lookup
    /// carries its own `Result`; the ids are dense, so in practice the
    /// iterator is infallible, but the errors stay explicit.
    pub fn markers(self) -> impl Iterator<Item = Result<Marker>> {
        (0..self.marker_count()).map(move |id| self.marker(id))
    }

    /// The stored rotation bytes backing the dictionary — all four
    /// rotations of every marker, as OpenCV stores them. All four
    /// dictionary sizes share the one physical table; the size is the
    /// `marker_count` bound on reads, exactly how OpenCV's
    /// `Mat(rows, …, ptr)` limits rows over the same pointer.
    fn bytes(self) -> &'static [[[u8; 2]; 4]] {
        &data::DICT_4X4_1000_BYTES
    }
}

impl FromStr for ArucoDictionary {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        match text {
            "4x4_50" => Ok(Self::Dict4x4_50),
            "4x4_100" => Ok(Self::Dict4x4_100),
            "4x4_250" => Ok(Self::Dict4x4_250),
            "4x4_1000" => Ok(Self::Dict4x4_1000),
            other => Err(Error::aruco(format!(
                "unknown dictionary `{other}`; expected one of 4x4_50, 4x4_100, 4x4_250, 4x4_1000"
            ))),
        }
    }
}
