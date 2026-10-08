use crate::{
    OcctError, OcctResult, grey_box::GreyBbox, native_api::NativeApi, scene::Scene,
    step_info::StepInfo,
};

use mesh::{FaceAttrib, FaceRange, Mesh, MeshBuilder, MeshProperties, Node, Scene as NodeScene};

use std::{
    ffi::{CString, c_char, c_void},
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

    /// Tessellates every unique mesh of the assembly forest and returns the
    /// node hierarchy with the mesh and color payloads. Meshes are
    /// deduplicated by the shim: repeated instances share one `Mesh`.
    pub fn scene(&self, deflection: f64, angle_rad: f64) -> OcctResult<Scene> {
        let (mut nodes, mut meshes, mut colors, mut name_bytes) = (0, 0, 0, 0);
        // SAFETY: valid handle and live count outputs; the operation walks
        // the XCAF assembly forest synchronously before returning sizes.
        let status = unsafe {
            (self.api.scene_counts)(
                self.handle.as_ptr(),
                &mut nodes,
                &mut meshes,
                &mut colors,
                &mut name_bytes,
            )
        };
        if status != 0 {
            return Err(self.api.native_error("scene counts"));
        }

        let node_len =
            usize::try_from(nodes).map_err(|_| OcctError::step("node count overflow"))?;
        let mesh_len =
            usize::try_from(meshes).map_err(|_| OcctError::step("mesh count overflow"))?;
        let color_len =
            usize::try_from(colors).map_err(|_| OcctError::step("color count overflow"))?;
        let name_len =
            usize::try_from(name_bytes).map_err(|_| OcctError::step("name length overflow"))?;

        let mut node_buffer = Vec::<u32>::new();
        node_buffer
            .try_reserve_exact(node_len * 4)
            .map_err(|error| OcctError::step(format!("node allocation: {error}")))?;
        node_buffer.resize(node_len * 4, 0);
        let mut transforms = Vec::<[f32; 12]>::new();
        transforms
            .try_reserve_exact(node_len)
            .map_err(|error| OcctError::step(format!("transform allocation: {error}")))?;
        transforms.resize(node_len, [0.0; 12]);
        let mut names = Vec::<u8>::new();
        names
            .try_reserve_exact(name_len)
            .map_err(|error| OcctError::step(format!("name allocation: {error}")))?;
        names.resize(name_len, 0);
        let mut palette = Vec::<[f32; 4]>::new();
        palette
            .try_reserve_exact(color_len)
            .map_err(|error| OcctError::step(format!("color allocation: {error}")))?;
        palette.resize(color_len, [0.0; 4]);

        // SAFETY: buffers match the counts for this same document. No native
        // operation mutates it between counts and fill; ownership stays Rust's.
        // The typed element pointers are layout-identical to their flat forms.
        let status = unsafe {
            (self.api.scene_fill)(
                self.handle.as_ptr(),
                node_buffer.as_mut_ptr(),
                transforms.as_mut_ptr().cast::<f32>(),
                names.as_mut_ptr().cast::<c_char>(),
            )
        };
        if status != 0 {
            return Err(self.api.native_error("scene fill"));
        }
        // SAFETY: same ownership guarantees as the scene_fill call above.
        let status = unsafe {
            (self.api.color_fill)(self.handle.as_ptr(), palette.as_mut_ptr().cast::<f32>())
        };
        if status != 0 {
            return Err(self.api.native_error("color fill"));
        }

        let mut forest_nodes = Vec::<Node>::with_capacity(node_len);
        for (fields, transform) in node_buffer
            .as_chunks::<4>()
            .0
            .iter()
            .zip(transforms.iter().copied())
        {
            forest_nodes.push(Node {
                parent: fields[0],
                mesh: fields[1],
                name_offset: fields[2],
                name_length: fields[3],
                transform,
            });
        }
        let forest = NodeScene::try_new(forest_nodes, meshes, names).map_err(|error| {
            OcctError::step(format!("node forest rejected by the mesh model: {error}"))
        })?;

        let mut built_meshes = Vec::<Mesh>::with_capacity(mesh_len);
        for index in 0..meshes {
            let mesh = self.mesh_at(index, deflection, angle_rad, &palette)?;
            built_meshes.push(mesh);
        }

        Ok(Scene::new(forest, built_meshes))
    }

    /// The exact BRep mass properties of one unique mesh (local frame,
    /// density-free; see `MeshProperties`).
    pub fn mesh_properties(&self, mesh: u32) -> OcctResult<MeshProperties> {
        let mut out = [0.0f64; 17];
        // SAFETY: valid handle, live flat output buffer matching the ABI
        // header's documented 17-double layout.
        let status = unsafe {
            (self.api.mesh_properties)(self.handle.as_ptr(), mesh, 0.0, out.as_mut_ptr())
        };
        if status != 0 {
            return Err(self.api.native_error("mesh properties"));
        }
        Ok(MeshProperties {
            volume_mm3: out[0],
            centre_of_gravity: [out[1], out[2], out[3]],
            principal_axes: [
                [out[4], out[5], out[6]],
                [out[7], out[8], out[9]],
                [out[10], out[11], out[12]],
            ],
            principal_moments: [out[13], out[14], out[15]],
            file_density: out[16],
        })
    }

    fn mesh_at(
        &self,
        mesh: u32,
        deflection: f64,
        angle_rad: f64,
        colors: &[[f32; 4]],
    ) -> OcctResult<Mesh> {
        let (mut nverts, mut nindices, mut nfaces) = (0, 0, 0);
        // SAFETY: valid handle and live count outputs; the operation
        // finishes meshing synchronously before returning the buffer sizes.
        let status = unsafe {
            (self.api.mesh_counts)(
                self.handle.as_ptr(),
                mesh,
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
        let attrib_len = face_len;

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
        let mut face_attribs = Vec::<u32>::new();
        face_attribs
            .try_reserve_exact(attrib_len)
            .map_err(|error| OcctError::step(format!("face attrib allocation: {error}")))?;
        face_attribs.resize(attrib_len, 0);

        // SAFETY: buffers match the counts for this same document. No native
        // operation mutates it between counts and fill; ownership stays Rust's.
        // The typed element pointers are layout-identical to their flat forms.
        let status = unsafe {
            (self.api.mesh_fill)(
                self.handle.as_ptr(),
                mesh,
                vertices.as_mut_ptr().cast::<f32>(),
                normals.as_mut_ptr().cast::<f32>(),
                triangles.as_mut_ptr().cast::<u32>(),
                face_counts.as_mut_ptr(),
                face_attribs.as_mut_ptr(),
            )
        };
        if status != 0 {
            return Err(self.api.native_error("mesh fill"));
        }

        let mut faces = Vec::with_capacity(face_len / 2);
        let mut attribs = Vec::with_capacity(face_len / 2);
        let (mut vertex_start, mut index_start) = (0u32, 0u32);
        for (pair, attrib) in face_counts
            .as_chunks::<2>()
            .0
            .iter()
            .zip(face_attribs.as_chunks::<2>().0)
        {
            let (vertex_count, index_count) = (pair[0], pair[1]);
            faces.push(FaceRange {
                vertex_start,
                vertex_count,
                index_start,
                index_count,
            });
            attribs.push(FaceAttrib {
                solid: attrib[0],
                color: attrib[1],
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
            .with_face_attribs(attribs)
            .with_colors(colors.to_vec())
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
