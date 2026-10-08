mod app_config;
mod args;
mod config_error;
mod occt_config;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use app_config::AppConfig;
use args::Args;

use occt_sys::Occt;
use unity_projection::{ProjectionSettings, project_scene};

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(); // --help exits before config or native loading.
    let config = AppConfig::load(&args.config)?;
    let occt = Occt::load(&config.occt.library_dir, &config.occt.shim_path)?;
    let doc = occt.open_step(&args.path)?;
    let info = doc.info()?;

    println!("file:     {}", args.path.display());
    println!("solids:   {}", info.solids);
    println!("faces:    {}", info.faces);
    println!("edges:    {}", info.edges);
    println!(
        "bbox min: [{:.3} {:.3} {:.3}]",
        info.bbox.min[0], info.bbox.min[1], info.bbox.min[2]
    );
    println!(
        "bbox max: [{:.3} {:.3} {:.3}]",
        info.bbox.max[0], info.bbox.max[1], info.bbox.max[2]
    );
    if args.mesh || args.unity {
        let scene = doc.scene(0.01, 0.5)?;
        let forest = scene.forest();
        println!("nodes:    {}", forest.nodes().len());
        println!("meshes:   {}", scene.meshes().len());
        let mut total_verts = 0;
        let mut total_tris = 0;
        let mut total_ranges = 0;
        for mesh in scene.meshes() {
            total_verts += mesh.vertices().len();
            total_tris += mesh.triangles().len();
            total_ranges += mesh.faces().len();
        }
        println!("verts:    {total_verts}");
        println!("tris:     {total_tris}");
        println!("ranges:   {total_ranges}");
        println!(
            "colors:   {}",
            scene
                .meshes()
                .first()
                .map_or(0, |m| m.colors().map_or(0, |c| c.len()))
        );
        if args.names {
            for (index, node) in forest.nodes().iter().enumerate() {
                let name = String::from_utf8_lossy(forest.node_name(node));
                println!(
                    "node {index} parent={} mesh={mesh}: {name}",
                    node.parent,
                    mesh = node.mesh
                );
            }
        }
        if args.unity {
            let projected = project_scene(forest, scene.meshes(), ProjectionSettings::default())?;
            println!("u-nodes:  {}", projected.nodes().len());
            println!("u-meshes: {}", projected.meshes().len());
            let mut u_verts = 0;
            let mut u_tris = 0;
            let mut u_ranges = 0;
            for mesh in projected.meshes() {
                u_verts += mesh.vertices().len();
                u_tris += mesh.indices().len() / 3;
                u_ranges += mesh.submeshes().len();
            }
            println!("u-verts:  {u_verts}");
            println!("u-tris:   {u_tris}");
            println!("u-ranges: {u_ranges}");
        }
    }
    Ok(())
}
