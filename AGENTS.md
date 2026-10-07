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

## Repository map

- `occt-linux.md`, `occt-mac.md`, `occt-win.md`: user-facing OCCT build,
  installation, validation, and distribution instructions.
- `crates/occt-sys/`: Rust OCCT integration with C++ shims, including STEP
  loading, document information, and tessellation.
- `.cargo/config.toml`: currently points `OCCT_PREFIX` at the repo-local
  Linux installation in `dist/x86_64-linux`.
- `dist/`: repo-local native distributions; currently gitignored.
- `tmp/`: disposable OCCT source and build scaffolding; currently gitignored.
- `assets/`: sample assets, including a STEP file.

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
- Use explicit Rust imports. Never use glob imports in a `use` statement.
- Put Rust test modules under each crate's `src/tests/` directory, registered
  through `src/tests/mod.rs` and a `#[cfg(test)] mod tests;` in the crate root.
- Keep early-stage changes focused on the requested task; do not invent an
  unrequested package layout or architecture.
