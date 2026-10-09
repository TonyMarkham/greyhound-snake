use crate::{
    error::{Error, Result},
    model::{Body, BodyKind, Inertial, MeshAsset, MjcfModel},
    naming::{NameAllocator, comment_text},
    quat,
};

use mesh::{MeshProperties, Node, Scene};

/// Tolerance for the orthonormality and rigidity checks on f32-sourced node
/// transforms and GProp principal axes (the `tools/verify-package.py` axes
/// tolerance).
const TOLERANCE: f64 = 1e-4;

/// Unit factors to MJCF units: volume mm³ and density g/cm³ give kilograms
/// via 1e-6; density-1 moments in mm⁵ give kg·m² via 1e-12. Both are
/// independent of the baked length scale — physics does not care about the
/// model's unit choice.
const MASS_FACTOR: f64 = 1e-6;
const INERTIA_FACTOR: f64 = 1e-12;

pub struct AssembleOptions {
    /// The `<mujoco model>` value (the STEP file stem).
    pub model_name: String,
    /// Length scale baked into positions (meters per millimeter).
    pub scale: f64,
    /// Whether the root body gets a `<freejoint/>`.
    pub free_root: bool,
    /// The `--density` fallback in g/cm³.
    pub density_parameter: f64,
    /// The tessellation deflection, for the header comment only.
    pub deflection: f64,
}

/// Validates the scene and mass properties and builds the MJCF document
/// model: body names, body frames, mesh assets, and principal-frame
/// inertials. Pure — no OCCT, no IO.
pub fn assemble(
    forest: &Scene,
    properties: &[MeshProperties],
    options: &AssembleOptions,
) -> Result<MjcfModel> {
    if !options.scale.is_finite() || options.scale <= 0.0 {
        return Err(Error::invalid(format!(
            "scale {} must be finite and positive",
            options.scale
        )));
    }
    if !options.density_parameter.is_finite() || options.density_parameter <= 0.0 {
        return Err(Error::invalid(format!(
            "density {} must be finite and positive",
            options.density_parameter
        )));
    }
    let mesh_count = usize::try_from(forest.mesh_count())
        .map_err(|_| Error::invalid("mesh count exceeds the usize range"))?;
    if properties.len() != mesh_count {
        return Err(Error::invalid(format!(
            "{} mesh property sets do not match {} unique meshes",
            properties.len(),
            mesh_count
        )));
    }

    let roots: Vec<usize> = forest
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent == Node::NO_PARENT)
        .map(|(index, _)| index)
        .collect();
    if roots.len() != 1 {
        return Err(Error::invalid(format!(
            "expected exactly one scene root, found {}",
            roots.len()
        )));
    }
    let root_index = roots[0];
    if options.free_root && forest.nodes()[root_index].mesh == Node::NO_MESH {
        return Err(Error::invalid(
            "free-root requires the root body to carry mass; the root is a geometry-free grouping node",
        ));
    }

    // First user per unique mesh, for the asset comments.
    let mut first_user: Vec<Option<String>> = vec![None; mesh_count];
    for node in forest.nodes() {
        if node.mesh != Node::NO_MESH {
            let index = usize::try_from(node.mesh)
                .map_err(|_| Error::invalid("mesh index exceeds the usize range"))?;
            let raw = String::from_utf8_lossy(forest.node_name(node)).into_owned();
            if first_user[index].is_none() {
                first_user[index] = Some(raw);
            }
        }
    }

    let mut inertials = Vec::with_capacity(mesh_count);
    for (index, properties) in properties.iter().enumerate() {
        let mesh = u32::try_from(index)
            .map_err(|_| Error::invalid("mesh count exceeds the u32 ABI range"))?;
        let density = if properties.file_density > 0.0 {
            properties.file_density
        } else {
            options.density_parameter
        };
        inertials.push(inertial(mesh, properties, density, options.scale)?);
    }

    let mut assets = Vec::with_capacity(mesh_count);
    for (index, first) in first_user.iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| Error::invalid("mesh count exceeds the u32 ABI range"))?;
        let comment = first
            .as_deref()
            .map(|raw| format!("first used by: {}", comment_text(raw)))
            .unwrap_or_default();
        assets.push(MeshAsset {
            name: format!("mesh_{index:03}"),
            file: format!("mesh_{index:03}.stl"),
            comment,
        });
    }

    // Depth-first pre-order with parents preceding children: a stack of open
    // ancestors yields each node's nesting depth.
    let mut stack: Vec<u32> = Vec::new();
    let mut allocator = NameAllocator::new();
    let mut bodies = Vec::with_capacity(forest.nodes().len());
    for (index, node) in forest.nodes().iter().enumerate() {
        while let Some(&top) = stack.last() {
            if top == node.parent {
                break;
            }
            stack.pop();
        }
        let depth = stack.len();
        stack.push(
            u32::try_from(index)
                .map_err(|_| Error::invalid("node count exceeds the u32 ABI range"))?,
        );

        let node_id = u32::try_from(index)
            .map_err(|_| Error::invalid("node count exceeds the u32 ABI range"))?;
        let raw = String::from_utf8_lossy(forest.node_name(node)).into_owned();
        let name = allocator.allocate(&raw, &format!("part{index}"));
        let (pos, orientation) = body_frame(node_id, &node.transform, options.scale)?;
        let kind = if node.mesh == Node::NO_MESH {
            BodyKind::Grouping
        } else {
            let mesh_index = usize::try_from(node.mesh)
                .map_err(|_| Error::invalid("mesh index exceeds the usize range"))?;
            BodyKind::Meshed {
                mesh: assets[mesh_index].name.clone(),
                inertial: inertials[mesh_index].clone(),
            }
        };
        bodies.push(Body {
            name,
            comment: part_comment(&raw),
            depth,
            pos,
            quat: orientation,
            kind,
        });
    }

    Ok(MjcfModel {
        model_name: options.model_name.clone(),
        header: format!(
            "exported by greyhound-export-mjc from {}; density {:.1} g/cm3, scale {}, free-root {}, deflection {}",
            comment_text(&options.model_name),
            options.density_parameter,
            options.scale,
            if options.free_root { "on" } else { "off" },
            options.deflection,
        ),
        assets,
        bodies,
        free_root: options.free_root,
    })
}

