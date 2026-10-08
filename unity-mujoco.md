# MuJoCo MJCF export + joint authoring (facts and plan)

The second (and third) consumer of the core mesh model, after the Unity
projection: an MJCF exporter for MuJoCo, then joint authoring on top of it.
Companion to `occt-to-unity.md` (transformation pipeline, mass properties)
and `occt-mesh.md` (mesh data model). Facts below were checked against the
MuJoCo stable XML reference on 2026-10-08.

## MuJoCo / MJCF facts

- MJCF is XML: `<mujoco>` → `<compiler>`, `<option>`, `<asset>`,
  `<worldbody>`, `<contact>`, `<equality>`, `<actuator>`. Bodies form a
  kinematic tree under `<worldbody>`; each `<body>` has `pos`/`quat`
  relative to its parent.
- **A body without a joint is rigidly welded to its parent.** Bodies welded
  to the worldbody (directly or up the chain) are static. "Movement vs
  static" is therefore purely additive: joints create DOFs, everything else
  is rigid.
- Units are agnostic; SI (meters, kilograms) is the convention. Gravity
  defaults to `(0, 0, −9.81)` — the world is Z-up, right-handed.
- `<compiler>` attributes this export relies on:
  - `autolimits` defaults **true**: `limited` is inferred from the presence
    of `range` — emit `range`, never emit `limited`.
  - `angle="radian"` — all angles in MJCF are radians under this setting.
  - `meshdir` — base directory for mesh files.
  - `inertiafromgeom` is `[false, true, auto]` defaulting **auto**: an
    explicit `<inertial>` wins when present; otherwise inference from geoms.
    We always emit explicit inertials, so inference never fires.
  - `balanceinertia` defaults false — exact-BRep inertias are physically
    valid, leave it off.
  - `fusestatic` defaults false for MJCF (true is the URDF convention) —
    static subtrees stay separate bodies.
- `<asset><mesh name file></asset>` — mesh files may be binary STL, OBJ, or
  MuJoCo's binary MSH; inline `vertex`/`face` attributes also exist. The
  asset has a `scale` attribute (`1 1 1` default), but baking units into the
  exported file keeps the STL standalone-correct, so we do that instead.
- Mesh geoms (`<geom type="mesh" mesh="...">`) are convex-hulled for
  collision (see `maxhullvert`); concave collision needs decomposition, out
  of scope. The `<mesh inertia>` attribute (legacy vs convex) only affects
  *inferred* inertia — irrelevant while we emit explicit `<inertial>`.
- `<inertial pos quat mass diaginertia>` — `diaginertia` is the principal
  moment diagonal expressed in the frame given by `quat`, at `pos`
  (body-local). Alternative: `fullinertia` (6 unique components,
  body frame) — MuJoCo eigendecomposes it internally; we emit
  pos+quat+diaginertia because the core model already stores principal
  axes with moments *matched to those axes*.
- `<joint type="hinge" | "slide" | "ball" | "free">` — the joint lives
  **in the child body**; `pos` (anchor) and `axis` are body-local;
  `range="lo hi"` is radians (hinge) or meters (slide). `<freejoint/>` on
  the root body makes an assembly floating instead of static.
- `<geom density>` defaults to **1000 kg/m³** (water) — numerically the same
  default as the importer's 1.0 g/cm³ fallback.

## Frame, units, mapping

- Core model: OCCT millimeters, Z-up, right-handed, triangles outward-CCW in
  OCCT algebra. MuJoCo: meters, Z-up, right-handed. Unity is the only
  left-handed consumer in the set — see the handedness map in
  `occt-to-unity.md`.
- **The MuJoCo projection is the identity axis map plus the mm→m scale** —
  no conjugation, no winding flip, unlike Unity's `(x, z, y)` reflection.
  This is the architectural claim (every consumer gets its own projection,
  never Unity assumptions) getting its second customer. Bake the scale into
  STL vertices and into body `pos`/`inertial pos`.
- Winding stays as-is: MuJoCo convex-hulls mesh geoms for collision and
  does not depend on STL normal consistency.
- Joints are *authored* in Unity space against Unity geometry (child-local
  meters). The exporter conjugates them into the OCCT-frame MJCF with the
  same math as node transforms: `R' = M·R·M`, `t' = s·M·t`,
  `M = (x, z, y)` — written and composition-tested in the Unity projection.
- Validation numbers come from **per-part** exact-BRep sums, never from a
  single-call whole-compound integral (OCCT's compound integral is not the
  sum of its parts; see the mass-properties section of `occt-to-unity.md`).
  Measured: cart-asy has 39 part occurrences summing 255,450.2 mm³ →
  **0.2554502 kg** at 1.0 g/cm³; cart part 53,605.59 mm³; rod-clamp-16mm
  3,182.62 mm³.

## Where it lives

Exporter = Rust-side consumer of the core model, like the Unity projection.
No shim, host-ABI, or C# changes: `occt-sys` already exposes the scene
(nodes, names, local transforms, mesh refs) and per-mesh `MeshProperties`.
Shape to settle at M1 start: a `greyhound-export-mjc` binary crate, or a
`--mujoco <dir>` flag on `step-stats`. Workspace conventions apply
(dependencies in `[workspace.dependencies]`, one type per file, tests under
`src/tests/`).

## Bite M1 — naive rigid-tree exporter

CLI: `greyhound-export-mjc <file.step> --out <dir> [--density 1.0]
[--free-root] [--scale 0.001]`.

