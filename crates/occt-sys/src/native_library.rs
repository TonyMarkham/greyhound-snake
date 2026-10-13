use crate::{OcctError, OcctResult};
use std::path::Path;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("occt-sys runtime loader supports linux and windows");

// The loader facade every explicit open goes through — the shim itself and
// every preloaded dependency. Platform differences live here and nowhere
// else: Linux opens with RTLD_NOW | RTLD_LOCAL; Windows opens with
// LOAD_WITH_ALTERED_SEARCH_PATH, where the absolute paths plus the
// explicit children-first dependency preloads replace the search-path
// machinery the dlopen flags provide.
pub(crate) struct Library {
    #[cfg(target_os = "linux")]
    inner: libloading::os::unix::Library,
    #[cfg(target_os = "windows")]
    inner: libloading::os::windows::Library,
}

impl Library {
    pub(crate) fn open(path: &Path, directory: &Path, shim: &Path) -> OcctResult<Self> {
        #[cfg(target_os = "linux")]
        let inner = unsafe {
            libloading::os::unix::Library::open(
                Some(path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .map_err(|error| OcctError::load(directory, shim, error.to_string()))?;
        #[cfg(target_os = "windows")]
        let inner = unsafe {
            libloading::os::windows::Library::load_with_flags(
                path,
                libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        }
        .map_err(|error| OcctError::load(directory, shim, error.to_string()))?;
        Ok(Self { inner })
    }

    // SAFETY (per call site): callers supply the exact ABI function-pointer
    // type. The copied pointer stays private and is stored with this
    // owning Library.
    pub(crate) fn get<T: Copy>(&self, name: &[u8], shim: &Path) -> OcctResult<T> {
        let value = unsafe { self.inner.get::<T>(name) }
            .map_err(|error| OcctError::symbol(shim, name, error.to_string()))?;
        Ok(*value)
    }
}
