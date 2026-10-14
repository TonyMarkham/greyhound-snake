use serde::Deserialize;

use std::{fs, path::Path, path::PathBuf};

/// The root `config.toml`'s `[occt]` table, as this test helper reads
/// it: the host-neutral shim directory plus the per-OS library
/// sections. Same file, same shape the CLI crates parse through their
/// own config layer — paths live in the config, never in source.
#[derive(Deserialize)]
struct RootConfig {
    occt: RootOcct,
}

#[derive(Deserialize)]
struct RootOcct {
    #[serde(default)]
    linux: Option<PlatformSection>,
    #[serde(default)]
    windows: Option<PlatformSection>,
    #[serde(default)]
    macos: Option<PlatformSection>,
}

#[derive(Deserialize)]
struct PlatformSection {
    library_dir: PathBuf,
    shim_dir: PathBuf,
}

/// `(library_dir, shim_dir)` for this build target, resolved against
/// the repo root — the config's parent. Panics name the section or
/// directory that is missing: tests may panic.
pub(crate) fn occt_dirs(repo_root: &Path) -> (PathBuf, PathBuf) {
    let text = fs::read_to_string(repo_root.join("config.toml"))
        .unwrap_or_else(|error| panic!("cannot read root config.toml: {error}"));
    let config: RootConfig = toml::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse root config.toml: {error}"));
    let section = if cfg!(target_os = "windows") {
        config.occt.windows.as_ref()
    } else if cfg!(target_os = "macos") {
        config.occt.macos.as_ref()
    } else {
        config.occt.linux.as_ref()
    };
    let section = section.unwrap_or_else(|| {
        panic!(
            "root config.toml has no [occt.{}] section for this build target",
            section_key()
        )
    });
    // Canonicalize so a missing directory names itself: the library
    // dir means the extern/ dev tree was not fetched (`just
    // fetch-occt`), the shim dir means the package was not assembled.
    let library_dir = repo_root
        .join(&section.library_dir)
        .canonicalize()
        .unwrap_or_else(|error| {
            panic!(
                "occt.library_dir {} is missing (dev tree not fetched? `just fetch-occt`): {error}",
                section.library_dir.display()
            )
        });
    let shim_dir = repo_root
        .join(&section.shim_dir)
        .canonicalize()
        .unwrap_or_else(|error| {
            panic!(
                "occt.shim_dir {} is missing (package not assembled?): {error}",
                section.shim_dir.display()
            )
        });
    (library_dir, shim_dir)
}

fn section_key() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}
