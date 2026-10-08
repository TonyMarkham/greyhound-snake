use crate::{
    HostError, HostMeshCounts, HostMeshProperties, HostResult, HostSceneCounts, host::Host,
};

use occt_sys::step_doc::StepDoc;
use unity_projection::{
    ProjectionSettings, UnityMesh, UnityNode, UnitySubMesh, UnityVertex, project_properties,
    project_scene,
};

use std::path::Path;

pub(crate) struct Doc {
    doc: StepDoc,
    scene: Option<ProjectedScene>,
}

pub(crate) struct ProjectedScene {
    nodes: Vec<UnityNode>,
    names: Vec<u8>,
    meshes: Vec<UnityMesh>,
    properties: Vec<HostMeshProperties>,
}

impl Doc {
    pub(crate) fn open(host: &Host, path: &Path) -> occt_sys::OcctResult<Self> {
        let doc = host.occt().open_step(path)?;
        Ok(Self { doc, scene: None })
    }

    pub(crate) fn scene_counts(
        &mut self,
        deflection: f64,
        angle_rad: f64,
        scale: f64,
    ) -> HostResult<HostSceneCounts> {
        let occt_scene = self
            .doc
            .scene(deflection, angle_rad)
            .map_err(|error| HostError::host(format!("tessellation: {error}")))?;
        let projected = project_scene(
            occt_scene.forest(),
            occt_scene.meshes(),
            ProjectionSettings { scale },
        )
        .map_err(|error| HostError::host(format!("projection: {error}")))?;

        let nodes = projected.nodes().to_vec();
        let names = projected.names().to_vec();
        let meshes = projected.meshes().to_vec();

        let mut properties = Vec::with_capacity(meshes.len());
        for index in 0..meshes.len() {
            let mesh_index = u32::try_from(index)
                .map_err(|_| HostError::host("mesh index exceeds the u32 range"))?;
            let occt_props = self
                .doc
                .mesh_properties(mesh_index)
                .map_err(|error| HostError::host(format!("mesh properties: {error}")))?;
            let projected = project_properties(&occt_props, ProjectionSettings { scale })
                .map_err(|error| HostError::host(format!("property projection: {error}")))?;
            properties.push(HostMeshProperties {
                volume_mm3: projected.volume_mm3,
                file_density: projected.file_density,
                centre_of_gravity: projected.centre_of_gravity,
                gyration_radii: projected.gyration_radii,
                principal_axes: projected.principal_axes,
            });
        }

        let node_count = u32::try_from(nodes.len())
            .map_err(|_| HostError::host("node count exceeds the u32 ABI range"))?;
        let mesh_count = u32::try_from(meshes.len())
            .map_err(|_| HostError::host("mesh count exceeds the u32 ABI range"))?;
        let name_bytes = u32::try_from(names.len())
            .map_err(|_| HostError::host("name bytes exceed the u32 ABI range"))?;
        let color_count = u32::try_from(meshes.first().map_or(0, |mesh| mesh.colors().len()))
            .map_err(|_| HostError::host("color count exceeds the u32 ABI range"))?;

        self.scene = Some(ProjectedScene {
            nodes,
            names,
            meshes,
            properties,
        });
        Ok(HostSceneCounts {
            node_count,
            mesh_count,
            color_count,
            name_bytes,
        })
    }

    pub(crate) fn scene(&self) -> HostResult<&ProjectedScene> {
        self.scene.as_ref().ok_or_else(|| {
            HostError::host("no projected scene; call greyhound_host_scene_counts first")
        })
    }

    pub(crate) fn scene_fill(
        &self,
        nodes: *mut u32,
        transforms: *mut [f32; 12],
        names: *mut u8,
    ) -> HostResult<()> {
        let scene = self.scene()?;
        // SAFETY: the caller allocated the buffers to the sizes reported by
        // the preceding scene_counts call (the ABI contract, same as the
        // shim); the source slices cannot overlap the caller's memory.
        unsafe {
            let mut node_cursor = nodes;
            for node in scene.nodes.iter() {
                *node_cursor = node.parent;
                *node_cursor.add(1) = node.mesh;
                *node_cursor.add(2) = node.name_offset;
                *node_cursor.add(3) = node.name_length;
                node_cursor = node_cursor.add(4);
            }
            let mut transform_cursor = transforms;
            for node in scene.nodes.iter() {
                transform_cursor.write(node.transform);
                transform_cursor = transform_cursor.add(1);
            }
            if !names.is_null() {
                std::ptr::copy_nonoverlapping(scene.names.as_ptr(), names, scene.names.len());
            }
        }
        Ok(())
    }

