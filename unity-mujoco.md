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

Sharded 2026-10-09 — four worktree bites in dependency order, each ending
in a manual Unity validation:

- **M2a — data model + creation**: `StepJointSet`/`StepJoint`/
  `StepActuator` assets, `StepJointHandle`, root-mobility component, set
  folder locate/create, `Add >` menu items with validation, auto-select.
  Verify: menus create the asset family on cart-pole; default SO
  inspectors are editable; container lists populate.
- **M2b — UIToolkit editors**: the three `[CustomEditor]`s + six
  UXML/USS files, bound `PropertyField`s, container ListViews as the set
  overview. Verify: styled forms render and edit; overview pings parts.
- **M2c — scene authoring**: axis-pick rays, `PositionHandle`, gizmos,
  wiggle slider + pose restore. Verify: transcribe the real cart slide +
  pole hinge via axis pick + fields; wiggle both — signs and anchors
  confirmed. This is the UX payoff shard.
- **M2d — reimport survival + lifecycle**: AssetPostprocessor
  auto-restore by `childPart` matching, unmatched-name Console
  reporting, container prune on deleted assets. Verify: reimport →
  handles return; delete a joint → prune.

Decided post-M1 (2026-10-09):

- **Actuation ships in the M2 schema as first-class per-actuator assets**
  (the potpourri "M2 consequence") — authored once, so a later exporter
  bite consumes it without re-authoring; the M2 exporter surface is
  untouched.
- **The annotation family lives in a folder beside the STEP file**:
  `<step-stem>.joints/` next to `<step-stem>.stp` (e.g.
  `cart-pole-asm.joints/` beside `cart-pole-asm.stp`), containing the
  container asset plus one .asset file per joint and per actuator. All
  authoring artifacts for a STEP sit in one folder, adjacent to the
  geometry they annotate.
- **ScriptableObjects are the only serialization.** No interchange file
  is built in M2; how the exporter consumes the set (Unity-triggered
  export per the potpourri endgame — C ABI or probe-side JSON) is decided
  when M3 starts.

Data model (`Greyhound.Step`, Runtime assembly):

- `StepJointSet` (the container asset, `<step-stem>.joints.asset`):
  `stepAssetPath` (the .stp's project path — the stable key; a serialized
  reference to the imported root GameObject dies on every reimport, so
  the path is the only durable link), `rootMobility: free | welded`,
  `List<StepJoint> joints`, `List<StepActuator> actuators` — **direct
  object references; no GUIDs anywhere**.
- `StepJoint` (one .asset per joint): `name`, `child part` (STEP part
  name), `parent part`, `type: slide | hinge`, `origin` (child-local, m),
  `axis` (child-local unit), `limits lo/hi` (optional — the mechanical
  range), `damping`, `frictionloss`, `armature`. Pure kinematics. Entries
  key on **STEP part names** — stable across reimports, unlike node
  indices; creation warns on duplicate part names, since keying assumes
  unique occurrence names.
- `StepActuator` (one .asset per actuator): `name`, `target joint`
  (object reference to a `StepJoint`), `type: position` (v1), belt pitch
  radius (m), `forcerange lo/hi`, `ctrlrange lo/hi` (the operating
  range), calibration field (homing end, approach speed, zero offset).
  Mirrors MJCF, where `<actuator>` is a separate element referencing a
  joint by name — the joint asset stays kinematics-only.
- The asset family is the **data source of truth for articulation** —
  STEP has no joint semantics to import — while geometry remains the
  STEP's truth; the exporter merges the two, so the assets must survive
  reimports of the geometry they annotate. MJCF mapping of the range
  split (joint limits → joint `range`, actuator `ctrlrange` → actuator
  envelope, calibration → deployment-side only) is settled with the
  exporter bite, not M2.

Scene integration and editor UX (decided 2026-10-09):

- Authoring rides Unity's own affordances — **no custom window**.
  Right-click a part in the Hierarchy → `Add > Joint > Slide` /
  `Add > Joint > Hinge` / `Add > Actuator` (GameObject menu items,
  rendered in the Hierarchy context menu, the Hierarchy `+` dropdown,
  and the GameObject menu bar). Validation greys the joint items when
  the selection is not a STEP-imported part or already carries a joint
  handle, and greys the actuator item when the selection carries no
  joint. The menu handler walks up from the selection to its prefab
  root, resolves the source `.stp` path, and finds or creates the set
  folder + container. The parent of a joint is never picked: it is the
  part's hierarchy parent — the parent–child rule holds by construction
  (and matches MJCF, where a joint couples a body to its implicit tree
  parent).
