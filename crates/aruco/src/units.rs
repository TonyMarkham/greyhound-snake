use crate::error::{Error, Result};

/// Physical unit for the emitted SVG's `width`/`height` attributes.
///
/// The viewBox always stays in cells, so the unit choice never
/// affects the geometry — only how viewers map the vector content to
/// physical size. `px` emits unit-less numbers (CSS pixels); `mm`
/// emits a `mm` suffix (print pipelines).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Units {
    #[default]
    Px,
    Mm,
}

impl Units {
    /// The suffix appended to `width`/`height` values (empty for px).
    pub fn suffix(self) -> &'static str {
        match self {
            Units::Px => "",
            Units::Mm => "mm",
        }
    }
}

impl std::str::FromStr for Units {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        match text {
            "px" => Ok(Units::Px),
            "mm" => Ok(Units::Mm),
            other => Err(Error::aruco(format!(
                "unknown unit `{other}`; expected `px` or `mm`"
            ))),
        }
    }
}
