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
  `StepActuator` assets, set folder locate/create, `Add >` menu items
  with validation, auto-select. No components attach to the imported
  hierarchy.
  Verify: menus create the asset family on cart-pole; default SO
  inspectors are editable; container lists populate.
- **M2b — UIToolkit editors**: the three `[CustomEditor]`s + six
  UXML/USS files, bound `PropertyField`s, container ListViews as the set
  overview (ListView has lived in `UnityEngine.UIElements` since Unity
  6 — the UXML spells it `ui:ListView`; the legacy
  `UnityEditor.UIElements` namespace has no `UxmlElement` descriptor and
  fails to instantiate); the `stepAssetPath` drawer re-skins as a
  `CreatePropertyGUI` UIToolkit binding and the IMGUI plumbing is
  deleted; the joint's `StepSiblingPath` renders through its own
  `PropertyDrawer` as an immutable hierarchy accordion (live chain from
  the assembly root, creation-time name alongside) with a drop zone for
  drag-to-rebind — the re-pick affordance. Verify: styled forms render
  and edit; overview pings parts.
- **M2c — scene authoring**: axis-pick rays, `PositionHandle`, gizmos;
  Hierarchy marker hook (persistent tint
  + bar on jointed part rows via `hierarchyWindowItemByEntityIdOnGUI`,
  membership through the containers, nothing stored on parts). Verify:
  transcribe the real cart slide + pole hinge via axis pick + fields;
  signs and anchors confirmed against the rendered geometry. This is
  the UX payoff shard.
- **M2d — reimport survival + lifecycle**: AssetPostprocessor
  validation by sibling-path resolution, unmatched-path Console
  reporting, container prune on deleted assets. Verify: reimport →
  unmatched joints reported; delete a joint → prune.

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
  the path is the only durable link; a `PropertyDrawer` renders it as an
  asset link to the .stp — drag, ping, or clear — while the string stays
  the serialized store), `rootMobility: free | welded`,
  `List<StepJoint> joints`, `List<StepActuator> actuators` — **direct
  object references; no GUIDs anywhere**.
- `StepJoint` (one .asset per joint): the SO carries identity and Unity
  references only — `root` (reference to the owning `StepJointSet`; the
  required affordance, durable because .asset files are not reimported),
  the asset's own `name` (the MJCF `<joint name>`), and `body`, a
  `StepSiblingPath`: the **sibling-index path from the assembly root as
  the source of truth** for the body the joint is defined in — the
  hierarchy rebuilds deterministically per STEP file, so the path is
  rename-robust and distinguishes duplicate-named parts (the
  duplicate-name warning now guards MJCF body-name collisions at
  export, an M3 concern, not keying) — plus `mj`, an `MjJoint` payload:
  the `[Serializable]` MJCF `<joint>` mirror with attribute names
  verbatim (`type`, `pos` (m), `axis`, `limited` + `rangeLo/hi` —
  the spec's own limit switch for the `range` attribute; renamed from
  the earlier invented `rangeEnabled` when the actuator's
  `ctrllimited`/`forcelimited` completed the spec mirror), `damping`,
  `frictionloss`, `armature`. No parent field — parenting is the
  hierarchy's job. Reimport conflict resolution (compare the resolved
  part's name against the stored one; accept rename or re-pick) is
  **deferred** — the design lives in the `StepSiblingPath` comment;
  until then a shifted index path rebinds to whatever sits at the
  position now.
- `StepActuator` (one .asset per actuator): the SO carries `root` (the
  required affordance), the asset's own `name`, `target joint` (object
  reference to a `StepJoint`) — plus `mj`, an `MjActuator` payload: the
  `[Serializable]` MJCF `<position>` mirror (`type`, `forcerange lo/hi`,
  `ctrlrange lo/hi` in joint units). Nothing deployment-side lives in
  the schema: calibration (homing end, approach speed, zero offset) and
  transmission constants (the belt pitch radius) enter when their
  consumer exists — at M3 or on the deployment side — never as schema
  fields. Mirrors MJCF, where `<actuator>` is a separate element
  referencing a joint by name — the joint asset stays kinematics-only.
- The `Mj*` payload classes (`MjJoint`, `MjActuator`) are pure,
  attribute-verbatim MJCF mirrors and the future seam for MJCF XML
  serialization: the M3 exporter writes XML elements from them, and
  Unity-specific concerns never leak into them.
