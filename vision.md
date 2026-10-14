# Cart-pole vision integration — companion package + OpenCV bridge (facts and plan)

Migrate the cart-pole project's vision layer into a companion UPM package in
this repo, give it native OpenCV through the established shim/ABI patterns,
and retire the project's Python tooling. The motivating event: the project's
Python checker confirmed **0 of 80** markers on the 2026-10-14 capture run
and rewrote every label file to empty — the root-cause chain below resolves
that failure and names every fix that landed from it.

**Status: bite V1 landed (2026-10-14, branch `feature/unity`); V2+ are plan,
not landed state.** Landed: `crates/opencv-sys` (the ghcv detection shim),
`tools/ArucoIdentify` (the C# checker harness), `aruco-cli` PNG output +
quiet-zone default. Validated on the real corpus: 80/80 crops. Everything
under "Work breakdown" from V2 on is intent; verify against the repo before
citing it as current state.

Companion to `occt-to-unity.md` (import pipeline, gap table G1–G12),
`unity-mujoco.md` (joint authoring + MJCF export), and `occt-mesh.md`
(shim-facts discipline: OpenCV facts below were read from the fetched
headers and OpenCV sources the same way).

## Landed decisions

- **V-D1 — Companion package.** The marker/vision integration lands in a
  companion package, not in `com.greyhound.step`. Working id
  `com.greyhound.vision` (matches the `com.greyhound.*` scheme); the id is
  confirmed at V2 start when the package skeleton is authored.
- **V-D2 — OpenCV comes from `TonyMarkham/opencv-dev` releases only.**
  Pinned `v0.1.0`, dev archives for building, libs-only archives for the
  runtime payload, SHA-256 pins held in `extern-fetch` config. Same split
  and same same-tag invariant as `[occt]`.
- **V-D3 — Native access only through C ABI shims.** Layering mirrors OCCT:
  all OpenCV knowledge (cv::Mat, detection parameters, dictionary handling)
  lives in the C++ shim; no OpenCV type crosses an ABI. Rust consumers go
  through a `opencv-sys` crate; Unity goes through a flat, version-gated
  cdylib (`opencv-host`) P/Invoked by C# — the `importer-host` pattern.
- **V-D4 — Python retires.** `aruco_check.py` is replaced by the C# verify
  step running on the native detection library (per V-D7) — not by a Rust
  CLI. `gym_state_sender.py` becomes a Rust CLI. Exception:
  `freecad/tools/step-mass.py` **stays Python** — FreeCAD's scripting API
  is Python-only; there is no Rust path into FreeCAD.
- **V-D5 — The dataset layer stays project-side in cart-pole.** Desks,
  forced situations, `RigStateRandomizer`, `RigStateCapture`, the dataset
  editor windows, `StateServer`, `SnapshotCapture`, `RobotBounds`. The
  package generalizes mechanisms; the project owns the dataset semantics
  built on them.
- **V-D6 — The migratable core is the marker family + `JointDriver`.**
  `MarkerSO`, `RigSO`, `MarkerManager` (+ `MarkerMenu`, `MarkerSOEditor`,
  `RigSOEditor`, the `UI/` UXML/USS), and `JointDriver`. `MarkerMaterialSet`
  moves into the package's `Editor/` — fixing its current unguarded
  `using UnityEditor` (compiles only because Assembly-CSharp never targets
  a player).
- **V-D7 — C# orchestrates; the native side is a stateless detection
  library.** All workflow lives C#-side — which images exist, in what
  order they are checked, filename ground truth, comparison, reporting,
  label accounting. The native ABI is a pure function: encoded image bytes
  in → detected tag ids out. No file I/O, no label rewriting, no output
  artifacts, no dataset knowledge on the native side. The Python checker's
  file-based contracts (stdout lines, `verification_stats.json`,
  `markers.json`/`classes.txt` as an inter-lingua) exist only because the
  checker was an out-of-process workaround for missing native detection;
  they die with it. The `StateServer` wire protocol and the YOLO label
  file format are real contracts and survive (see Interfaces).
- **V-D8 — Dictionary source settled.** Generation: `crates/aruco`'s
  transcription of OpenCV's table (byte-verified against the pinned
  upstream commit — see Facts). Detection: OpenCV's own predefined
  `DICT_4X4_1000` (API confirmed from the fetched 5.0 headers). The two
  are one shared table in two implementations, cross-validated by the
  corpus: generated-by-one, detected-by-the-other, 80/80.

