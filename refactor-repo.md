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
   - `package/com.greyhound.step/Runtime/Plugins/x86_64/` — host cdylib
     (cargo build output);
   - `package/com.greyhound.step/Runtime/Plugins/shim/x86_64/` — shim
     (cargo build output);
   - `package/com.greyhound.step/Runtime/Plugins/occt/x86_64/` —
     provisioned libs zip contents (OCCT libs + legal texts).
   All of `Runtime/Plugins/` is ignored (binaries plus Unity's minted
   `.meta` for them); authored sources keep tracked `.meta`.
   This supersedes the earlier "extract into a project-local hidden dir"
   idea: the provisioner's destination is the package tree's
   `Plugins/occt/`. Unity will import those files as assets (bounded
   `.meta` churn, all ignored).
8. **`extern-fetch` and the C# provisioner share one payload path.** For
   non-Unity validation, `extern-fetch` can extract the libs zip into the
   same `Plugins/occt/` destination; manifest/hash checks keep the two
   provisioners idempotent against one location. Pins parity: the C# pins
   (embedded in the package, self-contained) must reference the same
   release tag + hashes as `extern-fetch`'s config — two copies, one
   contract, kept visible by a parity check (verify-package or a test).

## Repo layout after

```
package/com.greyhound.step/            # tracked — authored sources + committed .meta
  Runtime/Plugins/x86_64/              # ignored — host cdylib (cargo build output)
  Runtime/Plugins/shim/x86_64/         # ignored — shim (cargo build output)
  Runtime/Plugins/occt/x86_64/         # ignored — provisioned libs zip (libs + legal texts)
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
  install → `extern/`, `stage-occt` → provisioning). Update or retire it
  with the bite that lands this — until then it is a plan, not state.
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
  config). Linux/macOS entries exist but are unpinned until run there.
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