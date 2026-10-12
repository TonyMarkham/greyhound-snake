# ArUco 4x4 SVG generator (library + CLI)

Plan for a Rust library and CLI that generate 4x4 ArUco markers as SVG
files, compatible with OpenCV's predefined dictionaries. Written
2026-10-12 before any implementation work, with every factual claim
grounded in a named source (repo file or OpenCV source file, cited
inline); this doc is the feature definition — consult and update it as
bites land.

Cross-reference: `add-markers.md` plans the *scene-side* marker authoring
(coord nodes, manager, synthetic capture). This doc covers the
*physical-print* side: the marker designs themselves as printable SVGs.
No prior repo decision covers dictionary generation.

## Decided

- **Exact OpenCV compatibility.** Every emitted marker must be detectable
  by stock `cv2.aruco` with the matching predefined dictionary. Bit
  patterns therefore come from OpenCV's own tables, not from our own
  dictionary search.
- **4x4 only, all four sizes.** `DICT_4X4_50`, `DICT_4X4_100`,
  `DICT_4X4_250`, `DICT_4X4_1000`. No 5x5/6x6/7x7 families.
- **Angular (rotational) uniqueness is a hard requirement of any emitted
  set.** Two meanings, both enforced:
  - **No self-symmetry.** Every emitted marker must look different at
    0°, 90°, 180°, and 270° — a marker identical to its own rotation
    cannot have its orientation resolved from the image, so markers
    whose rotations coincide with themselves are never emitted
    (`--count` filters them out; `--all`/`--sheet` note them in output
    but still emit the full dict unchanged, since OpenCV compatibility
    governs dictionary emission).
  - **No cross-marker rotation collisions.** No rotation of one emitted
    marker may equal any rotation of another emitted marker, so the set
    is globally unique under any viewing orientation. Within a
    predefined OpenCV dict this holds automatically — the documented
    minimum Hamming distances are 4/3/3/2 for dicts
    50/100/250/1000, all ≥ 1; selection must preserve it, and the
    selection tests assert it as a property.
- **Lives in this repo** as workspace members `crates/aruco` (library)
  and `crates/aruco-cli` (binary).
- **Output surface for v1:** one SVG file per marker; `--all` emits the
  full dictionary as individual files; `--sheet` emits one contact-sheet
  SVG (grid with id captions). PNG/PDF deferred; the render path writes
  to `io::Write` so a raster backend can slot in later.
- **`--count N` selects from the OpenCV dict, not free generation.** The
  tool filters self-rotation-symmetric markers out of the dictionary and
  selects N of the remainder. Every selected marker is rotationally
  unambiguous and pairwise rotation-distinct, and stays OpenCV-detectable.
