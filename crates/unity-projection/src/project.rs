use crate::{
    OcctBounds, ProjectionError, ProjectionResult, ProjectionSettings, UnityBounds, UnityMesh,
    UnitySubMesh, UnityVertex,
};

use mesh::Mesh;

pub fn project(
    mesh: &Mesh,
    occt_bounds: OcctBounds,
    settings: ProjectionSettings,
) -> ProjectionResult<UnityMesh> {
    let scale = settings.scale;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(ProjectionError::projection(format!(
            "scale {scale} must be finite and positive"
        )));
    }
    let vertex_scale = scale as f32;

    let normals = mesh
        .normals()
        .ok_or_else(|| ProjectionError::projection("mesh has no normals"))?;

    let vertices = mesh
        .vertices()
        .iter()
        .zip(normals)
        .map(|(position, normal)| UnityVertex {
            position: [
                position[0] * vertex_scale,
                position[2] * vertex_scale,
                position[1] * vertex_scale,
            ],
            normal: [normal[0], normal[2], normal[1]],
        })
        .collect();

    let mut indices = Vec::with_capacity(mesh.triangles().len() * 3);
    for triangle in mesh.triangles() {
        // The permutation has det -1, so each triangle's on-screen order is
        // counter-clockwise in Unity where front faces are clockwise; the
        // swap is the recorded fix (occt-to-unity.md, Winding).
        indices.push(triangle[0]);
        indices.push(triangle[2]);
        indices.push(triangle[1]);
    }

    let submeshes = mesh
        .faces()
        .iter()
        .map(|face| UnitySubMesh {
            index_start: face.index_start,
            index_count: face.index_count,
            first_vertex: face.vertex_start,
            vertex_count: face.vertex_count,
        })
        .collect();

    let bounds = UnityBounds {
        min: [
            (occt_bounds.min[0] * scale) as f32,
            (occt_bounds.min[2] * scale) as f32,
            (occt_bounds.min[1] * scale) as f32,
        ],
        max: [
            (occt_bounds.max[0] * scale) as f32,
            (occt_bounds.max[2] * scale) as f32,
            (occt_bounds.max[1] * scale) as f32,
        ],
    };

    Ok(UnityMesh::new(vertices, indices, submeshes, bounds))
}
