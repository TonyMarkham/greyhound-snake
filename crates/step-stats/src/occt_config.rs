use serde::Deserialize;
use std::path::PathBuf;

/// The `[occt]` table: the host-neutral shim directory plus per-OS
/// library directories. `library_dir` lives in a per-OS sub-table
/// because the dev trees' runtime libraries sit in different
/// subdirectories (`bin/` on Windows, `lib/` elsewhere) — the section
/// matching the build target supplies it, and a missing section is the
/// platform gate, the same rule crates/occt-sys/config.toml applies.
/// The deserialized `library_dir` field is the selected and resolved
/// one; the TOML never names it directly at the `[occt]` level.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcctConfig {
    #[serde(default)]
    linux: Option<PlatformOcct>,
    #[serde(default)]
    windows: Option<PlatformOcct>,
    #[serde(default)]
    macos: Option<PlatformOcct>,
    #[serde(skip)]
    pub library_dir: PathBuf,
    pub shim_dir: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformOcct {
    library_dir: PathBuf,
}

impl OcctConfig {
    /// The build target's section key, for error messages.
    pub(crate) fn section_key() -> &'static str {
        if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        }
    }

    /// The build target's `library_dir`, before resolution. None means
    /// the config carries no section for this OS — the platform gate.
    pub(crate) fn selected_library_dir(&self) -> Option<&PathBuf> {
        if cfg!(target_os = "windows") {
            self.windows.as_ref().map(|platform| &platform.library_dir)
        } else if cfg!(target_os = "macos") {
            self.macos.as_ref().map(|platform| &platform.library_dir)
        } else {
            self.linux.as_ref().map(|platform| &platform.library_dir)
        }
    }
}
