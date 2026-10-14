use std::{
    env,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    slice,
};

// The native-build configuration: everything build.rs consumes, selected
// per target OS from the crate-local config.toml.
struct PlatformConfig {
    os: String,
    toolchain: Toolchain,
    // OCCT install root supplying the build-time inputs: headers under
    // include, link libraries under lib.
    include: PathBuf,
    lib: PathBuf,
    // The shim's own plugins directory inside the package layout.
    shim_dir: PathBuf,
    // Filename patterns; {tk} is replaced by the toolkit name.
    toolkit_pattern: String,
    runtime_pattern: String,
    shim_filename: String,
    // MSVC: link every TK*.lib in lib_dir instead of the named list.
    link_all_toolkits: bool,
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
    build_loader_fixtures(&manifest, &config)?;
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

fn platform_config(manifest: &Path) -> Result<PlatformConfig, Box<dyn std::error::Error>> {
    // The shim is loaded by processes of the machine building it (the Unity
    // editor, the loader tests); cross-compilation has no story here.
    if env::var("HOST")? != env::var("TARGET")? {
        return Err(io::Error::other("native shim requires host == target").into());
    }
    let os = env::var("CARGO_CFG_TARGET_OS")?;
    // config.toml paths resolve against the repository root — two levels
    // above this crate directory (crates/occt-sys) — so every checkout on
    // every platform parses identical values.
    let repo_root = manifest.parent().and_then(Path::parent).ok_or_else(|| {
        io::Error::other("cannot locate the repository root above crates/occt-sys")
    })?;
    let text = fs::read_to_string(manifest.join("config.toml"))
        .map_err(|error| io::Error::other(format!("cannot read occt-sys config.toml: {error}")))?;
    let table: toml::Table = toml::from_str(&text)
        .map_err(|error| io::Error::other(format!("invalid occt-sys config.toml: {error}")))?;
    let section = table
        .get(&os)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            io::Error::other(format!(
                "occt-sys config.toml has no [{os}] section; \
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
                    "occt-sys config.toml [{os}] is missing the string key {key}"
                ))
                .into()
            })
    };
    let toolchain = match string("toolchain")?.as_str() {
        "gnu" => Toolchain::Gnu,
        "msvc" => Toolchain::Msvc,
        other => {
            return Err(io::Error::other(format!(
                "occt-sys config.toml [{os}] names unknown toolchain {other}; \
                 supported: gnu, msvc"
            ))
            .into());
        }
    };
    let prefix_value = string("occt_prefix")?;
    Ok(PlatformConfig {
        // Cloned: the `string` closure still borrows `os` for the calls
        // below, and the config keeps the value for toolchain dispatch.
        os: os.clone(),
        toolchain,
        include: repo_root.join(&prefix_value).join(string("include_dir")?),
        lib: repo_root.join(&prefix_value).join(string("lib_dir")?),
        shim_dir: repo_root.join(string("shim_dir")?),
        toolkit_pattern: string("toolkit_pattern")?,
        runtime_pattern: string("runtime_pattern")?,
        shim_filename: string("shim_filename")?,
        link_all_toolkits: section
            .get("link_all_toolkits")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false),
    })
}

// OCCT toolkits the shim is built against (the XCAF pair carry the
// color/assembly document layer).
const TOOLKITS: [&str; 11] = [
    "TKernel",
    "TKMath",
    "TKGeomBase",
    "TKBRep",
    "TKPrim",
    "TKTopAlgo",
    "TKMesh",
    "TKXSBase",
    "TKDESTEP",
    "TKLCAF",
    "TKXCAF",
];

// The fixture's exported entry points. MSVC fixtures export through
// explicit /EXPORT directives (see build_loader_fixtures).
const ABI_EXPORTS: [&str; 12] = [
    "greyhound_abi_version",
    "greyhound_last_error",
    "greyhound_box_volume",
    "greyhound_step_open",
    "greyhound_step_info",
    "greyhound_scene_counts",
    "greyhound_scene_fill",
    "greyhound_color_fill",
    "greyhound_mesh_properties",
    "greyhound_mesh_counts",
    "greyhound_mesh_fill",
    "greyhound_step_close",
];

