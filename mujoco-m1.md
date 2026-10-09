# MuJoCo M1 implementation reference (naive rigid-tree MJCF exporter)

Implementation homework for bite M1 of `unity-mujoco.md`: everything needed to
build the `greyhound-export-mjc` binary — the emitted MJCF schema, binary STL
layout, projection/units math, naming, CLI, crate layout, and the validation
numbers — checked against primary sources so the implementation worktree
starts without re-deriving any of it.

Facts checked 2026-10-09 against the MuJoCo **stable** docs (XML reference,
modeling guide, Python bindings) and, where compile-time behavior needed
confirmation, the `main`-branch compiler sources (`user_model.cc`,
`user_mesh.cc`, `user_objects.cc` — dev tree, flagged where used). Local
`python3 -c "import mujoco"` fails today (not installed); MuJoCo validation is
therefore a user-run step (`pip install mujoco`). Repo facts come from source
inspection only — no builds or tests were run for this document. This is a
facts/plan doc like `occt-mesh.md`, **not** the guided-implement bite doc
(that gets written from the worktree diff, per the bite workflow).

## Scope recap (M1 only)

`greyhound-export-mjc [--config config.toml] <file.step> --out <dir>
[--density 1.0] [--free-root] [--scale 0.001]` writes `<dir>/<stem>.xml` plus
one binary STL per unique mesh under `<dir>/meshes/`. Naive rigid tree: one
`<body>` per scene node, no joints (M2/M3), no colors (G13 deferred), no
actuation/keyframes/sensors. Root gets `<freejoint/>` iff `--free-root`,
else the assembly is static.

## Crate decision (closes the open M1-start decision)

New binary crate **`crates/greyhound-export-mjc`**, not a `step-stats` flag:

- `step-stats` is read-only reporting; the exporter writes artifacts and has
  its own option surface. A flag would entangle the two CLIs and their tests.
- The exporter consumes `occt-sys` + `mesh` types only — no dependency on
  `unity-projection` or `importer-host` (identity map; see Projection math).
- Workspace additions: one member in `Cargo.toml`; deps from
  `[workspace.dependencies]` only (`clap`, `occt-sys`, `mesh`, `thiserror`,
  `error-location`, `serde`, `toml`; dev `tempfile`). Never pin versions in
  the member crate.

Build prerequisite: OCCT must be installed per `occt-linux.md` — every cargo
build in this workspace requires `OCCT_PREFIX`/`OCCT_SHIM_DIR`
(`.cargo/config.toml`; `crates/occt-sys/build.rs:57-67`).

## The exported MJCF (emitted schema)

Exact shape for the synthetic fixture box (one meshed root, identity
transform, box spanning `[0,2]×[0,1]×[0,1]` mm, density 1.0 g/cm³,
scale 0.001):

```xml
<?xml version="1.0" encoding="utf-8"?>
<!-- exported by greyhound-export-mjc from box-2x1x1.step;
     density 1.0 g/cm3, scale 0.001, free-root off, deflection 0.01 -->
<mujoco model="box-2x1x1">
  <compiler angle="radian" meshdir="meshes" autolimits="true"
            inertiafromgeom="auto" balanceinertia="false"/>
  <asset>
    <!-- first used by: box -->
    <mesh name="mesh_000" file="mesh_000.stl"/>
  </asset>
  <worldbody>
    <!-- part: box -->
    <body name="box" pos="0 0 0" quat="1 0 0 0">
      <geom type="mesh" mesh="mesh_000"/>
      <inertial pos="0.001 0.0005 0.0005" quat="1 0 0 0" mass="2e-6"
                diaginertia="3.333333333333333e-13 8.333333333333334e-13
                             8.333333333333334e-13"/>
    </body>
  </worldbody>
</mujoco>
```

Element-by-element, with the checked facts:

