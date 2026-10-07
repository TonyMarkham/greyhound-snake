use crate::{
    OcctError, OcctResult, grey_box::GreyBbox, native_api::NativeApi, step_info::StepInfo,
};

use std::{
    ffi::{CString, c_void},
    os::unix::ffi::OsStrExt,
    path::Path,
    ptr::NonNull,
    rc::Rc,
};

pub struct StepDoc {
    handle: NonNull<c_void>,
    api: Rc<NativeApi>,
}

impl StepDoc {
    pub(crate) fn open(api: Rc<NativeApi>, path: &Path) -> OcctResult<Self> {
        let path_string = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| OcctError::step("path contains interior NUL"))?;
        // SAFETY: path is NUL-terminated for the call; api owns the function.
        let raw = unsafe { (api.step_open)(path_string.as_ptr()) };
        let Some(handle) = NonNull::new(raw) else {
            return Err(api.native_error(&format!("STEP open {}", path.display())));
        };
        // No fallible work after ownership of the native handle transfers.
        Ok(Self { handle, api })
    }

    pub fn info(&self) -> OcctResult<StepInfo> {
        let (mut solids, mut faces, mut edges) = (0, 0, 0);
        let mut bbox = GreyBbox {
            min: [0.0; 3],
            max: [0.0; 3],
        };
        // SAFETY: valid owned handle; all outputs are live, distinct locals
        // matching the ABI header, including the repr(C) bounding box.
        let status = unsafe {
            (self.api.step_info)(
                self.handle.as_ptr(),
                &mut solids,
                &mut faces,
                &mut edges,
                &mut bbox,
            )
        };
        if status != 0 {
            return Err(self.api.native_error("STEP info"));
        }
        Ok(StepInfo {
            solids,
            faces,
            edges,
            bbox,
        })
    }

    pub fn mesh(&self, deflection: f64, angle_rad: f64) -> OcctResult<(Vec<f32>, Vec<u32>)> {
        let (mut nverts, mut nindices) = (0, 0);
        // SAFETY: valid handle and live count outputs; the operation finishes
        // meshing synchronously before returning the buffer sizes.
        let status = unsafe {
            (self.api.mesh_counts)(
                self.handle.as_ptr(),
                deflection,
                angle_rad,
                &mut nverts,
                &mut nindices,
            )
        };
        if status != 0 {
            return Err(self.api.native_error("mesh counts"));
        }
        let vertex_len = usize::try_from(nverts)
            .ok()
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(|| OcctError::step("vertex buffer length overflow"))?;
        let index_len = usize::try_from(nindices)
            .map_err(|_| OcctError::step("index buffer length overflow"))?;
        let mut verts = Vec::new();
        verts
            .try_reserve_exact(vertex_len)
            .map_err(|error| OcctError::step(format!("vertex allocation: {error}")))?;
        verts.resize(vertex_len, 0.0f32);
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(index_len)
            .map_err(|error| OcctError::step(format!("index allocation: {error}")))?;
        indices.resize(index_len, 0u32);
        // SAFETY: buffers match the counts for this same document. No native
        // operation mutates it between counts and fill; ownership stays Rust's.
        let status = unsafe {
            (self.api.mesh_fill)(
                self.handle.as_ptr(),
                verts.as_mut_ptr(),
                indices.as_mut_ptr(),
            )
        };
        if status != 0 {
            return Err(self.api.native_error("mesh fill"));
        }
        Ok((verts, indices))
    }
}

impl Drop for StepDoc {
    fn drop(&mut self) {
        // SAFETY: this is the sole owner of the native document. Its Rc is
        // still alive during Drop, and the ABI close operation is nonthrowing.
        unsafe { (self.api.step_close)(self.handle.as_ptr()) }
    }
}
