use crate::{
    OcctError, OcctResult, grey_box::GreyBbox, native_api::NativeApi, step_info::StepInfo,
};

use mesh::{FaceRange, Mesh, MeshBuilder};

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

    pub fn mesh(&self, deflection: f64, angle_rad: f64) -> OcctResult<Mesh> {
        let (mut nverts, mut nindices, mut nfaces) = (0, 0, 0);
        // SAFETY: valid handle and live count outputs; the operation finishes
        // meshing synchronously before returning the buffer sizes.
        let status = unsafe {
            (self.api.mesh_counts)(
                self.handle.as_ptr(),
                deflection,
                angle_rad,
                &mut nverts,
                &mut nindices,
                &mut nfaces,
            )
        };
        if status != 0 {
            return Err(self.api.native_error("mesh counts"));
        }

        let vertex_len =
            usize::try_from(nverts).map_err(|_| OcctError::step("vertex count overflow"))?;
        let index_len = usize::try_from(nindices)
            .map_err(|_| OcctError::step("index buffer length overflow"))?;
        if index_len % 3 != 0 {
            return Err(OcctError::step("index count is not a multiple of 3"));
        }
        let face_len = usize::try_from(nfaces)
            .ok()
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| OcctError::step("face count buffer length overflow"))?;

        let mut vertices = Vec::<[f32; 3]>::new();
        vertices
            .try_reserve_exact(vertex_len)
            .map_err(|error| OcctError::step(format!("vertex allocation: {error}")))?;
        vertices.resize(vertex_len, [0.0; 3]);
        let mut normals = Vec::<[f32; 3]>::new();
        normals
            .try_reserve_exact(vertex_len)
            .map_err(|error| OcctError::step(format!("normal allocation: {error}")))?;
        normals.resize(vertex_len, [0.0; 3]);
        let triangle_len = index_len / 3;
        let mut triangles = Vec::<[u32; 3]>::new();
        triangles
            .try_reserve_exact(triangle_len)
            .map_err(|error| OcctError::step(format!("index allocation: {error}")))?;
        triangles.resize(triangle_len, [0; 3]);
        let mut face_counts = Vec::<u32>::new();
        face_counts
            .try_reserve_exact(face_len)
            .map_err(|error| OcctError::step(format!("face count allocation: {error}")))?;
        face_counts.resize(face_len, 0);

        // SAFETY: buffers match the counts for this same document. No native
        // operation mutates it between counts and fill; ownership stays Rust's.
        // The typed element pointers are layout-identical to their flat forms.
        let status = unsafe {
            (self.api.mesh_fill)(
                self.handle.as_ptr(),
                vertices.as_mut_ptr().cast::<f32>(),
                normals.as_mut_ptr().cast::<f32>(),
                triangles.as_mut_ptr().cast::<u32>(),
                face_counts.as_mut_ptr(),
            )
        };
        if status != 0 {
            return Err(self.api.native_error("mesh fill"));
        }

        let mut faces = Vec::with_capacity(face_len / 2);
        let (mut vertex_start, mut index_start) = (0u32, 0u32);
        for pair in face_counts.as_chunks::<2>().0 {
            let (vertex_count, index_count) = (pair[0], pair[1]);
            faces.push(FaceRange {
                vertex_start,
                vertex_count,
                index_start,
                index_count,
            });
            vertex_start = vertex_start
                .checked_add(vertex_count)
                .ok_or_else(|| OcctError::step("face range vertex offset overflow"))?;
            index_start = index_start
                .checked_add(index_count)
                .ok_or_else(|| OcctError::step("face range index offset overflow"))?;
        }

        MeshBuilder::default()
            .with_vertices(vertices)
            .with_normals(normals)
            .with_triangles(triangles)
            .with_faces(faces)
            .build()
            .map_err(|error| OcctError::step(format!("mesh build: {error}")))
    }
}

impl Drop for StepDoc {
    fn drop(&mut self) {
        // SAFETY: this is the sole owner of the native document. Its Rc is
        // still alive during Drop, and the ABI close operation is nonthrowing.
        unsafe { (self.api.step_close)(self.handle.as_ptr()) }
    }
}
