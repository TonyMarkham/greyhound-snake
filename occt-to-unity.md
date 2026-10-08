# OCCT → Unity mesh transformation reference

The transformation pipeline from OCCT tessellation to a Unity `Mesh`, and the
gaps between what exists today and what the importer needs. Companion to:

- `occt-mesh.md` — OCCT mesh API facts (read from the 8.0.1 headers)
- `unity-mesh.md` — Unity `Mesh` API facts (Unity 6.0 docs)

Both transformations sections below rest on conventions verified in those two
docs; this document adds the derivation and the concrete rules. Current-state
claims are checked against the actual code (`step_mesh.cpp`, `step_doc.rs`,
`native_api.rs`). Written 2026-10-07; updated 2026-10-08 for the XCAF/colors
bite (solid+color attribution, submesh grouping, G5/G13) and the assembly bite
(scene forest, per-instance nodes, mesh dedup, G8).

## Pipeline overview

```
STEP file
  │  STEPCAFControl_Reader → XCAF document → free shapes (mm, per session unit)
  ▼
TopoDS_Shape (BRep: faces/edges, locations) + per-face colors
  │  BRepMesh_IncrementalMesh (deflection, angle)      [shim]
  ▼
Poly_Triangulation per TopoDS_Face
  │  extract + normalize per face:                     [shim]
  │    • transform nodes by TopLoc_Location
  │    • fix TopAbs_REVERSED winding
  │    • 1-based → 0-based indices
  │    • per-node normals: location transform,
  │      TopAbs_REVERSED negation
  │    • solid-ordered walk: per-face (solid, color)
  │      attribution; non-solid faces merge into one
  │      GREYHOUND_NO_SOLID group
  ▼
Greyhound core mesh model (Rust)                      [crates/mesh]
  │  neutral: flat f32 positions + u32 indices,
  │  per-face ranges, normals, per-face FaceAttrib
  │  (solid, color) + sRGB RGBA color table;
  │  OCCT coords (mm), all triangles outward-CCW
  │  in OCCT algebra
  ▼
XCAF assembly walk → scene forest (per unique shape)  [shim]
  │  nodes in depth-first pre-order: parent, mesh
  │  ref, UTF-8 name, local 3x4 transform (the
  │  instance's own placement); referred simple shapes
  │  mesh once
  │  (dedup across instances); geometry-free assembly
  │  labels become grouping nodes
  ▼
Unity projection (Rust)                               [crates/unity-projection]
  │  • axis permutation (Z-up RH → Y-up LH)
  │  • winding flip (det −1 consequence)
  │  • uniform scale (mm → m, import setting)
  │  • per-(solid, color) submesh grouping
  │    (index-buffer reorder) + color table +
  │    interleaved vertex layout
  │  • per-node transform conjugation: rotation
  │    M·R·M, translation s·M·t (Unity space, scale
  │    baked); names pass through as a UTF-8 blob
  ▼
Greyhound host ABI (Rust cdylib)                      [crates/importer-host]
  │  flat C ABI over occt-sys + the projection;
  │  scene counts/fill + per-mesh two-phase
  │  counts/fill + palette fill into C#-pinned buffers
  ▼
C# blit → GameObject tree, one Mesh asset per         [package/com.greyhound.step]
     unique mesh: SetVertexBufferParams → data →
     SetIndexBufferParams → data → SetSubMeshes →
     bounds; per-node GameObject with TRS decomposed
     from the projected transform; MeshFilter
     references the shared mesh, MeshRenderer assigns
     one URP Lit material per palette color used by
     the mesh's submeshes
```

Layering rule: **OCCT knowledge stays in the shim; host knowledge stays in the
projection.** The core model is neutral (OCCT coordinates, outward-CCW
triangles) so Blender later consumes it without Unity assumptions.

## Coordinate systems and units

