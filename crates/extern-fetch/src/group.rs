use crate::{ExternFetchError, ExternFetchResult, host_os::HostOs, platform::Platform};

use serde::Deserialize;
use std::collections::BTreeMap;

/// One third-party dependency family (`occt`, `opencv`, ...): its release
/// anchor (repo + tag) and the per-OS artifact tables.
#[derive(Debug, Deserialize)]
pub(crate) struct Group {
    repo: String,
    tag: String,
    #[serde(flatten)]
    platforms: BTreeMap<String, Platform>,
}

impl Group {
    pub(crate) fn repo(&self) -> &str {
        &self.repo
    }

    pub(crate) fn tag(&self) -> &str {
        &self.tag
    }

    pub(crate) fn section(&self, os: HostOs) -> Option<&Platform> {
        self.platforms.get(os.section_key())
    }

    pub(crate) fn validate(&self, name: &str) -> ExternFetchResult<()> {
        if self.repo.is_empty() {
            return Err(ExternFetchError::config(format!(
                "[{name}] repo must not be empty"
            )));
        }
        if self.tag.is_empty() {
            return Err(ExternFetchError::config(format!(
                "[{name}] tag must not be empty"
            )));
        }
        for (os_key, platform) in &self.platforms {
            if !HostOs::ALL.contains(&os_key.as_str()) {
                return Err(ExternFetchError::config(format!(
                    "[{name}.{os_key}] is not a known OS section (expected one of windows, linux, macos)"
                )));
            }
            for (role, artifact) in platform.artifacts() {
                artifact.validate(&format!("{name}.{os_key}.{role}"), &self.repo, &self.tag)?;
            }
        }
        Ok(())
    }
}
