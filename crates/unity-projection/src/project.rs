use crate::{
    OcctBounds, ProjectionError, ProjectionResult, ProjectionSettings, UnityBounds, UnityMesh,
    UnityNode, UnityScene, UnitySubMesh, UnityVertex,
};

use mesh::Mesh;

use std::collections::HashMap;

pub fn project(
    mesh: &Mesh,
    occt_bounds: OcctBounds,
    settings: ProjectionSettings,
) -> ProjectionResult<UnityMesh> {
    let bounds = UnityBounds {
        min: [
            (occt_bounds.min[0] * settings.scale) as f32,
            (occt_bounds.min[2] * settings.scale) as f32,
            (occt_bounds.min[1] * settings.scale) as f32,
        ],
        max: [
            (occt_bounds.max[0] * settings.scale) as f32,
            (occt_bounds.max[2] * settings.scale) as f32,
            (occt_bounds.max[1] * settings.scale) as f32,
        ],
    };
    assemble(mesh, settings, bounds)
}

/// Projects the whole assembly forest: every unique mesh once (bounds
/// derived from the projected vertices), every node with its transform
/// conjugated into Unity space.
pub fn project_scene(
    forest: &mesh::Scene,
    meshes: &[Mesh],
    settings: ProjectionSettings,
) -> ProjectionResult<UnityScene> {
    if !settings.scale.is_finite() || settings.scale <= 0.0 {
        return Err(ProjectionError::projection(format!(
            "scale {} must be finite and positive",
            settings.scale
        )));
    }
    if meshes.len() != usize::try_from(forest.mesh_count()).unwrap_or(usize::MAX) {
        return Err(ProjectionError::projection(format!(
            "{} meshes do not match the forest's mesh count {}",
            meshes.len(),
            forest.mesh_count()
        )));
    }
    let mut projected_meshes = Vec::with_capacity(meshes.len());
    for mesh in meshes {
        let projected = assemble(
            mesh,
            settings,
            UnityBounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
        )?;
        let bounds = projected_vertex_bounds(projected.vertices());
        let projected = assemble(mesh, settings, bounds)?;
        projected_meshes.push(projected);
    }
    let mut nodes = Vec::with_capacity(forest.nodes().len());
    for node in forest.nodes() {
        nodes.push(UnityNode {
            parent: node.parent,
            mesh: node.mesh,
            name_offset: node.name_offset,
            name_length: node.name_length,
            transform: project_transform(&node.transform, settings.scale),
        });
    }
    Ok(UnityScene::new(
        nodes,
        projected_meshes,
        forest.names().to_vec(),
    ))
}

/// Conjugates a node transform (row-major 3x4, OCCT space) by the axis
/// permutation `Unity = (x, z, y)` and bakes the scale into the translation.
/// `Unity = M · occt` with M an involution, so the rotation becomes
/// `M · R · M` and the translation `scale · M · t`; the rotation keeps any
/// uniform scale factor carried by the source transform, which the C# side
/// recovers through column norms during TRS decomposition.
pub(crate) fn project_transform(transform: &[f32; 12], scale: f64) -> [f32; 12] {
    // new row i reads old row sigma(i); sigma = (0, 2, 1).
    const SIGMA: [usize; 3] = [0, 2, 1];
    let mut projected = [0.0f32; 12];
    for (row, source) in SIGMA.iter().enumerate() {
        for (col, source_col) in SIGMA.iter().enumerate() {
            projected[4 * row + col] = transform[4 * source + source_col];
        }
        projected[4 * row + 3] = (f64::from(transform[4 * source + 3]) * scale) as f32;
    }
    projected
}

fn projected_vertex_bounds(vertices: &[UnityVertex]) -> UnityBounds {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for vertex in vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    if vertices.is_empty() {
        (min, max) = ([0.0; 3], [0.0; 3]);
    }
    UnityBounds { min, max }
}

fn assemble(
    mesh: &Mesh,
    settings: ProjectionSettings,
    bounds: UnityBounds,
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

    Ok(UnityMesh::new(
        vertices,
        indices,
        submeshes,
        submesh_colors,
        colors.to_vec(),
        bounds,
    ))
}
