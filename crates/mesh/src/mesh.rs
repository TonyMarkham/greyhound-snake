use crate::{FaceRange, MeshError, MeshResult, builder::Builder};

#[derive(Debug)]
pub struct Mesh {
    vertices: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    faces: Vec<FaceRange>,
    uvs: Option<Vec<[f32; 2]>>,
    normals: Option<Vec<[f32; 3]>>,
}

impl Mesh {
    pub fn vertices(&self) -> &[[f32; 3]] {
        &self.vertices
    }

    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    pub fn faces(&self) -> &[FaceRange] {
        &self.faces
    }

    pub fn uvs(&self) -> Option<&[[f32; 2]]> {
        self.uvs.as_deref()
    }

    pub fn normals(&self) -> Option<&[[f32; 3]]> {
        self.normals.as_deref()
    }
}

impl TryFrom<Builder> for Mesh {
    type Error = MeshError;

    fn try_from(builder: Builder) -> MeshResult<Self> {
        let (vertices, triangles, faces, uvs, normals) = builder.into_parts()?;
        Ok(Self {
            vertices,
            triangles,
            faces,
            uvs,
            normals,
        })
    }
}