fn part_comment(raw: &str) -> String {
    if raw.is_empty() {
        String::new()
    } else {
        format!("part: {}", comment_text(raw))
    }
}

fn body_frame(node: u32, transform: &[f32; 12], scale: f64) -> Result<([f64; 3], [f64; 4])> {
    for column in [3, 7, 11] {
        if !transform[column].is_finite() {
            return Err(Error::transform(
                node,
                "transform carries non-finite entries",
            ));
        }
    }
    let rotation = [
        [
            f64::from(transform[0]),
            f64::from(transform[1]),
            f64::from(transform[2]),
        ],
        [
            f64::from(transform[4]),
            f64::from(transform[5]),
            f64::from(transform[6]),
        ],
        [
            f64::from(transform[8]),
            f64::from(transform[9]),
            f64::from(transform[10]),
        ],
    ];
    validate_rigid(node, &rotation)?;
    let translation = [
        f64::from(transform[3]) * scale,
        f64::from(transform[7]) * scale,
        f64::from(transform[11]) * scale,
    ];
    Ok((translation, quat::from_matrix(&rotation)))
}

/// MJCF bodies carry no scale and meshes are shared across instances, so a
/// non-rigid node transform cannot be represented: `RᵀR ≈ I` and
/// `det(R) ≈ +1` are validated instead of silently dropped.
fn validate_rigid(node: u32, r: &[[f64; 3]; 3]) -> Result<()> {
    for row in r {
        for value in row {
            if !value.is_finite() {
                return Err(Error::transform(
                    node,
                    "transform carries non-finite entries",
                ));
            }
        }
    }
    let mut rtr = [[0.0f64; 3]; 3];
    for (row, r_row) in rtr.iter_mut().enumerate() {
        for (column, value) in r_row.iter_mut().enumerate() {
            *value = r[0][row] * r[0][column] + r[1][row] * r[1][column] + r[2][row] * r[2][column];
        }
    }
    for (row, r_row) in rtr.iter().enumerate() {
        if (r_row[row] - 1.0).abs() > TOLERANCE {
            return Err(Error::transform(
                node,
                "transform is not a rigid placement (MJCF bodies cannot scale)",
            ));
        }
        for deviation in r_row.iter().skip(row + 1) {
            if deviation.abs() > TOLERANCE {
                return Err(Error::transform(
                    node,
                    "transform is not a rigid placement (MJCF bodies cannot scale)",
                ));
            }
        }
    }
    if (determinant(r) - 1.0).abs() > TOLERANCE {
        return Err(Error::transform(
            node,
            "transform is not a rigid placement (mirrored placements are not representable)",
        ));
    }
    Ok(())
}

