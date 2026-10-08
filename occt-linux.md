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

> ## Prerequisites
>
> - A compiler with C++17 support (GCC or Clang); the package commands below install the distribution's GCC toolchain.
> - CMake 3.16+ for the commands in this guide, Ninja, Git, and binutils (`readelf` for dependency inspection). OCCT itself declares CMake 3.10 as its minimum, but this guide uses newer CMake command-line features.
> - The build below includes Visualization, so install **FreeType**, X11, and desktop OpenGL development packages. The DRAW test harness is disabled in step 2, so Tcl/Tk is not required. These build-machine dependencies are not instructions for users of the eventual Unity package.
>
> Build on the oldest Linux environment you intend to support for the Unity Editor. A build on a newer distribution can require newer glibc or `libstdc++` symbols than an older target provides.
>
> Ubuntu 22.04/24.04, Debian 12+:
>
> ```
> sudo apt install build-essential git cmake ninja-build binutils \
>      libfreetype-dev \
>      libx11-dev libxext-dev libxft-dev libxrender-dev libxi-dev libxmu-dev \
>      libgl1-mesa-dev libglu1-mesa-dev
> ```
>
> Optional, only if you enable the matching flags later:
>
> ```
> sudo apt install libtbb-dev libfreeimage-dev rapidjson-dev \
>      libdraco-dev libvtk9-dev libeigen3-dev libjemalloc-dev
> # USE_TBB, USE_FREEIMAGE, USE_RAPIDJSON, USE_DRACO, USE_VTK, USE_EIGEN,
> # USE_MMGR_TYPE=JEMALLOC
> ```
>
> The optional boolean flags below default to OFF; the default memory manager is `USE_MMGR_TYPE=NATIVE`:
>
> | Package | Flag | What it enables |
> |---|---|---|
> | `libtbb-dev` | `USE_TBB` | TBB-backed parallel computations. OCCT also has native parallel facilities when TBB is disabled. |
> | `libfreeimage-dev` | `USE_FREEIMAGE` | FreeImage in the Visualization module — reading/writing popular image formats (PNG, BMP, ...) for screenshots/view dumps and textures. |
> | `rapidjson-dev` | `USE_RAPIDJSON` | RapidJSON in Data Exchange — JSON support for the glTF reader/writer (TKDEGLTF). Without it the glTF toolkits are skipped entirely. |
> | `libdraco-dev` | `USE_DRACO` | Draco mesh decoding for the glTF reader (`KHR_draco_mesh_compression`). Needs `USE_RAPIDJSON`. |
> | `libvtk9-dev` | `USE_VTK` | VTK bridge (TKIVtk) — push OCCT shapes into VTK for scientific-visualization pipelines. |
> | `libeigen3-dev` | `USE_EIGEN` | Eigen linear algebra for a few internal routines; header-only, adds an include path — nothing is linked. |
> | `libjemalloc-dev` | `USE_MMGR_TYPE=JEMALLOC` | jemalloc as OCCT's internal memory manager (TKernel). Default is `NATIVE`; alternatives are `FLEXIBLE` and `TBB`. |
>
> (`libegl-dev`/`libgles-dev` are the deps for `USE_GLES2` — see below.)

> ## 1. Get the OCCT source at the release tag
>
> Run all shell commands from the repository root unless stated otherwise. Clone the tag into the repo's gitignored source/build area:
>
> ```
> git clone --depth 1 --branch V8.0.1 https://github.com/Open-Cascade-SAS/OCCT.git tmp/occt/OCCT
> ```
>
> If `tmp/occt/OCCT` already exists, use the existing `V8.0.1` checkout rather than cloning over it. Keep the modified source until you have preserved the corresponding source and build information needed for redistribution.

