use crate::{FaceAttrib, FaceRange, Mesh, MeshError, MeshResult};

pub(crate) type Parts = (
    Vec<[f32; 3]>,
    Vec<[u32; 3]>,
    Vec<FaceRange>,
    Option<Vec<FaceAttrib>>,
    Option<Vec<[f32; 4]>>,
    Option<Vec<[f32; 2]>>,
    Option<Vec<[f32; 3]>>,
);

#[derive(Default)]
#[must_use = "Call .build() or continue chaining setters; dropping the builder does nothing."]
pub struct Builder {
    vertices: Option<Vec<[f32; 3]>>,
    triangles: Option<Vec<[u32; 3]>>,
    faces: Option<Vec<FaceRange>>,
    face_attribs: Option<Vec<FaceAttrib>>,
    colors: Option<Vec<[f32; 4]>>,
    uvs: Option<Vec<[f32; 2]>>,
    normals: Option<Vec<[f32; 3]>>,
}

impl Builder {
    pub fn with_vertices(mut self, vertices: Vec<[f32; 3]>) -> Self {
        self.vertices = Some(vertices);
        self
    }

    pub fn with_triangles(mut self, triangles: Vec<[u32; 3]>) -> Self {
        self.triangles = Some(triangles);
        self
    }

    pub fn with_faces(mut self, faces: Vec<FaceRange>) -> Self {
        self.faces = Some(faces);
        self
    }

    pub fn with_face_attribs(mut self, face_attribs: Vec<FaceAttrib>) -> Self {
        self.face_attribs = Some(face_attribs);
        self
    }

    pub fn with_colors(mut self, colors: Vec<[f32; 4]>) -> Self {
        self.colors = Some(colors);
        self
    }

    pub fn with_uvs(mut self, uvs: Vec<[f32; 2]>) -> Self {
        self.uvs = Some(uvs);
        self
    }

    pub fn with_normals(mut self, normals: Vec<[f32; 3]>) -> Self {
        self.normals = Some(normals);
        self
    }

    pub(crate) fn into_parts(self) -> MeshResult<Parts> {
        let vertices = validate_vertices(self.vertices)?;
        let triangles = validate_triangles(self.triangles)?;
        let faces = validate_faces(self.faces, vertices.len(), triangles.len() * 3)?;

        validate_face_attribs(&self.face_attribs, &self.colors, faces.len())?;
        validate_uvs(&self.uvs, vertices.len())?;
        validate_normals(&self.normals, vertices.len())?;
        validate_indices(&triangles, vertices.len())?;

        Ok((
            vertices,
            triangles,
            faces,
            self.face_attribs,
            self.colors,
            self.uvs,
            self.normals,
        ))
    }

    pub fn build(self) -> MeshResult<Mesh> {
        Mesh::try_from(self)
    }
}

fn validate_vertices(vertices: Option<Vec<[f32; 3]>>) -> MeshResult<Vec<[f32; 3]>> {
    let vertices = vertices.ok_or_else(|| MeshError::mesh("no vertices defined"))?;
    if vertices.len() < 3 {
        return Err(MeshError::mesh("a mesh needs at least 3 vertices"));
    }

    Ok(vertices)
}

fn validate_triangles(triangles: Option<Vec<[u32; 3]>>) -> MeshResult<Vec<[u32; 3]>> {
    let triangles = triangles.ok_or_else(|| MeshError::mesh("no triangles defined"))?;
    if triangles.is_empty() {
        return Err(MeshError::mesh("a mesh needs at least 1 triangle"));
    }

    Ok(triangles)
}

