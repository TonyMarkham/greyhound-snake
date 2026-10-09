use crate::quat::{from_matrix, to_matrix};

const TOLERANCE: f64 = 1e-12;

fn assert_quat_close(actual: &[f64; 4], expected: &[f64; 4]) {
    for (axis, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < TOLERANCE,
            "axis {axis}: {actual} vs {expected}"
        );
    }
}

fn assert_rotation_close(actual: &[[f64; 3]; 3], expected: &[[f64; 3]; 3]) {
    for (row, (actual_row, expected_row)) in actual.iter().zip(expected).enumerate() {
        for (column, (actual, expected)) in actual_row.iter().zip(expected_row).enumerate() {
            assert!(
                (actual - expected).abs() < TOLERANCE,
                "[{row}][{column}]: {actual} vs {expected}"
            );
        }
    }
}

fn rotation_z(angle: f64) -> [[f64; 3]; 3] {
    let (sine, cosine) = angle.sin_cos();
    [[cosine, -sine, 0.0], [sine, cosine, 0.0], [0.0, 0.0, 1.0]]
}

fn rotation_x(angle: f64) -> [[f64; 3]; 3] {
    let (sine, cosine) = angle.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, cosine, -sine], [0.0, sine, cosine]]
}

fn multiply(a: &[f64; 4], b: &[f64; 4]) -> [f64; 4] {
    let [aw, ax, ay, az] = *a;
    let [bw, bx, by, bz] = *b;
    [
        aw * bw - ax * bx - ay * by - az * bz,
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
    ]
}

fn multiply_matrix(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut product = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            product[row][column] =
                a[row][0] * b[0][column] + a[row][1] * b[1][column] + a[row][2] * b[2][column];
        }
    }
    product
}

fn dot(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

#[test]
fn given_the_identity_matrix_when_converted_then_the_quaternion_is_identity() {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_quat_close(&from_matrix(&identity), &[1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn given_quarter_turns_when_converted_then_the_axis_quaternions_match() {
    let half = 0.5_f64.sqrt();
    assert_quat_close(
        &from_matrix(&rotation_z(std::f64::consts::FRAC_PI_2)),
        &[half, 0.0, 0.0, half],
    );
    assert_quat_close(
        &from_matrix(&rotation_x(std::f64::consts::FRAC_PI_2)),
        &[half, half, 0.0, 0.0],
    );
}

#[test]
fn given_rotations_when_converted_then_matrix_and_quaternion_round_trip() {
    let matrices = [
        rotation_z(0.5),
        rotation_x(1.1),
        rotation_z(-2.0),
        multiply_matrix(&rotation_z(0.7), &rotation_x(0.3)),
    ];
    for matrix in &matrices {
        assert_rotation_close(&to_matrix(&from_matrix(matrix)), matrix);
    }
}

#[test]
fn given_composed_rotations_when_converted_then_quaternion_composition_matches() {
    let a = rotation_z(0.7);
    let b = rotation_x(0.3);
    let composed = from_matrix(&multiply_matrix(&a, &b));
    let product = multiply(&from_matrix(&a), &from_matrix(&b));
    // Double cover: q and -q are the same rotation.
    assert!(
        (dot(&composed, &product) - 1.0).abs() < 1e-9,
        "{composed:?} vs {product:?}"
    );
}

#[test]
fn given_identity_principal_axes_when_transposed_then_the_inertial_quat_is_identity() {
    // The fixture-box convention: rows are the axes; transposing puts them
    // in columns for Shepperd.
    let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let transposed = [
        [axes[0][0], axes[1][0], axes[2][0]],
        [axes[0][1], axes[1][1], axes[2][1]],
        [axes[0][2], axes[1][2], axes[2][2]],
    ];
    assert_quat_close(&from_matrix(&transposed), &[1.0, 0.0, 0.0, 0.0]);
}