Existing repo decisions apply unchanged and are not restated here: the
dist-first loop for Unity package work, Unity-minted `.meta` files, the
workspace Cargo.toml pattern, one type per file, tests under `src/tests/`,
the error-location pattern, license/notices rules for redistributed native
binaries.

## Why the checker read 0/80 — the root-cause chain

Resolved 2026-10-14 by experiment through the native detector (not by
reading code alone). Three independent defects, in the order they were
found:

1. **The crops were vertically mirrored — the primary defect.**
   `RigStateCapture.CaptureMarker8K` copied `GetPixels` output (already
   bottom-up, the framebuffer's convention) through a reversal loop into
   an "upright" array — un-flipping something that was never flipped — so
   every crop ever written was vertically flipped relative to its frame.
   A mirrored field is not a codeword (dictionaries store only the four
   rotations): mirror(596) coincides with **662's rotation-3 codeword**
   (verified against the real table bytes), the other three mirrors are
   nothing — exactly the observed 0/80-with-one-662 signature. The Python
   checker was **correct every time**; its crime was wiping labels on
   `--write`. Fixed by deleting the reversal (crop = the block as
   `GetPixels` returned it).
2. **Edge-to-edge markers never detect.** The canonical generator image
   for a 4×4 marker is 6×6 (1-cell black ring + 4×4 data) — OpenCV's
   `generateImageMarker` emits **no white margin**; white surroundings are
   the presenter's job. The repo's textures rasterized the 6×6 flush to
   the image edge; the detector's candidate filter
   (`minDistanceToBorder`, default 3 px) rejects corner-at-border
   candidates, and setting it to 0 (the assert allows ≥ 0) was measured —
   **still no candidates**: the failure sits in candidate formation, not
   the border filter alone. Fix: bake the quiet zone into the texture —
   `aruco-cli --margin` default changed 0 → 1 (8×8 = ring + data + 1-cell
   white ring).
3. **Hairline seams in non-integer-pixel-per-module textures.** The
   installed 1024px 6×6 texture put 170.67 px per module; a measured
   1-px ~56/255 gray line ran through the black at every module boundary
   (both neighbors black) — fatal for thresholding, and provably not from
   the current seam-free rasterizers. Durable guard: `aruco-cli` PNG mode
   validates `--size` as a multiple of the module grid, making fractional
   modules structurally impossible; `MarkerMaterialSet`'s center-sampling
   loop is seam-free at any size by construction.

Post-fix validation: 80/80 crops through the full chain
(C# → P/Invoke → ghcv.dll → OpenCV → ids). The single later crop miss
(frame_000005, Left/0376) was a genuinely obscured marker — a correct
no-detection (see the finder's role below).

## Marker presentation rules (measured invariants)

The rules any image of these markers must satisfy for OpenCV
DICT_4X4_1000 detection; each carries its evidence:

- **Quiet zone:** white ring around the black ring in any presented
  image; edge-to-edge never detects and `minDistanceToBorder = 0` does
  not rescue it (measured). `aruco-cli` bakes margin 1 (8×8 layout).
- **Polarity:** bit 1 renders white, bit 0 black, border black —
  source-verified against `generateImageMarker` (all-black canvas, bits ×
  255). The opposite polarity emits every marker's bit-inverse, which
  decodes as a different marker or nothing.
- **Mirror ≠ rotation:** a vertical flip decodes as a different id or
  nothing; mirror(596) = 662's rot-3 codeword (the lone-wrong-id
  signature). Detection cannot be fooled into the right id by a mirrored
  marker — it fails, which is the desired behavior.
- **Integer pixels per module** (or pixel-exact center sampling):
  fractional module sizes risk boundary hairlines; measured ~56/255 at
  170.67 px/module. `aruco-cli` enforces the multiple-of-grid rule.
- **Orientation is authored per marker:** `MarkerSO.flipVertical` /
  `flipHorizontal` (landed in cart-pole with the `MarkerManager`
  consumer: negative texture scale + offset on the quad's per-instance
  material) correct the rendered orientation against the physical
  print's mounting.

The finder's role (the design's purpose): detection absence on a crop is
a **high-confidence visibility signal** — the labeller suppresses labels
for markers the native detector cannot read, keeping bad data out of
training. Occlusion geometry: only the rail-mounted Left/Right markers
(376/596) can be obscured (pole sweep); Pivot/End ride the pole and fail
only by obliquity. The 1440×1080 main frames are the simulated real
camera (50 mm markers → low pixel density); the 4× (8K) path exists to
detect markers better. Real-density margins and occlusion are the
operating envelope, not defects.

## Facts checked 2026-10-14

Sources read: cart-pole sources at
`D:\git\cart-pole\unity\projects\cart-pole\Assets\Cart-Pole\Vision\`
(+ `Editor\`, + `Editor\UI\`; namespace `CartPole.Vision`, no asmdef —
compiles as Assembly-CSharp / -Editor; 12 runtime scripts, 11 editor
scripts, 4 UXML/USS files); the `TonyMarkham/opencv-dev` release page;
OpenCV 5.0.0 fetched headers; OpenCV upstream sources at the pinned
commit.

- cart-pole consumes this repo as: git submodule
  (`submodules/greyhound-snake`) + Unity `file:` manifest entry to
  `dist/package/com.greyhound.step`.
- Capture corpus at
  `D:\git\cart-pole\unity\projects\cart-pole\Assets\Captures\cart-pole\`:
  20 frames (1440×1080) + 20 8K renders + 80 marker crops
  (tags 0376/0596/0920/0937 × 20, the tag encoded in each crop filename),
  `markers.json` (class→tag: 0→376 Left, 1→596 Right, 2→920 Pivot,
  3→937 End), `classes.txt`, per-frame manifests. The motivating run's
  `verification_stats.json` recorded 0/80 confirmed; its `--write` pass
  zeroed all 20 label files (historical fact — that behavior is not
  preserved).
- OpenCV release `v0.1.0` (2026-10-14): OpenCV 5.0.0, upstream tag commit
  `40738fb`, full 16-module set, six archives (dev + libs-only per
  platform: x86_64-linux, x86_64-windows, arm64-macos). Dev = full
  install tree; libs-only = runtime libraries + 3 legal texts. SHA-256
  checksums published on the release page; they get pinned into
  `crates/extern-fetch/config.toml` at V2 — not duplicated here.
- **Actual header layout is `include/opencv2/`** (the release page's
  `include/opencv5/opencv2` text does not match the archive — the
  extracted tree wins, and `opencv-sys/config.toml` carries `include`).
- Windows x64 layout: stock SDK — `x64/vc17/bin`
  (versioned `opencv_*500.dll`), `x64/vc17/lib` (import libs). Linux
  x86_64: unversioned `libopencv_*.so` (local build patch, release's
  `opencv-linux.md` §2.1). macOS arm64: unversioned `libopencv_*.dylib`,
  `@rpath` install names. License Apache-2.0; `LICENSE`, `COPYRIGHT`,
  `THIRD_PARTY_NOTICES.md` in every archive; per-component license texts
  in dev archives. Intel IPP and the GPL Unicode font are not built.
- OpenCV 5 API facts (fetched headers): `cv::aruco` lives in `objdetect`;
  `PredefinedDictionaryType::DICT_4X4_1000` (4×4 bits, min Hamming
  distance 2, 1000 codes; `maxCorrectionBits = (2-1)/2 = 0` — no error
  correction); `getPredefinedDictionary()`;
  `ArucoDetector(dictionary, parameters)` with `detectMarkers(image,
  corners, ids, ...)` and a 5.0 addition `detectMarkersWithConfidence`.
  `DetectorParameters`: `minDistanceToBorder = 3` ("minimum distance of
  any corner to the image border for detected markers"),
  `markerBorderBits = 1`, `perspectiveRemoveIgnoredMarginPerCell = 0.13`,
  `maxErroneousBitsInBorderRate = 0.35`.
- `generateImageMarker` source (upstream `aruco_dictionary.cpp` at
  `802cf95aa3d42816995695bf68a86b3532f12760`, the commit
  `crates/aruco`'s table cites): canonical image = all-black canvas,
  4×4 bits scaled by 255 into the inner region — 6×6 total for 4×4
  markers, **no white margin emitted**. The repo's table transcription
  is byte-verified against this commit (marker 0 `[181, 50]`, marker 376
  `[230, 15]` spot-checked in the pinned hpp).

## Cart-pole source inventory and disposition

Verified by reading every file. Disposition: **P** = migrates into the
companion package, **R** = re-homed as a Rust CLI (Python retired),
**Q** = stays project-side in cart-pole.

| File | Role | Disposition |
| --- | --- | --- |
| `JointDriver.cs` | Applies gym qpos to the imported assembly through `StepJoint` assets, MJCF slide/hinge semantics, base poses captured once, world-space math | P |
| `MarkerSO.cs` | One ArUco marker: label, `arUcoId` (DICT_4X4_1000, zero-padded string), anchor = `StepSiblingPath`, prefab, size, offsets, flip toggles | P |
| `RigSO.cs` | Rig binding: marker list + cart/pole `StepSiblingPath`s + step asset ref/path fallback | P |
| `MarkerManager.cs` | Resolves anchors via sibling paths, disables anchor renderer/colliders, instantiates marker quads (incl. the flip consumer), applies rig-state payloads | P |
| `MarkerMaterialSet.cs` | Rasterizes ArUco SVGs to seam-free PNGs + URP materials (center-sampling: seam-free at any size) | P (into `Editor/`) |
| `MarkerMenu.cs`, `MarkerSOEditor.cs`, `RigSOEditor.cs`, `UI/Marker.uxml`, `UI/Marker.uss`, `UI/Rig.uxml`, `UI/Rig.uss` | Marker/rig authoring tooling | P (menus re-homed off `Cart-Pole/`) |
| `RigStateCapture.cs` | Dataset capture: JPEG + YOLO labels + 8K marker close-ups + per-frame manifest; on run completion shells out to `Tools/aruco_check.py` via `uv` | Q; its Python hook retires at V4. The crop-write reversal bug is fixed (2026-10-14) |
| `DeskSO.cs`, `ForcedSituationSO.cs`, `RigStateRandomizer.cs` | Desk props, forced situations (cart/pole semantics), state randomization incl. skybox/light | Q |
| `DeskMenu.cs`, `DeskSOEditor.cs`, `ForcedSituationEditor.cs`, `DatasetGeneratorWindow.cs`, `CaptureViewer.cs`, `CaptureReportWindow.cs`, `HdriSetup.cs`, `StaleSelectionCleaner.cs` | Dataset/situation tooling, `Cart-Pole/` menus | Q |
| `StateServer.cs` | TCP bridge: JSON joint-state lines in → render → 4-byte LE length header + JPEG out (localhost) | Q |
| `SnapshotCapture.cs` | URP-aware supersampled PNG capture | Q |
| `RobotBounds.cs` | Workspace envelope cube driving the camera-jitter frustum contract | Q |
| `Tools/aruco_check.py` (cart-pole repo) | OpenCV re-verification of label crops; rewrites YOLO labels; emits `verification_stats.json` | Retires at V4 — replaced by the C# verify step on the native library (V-D4, V-D7); its file contracts are not preserved |
| `vision/gym_state_sender.py` (cart-pole repo) | Gym-side client of the `StateServer` bridge | R — Rust CLI (V4) |
| `freecad/tools/step-mass.py` (cart-pole repo) | FreeCAD scripting for STEP mass checks | stays Python (V-D4) |

## The companion package

- Tracked tree: `package/com.greyhound.vision/`; staged by
  `assemble-package` to `dist/package/com.greyhound.vision/`; consumed by
  cart-pole's `manifest.json` as a second `file:` entry beside
  `com.greyhound.step`.
- Declares a dependency on `com.greyhound.step` — marker anchors are
  `StepSiblingPath`s, `JointDriver` drives `StepJoint`/`MjJoint` payloads.
  Dependency direction already proven by the cart-pole code.
- `Runtime/`: marker family + `JointDriver`, the OpenCV P/Invoke layer
  (struct mirrors, version gate), `Plugins/opencv/<arch>/lib` payload
  (real files, license + notices included).
- `Editor/`: marker/rig menus + editors + `UI/`, `MarkerMaterialSet`, the
  verify step calling the native bridge (C# owns the accounting per
  V-D7; never wipes labels — writes only confirmed lines and refuses the
  write when confirmations are zero), and the payload provisioner (clone
  of `StepPayloadProvisioner`) reading a staged `Editor/opencv-fetch.toml`.
- asmdefs `com.greyhound.vision.runtime` / `.editor`, mirroring the step
  package's split and `AssemblyInfo.cs` convention.
- Tooling deltas: `extern-fetch` gains the `[opencv]` group (payload
  `dest`s point at the vision package); `just assemble-package` and
  `tools/verify-package.py` learn a second package.

## Bridges and CLIs

- **`crates/opencv-sys` — landed.** The ghcv shim: C++ (`cpp/detect.cpp`)
  compiled by `build.rs` from the crate-local `config.toml` (windows
  section only in V1 — a missing OS section is the platform gate; the
  other platforms' sections land with their dev entries). ABI:
  `ghcv_abi_version()` → 1, and `ghcv_aruco_detect(data, len, ids_out,
  max_ids)` → detection count (negative = decode/internal error);
  encoded image bytes in, ids out, no OpenCV type crosses, stateless.
  Corners are not exposed yet — deferred until the package's verify step
  needs them. Shim artifact: `dist/opencv/ghcv.dll` (built artifact, not
  tracked).
- **`tools/ArucoIdentify` — landed.** The C# checker harness (net10.0,
  plain console — not Unity code): enumerates image files, ground truth
  from the filename (`marker_NNNN.(png|jpg)`), compares against the
  shim's detections, prints a per-file report + tally; also lists the
  1440×1080 frames' detected tags against `markers.json` (visibility
  listing). Loads ghcv.dll + the OpenCV DLLs via
  `SetDefaultDllDirectories`/`AddDllDirectory` — no PATH mutation.
  Invocation: `dotnet run --project tools/ArucoIdentify -- <images...>
  <ghcvDir> <opencvBinDir>`. Open item: reword the frames output from
  MISS/failure framing to the visibility-oracle framing.
- **`crates/aruco-cli` — SVG + PNG, landed.** Output format follows the
  `--output` extension (`.png` rasterizes via `MarkerRaster`; anything
  else writes SVG). `--margin` default 1 (quiet zone baked in; the doc
  comment records the measured invariant). PNG guards: `--size` must be
  a multiple of the module grid (6 + 2·margin cells per side),
  `--label`/`--units mm`/`--sheet` are SVG-only. Single geometry source
  (`render::black_rects`) shared by the SVG writer and the raster,
  parity-tested. CLI crate's `README.md` is its user manual.
- `crates/opencv-host` — planned (V2): cdylib for Unity, flat
  version-gated C ABI over the same shim family; C# `NativeMethods`
  pattern in the package.
- Vision-sender CLI — planned (V4): Rust client for the `StateServer`
  protocol. Crate shape (standalone vs subcommand) settled at V4.

## Interfaces

Preserved:

- **`StateServer` wire protocol** — JSON line in (joints + optional rig),
  4-byte LE length header + JPEG out. The Rust sender (V4) must speak it;
  the Unity server side is unchanged.
- **YOLO label files** — `class x_center y_center w h`, normalized,
  top-left origin (y flipped from Unity's viewport). The C# verify step
  reads/writes them in-process.
- **The never-wipe rule** (from the motivating failure): the verify step
  writes only confirmed lines and refuses the write when confirmations
  are zero; the original is kept on a refused write.

Retired with the Python checker (V-D7): the `aruco_check.py` stdout
lines, `verification_stats.json`, and `markers.json`/`classes.txt` as
the checker's inter-lingua. The C# verify step computes what it needs
in-process from the rig's Marker SOs; if any other consumer of those
files turns up, it is found at V4, not assumed here.

## Work breakdown

Bites in dependency order. V2–V3 are Unity package work → dist-first
loop; V4 spans both repos. Bite numbering settled 2026-10-14 (re-cut
twice during planning; nothing under old numbers ever landed).

- **Bite V1 — ArUco target identifier — LANDED 2026-10-14.**
  `crates/opencv-sys` (ghcv shim, DICT_4X4_1000, bytes→ids),
  `tools/ArucoIdentify` (filename ground truth → per-file report),
  `aruco-cli` PNG mode + `--margin` default 1, `MarkerRaster` +
  `black_rects` single-source geometry. Validation: 4/4 on ring-ed
  reference rasterizations; 80/80 on the real corpus crops; 54 lib + 20
  CLI tests green; first Windows exercise of the shim chain. Landed with
  the session's cart-pole-side fixes (crop-write reversal deleted;
  `MarkerSO` flip toggles + consumer). Out of V1 (deferred, not
  forgotten): corners over the ABI, the other platforms' entries, the
  package.
- **Bite V2 — companion package skeleton + `opencv-host`.** Package
  skeleton authored in dist (Unity mints `.meta`s), manifest entries
  added in cart-pole, `opencv-host` cdylib + payload provisioner; the
  `[opencv]` group in `extern-fetch` config: the payload entries (all
  three platforms, pinned) and the remaining dev entries (linux/macos,
  pinned when a build machine verifies them). **First Windows exercise
  of the payload path** — the OCCT integration is validated on Linux
  x86_64 only, so expect firsts here (Plugins loading, DLL
  adjacency rules). Validation: Unity compiles the package; a minimal
  P/Invoke round-trip works from the editor.
- **Bite V3 — marker family migration.** `MarkerSO`/`RigSO`/
  `MarkerManager`/`JointDriver` + editor tooling + `MarkerMaterialSet`
  into the package (dist-first loop), cart-pole scene/asset bindings
  switched over, package-side sources pruned from cart-pole.
  Validation: cart-pole applies markers and drives joints exactly as
  before.
- **Bite V4 — retire the Python.** `RigStateCapture`'s `RunCompleted`
  hook swaps uv+python → the C# verify step on the native library
  (never wipes labels); `gym_state_sender.py` → Rust CLI speaking the
  unchanged `StateServer` protocol. Validation: a full capture run
  produces labels and the verification accounting without Python on
  PATH.

## Open items

- Package id: `com.greyhound.vision` is the working name — confirmed or
  renamed at V2 start.
- Harness frames output: reword from MISS/failure framing to the
  visibility-oracle framing (cosmetic; before it becomes the package's
  verify UX).
- AGENTS.md repository-map entries for `vision.md`, the new package
  tree (when it exists), and the new crates — owed now that V1 landed;
  a doc edit, not part of any bite's scope until done on request.
- Vision-sender crate shape — settled at V4.
- Windows/macOS Unity integration for the OCCT side remains unexercised
  and is not in scope here; the vision package's Windows payload path
  (V2) is the first native-payload work on this platform, and its
  findings belong in the occt gap table if they generalize.
