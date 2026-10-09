use crate::model::{BodyKind, MjcfModel};

use std::io::{self, Write};

/// Serializes the MJCF document model. Pure IO and formatting — all
/// validation happened in `assemble`, so only writes can fail. Numbers use
/// Rust's shortest round-trip formatting (MuJoCo parses with strtod, which
/// accepts both its decimal and scientific shapes).
pub fn write_model(writer: &mut impl Write, model: &MjcfModel) -> io::Result<()> {
    writeln!(writer, "<?xml version=\"1.0\" encoding=\"utf-8\"?>")?;
    writeln!(writer, "<!-- {} -->", model.header)?;
    writeln!(
        writer,
        "<mujoco model=\"{}\">",
        escape_attr(&model.model_name)
    )?;
    writeln!(
        writer,
        "  <compiler angle=\"radian\" meshdir=\"meshes\" autolimits=\"true\" inertiafromgeom=\"auto\" balanceinertia=\"false\"/>"
    )?;

    writeln!(writer, "  <asset>")?;
    for asset in &model.assets {
        if !asset.comment.is_empty() {
            writeln!(writer, "    <!-- {} -->", asset.comment)?;
        }
        writeln!(
            writer,
            "    <mesh name=\"{}\" file=\"{}\"/>",
            escape_attr(&asset.name),
            escape_attr(&asset.file)
        )?;
    }
    writeln!(writer, "  </asset>")?;

    writeln!(writer, "  <worldbody>")?;
    // Nodes are depth-first pre-order; the depth recorded per body tracks
    // how many `<body>` tags are open, so closing is a simple count-down.
    let mut open = 0usize;
    for body in &model.bodies {
        while open > body.depth {
            open -= 1;
            writeln!(writer, "{}</body>", indent(open))?;
        }
        let body_indent = indent(body.depth);
        if !body.comment.is_empty() {
            writeln!(writer, "{body_indent}<!-- {} -->", body.comment)?;
        }
        writeln!(
            writer,
            "{body_indent}<body name=\"{}\" pos=\"{}\" quat=\"{}\">",
            escape_attr(&body.name),
            vec3(&body.pos),
            vec4(&body.quat)
        )?;
        open += 1;
        let inner = indent(open);
        if model.free_root && body.depth == 0 {
            writeln!(writer, "{inner}<freejoint/>")?;
        }
        if let BodyKind::Meshed { mesh, inertial } = &body.kind {
            writeln!(
                writer,
                "{inner}<geom type=\"mesh\" mesh=\"{}\"/>",
                escape_attr(mesh)
            )?;
            writeln!(
                writer,
                "{inner}<inertial pos=\"{}\" quat=\"{}\" mass=\"{}\" diaginertia=\"{}\"/>",
                vec3(&inertial.pos),
                vec4(&inertial.quat),
                fmt_f64(inertial.mass),
                vec3(&inertial.diaginertia)
            )?;
        }
    }
    while open > 0 {
        open -= 1;
        writeln!(writer, "{}</body>", indent(open))?;
    }
    writeln!(writer, "  </worldbody>")?;
    writeln!(writer, "</mujoco>")?;
    Ok(())
}

fn indent(depth: usize) -> String {
    // `worldbody` sits at 2 spaces and every body level adds 2, so a body at
    // nesting depth d indents 4 + 2d and its children 6 + 2d.
    "  ".repeat(depth + 2)
}

fn vec3(values: &[f64; 3]) -> String {
    values
        .iter()
        .map(|value| fmt_f64(*value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn vec4(values: &[f64; 4]) -> String {
    values
        .iter()
        .map(|value| fmt_f64(*value))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Decimal formatting in the readable range, scientific outside it; zero is
/// spelled `0`. Both shapes round-trip through strtod.
fn fmt_f64(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let magnitude = value.abs();
    if (1e-4..1e6).contains(&magnitude) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

fn escape_attr(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}
