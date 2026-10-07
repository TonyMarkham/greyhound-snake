# Performance benchmarks

Plan for measuring the STEP-import pipeline in this repo. Written 2026-10-07,
when the pipeline is `STEP open → tessellate → extract` and the mesh model /
Unity projection do not exist yet (`occt-to-unity.md` gaps G3/G4). The plan
anticipates them; adjust as the code lands.

## Why benchmarks here

- The pipeline is a one-shot import whose cost is dominated by native OCCT
  tessellation. Intuition about where time goes is currently estimates (e.g.
  the SIMD decision below) — benchmarks turn them into numbers.
- Deflection tuning has no absolute answer: the cost/quality curve is the
  data that picks defaults.
- ABI and projection changes should be gated against saved baselines, like
  any other regression.

## Tool and layout

- **`criterion`** in `crates/occt-sys/benches/`, registered as `[[bench]]`
  entries in `crates/occt-sys/Cargo.toml` (parallel to the `src/tests/`
  convention).
- Criterion forces `--release`; never measure debug builds.
- Keep a named baseline before any ABI/mesh-model change and compare against
  it after (`cargo bench -- --save-baseline <name>` /
  `--baseline <name>`).

## Two measurement layers

### 1. Micro (pure Rust functions)

Target: functions with no OCCT in the loop — once the mesh model and Unity
projection exist: coordinate permutation + winding flip + scale (per 1k
vertices), bbox map, model construction/copy costs.

- These are cheap (`ns`–`µs` range); criterion's statistics are the point.
- This is the layer where the **SIMD decision** (below) gets re-litigated
  with data.

### 2. Macro (pipeline stages on real files)

Target: wall time per stage on real STEP input:

```
open (STEP parse + transfer)
mesh_counts (tessellation — the expensive native stage)
mesh_fill (extraction into Rust buffers)
project (Unity projection, once it exists)
```

- Use `Instant` splits around each ABI call, aggregated by criterion as a
  normal timed routine.
- Runtime needs the real shim + OCCT libraries from `dist/` — same
  resolution as the loader tests (`OCCT_PREFIX` via `.cargo/config.toml`;
  see `src/tests/runtime_loading.rs`). No mocks: the numbers only mean
  something against the real libraries.
- Report both **absolute stage time** and **µs per 1k triangles**
  (triangle count from `greyhound_step_info`/mesh counts) — normalized
  numbers survive deflection changes; absolute ones expose fixed overhead.
- Deflection sweep (0.1 / 0.01 / 0.001 mm) as a parameterized bench: shows
  the tessellation cost curve and mesh-size explosion — the argument for
  per-part meshes and progress/cancel (G12) later.

### Assets

- `assets/rod-clamp-16mm.stp`: fine for extraction/projection micro-benches;
  too small for meaningful tessellation statistics.
- Larger corpus needed for the macro layer. Options: ship a bigger STEP
  asset (license-clean), or generate synthetic parts through OCCT primitives
  (needs a new shim entry point). Open decision — pick when the macro bench
  lands.

## Metrics to add later

- Peak memory per stage (allocation profile): the f64→f32 extraction and the
  projection's buffers roughly double memory at import time.
- The C# blit side (`SetVertexBufferData` etc.) once the Unity package
  exists — measurable from an Editor script.

## SIMD decision note (recorded rationale)

Decision: **scalar first for the Unity projection.** Rationale, to be
checked against micro-benchmarks:

- The projection is memory-bandwidth-bound (streaming ~12 MB positions +
  ~12 MB normals for a 1 M-vertex part, one multiply + element moves per
  float); SIMD's ALU win buys little when the bus is the limit.
- It runs once per import and is dwarfed by tessellation — projected ~1%
  of total time. Wrong line item to optimize first.
- The scale multiply autovectorizes trivially; the `(x, z, y)` permutation
  is a strided shuffle that LLVM's vectorizer handles poorly and will likely
  stay scalar. Fine — reaching for `std::arch` intrinsics per-target in a
  learning codebase for a ~1% step is a bad trade.
- Escalation order if profiling disagrees: (1) `rayon` parallelism — for
  memory-bound work, cores beat lanes; (2) portable SIMD (`std::simd`
  nightly or a runtime-dispatch crate like `pulp`), not raw intrinsics.

Revisit trigger: micro-benchmarks showing the projection taking > ~5% of
total import wall time on a representative part.

## Rules for recording numbers

- Native OCCT timings vary by machine, compiler, and OCCT build. Never write
  absolute timings into docs as facts — record them as observations with
  machine context (CPU, RAM, OCCT build type) in commit messages, PR
  descriptions, or a `benches/README` results log.
- Compare only within a machine, against a saved baseline.
