# Building Open CASCADE Technology on Linux for the Unity STEP importer

This repository is early groundwork for a **Unity Scripted Importer that uses
OCCT to import STEP files**. The importer will ship as a **Unity Package
Manager (UPM) package containing the required OCCT native binaries**. Unity
users should not need a separate OCCT installation.

These are instructions for you to run on an **x86_64 Linux** build machine.
They build **OCCT 8.0.1** (tag `V8.0.1`) from source, with a small CMake change
to produce unversioned shared libraries without symlinks. The install goes
into `dist/occt/x86_64-linux`, which the existing Rust/C++ integration uses for
development. Building this dist does not by itself create or validate the
Unity importer or its UPM package.

- Repo: <https://github.com/Open-Cascade-SAS/OCCT>
- Tag: <https://github.com/Open-Cascade-SAS/OCCT/releases/tag/V8.0.1>
- License: LGPL-2.1 with exception

## Prerequisites

- A compiler with C++17 support (GCC or Clang); the package commands below
  install the distribution's GCC toolchain.
- CMake 3.16+ for the commands in this guide, Ninja, Git, and binutils
  (`readelf` for dependency inspection). OCCT itself declares CMake 3.10 as
  its minimum, but this guide uses newer CMake command-line features.
- The build below includes Visualization and the DRAW test harness, so
  install **FreeType**, **Tcl/Tk**, X11, and desktop OpenGL development
  packages. These build-machine dependencies are not instructions for
  users of the eventual Unity package.

Build on the oldest Linux environment you intend to support for the Unity
Editor. A build on a newer distribution can require newer glibc or
`libstdc++` symbols than an older target provides.

Ubuntu 22.04/24.04, Debian 12+:

```
sudo apt install build-essential git cmake ninja-build binutils \
     libfreetype-dev tcl-dev tk-dev \
     libx11-dev libxext-dev libxft-dev libxrender-dev libxi-dev libxmu-dev \
     libgl1-mesa-dev libglu1-mesa-dev
```

Optional, only if you enable the matching flags later:

```
sudo apt install libtbb-dev libfreeimage-dev rapidjson-dev \
     libdraco-dev libvtk9-dev libeigen3-dev libjemalloc-dev
# USE_TBB, USE_FREEIMAGE, USE_RAPIDJSON, USE_DRACO, USE_VTK, USE_EIGEN,
# USE_MMGR_TYPE=JEMALLOC
```

The optional boolean flags below default to OFF; the default memory manager
is `USE_MMGR_TYPE=NATIVE`:

| Package | Flag | What it enables |
|---|---|---|
| `libtbb-dev` | `USE_TBB` | TBB-backed parallel computations. OCCT also has native parallel facilities when TBB is disabled. |
| `libfreeimage-dev` | `USE_FREEIMAGE` | FreeImage in the Visualization module — reading/writing popular image formats (PNG, BMP, ...) for screenshots/view dumps and textures. |
| `rapidjson-dev` | `USE_RAPIDJSON` | RapidJSON in Data Exchange — JSON support for the glTF reader/writer (TKDEGLTF). Without it the glTF toolkits are skipped entirely. |
| `libdraco-dev` | `USE_DRACO` | Draco mesh decoding for the glTF reader (`KHR_draco_mesh_compression`). Needs `USE_RAPIDJSON`. |
| `libvtk9-dev` | `USE_VTK` | VTK bridge (TKIVtk) — push OCCT shapes into VTK for scientific-visualization pipelines. |
| `libeigen3-dev` | `USE_EIGEN` | Eigen linear algebra for a few internal routines; header-only, adds an include path — nothing is linked. |
| `libjemalloc-dev` | `USE_MMGR_TYPE=JEMALLOC` | jemalloc as OCCT's internal memory manager (TKernel). Default is `NATIVE`; alternatives are `FLEXIBLE` and `TBB`. |

(`libegl-dev`/`libgles-dev` are the deps for `USE_GLES2` — see below.)

