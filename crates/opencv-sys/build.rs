use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

// Native-build driver for the ghcv shim, a compact clone of the
// crates/occt-sys pattern: config.toml is the sole input source, the
// shim is a plain C++ shared library compiled by the section's
// toolchain (msvc: cl.exe driving link.exe; gnu: the cc crate's
// compiler driving the linker), and the artifact lands in the
// vision package's shim plugins directory inside the staged package
// layout, separate from the third-party OpenCV tree it bridges. The
// windows section is exercised (bite V1); the
// linux/macos sections and their gnu arm are pre-populated and wait
// for their first run on a target machine.

struct Config {
    toolchain: Toolchain,
    include: PathBuf,
    lib: PathBuf,
    shim_dir: PathBuf,
    shim_filename: String,
    import_pattern: String,
    modules: Vec<String>,
}

enum Toolchain {
    Gnu,
    Msvc,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = manifest_dir()?;
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        manifest.join("config.toml").display()
    );
    for name in ["CXX", "CXXFLAGS"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let config = platform_config(&manifest)?;
    build_native_shim(&manifest, &config)?;
    Ok(())
}

fn manifest_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| io::Error::other("missing CARGO_MANIFEST_DIR"))?,
    ))
}

fn out_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("missing OUT_DIR"))?,
    ))
}