    pub(crate) fn color_fill(&self, colors: *mut [f32; 4]) -> HostResult<()> {
        let scene = self.scene()?;
        let Some(mesh) = scene.meshes.first() else {
            return Err(HostError::host("scene has no meshes"));
        };
        // SAFETY: the caller allocated the buffer to color_count reported by
        // scene_counts; the source slice cannot overlap the caller's memory.
        unsafe {
            std::ptr::copy_nonoverlapping(mesh.colors().as_ptr(), colors, mesh.colors().len());
        }
        Ok(())
    }

    pub(crate) fn mesh_properties(&self, mesh_index: u32) -> HostResult<HostMeshProperties> {
        let scene = self.scene()?;
        scene
            .properties
            .get(
                usize::try_from(mesh_index)
                    .map_err(|_| HostError::host("mesh index exceeds the address range"))?,
            )
            .copied()
            .ok_or_else(|| {
                HostError::host(format!(
                    "mesh index {mesh_index} beyond the {} scene meshes",
                    scene.properties.len()
                ))
            })
    }

    pub(crate) fn mesh_counts(&self, mesh_index: u32) -> HostResult<HostMeshCounts> {
        let scene = self.scene()?;
        let mesh = scene
            .meshes
            .get(
                usize::try_from(mesh_index)
                    .map_err(|_| HostError::host("mesh index exceeds the address range"))?,
            )
            .ok_or_else(|| {
                HostError::host(format!(
                    "mesh index {mesh_index} beyond the {} scene meshes",
                    scene.meshes.len()
                ))
            })?;
        Ok(HostMeshCounts {
            vertex_count: u32::try_from(mesh.vertices().len())
                .map_err(|_| HostError::host("vertex count exceeds the u32 ABI range"))?,
            index_count: u32::try_from(mesh.indices().len())
                .map_err(|_| HostError::host("index count exceeds the u32 ABI range"))?,
            submesh_count: u32::try_from(mesh.submeshes().len())
                .map_err(|_| HostError::host("submesh count exceeds the u32 ABI range"))?,
            color_count: u32::try_from(mesh.colors().len())
                .map_err(|_| HostError::host("color count exceeds the u32 ABI range"))?,
            bounds: mesh.bounds(),
        })
    }

    pub(crate) fn mesh_fill(
        &self,
        mesh_index: u32,
        verts: *mut UnityVertex,
        indices: *mut u32,
        submeshes: *mut UnitySubMesh,
        submesh_colors: *mut u32,
    ) -> HostResult<()> {
        let scene = self.scene()?;
        let mesh = scene
            .meshes
            .get(
                usize::try_from(mesh_index)
                    .map_err(|_| HostError::host("mesh index exceeds the address range"))?,
            )
            .ok_or_else(|| {
                HostError::host(format!(
                    "mesh index {mesh_index} beyond the {} scene meshes",
                    scene.meshes.len()
                ))
            })?;
        // SAFETY: the caller allocated the buffers to the sizes reported by
        // the preceding mesh_counts call (the ABI contract, same as the
        // shim); the source slices cannot overlap the caller's memory.
        unsafe {
            std::ptr::copy_nonoverlapping(mesh.vertices().as_ptr(), verts, mesh.vertices().len());
            std::ptr::copy_nonoverlapping(mesh.indices().as_ptr(), indices, mesh.indices().len());
            std::ptr::copy_nonoverlapping(
                mesh.submeshes().as_ptr(),
                submeshes,
                mesh.submeshes().len(),
            );
            std::ptr::copy_nonoverlapping(
                mesh.submesh_colors().as_ptr(),
                submesh_colors,
                mesh.submesh_colors().len(),
            );
        }
        Ok(())
    }
}