Fedora/RHEL equivalent:

```
sudo dnf install gcc-c++ cmake ninja-build git binutils freetype-devel tcl-devel \
     tk-devel libX11-devel libXext-devel libXft-devel libXi-devel libXmu-devel \
     mesa-libGL-devel mesa-libGLU-devel
```

## 1. Get the OCCT source at the release tag

Run all shell commands from the repository root unless stated otherwise.
Clone the tag into the repo's gitignored source/build area:

```
git clone --depth 1 --branch V8.0.1 https://github.com/Open-Cascade-SAS/OCCT.git tmp/occt/OCCT
```

If `tmp/occt/OCCT` already exists, use the existing `V8.0.1` checkout rather
than cloning over it. Keep the modified source until you have preserved the
corresponding source and build information needed for redistribution.

## 2. Configure, build, install to the repo's dist

Run from the repo root. OCCT installs **directly into the repo's vendored
dist** — the directory `OCCT_PREFIX` in `.cargo/config.toml` must point at —
so there is no external install and no copy step afterwards.

**Disable library versioning in the source** before configuring. In
`tmp/occt/OCCT/adm/cmake/occt_toolkit.cmake`, replace:

```cmake
set_target_properties (${PROJECT_NAME} PROPERTIES COMPILE_FLAGS "${PRECOMPILED_DEFS}"
                                                  SOVERSION     "${OCC_SOVERSION}"
                                                  VERSION       "${OCC_VERSION_MAJOR}.${OCC_VERSION_MINOR}.${OCC_VERSION_MAINTENANCE}")
```

with:

```cmake
set_target_properties (${PROJECT_NAME} PROPERTIES
                       COMPILE_FLAGS "${PRECOMPILED_DEFS}")
```

Removing both `VERSION` and `SOVERSION` makes CMake build and install plain
`libTK*.so` files directly, without version suffixes or library symlinks.
Keep `BUILD_SHARED_LIBRARY_NAME_POSTFIX` at its default empty value; a
nonempty postfix enables a separate OCCT symlink rule.

This is a **local source modification**, not a stock configure option.
Add a comment recording the change and its date in the modified file, and
retain it with the corresponding OCCT source for redistribution.

`-DBUILD_SOVERSION_NUMBERS=0` alone does **not** achieve this: OCCT 8.0.1
still sets `VERSION` unconditionally, and its top-level CMake logic can
reset a zero `BUILD_SOVERSION_NUMBERS` value to the Linux default of 2.
There is no stock OCCT 8.0.1 configure flag that disables both properties.

**Configure** — generates the build plan; nothing compiles yet:

```
cmake -S tmp/occt/OCCT -B tmp/occt/build -G Ninja -DCMAKE_BUILD_TYPE=Release \
      -DINSTALL_DIR="$PWD/dist/occt/x86_64-linux" \
      -DINSTALL_DIR_LAYOUT=Unix -DINSTALL_DIR_WITH_VERSION=OFF \
      -DBUILD_LIBRARY_TYPE=Shared -DBUILD_SHARED_LIBRARY_NAME_POSTFIX=""
```

The switches:

- `-S tmp/occt/OCCT` — source directory: the clone from step 1.
- `-B tmp/occt/build` — build directory: where all intermediate output
  (object files, generated scripts) goes. Keeping it separate from the
  source means deleting it deletes the build and nothing else.
- `-G Ninja` — generate for the Ninja build system (`ninja-build` package);
  faster and quieter than `make`.
- `-DCMAKE_BUILD_TYPE=Release` — explicitly select an optimized build.
- `-DINSTALL_DIR="$PWD/dist/occt/x86_64-linux"` — OCCT's own name for the install
  prefix (same as `CMAKE_INSTALL_PREFIX`): where `cmake --install` puts the
  final files. Here: the repo's vendored dist.
- `-DINSTALL_DIR_LAYOUT=Unix -DINSTALL_DIR_WITH_VERSION=OFF` — use the
  unversioned Unix directory layout expected by this guide and `build.rs`.
