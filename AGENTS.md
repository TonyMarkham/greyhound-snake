# Project purpose

This repository is near the beginning of development. Its intended product is
a **Unity Scripted Importer that uses Open CASCADE Technology (OCCT) to import
STEP files into Unity**.

The importer will be distributed as a **Unity Package Manager (UPM) package**.
The OCCT native binaries needed by the importer must be distributed inside
that package. The current Rust workspace and C++ shims are groundwork for
this integration, not a finished Unity package.

## Working with the user

- Follow the scope of the user's explicit request. Do not assume permission
  to perform additional work merely because it would advance the project.
- **The user builds OCCT themselves.** The platform documents give the user
  instructions to execute; they are not instructions for the agent to run.
- Do not run OCCT configure, build, install, or cleanup commands, install
  system dependencies, or launch native validation examples unless the user
  explicitly asks you to execute them. This also applies to commands that
  would trigger those operations indirectly.
- When asked to edit documentation, edit the documentation. When shown an
  error, investigate and explain its cause and the instructions the user
  needs to follow. An error report alone does not authorize executing the
  remedy or making additional edits.
- Reading files and inspecting source to answer a question is appropriate.
  Do not turn a question into an implementation task.
- Clearly distinguish source inspection from an executed build or test.
  Do not claim validation that has not actually happened.
- Preserve existing user changes. Do not commit, stage, or discard changes
  unless explicitly requested.

## Bite workflow (worktree prototype + guided implementation)

Implement each bite (vertical slice) through the established loop:

1. **Prototype in a worktree.** Create a detached worktree at HEAD
   (`git worktree add --detach ../<repo>-<bite> HEAD`) with a `dist`
   symlink to the main checkout's `dist`. Implement and validate there —
   build, tests, recipes — without touching main repo source.
2. **Write the implementation doc from the diff.** Capture the complete
   diff (`git add -N` for untracked files, then `git diff`) and write a
   bite doc in the repo root mechanically from it, in the guided-implement
   skill's step formats: one atomic action per step (`Target`/`Intent`/
   `Run Manually` or `Find`/`Replace With` with exact landmarks / `Why`),
   each step ending with its `Stop after …` line. Verification steps carry
   expected output. Steps the user already applied by hand go into a
   verified "Current state" note instead of re-derived steps.
3. **The user applies via the guided-implement skill** and commits. Under
   that skill the agent never edits main repo source; it verifies each
   applied step against the doc's landmarks and stops on mismatch. If the
   user takes over direct application mid-bite (including agent-edited
   fixes on request), main becomes the source of truth: keep all synced
   copies consistent and say so explicitly.
4. **Clean up on request.** Diff the worktree against committed main for
   parity (main wins on the user's formatting choices; fix only real
   omissions such as missing EOF newlines), then remove the `dist` symlink
   and `git worktree remove --force`.

## Repository map

- `occt-linux.md`, `occt-mac.md`, `occt-win.md`: user-facing OCCT build,
  installation, validation, and distribution instructions.
- `crates/occt-sys/`: Rust OCCT integration with C++ shims, including STEP
  loading, document information, and tessellation.
- `crates/importer-host/`: Unity-facing cdylib — a flat, version-gated C ABI
  (open STEP, two-phase mesh counts/fill) that loads OCCT via `occt-sys` and
  emits projected Unity buffers.
- `.cargo/config.toml`: points `OCCT_PREFIX` at the OCCT install inside the
  package (`dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64`)
  and `OCCT_SHIM_DIR` at the shim's own plugins directory — OCCT installs
  and the shim compiles directly into the package layout.
- `dist/`: repo-local native distributions, including the fully assembled
  UPM package in `dist/package/`; currently gitignored.
- `package/com.greyhound.step/`: tracked UPM package skeleton — authored
  sources (`package.json`, `Third Party Notices.md`, and the C# code: the
  `Runtime/` P/Invoke layer with struct mirrors, the `Editor/`
  `ScriptedImporter`). `just assemble-package` stages it with the native
  payload into `dist/package/com.greyhound.step/`; `just verify-package`
  checks the staged package from a copied layout
  (`tools/verify-package.py`).
- `tmp/`: disposable OCCT source and build scaffolding; currently gitignored.
- `assets/`: sample assets, including a STEP file.

Design reference docs (source of truth for their facts; consult before
mesh/projection work instead of re-deriving):

- `occt-mesh.md`: OCCT 8.0.1 mesh data model and tessellation API facts,
  read from the installed headers (`Poly_Triangulation`, `BRepMesh`,
  extraction, parameter tables); includes the no-own-mesh-format conclusion
  and available interchange writers.
- `unity-mesh.md`: Unity 6.0 `Mesh` API facts — advanced/data-oriented API
  call order, vertex-layout rules, submesh descriptors, index format,
  winding conventions; source doc for the C# side.
- `occt-to-unity.md`: the OCCT→Unity transformation pipeline with the
  derivation (axis map, winding flip, scale, vertex layout), an audit of
  current shim/Rust state, the gap list **G1–G12 (the working backlog)**,
  and the import verification checklist.
