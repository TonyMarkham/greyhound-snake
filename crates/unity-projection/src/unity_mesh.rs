use crate::{UnityBounds, UnitySubMesh, UnityVertex};

#[derive(Debug)]
pub struct UnityMesh {
    vertices: Vec<UnityVertex>,
    indices: Vec<u32>,
    submeshes: Vec<UnitySubMesh>,
    bounds: UnityBounds,
}

impl UnityMesh {
    pub(crate) fn new(
        vertices: Vec<UnityVertex>,
        indices: Vec<u32>,
        submeshes: Vec<UnitySubMesh>,
        bounds: UnityBounds,
    ) -> Self {
        Self {
            vertices,
            indices,
            submeshes,
            bounds,
        }
    }

    pub fn vertices(&self) -> &[UnityVertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn submeshes(&self) -> &[UnitySubMesh] {
        &self.submeshes
    }

    pub fn bounds(&self) -> UnityBounds {
        self.bounds
    }
}