- `-DBUILD_LIBRARY_TYPE=Shared` — build the `.so` libraries to distribute
  with the importer.
- `-DBUILD_SHARED_LIBRARY_NAME_POSTFIX=""` — disable the separate postfix
  symlink rule, including when reusing a CMake cache.

If you previously customized `INSTALL_DIR_LIB`, `INSTALL_DIR_INCLUDE`, or
other install subdirectories in this build cache, reset those overrides to
the layout below or use a fresh build directory.

**Build** — the actual compile; the 10–30 minute part:

```
cmake --build tmp/occt/build --parallel
```

`--parallel` uses Ninja's default parallelism. To bound memory usage, give
it a job count, for example `--parallel 4`.

After changing the library-versioning properties, reconfigure and build
before installing, even if you previously built the versioned libraries.
`cmake --install` only copies existing build output; it does not compile or
relink the new unversioned libraries.

**Install** — copies headers, libraries, resources into the dist:

Run this only after the build has completed successfully. If reusing a
dist from an older versioned installation, first remove its obsolete
`libTK*.so*` files from `dist/occt/x86_64-linux/lib` so stale symlinks and old
libraries cannot survive alongside the new files. CMake does not remove
obsolete installed files.

```
cmake --install tmp/occt/build
```

The headers and libraries go into `include/opencascade` and `lib`, matching
the paths `build.rs` expects. Resources go into
`share/opencascade/resources`. The prefix is
read from `INSTALL_DIR` at **configure time** — `cmake --install --prefix
<dir>` afterwards does **not** fully work: OCCT bakes absolute header-install
paths into the generated scripts, so only some files would follow the new
prefix.

Useful switches (add to the `cmake -S` line):

- `-DUSE_TBB=ON -DUSE_RAPIDJSON=ON -DUSE_DRACO=ON -DUSE_FREEIMAGE=ON` —
  enable optional third parties (install the matching packages first).
- `-DBUILD_MODULE_Draw=OFF` — skip the DRAW test harness (faster build;
  then Tcl/Tk is not required).
- `-DUSE_GLES2=ON -DUSE_OPENGL=OFF` — select the GLES2/EGL driver instead
  of desktop OpenGL (install `libegl-dev libgles-dev` first). OCCT 8.0.1 has
  no separate `USE_EGL` option. This does not by itself guarantee headless
  rendering, and STEP loading/tessellation does not need a viewer.

Build time: roughly 10–30 minutes depending on machine and `-j` level.

