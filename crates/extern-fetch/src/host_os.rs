use crate::{ExternFetchError, ExternFetchResult};

use std::env;

/// The host operating system, selecting which per-OS section of a group
/// applies. Mirrors the occt-sys gating convention: a config group with no
/// section for this OS is the platform gate — nothing was written for it,
/// so the run fails for that group instead of silently doing nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostOs {
    Windows,
    Linux,
    MacOs,
}

impl HostOs {
    /// Every section key a group may carry. Used by validation so a
    /// typo'd section (`[occt.windwos]`) fails loudly at config load
    /// instead of never matching any OS.
    pub(crate) const ALL: [&'static str; 3] = ["windows", "linux", "macos"];

    pub(crate) fn current() -> ExternFetchResult<Self> {
        match env::consts::OS {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOs),
            other => Err(ExternFetchError::config(format!(
                "unsupported host OS {other:?}: no config sections exist for it"
            ))),
        }
    }

    pub(crate) fn section_key(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::MacOs => "macos",
        }
    }
}