- Binary STL writer over the core mesh model: per-triangle float32 with the
  normal derived from winding; scale baked into vertices.
- MJCF writer:
  - `compiler`: `angle="radian"`, `meshdir="meshes"`, `autolimits="true"`,
    `inertiafromgeom="auto"`, `balanceinertia="false"`.
  - `asset`: one `<mesh name="..." file="...">` per unique mesh.
  - `worldbody`: one `<body name pos quat>` per scene node (grouping nodes
    become geometry-free bodies); meshed bodies get
    `<geom type="mesh" mesh="..."/>` and
    `<inertial pos="com" quat="principal axes" mass="..." diaginertia="...">`
    — mass = volume × density × 1e-6 (kg), density = file density when the
    file carries one, else the CLI parameter (same `file > 0 ? file :
    parameter` policy as C#).
  - Root: `<freejoint/>` iff `--free-root`, else the assembly is static.
- Naming: sanitize STEP part names into XML identifiers; dedupe with
  suffixes; keep the raw name in a comment or `user` attribute if useful.
- Quaternions: writer-side 3×3-rotation-matrix → quaternion helper
  (Shepperd), unit-tested by composition against the fixture box.
- Validation:
  - Rust tests: the 2×1×1 fixture box (2 mm³, analytic principal moments
    0.4166667/0.8333333/0.8333333 about its own axes) exports to a known
    MJCF; assert body tree, inertial values, scale.
  - With `pip install mujoco`: `mujoco.MjModel.from_xml_path` compiles;
    assert `nbody`, `nmesh`, Σ`body_mass` = 0.2554502 kg for cart-asy,
    per-body inertia matches `MeshProperties` within float noise.
  - Optionally open in `mujoco.simulate` for a visual smoke test.
- Out of M1: joints (M2/M3), colors/materials (G13 palette → `geom rgba` —
  decided during M1 or M3), actuation, keyframes, sensors.

## Bite M2 — joint annotation model + editor UX

- `StepJointSet` ScriptableObject: entries of
  `{name, child part, parent part, type: revolute|prismatic, origin
  (child-local, m), axis (child-local unit), limits lo/hi (optional)}`,
  plus `rootMobility: free | welded`. Entries key on **STEP part names** —
  stable across reimports, unlike node indices. The asset is the **data
  source of truth for articulation** — STEP has no joint semantics to
  import — while geometry remains the STEP's truth; the exporter merges the
  two, so the asset must survive reimports of the geometry it annotates.
- Reimport survival: annotations live in the asset, never on the imported
  hierarchy (a reimport recreates the GameObjects). The window's Apply
  attaches lightweight marker components to the current scene; after a
  reimport, Apply restores them from the asset. Deeper importer-settings
  integration deferred.
- Editor window (`Step Joints`): list/add/edit/delete, part pickers bound
  to the selected prefab's hierarchy.
- Scene-view authoring: `Handles.PositionHandle` for the joint origin, axis
  snap buttons (±X/±Y/±Z of the child's local frame) for v1; a marker
  component draws the axis line plus a rotation arc (revolute) or double
  arrow (prismatic) for the selected joint.
- V1 restriction: joints on **parent–child edges** only (arbitrary pairs
  need inserted intermediate fixed bodies — later).

## Bite M3 — exporter consumes joints

- Map revolute → `<joint type="hinge" pos axis range>`, prismatic →
  `type="slide"`, in the child body; limits → `range` (autolimits infers
  `limited`); an edge without a joint stays welded; `rootMobility` chooses
  `<freejoint/>` vs static root.
- Conjugate the Unity-authored joint frames into the OCCT-frame MJCF.
- Validation: MuJoCo compiles; joint count/types/axes match the asset;
  Σ`body_mass` unchanged from M1's rigid case; drive a hinge in `simulate`
  and watch the subtree move while welded siblings stay rigid.

## Bite M4 (later) — geometry snapping

- Raycast vertex-normal axis pick (prismatic on flat faces); two-click
  cylinder axis solve (revolute on bores/shafts). Precise variant: a new
  shim query returning per-part cylinder face axes (`TopAbs_CYLINDER` via
  `BRepAdaptor_Surface`), enabling snap-to-real-axis proposals.

## Open decisions / deferred

- Exporter crate vs `step-stats` flag (M1 start).
- Colors: G13 palette → `geom rgba` per submesh... a mesh geom has one
  rgba; per-face color would need submesh split or texture — decide at M1
  or M3 whether v1 ships uncolored.
- Concave collision (convex-hull coarseness, `maxhullvert`, coacd): deferred.
- URDF as a second writer: deferred; MJCF first.
- The scale default (0.001) ties into the open unit-scale question G6.
- Articulation beyond hinge/slide, actuation, sensors: out of scope until
  the joint UX exists.

## References

- MuJoCo XML reference (stable):
  <https://mujoco.readthedocs.io/en/stable/XMLreference.html>
- MuJoCo modeling guide:
  <https://mujoco.readthedocs.io/en/stable/modeling.html>
- `occt-to-unity.md` — axis-map derivation, mass properties (incl. the
  compound-integral caveat), gap table.
- `occt-mesh.md` — mesh data model and the no-own-mesh-format conclusion.
- `tools/verify-package.py` — measured scene constants (43 nodes / 20
  unique meshes / 39 meshed occurrences for cart-asy).
