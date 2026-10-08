use crate::{OcctError, OcctResult, grey_box::GreyBbox};

use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
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
    pub(crate) fn load(directory: &Path, shim_path: &Path) -> OcctResult<Self> {
        let directory = directory
            .canonicalize()
            .map_err(|error| OcctError::load(directory, shim_path, error.to_string()))?;

        let shim = shim_path.canonicalize().map_err(|error| {
            OcctError::load(
                &directory,
                shim_path,
                format!("cannot locate shim: {error}"),
            )
        })?;

        let kernel = directory.join("libTKernel.so");
        if !kernel.is_file() {
            return Err(OcctError::load(
                &directory,
                &shim,
                format!("missing OCCT library {}", kernel.display()),
            ));
        }

        let dependencies = crate::dependencies::load(&shim, &directory)?;

        // SAFETY: load our compatible bridge after its OCCT dependencies.
        // Both the bridge and dependency handles remain owned by NativeApi.
        let library = unsafe { Library::open(Some(&shim), RTLD_NOW | RTLD_LOCAL) }
            .map_err(|error| OcctError::load(&directory, &shim, error.to_string()))?;

        // SAFETY: the version probe's signature is fixed by our C ABI.
        let version: unsafe extern "C" fn() -> u32 =
            unsafe { symbol(&library, &shim, b"greyhound_abi_version\0")? };

        // SAFETY: library owns the resolved probe.
        let actual = unsafe { version() };
        if actual != 5 {
            return Err(OcctError::abi(&shim, 5, actual));
        }

        // SAFETY: these signatures match ABI version 5. Nothing is published
        // until every lookup succeeds and the owning handles are stored.
        unsafe {
            Ok(Self {
                box_volume: symbol(&library, &shim, b"greyhound_box_volume\0")?,
                step_open: symbol(&library, &shim, b"greyhound_step_open\0")?,
                step_info: symbol(&library, &shim, b"greyhound_step_info\0")?,
                scene_counts: symbol(&library, &shim, b"greyhound_scene_counts\0")?,
                scene_fill: symbol(&library, &shim, b"greyhound_scene_fill\0")?,
                color_fill: symbol(&library, &shim, b"greyhound_color_fill\0")?,
                mesh_counts: symbol(&library, &shim, b"greyhound_mesh_counts\0")?,
                mesh_fill: symbol(&library, &shim, b"greyhound_mesh_fill\0")?,
                step_close: symbol(&library, &shim, b"greyhound_step_close\0")?,
                last_error: symbol(&library, &shim, b"greyhound_last_error\0")?,
                _library: library,
                _dependencies: dependencies,
            })
        }
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

unsafe fn symbol<T: Copy>(library: &Library, shim: &Path, name: &[u8]) -> OcctResult<T> {
    // SAFETY: callers supply the exact ABI function-pointer type. The copied
    // pointer remains private and is stored with this owning Library.
    let value = unsafe { library.get::<T>(name) }
        .map_err(|error| OcctError::symbol(shim, name, error.to_string()))?;
    Ok(*value)
}