fn build_native_shim(
    manifest: &Path,
    config: &PlatformConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let version_header = config.include.join("Standard_Version.hxx");
    if !version_header.is_file() {
        return Err(io::Error::other(format!(
            "missing OCCT headers: {}",
            version_header.display()
        ))
        .into());
    }
    for toolkit in TOOLKITS {
        let file = config
            .lib
            .join(config.toolkit_pattern.replace("{tk}", toolkit));
        if !file.is_file() {
            return Err(
                io::Error::other(format!("missing OCCT library: {}", file.display())).into(),
            );
        }
        println!("cargo:rerun-if-changed={}", file.display());
    }
    println!("cargo:rerun-if-changed={}", config.include.display());
    let cpp = manifest.join("cpp");
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
    let output = config.shim_dir.join(&config.shim_filename);
    // The runtime toolkit filename pattern and the shim artifact name
    // reach the loader and its tests through the compiled environment:
    // single source of truth for per-OS naming.
    println!(
        "cargo:rustc-env=OCCT_RUNTIME_PATTERN={}",
        config.runtime_pattern
    );
    println!(
        "cargo:rustc-env=OCCT_SHIM_FILENAME={}",
        config.shim_filename
    );
    match config.toolchain {
        Toolchain::Gnu => {
            let (shared_flags, ld_flags, name_flags) = gnu_flags(config, output.file_name())?;
            let link_args = gnu_link_args(config, &TOOLKITS)?;
            compile_gnu(
                &output,
                &sources,
                &[config.include.clone(), cpp.clone()],
                &shared_flags,
                &ld_flags,
                &name_flags,
                &link_args,
            )?;
        }
        Toolchain::Msvc => {
            let toolkit_libs = if config.link_all_toolkits {
                msvc_link_libs(config)?
            } else {
                TOOLKITS.iter().map(|toolkit| toolkit.to_string()).collect()
            };
            let obj_dir = out_dir()?.join("shim-obj");
            compile_msvc(
                &output,
                &sources,
                &[config.include.clone(), cpp.clone()],
                &[],
                &MsvcLink {
                    args: &[],
                    lib_dir: &config.lib,
                    toolkit_libs: &toolkit_libs,
                },
                &obj_dir,
            )?;
        }
    }
    println!("cargo:warning=Shared shim built: {}", output.display());
    Ok(())
}

// The per-OS gnu flag groups in the order they reach the compiler driver:
// shared flags, linker-driver flags, then the output-name flags.
type GnuFlags = (Vec<OsString>, Vec<OsString>, Vec<OsString>);

// Per-OS flags for the gnu toolchain, in the exact order they reach the
// compiler driver: shared flags, then sources, then linker-driver flags.
// The linux arm is byte-for-byte the flag set this build script has used
// all along; the macos arm is unexercised (AGENTS.md).
fn gnu_flags(
    config: &PlatformConfig,
    name: Option<&OsStr>,
) -> Result<GnuFlags, Box<dyn std::error::Error>> {
    let name = name
        .ok_or_else(|| io::Error::other("missing shim filename"))?
        .to_string_lossy()
        .into_owned();
    match config.os.as_str() {
        "linux" => Ok((
            vec![
                OsString::from("-shared"),
                OsString::from("-fPIC"),
                OsString::from("-fexceptions"),
            ],
            vec![OsString::from("-Wl,--disable-new-dtags,-rpath,$ORIGIN")],
            vec![
                OsString::from("-Xlinker"),
                OsString::from("-soname"),
                OsString::from("-Xlinker"),
                OsString::from(name),
            ],
        )),
        "macos" => Ok((
            vec![
                OsString::from("-dynamiclib"),
                OsString::from("-fPIC"),
                OsString::from("-fexceptions"),
            ],
            vec![OsString::from("-Wl,-rpath,@loader_path")],
            vec![OsString::from(format!("-Wl,-install_name,@rpath/{name}"))],
        )),
        other => Err(io::Error::other(format!(
            "occt-sys config.toml sets toolchain gnu for target OS {other}; \
             gnu has flag tables only for linux and macos"
        ))
        .into()),
    }
}

