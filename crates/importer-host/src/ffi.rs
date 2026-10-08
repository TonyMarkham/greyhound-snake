use crate::{
    HostError, HostMeshCounts, HostResult, doc::Doc, guard::guard, host::Host, last_error,
};

use unity_projection::{UnitySubMesh, UnityVertex};

use std::{
    ffi::{CStr, OsStr, c_char},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

const HOST_ABI_VERSION: u32 = 2;

fn path_from(pointer: *const c_char, what: &str) -> HostResult<PathBuf> {
    if pointer.is_null() {
        return Err(HostError::host(format!("{what} is null")));
    }
    // SAFETY: the caller passes a NUL-terminated C string per the ABI.
    let text = unsafe { CStr::from_ptr(pointer) };
    let bytes = text.to_bytes();
    if bytes.is_empty() {
        return Err(HostError::host(format!("{what} is empty")));
    }
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_version() -> u32 {
    HOST_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_last_error() -> *const c_char {
    last_error::last_error()
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_new(
    occt_dir: *const c_char,
    shim_path: *const c_char,
) -> *mut Host {
    guard(std::ptr::null_mut(), || {
        last_error::clear_error();
        let result = path_from(occt_dir, "OCCT directory")
            .and_then(|occt_dir| {
                path_from(shim_path, "shim path").map(|shim_path| (occt_dir, shim_path))
            })
            .and_then(|(occt_dir, shim_path)| {
                Host::new(&occt_dir, &shim_path)
                    .map_err(|failure| HostError::host(failure.to_string()))
            });
        match result {
            Ok(host) => Box::into_raw(Box::new(host)),
            Err(failure) => {
                last_error::set_error(failure.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_free(host: *mut Host) {
    guard((), || {
        if !host.is_null() {
            // SAFETY: created by greyhound_host_new and freed exactly once here.
            drop(unsafe { Box::from_raw(host) });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_open_step(host: *mut Host, path: *const c_char) -> *mut Doc {
    guard(std::ptr::null_mut(), || {
        last_error::clear_error();
        if host.is_null() {
            last_error::set_error("host handle is null");
            return std::ptr::null_mut();
        }
        let result = path_from(path, "STEP path").and_then(|path| {
            // SAFETY: the host was created by greyhound_host_new and stays
            // alive for the duration of this call.
            let host = unsafe { &*host };
            Doc::open(host, &path).map_err(|failure| HostError::host(failure.to_string()))
        });
        match result {
            Ok(doc) => Box::into_raw(Box::new(doc)),
            Err(failure) => {
                last_error::set_error(failure.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_close_step(doc: *mut Doc) {
    guard((), || {
        if !doc.is_null() {
            // SAFETY: created by greyhound_host_open_step and freed exactly
            // once here.
            drop(unsafe { Box::from_raw(doc) });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_mesh_counts(
    doc: *mut Doc,
    deflection: f64,
    angle_rad: f64,
    scale: f64,
    counts: *mut HostMeshCounts,
) -> i32 {
    guard(1, || {
        last_error::clear_error();
        if doc.is_null() || counts.is_null() {
            last_error::set_error("mesh counts handle or output pointer is null");
            return 1;
        }
        // SAFETY: the document was created by greyhound_host_open_step and
        // stays alive for the duration of this call; this call takes the
        // counts-only mutable access.
        let result = unsafe { (*doc).counts(deflection, angle_rad, scale) };
        match result {
            Ok(value) => {
                // SAFETY: counts is a live, caller-provided output pointer.
                unsafe { *counts = value };
                0
            }
            Err(failure) => {
                last_error::set_error(failure.to_string());
                1
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn greyhound_host_mesh_fill(
    doc: *mut Doc,
    verts: *mut UnityVertex,
    indices: *mut u32,
    submeshes: *mut UnitySubMesh,
    submesh_colors: *mut u32,
    colors: *mut [f32; 4],
) -> i32 {
    guard(1, || {
        last_error::clear_error();
        if doc.is_null()
            || verts.is_null()
            || indices.is_null()
            || submeshes.is_null()
            || submesh_colors.is_null()
            || colors.is_null()
        {
            last_error::set_error("mesh fill handle or output pointer is null");
            return 1;
        }
        // SAFETY: the document was created by greyhound_host_open_step and
        // stays alive for the duration of this call; this call takes the
        // fill-only mutable access.
        let result = unsafe { (*doc).fill(verts, indices, submeshes, submesh_colors, colors) };
        match result {
            Ok(()) => 0,
            Err(failure) => {
                last_error::set_error(failure.to_string());
                1
            }
        }
    })
}
