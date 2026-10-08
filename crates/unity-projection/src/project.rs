use crate::{
    OcctBounds, ProjectionError, ProjectionResult, ProjectionSettings, UnityBounds, UnityMesh,
    UnitySubMesh, UnityVertex,
};

use mesh::Mesh;

use std::collections::HashMap;

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
    let face_attribs = mesh
        .face_attribs()
        .ok_or_else(|| ProjectionError::projection("mesh has no face attribs"))?;
    let colors = mesh
        .colors()
        .ok_or_else(|| ProjectionError::projection("mesh has no colors"))?;
    if face_attribs.len() != mesh.faces().len() {
        return Err(ProjectionError::projection(format!(
            "face attrib count {} does not match {} faces",
            face_attribs.len(),
            mesh.faces().len()
        )));
    }

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

    // Faces regroup into submeshes keyed by (solid, color), in
    // first-appearance order; the index buffer is reordered so each
    // submesh owns a contiguous range.
    let mut group_order: Vec<(u32, u32)> = Vec::new();
    let mut groups: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (face_index, attrib) in face_attribs.iter().enumerate() {
        let key = (attrib.solid, attrib.color);
        if !groups.contains_key(&key) {
            group_order.push(key);
        }
        groups.entry(key).or_default().push(face_index);
    }

    let mut indices = Vec::with_capacity(mesh.triangles().len() * 3);
    let mut submeshes = Vec::with_capacity(group_order.len());
    let mut submesh_colors = Vec::with_capacity(group_order.len());
    for key in &group_order {
        let members = &groups[key];
        let index_start = indices.len();
        let mut first_vertex = u32::MAX;
        let mut vertex_end = 0u32;
        for &face_index in members {
            let face = &mesh.faces()[face_index];
            let face_index_start = usize::try_from(face.index_start).map_err(|_| {
                ProjectionError::projection("face index start exceeds the address range")
            })?;
            let face_index_end = usize::try_from(
                face.index_start
                    .checked_add(face.index_count)
                    .ok_or_else(|| ProjectionError::projection("face index range overflow"))?,
            )
            .map_err(|_| ProjectionError::projection("face index end exceeds the address range"))?;
            for triangle in &mesh.triangles()[face_index_start / 3..face_index_end / 3] {
                // The permutation has det -1, so each triangle's on-screen
                // order is counter-clockwise in Unity where front faces are
                // clockwise; the swap is the recorded fix (occt-to-unity.md,
                // Winding).
                indices.push(triangle[0]);
                indices.push(triangle[2]);
                indices.push(triangle[1]);
            }
            first_vertex = first_vertex.min(face.vertex_start);
            vertex_end = vertex_end.max(
                face.vertex_start
                    .checked_add(face.vertex_count)
                    .ok_or_else(|| ProjectionError::projection("face vertex range overflow"))?,
            );
        }
        let index_count = indices.len() - index_start;
        let index_start = u32::try_from(index_start)
            .map_err(|_| ProjectionError::projection("index buffer exceeds the u32 ABI range"))?;
        let index_count = u32::try_from(index_count)
            .map_err(|_| ProjectionError::projection("index buffer exceeds the u32 ABI range"))?;
        submeshes.push(UnitySubMesh {
            index_start,
            index_count,
            first_vertex,
            vertex_count: vertex_end - first_vertex,
        });
        submesh_colors.push(key.1);
    }

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

    Ok(UnityMesh::new(
        vertices,
        indices,
        submeshes,
        submesh_colors,
        colors.to_vec(),
        bounds,
    ))
}
