# Marker authoring and synthetic capture plan

Plan for the object-detection markers on the imported assemblies, the
ground-truth labeling loop they enable, and the detector training that
consumes the dataset. The design rationale (why ML detection over
OpenCV fiducials, the randomization envelope, three-datasets-from-one
render loop) is on record in `potpori.md`, *Detector training (synthetic
labeling loop)* — this doc records the implementation decisions and
bites. Consulted before marker, capture, or training work; update it
when a decision lands.

## Decided

- **Marker pose source is the CAD assembly.** A meshless marker coord
  (a shapeless/vertex-only product) is authored in FreeCAD at the
  physical marker location, parented inside the moving part's subtree
  so it tracks that part's motion. Placement error is label error —
  the CAD coord is the most precise authoring instrument available and
  survives every reimport for free. The coord's product placement
  (position **and** rotation) is the marker pose; the rotation carries
  the surface normal the marker quad faces.
- **The pipeline must carry face-less labels as geometry-free nodes.**
  Today `mesh_for_label` registers a mesh entry even when the label's
  shape has zero faces, and `walk_label` assigns it unconditionally
  (`crates/occt-sys/cpp/step_mesh.cpp:260`): a shapeless product would
  import as a node with an *empty mesh* rather than a `NO_MESH` node —
  an empty `MeshFilter` in Unity and a candidate empty-mesh geom in the
  MJCF exporter. The fix: face-less shapes leave the node at
  `GREYHOUND_NO_MESH`, which is already the documented sentinel for
  geometry-free nodes (`greyhound_abi.h:10`, `crates/mesh/src/node.rs:13`).
  Unity already handles `NO_MESH` correctly — no `MeshFilter`,
  `MeshRenderer`, or `StepGeomProperties` is attached
  (`package/com.greyhound.step/Editor/StepImporter.cs:131`), so a coord
  node arrives as a pure transform.
- **Marker manager is scene-local intent, keyed like the joint SOs.**
  A `MarkerManager : MonoBehaviour` at the scene root holds
  `MarkerDef[]` entries; each entry binds a marker to its coord node by
  `StepSiblingPath` (the reimport-safe identity of the annotation asset
  family, `package/com.greyhound.step/Runtime/StepSiblingPath.cs:18`).
  Direct references are used only for the imported **instance root** —
  that is a scene-level identity which survives reimport; everything
  below the root goes through sibling paths, because a direct ref into
  the imported hierarchy silently slides onto whatever occupies the
  position after a structural CAD change, and a misbound marker poisons
  ground-truth labels silently.
- **Marker quads are derived objects, not authored scene content.**
  The manager spawns textured 50 mm quads (0.05 m — scale is baked to
  meters) at the coord poses and re-composes their poses from the bound
  node every frame (`LateUpdate`), so the quads track the moving parts
  with zero structural parenting into the imported hierarchy. Quads are
  plain primitives, not Decal Projectors: no URP Decal Renderer Feature
  requirement, no decal shader stack, identical behavior in headless
  render-to-texture capture, and the quad transform itself is the
  ground-truth anchor. Quad texture is URP/Lit (not Unlit) so the
  light randomizers affect it like the physical printed sticker;
  textures are PNGs converted from the printed marker designs (Unity
  has no native SVG support).
- **Ground-truth boxes come from the quad corners.** The four corners
  (±25 mm in quad local XY) projected through the capture camera give
  pixel-exact boxes for free — no labeling step, no Perception
  bounding-box labeler needed for correctness. The capture camera stays
  in a small jitter band around its nominal pose in v1: the vision
  mapping (pixel angle = θ, tag centroid = position — no solve between
  pixels and state) assumes a near-fixed camera
  (`potpori.md`, *Product thesis*, background item).
- **Repo split.** This repo owns the pipeline change (shim) only. The
  marker manager, capture scene, and dataset writer live in the Unity
  project of the robot repo (the Unity project moves there per the
  repo-split decision: this repo stays the authoring toolchain, the
  robot repo holds the Unity project with the package as a UPM
  dependency). The training harness placement is an open decision.

## Bites

