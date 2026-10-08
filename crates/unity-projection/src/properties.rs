use crate::{ProjectionResult, settings::ProjectionSettings};

use mesh::MeshProperties;

/// One unique mesh's mass properties projected into Unity space: the
/// centre of gravity permuted and scaled (body-local meters), principal
/// axes permuted (body-local unit directions); moments stay density-free
/// volume integrals and consumers derive inertia from mass and gyration.
/// Volume, gyration radii and the file density pass through unchanged.
pub struct UnityPartProperties {
    pub volume_mm3: f32,
    pub file_density: f32,
    pub centre_of_gravity: [f32; 3],
    pub gyration_radii: [f32; 3],
    pub principal_axes: [f32; 9],
}

/// Conjugates the mass properties by the axis permutation
/// `Unity = (x, z, y)` and bakes the scale into the centre of gravity.
/// Axes are directions: permuted, never scaled. The principal-axes
/// matrix stays orthonormal, so the reflection reverses rotation sense
/// exactly like the node transforms.
pub fn project_properties(
    properties: &MeshProperties,
    settings: ProjectionSettings,
) -> ProjectionResult<UnityPartProperties> {
    if !settings.scale.is_finite() || settings.scale <= 0.0 {
        return Err(crate::ProjectionError::projection(format!(
            "scale {} must be finite and positive",
            settings.scale
        )));
    }
    const SIGMA: [usize; 3] = [0, 2, 1];
    let scale = settings.scale as f32;
    let com = properties.centre_of_gravity;
    let axes = properties.principal_axes;
    let mut projected_axes = [0.0f32; 9];
    for (row, source) in SIGMA.iter().enumerate() {
        projected_axes[3 * row] = axes[*source][0] as f32;
        projected_axes[3 * row + 1] = axes[*source][2] as f32;
        projected_axes[3 * row + 2] = axes[*source][1] as f32;
    }
    Ok(UnityPartProperties {
        volume_mm3: properties.volume_mm3 as f32,
        file_density: properties.file_density as f32,
        centre_of_gravity: [
            (com[0] * f64::from(scale)) as f32,
            (com[2] * f64::from(scale)) as f32,
            (com[1] * f64::from(scale)) as f32,
        ],
        gyration_radii: [
            (properties.principal_moments[0] / properties.volume_mm3)
                .max(0.0)
                .sqrt() as f32,
            (properties.principal_moments[1] / properties.volume_mm3)
                .max(0.0)
                .sqrt() as f32,
            (properties.principal_moments[2] / properties.volume_mm3)
                .max(0.0)
                .sqrt() as f32,
        ],
        principal_axes: projected_axes,
    })
}