| | OCCT | Unity |
|---|---|---|
| Handedness | right-handed | left-handed |
| Up axis | +Z | +Y |
| "Into screen" (front view) | +Y | +Z (camera looks along +Z) |
| Units | millimeters (session unit; key `xstep.cascade.unit`, default MM — see `XSAlgo_ShapeProcessor.hxx:159`; the reader converts the file's unit to it) | unitless; **1 unit = 1 meter** by convention (physics gravity −9.81 along Y, audio/audio-source ranges, etc.) |

### The coordinate transform

```
Unity.x = OCCT.x
Unity.y = OCCT.z
Unity.z = OCCT.y
```

i.e. the axis permutation **P = (x, y, z) → (x, z, y)**, matrix
`[[1,0,0],[0,0,1],[0,1,0]]`, det(**P**) = **−1**.

Why this map, and why det −1 is *required* rather than incidental:

- Up: OCCT +Z must become Unity +Y, so models don't import lying on their
  back. Both "up" directions identified.
- View: OCCT +Y (into the screen of a CAD front view) becomes Unity +Z
  (forward, into the screen for Unity's default camera). The part then faces a
  Unity viewer the same way it faced the CAD user.
- **Chirality:** mapping the *same physical points* between a right-handed and
  a left-handed labeling always has det = −1 (frame-map determinants: RH +1,
  LH −1). Any det = +1 alternative — e.g. the popular `(x, z, −y)` "rotation"
  — silently produces the **mirror image of the part**. For DCC content
  nobody notices; for CAD it is wrong (left/right-hand threads, asymmetric
  brackets, engraved text flip). Our sample `rod-clamp-16mm.stp` will show it
  immediately.

> **Rule: never fix winding or handedness by negating an axis.** A sign flip
> anywhere in the coordinate transform mirrors every part. Handedness
> conversion is done by the permutation above; winding is fixed by swapping
> triangle indices (below), not by coordinates.

### Handedness map across consumers

- **Right-handed, like the core model:** binary STL (CCW-outward right-hand
  rule), MuJoCo (Z-up), Blender, glTF. These consume the core model
  **identity-mapped** — their only transform is the mm→m scale. Baking that
  scale is each projection's own choice (`unity-mujoco.md`).
- **Left-handed: Unity is the only one in our set** (Y-up). The `(x, z, y)`
  reflection plus the two-index winding flip are confined to the Unity
  projection; every other consumer skips both.
- **Data source of truth is domain-partitioned.** Geometry — meshes, part
  structure, exact-BRep mass properties — belongs to the **STEP file**,
  read through the core model: a reimport regenerates it and never carries
  authored edits. Articulation — joints, root mobility — belongs to
  **Unity**: STEP carries no joint semantics (AP242 kinematics is
  essentially never present in real files and unread by XCAF), so the
  `StepJointSet` asset (`unity-mujoco.md`) is the sole authority; it must
  survive reimports, and exporters merge it with the geometry. Neither
  domain edits the other: reimporting refreshes geometry and re-applies
  joints; editing joints never touches geometry.
- **Debugging rule:** STL written from the core model must look identical to
  the STEP in any right-handed viewer (CAD viewer, Blender,
  `mujoco.simulate`). If something appears mirrored, the bug is in the Unity
  projection — never in the data.

Normals transform with the same permutation (`n' = P·n`): it is orthogonal,
so unit length and outward direction are preserved; no renormalization
needed for permutation + uniform positive scale.

### Scale

**Decision:** scale is baked into vertex positions by the Unity projection;
the imported GameObject keeps Unity scale (1, 1, 1).

- Uniform positive scale only. `0.001` (mm → m) is the recommended default so
  Unity physics, lighting and camera defaults behave sanely at (1, 1, 1).
- Keep the core model in native mm — no scale there; tessellation deflection
  is specified in model units (mm) and applied before any scaling
  (`step_mesh.cpp:238`), so nothing changes for the shim. Changing the scale
  factor is a re-projection of the core model (cheap buffer math), never a
  re-tessellation.
- Baking mm → m before the f64→f32 cast also improves f32 relative precision:
  positions shrink before rounding.
- Never non-uniform scale: normals would need the inverse-transpose matrix
  and UV/space density distorts. If a display-size tweak is wanted at runtime,
  do it on the GameObject transform in C#, not in the buffers.

Open decision (gap G6): default factor (0.001) and whether it is
user-configurable as an import setting.

## Winding and orientation

- **OCCT:** front faces are wound so that the right-hand-rule normal
  `(v1−v0)×(v2−v0)` points **outward** (counter-clockwise seen from outside).
  Faces flagged `TopAbs_REVERSED` store inverted order; the shim already
  swaps two indices for them (`step_mesh.cpp:307`), so the core model is
  uniformly outward-CCW in OCCT algebra. The shim's emitted normals are
  negated the same way, so they point outward too and agree with the winding
  (gap G1).
- **Unity:** front faces connect **clockwise** as viewed; Unity derives
  facing from winding order and culls back faces by default
  (Unity Manual *Mesh index data*, "Winding order").

Under the permutation P (det = −1), triangle vertex *order on screen, viewed
from outside the face, is preserved* — but the right-hand-rule algebra flips:

```
(P·a) × (P·b) = det(P) · P·(a × b) = −P·(a × b)
```

So a triangle whose right-hand-rule normal pointed outward in OCCT coords has
its RH normal pointing *inward* in Unity coords — equivalently, it appears
counter-clockwise from outside in Unity — and Unity culls it. **Therefore the
Unity projection must swap two indices of every triangle.** This is the
det −1 consequence of the (correct) non-mirroring coordinate map; it is a
separate, additional step on top of the shim's `TopAbs_REVERSED` fix.

Two notes:

- Vertex **normals are not negated**. They are geometric data (used for
  shading), and after `n' = P·n` they still point physically outward, which
  is what lighting wants. The winding flip only affects facing/culling.
- Edge case — negative-scale `TopLoc_Location`: `gp_Trsf` may carry a
  negative uniform scale (a mirror). The shim transforms positions and
  normals with the full trsf (`step_mesh.cpp:296-298`) but does not flip
  winding or normals for such faces, so a mirrored instance would render
  inside-out. Standard STEP
  placements (`AXIS2_PLACEMENT_3D`) are rotation+translation frames and never
  mirror, so this is theoretical for our reader; add a defensive
  `det(trsf) < 0 → flip` check anyway (gap G9).

## Vertex data mapping

OCCT per-node attributes (`Poly_Triangulation`, see `occt-mesh.md`) vs the
planned Unity vertex struct:

| OCCT | Type | Transform | Unity attribute |
|---|---|---|---|
| `Node(i)` position | f64 (or f32) | `P·p`, then × scale | `Position` Float32 ×3 |
| `Normal(i)` | f32, unit | `P·n` | `Normal` Float32 ×3 |
| `UVNode(i)` | f64 | — | omitted for v1 (G2 decided) |

- Target layout, single stream: interleaved
  `[pos: 12 B][normal: 12 B][uv: 8 B]` = 32 B, or `[pos][normal]` = 24 B if
  UVs are omitted. Both satisfy Unity's every-attribute-multiple-of-4-bytes
  rule; the matching C# struct needs
  `[StructLayout(LayoutKind.Sequential)]` (`unity-mesh.md`).
- Precision: positions/normals converted to f32 at extraction (shim already
  casts positions, `step_mesh.cpp:288-290`). Same limit Unity itself has; the
  f64→f32 loss is the importer's only precision loss (gap G10 for huge
  models).

## Indices and submeshes

- OCCT gives one `Poly_Triangulation` per face with 1-based node indices; the
  shim already concatenates faces into one global 0-based `u32` buffer
  (`step_mesh.cpp` `mesh_fill`). Faces share **no** vertices (each face owns its
  node array; UV-seam nodes are duplicated by design), so concatenation is
  lossless.
- Unity consumes one shared index buffer + `SubMeshDescriptor`s
  (`indexStart`, `indexCount`, `topology = Triangles`, `baseVertex`,
  `firstVertex`, `vertexCount`). With global indices we set `baseVertex = 0`
  and give each submesh its contiguous `indexStart`/`indexCount` slice.
- `IndexFormat.UInt32` is mandatory: CAD tessellation regularly exceeds the
  16-bit limit (65,535 vertices; and the max index value itself is unusable
  on some GPUs). All counts in the Rust model/ABI are already `u32`.
- Grouping (gap G5, done): one submesh per `(solid_index, color_index)` pair,
  in first-appearance order. The shim walks solids first and attributes each
  face occurrence to its solid; faces outside any solid (open shells, sheet
  bodies, loose faces) merge into one trailing group carrying the
  `GREYHOUND_NO_SOLID` sentinel. The projection reorders the index buffer so
  each group owns a contiguous range (Unity submesh descriptors need
  contiguous slices); per-face ranges stay in the core model so any grouping
  remains a projection-time choice.

## Colors

Verified against the OCCT 8.0.1 sources (`STEPCAFControl_Reader.cxx`,
`XCAFPrs.cxx`, `STEPConstruct_Styles.cxx`):

- Reading uses `STEPCAFControl_Reader` (ColorMode on by default) into an
  `XCAFApp_Application` document; free shapes come from
  `XCAFDoc_ShapeTool::GetFreeShapes`. Style/color lookup goes through
  `XCAFPrs::CollectStyleSettings(label, loc, map)` per free-shape label — the
  same function OCCT's own glTF/mesh exporters use.
- `XCAFDoc_ColorTool::GetColor(label, type, …)` is a direct TreeNode lookup
  with **no** upward inheritance. The resolution instead happens in
  `CollectStyleSettings` top-down: recurse referred shapes (cumulative
  locations), components, then the label's subshape labels and the label
  itself; **later entries overwrite earlier ones**, so instance styles beat
  referred-shape styles and SHUO beats instance. Per label, `fillStyleColors`
  applies Gen (sets surf+curv defaults) then Surf/Curv overrides.
- `STEPCAFControl_Reader::ReadColors` additionally propagates assembly-level
  colors down to parts that have none, and applies root styles before leaf
  styles so leaf (overriding) styles win.
- Face colors are resolved by expanding each style key onto its located
  faces (first-wins, mirroring `RWMesh_ShapeIterator::dispatchStyles`).
- **Color space:** `STEPConstruct_Styles::DecodeColor` decodes STEP
  `COLOUR_RGB` as sRGB (`Quantity_TOC_sRGB`, with >1.0 normalization);
  `Quantity_Color` stores linear internally. The shim emits **sRGB** floats
  via `Values(r, g, b, Quantity_TOC_sRGB)`; Unity converts material colors
  sRGB→linear on upload in linear-color-space projects (verify visually in
  the editor once).
- Fallback for faces with no color anywhere: sRGB `0.72` gray.
- Visibility (`XCAFPrs_Style::IsVisible`) is not consumed yet — hidden
  entities still mesh (deferred).
- C# side: one material per distinct submesh color (`Universal Render
  Pipeline/Lit`, `_BaseColor`); the imported object is a GameObject root with
  `MeshFilter` + `MeshRenderer` because a bare `Mesh` cannot hold materials.

## Bounds

- OCCT: shape-level bbox from `greyhound_step_info` (`GreyBbox`, min/max as
  f64, OCCT coords — `grey_box.rs`).
- Unity: transform corners by P ∘ scale. Because P is an axis permutation and
  the scale is positive, min/max map min/max — no corner enumeration needed:
  `Unity.min = (min.x, min.z, min.y)·s`, same for max.
- Set `mesh.bounds` directly instead of `RecalculateBounds()` (skips a Unity
  side scan); per-submesh bounds are auto-computed by `SetSubMesh` unless
  `DontRecalculateBounds` is passed (`unity-mesh.md`).

## Mass properties

- Source: the exact BRep, not the tessellation — `BRepGProp::VolumeProperties`
  gives volume, centre of gravity, and principal axes/moments; mesh-derived
  properties would carry the deflection error into physics.
- Density: `XCAFDoc_MaterialTool::GetDensityForShape(label)` returns the file's
  material density and **0.0 when the file has none** — the shim therefore
  reports density-free raw values and the policy `file > 0 ? file : parameter`
  lives with the consumer (C# default 1.0 g/cm³).
- Shim ABI v6 `greyhound_mesh_properties`: 17 doubles per unique mesh —
  `[0]` volume (mm³), `[1..3]` local COM (mm), `[4..12]` principal axes
  (row-major unit vectors, local frame), `[13..15]` moments **matched to those
  axes** (mm⁵ at density 1), `[16]` file density (0.0 = none). A
  `default_density` parameter is reserved for ABI stability but unused.
- Ordering trap: `GProp_PrincipalProps::Moments()` order versus the
  First/Second/Third axes is **not guaranteed** — the shim resolves each
  axis's moment with `GProp_GProps::MomentOfInertia(gp_Ax1(com, axis))`.
- Properties are computed during the scene walk (same dedup as meshes),
  cached by the host, and served per mesh via host ABI v4
  `greyhound_host_mesh_properties`.
- Consumers derive: `mass = volume × density`, `gyration_i =
  sqrt(moment_i / volume)`, `inertia_i = mass × gyration_i²`. C# converts
  `volume_mm3 × density × 1e-6` → kg (g/cm³ × mm³ is dimensionally mg).
- Projection: COM permutes and scales like a position (`(x, z, y)·s`), axes
  conjugate like a rotation (`R' = M·R·M`), gyration radii are scale-invariant
  in form (`sqrt(moment/volume)` — moment scales as length⁵, volume as
  length³, so gyration scales with the length factor; the projection bakes
  the scale into COM and moments and re-derives gyration).
- Unity surface: `StepMassProperties` component per meshed node (mass in kg,
  body-local COM, `inertiaTensor` + `inertiaTensorRotation` ready for
  `Rigidbody`); the importer fills a `Rigidbody` only when the node has one.

## C# call sequence (planned)

With the v1 layout (positions + normals, 24 B stride, UVs omitted):

```csharp
mesh.SetVertexBufferParams(vertexCount, new []
{
    new VertexAttributeDescriptor(VertexAttribute.Position, VertexAttributeFormat.Float32, 3),
    new VertexAttributeDescriptor(VertexAttribute.Normal,   VertexAttributeFormat.Float32, 3),
});
mesh.SetVertexBufferData(vertices, 0, 0, vertexCount, 0, MeshUpdateFlags.Default);
mesh.SetIndexBufferParams(indexCount, IndexFormat.UInt32);
mesh.SetIndexBufferData(indices, 0, 0, indexCount, MeshUpdateFlags.Default);
mesh.subMeshCount = submeshes.Length;
for (int i = 0; i < submeshes.Length; i++) mesh.SetSubMesh(i, submeshes[i]);
mesh.bounds = bounds; // mapped from GreyBbox
```

- Flags policy: start with `Default` (Unity validates indices/ranges); move to
  `DontValidateIndices` once the projection is trusted.
- Buffer bridging from Rust (gap G11, decided): blittable `UnityVertex[]`,
  `uint[]` and `UnitySubMesh[]` P/Invoke parameters — the marshaler pins the
  arrays for the call and Rust writes directly into them (no extra copy).
  `NativeArray`/Burst paths remain a revisit-if-profiling-matters item.

## Gaps

Status: **shim** = exists in C++ shim today; **planned** = agreed next step;
**open** = needs a decision; **deferred** = later, recorded here.

| # | Area | Status | Note |
|---|---|---|---|
| G1 | Normal extraction | done | Shim emits per-vertex unit normals (ABI v3): `BRepLib_ToolTriangulatedShape::ComputeNormals()` when `HasNormals()` is false, location transform, `TopAbs_REVERSED` negation — matching OCCT's own exporters (`RWMesh_FaceIterator::NormalTransformed` reverses the same way; stored normals follow surface-natural orientation, not the face flag). Empirically verified on `rod-clamp-16mm.stp`: 0/5580 vertex normals oppose their triangle winding with the negation, 4572/5580 without (`step_mesh.cpp:296-298`) |
| G2 | UVs | decided | Omit `TexCoord0` for v1 (24 B `[pos][normal]` layout). OCCT UVs are surface *parameters* (arbitrary ranges, per-face space, seam-duplicated), not normalized texture coords, and CAD STEP has no textures to map. Decided 2026-10-07; the shim never reads `UVNode`, the mesh model keeps its `uvs` field for a future texturing pass |
| G3 | Core mesh model + per-face ranges | done | `crates/mesh` model validated at the ABI boundary; shim reports per-face counts (`greyhound_mesh_counts`/`greyhound_mesh_fill`, ABI v2); two-phase tuple return retired |
| G4 | Unity projection | done | `crates/unity-projection` consumes the core model: permutes positions and normals (`(x, z, y)`), bakes the scale parameter into positions, swaps two indices per triangle, emits one submesh per face with `firstVertex`/`vertexCount` from the face ranges, maps the OCCT bbox to Unity bounds; all output types are `repr(C)` for the future C# blit (gap G11). Scalar loops only (`perf.md`) |
| G5 | Submesh grouping | done | One submesh per `(solid_index, color_index)` in first-appearance order; shim walks `TopExp_Explorer` solids first (occurrence semantics), non-solid faces merge into one `GREYHOUND_NO_SOLID` group (decided 2026-10-08). Shim emits per-face `(solid, color)` attribution (ABI v4); the projection reorders the index buffer so each group is contiguous. Non-solid policy: merge into one group; free-edge shells are not diagnosed yet |
| G6 | Unit scale policy | open | Mechanism decided: bake into vertices, GameObject (1,1,1). The projection takes the factor as `ProjectionSettings` (default 0.001) and `step-stats --unity` uses the default; still open: default factor (0.001 vs 1.0) and import-setting configurability |
| G7 | Vertex welding | deferred | Edge nodes are duplicated across faces; welding by (position, normal) pairs could cut memory but is unnecessary for correctness. Unity `Optimize*` methods are a cheaper post-step |
| G8 | Assembly/instance hierarchy | done | Shim ABI v5 scene walk: `GetFreeShapes` → recursive label walk following reference labels (`GetReferredShape`, `GetLocation`), depth-first pre-order nodes (parent, mesh ref, UTF-8 name from the reference label falling back to the referred label, **local** 3×4 transform = the instance's own placement — consumers compose the hierarchy by parenting; an earlier draft stored accumulated world transforms and double-applied every parent, caught in the editor and now guarded by a nested-composition assertion in `tools/verify-package.py`). Referred simple shapes mesh once — dedup across instances (`NCollection_IndexedMap` of labels); referred assemblies become geometry-free grouping nodes. Projection conjugates node transforms into Unity space (`R' = M·R·M`, `t' = s·M·t`; reflection reverses rotation sense — verified by composition in tests). C# decomposes each transform to TRS (column norms → localScale, normalized columns → localRotation; negative determinant flips `scale.x`, G9's defensive piece) and builds the GameObject tree. SHUO per-instance color overrides stay unresolved: colors resolve per unique referred label, so instances share the part's colors (this file has no SHUO; documented limitation) |
| G9 | Negative-scale locations | deferred | Defensive `det(trsf) < 0` winding flip; theoretical for STEP (see Winding section) |
| G10 | f32 precision for huge models | deferred | Re-origination (subtract pivot before f32, restore via GameObject position) if parts far from origin show jitter |
| G11 | C# buffer bridging | done | Pinned `T[]` P/Invoke: blittable `UnityVertex[]`/`uint[]`/`UnitySubMesh[]`/`uint[]`/`float[]` pin for the duration of `mesh_fill`; `NativeArray` copy rejected for v1. Implemented in `package/com.greyhound.step/Runtime/NativeMethods.cs` |
| G12 | Progress/cancel, threading | deferred | `BRepMesh` supports `Message_ProgressRange`; unused today. Large assemblies tessellate for seconds |
| G13 | Colors + materials | done | XCAF path in the shim (ABI v4): `STEPCAFControl_Reader` + `XCAFPrs::CollectStyleSettings`; per-face sRGB RGBA color table + `(solid, color)` attribution. ABI v5 moved the palette to scene scope (`greyhound_color_fill`); each `Mesh` still carries it for the mesh model's validation. C# builds one URP Lit material per palette entry and assigns per submesh through the mesh's `submesh_colors`. Color space: shim emits sRGB; Unity converts on upload (verify visually). Visibility/`XCAFPrs_Style::IsVisible` not consumed (deferred); SHUO instance colors follow `CollectStyleSettings` but per-instance overrides stay unresolved (G8 note) |
| G14 | Mass properties | done | Exact-BRep mass properties per unique mesh (shim ABI v6): `BRepGProp::VolumeProperties` volume/COM/principal axes, each axis's moment resolved via `MomentOfInertia(gp_Ax1(com, axis))` because `GProp_PrincipalProps::Moments()` order is not guaranteed; density stays out of the shim (`GetDensityForShape` is 0.0 without a file material — the `file > 0 ? file : parameter` policy and the 1.0 g/cm³ default live in C#). Core model type `MeshProperties` in `crates/mesh` (density-free raw values, like Node/Scene); projection conjugates COM/axes into Unity space; host ABI v4 caches and serves per-mesh properties; `StepMassProperties` component fills `Rigidbody` when present |

## Verification checklist (once implemented)

1. **Up axis:** import `assets/rod-clamp-16mm.stp`; the part must stand
   Y-up in Unity; bbox extents must read permuted (`(dx, dz, dy)` × scale).
2. **Scale:** with 0.001, the 16 mm part spans ≈ 0.016 units.
3. **Winding:** render with default backface culling; nothing may appear
   inside-out; walking the camera around shows consistent surfaces.
4. **Chirality:** compare an asymmetric feature (clamp opening) against a CAD
   viewer from two opposite angles — must not be mirrored.
5. **Submeshes:** counts/slices match the per-solid ranges emitted by the
   projection; materials assign per slot.
6. **Hierarchy (G8):** import `assets/cart-asy.step`; the GameObject tree
   mirrors the STEP assembly (root `cart-asy`, `Assembly` and bearing
   sub-assemblies as grouping nodes, one child per instance with its NAUO
   name); the two Pillow Block instances share one Mesh asset; the two
   bearing sub-assemblies land in different places; total vertex count is
   the unique-mesh sum (69,519 at deflection 0.01), not a per-occurrence
   sum. Instance placement comes from the node transforms, not baked
   vertices: select a child and confirm localPosition/localRotation differ
   from identity.
7. **Mass properties (G14):** import `assets/cart-asy.step`; every meshed
   node gets a `StepMassProperties`; the cart part reads volume
   ≈ 53,605.59 mm³, COM (0.0169977, −0.0002281, 0.0629348) m, gyration
   (52.63, 50.33, 20.05) mm; adding a `Rigidbody` fills mass (density 1.0
   g/cm³ → ≈ 0.0536 kg), body-local centre of mass, and the inertia tensor +
   rotation; `inertiaTensorRotation * diag(inertiaTensor) * its inverse`
   must reproduce the principal moments.

## References

- `occt-mesh.md`, `unity-mesh.md` — the two source docs for both sides.
- `crates/occt-sys/cpp/step_mesh.cpp` — current shim (extraction state).
- `crates/occt-sys/src/step_doc.rs`, `native_api.rs`, `scene.rs`,
  `grey_box.rs` — current Rust ABI surface (scene + per-mesh two-phase, f64
  bbox).
- Unity Manual *Mesh index data* (winding order):
  <https://docs.unity3d.com/6000.0/Documentation/Manual/mesh-index-data.html>
- Unity Manual *Mesh data* index:
  <https://docs.unity3d.com/6000.0/Documentation/Manual/AnatomyofaMesh.html>
