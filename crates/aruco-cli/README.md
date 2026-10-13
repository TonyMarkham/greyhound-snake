# aruco-cli

Generates **4x4 ArUco marker SVG files** — OpenCV-compatible (every
marker is detectable by stock `cv2.aruco` with the matching
predefined dictionary), rotationally unique, and reproducible.

Part of the `greyhound-snake` workspace. Plan and OpenCV
source-verified facts: `aruco-svg.md` (repo root). License: the
dictionary bit patterns are OpenCV source data, redistributed under
the Apache License, Version 2.0 — attribution in the repo's
`THIRD_PARTY_NOTICES.md` and `third_party/opencv/`. Everything else is
the repo's license.

## Prerequisites

- **Rust toolchain** — `cargo` builds the CLI
  ([rustup](https://rustup.rs) if missing).
- **An SVG rasterizer** — needed only to turn the generated SVGs into
  PNG textures; the crate emits SVG and ships no rasterizer. On
  Ubuntu:

  ```bash
  sudo apt install librsvg2-bin
  ```

  ImageMagick, Inkscape, or CairoSVG work the same.

## Build and run

```bash
cargo run -p aruco-cli -- <arguments>        # or: cargo build -p aruco-cli
```

## Generate N unique, robust markers as individual files

The primary use case — N standalone SVG files, one marker each, ready
for rasterizing into textures or printing:

```bash
aruco --count 4 --dict 4x4_1000 --seed 6 --min-distance 6 -o out/
```
```text
selected 4 markers: 376 596 920 937

out/
├── marker_0376.svg
├── marker_0596.svg
├── marker_0920.svg
└── marker_0937.svg
```

Each file is a standalone SVG document — the **6x6-cell grid itself**
(black border ring flush to the SVG edge, 4x4 data field inside,
OpenCV `generateImageMarker` geometry; 256 px default). No baked-in
white padding; add a quiet zone with `--margin` if a consumer wants
one. Rasterize one per texture (any SVG rasterizer — see
Prerequisites):

```bash
rsvg-convert -w 256 -h 256 out/marker_0376.svg -o marker_0376.png
# equivalents: magick -background none out/marker_0376.svg -resize 256x256 marker_0376.png
#              inkscape out/marker_0376.svg -w 256 -h 256 -o marker_0376.png
#              cairosvg out/marker_0376.svg -o marker_0376.png -W 256 -H 256
```

What the flags guarantee:

- **`--count 4`** — four markers, each readable differently at 0°,
  90°, 180°, and 270° (self-symmetric markers are filtered out
  before picking).
- **`--min-distance 6`** — robustness floor: any two of the picked
  markers differ in ≥ 6 of 16 cells at *every* orientation, so a
  detector cannot confuse one for another under print noise.
- **`--seed 6`** — reproducibility: same seed ⇒ the same marker set
  on any machine (SplitMix64 Fisher-Yates, golden vectors pinned by
  tests). Re-running the command reproduces the files exactly.

Notes on the floor:

- The dictionary's own documented minimum distances are 4/3/3/2
  (dicts `4x4_50/100/250/1000`); the selection paper's theoretical
  target is `tau = 10`.
- The floor is **seed-dependent by design**: the greedy selection
  maximizes the achieved minimum, and if that stays under D the tool
  errors naming the floor, the achieved value, and the paper's
  theoretical target — `cannot pick 12 markers with minimum distance
  6; the greedy pass reached 5 (paper target tau = 10)` — so retry
  other seeds or lower D. What floor a count can reach depends on
  dictionary size (measured with seed 6): `4x4_1000` picks 12 markers
  at distance 6 (floor 7 fails, reaching 6) but 50 markers top out at
  4; `4x4_50` picks 12 at distance 5 (floor 6 fails, reaching 5).
  Larger dictionaries have more headroom; more data bits (5x5/6x6)
  are not in v1.
- Every selected marker keeps its OpenCV id — detection with
  `cv2.aruco` reports that id.

## Other modes

```bash
# one marker by id (0-based; id 0 is valid) — ONE file at -o
aruco 7 --dict 4x4_50 --size 50 --units mm --label -o marker_7.svg

# every marker of a dictionary — one marker_{id:04}.svg per file in -o
aruco all --dict 4x4_50 -o dict50/

# opt-in: the whole dictionary (or a --count selection) as ONE
# contact-sheet SVG instead of individual files
aruco all --dict 4x4_50 --sheet --cols 10 --size 200 -o sheet_50.svg
```

## Flag reference

| Flag | Default | Meaning |
|---|---|---|
| `<id>\|all` / `--count N` | — | mode selector; mutually exclusive |
| `--dict` | `4x4_50` | predefined OpenCV dictionary (4x4 only: 50/100/250/1000) |
| `-o` | — | file path (single id, `--sheet`) or directory (batch/selection), created if missing |
| `--size` | 256 | rendered side length (per marker; sheet width) in `--units` |
| `--units` | `px` | `px` or `mm` — attributes only; geometry stays in cells |
| `--margin` | 0 | quiet-zone cells around the border (0 = OpenCV `generateImageMarker` parity: the black ring sits flush to the SVG edge) |
| `--label` | off | id caption row under each marker |
| `--sheet` | off | one contact-sheet document instead of individual files |
| `--cols` | 10 | sheet columns (clamped to the marker count) |
| `--seed` | — | selection seed; requires `--count` |
| `--min-distance` | — | selection floor; requires `--count` |

Cross-flag rules (clap-enforced): positional xor `--count`;
`--seed`/`--min-distance` require `--count`; `--cols` requires
`--sheet`; `--sheet` with a single id is rejected at runtime.

## Output rules

- Batch/selection naming: `marker_{id:04}.svg` — zero-padded so
  directory listings sort numerically.
- Existing files are overwritten silently — the print pipeline
  regenerates.
- Exit 0 on success; exit 1 with a located `error:` line for every
  rejected combination (including an unmet selection floor).
- `--help` renders the same facts live.