fn validate_faces(
    faces: Option<Vec<FaceRange>>,
    total_vertices: usize,
    total_indices: usize,
) -> MeshResult<Vec<FaceRange>> {
    let faces = faces.ok_or_else(|| MeshError::mesh("no face ranges defined"))?;
    let total_vertices = u64::from(
        u32::try_from(total_vertices)
            .map_err(|_| MeshError::mesh("vertex count exceeds the u32 index range"))?,
    );
    let total_indices = u64::from(
        u32::try_from(total_indices)
            .map_err(|_| MeshError::mesh("index count exceeds the u32 index range"))?,
    );

    let mut vertex_cursor = 0u64;
    let mut index_cursor = 0u64;

    for (position, face) in faces.iter().enumerate() {
        let vertex_start = u64::from(face.vertex_start);
        let vertex_end = vertex_start + u64::from(face.vertex_count);
        let index_start = u64::from(face.index_start);
        let index_end = index_start + u64::from(face.index_count);

        if face.index_count % 3 != 0 {
            let count = face.index_count;
            return Err(MeshError::mesh(format!(
                "face {position} index count {count} is not a multiple of 3"
            )));
        }
        if vertex_end > total_vertices {
            return Err(MeshError::mesh(format!(
                "face {position} vertices [{}, {}) exceed {} vertices",
                face.vertex_start, vertex_end, total_vertices
            )));
        }
        if index_end > total_indices {
            return Err(MeshError::mesh(format!(
                "face {position} indices [{}, {}) exceed {} indices",
                face.index_start, index_end, total_indices
            )));
        }
        if vertex_start != vertex_cursor {
            return Err(MeshError::mesh(format!(
                "face {position} starts at vertex {vertex_start} but {vertex_cursor} was expected"
            )));
        }
        if index_start != index_cursor {
            return Err(MeshError::mesh(format!(
                "face {position} starts at index {index_start} but {index_cursor} was expected"
            )));
        }

        vertex_cursor = vertex_end;
        index_cursor = index_end;
    }

    if vertex_cursor != total_vertices {
        return Err(MeshError::mesh(format!(
            "face ranges cover {vertex_cursor} of {total_vertices} vertices"
        )));
    }
    if index_cursor != total_indices {
        return Err(MeshError::mesh(format!(
            "face ranges cover {index_cursor} of {total_indices} indices"
        )));
    }

    Ok(faces)
}

fn validate_face_attribs(
    face_attribs: &Option<Vec<FaceAttrib>>,
    colors: &Option<Vec<[f32; 4]>>,
    face_count: usize,
) -> MeshResult<()> {
    let Some(face_attribs) = face_attribs else {
        return Ok(());
    };

    if face_attribs.len() != face_count {
        let count = face_attribs.len();
        return Err(MeshError::mesh(format!(
            "face attrib count {count} does not match {face_count} faces"
        )));
    }

    let color_count = colors.as_ref().map_or(0, |colors| colors.len());
    let color_count = u32::try_from(color_count)
        .map_err(|_| MeshError::mesh("color count exceeds the u32 index range"))?;
    for (face, attrib) in face_attribs.iter().enumerate() {
        if attrib.color >= color_count {
            return Err(MeshError::mesh(format!(
                "face {face} references color {} of {color_count}",
                attrib.color
            )));
        }
    }

    Ok(())
}

fn validate_uvs(uvs: &Option<Vec<[f32; 2]>>, vertex_count: usize) -> MeshResult<()> {
    let Some(uvs) = uvs else {
        return Ok(());
    };

    if uvs.len() != vertex_count {
        let count = uvs.len();
        return Err(MeshError::mesh(format!(
            "uv count {count} does not match {vertex_count} vertices"
        )));
    }

    Ok(())
}

fn validate_normals(normals: &Option<Vec<[f32; 3]>>, vertex_count: usize) -> MeshResult<()> {
    let Some(normals) = normals else {
        return Ok(());
    };

    if normals.len() != vertex_count {
        let count = normals.len();
        return Err(MeshError::mesh(format!(
            "normals count {count} does not match {vertex_count} vertices"
        )));
    }

    Ok(())
}

fn validate_indices(triangles: &[[u32; 3]], vertex_count: usize) -> MeshResult<()> {
    let vertex_count = u32::try_from(vertex_count)
        .map_err(|_| MeshError::mesh("vertex count exceeds the u32 index range"))?;

    for (triangle_index, triangle) in triangles.iter().enumerate() {
        for &index in triangle {
            if index >= vertex_count {
                return Err(MeshError::mesh(format!(
                    "triangle {triangle_index} references vertex {index} of {vertex_count}"
                )));
            }
        }
    }

    Ok(())
}