Installed layout (= the repo's `dist/occt/x86_64-linux/`):

```
<prefix>/
├── bin/                         DRAWEXE, draw.sh, env.sh, custom*.sh
├── lib/                         libTK*.so (plain real files, unversioned)
│   └── cmake/opencascade/
├── include/opencascade/
└── share/
    ├── opencascade/
    │   ├── resources/           Shaders, DrawResources, UnitsAPI, ...
    │   ├── data/                DRAW sample data
    │   └── tests/               only with INSTALL_TEST_CASES=ON
    └── doc/opencascade/         license/exception; docs if enabled
```

Check that the libraries are real, unversioned files:

```bash
test -f dist/occt/x86_64-linux/lib/libTKernel.so && \
    test ! -L dist/occt/x86_64-linux/lib/libTKernel.so
find dist/occt/x86_64-linux -type l
```

The `test` command must succeed and `find` must print nothing. The source
change removes OCCT's library symlinks; separately copied third-party
libraries must also be packaged as real files.

## 3. Verify with DRAWEXE

This section applies when `BUILD_MODULE_Draw` is enabled (the default).

```
dist/occt/x86_64-linux/bin/draw.sh
```

`draw.sh` sources `env.sh` next to it (sets `CASROOT`, resource paths,
`LD_LIBRARY_PATH`) and starts DRAWEXE. Then:

```
pload ALL
box b 10 20 30
vinit
vdisplay b
vfit
```

A viewer window with a box should appear. The default Linux viewer needs
an X display, including XWayland on a Wayland desktop. Type `exit` to leave.

For a headless check, start DRAW in batch mode instead:

```bash
source dist/occt/x86_64-linux/bin/env.sh
dist/occt/x86_64-linux/bin/DRAWEXE -b
```

Then run `pload MODELING`, `box b 10 20 30`, and `bprops b`; omit `vinit`
and the other viewer commands. `bprops` prints mass properties without a
GUI. Type `exit` to leave. DRAW validation checks OCCT independently of
the Rust shim and Unity.

On a default build, `pload ALL` can report:

```
Pload : Cannot load plugin GLTF: Could not open: libTKXSDRAWGLTF.so; ...
```

That GLTF warning is **normal**: with `USE_RAPIDJSON` OFF (the default) the
glTF toolkit `libTKXSDRAWGLTF.so` is simply not built, and DRAW skips it. It
does not indicate a failure of STEP support.

## 4. Use OCCT from Rust (validation)

The repo already contains `crates/occt-sys`. It links OCCT and compiles C++
shims with a C ABI for Rust to call. Use these existing files; there is no
need to create a throwaway crate or replace the workspace configuration.

Layout (from the repo root):

```
Cargo.toml                  # workspace root
crates/occt-sys/
├── Cargo.toml
├── build.rs
├── cpp/box_volume.cpp      # smoke-test shim
├── cpp/step_mesh.cpp       # STEP shim: open/info/mesh/close
├── src/lib.rs              # C ABI declarations + safe wrappers/re-exports
├── src/step_doc.rs         # StepDoc: RAII handle, open/info/mesh
├── src/step_info.rs        # StepInfo
├── src/grey_box.rs         # GreyBbox (#[repr(C)])
├── src/error/              # OcctError / OcctResult
└── examples/box_volume.rs
```

`OCCT_PREFIX` in `.cargo/config.toml` must point at the repo-relative
`dist/occt/x86_64-linux` — the install prefix from step 2 (update the value if it
differs). `build.rs` uses `include/opencascade` and `lib` under
that prefix, compiles both C++ shims as C++17, and links these nine toolkits:

```
TKernel TKMath TKGeomBase TKBRep TKPrim TKTopAlgo TKMesh TKXSBase TKDESTEP
```

It also requests a development ELF `RPATH` containing the absolute dist
library directory and `$ORIGIN`, using `--disable-new-dtags`. `$ORIGIN`
means the directory of the ELF object carrying that path. This supports
the local Cargo example; it is not a completed Unity loading strategy.

After installing OCCT, run the existing smoke example:

```
cargo run -p occt-sys --example box_volume
# volume: 6000
```

`volume: 6000` (= 10·20·30) proves the full loop: `build.rs` finds the
installed prefix, links the right toolkits, the shim compiles against the
OCCT headers, and the example runs. It does not validate STEP import,
Unity integration, or deployment to another machine. The current crate
is not a Unity-loadable shared plugin; the importer still needs that
integration and a managed Scripted Importer.

## 5. Distributing OCCT inside the UPM package

The dist is a development installation, not the final package layout. The
UPM package must contain the importer native plugin, its required OCCT
libraries, any third-party runtime libraries that are not part of the
supported system baseline, required external resources, and license/source
notices. Headers, CMake metadata, DRAWEXE, sample data, and DRAW plugins
are development tools rather than importer runtime requirements.

### Determine the runtime dependency closure

The nine libraries in `build.rs` are a **link-time list**. Each library has
its own `DT_NEEDED` entries, so the package must include the full OCCT
runtime dependency closure. The exact list and size depend on the OCCT
build options and the eventual importer plugin; do not assume a fixed
library count from another build.

To inspect the current integration's dependencies after installation:

```bash
occt_lib_dir="$PWD/dist/occt/x86_64-linux/lib"
for tk in TKernel TKMath TKGeomBase TKBRep TKPrim TKTopAlgo TKMesh TKXSBase TKDESTEP; do
    LD_LIBRARY_PATH="$occt_lib_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
        ldd "$occt_lib_dir/lib${tk}.so"
done
```

`ldd` lists resolved transitive dependencies. Every entry must resolve:
`not found` means the closure is incomplete, including dependencies below
the missing library. Retain the required `libTK*.so` files and identify
which other dependencies must ship versus which the supported Linux/Unity
environment supplies. For example, enabling TBB or FreeImage can add
runtime dependencies; glibc and graphics-driver libraries belong to the
system baseline rather than being copied indiscriminately into a package.

When the importer native plugin exists, repeat the inspection on that
actual `.so`, and account for any libraries loaded explicitly at runtime
that do not appear in `DT_NEEDED`. If a third-party library is copied from
a system installation, dereference its symlink and retain the filename
that the loader expects. All packaged native files must be real files.

### Make native loading independent of the checkout

The Cargo example's absolute development `RPATH` is not suitable for the
UPM package. Unity loads a native shared plugin inside the Editor process;
the example executable's linker paths do not configure Unity or every
OCCT dependency.

Inspect the ELF metadata of each final native artifact with:

```bash
readelf -d /absolute/path/to/packaged/library.so
```

Check `NEEDED`, `SONAME`, and `RPATH`/`RUNPATH`. Shipped artifacts must not
contain dependency or search paths into this checkout. If the package
places the native plugin and its dependencies together, `$ORIGIN` can be
used to find adjacent libraries. With `DT_RUNPATH`, each shared library
needs a search path for its own direct dependencies; a path on the top-level
plugin alone does not cover the entire transitive chain. Set the paths at
link time or adjust the final ELF files with a tool such as `patchelf`,
according to the eventual package layout.

Configure Unity's native Plugin Importer settings for the intended Linux
x86_64 Editor use. Test loading through the actual managed importer from a
relocated UPM installation, without a separately installed OCCT or
development `LD_LIBRARY_PATH`. That integration and validation remain to
be implemented; the build in this guide does not provide them.

### Resolve external resources from the installed package

The installation contains resources in
`dist/occt/x86_64-linux/share/opencascade/resources`. OCCT 8.0.1 embeds some
defaults, messages, and other resource content; do not assume that every
installed file is required, or that every resource lookup is embedded.
Retain the external resources used by the importer's chosen APIs.

For local development, `bin/env.sh` sets OCCT resource variables from the
installed prefix. Relevant mappings in the OCCT 8.0.1 template include:

| Variable | Directory relative to the resource root |
|---|---|
| `CSF_STEPDefaults`, `CSF_IGESDefaults` | `XSTEPResource` |
| `CSF_XSMessage` | `XSMessage` |
| `CSF_SHMessage` | `SHMessage` |
| `CSF_StandardDefaults`, `CSF_StandardLiteDefaults`, `CSF_PluginDefaults`, `CSF_XCAFDefaults` | `StdResource` |
| `CSF_XmlOcafResource` | `XmlOcafResource` |
| `CSF_ShadersDirectory` | `Shaders` (when using OCCT rendering) |

For the Unity importer, derive any required resource paths from the actual
installed UPM package location and initialize lookup before the first
OCCT operation that uses them. Resource configuration through environment
variables affects the whole Unity process. The existing shim has no
package-resource initialization function. Merely copying `env.sh` into a
package does not make Unity source it, and neither `CASROOT` nor an absolute
developer path is a substitute for package-relative initialization.

### Include licenses and corresponding source

The Unix installation places `LICENSE_LGPL_21.txt` and
`OCCT_LGPL_EXCEPTION.txt` in `dist/occt/x86_64-linux/share/doc/opencascade`.
Include both in the UPM distribution, retain applicable copyright notices,
and give prominent notice that the importer uses OCCT. Record the exact
release tag, the local CMake modification, and the build configuration.
Also include applicable notices for redistributed third-party libraries.

The Open CASCADE exception allows header material in the importer's object
code under your chosen terms with the required notice; it does not remove
the LGPL obligations for the redistributed OCCT libraries. Provide complete
corresponding OCCT source, including the CMake change and its dated notice,
with the binaries or equivalent access to that source from the same
distribution location. A link to the stock tag alone does not represent
the modified source used here. Preserve the ability to replace the shared
OCCT libraries with an interface-compatible modified version.

## 6. Optional cleanup

After installation and your chosen validation, preserve the modified OCCT
source and build configuration needed to reproduce and redistribute these
binaries. Then, if you do not need the source/build tree for further work:

```bash
rm -rf tmp/occt
```

This deletes both the modified source checkout and the build output, but
leaves `dist/occt/x86_64-linux`. Re-cloning the stock tag alone will not restore
the local CMake modification.

## Swapping versions

Replace `V8.0.1` in the step-1 clone command and re-run steps 1–2 — the
vendored dist is rebuilt from the new tag. If retaining the old source
checkout, replace it with the requested tag before rebuilding. Reapply the
library-versioning change in step 2, checking the new tag's CMake code first,
and remove obsolete installed libraries before installing the new build.
Repeat validation and runtime dependency inspection. Tag naming differs by era:
8.0.x uses `V8.0.1`/`V8_0_0` (dots and underscores both appear), 7.9.x uses
underscores (`V7_9_3`). The 8.0 series requires a C++17 compiler.

Platform docs (`overview-doc.zip`, `refman-doc.zip`) are attached to each
release and are platform-independent — download and unzip them if you want
offline documentation.

## Troubleshooting

- **`file INSTALL cannot find .../libTKernel.so`** — the unversioned library
  has not been built. After applying the CMake change in step 2, run the
  configure command again, then `cmake --build tmp/occt/build --parallel`
  and wait for it to succeed before running `cmake --install tmp/occt/build`.
  If the build fails, resolve its first error; rerunning install alone
  cannot generate the missing library.
- **CMake can't find Tcl/Tk or FreeType** — make sure the `-dev`/`-devel`
  packages above are installed; check `CMakeError` output for the missing
  component name.
- **`X11` or `GL/gl.h` not found** — install `libx11-dev`/`libgl1-mesa-dev`
  (Ubuntu) or `libX11-devel`/`mesa-libGL-devel` (Fedora).
- **Viewer doesn't open on a headless server** — DRAW's GUI needs X. Use an
  X display for viewer commands, or start `DRAWEXE -b` as in section 3 and
  use non-graphical commands such as `bprops b`.
- **`Pload: Cannot load plugin GLTF`** in DRAWEXE — harmless: the glTF
  toolkit (`libTKXSDRAWGLTF.so`) isn't built because `USE_RAPIDJSON` is OFF
  (the default). Ignore it, or enable `USE_RAPIDJSON` (+ `USE_DRACO` if you
  want compressed meshes) and rebuild.
- **`file cannot create directory: /usr/local/include/opencascade`** (or
  headers land in `/usr/local` despite `--prefix`) — the prefix was not set at
  configure time. Re-run the configure step with
  `-DINSTALL_DIR="$PWD/dist/occt/x86_64-linux"`, then
  `cmake --build tmp/occt/build --parallel` and
  `cmake --install tmp/occt/build`. Use the same build directory throughout.
- **`libTK*.so` not found at runtime** — confirm installation succeeded and
  inspect the executable/plugin's ELF search paths and dependency closure.
  The current Cargo example uses the configured dist through development
  rpaths; the Unity package needs the relocatable loading setup in section 5.
- **Libraries still have symlinks or version suffixes** — confirm both
  `VERSION` and `SOVERSION` were removed, the postfix is empty, and you
  reconfigured and rebuilt before installing. Remove leftovers from an
  older dist; reinstalling does not clean them up.
