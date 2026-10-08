use crate::{HostError, HostMeshCounts, HostResult, host::Host};

use occt_sys::step_doc::StepDoc;
use unity_projection::{
    OcctBounds, ProjectionSettings, UnityMesh, UnitySubMesh, UnityVertex, project,
};

use std::path::Path;

pub(crate) struct Doc {
    doc: StepDoc,
    mesh: Option<UnityMesh>,
}

impl Doc {
    pub(crate) fn open(host: &Host, path: &Path) -> occt_sys::OcctResult<Self> {
        let doc = host.occt().open_step(path)?;
        Ok(Self { doc, mesh: None })
    }

    pub(crate) fn counts(
        &mut self,
        deflection: f64,
        angle_rad: f64,
        scale: f64,
    ) -> HostResult<HostMeshCounts> {
        let info = self
            .doc
            .info()
            .map_err(|error| HostError::host(format!("STEP info: {error}")))?;
        let mesh = self
            .doc
            .mesh(deflection, angle_rad)
            .map_err(|error| HostError::host(format!("tessellation: {error}")))?;
        let projected = project(
            &mesh,
            OcctBounds {
                min: info.bbox.min,
                max: info.bbox.max,
            },
            ProjectionSettings { scale },
        )
        .map_err(|error| HostError::host(format!("projection: {error}")))?;

        let counts = HostMeshCounts {
            vertex_count: u32::try_from(projected.vertices().len())
                .map_err(|_| HostError::host("vertex count exceeds the u32 ABI range"))?,
            index_count: u32::try_from(projected.indices().len())
                .map_err(|_| HostError::host("index count exceeds the u32 ABI range"))?,
            submesh_count: u32::try_from(projected.submeshes().len())
                .map_err(|_| HostError::host("submesh count exceeds the u32 ABI range"))?,
            bounds: projected.bounds(),
        };
        self.mesh = Some(projected);
        Ok(counts)
    }

    pub(crate) fn fill(
        &mut self,
        verts: *mut UnityVertex,
        indices: *mut u32,
        submeshes: *mut UnitySubMesh,
    ) -> HostResult<()> {
        let Some(mesh) = self.mesh.as_ref() else {
            return Err(HostError::host(
                "no projected mesh; call greyhound_host_mesh_counts first",
            ));
        };
        // SAFETY: the caller allocated the buffers to the sizes reported by
        // the preceding counts call (the ABI contract, same as the shim);
        // the source slices cannot overlap the caller's memory.
        unsafe {
            std::ptr::copy_nonoverlapping(mesh.vertices().as_ptr(), verts, mesh.vertices().len());
            std::ptr::copy_nonoverlapping(mesh.indices().as_ptr(), indices, mesh.indices().len());
            std::ptr::copy_nonoverlapping(
                mesh.submeshes().as_ptr(),
                submeshes,
                mesh.submeshes().len(),
            );
        }
        Ok(())
    }
}
