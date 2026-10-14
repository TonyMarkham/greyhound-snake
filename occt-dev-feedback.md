# occt-dev-feedback — parallel-mesh unload deadlock in the occt-dev build

Feedback for the `TonyMarkham/occt-dev` repository (local checkout
`D:\git\occt-dev`, recipe: its `occt-win.md`): binaries built by that
recipe **deadlock at unload after any parallel BRepMesh tessellation**.
This repo meshes serially and is unaffected today; this note exists so
the build difference gets investigated instead of forgotten.

## Symptom

A process that tessellates with `isInParallel = true` and then releases
the OCCT runtime hangs forever inside `FreeLibrary(TKernel.dll)` — the
last library released in the payload's parents-first unload order. The
same process without tessellation, or tessellating serially, unloads in
milliseconds. Found while verifying the `extern/…-dev` build-input flip
(`refactor-repo.md`, Landed 2026-10-14).

## Evidence

Proven (probe runs against the staged payload):

- Tessellation is the trigger: `host_new → open_step → close_step →
  host_free` unloads clean; adding `scene_counts` (which tessellates)
  makes `host_free` hang.
- The hang is exactly inside `FreeLibrary(TKernel.dll)` — instrumented
  `Library` drop; the shim and the other 22 TK DLLs release normally.
- Serial tessellation (`isInParallel = false`) never instantiates
  OCCT's thread pool and unloads clean.
- Parallel meshing itself works: identical meshes, ~2× faster on the
  cart assembly (~300 ms vs ~600 ms `scene_counts`).

Inferred (no debugger attached; `cdb` is not installed):

- The deadlocking frame is the static destructor of
  `OSD_ThreadPool::DefaultPool()` — a TKernel static instantiated by
  parallel meshing — joining its worker threads under the loader lock.

## Why the build is suspected

The official V8.0.1 release binaries do **not** have the issue: the same
shim, the same `isInParallel = true`, and the same unload sequence passed
with them before the flip (`occt-win.md` here installs exactly those
binaries, and its real-OCCT tests were the passing baseline). The
difference is what got compiled in. The official payload's third-party
DLLs were staged from the official TK DLLs' own import tables — each was
actively imported — while the occt-dev build bundles nothing:

| Runtime | Official payload | occt-dev build |
|---|---|---|
| `tbb12.dll` | imported | absent (`USE_TBB` defaults OFF) |
| `jemalloc.dll` | imported | absent (`USE_MMGR_TYPE` defaults NATIVE) |
| `freetype.dll` | imported | absent (`USE_FREETYPE=OFF`) |
| FreeImage, ffmpeg, openvr | imported | absent (none bundled) |

`occt-dev`'s `occt-win.md` documents the relevant default itself: without
`-DUSE_TBB=ON`, "OCCT's native parallelism is used" — i.e.
`OSD_ThreadPool`, the component whose teardown hangs. The open question
is whether TBB presence is what saves the official binaries (does
TBB-backed parallelism replace the native pool for BRepMesh, or is the
difference elsewhere, e.g. the memory manager?).

## Verification plan

1. In `occt-dev`: reconfigure with `-DUSE_TBB=ON` plus
   `-D3RDPARTY_DIR=<official 3rdparty-vc14-64 bundle>` (its
   `occt-win.md` documents the bundle-discovery pattern in the FreeType
   section), rebuild, re-release.
2. Here: flip `crates/occt-sys/cpp/step_mesh.cpp`'s `isInParallel` back
   to `true`, restage, and run `cargo test -p importer-host` — the
   real-OCCT assembly test exercises tessellation plus the full unload.
   A clean unload means the recipe + TBB explains the official behavior,
   and the shim returns to parallel meshing; the serial choice and its
   comment in `step_mesh.cpp` are then revisited together with
   `perf.md`.

## Current posture (this repo)

`step_mesh.cpp` tessellates serially, with the invariant comment, so no
pool is ever instantiated and the payload unloads cleanly on any build.
Cost: `scene_counts` ~300 ms (parallel) → ~600 ms (serial) on the cart
assembly — a `perf.md`-trigger concern, not a correctness one. Do not
re-enable parallel meshing until the occt-dev teardown story is
resolved.

The choice is deliberately a hard-coded token, not a knob: if a knob is
ever justified, the house-shaped form is a build-time `config.toml` key
compiled through `build.rs` — not an ABI or environment runtime setting,
which would touch the host ABI and C# for a boolean with no consumer.