- **`--min-distance D` is optional** (robustness): with it, selection
  greedily maximizes the minimum pairwise Hamming distance (over all
  rotations of both markers) while treating D as a floor — if D cannot be
  met with N markers, it errors. The standard the flag anchors to (cpp
  `extendDictionary`, citing Garrido-Jurado et al. 2014): min inter-marker
  Hamming distance over all 4 rotations of both markers, plus
  self-distance to the marker's own rotations; the paper's target is
  `C = floor(markerSize²/4)`, `tau = 2·floor(4C/3)` → **C = 4, tau = 10**
  for 4x4. The predefined dicts fall short of tau (documented
  d = 4/3/3/2), so the meaningful D range is (dict's d … tau]; D at or
  below the dict's own minimum adds nothing. Without the flag, selection
  is by simple order or seed.
- **Selection mirrors the standard's self-distance check, stricter.**
  The dictionary filter drops self-symmetric markers entirely
  (self-distance 0); `MaxDistance` additionally prefers higher
  self-distance when scores tie, matching the standard's
  `min(self-distance, inter-marker)` acceptance order.
- **`--seed S` makes selection reproducible.** Deterministic PRNG
  (SplitMix64, hand-rolled — no `rand` dependency), seeded Fisher-Yates
  ordering with **debiased bounded sampling** (Lemire's method, no
  `%`-modulo on the raw draw) so the permutation — not just the PRNG —
  is identical on every platform; same seed ⇒ same tag set. Chosen ids
  are printed to stdout.
- **The transcribed OpenCV table carries Apache-2.0 attribution.**
  OpenCV's `LICENSE` is Apache License 2.0 (verified against the repo's
  `4.x/LICENSE`, 2026-10-12); §4 requires license copy + attribution
  when its source data is redistributed. The transcription is that
  redistribution: the `data` module's doc-comment names the source file
  and branch/commit it was transcribed from, and Bite A adds the
  corresponding entry to `THIRD_PARTY_NOTICES.md` (repo precedent: the
  OCCT license bullet in AGENTS.md's native-distribution requirements).

## Facts (read from OpenCV 4.x source, fetched 2026-10-12)

Sources, cited as `hpp:<line>` / `cpp:<function>`:
`modules/objdetect/src/aruco/predefined_dictionaries.hpp` (hpp),
`modules/objdetect/src/aruco/aruco_dictionary.cpp` (cpp), and
`modules/objdetect/include/opencv2/objdetect/aruco_dictionary.hpp`
(dict-hpp) — github.com/opencv/opencv, `4.x` branch. Byte-level
layout is additionally documented in OpenCV's official class docs
(<https://docs.opencv.org/4.13.0/d5/d0b/classcv_1_1aruco_1_1Dictionary.html>,
verified 2026-10-12); the MSB-first bit order appears only in the cpp.
Every claim below was checked in those files at the cited location;
anything not checkable there lives in *Open verification items*.

- **Marker geometry.** dict-hpp `generateImageMarker(int id, int
  sidePixels, OutputArray _img, int borderBits = 1)` — default
  `borderBits = 1`. cpp `generateImageMarker` builds a
  `(markerSize + 2*borderBits)`-cell grid (`tinyMarker`), fills the
  inner `borderBits … rows-borderBits` region with the marker bits.
  So a 4x4 marker renders as a **6x6 cell grid: a 1-cell solid black
  border ring surrounding the 4x4 data field**. 16 data bits per
  marker; 2^16 total patterns (arithmetic).
- **Predefined dictionaries are nested.** cpp
  `getPredefinedDictionary`: all four 4x4 dictionaries are constructed
  from the *same* pointer — `Mat(50 | 100 | 250 | 1000, (4*4+7)/8,
  CV_8UC4, (uchar*)DICT_4X4_1000_BYTES)` (rows 50/100/250/1000, cols
  2, 4 channels). The first 50 entries are `DICT_4X4_50`, the first
  100 are `DICT_4X4_100`, the first 250 are `DICT_4X4_250`.
- **Table shape.** hpp:4120 declares
  `static unsigned char DICT_4X4_1000_BYTES[][4][2]`; hpp:11–18
  documents the layout: `[marker][rotation][byte]` with
  `ceil(markerSize²/8) = 2` bytes per rotation — all **4 rotations of
  every marker are precomputed and stored**. Rotation j is "a 90 degree
  rotation of the marker in anticlockwise direction" (hpp:18). The bit
  order within a rotation is row-major, MSB-first per byte (cpp
  `getBitsFromByteList` walks bytes applying `mask = 1 << 7 … >>= 1`;
  cpp `getByteListFromBits` fills bit positions from
  `bits.at(row, col)` in row-major order; the first stored marker,
  hpp:4121, rot-0 bytes `{181, 50}` = rows `1011 / 0101 / 0011 /
  0010`, is the concrete instance of this rule).
- **Canonical-form mapping for the library.** `bits: u16` with bit
  index `b = r*4 + c` encoded at position `15 - b` reproduces rotation
  0 from `DICT_4X4_1000_BYTES[i][0]` (consequence of the row-major
  MSB-first rule above).
- **Minimum Hamming distances are documented per dictionary** (enum
  comments in dict-hpp, distance computed over all rotations):
  `DICT_4X4_50` = 4, `DICT_4X4_100` = 3, `DICT_4X4_250` = 3,
  `DICT_4X4_1000` = 2. Since every documented distance is ≥ 2, **no
  two markers in a predefined dict share any rotation** — cross-marker
  angular uniqueness holds automatically inside any dict subset.
  Measured distances are re-derived as test constants anyway (the
  tests are the check on this doc).
- **Correction bits.** cpp `getPredefinedDictionary` sets
  `maxCorrectionBits`: `DICT_4X4_50` → `(4-1)/2 = 1`,
  `DICT_4X4_100`/`DICT_4X4_250` → `(3-1)/2 = 1`,
  `DICT_4X4_1000` → `(2-1)/2 = 0`. cpp `Dictionary::identify` accepts
  the *first* entry whose minimum-over-rotations distance is ≤ the
  correction budget — the mechanism that makes self-symmetric markers
  and near-collisions hazardous.
- A marker can still be **self-rotation-symmetric** — equal to itself
  at any of 90°, 180°, or 270° (the inter-marker distance does not
  forbid it). **The filter is over all three**: a marker is dropped if
  *any* of its rotations equals *any other* of its rotations. The
  arithmetic bound on how many can exist uses 180° only because it is
  the safe superset: anything fixed by 90° is automatically fixed by
  180° (180° = 90° applied twice), so in the 2^16 space at most 256
  patterns are self-symmetric under any of the three angles (16 cells
  → 8 two-cycles under 180° rotation → 2^8 fixed points; the
  90°-symmetric subset is smaller). Whether any exist inside
  `DICT_4X4_1000` is a test-determined constant recorded in Bite A.
- Consequence for `--count N`: the candidate pool is ≤ 1000 markers;
  the exact pool size falls out of the self-symmetry filter test.
- `extendDictionary(nMarkers, markerSize, baseDictionary, randomSeed)`
  exists (the runtime-generation path, seeded `theRNG()`), but the
  predefined dictionaries are hardcoded tables — no runtime generation
  is involved, and we do not replicate `extendDictionary`.
- Pure-rust SVG output needs no OCCT, no Unity, no native code — all
  three bites validate entirely with `cargo test`/`cargo build` in the
  worktree loop.
- **Measured greedy behavior** (Rust implementation, SplitMix64
  Fisher-Yates, verified via the CLI 2026-10-12): on `DICT_4X4_1000`,
  12 picks at a 6-bit floor succeed for seed 6 and fail (reaching 5)
  for seeds 1–5, 7, 8; 24 picks reach 5. The floor is therefore
  **seed-dependent by design**: a refused floor names the achieved
  value in its error; retry other seeds or lower the floor. Tests pin
  both the reachable (seed 6) and refusing (seed 42) cases.
- **Dependency status, stated in full.** `crates/aruco` has **no
  OpenCV link-time or runtime dependency** — the committed table is
  self-contained data and the crate never touches OpenCV. The bite
  *does* carry a **generation-time dependency**: reproducing the table
  requires OpenCV's `predefined_dictionaries.hpp` at pinned commit
  `802cf95aa3d42816995695bf68a86b3532f12760` (network fetch at
  transcription time; the commit URL is the provenance anchor). If
  offline reproducibility of the provenance chain is required, the
  alternative is vendoring the pinned hpp into `third_party/opencv/`
  (~640 KB) — an open call, not silently decided.

## Open verification items

1. The 90°-anticlockwise rotation comment is validated *structurally* in
   Bite A: our rotation implementation must reproduce rotations 1–3 of
   all 1000 stored markers from their rotation-0 bytes — the stored
   rotations are OpenCV's own ground truth.
2. Known-id spot check: render ids 0–2 with our SVG writer and compare
   against reference `DICT_4X4_50` renderings (or run one through
   `cv2.aruco.detectMarkers`) to close the loop end to end.
3. Self-symmetric-marker count inside `DICT_4X4_1000` — test constant
   recorded in Bite A.

## Architecture

Repo-facing claims below were verified 2026-10-12 against: the workspace
`Cargo.toml` (`members`, `[workspace.dependencies]` with `clap` 4.6.7 and
`thiserror` 2.0.21, no `rand`; `[workspace.lints]` denying
`unwrap_used`/`expect_used`/`panic` at lines 32–35),
`crates/mesh/Cargo.toml` (member pattern), `crates/mesh/src/lib.rs`
(crate-root shape), `crates/mesh/src/error/` and
`crates/occt-sys/src/error/` (both `mod.rs` + `result.rs`, no
`error.rs`), `crates/occt-sys/src/step_doc.rs` (import pattern), and
AGENTS.md (style rules).

### `crates/aruco` (library)

Crate root mirrors `crates/mesh/src/lib.rs`: flat `mod` list, then
`#[cfg(test)] #[allow(clippy::unwrap_used, clippy::expect_used,
clippy::panic)] mod tests;`, then `pub use` re-exports.

- `src/marker.rs` — `Marker { id: u32, bits: u16 }` (ids are `u32`
  like every other index in this repo; 16-bit patterns are `u16`).
  One type per file, so `Marker` does not share `dictionary.rs`.
- `src/dictionary.rs` — `ArucoDictionary` (enum over the four sizes),
  id → marker lookup, iteration over all markers.
- `src/rotation.rs` — bit permutation for 90/180/270 rotation on the
  4x4 field; `rotations(bits) -> [u16; 4]`.
- `src/hamming.rs` — the standard error checks (cpp
  `extendDictionary` / dict-hpp enum comments): pairwise min Hamming
  distance over all 4 rotations of both markers
  (`min_rotation_hamming`), self-distance to a marker's own rotations
  (`self_distance`), popcount-based; the paper's target
  `tau = 2·floor(4C/3)` with `C = floor(markerSize²/4)` (= 10 for 4x4)
  as the named constant `TAU_TARGET`.
- `src/data/` — the table, transcribed verbatim as
  `[u8; 1000][4][2]`-shaped constants in `dict_4x4_1000.rs` (verified:
  OpenCV stores one table `[marker][rotation][byte]`, all four dicts
  are row-count-limited views of it); `dict_4x4_50/100/250.rs` expose
  the prefix slices — one physical table, four dictionary views. The
  module doc-comment carries the OpenCV Apache-2.0 attribution (source
  file, branch, commit hash recorded at transcription time).
- `src/select.rs` — selection engine (Bite C):
  `filter_self_symmetric(dict)` (drops markers where any two of their
  own rotations are equal), `select(dict, n, strategy)` with
  `Selection::{FirstN, Seeded{seed}, MaxDistance{seed, min_distance}}`;
  `prng.rs` — SplitMix64 (one type per file convention).
- `src/render.rs` — SVG writer (Bite B): pattern → SVG document,
  `MarkerSvg` builder with cell size, units (`px`/`mm`), quiet-zone
  margin in cells, optional id caption; `sheet.rs` — contact-sheet
  layout (Bite B).
- `src/error/` — module directory copied from the existing pattern
  (`crates/mesh/src/error/`, same as `crates/occt-sys/src/error/`):
  `mod.rs` holds the `Error` enum (thiserror, `#[error("{message}
  {location}")]`, variant carrying an `error_location::ErrorLocation`
  captured via `#[track_caller]` constructors) and `result.rs` holds
  the crate-local `pub type Result<T>` alias; re-exported from the
  crate root as `Error as ArucoError, result::Result as ArucoResult`.
  Tests may unwrap/panic; library code may not (workspace lints
  already deny `unwrap_used`/`expect_used`/`panic`).
- `src/tests/` — all unit tests, registered via `src/tests/mod.rs` and
  `#[cfg(test)] mod tests;` in the crate root.

### `crates/aruco-cli` (binary)

- `src/main.rs` + `src/cli.rs` — clap derive; thin: parse, call
  library, write files, print chosen ids.

### Workspace wiring

- Add both members to `Cargo.toml` `members`; add
  `aruco = { path = "./crates/aruco" }` under `[workspace.dependencies]`
  for the CLI crate to reference. `clap` and `thiserror` already exist
  in `[workspace.dependencies]`; no new external deps.
- Member `Cargo.toml`s follow the workspace pattern exactly
  (`crates/mesh/Cargo.toml` as the model): `version = { workspace =
  true }`, `edition = { workspace = true }`, every dependency as
  `name = { workspace = true }`, `[lints] workspace = true` — no
  versions or lint settings pinned in the member crates.

### Style rules the bites apply (from AGENTS.md)

- **One Rust type per file.** Grouped types only via a module
  directory with a `mod.rs` re-export (the `occt-sys/src/error/`
  pattern); the plan above assigns one file per type.
- **Explicit imports.** One `use` statement per source crate
  (`crate`, `std`, each external), items grouped in nested braces, no
  glob imports; order `crate::` → workspace crates → other crates
  (pattern: `crates/occt-sys/src/step_doc.rs`).
- **Numeric conversions.** `usize` lengths → `u32` via
  `u32::try_from(len)` with a real error (table length, selection
  count); mixed-domain arithmetic in `u64` via `u64::from(u32)`; no
  `as` casts. Ids stay `u32`; bit patterns stay `u16`; no widening
  beyond what `From` actually provides.
- **Tests out of line** in `src/tests/`, never inline in source
  files; tests may unwrap, library code may not.

## CLI spec (summary)

The user manual lives in `crates/aruco-cli/README.md` (the binary
crate's own README) — that is the usage reference: modes, `-o`
semantics per mode, flag rules, selection-floor behavior, output
naming, exit codes. `aruco --help` renders the same facts live.
What belongs in this plan doc is the shape only:

- Modes: `<id>` (one file), `all` (a directory of files or, with
  `--sheet`, one contact sheet), `--count N` (a selected set — same
  outputs as `all`).
- Flags: `--dict` (default `4x4_50`), `--size`/`--units`,
  `--margin` (default 0 — OpenCV `generateImageMarker` parity: the
  ring sits flush to the edge; quiet zone opt-in), `--label`,
  `--sheet`/`--cols`,
  `--seed`/`--min-distance` (selection only).
- Cross-flag rules clap enforces: positional xor `--count`;
  `--seed`/`--min-distance` require `--count`; `--cols` requires
  `--sheet`. One runtime rule: `--sheet` rejects a single id.

## SVG output model

- viewBox in cells: `(6 + 2*margin) × (6 + 2*margin)`.
- White background rect (quiet zone), 1-cell black border ring
  (verified: OpenCV renders `borderBits = 1` by default), data cells:
  one `<rect>` per black data cell over the white background.
- `--label` adds a caption row (one cell tall) below the marker with the
  id text; the viewBox extends accordingly.
- No styling beyond fill colors; consumers (print pipeline) control the
  rest.

## Verification strategy

1. **Table correctness (Bite A gate).** Transcribe
   `DICT_4X4_1000_BYTES` verbatim from
   `predefined_dictionaries.hpp` (the fetch is done — see *Facts*).
   Prove by invariants rather than trusting the transcription:
   - **Rotation reproduction**: our rotation code, applied to each
     marker's rotation-0 bytes, must reproduce the stored rotations
     1–3 for all 1000 markers — OpenCV's own precomputed rotations are
     the ground truth for the bit-order/rotation conventions.
   - Pairwise min rotation Hamming distance per dict size must equal
     the documented values: 4 (dict 50), 3 (dict 100), 3 (dict 250),
     2 (dict 1000) — this also proves no shared rotations.
   - Count of self-symmetric markers recorded as a test constant.
   - Spot-check: marker 0 rotation 0 = bytes `{181, 50}` (rows
     `1011 / 0101 / 0011 / 0010`), then 2–3 ids against reference
     renderings of `DICT_4X4_50`.
2. **Selection (Bite C gate).** Filtered pool never contains a
   self-symmetric marker; seeded selection is reproducible (same seed,
   same ids; different seeds, distribution sanity); MaxDistance result
   has min pairwise distance ≥ `--min-distance` or errors; uniqueness
   property test over all rotations of all selected markers.
3. **Rendering (Bite B gate).** SVG snapshot tests (stable serialization
   order), cell-count math (6 + 2·margin grid, label row), CLI smoke
   test over `--count`, `all`, `--sheet`.
4. **Human check.** Open 2–3 generated SVGs in a viewer and — the real
   end-to-end test — run one through `cv2.aruco.detectMarkers` with the
   matching dict.

## Bites

- **Bite A — dictionary tables + core model** — **implemented and
  validated** (worktree `../greyhound-snake-aruco-a`, doc
  `aruco-bite-a.md`): `crates/aruco` skeleton, table extraction
  script, Apache-2.0 notice, invariant suite (rotation reproduction
  of all 1000×3 stored rotations, distances 4/3/3/2, self-symmetry
  count 0, marker-0 spot check). Gate green: tests/clippy/fmt.
- **Bite B — SVG renderer + single/whole-dict CLI** — **implemented
  and validated** (worktree, doc `aruco-bite-bc.md`): `units.rs`,
  `render.rs` (cell-grid SVG, snapshot-pinned; caught and fixed a
  data-offset bug — data hugs the ring at `m+1`), `sheet.rs`
  (marker-set-driven: `SheetSvg::new(dict)`/`from_markers`),
  `crates/aruco-cli` (`<id|all>`, `--dict`, `--size`, `--units`,
  `--margin`, `--label`, `--sheet`, `--cols`), clap parse + run smoke
  tests. Deviations recorded in `aruco-bite-bc.md` (sheet `--size` =
  full width with aspect-derived height; cols clamp to marker count).
- **Bite C — selection mode** — **implemented and validated**
  (worktree, doc `aruco-bite-bc.md`): `prng.rs` (SplitMix64, Lemire
  debiased draws, golden-vector test), `select.rs` (`FirstN`/
  `Seeded`/`MaxDistance`, property tests for rotation uniqueness and
  floor refusals), CLI `--count`/`--seed`/`--min-distance` with the
  chosen ids printed to stdout. Gate green: 48 lib + 18 CLI tests,
  clippy/fmt clean, whole workspace green.
- **Open item carried by the user:** `cv2.aruco.detectMarkers` check
  on generated SVGs (`cv2` is not installed on this machine; open
  verification item 2). Sample outputs wait in
  the worktree's gitignored `tmp/aruco-samples/` for the eyeball +
  detection check.

Each bite's implementation doc is written mechanically from its
worktree diff per the AGENTS.md bite workflow; this doc holds the
feature definition and decisions, the bite docs hold the exact steps.

## Related docs

- `add-markers.md` — scene-side marker authoring and synthetic capture;
  the marker *manager* and capture loop do not depend on this
  generator, but the printed-marker texture assets may be produced
  from its SVGs.
- `potpori.md` — *Detector training (synthetic labeling loop)*: the
  ML-detection rationale that `add-markers.md` builds on.
