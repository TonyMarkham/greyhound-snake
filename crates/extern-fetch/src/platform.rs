use crate::artifact::Artifact;

use serde::Deserialize;
use std::collections::BTreeMap;

/// The artifacts of one group on one OS, keyed by role (`dev`, `libs`,
/// ...). BTreeMap keeps the run order deterministic and puts `dev` before
/// `libs` alphabetically.
#[derive(Debug, Deserialize)]
pub(crate) struct Platform {
    #[serde(flatten)]
    artifacts: BTreeMap<String, Artifact>,
}

impl Platform {
    pub(crate) fn artifacts(&self) -> impl Iterator<Item = (&String, &Artifact)> {
        self.artifacts.iter()
    }
}
