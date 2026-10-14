# refactor-repo — native binaries from release repos, `extern/` cache, provisioned payload

Decision record for the native-distribution refactor: this repo stops being
a place where OCCT (and later OpenCV) binaries are built or staged, and
becomes a consumer of **pinned release archives** from the TonyMarkham
`-dev` repos. Dev archives feed shim compilation; libs archives feed the
runtime payload, which the Unity package **provisions into the project at
package load** instead of shipping inside the UPM package.

Recorded 2026-10-13/14 from a working session. The Landed section tracks
what has been built and verified; the rest is intent. Each bite that lands
a piece updates the affected docs and this record; pieces still open are
listed in Open items.

## Current state (verified 2026-10-13, before any work)

- This repo has **never tracked OCCT or OpenCV binaries**: `git ls-files`
  finds zero `.so/.dll/.dylib/.lib/.a`; `dist/` and `tmp/` are gitignored;
  `third_party/` holds only three license texts (`occt/LICENSE_LGPL_21.txt`,
  `occt/OCCT_LGPL_EXCEPTION.txt`, `opencv/LICENSE`). The refactor changes
  the binary *source*, not a tracked payload.
- **Linux today**: OCCT is built from source (`occt-linux.md`) and installed
  *straight into the package payload dir* — the justfile prerequisite
  requires `$occt/include/opencascade/Standard_Version.hxx` inside
  `dist/package`, so build-time headers ship into the Unity package.
  `better-occt-bins.md` planned to end this (neutral install +
  `just stage-occt`) but was **not landed** — the justfile has no
  `stage-occt` recipe.
- **Windows today**: the official OCCT release zips fill the neutral install
  `dist/occt/win64/opencascade-8.0.1-vc14-64` (`occt-win.md`); that path is
  `crates/occt-sys/config.toml` `[windows].occt_prefix`, and `build.rs`
  stages the shim's DLL closure into the package payload.
- **OpenCV**: no binaries exist anywhere in this repo and never did. The
  `aruco` crate is data-only ("No OpenCV link-time or runtime dependency",
  `aruco-svg.md`) and the package has zero OpenCV references. `opencv-dev`
  matters for a *future* Unity consumer (marker detection, `add-markers.md`).
- The tracked `package/com.greyhound.step/` tree has **no**
  `Runtime/Plugins/` content (those dirs are created only during assembly
  into `dist/package/`), and there is **no Unity project inside this repo**.

## Binary sources

- <https://github.com/TonyMarkham/occt-dev/releases> — OCCT **8.0.1**,
  tag `v0.1.0` (pre-release). Six archives per release: per platform
  (Windows x64, Linux x86_64, macOS arm64) a **`-dev.zip`** (full install
  tree: headers, import libs, CMake config, resources, env scripts) and a
  **`-dlls-only.zip` / `-libs-only.zip`** (50 runtime libraries + 3 legal
  texts, flattened at the zip root).
- <https://github.com/TonyMarkham/opencv-dev/releases> — OpenCV **5.0.0**,
  tag `v0.1.0` (pre-release). Same six-archive shape.
- Release facts that matter to this repo:
  - Linux/macOS toolkits are **unversioned** `libTK*.so` / `libTK*.dylib`
    (SOVERSION/VERSION properties removed — no symlink chains), matching
    `config.toml`'s `runtime_pattern` and the "real files, not symlinks"
    distribution requirement.
  - **No bundled third-party libraries** (FreeType, IPP not built) → the
    Windows `thirdparty_dir` staging (`3rdparty-vc14-64`) becomes obsolete.
  - Legal texts ship at the root of every libs-only zip → LGPL
    redistribution requirement is met by extraction, not by hand-copying.
  - SHA-256 checksums are pinned on each release page.
  - All archives within one release come from the same upstream tag.
  - **Unverified**: the Windows dev zip's directory layout — the release
    prose suggests a unified CMake layout, not the stock `inc/` +
    `win64/vc14/{lib,bin}` tree `config.toml [windows]` encodes today.
    Confirm from the actual zip before writing new layout keys.

## Decisions (settled; do not re-open without new information)

