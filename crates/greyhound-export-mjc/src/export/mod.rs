mod options;
mod summary;

pub use options::ExportOptions;
pub use summary::ExportSummary;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    app_config::AppConfig,
    assemble::{AssembleOptions, assemble},
    error::{Error, Result},
    mjcf, stl,
};

use occt_sys::Occt;

use std::{
    fs,
    io::{BufWriter, Write},
};

/// Tessellation parameters, matching `step-stats`, `tools/verify-package.py`,
/// and the C# importer.
const DEFLECTION: f64 = 0.01;
const ANGLE_RAD: f64 = 0.5;

pub fn export(options: &ExportOptions) -> Result<ExportSummary> {
    let config = AppConfig::load(&options.config_path).map_err(Error::config)?;
    let occt = Occt::load(&config.occt.library_dir, &config.occt.shim_path).map_err(Error::step)?;
    let doc = occt.open_step(&options.step_path).map_err(Error::step)?;
    let scene = doc.scene(DEFLECTION, ANGLE_RAD).map_err(Error::step)?;
    let mesh_count = scene.meshes().len();

    let mut properties = Vec::with_capacity(mesh_count);
    for index in 0..mesh_count {
        let index = u32::try_from(index)
            .map_err(|_| Error::invalid("mesh count exceeds the u32 ABI range"))?;
        properties.push(doc.mesh_properties(index).map_err(Error::step)?);
    }

    let stem = options
        .step_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .ok_or_else(|| {
            Error::invalid(format!(
                "STEP path {} has no file stem",
                options.step_path.display()
            ))
        })?;

    let assemble_options = AssembleOptions {
        model_name: stem.clone(),
        scale: options.scale,
        free_root: options.free_root,
        density_parameter: options.density,
        deflection: DEFLECTION,
    };
    let model = assemble(scene.forest(), &properties, &assemble_options)?;

    let mesh_dir = options.out_dir.join("meshes");
    fs::create_dir_all(&mesh_dir).map_err(|source| Error::io(mesh_dir.clone(), source))?;

    for (index, mesh) in scene.meshes().iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| Error::invalid("mesh count exceeds the u32 ABI range"))?;
        let path = mesh_dir.join(format!("mesh_{index:03}.stl"));
        let file = fs::File::create(&path).map_err(|source| Error::io(path.clone(), source))?;
        let mut writer = BufWriter::new(file);
        stl::write_mesh(&mut writer, mesh, options.scale)
            .map_err(|source| Error::io(path.clone(), source))?;
        writer
            .flush()
            .map_err(|source| Error::io(path.clone(), source))?;
    }

    let model_path = options.out_dir.join(format!("{}.xml", stem));
    let file =
        fs::File::create(&model_path).map_err(|source| Error::io(model_path.clone(), source))?;
    let mut writer = BufWriter::new(file);
    mjcf::write_model(&mut writer, &model)
        .map_err(|source| Error::io(model_path.clone(), source))?;
    writer
        .flush()
        .map_err(|source| Error::io(model_path.clone(), source))?;

    let body_count = model.bodies.len();
    let geom_count = model
        .bodies
        .iter()
        .filter(|body| matches!(body.kind, crate::model::BodyKind::Meshed { .. }))
        .count();
    let total_mass_kg = model
        .bodies
        .iter()
        .filter_map(|body| match &body.kind {
            crate::model::BodyKind::Meshed { inertial, .. } => Some(inertial.mass),
            crate::model::BodyKind::Grouping => None,
        })
        .sum();

    Ok(ExportSummary {
        model_path,
        body_count,
        mesh_count,
        geom_count,
        total_mass_kg,
    })
}