> ## 2. Configure, build, install to the repo's dist
>
> Run from the repo root. OCCT installs **directly into the repo's vendored dist** — the directory `OCCT_PREFIX` in `.cargo/config.toml` must point at — so there is no external install and no copy step afterwards.
>
> **Disable library versioning in the source** before configuring. In `tmp/occt/OCCT/adm/cmake/occt_toolkit.cmake`, replace:
>
> ```cmake
> set_target_properties (${PROJECT_NAME} PROPERTIES COMPILE_FLAGS "${PRECOMPILED_DEFS}"
>                                                   SOVERSION     "${OCC_SOVERSION}"
>                                                   VERSION       "${OCC_VERSION_MAJOR}.${OCC_VERSION_MINOR}.${OCC_VERSION_MAINTENANCE}")
> ```
>
> with:
>
> ```cmake
> set_target_properties (${PROJECT_NAME} PROPERTIES
>                        COMPILE_FLAGS "${PRECOMPILED_DEFS}")
> ```
>
> Removing both `VERSION` and `SOVERSION` makes CMake build and install plain `libTK*.so` files directly, without version suffixes or library symlinks. Keep `BUILD_SHARED_LIBRARY_NAME_POSTFIX` at its default empty value; a nonempty postfix enables a separate OCCT symlink rule.
>
> This is a **local source modification**, not a stock configure option. Add a comment recording the change and its date in the modified file, and retain it with the corresponding OCCT source for redistribution.
>
> `-DBUILD_SOVERSION_NUMBERS=0` alone does **not** achieve this: OCCT 8.0.1 still sets `VERSION` unconditionally, and its top-level CMake logic can reset a zero `BUILD_SOVERSION_NUMBERS` value to the Linux default of 2. There is no stock OCCT 8.0.1 configure flag that disables both properties.
>
> Each stage command below tees its output to a log file in `tmp/occt/` — `configure.log`, `build.log`, `install.log` — so a failed stage can be shared in full. The terminal still shows everything. Note that in a pipeline the exit status comes from `tee`, so run `set -o pipefail` first if you script these commands. `tmp/` is gitignored, and the optional cleanup deletes the logs along with the rest of the tree.
>
> **Configure** — generates the build plan; nothing compiles yet:
>
> ```
> cmake -S tmp/occt/OCCT -B tmp/occt/build -G Ninja \
>       -DCMAKE_BUILD_TYPE=Release \
>       -DINSTALL_DIR="$PWD/dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64" \
>       -DINSTALL_DIR_LAYOUT=Unix \
>       -DINSTALL_DIR_WITH_VERSION=OFF \
>       -DBUILD_LIBRARY_TYPE=Shared \
>       -DBUILD_SHARED_LIBRARY_NAME_POSTFIX="" \
>       -DBUILD_MODULE_Draw=OFF \
>       2>&1 | tee tmp/occt/configure.log
> ```
>
> The switches:
>
> - `-S tmp/occt/OCCT` — source directory: the clone from step 1.
> - `-B tmp/occt/build` — build directory: where all intermediate output (object files, generated scripts) goes. Keeping it separate from the source means deleting it deletes the build and nothing else.
> - `-G Ninja` — generate for the Ninja build system (`ninja-build` package); faster and quieter than `make`.
> - `-DCMAKE_BUILD_TYPE=Release` — explicitly select an optimized build.
> - `-DINSTALL_DIR="$PWD/dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64"` — OCCT's own name for the install prefix (same as `CMAKE_INSTALL_PREFIX`): where `cmake --install` puts the final files. Here: **the OCCT directory inside the Unity package itself** — the install *is* the package payload; there is no separate dev install to keep in sync.
> - `-DINSTALL_DIR_LAYOUT=Unix -DINSTALL_DIR_WITH_VERSION=OFF` — use the unversioned Unix directory layout expected by this guide and `build.rs`.
> - `-DBUILD_LIBRARY_TYPE=Shared` — build the `.so` libraries to distribute with the importer.
> - `-DBUILD_SHARED_LIBRARY_NAME_POSTFIX=""` — disable the separate postfix symlink rule, including when reusing a CMake cache.
> - `-DBUILD_MODULE_Draw=OFF` — skip the DRAW test harness: DRAWEXE and draw.sh are not built, Tcl/Tk is not required, and the DRAW sample data and DrawResources are not installed. OCCT validation in this guide goes through section 3 instead.
>
> If you previously customized `INSTALL_DIR_LIB`, `INSTALL_DIR_INCLUDE`, or other install subdirectories in this build cache, make them match the switches above or use a fresh build directory.
>
> **Build** — the actual compile; the 10–30 minute part:
>
> ```
> cmake --build tmp/occt/build --parallel 2>&1 | tee tmp/occt/build.log
> ```
>
> `--parallel` uses Ninja's default parallelism. To bound memory usage, give it a job count, for example `--parallel 4`.
>
> After changing the library-versioning properties, reconfigure and build before installing, even if you previously built the versioned libraries. `cmake --install` only copies existing build output; it does not compile or relink the new unversioned libraries.
>
> **Install** — copies headers, libraries, resources into the dist:
>
> Run this only after the build has completed successfully. If reusing a dist from an older versioned installation, first remove its obsolete `libTK*.so*` files so stale symlinks and old libraries cannot survive alongside the new files. CMake does not remove obsolete installed files.
>
> ```
> cmake --install tmp/occt/build 2>&1 | tee tmp/occt/install.log
> ```
>
> The whole install — headers, libraries, resources, CMake package config — lands inside the package at `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/`, which is the path `.cargo/config.toml`'s `OCCT_PREFIX` points at. The prefix is read from `INSTALL_DIR` at **configure time** — `cmake --install --prefix <dir>` afterwards does **not** fully work: OCCT bakes absolute header-install paths into the generated scripts, so only some files would follow the new prefix.
>
> Useful switches (add to the `cmake -S` line):
>
> - `-DUSE_TBB=ON -DUSE_RAPIDJSON=ON -DUSE_DRACO=ON -DUSE_FREEIMAGE=ON` — enable optional third parties (install the matching packages first).
> - `-DUSE_GLES2=ON -DUSE_OPENGL=OFF` — select the GLES2/EGL driver instead of desktop OpenGL (install `libegl-dev libgles-dev` first). OCCT 8.0.1 has no separate `USE_EGL` option. This does not by itself guarantee headless rendering, and STEP loading/tessellation does not need a viewer.
>
> Build time: roughly 10–30 minutes depending on machine and `-j` level.
>
> Installed layout (= the package's OCCT directory `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/`):
>
> ```
> <prefix>/
> ├── bin/                         env.sh, custom*.sh
> ├── lib/                         libTK*.so (plain real files, unversioned)
> │   └── cmake/opencascade/
> ├── include/opencascade/
> └── share/
>     ├── opencascade/
>     │   ├── resources/           Shaders, UnitsAPI, ...
>     │   └── tests/               only with INSTALL_TEST_CASES=ON
>     └── doc/opencascade/         license/exception; docs if enabled
> ```
>
> Check that the libraries are real, unversioned files:
>
> ```bash
> test -f dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib/libTKernel.so && \
>     test ! -L dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib/libTKernel.so
> find dist/package/com.greyhound.step -type l
> ```
>
> The `test` command must succeed and `find` must print nothing. The source change removes OCCT's library symlinks; separately copied third-party libraries must also be packaged as real files.

> ## 3. Use OCCT from Rust (validation)
>
> The repo already contains `crates/occt-sys`, `crates/step-stats`, and a sample STEP asset. They are the validation harness for this guide; there is no need to create a throwaway crate or replace the workspace configuration.
>
> Layout (from the repo root):
>
> ```
> Cargo.toml                      # workspace root
> config.toml                     # runtime loading config for step-stats
> crates/occt-sys/
> ├── Cargo.toml
> ├── build.rs                    # compiles shims + fixtures, links the toolkits
> ├── cpp/
> │   ├── abi.cpp                 # ABI version + last-error entry points
> │   ├── box_volume.cpp          # smoke-test shim
> │   ├── step_mesh.cpp           # STEP shim: open/info/mesh/close
> │   ├── greyhound_abi.h
> │   ├── native_guard.h
> │   └── tests/loader_fixture.cpp
> └── src/
>     ├── lib.rs                  # re-exports
>     ├── native_api.rs           # dlopen shim, resolve C ABI symbols
>     ├── dependencies.rs         # explicit preload of the OCCT DT_NEEDED chain
>     ├── occt.rs                 # Occt
>     ├── step_doc.rs             # StepDoc: RAII handle, open/info/mesh
>     ├── step_info.rs            # StepInfo
>     ├── grey_box.rs             # GreyBbox (#[repr(C)])
>     └── error/                  # OcctError / OcctResult
> crates/step-stats/               # CLI: loads config.toml, prints STEP stats
> assets/rod-clamp-16mm.stp        # validation asset
> ```

> `OCCT_PREFIX` in `.cargo/config.toml` must point at the repo-relative `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64` — the install prefix from step 2 — and `OCCT_SHIM_DIR` at `dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64` (update the values if they differ). `build.rs` uses `include/opencascade` and `lib` under `OCCT_PREFIX`, compiles the three C++ shims as C++17, and links these nine toolkits:
>
> ```
> TKernel TKMath TKGeomBase TKBRep TKPrim TKTopAlgo TKMesh TKXSBase TKDESTEP
> ```
>
> It writes the shim to `dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so` — its own first-party plugins directory beside the third-party OCCT tree — with `RPATH=$ORIGIN`, using `--disable-new-dtags`. `$ORIGIN` means the directory of the ELF object carrying that path; the shim's real dependency resolution happens through the loader's explicit pre-dlopen below, so the split directories need no rpath setup. Nothing links the shim into a Rust binary. At runtime the loader reads `config.toml`, which points `library_dir` at `Runtime/Plugins/occt/x86_64/lib` and `shim_path` at the shim (values resolve relative to the config file's directory), preloads every OCCT library reachable through the shim's `DT_NEEDED` chain, and then opens the shim and resolves the C ABI symbols. This config-driven loading already matches the package layout; Unity still needs the managed Scripted Importer on top.
>
> After installing OCCT, validate against the repo's test asset:
>
> ```
> cargo run -p step-stats -- assets/rod-clamp-16mm.stp
> ```
>
> A successful run prints the file path, solid/face/edge counts, and the bounding box for the sample asset, and exits successfully. It proves the full loop: `build.rs` finds the installed prefix, the shims compile against the OCCT headers, the loader preloads the configured OCCT libraries and opens the shim, and a real STEP document is read. Check that the counts are nonzero and sensible for the asset. It does not exercise meshing, Unity integration, or deployment to another machine. The current crate is not a Unity-loadable shared plugin; the importer still needs that integration and a managed Scripted Importer.
>
> `cargo test` runs the loader tests against stub fixtures compiled by `build.rs`; they do not load real OCCT libraries at runtime, but building the crate still requires the installed OCCT.

> ## 4. Optional cleanup
>
> After installation and your chosen validation, preserve the modified OCCT source and build configuration needed to reproduce and redistribute these binaries. Then, if you do not need the source/build tree for further work:
>
> ```bash
> rm -rf tmp/occt
> ```
>
> This deletes both the modified source checkout and the build output, but leaves `dist/occt/x86_64-linux`. Re-cloning the stock tag alone will not restore the local CMake modification.
>
> ## Swapping versions
>
> Replace `V8.0.1` in the step-1 clone command and re-run steps 1–2 — the vendored dist is rebuilt from the new tag. If retaining the old source checkout, replace it with the requested tag before rebuilding. Reapply the library-versioning change in step 2, checking the new tag's CMake code first, and remove obsolete installed libraries before installing the new build. Repeat validation and runtime dependency inspection. Tag naming differs by era: 8.0.x uses `V8.0.1`/`V8_0_0` (dots and underscores both appear), 7.9.x uses underscores (`V7_9_3`). The 8.0 series requires a C++17 compiler.
>
> Platform docs (`overview-doc.zip`, `refman-doc.zip`) are attached to each release and are platform-independent — download and unzip them if you want offline documentation.

> ## Troubleshooting
>
> - **`file INSTALL cannot find .../libTKernel.so`** — the unversioned library has not been built. After applying the CMake change in step 2, run the configure command again, then `cmake --build tmp/occt/build --parallel` and wait for it to succeed before running `cmake --install tmp/occt/build`. If the build fails, resolve its first error; rerunning install alone cannot generate the missing library.
> - **CMake can't find FreeType** — make sure `libfreetype-dev` is installed; check `CMakeError` output for the missing component name.
> - **`X11` or `GL/gl.h` not found** — install `libx11-dev`/`libgl1-mesa-dev` (Ubuntu) or `libX11-devel`/`mesa-libGL-devel` (Fedora).
> - **`file cannot create directory: /usr/local/include/opencascade`** (or headers land in `/usr/local` despite `--prefix`) — the prefix was not set at configure time. Re-run the configure step with `-DINSTALL_DIR="$PWD/dist/occt/x86_64-linux"`, then `cmake --build tmp/occt/build --parallel` and `cmake --install tmp/occt/build`. Use the same build directory throughout.
> - **`libTK*.so` not found at runtime** — confirm installation succeeded and check `config.toml`: `library_dir` must point at `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib` and `shim_path` at `dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so`, both resolved relative to the config file's directory. The same layout must be reachable when the package is relocated, which `just verify-package` checks from a copied layout.
> - **Libraries still have symlinks or version suffixes** — confirm both `VERSION` and `SOVERSION` were removed, the postfix is empty, and you reconfigured and rebuilt before installing. Remove leftovers from an older dist; reinstalling does not clean them up.