1. **Two-role split per dependency.** The `-dev.zip` extraction is the
   *build-time prefix* (headers + import libs the shim compiles against —
   `config.toml`'s `occt_prefix`). The libs-only zip is the *runtime
   payload pool*. The payload is still the **closure derived from the built
   shim** (11 of 50 toolkits, via `DT_NEEDED` / the MSVC DLL-closure list),
   not all 50 libraries.
2. **Same-tag invariant.** The dev zip and the libs zip must come from the
   same release tag — `config.toml` and the platform docs name it
   (currently `v0.1.0`, OCCT 8.0.1), so a mismatched pairing is visible by
   hand.
3. **`extern/` cache.** Fetched artifacts live in a gitignored, repo-local
   `extern/` cache, populated by pinned fetch recipes, never committed.
   (`extern/` over `vendor/`: vendor connotes *committed* vendored sources
   in Go/cargo/Composer; this cache is untracked by definition.) Tag-first
   layout — a release bump is one directory.
4. **Fetch tooling is a Rust crate** with its own local `config.toml`
   (`crates/extern-fetch/`, working name): per artifact — release URL,
   SHA-256, extraction destination — in the same OS-section gating style as
   `crates/occt-sys/config.toml` (missing OS section = platform gate).
   Just recipes become one-line launchers (`cargo run -p extern-fetch`),
   retiring the "Windows bodies single-line launchers / logic in
   `tools/*.ps1`" split: one Rust implementation serves all three OSes.
   Requirements: verify SHA-256 after download, fail loudly on mismatch;
   idempotent (destination present + provenance marker matching the config
   → no-op). Recommendations: blocking `ureq` + `zip`, direct release-asset
   URLs (no GitHub API), no async runtime; `assemble`-type recipes assert
   presence and do **not** auto-fetch.
5. **The Unity package provisions its native payload at load.** The UPM
   package ships light; C# tooling inside it downloads the pinned libs zip
   and extracts it into the project ("handshake"). UPM has no install-time
   hook — the handshake point is Editor assembly load
   (`[InitializeOnLoad]` / `[InitializeOnLoadMethod]`): cheap manifest
   check (tag + asset hash) → download keyed on `EditorApplication.platform`
   → SHA-256 verify in C# → extract → write manifest →
   `AssetDatabase.Refresh()`. A STEP import arriving before provisioning
   fails with a clear "payload not provisioned yet" message, not a cryptic
   dlopen error. The AGENTS.md native-distribution requirement flips from
   "ship with the package" to "provision at import". The extractor is
   zip-source-agnostic, so an air-gapped install could alternatively point
   it at a payload zip shipped in-package (a choice, not the default).
6. **Payload split follows load mechanics.** The host cdylib ships in the
   package (Unity-imported plugin asset, `DllImport` resolution); the shim
   and OCCT libs are dlopen'd by explicit path and come from the
   provisioned payload. License/exception/notice texts land beside the
   libs. Zip extraction yields real files.
7. **Package-tree payload is gitignored; `dist/` is deleted entirely.** No
   neutral install under `dist/`, no assembled copy under `dist/package/`.
   The built and provisioned payload lives inside the tracked package tree
   and is ignored there:
   - `package/com.greyhound.step/Runtime/Plugins/<arch>/` — host cdylib
     (cargo build output);
   - `package/com.greyhound.step/Runtime/Plugins/shim/<arch>/` — shim
     (cargo build output);
   - `package/com.greyhound.step/Runtime/Plugins/occt/<arch>/` —
     provisioned libs zip contents (OCCT libs + legal texts).
   `<arch>` is the platform's plugin architecture: `arm64` on macOS,
   `x86_64` on Linux/Windows (amended 2026-10-14, first macOS Unity
   exercise: Unity resolves native plugins per architecture folder and
   the arm64 editor never loads from `x86_64/`).
   All of `Runtime/Plugins/` is ignored (binaries plus Unity's minted
   `.meta` for them); authored sources keep tracked `.meta`.
   This supersedes the earlier "extract into a project-local hidden dir"
   idea: the provisioner's destination is the package tree's
   `Plugins/occt/`. Unity will import those files as assets (bounded
   `.meta` churn, all ignored).
8. **`extern-fetch` and the C# provisioner share one payload path and
   one config.** The C# provisioner does not embed pins: `just
   assemble-package` stages `crates/extern-fetch/config.toml` into the
   package as `Editor/occt-fetch.toml` and the provisioner reads its
   `[occt.<platform>.payload]` entry from that copy — one source of pins,
   refreshed on every assembly, no parity check needed. Ownership of the
   shared destination is the provenance marker's coordinates + hash, not
   the writing tool, so a marker either provisioner wrote certifies the
   same bytes to both. (Amended 2026-10-14: originally "C# pins embedded
   in the package + a parity check"; the staged copy replaces the
   embedded copy.) The payload entry supersedes the old `libs` entries:
   same archive, dest = the package payload's `lib/`. (Amended
   2026-10-14, later: **only the C# provisioner writes the package
   payload.** The CLI fetch skips `payload` entries — they remain in the
   config solely as the pins source for the staged copy; the CLI's job
   is the `extern/` dev trees, and it must never write into the dist
   package. The marker cross-recognition stays, but the CLI no longer
   provisions the payload, even on explicit request.)

## Repo layout after

```
package/com.greyhound.step/            # tracked — authored sources + committed .meta
  Runtime/Plugins/<arch>/              # ignored — host cdylib (cargo build output)
  Runtime/Plugins/shim/<arch>/         # ignored — shim (cargo build output)
  Runtime/Plugins/occt/<arch>/         # ignored — provisioned libs zip (libs + legal texts)
                                       # <arch>: arm64 on macOS, x86_64 elsewhere
extern/                                # ignored — fetched dev/libs zip extractions
  occt/8.0.1/
    win64-dev/  win64-libs/  linux-x86_64-dev/  linux-x86_64-libs/
    macos-arm64-dev/  macos-arm64-libs/
  opencv/5.0.0/
    ... (same per-platform shape)
```

`.gitignore`: `dist/` goes; `extern/` and
`package/com.greyhound.step/Runtime/Plugins/` arrive.

## What changes, per area

- **`AGENTS.md`**: the "user builds OCCT themselves" premise becomes "the
  user builds OCCT/OpenCV in the `-dev` repos; this repo consumes their
  releases"; the repo map gains `extern/` and `crates/extern-fetch/`;
  `dist/` and the dist-first Unity tweak workflow section are removed; the
  native-distribution requirements flip to provision-at-import.
- **`crates/occt-sys/config.toml`**: per-OS `occt_prefix` points at the
  `extern/…-dev` extractions; the Windows layout keys are rewritten from
  the confirmed dev-zip layout; `thirdparty_dir` goes.
- **justfile / tools**: `assemble-package` loses its meaning and becomes
  "build into the package tree" (cargo writes the shim and host straight to
  their runtime paths); `verify-package` re-targets the tracked tree and
  gains a provisioning prerequisite; `tools/assemble-package.ps1` retires
  into the Rust crate.
- **Platform docs** (`occt-win.md`, `occt-linux.md`, `occt-mac.md`): flip
  from configure/build/install to fetch-and-provision instructions; the
  from-source recipes now live in the `-dev` repos (their release notes
  cite their own copies). `tmp/` loses its purpose here and goes.
- **Unity package C#**: the provisioner (editor hook + download/verify/
  extract) lands in `Runtime/`; `NativeMethods.cs` path resolution points
  at the package tree's `Plugins/occt/`.
- **`better-occt-bins.md`**: superseded in its `dist/` aspects (neutral
  install → `extern/`, `stage-occt` → provisioning). Left untouched —
  the platform-docs flip bite retires it; do not edit it before then.
- **dist-first Unity tweak workflow**: deleted. Unity's `file:` manifest
  points at the tracked tree; C#/UXML/USS is authored in place, Unity
  mints `.meta` there, and those `.meta` files are committed. The
  "author in dist → validate → sync back → `diff -r` parity" loop collapses
  to "edit the tracked package; Unity is the validator".

## Landed

- **2026-10-14 — `extern-fetch` crate** (D4): `crates/extern-fetch/` with
  its own `config.toml` (occt group only; opencv waits for a consumer).
  SHA-256 verify, zip-slip-guarded extraction, sibling
  `<dest>.provenance.toml` markers, idempotent re-runs, and unpinned
  entries that extract + report the hash and fail the run until pinned.
  27 unit tests; end-to-end verified on Windows: `just fetch-occt`
  populated `extern/occt/8.0.1/` (dev + libs), hashes pinned, second run
  skips with exit 0. `.gitignore` gains `extern/`. Windows pins: dev
  `8ceafbaa…3b0dd11`, libs `7a336357…691203` (full values in the
  config). Linux/macOS entries are pinned too (2026-10-14, from their
  release pages); the downloaded bytes there are verified against the
  pins by the first `just fetch-occt` run on such a machine, which also
  answers the Linux dev-zip layout question the record left unverified.
  The libs fetch was ahead of any consumer — nothing reads
  `win64-libs` until the provisioning bite wires D7/D8; it validated the
  tool against the second archive shape (flattened 50 DLLs + 3 legal
  texts) and captured the pin so that bite needs no download.

Facts this landed run resolved:

- **The Windows dev zip is a unified CMake install tree** (not the stock
  `inc/` + `win64/vc14/{lib,bin}` official layout): `bin/` (50 DLLs),
  `include/opencascade/` (7,266 headers), `lib/` (50 import `.lib` +
  `cmake/opencascade`), `share/`, plus the 3 legal texts at the zip root.
  The libs-only zip is 50 DLLs + 3 legal texts, flat. The next bite
  rewrites `config.toml [windows]` to `include_dir = include/opencascade`,
  `lib_dir = lib`, `runtime_dir = bin`, and drops `thirdparty_dir`.

- **2026-10-14 — provisioning bite** (D5/D6/D8, Unity side pending Unity
  validation): the package provisions its OCCT payload at editor load.
  `extern-fetch` gains `[occt.<os>.payload]` entries (the old `libs`
  entries, retargeted at `dist/package/com.greyhound.step/Runtime/
  Plugins/occt/x86_64/lib`) and the provenance marker's ownership drops
  the `tool` field from `matches` so `just fetch-occt` and the C#
  provisioner honor each other's markers (28 tests pass, including the
  new cross-tool case). `build.rs` loses `stage_runtime_payload` /
  `pe_imports` and the `payload_dir`/`license_dir`/`runtime_dir` config
  keys — goblin stays only in `[dependencies]` for `dependencies.rs` —
  so a cargo build no longer wipes the payload; `assemble-package`
  (both recipes) stages the fetch config as the package's
  `Editor/occt-fetch.toml` and its prerequisites assert the provisioned
  payload. The C# provisioner (Editor/`StepToml`, `StepFetchConfig`,
  `StepProvenance`, `StepProvisionState`, `StepPayloadProvisioner` in the
  dist package) does marker check → `UnityWebRequest` download (held in
  memory by default; a per-machine `download_location` key at the top of
  the fetch config — empty in the tracked default, else an absolute
  pre-existing directory — stages the zip on disk instead, for machines
  whose IT policy locks down temp dirs; both provisioners honor the
  key, staging the zip there and deleting it after extraction) → SHA-256 →
  zip-slip-guarded extract to a staging dir → atomic move → marker
  write → `AssetDatabase.Refresh()`;
  `StepImporter` calls `EnsureReady()` first so an early import fails
  with a clear provisioning message. **Consequence, superseding D1's
  payload note:** the provisioned payload is the full libs-zip content
  (50 toolkits + 3 legal texts), not the shim-derived 23-DLL closure —
  the closure derivation needs the built shim's PE import table, which
  Unity does not have. **Validated in Unity 6000.7.0b4 (Windows),
  2026-10-14; synced to the tracked `package/` tree (hash parity on all
  six files).** The cold loop ran twice — payload dir deleted, Unity
  restarted, the provisioner downloaded the ~50 MB zip, verified the
  pin, extracted, wrote the marker, and the cart-pole STEP import
  built — and the marker it wrote (`tool = "com.greyhound.step"`)
  carries exactly the extern-fetch config's pin. Two Unity compile
  fixes are part of this bite: `PackageInfo` is ambiguous between
  `UnityEditor.PackageInfo` and `UnityEditor.PackageManager.PackageInfo`
  (CS0104) — the provisioner qualifies the PackageManager one — and
  Unity's statics lifecycle analyzer rejects unattributed static state
  (UAL0010) — the provisioner carries `[NoAutoStaticsCleanup]` +
  `partial`, mirroring `StepHost`. The warm path (marker present →
  silent no-op on domain reload) is designed but not yet observed in an
  editor session.

- **2026-10-14 — `occt-sys` build-input flip** (D1): `[windows]` in
  `crates/occt-sys/config.toml` points at `extern/occt/8.0.1/win64-dev`
  (`include_dir = include/opencascade`, `lib_dir = lib`,
  `runtime_dir = bin`); `thirdparty_dir` is gone from the config and from
  `build.rs` — the release builds no bundled third-party libraries, so
  the shim's DLL closure resolves from `bin/` plus the system directory
  alone. The staging step wipes the payload `lib/` first, making the
  payload exactly the closure derived from the built shim (23 TK DLLs +
  2 licenses for the cart assembly); a build killed mid-wipe leaves a
  damaged payload until the next staging. Verified on Windows: the shim
  compiles against the dev headers, links the dev import libraries, and
  `cargo test -p importer-host` (the real-OCCT assembly import) passes
  end to end on the dev runtime. The neutral install `dist/occt` is no
  longer a build input. `[linux]`/`[macos]` stay unchanged until their
  hashes are pinned there.

- **2026-10-14 — serial tessellation** (found by the flip's
  verification): `step_mesh.cpp` passed `isInParallel = true` to
  `BRepMesh_IncrementalMesh`; parallel meshing instantiates
  `OSD_ThreadPool::DefaultPool()` in TKernel, and its static destructor
  deadlocks `FreeLibrary(TKernel.dll)` under the loader lock on the
  occt-dev binaries — the full host pipeline hung at teardown, while the
  same sequence passed with the official release binaries. The shim now
  tessellates serially (`false`, with the invariant in a comment): no
  pool is ever instantiated, and the payload unloads cleanly. Cost on the
  cart assembly: `scene_counts` ~300 ms (parallel) → ~600 ms (serial).
  Revisit parallel meshing only together with the unload story and
  `perf.md`'s trigger; feedback for the occt-dev recipe is recorded in
  `occt-dev-feedback.md`.

- **2026-10-14 — Linux flip verified on the Linux box**: `just fetch-occt`
  ran on Linux for the first time, verifying both Linux pins against their
  release-page hashes. Facts this resolved: the Linux dev zip is the
  unified CMake install tree — `include/opencascade` (7,266 headers),
  `lib/` (50 unversioned `libTK*.so` + `cmake/`), `bin/`, `share/`, the 3
  legal texts at the zip root — so the `[linux]` keys in
  `crates/occt-sys/config.toml` stand as written and the UNVERIFIED-LAYOUT
  note is replaced by the confirmation. The stale old-model OCCT install
  inside `dist/package/…/Plugins/occt/x86_64` (bin/include/lib/share from
  the install-into-payload workflow, no provenance marker) was deleted by
  hand per the tool's rule, and the payload provisioned cleanly into
  `lib/` (50 libs + 3 legal texts). The shim compiled against the dev
  headers, `cargo test -p importer-host` passed end to end (the
  real-OCCT assembly import), and `just verify-package` passes — the
  `[linux] assemble-package` recipe now mirrors the Windows body,
  asserting the provisioned payload lib + built shim instead of an
  in-package OCCT install. (Fix mid-bite: the first cut carried the old
  model's `rm -f` of the host cdylib ahead of the prerequisite check, so
  a failed run damaged the assembled package; removed — every step
  before the check is non-destructive and the host copy overwrites in
  place, matching the Windows body.) Two `extern-fetch` tests assumed
  Windows-hosted path shapes and failed on Linux: the `C:/extern` dest
  rejection (which check fires is host-dependent) and the `D:/no-such-dir`
  download_location existence check (not absolute on Linux); both now
  branch on `HostOs::current()` — 38 pass on Linux, the Windows
  expectations unchanged.

- **2026-10-14 — CLI fetch no longer writes the package payload** (D8
  amendment, user decision: only the C# fetcher puts anything in the
  dist package): `main.rs` skips `payload`-role entries when fetching —
  they stay in the config as the staged-copy pins source. The assemble
  prerequisite messages (both platforms) no longer point at
  `just fetch-occt` for the payload; they name the C# provisioner. This
  resolves the day's provisioning fight: the CLI had been re-extracting
  the payload into the package on every fetch, stepping on the C#
  provisioner's exclusive ownership.
- **2026-10-14 — Rust side never touches the package payload, reads
  included** (rule as stated: NOTHING on the Rust side should EVER
  touch `dist/package/…/Plugins/occt`): the real-OCCT tests
  (`importer-host` `host_abi.rs`) and the root `config.toml` consumers
  (`greyhound-export-mjc`, `step-stats`) now read their runtime
  libraries from the `extern/` dev trees — the Rust side's sandbox —
  instead of the provisioned payload. The root config's `library_dir`
  moved into per-OS `[occt.<os>]` sections (the dev-tree runtime dir is
  `bin/` on Windows, `lib/` elsewhere; missing section = platform gate,
  mirroring `crates/occt-sys/config.toml`); both crates' `OcctConfig`
  select and resolve the build target's section at load. `build.rs` was
  already clean (shim-only writer; the payload staging was removed with
  the provisioning bite). Verified with the payload directory removed:
  `cargo test --workspace` passes fully without it (0 failures across
  all crates, clippy clean), and the whole just workflow was then
  decoupled from the payload at the user's ruling: `assemble-package`
  (both platforms) asserts only the shim it just built; `verify-package`
  loads the dev-tree runtime (repo root anchored on the script path)
  and the Linux copied-layout check excludes `Plugins/occt` from the
  copy; the recipe's file listing prunes the subtree. Verified end to
  end with the payload directory physically absent: `just verify-package`
  passes (43 nodes, 20 meshes, mass properties). The payload
  (`Runtime/Plugins/occt`) is now written and read by the C# provisioner
  alone, on every platform.
- **2026-10-14 — macOS plugin-architecture directories** (first macOS
  Unity exercise, arm64): Unity resolves native plugins per
  architecture folder, and the arm64 editor never loads from `x86_64/`
  — the staged host cdylib there failed with
  `DllNotFoundException: importer_host` while everything dlopen'd by
  explicit path (shim, payload) kept working. `<arch>` is now per-OS
  everywhere the layout names it: the host plugin folder
  (`assemble-package` branches on `uname`; verify-package follows), the
  shim dir (`shim_dir` moved from the top level into the per-OS
  sections of `crates/occt-sys/config.toml` and the root `config.toml`,
  `build.rs` reads it from the section, the Rust consumers' `OcctConfig`
  select it per target), the payload dir (the C# provisioner and
  `NativeMethods` derive it at runtime via `NativeMethods.PluginArch()`),
  and the `[occt.macos.payload]` dest. The arm64 macOS tessellation of
  cart-asy differs from the Linux pin by one ring band on a curved face
  (64 verts / 128 tris, deterministic run to run) — the verify script's
  exact totals now assert on Linux only, per user decision, printing
  the measured totals everywhere. Verified on the arm64 Mac: cargo
  build/test green across the workspace, `just verify-package` passes,
  and the STEP imports in Unity 6000.7.0b4.

## Open items

- **Shim + host distribution for consumers outside this repo.** Consumers
  get the OCCT libs via handshake, but the shim and host cdylib are local
  build outputs with no home: either this repo publishes a runtime payload
  release (shim + host + license texts) that the C# provisioner can also
  pull, or the package simply requires a local build. Unresolved; settle at
  the first consumer-facing bite.
- **OpenCV consumer**: `opencv-dev` archives are ready, but no Unity or
  Rust consumer links OpenCV yet; wire the fetch config when the first one
  lands (marker detection per `add-markers.md`).

## References

- `better-occt-bins.md` — the neutral-install plan this record supersedes
  in its `dist/` aspects
- `crates/occt-sys/config.toml` — build-time input contract (`occt_prefix`)
- `AGENTS.md` — native distribution requirements, bite workflow
- `occt-to-unity.md` — gap table (G1–G12); consult before related work
- `add-markers.md` — the future OpenCV consumer (marker detection)