| Element | Emitted | Checked facts |
|---|---|---|
| `<mujoco>` | `model="<step file stem>"` | `model` optional, default "MuJoCo Model", shown in simulate's title bar (`xmlref #mujoco-model`) |
| `<compiler>` | `angle="radian" meshdir="meshes" autolimits="true" inertiafromgeom="auto" balanceinertia="false"` | MJCF default for `angle` is **degree** — emitting `radian` is mandatory before M2 joint ranges. `autolimits` default true (emit anyway). `balanceinertia` default false; exact-BRep inertias satisfy A+B≥C physically, leave off. `inertiafromgeom` [false,true,auto] default **auto**: explicit `<inertial>` wins; its presence alone disables inference (`xmlref #compiler-inertiafromgeom`, `#body-inertial`). `strippath` default false — our `file` values are bare filenames, so it never matters. `meshdir` resolution: relative → *directory of the model file* + meshdir + filename (`xmlref #compiler-meshdir`), so `meshdir="meshes"` works wherever the package lands |
| `<asset><mesh>` | one per unique mesh, index order: `name="mesh_NNN" file="mesh_NNN.stl"` | `file` extension must be stl/msh/obj (case-insensitive). `name` optional (defaults to file stem) but we emit it — assets exist to be referenced. `scale` attribute exists ("1 1 1") but we bake units into the file instead (decided). `inertia` attr [convex,exact,legacy,shell] default legacy only affects *inferred* inertia — never fires (explicit inertials). `maxhullvert` default −1 (unlimited). `refpos`/`refquat` unused (`xmlref #asset-mesh`) |
| `<geom type="mesh" mesh="..."/>` | one per meshed body, no `pos`/`quat` (defaults), unnamed | Collision uses the convex hull; rendering uses the original triangles. With default pose the geom renders the mesh at its **authored file coordinates** relative to the body frame — see "Mesh recentering" below (`xmlref #asset-mesh`, modeling guide *Collision detection*) |
| `<inertial>` | `pos quat mass diaginertia`, all four, on every meshed body | `pos` and `mass` are **required** when the element is present; one of `diaginertia`/`fullinertia` required (we always emit `diaginertia`). The inertial frame's center is the COM and its axes the principal axes, so the inertia matrix is diagonal in it (`xmlref #body-inertial`) |
| `<body>` | `name pos quat` always emitted, nested per forest | `pos` is relative to the parent, default (0,0,0); orientation via `quat` (Frame orientations). A body without a joint is welded to its parent; welded-to-world bodies are static (modeling guide *Kinematic tree*) |
| `<freejoint/>` | root body only, iff `--free-root` | XML shortcut for `<joint type="free" stiffness="0" .../>` immune to joint defaults; free joints are only allowed in bodies that are children of the world body (`xmlref #body-freejoint`, `#body-joint`) |
| `<worldbody>` | top-level grouping; the scene root body hangs off it | The world body cannot have `inertial`/`joint` or attributes; its name is fixed "world" (`xmlref #worldbody`) |
| comments | header line + `<!-- part: ... -->` per body/mesh with raw names | XML comments; `--` is illegal inside a comment, so comment text is sanitized (see Naming) |

Not emitted: `<option>` (gravity default `(0, 0, −9.81)` — Z-up like OCCT,
`xmlref #option-gravity`), `<contact>`, `<equality>`, `<actuator>`,
`<keyframe>`, sensors, materials/`rgba` (G13 deferred), `<site>`, lights,
cameras, joints of any kind (M2/M3).

Number formatting: a small helper prints f64 via Rust `Display` when
`v == 0.0` or `1e-4 ≤ |v| < 1e6`, else `{:e}` scientific (Rust shortest
round-trip in both modes). MuJoCo parses attributes with strtod — its own
examples carry `1.66667e-05`.

## Compile-time behavior we rely on (and traps)

- **Names** must be unique among elements of the same type and are
  case-sensitive; no character restrictions are documented (modeling guide
  *Naming elements*; dedupe check `user_model.cc:5446`, dev tree). Our
  sanitizer is therefore policy, not a MuJoCo requirement.