fn platform_config(manifest: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    // The shim is loaded by processes of the machine building it (the C#
    // harness, later the Unity editor); cross-compilation has no story
    // here, same as occt-sys.
    if env::var("HOST")? != env::var("TARGET")? {
        return Err(io::Error::other("native shim requires host == target").into());
    }
    let os = env::var("CARGO_CFG_TARGET_OS")?;
    // config.toml paths resolve against the repository root — two levels
    // above this crate directory (crates/opencv-sys) — so every checkout
    // on every platform parses identical values.
    let repo_root = manifest.parent().and_then(Path::parent).ok_or_else(|| {
        io::Error::other("cannot locate the repository root above crates/opencv-sys")
    })?;
    let text = fs::read_to_string(manifest.join("config.toml")).map_err(|error| {
        io::Error::other(format!("cannot read opencv-sys config.toml: {error}"))
    })?;
    let table: toml::Table = toml::from_str(&text)
        .map_err(|error| io::Error::other(format!("invalid opencv-sys config.toml: {error}")))?;
    let section = table
        .get(&os)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            io::Error::other(format!(
                "opencv-sys config.toml has no [{os}] section; \
                 supported target OS values: linux, windows, macos"
            ))
        })?;
    let string = |key: &str| -> Result<String, Box<dyn std::error::Error>> {
        section
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "opencv-sys config.toml [{os}] is missing the string key {key}"
                ))
                .into()
            })
    };
    let toolchain = match string("toolchain")?.as_str() {
        "gnu" => Toolchain::Gnu,
        "msvc" => Toolchain::Msvc,
        other => {
            return Err(io::Error::other(format!(
                "opencv-sys config.toml names unknown toolchain {other}; \
                 supported: gnu, msvc"
            ))
            .into());
        }
    };
    let modules = section
        .get("modules")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| io::Error::other("opencv-sys config.toml is missing the modules array"))?
        .iter()
        .map(|module| {
            module
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| io::Error::other("opencv-sys config.toml modules must be strings"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if modules.is_empty() {
        return Err(io::Error::other("opencv-sys config.toml modules is empty").into());
    }
    let prefix = repo_root.join(string("opencv_prefix")?);
    Ok(Config {
        toolchain,
        include: prefix.join(string("include_dir")?),
        lib: prefix.join(string("lib_dir")?),
        shim_dir: repo_root.join(string("shim_dir")?),
        shim_filename: string("shim_filename")?,
        import_pattern: string("import_pattern")?,
        modules,
    })
}

impl Config {
    // The CARGO_CFG_TARGET_OS value the section was matched on; the
    // gnu flag tables branch on it (linux/macos), mirroring occt-sys.
    fn toolchain_os(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(env::var("CARGO_CFG_TARGET_OS")?)
    }
}

fn build_native_shim(manifest: &Path, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let header = config.include.join("opencv2/objdetect/aruco_detector.hpp");
    if !header.is_file() {
        return Err(
            io::Error::other(format!("missing OpenCV headers: {}", header.display())).into(),
        );
    }
    println!("cargo:rerun-if-changed={}", config.include.display());
    let mut import_libs = Vec::new();
    for module in &config.modules {
        let file = config
            .lib
            .join(config.import_pattern.replace("{module}", module));
        if !file.is_file() {
            return Err(
                io::Error::other(format!("missing OpenCV library: {}", file.display())).into(),
            );
        }
        println!("cargo:rerun-if-changed={}", file.display());
        // The link step receives the FINAL import-library filename from
        // the config's pattern (e.g. opencv_core500.lib), not the module
        // name — the pattern is the single source of the naming.
        import_libs.push(config.import_pattern.replace("{module}", module));
    }
    let cpp = manifest.join("cpp");
    let source = cpp.join("detect.cpp");
    println!("cargo:rerun-if-changed={}", source.display());
    let output = config.shim_dir.join(&config.shim_filename);
    let obj_dir = out_dir()?.join("shim-obj");
    match config.toolchain {
        Toolchain::Msvc => {
            compile_msvc(
                &output,
                &source,
                &config.include,
                &config.lib,
                &import_libs,
                &obj_dir,
            )?;
        }
        Toolchain::Gnu => {
            // linux/macos: pre-populated config sections, NOT exercised
            // yet — this arm follows the occt-sys gnu pattern (shared
            // flags + rpath + soname) and will need its first real run
            // on the target machine to shake out loader-path details
            // (the OpenCV archives' transitive codec deps).
            let (shared_flags, ld_flags, name_flags) =
                gnu_flags(config, output.file_name(), &import_libs)?;
            compile_gnu(
                &output,
                std::slice::from_ref(&source),
                std::slice::from_ref(&config.include),
                &shared_flags,
                &ld_flags,
                &name_flags,
            )?;
        }
    }
    println!("cargo:warning=Shared shim built: {}", output.display());
    Ok(())
}

// The per-OS gnu flag groups in the order they reach the compiler
// driver: shared flags, linker-driver flags, then the output-name
// flags.
type GnuFlags = (
    Vec<std::ffi::OsString>,
    Vec<std::ffi::OsString>,
    Vec<std::ffi::OsString>,
);

// Per-OS flags for the gnu toolchain, in the exact order they reach the
// compiler driver (same arrangement as occt-sys's gnu arms): shared
// flags, then sources, then linker-driver flags. Libraries follow the
// sources (-L then the module .so/.dylib files).
fn gnu_flags(
    config: &Config,
    name: Option<&std::ffi::OsStr>,
    import_libs: &[String],
) -> Result<GnuFlags, Box<dyn std::error::Error>> {
    let name = name
        .ok_or_else(|| io::Error::other("missing shim filename"))?
        .to_string_lossy()
        .into_owned();
    match config.toolchain_os()?.as_str() {
        "linux" => Ok((
            vec![
                std::ffi::OsString::from("-shared"),
                std::ffi::OsString::from("-fPIC"),
                std::ffi::OsString::from("-fexceptions"),
            ],
            vec![std::ffi::OsString::from(
                "-Wl,--disable-new-dtags,-rpath,$ORIGIN",
            )],
            vec![
                std::ffi::OsString::from("-Xlinker"),
                std::ffi::OsString::from("-soname"),
                std::ffi::OsString::from("-Xlinker"),
                std::ffi::OsString::from(name),
            ],
        )),
        "macos" => Ok((
            vec![
                std::ffi::OsString::from("-dynamiclib"),
                std::ffi::OsString::from("-fPIC"),
                std::ffi::OsString::from("-fexceptions"),
            ],
            vec![std::ffi::OsString::from("-Wl,-rpath,@loader_path")],
            vec![std::ffi::OsString::from(format!(
                "-Wl,-install_name,@rpath/{name}"
            ))],
        )),
        other => Err(io::Error::other(format!(
            "opencv-sys config.toml sets toolchain gnu for target OS {other}; \
             gnu has flag tables only for linux and macos"
        ))
        .into()),
    }
    .map(|(shared, ld, name_flags)| {
        // Library search path then the module libraries themselves —
        // passed as full paths so the pattern's exact filename governs.
        let mut ld = ld;
        ld.push(std::ffi::OsString::from(format!(
            "-L{}",
            config.lib.display()
        )));
        for lib in import_libs {
            ld.push(std::ffi::OsString::from(
                config.lib.join(lib).to_string_lossy().into_owned(),
            ));
        }
        (shared, ld, name_flags)
    })
}

// The gnu arm: the compiler driver takes shared flags, sources,
// linker-driver flags, then the output — one invocation, the same
// arrangement as occt-sys's compile_gnu.
fn compile_gnu(
    output: &Path,
    sources: &[PathBuf],
    includes: &[PathBuf],
    shared_flags: &[std::ffi::OsString],
    ld_flags: &[std::ffi::OsString],
    name_flags: &[std::ffi::OsString],
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = output
        .parent()
        .ok_or_else(|| io::Error::other("missing output directory"))?;
    fs::create_dir_all(parent)?;
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").pic(true).cargo_metadata(false);
    for include in includes {
        build.include(include);
    }
    let tool = build.try_get_compiler()?;
    let mut command = tool.to_command();
    command.args(shared_flags);
    command.args(sources);
    command.args(ld_flags);
    command.args(name_flags);
    command.arg("-o").arg(output);
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

// cl.exe drives link.exe in one invocation — the same arrangement as
// occt-sys: /LD marks the image as a DLL, /MD matches the release CRT the
// prebuilt OpenCV binaries are built against (mixing CRT flavors across a
// DLL boundary is how heaps corrupt), /EHsc because OpenCV throws, and
// the import library and .pdb are kept in the object directory, not the
// payload.
fn compile_msvc(
    output: &Path,
    source: &Path,
    include: &Path,
    lib_dir: &Path,
    import_libs: &[String],
    obj_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = output
        .parent()
        .ok_or_else(|| io::Error::other("missing output directory"))?;
    fs::create_dir_all(parent)?;
    fs::create_dir_all(obj_dir)?;
    // cc locates cl.exe and configures the MSVC/Windows SDK environment
    // (INCLUDE/LIB) for the child process; if that lookup fails, run
    // cargo from an x64 Native Tools prompt.
    let mut build = cc::Build::new();
    build.cpp(true);
    let tool = build.try_get_compiler().map_err(|error| {
        io::Error::other(format!(
            "cannot locate an MSVC toolchain for the shim \
             (install VS Build Tools with Desktop C++, or run from an x64 \
             Native Tools prompt): {error}"
        ))
    })?;
    let mut command = tool.to_command();
    // cl and link accept forward slashes, which avoids quoting a trailing
    // backslash against the closing quote of a Windows argument.
    command.args(["/nologo", "/EHsc", "/std:c++17", "/MD", "/LD"]);
    command.arg(format!("/I{}", fwd(include)));
    command.arg(fwd(source));
    command.arg(format!("/Fe{}", fwd(output)));
    command.arg(format!("/Fo{}/", fwd(obj_dir)));
    command.args(["/link", "/INCREMENTAL:NO"]);
    let stem = output
        .file_stem()
        .ok_or_else(|| io::Error::other("missing shim file stem"))?
        .to_string_lossy();
    command.arg(format!("/PDB:{}/{}.pdb", fwd(obj_dir), stem));
    command.arg(format!("/IMPLIB:{}/{}.lib", fwd(obj_dir), stem));
    command.arg(format!("/LIBPATH:{}", fwd(lib_dir)));
    for lib in import_libs {
        command.arg(lib);
    }
    let result = command.output()?;
    if !result.status.success() {
        return Err(io::Error::other(format!(
            "shim DLL compilation failed: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr),
        ))
        .into());
    }
    Ok(())
}

// cl/link accept forward-slash paths everywhere; normalizing avoids the
// Windows argument-quoting trap where a trailing backslash escapes the
// closing quote.
fn fwd(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
