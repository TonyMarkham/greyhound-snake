# better-occt-bins — neutral OCCT install + selective staging

Redesign of the OCCT payload workflow: build OCCT to a neutral repo-local
location and copy **only the shared libraries the runtime needs** into the
UPM package. Replaces the current install-into-the-package layout, which
ships the entire OCCT installation — 157 MB / ~15,000 files — and makes
Unity import all of it as assets.

## Measured facts (current payload, 2026-10-08)

Payload at `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/`:

| Dir | Files | Size | Verdict |
|---|---|---|---|
| `include/opencascade/` | 14,533 (6,878 `.hxx`) | 73 MB | build-time only — the shim compiles against it; never used at runtime |
| `lib/` | 50 `.so` + `cmake/` (184 K) | 82.7 MB | runtime needs **11 libs = 32.7 MB**; the other 39 (50 MB) are dead weight |
| `share/opencascade/` | 322 | 5 MB | runtime resources mixed with docs, samples, XRResources |
| `bin/` | 6 (env.sh, custom_*.sh) | 32 K | build helpers; unneeded at runtime |

- **Flat linking**: the installed OCCT libraries carry no inter-library
  `DT_NEEDED` (`readelf -d libTKernel.so` lists only system libs) — the
  runtime closure is therefore exactly the toolkits the shim links, which is
  `crates/occt-sys/build.rs`'s list: `TKernel, TKMath, TKGeomBase, TKBRep,
  TKPrim, TKTopAlgo, TKMesh, TKDESTEP, TKLCAF, TKXCAF, TKXSBase`
  (**32.7 MB**; largest: TKDESTEP 9.6 MB, TKGeomBase 6.1 MB).
- `dependencies.rs` preloads exactly that `DT_NEEDED` chain — anything else
  in `lib/` is never touched.
- Unity generated **7,537 `.meta` files** over the package, importing every
  header, license text, and shell script as assets. Only
  `libimporter_host.so` needs Unity's plugin import (`DllImport`
  resolution); the shim and OCCT libs are `dlopen`'d by explicit path —
  they must exist on disk, nothing more.
- OCCT's CMake has **no "install only libs" switch**: headers
  (`install(FILES ${OCCT_HEADER_FILES_INSTALLATION})`), resources (the
  `adm/RESOURCES` list), `env.sh`, and `OpenCASCADEConfig.cmake` all install
  unconditionally (`tmp/occt/OCCT/CMakeLists.txt`); library granularity is
  per-module only (`BUILD_MODULE_<Name>`). `BUILD_MODULE_Visualization=OFF`
  (and Draw, already off) is the only useful configure-side trim.

## The workflow

1. **Neutral install**: OCCT 8.0.1 installs to `dist/occt/x86_64/`
   (repo-local, gitignored like the rest of `dist/`). `occt-linux.md` gains
   the new path; the existing install can be `mv`-ed — no rebuild required
   (the shim has no rpath into the old location; it loads via
   `dependencies.rs` from the package).
2. **`.cargo/config.toml`**: `OCCT_PREFIX` points at `dist/occt/x86_64`
   (shim compilation: headers + link libs); `OCCT_SHIM_DIR` unchanged — the
   shim still compiles into the package layout.
3. **`just stage-occt`**: derives the closure from the freshly built shim's
   `DT_NEEDED` list (`readelf`), copies those libs plus
   `LICENSE_LGPL_21.txt` and `OCCT_LGPL_EXCEPTION.txt` (the LGPL
   redistribution requirement) into
   `dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib/`.
   Fails loudly if any needed lib is missing from the neutral install.
   Deriving from the shim keeps the recipe correct if `build.rs`'s toolkit
   list ever changes. `assemble-package` depends on it.
4. **Package payload after**: `lib/` = 11 `.so` + 2 license texts (~35 MB
   total payload with shim and host); no `include/`, no `share/`, no `bin/`,
   no `cmake/`; Unity-visible file count drops from ~15,000 to ~25.

**Unchanged** (paths still valid, now with less content): the root
`config.toml` (`library_dir`/`shim_path` point into the package),
`NativeMethods.cs` (package-relative plugin dirs), `verify-package.py`,
`dependencies.rs`.

## Open question — STEP resources

Every import so far ran with the full `share/opencascade/` tree present;
whether STEP reading needs any of `resources/` (XSAlgo, Units, StdResource,
…) is **untested**. Sequence: stage without resources → run `step-stats` +
`just verify-package` on both sample assets → if something fails, add back
only the needed resource directories and pin the lookup explicitly (the
shim sets `CSF_OCCTResourcePath` to the packaged path) so ambient `CASROOT`
discovery never matters. Docs/samples/XRResources are pruned regardless.

## Optional configure-side trim

`BUILD_MODULE_Visualization=OFF` at configure time (nothing links it) and
`BUILD_MODULE_Draw=OFF` (already off) shrink the build itself — build time
and the neutral install's footprint. Purely optional; staging is what
defines the package.

## References

- `occt-linux.md` — install instructions (path update lands with the bite)
- `AGENTS.md` — native distribution requirements
- `tools/verify-package.py` — end-to-end validation constants
- `unity-mujoco.md` — the first exporter consumer of the slimmed payload