- **Massless bodies**: a body with no `<inertial>` and no geoms gets zero
  mass; that is fine while welded. A *moving* (jointed) body with mass or
  inertia below `mjMINVAL` (1e-14) is a **compile error**
  (`user_model.cc:6075`, "mass and inertia of moving bodies must be larger
  than mjMINVAL"; `boundmass`/`boundinertia` compiler attrs exist as band-aids,
  default 0 — do not use).
  → `--free-root` caveat: the root body carries mass only if it is meshed.
  cart-asy's root is a grouping node, so `--free-root` there cannot compile.
  M1 behavior: the exporter pre-errors on `--free-root` with a massless root
  (clear message, no XML written); validation of `--free-root` uses
  `assets/rod-clamp-16mm.stp` (meshed root).
- **Degenerate triangles** with area below mjMINVAL are a compile error
  (`xmlref #asset-mesh` processing list). OCCT's `MinSize` mesher parameter
  (`occt-mesh.md`, `IMeshTools_Parameters`) guards this.
- **Convex hull** construction fails on colocated/collinear/coplanar vertex
  sets (`user_mesh.cc:1683-1733`, dev tree) — CAD solids are fine; a
  degenerate "solid" would fail loudly.
- **Mesh recentering**: MuJoCo pre-processes every mesh — translated to COM
  and rotated to principal axes — saving the offsets in `mjModel.mesh_pos` /
  `mesh_quat`, and composes them into the referencing geom's pose
  (`xmlref #asset-mesh`). Net effect: **with a default geom pose the mesh
  renders exactly at the STL's authored coordinates in the body frame** —
  our body-local OCCT-scaled coordinates. This is why no geom `pos`/`quat`
  is needed. Validation trap: `mjModel.mesh_vert` holds the *centered*
  vertices, not the file's coordinates.