// Link arguments after the sources: library search paths first, then the
// toolkit libraries. Linux adds -rpath-link so the linker can resolve
// symbols through the toolkits' own DT_NEEDED dependencies.
fn gnu_link_args(
    config: &PlatformConfig,
    toolkits: &[&str],
) -> Result<Vec<OsString>, Box<dyn std::error::Error>> {
    let mut args = vec![OsString::from("-L"), config.lib.as_os_str().to_owned()];
    match config.os.as_str() {
        "linux" => {
            args.push(OsString::from("-Xlinker"));
            args.push(OsString::from("-rpath-link"));
            args.push(OsString::from("-Xlinker"));
            args.push(config.lib.as_os_str().to_owned());
        }
        "macos" => {}
        other => {
            return Err(io::Error::other(format!(
                "occt-sys config.toml sets toolchain gnu for target OS {other}; \
                 gnu has flag tables only for linux and macos"
            ))
            .into());
        }
    }
    args.extend(
        toolkits
            .iter()
            .map(|name| OsString::from(format!("-l{name}"))),
    );
    Ok(args)
}

fn compile_gnu(
    output: &Path,
    sources: &[PathBuf],
    includes: &[PathBuf],
    shared_flags: &[OsString],
    ld_flags: &[OsString],
    name_flags: &[OsString],
    link_args: &[OsString],
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
    command.args(link_args);
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

// Everything the /link section receives beyond the compiled objects.
struct MsvcLink<'a> {
    // /EXPORT directives etc.
    args: &'a [OsString],
    lib_dir: &'a Path,
    toolkit_libs: &'a [String],
}

// cl.exe drives link.exe in one invocation: /LD marks the image as a DLL,
// /MD matches the CRT the release vc14 OCCT binaries are built against
// (mixing CRT flavors across a DLL boundary is how heaps corrupt),
// /EHsc because OCCT throws, and the import library and .exp are kept in
// the object directory instead of the package payload.
fn compile_msvc(
    output: &Path,
    sources: &[PathBuf],
    includes: &[PathBuf],
    definitions: &[OsString],
    link: &MsvcLink,
    obj_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = output
        .parent()
        .ok_or_else(|| io::Error::other("missing output directory"))?;
    fs::create_dir_all(parent)?;
    fs::create_dir_all(obj_dir)?;
    // cc locates cl.exe and configures the MSVC/Windows SDK environment
    // (INCLUDE/LIB) for the child process; if that lookup fails, run cargo
    // from an x64 Native Tools prompt.
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
    for include in includes {
        command.arg(format!("/I{}", fwd(include)));
    }
    command.args(definitions);
    for source in sources {
        command.arg(fwd(source));
    }
    command.arg(format!("/Fe{}", fwd(output)));
    command.arg(format!("/Fo{}/", fwd(obj_dir)));
    command.args(["/link"]);
    // cl links incrementally by default, which writes .ilk and a PDB
    // beside the output — the package payload carries the DLL and nothing
    // else.
    command.arg("/INCREMENTAL:NO");
    let stem = output
        .file_stem()
        .ok_or_else(|| io::Error::other("missing shim file stem"))?
        .to_string_lossy();
    command.arg(format!("/PDB:{}/{}.pdb", fwd(obj_dir), stem));
    command.arg(format!("/IMPLIB:{}/{}.lib", fwd(obj_dir), stem));
    command.args(link.args);
    command.arg(format!("/LIBPATH:{}", fwd(link.lib_dir)));
    for lib in link.toolkit_libs {
        command.arg(format!("{lib}.lib"));
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

// MSVC on the Windows section links every TK*.lib in lib_dir: link.exe
// resolves only the .libs it is handed (no transitive search through a
// DLL's own dependencies), while an unused import library contributes no
// DLL dependency, so the import closure is exactly what the shim uses.
fn msvc_link_libs(config: &PlatformConfig) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut libs = Vec::new();
    for entry in fs::read_dir(&config.lib)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("lib")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && stem.starts_with("TK")
        {
            libs.push(stem.to_owned());
        }
    }
    if libs.is_empty() {
        return Err(io::Error::other(format!(
            "no TK*.lib import libraries in {}",
            config.lib.display()
        ))
        .into());
    }
    libs.sort();
    Ok(libs)
}

