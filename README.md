# greyhound-snake

A Unity **Scripted Importer for STEP files** built on Open CASCADE Technology
(OCCT). Shipped as a **Unity Package Manager (UPM) package that bundles the
required OCCT native binaries**, so Unity users never install OCCT
themselves.

**Status: working on Linux x86_64.** The importer imports single parts and
assembly hierarchies into Unity with per-part solid colors, bounds, and
exact-BRep mass properties (per-part and rolled up over assemblies). The
packaged native payload and its loading are relocatable — validated from an
installed package, not just the developer checkout. Joint authoring is
built: right-click a part to add joints and actuators as a
ScriptableObject asset family beside the STEP file, with scene tools for
axis picking and travel gizmos, surviving STEP reimports. The joint set's
Export button writes a MuJoCo MJCF model (body tree, joints, `<position>`
actuators, exact-BRep inertials) with binary STL meshes into a
`<step file>.stp~` folder beside the file (`unity-mujoco.md` holds the
facts and decisions). A standalone Rust CLI,
`crates/greyhound-export-mjc`, exports the same rigid-tree MJCF without
Unity. Only Linux x86_64 is exercised end to end.

## Repository layout

| Path | Purpose |
|---|---|
| `crates/occt-sys` | Rust binding: runtime loader for the C++ shim and OCCT libraries; STEP open/info/mesh/scene/mass-properties API behind a version-gated C ABI |
| `crates/occt-sys/cpp` | C++ shims with a C ABI, compiled by `build.rs` directly into the package layout |
| `crates/mesh` | Host-neutral core mesh model (positions/indices/per-face ranges, assembly scene, exact-BRep `MeshProperties`) |
| `crates/unity-projection` | OCCT→Unity projection: axis permutation, winding flip, scale, submesh assembly |
| `crates/importer-host` | Unity-facing cdylib — flat, version-gated C ABI over the projection |
| `crates/greyhound-export-mjc` | CLI exporting a STEP file as a naive rigid-tree MJCF model plus binary STL meshes (reference implementation of the emitted schema) |
| `crates/step-stats` | CLI that prints geometry stats (solids/faces/edges/bbox) for a STEP file |
| `package/com.greyhound.step` | Tracked UPM package sources: the C# `Runtime/` P/Invoke layer, mass-properties components, the joint/actuator/geom annotation asset family with its `Mj*` MJCF mirrors, and the `Editor/` ScriptedImporter, authoring editors, scene tools, reimport validator, and MJCF exporter |
| `tools/verify-package.py` | End-to-end verification of the assembled package against measured constants |
| `justfile` | `just assemble-package` stages package + native payload into `dist/`; `just verify-package` checks it from a copied layout |
| `assets/` | Sample STEP files (single part `rod-clamp-16mm.stp`, assembly `cart-asy.step`) |
| `config.toml` | Runtime config for `step-stats`: OCCT `library_dir` and `shim_path` |
| `.cargo/config.toml` | Sets `OCCT_PREFIX` and `OCCT_SHIM_DIR` so OCCT installs and the shim compiles into the package layout |
| `third_party/occt` | OCCT license and exception texts bundled with the binaries |
| `dist/` | Gitignored native output (see below) |
| `tmp/` | Gitignored OCCT source/build scratch area, including the stage logs (see `occt-linux.md`) |

### Design reference docs

- `occt-to-unity.md` — the OCCT→Unity transformation pipeline, the gap
  backlog, and the handedness/source-of-truth rules
- `occt-mesh.md`, `unity-mesh.md` — mesh data-model facts for both sides
- `unity-mujoco.md` — MuJoCo/MJCF export and joint authoring facts, plan,
  and landed decisions
- `mujoco-schema.md` — MJCF facts checked against the MuJoCo stable docs
  (compiler, body/joint/inertial, `<position>` actuation)
- `perf.md` — benchmark plan and the scalar-first SIMD decision

## Native output layout

| Path | Contents |
|---|---|
| `dist/package/com.greyhound.step/` | the assembled UPM package: authored sources plus the native payload |
| `…/Runtime/Plugins/occt/x86_64/` | the OCCT installation (`lib/`, `include/opencascade/`, `share/`, `bin/`) |
| `…/Runtime/Plugins/shim/x86_64/` | `libgreyhound_occt.so` — the C++ shim built by `occt-sys/build.rs` |
| `…/Runtime/Plugins/x86_64/` | `libimporter_host.so` — the Unity-facing host cdylib |

## Getting started (Linux x86_64)

Prerequisites: a recent Rust toolchain (edition 2024) and a C++17 compiler.
`build.rs` only accepts native Linux x86_64 builds; other targets are rejected.

1. Install OCCT 8.0.1 into
   `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64` by
   following **`occt-linux.md`** (configure → build → install, then
   validate with `step-stats` as described there).
2. Build the workspace:

   ```
   cargo build
   ```

   `occt-sys/build.rs` compiles the C++ shims into
   `dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so`,
   linking the OCCT toolkits it needs. This requires the OCCT installation
   from step 1.
3. Inspect the sample STEP file:

   ```
   cargo run -p step-stats -- assets/rod-clamp-16mm.stp
   ```

   `--config` defaults to `config.toml` in the working directory; its
   `library_dir` and `shim_path` values resolve relative to the config file's
   directory.
4. Assemble and verify the UPM package:

   ```
   just verify-package
   ```

5. Use it in Unity: reference or copy
   `dist/package/com.greyhound.step/` into a project (a `file:` entry in
   `Packages/manifest.json` works), restart the editor so the native plugins
   are scanned, and drop a `.stp`/`.step` file into `Assets/`. Right-click
   an imported part to author joints and actuators; the joint set's
   inspector has the Export button that writes the MJCF model beside the
   STEP file.

## How native loading works

- Unity loads `importer_host` as a native plugin from the package's
  `Runtime/Plugins/x86_64` directory and P/Invokes it
  (`NativeMethods.cs`).
- The C# side resolves the package's own plugin directories at runtime
  (`OcctLibraryDirectory()`, shim path) and hands them to the host; nothing
  pins absolute paths from the developer's checkout.
- `occt-sys` `dlopen`s the shim and explicitly preloads every OCCT library
  reachable through the shim's ELF `DT_NEEDED` chain from that directory
  before any STEP work (`crates/occt-sys/src/dependencies.rs`).
- The shim is built with `RPATH=$ORIGIN`; the loader tests use stub fixtures
  compiled alongside it, so they exercise the loader without touching real
  OCCT libraries at runtime.

## Platform OCCT installation docs

- **`occt-linux.md`** — build OCCT 8.0.1 from source into the repo's dist
- **`occt-mac.md`** — build OCCT 8.0.1 from source on macOS (Homebrew for
  third-party libraries)
- **`occt-win.md`** — install the official prebuilt Windows binaries (MSVC x64)

Only the Linux x86_64 path is exercised by the Rust/C++/Unity integration
today; the macOS/Windows docs cover OCCT installation, not a working
integration.

## License

Code in this repository is licensed under the [MIT](LICENSE). It uses
[Open CASCADE Technology](https://github.com/Open-Cascade-SAS/OCCT), which is
LGPL-2.1 with the Open CASCADE exception — see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md); the OCCT license and
exception texts are bundled under `third_party/occt/`.