- **STL specifics**: binary STL only. Facet normals are **not read** as
  vertex normals (MuJoCo generates its own; "normals cannot be provided with
  STL meshes"). Repeated vertices are removed and faces re-indexed. Texture
  coordinates are impossible with STL (irrelevant — G2 omits UVs).
- **fusestatic** default false for MJCF: static subtrees stay separate
  bodies — our grouping nodes survive as bodies (`xmlref #compiler-fusestatic`).
- World body is id 0 in `mjModel`; `nbody` counts it. Explicit
  `<inertial>` (`pos`, `mass`, `diaginertia`) flows into
  `mjModel.body_ipos/body_iquat/body_mass/body_inertia` verbatim
  (`user_model.cc:2725-2728`, dev tree).

## Binary STL output

Per unique mesh, written from the core model (`mesh::Mesh`: flat
`[f32; 3]` vertices + `[[u32; 3]]` triangles, OCCT mm, outward-CCW).

| Field | Bytes | Content |
|---|---|---|
| header | 80 | zero bytes — deliberately **not** ASCII, and must not begin with `solid` (the heuristic other tools use to sniff ASCII) |
| triangle count | 4 | `u32` little-endian |
| per triangle | 50 | `f32` LE ×12: normal (3), v1 (3), v2 (3), v3 (3); then `u16` attribute-byte-count = 0 |

- Normal derived from winding: `normalize((v1−v0)×(v2−v0))` — outward-CCW
  core model ⇒ outward normal in RH algebra (matches STL's CCW right-hand
  rule; the identity map needs no flip, unlike Unity's projection). Zero-area
  triangles write a zero normal rather than NaN.
- Scale baked into vertices: `v × s` (mm → m at the default 0.001) before the
  f32 store. No normals/UVs beyond the STL facet-normal slot.
- Size check: `84 + 50 × ntri` bytes — 684 for the 12-triangle fixture box.
- MuJoCo ignores the facet normal and rebuilds normals; the field is still
  written correctly for other RH consumers (Blender, CAD viewers — the
  "STL must look identical to the STEP" debugging rule of
  `occt-to-unity.md`).

## Projection math (identity axis map + scale)

Second customer of the architectural claim from `unity-mujoco.md`: every
consumer gets its own projection; MuJoCo's is the **identity map + mm→m
scale** — no conjugation, no winding flip (`occt-to-unity.md` *Handedness
map*: MuJoCo is RH Z-up like the core model).

Inputs straight from `occt-sys` (no unity-projection, no shim changes):

- `Occt::load(library_dir, shim_path)` → `open_step(path)` →
  `StepDoc::scene(0.01, 0.5)` (same deflection/angle constants as
  `step-stats`, `verify-package.py`, and the C# importer) →
  `Scene::forest()` + `Scene::meshes()`; then `StepDoc::mesh_properties(i)`
  for each unique mesh (same call order as
  `crates/importer-host/src/doc.rs:52-69` — scene walk first, properties
  after).
- `mesh::Node` carries a **row-major 3×4 affine matrix** (not pos+quat):
  columns 0-2 rotation(+uniform scale), column 3 translation, mapping
  mesh-local OCCT points into the parent frame (`crates/mesh/src/node.rs`,
  `greyhound_abi.h:35-37`).
- `MeshProperties` (`crates/mesh/src/properties.rs`): `volume_mm3: f64`,
  `centre_of_gravity: [f64; 3]` (mm), `principal_axes: [[f64; 3]; 3]`
  (row-major, **rows are the unit axis vectors**), `principal_moments:
  [f64; 3]` (mm⁵ density-1 volume integrals, **matched to those axes**),
  `file_density: f64` (0.0 = none).

| Node/mesh datum | MJCF output | Rule |
|---|---|---|
| node translation `t` (mm) | `body pos` | `s · t`, parent-relative (matches `body pos` semantics) |
| node 3×3 `R` | `body quat` | **rigid check then Shepperd(R)**; `quat` is (w, x, y, z), w-first (modeling guide *Frame orientations*: angle a about unit axis (x,y,z) ⇔ quat (cos(a/2), sin(a/2)·(x,y,z))) |
| mesh vertices (mm) | STL vertices | `s · v`, winding untouched |
| COM `c` (mm) | `inertial pos` | `s · c`, body-local |
| principal axes `A` (rows = axes) | `inertial quat` | `Shepperd(Aᵀ)` — transposing puts the axes in columns, and MJCF's inertial frame satisfies `I_body = R · diag(m) · Rᵀ` (the C# reference does the same transpose: `StepMassProperties.cs` builds `InertiaTensorMath.Rotation` input from axis columns) |
| moments `m_i` (mm⁵) | `diaginertia` (kg·m²) | `m_i × ρ × 1e-12`, in the **same order as the axes** — independent of `--scale` (physics doesn't care about model units) |
| volume `V` (mm³) | `mass` (kg) | `V × ρ × 1e-6` — independent of `--scale` |

Density `ρ` is per unique mesh: `file_density > 0 ? file_density : --density`
(same `file > 0 ? file : parameter` policy as
`StepMassProperties.EffectiveDensity`; file densities come from
`XCAFDoc_MaterialTool::GetDensityForShape` and are treated as g/cm³ by
convention). Factor check: 1 g/cm³ = 1e-3 g/mm³ = 1e-6 kg/mm³; and
kg·m² = (1e-3 kg/g)·(s·1e-3 m/mm)²·g·mm² ⇒ `ρ × moments × 1e-12` at
ρ in g/cm³ — matches the C# path
(`mass = V×ρ×1e-6`, `inertia = mass × (r_mm×0.001)²`).

**Rigid-transform rule**: MJCF bodies have no scale, and meshes are shared
across instances, so a non-rigid node transform cannot be represented. The
exporter validates every node's 3×3: `RᵀR ≈ I` and `det(R) ≈ +1`
(tolerance 1e-4, the `verify-package.py` axes tolerance) and errors with the
node index otherwise. STEP placements (`AXIS2_PLACEMENT_3D`) are always
rigid (the mirror case is G9-theoretical), so this is a guard, not a
workaround. Rationale for erroring instead of dropping scale: silent
misplacement is worse than a loud error; per-occurrence mesh baking would
break dedup and is out of M1 scope.

**Principal-axes handedness**: `GProp` axes are orthonormal; if
`det(A) < 0`, flip the sign of one axis (its moment pairing is unaffected —
`e·eᵀ` is sign-invariant) to make `Aᵀ` a proper rotation before Shepperd,
mirroring the C# eigenvector det-fix (`InertiaTensorMath.cs:197-207`).
Orthonormality of `A` is validated at the same 1e-4 tolerance.

**Shepperd helper** (`src/quat.rs`): `matrix_to_quat([[f64; 3]; 3]) -> [f64; 4]`
(w-first), exactly the branch structure of the in-repo C# reference
(`InertiaTensorMath.cs:210-262`): trace > 0 → w-branch; else largest of
m00/m11/m22 picks the branch; normalize the result defensively. Tests
(`src/tests/quat.rs`):

- identity → `(1, 0, 0, 0)`; fixture axes `Aᵀ = I` → `(1, 0, 0, 0)`.
- 90° about Z → `(√2/2, 0, 0, √2/2)`; about X/Y analogues.
- quat→matrix→quat round-trip (helper's inverse used only in tests).
- composition: quat(from(A·B)) ≈ quat-compose(quat(from(A)), quat(from(B)))
  up to sign (|dot| = 1).

## Naming

- **Bodies**: node name from the UTF-8 name blob
  (`mesh::Scene::node_name`, reference-label name falling back to the
  referred label's — `step_mesh.cpp:226-229`), decoded lossily. Sanitize:
  map every char outside `[A-Za-z0-9_.-]` to `_` (policy, not a MuJoCo
  requirement — MJCF names are free strings). Empty result → `part{node_index}`.
  Dedupe per type with a used-set: on collision append `_2`, `_3`, … until
  free (handles STEP files where two instances legitimately share a name —
  cart-asy's two Pillow Block instances).
- **Mesh assets/files**: `mesh_000`, `mesh_001`, … (unique-mesh index,
  zero-padded); files `mesh_000.stl`. Deterministic, avoids picking a
  "winner" name for a shared asset; a comment above the asset lists the
  raw name of the first node using it.
- **Raw names preserved** as XML comments (`<!-- part: ... -->`), since
  `body/user` is numeric-only (`xmlref #body-user`) and `custom/text`
  (`<custom><text name data/>`, `xmlref #custom-text`) is machine-facing
  M2+ fodder. Comment bodies never contain `--` (stripped from raw text).
- **Model name**: STEP file stem, XML-escaped, unsanitized (display-only).

## CLI and crate layout

```
greyhound-export-mjc [--config config.toml] <file.step> --out <dir>
                     [--density 1.0] [--free-root] [--scale 0.001]
```

- `--config` default `config.toml`, same `[occt]` table as `step-stats`
  (`library_dir`, `shim_path`; relative paths canonicalized against the
  config's parent — `crates/step-stats/src/app_config.rs`). The three small
  config modules are duplicated into the exporter crate (extracting a shared
  config crate is a later cleanup, not M1).
- Output: `<out>/<step-stem>.xml`, meshes at `<out>/meshes/mesh_NNN.stl`;
  `create_dir_all`; existing files overwritten. `--scale` validated finite
  and positive (as `project_scene` does). `--density` in g/cm³, default 1.0.
  Tessellation constants hardcoded M1: deflection 0.01, angle 0.5.
- Modules (one type per file, tests under `src/tests/` + `#[cfg(test)] mod
  tests;` in main — `step-stats` precedent): `main.rs` (ExitCode +
  eprintln error, `step-stats` style), `args.rs` (clap derive),
  `app_config.rs`/`occt_config.rs`/`config_error.rs`, `error/mod.rs`
  (thiserror + `ErrorLocation` `#[track_caller]` constructors, `Result<T>`
  alias — the `crates/mesh/src/error/` pattern), `quat.rs`, `naming.rs`,
  `stl.rs`, `mjcf.rs`, `export.rs` (orchestration + `ExportSummary`).
- Writers are library-grade pure functions over
  `(&mesh::Scene, &[mesh::Mesh], &[MeshProperties], &Options)` so unit tests
  need no OCCT.

## Validation plan

### Rust (cargo test — needs the installed OCCT, like all workspace builds)

1. **Writer unit tests, synthetic fixture scene** (no OCCT): 2×1×1 mm box
   via `MeshBuilder` (12 outward-CCW triangles, 8 shared corners) +
   `Scene::try_new` (single root, identity transform, name "box") +
   hand-built `MeshProperties` with the analytic values below. Assert the
   exact MJCF text (structure, body tree, attribute values) and STL bytes
   (684 bytes, header zeros, count 12, first-triangle normal).
   A second scene exercises: grouping root + meshed child (rotated/translated
   transform → pos/quat path), two nodes sharing one mesh + same name →
   dedupe + one asset.
2. **End-to-end through the real shim** (pattern:
   `crates/importer-host/src/tests/host_abi.rs` — `workspace_root()` +
   `occt_dirs()` helpers, `Occt::load`): export `assets/cart-asy.step` to a
   `tempfile` dir. Assert: 43 bodies written, 20 assets, 39 geoms, per-node
   mass = `volume_mm3 × ρ × 1e-6`, Σ mass = **0.2554502 kg** (tol 1e-6;
   measured 255,450.2 mm³ over 39 part occurrences at 1.0 g/cm³),
   cart-part COM in the MJCF = **(0.0169977, 0.0629348, −0.0002281) m** —
   the identity-map image of OCCT (16.9977, 62.9348, −0.2281) mm; note Unity
   gets (0.0169977, −0.0002281, 0.0629348) (the (x, z, y) permutation) —
   this asymmetric pair is the cheapest identity-map regression check.
   Pillow Block node translation OCCT (17, 18, −19) mm → body pos
   (0.017, 0.018, −0.019) vs Unity (0.017, −0.019, 0.018). Also export
   `rod-clamp-16mm.stp` with `--free-root` (meshed root) and assert the
   `<freejoint/>` line + mass 3.18262e-3 kg (3,182.62 mm³).

   Analytic fixture numbers (density 1.0 g/cm³, box `[0,2]×[0,1]×[0,1]` mm):

   | quantity | value | derivation |
   |---|---|---|
   | volume | 2 mm³ | — |
   | COM | (1, 0.5, 0.5) mm → `inertial pos (1e-3, 5e-4, 5e-4)` | × 0.001 |
   | mass | 2e-6 kg | V × ρ × 1e-6 |
   | moments (mm⁵) | (1/3, 5/6, 5/6) matched to (x, y, z) axes | V/12·(b²+c²) etc. for extents (a,b,c)=(2,1,1) |
   | diaginertia (kg·m²) | ≈ (3.333333e-13, 8.333333e-13, 8.333333e-13) | × ρ × 1e-12 |
   | inertial quat | (1, 0, 0, 0) | axis-aligned box ⇒ A = I |

### MuJoCo (user-run; `pip install mujoco`, record the version)

```python
import mujoco
m = mujoco.MjModel.from_xml_path("cart-asy.xml")   # meshes resolve via meshdir
assert m.nbody == 44        # world + 43 nodes
assert m.nmesh == 20        # unique mesh assets
assert m.ngeom == 39        # one geom per meshed occurrence
assert abs(m.body_mass.sum() - 0.2554502) < 1e-6
for i in range(1, m.nbody):
    name = mujoco.mj_id2name(m, mujoco.mjtObj.mjOBJ_BODY, i)
    print(i, name, m.body_mass[i], m.body_ipos[i], m.body_iquat[i], m.body_inertia[i])
# spot-checks: cart body body_ipos ≈ (0.0169977, 0.0629348, -0.0002281);
# per-body body_inertia ≈ MeshProperties.principal_moments × ρ × 1e-12 (same order);
# body_iquat matches the emitted quat up to sign (|dot| ≈ 1).
```

Visual smoke test: `python -m mujoco.viewer --mjcf cart-asy.xml` — the
assembly must look like the STEP in a CAD viewer (same debugging rule as
STL: any mirroring is a bug in the exporter, never in the data). Optional:
repeat for the fixture-box export and watch it fall/float per `--free-root`.

## Corrections / flags for existing docs

- `unity-mujoco.md:112-113` quotes the fixture box's "analytic principal
  moments 0.4166667/0.8333333/0.8333333" — those came from the **stub ABI
  fixture** (`crates/occt-sys/cpp/tests/loader_fixture.cpp:76-91`), whose
  hand-filled values are not physically consistent with a 2×1×1 box (its own
  comment says "mass-1"). The correct density-1 values are
  **(1/3, 5/6, 5/6) mm⁵** → (3.333333e-13, 8.333333e-13, 8.333333e-13) kg·m²
  (table above). Suggest fixing the numbers in `unity-mujoco.md` (and
  optionally the stub) during M1; the stub itself is an ABI-plumbing fixture
  and stays out of the exporter's test path.
- MuJoCo mesh **recentering** (`mesh_pos`/`mesh_quat`) is a fact absent from
  `unity-mujoco.md`; it is harmless for M1 (default geom pose renders file
  coordinates) but must be known before M3 joint work touches `mesh_vert`
  data.

## Decisions closed here (recommendations — confirm at implementation start)

1. Binary crate `greyhound-export-mjc` (not a step-stats flag).
2. Config: duplicate the `[occt]` config modules; CLI shape per Scope recap.
3. Mesh asset/file naming `mesh_NNN`; body names sanitized + deduped; raw
   names in comments.
4. Non-rigid node transform → hard error (no MJCF scale).
5. `--free-root` with a massless root → pre-error before writing files.
6. No `<option>` emitted (defaults incl. Z-up gravity).
7. Float formatting helper (Display / `{:e}` split).
8. Fixture expectations per the analytic table (correcting the doc drift
   flagged above).

## References

- MuJoCo XML reference (stable): <https://mujoco.readthedocs.io/en/stable/XMLreference.html>
  (anchors used: `#compiler-*`, `#asset-mesh`, `#worldbody`, `#body`,
  `#body-inertial`, `#body-joint`, `#body-freejoint`, `#custom-text`,
  `#option-gravity`)
- Modeling guide: <https://mujoco.readthedocs.io/en/stable/modeling.html>
  (*Kinematic tree*, *Naming elements*, *Frame orientations*)
- Python bindings: <https://mujoco.readthedocs.io/en/stable/python.html>
  (`from_xml_path`, named access, viewer)
- Dev-tree sources for compile-behavior checks (fetched 2026-10-09):
  `src/user/user_model.cc` (massless moving bodies, name dedup),
  `src/user/user_mesh.cc` (hull failure modes)
- Repo: `unity-mujoco.md` (M1–M4 plan), `occt-to-unity.md` (handedness map,
  mass properties, compound-integral caveat), `occt-mesh.md` (mesh model,
  `IMeshTools_Parameters`), `crates/mesh/src/{node,scene,mesh,properties}.rs`,
  `crates/occt-sys/src/{occt,step_doc}.rs`,
  `crates/step-stats/src/{main,args,app_config}.rs`,
  `crates/importer-host/src/tests/host_abi.rs`,
  `package/com.greyhound.step/Runtime/{InertiaTensorMath,StepMassProperties}.cs`,
  `tools/verify-package.py` (43/20/15 constants, cart volume/COM),
  `assets/{cart-asy.step,rod-clamp-16mm.stp}`