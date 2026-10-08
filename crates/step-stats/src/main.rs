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
    if args.mesh {
        let mesh = doc.mesh(0.01, 0.5)?;
        println!("verts:    {}", mesh.vertices().len());
        println!("tris:     {}", mesh.triangles().len());
        println!("ranges:   {}", mesh.faces().len());
    }
    Ok(())
}
