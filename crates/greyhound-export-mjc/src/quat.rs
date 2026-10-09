/// The unit quaternion (w, x, y, z) rotating vectors like the given proper
/// 3x3 rotation matrix (row-major input). Shepperd's method, mirroring the
/// C# reference in `package/com.greyhound.step/Runtime/InertiaTensorMath.cs`;
/// the four branches pick the largest component to keep the division stable.
pub fn from_matrix(m: &[[f64; 3]; 3]) -> [f64; 4] {
    let trace = m[0][0] + m[1][1] + m[2][2];
    let (w, x, y, z);
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        w = 0.25 * s;
        x = (m[2][1] - m[1][2]) / s;
        y = (m[0][2] - m[2][0]) / s;
        z = (m[1][0] - m[0][1]) / s;
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        w = (m[2][1] - m[1][2]) / s;
        x = 0.25 * s;
        y = (m[0][1] + m[1][0]) / s;
        z = (m[0][2] + m[2][0]) / s;
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        w = (m[0][2] - m[2][0]) / s;
        x = (m[0][1] + m[1][0]) / s;
        y = 0.25 * s;
        z = (m[1][2] + m[2][1]) / s;
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        w = (m[1][0] - m[0][1]) / s;
        x = (m[0][2] + m[2][0]) / s;
        y = (m[1][2] + m[2][1]) / s;
        z = 0.25 * s;
    }
    normalize([w, x, y, z])
}

fn normalize(quat: [f64; 4]) -> [f64; 4] {
    let squared = quat[0] * quat[0] + quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3];
    let length = squared.sqrt();
    if length > 0.0 {
        [
            quat[0] / length,
            quat[1] / length,
            quat[2] / length,
            quat[3] / length,
        ]
    } else {
        // Unreachable for proper rotation matrices; defensive identity.
        [1.0, 0.0, 0.0, 0.0]
    }
}

/// The rotation matrix of a unit quaternion; test-only inverse used by the
/// round-trip and composition checks.
#[cfg(test)]
pub(crate) fn to_matrix(quat: &[f64; 4]) -> [[f64; 3]; 3] {
    let [w, x, y, z] = *quat;
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}
