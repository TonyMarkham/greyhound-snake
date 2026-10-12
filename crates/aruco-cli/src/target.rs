use aruco::ArucoError;

use std::str::FromStr;

/// The positional argument: a marker id or the `all` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Id(u32),
    All,
}

impl FromStr for Target {
    type Err = ArucoError;

    fn from_str(text: &str) -> Result<Self, ArucoError> {
        if text == "all" {
            return Ok(Target::All);
        }
        match text.parse::<u32>() {
            Ok(id) => Ok(Target::Id(id)),
            Err(_) => Err(ArucoError::aruco(format!(
                "expected a marker id or `all`, got `{text}`"
            ))),
        }
    }
}