- The asset family is the **data source of truth for articulation** —
  STEP has no joint semantics to import — while geometry remains the
  STEP's truth; the exporter merges the two, so the assets must survive
  reimports of the geometry they annotate. MJCF mapping of the range
  split (joint limits → joint `range`, actuator `ctrlrange` → actuator
  envelope) is settled with the exporter bite, not M2.

Scene integration and editor UX (decided 2026-10-09; revised same day —
no components anywhere in the imported hierarchy: the asset family is
self-sufficient, keyed and resolved by STEP part names):

- Authoring rides Unity's own affordances — **no custom window**.
  Right-click a part in the Hierarchy → `Add > Joint > Slide` /
  `Add > Joint > Hinge` / `Add > Actuator` (GameObject menu items,
  rendered in the Hierarchy context menu, the Hierarchy `+` dropdown,
   and the GameObject menu bar). Validation greys the joint items when
   the selection is not a STEP-imported part or already has a joint in
   the set for its position, and greys the actuator item when the
   selection has no unambiguous joint. The menu handler walks up from
   the selection to its prefab
  root, resolves the source `.stp` path, and finds or creates the set
  folder + container. The parent of a joint is never picked: it is the
  part's hierarchy parent — the parent–child rule holds by construction
  (and matches MJCF, where a joint couples a body to its implicit tree
  parent).
- Joint creation: inserts a `StepJoint` .asset into the set folder
  (named after the part), appends it to the container's
  list, and auto-selects the asset — its serialized fields render
  **natively** in the Inspector; no form-proxy editor needed. Nothing is
  attached to the part GameObject: the SO is self-sufficient, and M2c/M2d
  resolve the part by sibling-index path through the container's
  `stepAssetPath` → prefab instance → indexed walk.
- Axis pick: six `Handles.Button` rays at the part's origin (±X/±Y/±Z in
  the body's local frame); clicking a ray sets the axis **with sign**.
  A slide needs nothing more (its joint pos is render-only); a hinge's
  pos is then dragged with a `PositionHandle`. Esc cancels; the
  joint asset's editor offers a re-pick button. CAD axes are orthogonal
  — free axis rotation is deliberately impossible.
- Actuator creation: `Add > Actuator` on a part that carries a joint;
  the new `StepActuator` (type `position`, v1) targets that joint
  (object reference) and is auto-selected — drive fields (forcerange,
  ctrlrange) render natively.
- Deletion: a joint or actuator is its own .asset — delete it in the
  Project window; the container drops the cleared list slot (M2d scan),
  and actuators targeting the deleted joint show the broken reference
  natively in the Inspector.
- Custom editors are **UIToolkit, not IMGUI**: `StepJointSet`,
  `StepJoint`, and `StepActuator` each get a `[CustomEditor]`
  implementing `CreateInspectorGUI()`. Forms are **UXML assets styled by
  USS** shipped in the package's `Editor/UI/`; serialized fields render
   as bound `PropertyField`s, so layout changes are asset edits, not code
   edits. Numeric editing is **meters native**, axis nonzero, lo < hi.
   Each editor holds serialized references to its own pair from the
   package (`Editor/UI/<Type>.uxml` + `<Type>.uss`), assigned on the
   editor's script asset; stylesheets are applied in code — package-
   relative `Style src` in UXML is unreliable — and the shipped .meta
   GUIDs keep the assignments stable. The layout is data, editable in
   place, no recompile.
- The joint asset's editor keeps the scene tools in the
  `SceneView.duringSceneGui` callback (Handles are not UXML — only the
  forms are; Unity 6.7 removed the per-editor `OnSceneGUI` hook): the
  axis-pick rays, the `PositionHandle`, and the gizmo (axis line,
  rotation arc / travel double-arrow across the limits) while that
  joint owns the tools — the editor claims a static authored-joint
  slot on enable, so the tools keep working while its inspector is
  **locked** and the selection is elsewhere; the claim clears when the
  editor dies while deselected (unlocked lifecycle), another joint's
  editor opens, or the asset is deleted. The re-pick button also
  selects the asset first, so the authored asset stays pinged in the
  Project window.
- The container's UIToolkit editor doubles as the set overview: its
  `ListView`s of joints/actuators ping-select entries and their parts —
  the surface the dropped window would have been.