fn build_loader_fixtures(
    manifest: &Path,
    config: &PlatformConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let cpp = manifest.join("cpp");
    let source = cpp.join("tests/loader_fixture.cpp");
    println!("cargo:rerun-if-changed={}", source.display());
    println!(
        "cargo:rerun-if-changed={}",
        cpp.join("greyhound_abi.h").display()
    );
    let out = out_dir()?;
    let fixtures = out.join("fixtures");
    let cases: &[(&str, &[&str])] = &[
        ("valid", &[]),
        ("bad-abi", &["-DFIXTURE_BAD_ABI"]),
        ("missing-symbol", &["-DFIXTURE_MISSING_FILL"]),
        ("open-failure", &["-DFIXTURE_OPEN_FAIL"]),
    ];
    for (name, definitions) in cases {
        let directory = fixtures.join(name);
        // The fixture neutralizes the header's export attribute with this
        // definition, so MSVC exports come solely from the /EXPORT
        // directives below: the missing-symbol case must genuinely lack
        // its symbol, which declaration-driven dllexport cannot express
        // (MSVC emits the export despite the absent definition).
        let mut args: Vec<OsString> = vec![OsString::from("-DGREYHOUND_API=")];
        args.extend(
            definitions
                .iter()
                .map(|definition| OsString::from(*definition)),
        );
        let mut exports: Vec<&str> = ABI_EXPORTS.to_vec();
        if definitions.contains(&"-DFIXTURE_MISSING_FILL") {
            let index = exports
                .iter()
                .position(|entry| *entry == "greyhound_mesh_fill")
                .ok_or_else(|| {
                    io::Error::other("greyhound_mesh_fill missing from the export list")
                })?;
            exports.remove(index);
        }
        let link_args: Vec<OsString> = exports
            .iter()
            .map(|entry| OsString::from(format!("/EXPORT:{entry}")))
            .collect();
        // A fixture-owned stub satisfies the same file-presence check the
        // runtime library name must satisfy; it is not an OCCT binary and
        // is not copied into the real dist.
        let shim = directory.join(&config.shim_filename);
        let stub = directory.join(config.runtime_pattern.replace("{tk}", "TKernel"));
        match config.toolchain {
            Toolchain::Gnu => {
                let (shared_flags, ld_flags, name_flags) = gnu_flags(config, shim.file_name())?;
                compile_gnu(
                    &shim,
                    slice::from_ref(&source),
                    slice::from_ref(&cpp),
                    &shared_flags,
                    &ld_flags,
                    &name_flags,
                    &args,
                )?;
                let (shared_flags, ld_flags, name_flags) = gnu_flags(config, stub.file_name())?;
                compile_gnu(
                    &stub,
                    slice::from_ref(&source),
                    slice::from_ref(&cpp),
                    &shared_flags,
                    &ld_flags,
                    &name_flags,
                    &args,
                )?;
            }
            Toolchain::Msvc => {
                let obj_dir = out.join(format!("fixture-{name}-obj"));
                let link = MsvcLink {
                    args: &link_args,
                    lib_dir: &config.lib,
                    toolkit_libs: &[],
                };
                compile_msvc(
                    &shim,
                    slice::from_ref(&source),
                    slice::from_ref(&cpp),
                    &args,
                    &link,
                    &obj_dir,
                )?;
                compile_msvc(
                    &stub,
                    slice::from_ref(&source),
                    slice::from_ref(&cpp),
                    &args,
                    &link,
                    &obj_dir,
                )?;
            }
        }
    }
    // Compiled into feature-gated tests only; never a runtime loader fallback.
    println!(
        "cargo:rustc-env=OCCT_TEST_FIXTURE_DIR={}",
        fixtures.display()
    );
    Ok(())
}
