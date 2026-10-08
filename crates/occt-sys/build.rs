fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    build_native_shim()?;
    build_loader_fixtures()?;
    Ok(())
}

fn compile_shared(
    output: &std::path::Path,
    sources: &[std::path::PathBuf],
    includes: &[std::path::PathBuf],
    link_args: &[std::ffi::OsString],
) -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io};

    if env::var("CARGO_CFG_TARGET_OS")? != "linux"
        || env::var("CARGO_CFG_TARGET_ARCH")? != "x86_64"
        || env::var("HOST")? != env::var("TARGET")?
    {
        return Err(io::Error::other("VS-20 native build requires native Linux x86_64").into());
    }
    let parent = output
        .parent()
        .ok_or_else(|| io::Error::other("missing output directory"))?;
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .ok_or_else(|| io::Error::other("missing library filename"))?;

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").pic(true).cargo_metadata(false);
    for include in includes {
        build.include(include);
    }
    let tool = build.try_get_compiler()?;
    let mut command = tool.to_command();
    command.args(["-shared", "-fPIC", "-fexceptions"]);
    command.args(sources);
    command.arg("-Wl,--disable-new-dtags,-rpath,$ORIGIN");
    command.args(["-Xlinker", "-soname", "-Xlinker"]).arg(name);
    command.args(link_args).arg("-o").arg(output);
    let result = command.output()?;
    if !result.status.success() {
        return Err(io::Error::other(format!(
            "shared shim compilation failed: {}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr),
        ))
        .into());
    }
    Ok(())
}

fn build_native_shim() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, ffi::OsString, io, path::PathBuf};

    for name in ["OCCT_PREFIX", "OCCT_SHIM_DIR", "CXX", "CXXFLAGS"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let prefix =
        PathBuf::from(env::var_os("OCCT_PREFIX").ok_or_else(|| {
            io::Error::other("build-native-shim requires build-time OCCT_PREFIX")
        })?);
    let shim_dir =
        PathBuf::from(env::var_os("OCCT_SHIM_DIR").ok_or_else(|| {
            io::Error::other("build-native-shim requires build-time OCCT_SHIM_DIR")
        })?);
    let include = prefix.join("include/opencascade");
    let lib = prefix.join("lib");
    if !include.join("Standard_Version.hxx").is_file() {
        return Err(
            io::Error::other(format!("missing OCCT headers: {}", include.display())).into(),
        );
    }
    let toolkits = [
        "TKernel",
        "TKMath",
        "TKGeomBase",
        "TKBRep",
        "TKPrim",
        "TKTopAlgo",
        "TKMesh",
        "TKXSBase",
        "TKDESTEP",
    ];
    for toolkit in toolkits {
        let file = lib.join(format!("lib{toolkit}.so"));
        if !file.is_file() {
            return Err(
                io::Error::other(format!("missing OCCT library: {}", file.display())).into(),
            );
        }
        println!("cargo:rerun-if-changed={}", file.display());
    }
    println!("cargo:rerun-if-changed={}", include.display());
    let root = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| io::Error::other("missing CARGO_MANIFEST_DIR"))?,
    );
    let cpp = root.join("cpp");
    let sources = [
        cpp.join("abi.cpp"),
        cpp.join("box_volume.cpp"),
        cpp.join("step_mesh.cpp"),
    ];
    for source in &sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    for header in ["greyhound_abi.h", "native_guard.h"] {
        println!("cargo:rerun-if-changed={}", cpp.join(header).display());
    }
    // The shim compiles into its own plugins directory, separate from the
    // third-party OCCT toolkit tree it bridges.
    let output = shim_dir.join("libgreyhound_occt.so");
    let mut args = vec![
        OsString::from("-L"),
        lib.as_os_str().to_owned(),
        OsString::from("-Xlinker"),
        OsString::from("-rpath-link"),
        OsString::from("-Xlinker"),
        lib.as_os_str().to_owned(),
    ];
    args.extend(
        toolkits
            .iter()
            .map(|name| OsString::from(format!("-l{name}"))),
    );
    compile_shared(&output, &sources, &[include, cpp], &args)?;
    println!("cargo:warning=Shared shim built: {}", output.display());
    Ok(())
}

fn build_loader_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, ffi::OsString, io, path::PathBuf};

    let root = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| io::Error::other("missing CARGO_MANIFEST_DIR"))?,
    );
    let cpp = root.join("cpp");
    let source = cpp.join("tests/loader_fixture.cpp");
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        cpp.join("greyhound_abi.h").display()
    );
    let fixtures =
        PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("missing OUT_DIR"))?)
            .join("fixtures");
    let cases: &[(&str, &[&str])] = &[
        ("valid", &[]),
        ("bad-abi", &["-DFIXTURE_BAD_ABI"]),
        ("missing-symbol", &["-DFIXTURE_MISSING_FILL"]),
        ("open-failure", &["-DFIXTURE_OPEN_FAIL"]),
    ];
    for (name, definitions) in cases {
        let directory = fixtures.join(name);
        let args: Vec<OsString> = definitions
            .iter()
            .map(|definition| OsString::from(*definition))
            .collect();
        compile_shared(
            &directory.join("libgreyhound_occt.so"),
            std::slice::from_ref(&source),
            std::slice::from_ref(&cpp),
            &args,
        )?;
        // A fixture-owned stub satisfies the same file-presence check;
        // it is not an OCCT binary and is not copied into the real dist.
        compile_shared(
            &directory.join("libTKernel.so"),
            std::slice::from_ref(&source),
            std::slice::from_ref(&cpp),
            &args,
        )?;
    }
    // Compiled into feature-gated tests only; never a runtime loader fallback.
    println!(
        "cargo:rustc-env=OCCT_TEST_FIXTURE_DIR={}",
        fixtures.display()
    );
    Ok(())
}
