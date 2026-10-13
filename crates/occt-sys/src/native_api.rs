use crate::{OcctError, OcctResult, grey_box::GreyBbox, native_library::Library};

use std::{
    ffi::{CStr, c_char, c_void},
    path::Path,
};

pub(crate) struct NativeApi {
    pub(crate) box_volume: unsafe extern "C" fn(f64, f64, f64, *mut f64) -> i32,
    pub(crate) step_open: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    pub(crate) step_info:
        unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32, *mut i32, *mut GreyBbox) -> i32,
    pub(crate) scene_counts:
        unsafe extern "C" fn(*mut c_void, *mut u32, *mut u32, *mut u32, *mut u32) -> i32,
    pub(crate) scene_fill:
        unsafe extern "C" fn(*mut c_void, *mut u32, *mut f32, *mut c_char) -> i32,
    pub(crate) color_fill: unsafe extern "C" fn(*mut c_void, *mut f32) -> i32,
    pub(crate) mesh_properties: unsafe extern "C" fn(*mut c_void, u32, f64, *mut f64) -> i32,
    pub(crate) mesh_counts:
        unsafe extern "C" fn(*mut c_void, u32, f64, f64, *mut u32, *mut u32, *mut u32) -> i32,
    pub(crate) mesh_fill: unsafe extern "C" fn(
        *mut c_void,
        u32,
        *mut f32,
        *mut f32,
        *mut u32,
        *mut u32,
        *mut u32,
    ) -> i32,
    pub(crate) step_close: unsafe extern "C" fn(*mut c_void),
    last_error: unsafe extern "C" fn() -> *const c_char,
    _library: Library,
    _dependencies: Vec<Library>,
}

impl NativeApi {
    pub(crate) fn load(directory: &Path, shim_dir: &Path) -> OcctResult<Self> {
        let directory = directory
            .canonicalize()
            .map_err(|error| OcctError::load(directory, shim_dir, error.to_string()))?;

        // The shim's filename is the crate config's per-OS value compiled
        // into this library; callers hand over the shim directory only.
        let resolved_shim_dir = shim_dir.canonicalize().map_err(|error| {
            OcctError::load(
                &directory,
                shim_dir,
                format!("cannot locate shim directory: {error}"),
            )
        })?;
        let shim = resolved_shim_dir.join(env!("OCCT_SHIM_FILENAME"));

        // The runtime toolkit name comes from the crate config through
        // build.rs (single source of truth): GREYHOUND_RUNTIME_PATTERN
        // carries lib{tk}.so / {tk}.dll, {tk} replaced by the toolkit name.
        let kernel = directory.join(env!("OCCT_RUNTIME_PATTERN").replace("{tk}", "TKernel"));
        if !kernel.is_file() {
            return Err(OcctError::load(
                &directory,
                &shim,
                format!("missing OCCT library {}", kernel.display()),
            ));
        }

        let dependencies = crate::dependencies::load(&shim, &directory)?;

        // The bridge loads after its OCCT dependencies; both the bridge and
        // dependency handles remain owned by NativeApi.
        let library = Library::open(&shim, &directory, &shim)?;

        // The version probe's signature is fixed by our C ABI.
        let version: unsafe extern "C" fn() -> u32 =
            library.get(b"greyhound_abi_version\0", &shim)?;

        // SAFETY: library owns the resolved probe.
        let actual = unsafe { version() };
        if actual != 6 {
            return Err(OcctError::abi(&shim, 6, actual));
        }

        // The signatures match ABI version 6. Nothing is published until
        // every lookup succeeds and the owning handles are stored.
        Ok(Self {
            box_volume: library.get(b"greyhound_box_volume\0", &shim)?,
            step_open: library.get(b"greyhound_step_open\0", &shim)?,
            step_info: library.get(b"greyhound_step_info\0", &shim)?,
            scene_counts: library.get(b"greyhound_scene_counts\0", &shim)?,
            scene_fill: library.get(b"greyhound_scene_fill\0", &shim)?,
            color_fill: library.get(b"greyhound_color_fill\0", &shim)?,
            mesh_properties: library.get(b"greyhound_mesh_properties\0", &shim)?,
            mesh_counts: library.get(b"greyhound_mesh_counts\0", &shim)?,
            mesh_fill: library.get(b"greyhound_mesh_fill\0", &shim)?,
            step_close: library.get(b"greyhound_step_close\0", &shim)?,
            last_error: library.get(b"greyhound_last_error\0", &shim)?,
            _library: library,
            _dependencies: dependencies,
        })
    }

    pub(crate) fn native_error(&self, operation: &str) -> OcctError {
        // SAFETY: this ABI returns null or a NUL-terminated thread-local
        // message; self keeps its Library alive while the message is copied.
        let pointer = unsafe { (self.last_error)() };
        let message = if pointer.is_null() {
            "native operation failed".to_owned()
        } else {
            // SAFETY: same ABI/lifetime guarantee as above; no intervening call.
            unsafe { CStr::from_ptr(pointer) }
                .to_string_lossy()
                .into_owned()
        };
        OcctError::step(format!("{operation}: {message}"))
    }
}
