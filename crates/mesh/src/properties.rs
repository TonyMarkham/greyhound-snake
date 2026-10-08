/// The exact BRep mass properties of one unique mesh, in the mesh's local
/// frame (OCCT units). Density-free: `principal_moments` are the
/// density-1 volume integrals, so consumers apply their own density
/// (`mass = volume_mm3 * density`, `inertia_i = mass * gyration_i^2`
/// with `gyration_i = sqrt(principal_moments_i / volume_mm3)`).
/// `file_density` is what the STEP file carries for the shape's material
/// (0.0 when it carries none, units of the file's context).
pub struct MeshProperties {
    pub volume_mm3: f64,
    pub centre_of_gravity: [f64; 3],
    pub principal_axes: [[f64; 3]; 3],
    pub principal_moments: [f64; 3],
    pub file_density: f64,
}