fn inertial(mesh: u32, properties: &MeshProperties, density: f64, scale: f64) -> Result<Inertial> {
    if !properties.volume_mm3.is_finite() || properties.volume_mm3 <= 0.0 {
        return Err(Error::properties(
            mesh,
            format!(
                "volume {} must be finite and positive",
                properties.volume_mm3
            ),
        ));
    }
    for value in &properties.centre_of_gravity {
        if !value.is_finite() {
            return Err(Error::properties(
                mesh,
                "centre of gravity carries non-finite entries",
            ));
        }
    }
    for (axis, moment) in properties.principal_moments.iter().enumerate() {
        if !moment.is_finite() || *moment < 0.0 {
            return Err(Error::properties(
                mesh,
                format!("principal moment {axis} {moment} must be finite and non-negative"),
            ));
        }
    }
    Ok(Inertial {
        pos: [
            properties.centre_of_gravity[0] * scale,
            properties.centre_of_gravity[1] * scale,
            properties.centre_of_gravity[2] * scale,
        ],
        quat: principal_rotation(mesh, &properties.principal_axes)?,
        mass: properties.volume_mm3 * density * MASS_FACTOR,
        diaginertia: [
            properties.principal_moments[0] * density * INERTIA_FACTOR,
            properties.principal_moments[1] * density * INERTIA_FACTOR,
            properties.principal_moments[2] * density * INERTIA_FACTOR,
        ],
    })
}

/// The MJCF inertial-frame quaternion from the principal axes. `rows` holds
/// the axis unit vectors: orthonormality is validated, a left-handed frame
/// gets one axis sign flipped (an axis sign is invisible to the inertia
/// tensor), and the transpose — axes in columns — feeds Shepperd so that
/// `I_body = R · diag · Rᵀ`.
fn principal_rotation(mesh: u32, axes: &[[f64; 3]; 3]) -> Result<[f64; 4]> {
    let mut rows = *axes;
    for row in &rows {
        for value in row {
            if !value.is_finite() {
                return Err(Error::properties(
                    mesh,
                    "principal axes carry non-finite entries",
                ));
            }
        }
    }
    for (i, row) in rows.iter().enumerate() {
        let squared = row[0] * row[0] + row[1] * row[1] + row[2] * row[2];
        if (squared - 1.0).abs() > TOLERANCE {
            return Err(Error::properties(
                mesh,
                "principal axes are not unit vectors",
            ));
        }
        for other in rows.iter().skip(i + 1) {
            let dot = row[0] * other[0] + row[1] * other[1] + row[2] * other[2];
            if dot.abs() > TOLERANCE {
                return Err(Error::properties(mesh, "principal axes are not orthogonal"));
            }
        }
    }
    if determinant(&rows) < 0.0 {
        rows[2] = [-rows[2][0], -rows[2][1], -rows[2][2]];
    }
    let transposed = [
        [rows[0][0], rows[1][0], rows[2][0]],
        [rows[0][1], rows[1][1], rows[2][1]],
        [rows[0][2], rows[1][2], rows[2][2]],
    ];
    Ok(quat::from_matrix(&transposed))
}

fn determinant(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}
