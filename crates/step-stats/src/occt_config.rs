use serde::Deserialize;
use std::path::PathBuf;

/// The `[occt]` table: per-OS shim and library directories. Both live
/// in a per-OS sub-table: `library_dir` because the dev trees' runtime
/// libraries sit in different subdirectories (`bin/` on Windows,
/// `lib/` elsewhere), and `shim_dir` because the package plugin
/// directories carry the platform's plugin architecture in their name
/// (`arm64` on macOS, `x86_64` elsewhere) — the section matching the
/// build target supplies both, and a missing section is the platform
/// gate, the same rule crates/occt-sys/config.toml applies. The
/// deserialized `library_dir` and `shim_dir` fields are the selected
/// and resolved ones; the TOML never names them at the `[occt]` level.
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
    #[serde(skip)]
    pub shim_dir: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformOcct {
    library_dir: PathBuf,
    shim_dir: PathBuf,
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

    /// The build target's `shim_dir`, before resolution; the same
    /// platform gate as `library_dir`.
    pub(crate) fn selected_shim_dir(&self) -> Option<&PathBuf> {
        if cfg!(target_os = "windows") {
            self.windows.as_ref().map(|platform| &platform.shim_dir)
        } else if cfg!(target_os = "macos") {
            self.macos.as_ref().map(|platform| &platform.shim_dir)
        } else {
            self.linux.as_ref().map(|platform| &platform.shim_dir)
        }
    }
}
