# greyhound-snake

A Unity **Scripted Importer for STEP files** built on Open CASCADE Technology
(OCCT). The plan is to ship it as a **Unity Package Manager (UPM) package that
bundles the required OCCT native binaries**, so Unity users never install OCCT
themselves.

**Status: early groundwork.** The Unity integration and the UPM package do not
exist yet. What exists today is a Linux x86_64 Rust workspace with C++ shims
that load OCCT at runtime and exercise STEP import end to end from the command
line. Building this repository does not by itself create or validate the Unity
importer.

## Repository layout

| Path | Purpose |
|---|---|
| `crates/occt-sys` | Rust binding: runtime loader for the C++ shim and OCCT libraries; STEP open/info/mesh API (`Occt`, `StepDoc`, `StepInfo`, `GreyBbox`) |
| `crates/occt-sys/cpp` | C++ shims with a C ABI, compiled by `build.rs` |
| `crates/step-stats` | CLI that prints geometry stats (solids/faces/edges/bbox) for a STEP file |
| `occt-linux.md` / `occt-mac.md` / `occt-win.md` | Platform instructions for installing OCCT 8.0.1 |
| `assets/` | Sample STEP file |
| `config.toml` | Runtime config for `step-stats`: OCCT `library_dir` and `shim_path` |
| `.cargo/config.toml` | Sets `OCCT_PREFIX` for the shim build |
| `dist/` | Gitignored native output (see below) |
| `tmp/` | Gitignored OCCT source/build scratch area, including the stage logs (see `occt-linux.md`) |

## Native output layout

| Path | Contents |
|---|---|
| `dist/occt/<platform>/` | OCCT installation: `lib/`, `include/opencascade/`, `share/`, `bin/` |
| `dist/shim/<platform>/` | `libgreyhound_occt.so` — the C++ shim built by `occt-sys/build.rs` |

The only working `<platform>` today is `x86_64-linux`. The same
`<component>/<platform>` convention is what a future UPM `plugins/` staging
step would copy from.

## Getting started (Linux x86_64)

Prerequisites: a recent Rust toolchain (edition 2024) and a C++17 compiler.
`build.rs` only accepts native Linux x86_64 builds; other targets are rejected.

1. Install OCCT 8.0.1 into `dist/occt/x86_64-linux` by following
   **`occt-linux.md`** (configure → build → install, then validate with
   `step-stats` as described there).
2. Build the workspace:

   ```
   cargo build
   ```

   `occt-sys/build.rs` compiles the C++ shims into
   `dist/shim/x86_64-linux/libgreyhound_occt.so`, linking the OCCT toolkits it
   needs. This requires the OCCT installation from step 1.
3. Inspect the sample STEP file:

   ```
   cargo run -p step-stats -- assets/rod-clamp-16mm.stp
   ```

   `--config` defaults to `config.toml` in the working directory; its
   `library_dir` and `shim_path` values resolve relative to the config file's
   directory.

## How native loading works

- `step-stats` reads `config.toml`, then `occt-sys` `dlopen`s the shim and
  explicitly preloads every OCCT library reachable through the shim's ELF
  `DT_NEEDED` chain from `library_dir` before any STEP work
  (`crates/occt-sys/src/dependencies.rs`).
- The shim is built with `RPATH=$ORIGIN`; the loader tests use stub fixtures
  compiled alongside it, so they exercise the loader without touching real
  OCCT libraries at runtime.
- This config-file loading is development scaffolding, not the Unity loading
  strategy. Relocatable loading from an installed UPM package — dependency
  closure packaging, `$ORIGIN` setup, resource lookup, license inclusion — is
  not implemented or documented yet.

## Platform OCCT installation docs

- **`occt-linux.md`** — build OCCT 8.0.1 from source into the repo's dist
- **`occt-mac.md`** — build OCCT 8.0.1 from source on macOS (Homebrew for
  third-party libraries)
- **`occt-win.md`** — install the official prebuilt Windows binaries (MSVC x64)

Only the Linux x86_64 path is exercised by the Rust/C++ integration today; the
macOS/Windows docs cover OCCT installation, not a working integration.

## License

Code in this repository is licensed under the [MIT](LICENSE). It uses
[Open CASCADE Technology](https://github.com/Open-Cascade-SAS/OCCT), which is
LGPL-2.1 with the Open CASCADE exception — see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md); the OCCT license and
exception texts are bundled under `third_party/occt/`.