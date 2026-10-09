use mesh::Mesh;

use std::io::{self, Write};

/// Writes one unique mesh as a binary STL with the length scale baked into
/// the vertices (meters per millimeter). The facet normal derives from the
/// winding — the core model is outward-CCW in right-handed algebra, so the
/// normal points outward; MuJoCo ignores it, but other right-handed
/// consumers (Blender, CAD viewers) do not. Degenerate triangles carry a
/// zero normal rather than NaN.
///
/// Layout: 80 zero header bytes (deliberately not ASCII and never starting
/// with "solid", the ASCII-detection heuristic), `u32` LE triangle count,
/// then 50 bytes per triangle — `f32` LE normal, v0, v1, v2 — plus a zero
/// `u16` attribute-byte count.
pub fn write_mesh(writer: &mut impl Write, mesh: &Mesh, scale: f64) -> io::Result<()> {
    let triangles = mesh.triangles();
    let count = u32::try_from(triangles.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "triangle count exceeds the u32 STL range",
        )
    })?;

    writer.write_all(&[0u8; 80])?;
    writer.write_all(&count.to_le_bytes())?;

    for triangle in triangles {
        let a = vertex_at(mesh.vertices(), triangle[0])?;
        let b = vertex_at(mesh.vertices(), triangle[1])?;
        let c = vertex_at(mesh.vertices(), triangle[2])?;

        let mut record = [0u8; 50];
        let mut offset = 0;
        for vertex in [
            winding_normal(a, b, c),
            scaled(a, scale),
            scaled(b, scale),
            scaled(c, scale),
        ] {
            for component in vertex {
                record[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
                offset += 4;
            }
        }
        // record[48..50] stay zero: the attribute byte count.
        writer.write_all(&record)?;
    }
    Ok(())
}

fn vertex_at(vertices: &[[f32; 3]], index: u32) -> io::Result<[f32; 3]> {
    let index = usize::try_from(index).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "vertex index exceeds the usize range",
        )
    })?;
    vertices.get(index).copied().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("triangle references vertex {index} of {}", vertices.len()),
        )
    })
}

fn scaled(vertex: [f32; 3], scale: f64) -> [f32; 3] {
    [
        (f64::from(vertex[0]) * scale) as f32,
        (f64::from(vertex[1]) * scale) as f32,
        (f64::from(vertex[2]) * scale) as f32,
    ]
}

fn winding_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let squared = cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2];
    let length = squared.sqrt();
    if length > 0.0 {
        [cross[0] / length, cross[1] / length, cross[2] / length]
    } else {
        [0.0; 3]
    }
}