- **B1 — shim: face-less labels become `NO_MESH` nodes** (this repo,
  worktree loop). `mesh_for_label` counts faces on the label's shape
  (cheap `TopExp_Explorer` walk) and signals "no mesh" instead of
  registering an empty mesh; `walk_label` leaves the node at
  `GREYHOUND_NO_MESH` in both the reference-to-simple and simple-label
  branches. Validation asset: a crafted vertex-only STEP in `assets/`
  (a single part plus a vertex-only product, so one file exercises
  both node kinds). Update `occt-to-unity.md`'s scene-forest facts for
  the new node kind semantics. Then verify the C# MJCF exporter's
  behavior on a `NO_MESH` leaf node (does its validation problem list
  flag it; does it emit a static, massless, geom-less body — MuJoCo
  accepts that for non-moving bodies): exporter fixes, if needed, are
  dist-first package bites.
- **B2 — add marker coords to the FreeCAD assembly** (user, FreeCAD).
  Author the coords at the physical marker positions under the moving
  subtrees, with placement rotation carrying the surface normal; adopt
  a greppable naming convention (e.g. `MKR-*`) so the manager can
  auto-discover coord candidates by name prefix. Confirm what the chosen
  coord object exports as — datums typically do not survive STEP
  export; a vertex-only product does. Reimport in Unity and check the
  coord nodes arrive as `NO_MESH` transforms.
- **B3 — MarkerManager + quad spawning** (robot repo Unity project).
  `MarkerManager` (spawn/clear/follow, marker texture and size fields),
  a minimal custom editor: per entry a temp `ObjectField` pick
  converted to a `StepSiblingPath` on change (the `PathIndices` trick),
  resolved-name display, and a name check on resolve that flags
  renames and surfaces structural changes for re-picking — the deferred
  resolution design in `StepSiblingPath.cs:12`, done here from day one
  because a silent misbinding poisons labels. Facing convention (which
  coord-local axis the quad's visible face aligns to) is pinned here.
- **B4 — capture scene: camera, randomizers, GT projection, dataset
  writer** (robot repo Unity project). Camera rig at the nominal pose
  with the v1 jitter band; randomizers per the `potpori.md` envelope
  (lighting/exposure, viewing angle, backgrounds, hard-band
  oversampling); per-frame GT box projection from quad corners; dataset
  written beside the captures. First experiment: `potpori.md` open
  item 7 — 4 tags at nominal distance, 640-wide, measure ID accuracy.
- **B5 — training harness** (placement open). Dataset format and
  detector are open decisions (working default: COCO JSON +
  ultralytics YOLO); the real-image fine-tune set comes after the
  synthetic loop works.

## Open decisions

- Dataset format + detector (COCO + YOLO11 is the working default;
  Perception's SOCO export is the alternative if the capture loop
  adopts Perception randomizers).
- Perception package vs hand-rolled capture loop (randomizers are the
  only Perception feature in play; GT projection is custom either way).
- Training harness home (robot repo vs a separate training repo vs
  `tools/` in this repo).
- Marker texture assets: the printed designs as PNGs — source files
  and where they live.
- Motion blur for static captures (post-processing motion blur needs
  motion; static frames may fake it or defer it to rendered-loop
  episodes).

## Open verification items

1. What the FreeCAD marker coord exports as in STEP (vertex-only
   product vs skipped vs a shape with a face) — user empirical check
   in FreeCAD.
2. MJCF exporter behavior on `NO_MESH` leaf nodes — verified during B1.
3. Unity-side behavior of a zero-face mesh becomes moot after B1 (no
   mesh entry is created at all); confirm no code path still assumes
   `mesh != NO_MESH` implies a non-empty mesh.

## Related docs

- `potpori.md` — *Detector training (synthetic labeling loop)*:
  rationale, randomization design, three-datasets-from-one-run,
  real-image fine-tune set, open item 7.
- `occt-to-unity.md` — scene forest facts and the gap table.
- `unity-mujoco.md` / `mujoco-schema.md` — MJCF exporter facts the
  `NO_MESH` exporter check leans on.