- `perf.md`: benchmark plan (criterion, micro/macro layers, assets),
  the scalar-first SIMD decision and its revisit trigger, and rules for
  recording timings.

## Decided architecture direction

Decisions below are settled; the referenced docs hold the rationale. Do not
re-litigate them without new information, and keep them consistent when
implementing.

- The host-neutral mesh abstraction is a **Rust core mesh model** (built,
  gap G3 done): flat `f32` positions + `u32` indices + per-face ranges, in
  OCCT coordinates (mm), triangles outward-CCW in OCCT algebra.
  `Poly_Triangulation` is an internal detail behind the C++ shim, not the
  abstraction; no OCCT type crosses an ABI.
- Layering: OCCT knowledge stays in the C++ shim (1-based→0-based indices,
  `TopLoc_Location` transforms, `TopAbs_REVERSED` fix); host-specific work
  (axis permutation, Unity winding flip, scale, submesh assembly, buffer
  layout) lives in a Rust Unity projection (gap G4 done) so the C# side is a
  pure blit. Blender later consumes the same core model through its own
  projection — never through Unity assumptions.
- The Unity-facing surface is the `importer-host` cdylib: a flat,
  version-gated C ABI (two-phase counts/fill into caller-provided buffers).
  C# P/Invokes it and contains no OCCT or projection logic.
- Coordinate map: `Unity = (x, z, y)` of OCCT (det −1; the non-mirroring
  map). The projection flips two indices per triangle as a consequence;
  normals are permuted but never negated. Never mirror by negating an axis.
- Scale is baked into vertices by the projection (recommended default
  0.001, mm→m); GameObject transform stays identity. Core model stays mm.
- Unity C# side targets the advanced Mesh API
  (`SetVertexBufferParams` → data → `SetIndexBufferParams` → data →
  `SetSubMeshes` → bounds), `UInt32` indices, interleaved
  pos+normal stream, no `TexCoord0` for v1 (G2 decided: UVs omitted).
- The Unity projection is scalar first; SIMD only if profiling shows it
  matters (`perf.md` revisit trigger).
- Remaining open decisions (G6) and deferred items are tracked
  in the `occt-to-unity.md` gap table — consult it before proposing mesh or
  projection work.

## Native distribution requirements

- Plan for the OCCT libraries to ship with the UPM package, rather than
  requiring Unity users to install OCCT separately.
- Packaged native libraries must be real files, not symlinks.
- Include the runtime dependency closure needed by the importer; the
  libraries named in `build.rs` are only the link-time list.
- Native library loading and any required OCCT resource lookup must work
  when the package is installed at a different path on another machine.
  Absolute paths into the developer's checkout are not a distribution
  solution.
- Include the OCCT license, exception, and applicable source notices with
  redistributed OCCT binaries.
- Do not assume that platform build documentation means the corresponding
  Unity integration or packaged binaries have already been implemented or
  validated.

## Documentation and code changes

- Check build instructions against the relevant OCCT release's source.
  Distinguish stock configure options from local source modifications.
- Keep commands, paths, install-layout examples, and troubleshooting
  consistent. Make prerequisites and the configure → build → install order
  explicit.
- Follow the existing Rust/C++ structure and workspace lint settings.
- Cargo.toml pattern: all dependencies are declared once in the workspace
  `[workspace.dependencies]` (versions live only there); member crates
  reference them as `name = { workspace = true }`, inherit
  `version`/`edition` from `[workspace.package]`, and set
  `[lints] workspace = true`. Never pin a version inside a member crate.
- Use explicit Rust imports. Never use glob imports in a `use` statement.
  Write one `use` statement per source crate (`crate`, `std`, and each
  external crate), grouping all imported items from that crate into nested
  braces instead of scattered single-item lines (see
  `crates/occt-sys/src/step_doc.rs` for the pattern).
  Order: `crate::` first, blank line, workspace crate(s) (`{crate-name}::`),
  blank line, then all other crates (`std`, externals).
- Numeric conversions: `From` exists only for lossless-on-every-target
  widenings — `u8`/`u16` → `u32`/`u64`/`usize`, and `u32` → `u64`. There is
  no `From<u32> for usize` and no `From<usize> for u64`. Convert `usize`
  lengths to numeric domains via `u32::try_from(len)` with a real error
  (they are index-bound semantics), and do mixed-domain arithmetic in `u64`
  via `u64::from(u32)`; never use `as` casts to paper over the gap.
- One Rust type per file. For grouped/related types, create a module
  directory with a `mod.rs` re-exporting its members (see
  `crates/occt-sys/src/error/` for the pattern).
- Unit tests are never inlined in source files. They always live in a
  `src/tests/` module directory, registered through `src/tests/mod.rs` and a
  `#[cfg(test)] mod tests;` in the crate root.
- Tests may use `unwrap`/`panic` and generic `Result` types. Library source
  code uses the thiserror + error-location pattern implemented in
  `crates/occt-sys/src/error/`: a crate-local `Result<T>` alias, a
  `thiserror` enum whose variants carry an `ErrorLocation` captured via
  `#[track_caller]` constructors, so every error reports where it was raised.
- Keep early-stage changes focused on the requested task; do not invent an
  unrequested package layout or architecture.
