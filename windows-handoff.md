# Verifying the feature/windows branch on Linux

One-shot handoff for the session that verifies this branch on the Linux
machine. Delete this file once the checklist passes and the branch merges —
a note that outlives its purpose becomes a lie.

Mission: **verify parity, don't redesign.** Everything here was executed and
green on a Windows machine (`cargo build`/`clippy`/`test --workspace` all
pass there, the shim's exports verified with `dumpbin /EXPORTS`); none of it
has run on Linux.

## Checklist

1. **Before the first build, save the parity oracle:**

   ```
   cp dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so /tmp/shim-before.so
   ```

   The existing shim was built by the previous `build.rs`. If the rebuild
   matches it, the gnu flag table is proven byte-identical by execution,
   not by reading.

2. `cargo clippy --workspace` — expect clean, zero warnings.
3. `cargo build --workspace` — expect success, and expect **no**
   `Runtime payload staged` warning: the payload staging is gated on the
   `[windows]` section's `runtime_dir`, whose absence from `[linux]` is
   deliberate, not an omission.
4. Compare the rebuilt shim against the oracle:

   ```
   sha256sum /tmp/shim-before.so dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so
   ```

   Must match. If it differs: `readelf -d` (SONAME, DT_NEEDED) and
   `nm -D` (exports) before suspecting the flag table. The payload's
   `libTK*.so` files must be untouched — no staging runs on Linux.
5. `cargo test --workspace` — expect all green. The tests now pass shim
   *directory* semantics; the resolved shim path on Linux is identical to
   before (`shim_dir` + the loader-appended `libgreyhound_occt.so`).
6. The `occt-linux.md` §3 validation: `cargo run -p step-stats --
   assets/rod-clamp-16mm.stp`, then `just assemble-package && just
   verify-package`. The packaging flow is untouched and must stay green.

## Gotchas

- Root `config.toml` uses `shim_dir` now (was `shim_path`): the loader
  appends the platform's shim filename from the crate config's compiled
  `OCCT_SHIM_FILENAME`. Resolved path identical on Linux; some error
  strings changed slightly ("cannot locate shim directory").
- `.cargo/config.toml` is deleted. `build.rs` reads
  `crates/occt-sys/config.toml`; grep confirms nothing else ever consumed
  the old `OCCT_PREFIX`/`OCCT_SHIM_DIR` env vars.
- The fixture `.so` files may not be byte-identical to their previous
  builds (fixtures now get `-DGREYHOUND_API=` and explicit
  `visibility("default")` on definitions — same exported set). Harmless;
  the real shim is the parity oracle.
- Read the docs fresh — `occt-win.md` ("How this relates to the repo"),
  `better-occt-bins.md`, and the crate-local `config.toml` were all
  rewritten in this branch. Stale memory of their old content is the
  likeliest source of wrong conclusions.
- Do not add `runtime_dir` to `[linux]` — that is the separate
  neutral-install + `stage-occt` bite described in `better-occt-bins.md`,
  not this branch.
- The C ABI `greyhound_host_new`'s second parameter now receives a shim
  **directory** (the loader appends the filename). Linux resolves the same
  path as before; the C# P/Invoke layer in the dist package must pass the
  directory when the Unity-side work lands — known, named consequence.