- Joint creation: inserts a `StepJoint` .asset into the set folder
  (default name from the child part), appends it to the container's
  list, attaches a lightweight `StepJointHandle` component to the part
  holding a **direct object reference** to the joint asset, and
  auto-selects the asset — its serialized fields render **natively** in
  the Inspector; no form-proxy editor needed. The handle's only jobs are
  scene-side: host the gizmo/handles and give reimport re-binding a
  per-part attachment point.
- Axis pick: six `Handles.Button` rays at the part's origin (±X/±Y/±Z in
  the child's local frame); clicking a ray sets the axis **with sign**.
  A slide needs nothing more (its joint pos is render-only); a hinge's
  origin is then dragged with a `PositionHandle`. Esc cancels; the
  joint asset's editor offers a re-pick button. CAD axes are orthogonal
  — free axis rotation is deliberately impossible.
- Actuator creation: `Add > Actuator` on a part that carries a joint;
  the new `StepActuator` (type `position`, v1) targets that joint
  (object reference) and is auto-selected — drive fields (pitch radius,
  forcerange, ctrlrange, calibration field) render natively.
- Deletion: a joint or actuator is its own .asset — delete it in the
  Project window; the container drops the cleared list slot, and a
  handle whose joint reference is gone flags itself in the Inspector.
- Custom editors are **UIToolkit, not IMGUI**: `StepJointSet`,
  `StepJoint`, and `StepActuator` each get a `[CustomEditor]`
  implementing `CreateInspectorGUI()`. Forms are **UXML assets styled by
  USS** shipped in the package's `Editor/UI/`; serialized fields render
  as bound `PropertyField`s, so layout changes are asset edits, not code
  edits. Numeric editing is **meters native**, axis nonzero, lo < hi.
  Each editor references its own pair from the package
  (`Editor/UI/<Type>.uxml` styled by `<Type>.uss`) — the layout is
  data, editable in place, no recompile.
- The joint asset's editor keeps the scene tools in `OnSceneGUI` (Handles
  are not UXML — only the forms are): the axis-pick rays, the
  `PositionHandle`, the **wiggle slider's** kinematics, and the gizmo
  (axis line, rotation arc / travel double-arrow across the limits)
  while the part is selected. The wiggle slider itself is a UXML slider
  (prismatic across the mechanical range, revolute −π..π or the authored
  limits; pose restores on release — the check that catches sign flips
  and off-pivot anchors before any export).
- The container's UIToolkit editor doubles as the set overview: its
  `ListView`s of joints/actuators ping-select entries and their parts —
  the surface the dropped window would have been.
- `rootMobility` has no per-part home: a lightweight root-level
  component (attached automatically by the first joint creation; add
  manually for free-root with zero joints) edits the container's
  `rootMobility`. Reimport failures (unmatched names, duplicates)
  surface as Console messages — there is no window to report them in.
- Reimport survival: annotations live in the asset family, never on the
  imported hierarchy (a reimport recreates the GameObjects). After each
  `.stp` reimport, an AssetPostprocessor hook **auto-restores** the
  handles on the scene instance by matching part names to
  `StepJoint.childPart` — fully automatic, no Apply button anywhere.
  Unmatched part names surface as Console errors ("2 of 5 joints could
  not be placed"), never silently dropped. Deeper importer-settings
  integration deferred.
- V1 restriction: joints on **parent–child edges** only (arbitrary pairs
  need inserted intermediate fixed bodies — later).
- Validation is manual in the user's Unity (6.7.0b1): transcribe the
  FreeCAD joint tree's cart slide + pole hinge (right-click →
  `Add > Joint`, axis pick, fields), add the position actuator
  (`Add > Actuator`), reimport, and confirm the asset family survives
  and the handles auto-restore; wiggle each joint to verify signs and
  anchors. No Rust, shim, or host changes; nothing to build outside
  Unity.

## Bite M3 — exporter consumes joints

- Map revolute → `<joint type="hinge" pos axis range>`, prismatic →
  `type="slide"`, in the child body; limits → `range` (autolimits infers
  `limited`); an edge without a joint stays welded; `rootMobility` chooses
  `<freejoint/>` vs static root. A `StepActuator` on a joint emits
  `<position>` targeting it (`forcerange`/`ctrlrange` verbatim; how belt
  pitch radius enters — gear scaling vs ctrl semantics — is an M3
  decision); an actuator whose target joint is missing is an export
  error.
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

- Exporter crate vs `step-stats` flag — settled: standalone binary crate
  `greyhound-export-mjc` (M1).
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
