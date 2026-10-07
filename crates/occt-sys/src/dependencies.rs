use crate::{OcctError, OcctResult};
use goblin::elf::Elf;
use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
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

    let elf = Elf::parse(&bytes).map_err(|error| {
        OcctError::load(
            directory,
            shim,
            format!("invalid ELF file {}: {error}", file.display()),
        )
    })?;

    for name in elf.libraries {
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

        let candidate = directory.join(name);

        // TK libraries must come from the configured OCCT directory.
        // Other dependencies located there are also loaded explicitly.
        // Remaining system libraries are resolved normally by Linux.
        if !name.starts_with("libTK") && !candidate.is_file() {
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

        // SAFETY: these are the configured OCCT runtime libraries.
        // Their dependencies have been loaded first, and their handles
        // remain owned until after the shim is released.
        let library =
            unsafe { Library::open(Some(&path), RTLD_NOW | RTLD_LOCAL) }.map_err(|error| {
                OcctError::load(
                    directory,
                    shim,
                    format!("cannot load {}: {error}", path.display()),
                )
            })?;

        active.remove(&path);
        seen.insert(path);
        loaded.push(library);
    }

    Ok(())
}