- `rootMobility` is a plain field on the container asset, edited on the
  container's own Inspector (M2b's overview editor presents it with the
  rest); the free-root-with-zero-joints path is the container's
  CreateAssetMenu entry. Reimport failures (unmatched names, duplicates)
  surface as Console messages — there is no window to report them in.
- Reimport survival: annotations live in the asset family, never on the
  imported hierarchy (a reimport recreates the GameObjects), so nothing
  on the parts needs restoring. After each `.stp` reimport, an
  AssetPostprocessor hook validates the fresh instance by resolving each
  joint's sibling path in it (the deferred conflict flow compares the
  resolved name against the stored one) — fully automatic, no Apply
  button
  anywhere. Unmatched part names surface as Console errors ("2 of 5
  joints could not be placed"), never silently dropped. Deeper
  importer-settings integration deferred.
- V1 restriction: joints on **parent–child edges** only (arbitrary pairs
  need inserted intermediate fixed bodies — later).
- Validation is manual in the user's Unity (6.7.0b1): transcribe the
  FreeCAD joint tree's cart slide + pole hinge (right-click →
  `Add > Joint`, axis pick, fields), add the position actuator
  (`Add > Actuator`), reimport, and confirm the asset family survives
  the reimport with no unmatched-part errors. No Rust, shim, or host
  changes; nothing to build outside Unity.

## Bite M3 — exporter consumes joints

Decided at M3 start (2026-10-10):

- **The exporter is C#, in the Unity package.** The `Mj*` payload classes
  were designed as the MJCF XML serialization seam ("the M3 exporter
  writes XML elements from them"), so the Export button on the set
  overview writes the model directly from the imported prefab hierarchy
  and the annotation family. The two transport options left open at M2
  (C ABI into the host, probe-side JSON feeding the Rust CLI) are
  rejected: both would bolt an interchange layer onto a data model that
  already carries MJCF's attribute names, and the CLI path would ship a
  second native binary. The Rust `greyhound-export-mjc` stays frozen at
  M1 as the reference implementation of the rigid-tree schema.
- **Frame math lives in the C# exporter**, mirroring the importer's
  map: Unity→MJCF is the `(x, z, y)` permutation for points and
  directions (its own inverse — everything is already meters) and the
  `M·R·M` conjugation for rotations, including the inertial principal
  axes. Verified against M1's validated cart-asy numbers: Unity COM
  `(0.017, −0.000228, 0.0629)` m permutes to the MJCF COM
  `(0.017, 0.0629, −0.000228)` m.
- **Output location**: `<step file name>~` beside the `.stp` — the
  trailing tilde puts the folder on Unity's ignore list, so the XML and
  the `meshes/` STLs never enter the AssetDatabase.
- **Emission is a literal mirror** of the data model, including the
  spec's own limit switches: `ctrllimited`/`forcelimited` (actuator) are
  emitted verbatim as "true"/"false" — explicit `false` disables
  clamping even when a range is present (`xmlref
  #actuator-general-ctrllimited`), which makes the emitted file valid
  under autolimits with no heuristic. Ranges are emitted only when
  their limit switch is true (a default `0 0` range would otherwise
  trip the strict lo<hi validation). Joint `range` follows the joint's
  limit switch the same way. The range split maps verbatim per field:
  joint `rangeLo/Hi` → `<joint range>`, actuator `ctrlLo/Hi`/`forceLo/Hi`
  → the `<position>` attributes.
- **Actuation is 1:1**: `<position name joint=... ctrlrange forcerange>`
  with no `gear` attribute — transmission constants (belt pitch radius)
  enter only when their consumer exists, never as schema fields (the
  M2a purge already settled this; the stale "gear scaling vs ctrl
  semantics" question is struck).
- **A moving body needs mass — MuJoCo's own rule**: the exporter
  transcribes the compiler's `CheckBodyMassInertia` (checked against
  `user_model.cc`, current main): a jointed body or free root is
  exportable iff its own mass properties exist, or some static
  (joint-free) descendant carries mass — a massless assembly frame with
  welded meshy children is legal and emits no inertial of its own. The
  refusal survives only when the welded subtree is entirely massless
  (a massless body between joints cannot compile).

Mapping: revolute → `<joint type="hinge" pos axis range>`, prismatic →
`type="slide"`, in the child body; an edge without a joint stays welded;
`rootMobility` chooses `<freejoint/>` vs static root; an actuator whose
target joint is missing is an export error.

Validation: MuJoCo compiles; joint count/types/axes match the asset;
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
