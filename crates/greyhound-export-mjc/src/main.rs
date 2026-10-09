mod app_config;
mod args;
mod assemble;
mod config_error;
mod error;
mod export;
mod mjcf;
mod model;
mod naming;
mod occt_config;
mod quat;
mod stl;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

use args::Args;

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
    let options = export::ExportOptions {
        config_path: args.config,
        step_path: args.path,
        out_dir: args.out,
        density: args.density,
        free_root: args.free_root,
        scale: args.scale,
    };
    let summary = export::export(&options)?;

    println!("model:   {}", summary.model_path.display());
    println!("bodies:  {}", summary.body_count);
    println!("meshes:  {}", summary.mesh_count);
    println!("geoms:   {}", summary.geom_count);
    println!("mass kg: {:.6}", summary.total_mass_kg);
    Ok(())
}
