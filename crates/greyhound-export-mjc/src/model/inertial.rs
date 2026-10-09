/// An `<inertial>` element: principal-frame mass properties in MJCF units —
/// `pos` in meters, `mass` in kilograms, `diaginertia` in kg·m², and `quat`
/// (w, x, y, z) orienting the principal axes within the body frame so that
/// `I_body = R · diag(diaginertia) · Rᵀ`.
#[derive(Clone, Debug)]
pub struct Inertial {
    pub pos: [f64; 3],
    pub quat: [f64; 4],
    pub mass: f64,
    pub diaginertia: [f64; 3],
}
