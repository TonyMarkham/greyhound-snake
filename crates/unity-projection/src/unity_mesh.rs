use crate::{UnityBounds, UnitySubMesh, UnityVertex};

#[derive(Debug)]
pub struct UnityMesh {
    vertices: Vec<UnityVertex>,
    indices: Vec<u32>,
    submeshes: Vec<UnitySubMesh>,
    submesh_colors: Vec<u32>,
    colors: Vec<[f32; 4]>,
    bounds: UnityBounds,
}

impl UnityMesh {
    pub(crate) fn new(
        vertices: Vec<UnityVertex>,
        indices: Vec<u32>,
        submeshes: Vec<UnitySubMesh>,
        submesh_colors: Vec<u32>,
        colors: Vec<[f32; 4]>,
        bounds: UnityBounds,
    ) -> Self {
        Self {
            vertices,
            indices,
            submeshes,
            submesh_colors,
            colors,
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

    /// Per-submesh color index into `colors()`, in submesh order.
    pub fn submesh_colors(&self) -> &[u32] {
        &self.submesh_colors
    }

    /// Color table entries as sRGB RGBA.
    pub fn colors(&self) -> &[[f32; 4]] {
        &self.colors
    }

    pub fn bounds(&self) -> UnityBounds {
        self.bounds
    }
}
