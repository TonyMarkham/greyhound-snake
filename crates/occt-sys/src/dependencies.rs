use crate::{OcctError, OcctResult, native_library::Library};

#[cfg(target_os = "linux")]
use goblin::elf::Elf;
#[cfg(target_os = "macos")]
use goblin::mach::MachO;
#[cfg(target_os = "windows")]
use goblin::pe::PE;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn load(shim: &Path, directory: &Path) -> OcctResult<Vec<Library>> {
    let mut loaded = Vec::new();
    let mut seen = HashSet::new();
    let mut active = HashSet::new();

    let result = visit(shim, directory, shim, &mut seen, &mut active, &mut loaded);

    // Dependencies were loaded children first. Release parents first.
    loaded.reverse();
    result?;
    Ok(loaded)
}

fn visit(
    file: &Path,
    directory: &Path,
    shim: &Path,
    seen: &mut HashSet<PathBuf>,
    active: &mut HashSet<PathBuf>,
    loaded: &mut Vec<Library>,
) -> OcctResult<()> {
    let bytes = fs::read(file).map_err(|error| {
        OcctError::load(
            directory,
            shim,
            format!(
                "cannot read dependency metadata {}: {error}",
                file.display()
            ),
        )
    })?;

    for name in dependency_names(&bytes, file, directory, shim)? {
        if name.contains('/') {
            return Err(OcctError::load(
                directory,
                shim,
                format!(
                    "{} contains a path-based dependency: {name}",
                    file.display(),
                ),
            ));
        }

        let candidate = directory.join(&name);

        // TK libraries must come from the configured OCCT directory.
        // Other dependencies located there are also loaded explicitly.
        // Remaining system libraries are resolved normally by the platform.
        if !name.starts_with(toolkit_prefix()) && !candidate.is_file() {
            continue;
        }

        let path = candidate.canonicalize().map_err(|error| {
            OcctError::load(
                directory,
                shim,
                format!(
                    "{} requires {}; cannot locate it: {error}",
                    file.display(),
                    candidate.display(),
                ),
            )
        })?;

        if seen.contains(&path) {
            continue;
        }

        if !active.insert(path.clone()) {
            return Err(OcctError::load(
                directory,
                shim,
                format!("cyclic native dependency at {}", path.display()),
            ));
        }

        visit(&path, directory, shim, seen, active, loaded)?;

        // These are the configured OCCT runtime libraries; their
        // dependencies have been loaded first, and their handles remain
        // owned until after the shim is released.
        let library = Library::open(&path, directory, shim)?;

        active.remove(&path);
        seen.insert(path);
        loaded.push(library);
    }

    Ok(())
}

// The library names the file's dependency chain reaches for, resolved
// against `directory` by the caller.
#[cfg(target_os = "linux")]
fn dependency_names(
    bytes: &[u8],
    file: &Path,
    directory: &Path,
    shim: &Path,
) -> OcctResult<Vec<String>> {
    // ELF sonames (DT_NEEDED entries).
    let elf = Elf::parse(bytes).map_err(|error| {
        OcctError::load(
            directory,
            shim,
            format!("invalid ELF file {}: {error}", file.display()),
        )
    })?;
    Ok(elf.libraries.into_iter().map(String::from).collect())
}

// Mach-O dependency install names (LC_LOAD_DYLIB and its load/reexport/
// weak/lazy relatives). Install names are path-based by convention —
// @rpath/libTKernel.dylib for the OCCT toolkits, /usr/lib/libSystem.B.dylib
// for system libraries — while the walk resolves bare filenames against
// `directory`, so keep only the final path component. libs[0] carries the
// file's own install name (LC_ID_DYLIB), or goblin's "self" placeholder
// when absent: identification, not a dependency. The rest deduplicate
// through `seen`.
#[cfg(target_os = "macos")]
fn dependency_names(
    bytes: &[u8],
    file: &Path,
    directory: &Path,
    shim: &Path,
) -> OcctResult<Vec<String>> {
    let macho = MachO::parse(bytes, 0).map_err(|error| {
        OcctError::load(
            directory,
            shim,
            format!("invalid Mach-O file {}: {error}", file.display()),
        )
    })?;
    Ok(macho
        .libs
        .into_iter()
        .skip(1)
        .map(|name| match name.rsplit_once('/') {
            Some((_, base)) => base,
            None => name,
        })
        .map(String::from)
        .collect())
}

#[cfg(target_os = "windows")]
fn dependency_names(
    bytes: &[u8],
    file: &Path,
    directory: &Path,
    shim: &Path,
) -> OcctResult<Vec<String>> {
    // PE import-table DLL names, one entry per imported symbol; the walk
    // deduplicates through `seen`.
    let pe = PE::parse(bytes).map_err(|error| {
        OcctError::load(
            directory,
            shim,
            format!("invalid PE file {}: {error}", file.display()),
        )
    })?;
    Ok(pe
        .imports
        .iter()
        .map(|import| import.dll.to_owned())
        .collect())
}

// The filename prefix every OCCT toolkit library carries on this platform
// (libTKernel.so on Linux, libTKernel.dylib on macOS, TKernel.dll on
// Windows). On macOS the install names were reduced to bare filenames
// upstream, so the prefix check sees the same shape as Linux's sonames.
#[cfg(target_os = "linux")]
fn toolkit_prefix() -> &'static str {
    "libTK"
}

#[cfg(target_os = "macos")]
fn toolkit_prefix() -> &'static str {
    "libTK"
}

#[cfg(target_os = "windows")]
fn toolkit_prefix() -> &'static str {
    "TK"
}
