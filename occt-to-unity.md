# OCCT → Unity mesh transformation reference

The transformation pipeline from OCCT tessellation to a Unity `Mesh`, and the
gaps between what exists today and what the importer needs. Companion to:

- `occt-mesh.md` — OCCT mesh API facts (read from the 8.0.1 headers)
- `unity-mesh.md` — Unity `Mesh` API facts (Unity 6.0 docs)

Both transformations sections below rest on conventions verified in those two
docs; this document adds the derivation and the concrete rules. Current-state
claims are checked against the actual code (`step_mesh.cpp`, `step_doc.rs`,
`native_api.rs`). Written 2026-10-07.

## Pipeline overview

```
STEP file
  │  STEPControl_Reader (mm, per session unit)
  ▼
TopoDS_Shape (BRep: faces/edges, locations)
  │  BRepMesh_IncrementalMesh (deflection, angle)      [shim]
  ▼
Poly_Triangulation per TopoDS_Face
  │  extract + normalize per face:                     [shim]
  │    • transform nodes by TopLoc_Location
  │    • fix TopAbs_REVERSED winding
  │    • 1-based → 0-based indices
  │    • per-node normals: location transform,
  │      TopAbs_REVERSED negation
  ▼
Greyhound core mesh model (Rust)                      [crates/mesh]
  │  neutral: flat f32 positions + u32 indices,
  │  per-face ranges, normals; OCCT coords (mm),
  │  all triangles outward-CCW in OCCT algebra
  ▼
Unity projection (Rust)                               [crates/unity-projection]
  │  • axis permutation (Z-up RH → Y-up LH)
  │  • winding flip (det −1 consequence)
  │  • uniform scale (mm → m, import setting)
  │  • submesh grouping + interleaved vertex layout
  ▼
Greyhound host ABI (Rust cdylib)                      [crates/importer-host]
  │  flat C ABI over occt-sys + the projection;
  │  two-phase counts/fill into C#-pinned buffers
  ▼
C# blit → UnityEngine.Mesh                            [to be built]
     SetVertexBufferParams → SetVertexBufferData →
     SetIndexBufferParams → SetIndexBufferData →
     SetSubMeshes → RecalculateBounds
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
  (`step_mesh.cpp:84`), so nothing changes for the shim. Changing the scale
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
  swaps two indices for them (`step_mesh.cpp:152`), so the core model is
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
  normals with the full trsf (`step_mesh.cpp:131-147`) but does not flip
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
  casts positions, `step_mesh.cpp:133-135`). Same limit Unity itself has; the
  f64→f32 loss is the importer's only precision loss (gap G10 for huge
  models).

## Indices and submeshes

- OCCT gives one `Poly_Triangulation` per face with 1-based node indices; the
  shim already concatenates faces into one global 0-based `u32` buffer
  (`step_mesh.cpp:119-164`). Faces share **no** vertices (each face owns its
  node array; UV-seam nodes are duplicated by design), so concatenation is
  lossless.
- Unity consumes one shared index buffer + `SubMeshDescriptor`s
  (`indexStart`, `indexCount`, `topology = Triangles`, `baseVertex`,
  `firstVertex`, `vertexCount`). With global indices we set `baseVertex = 0`
  and give each submesh its contiguous `indexStart`/`indexCount` slice.
- `IndexFormat.UInt32` is mandatory: CAD tessellation regularly exceeds the
  16-bit limit (65,535 vertices; and the max index value itself is unusable
  on some GPUs). All counts in the Rust model/ABI are already `u32`.
- Grouping (open decision, gap G5): Unity assigns materials per submesh, so
  per-face grouping (one submesh per OCCT face) means one material slot per
  face — unmanageable for complex parts. Recommended: one submesh per solid
  for v1; keep per-face ranges in the core model so any grouping stays a
  projection-time choice.

## Bounds

- OCCT: shape-level bbox from `greyhound_step_info` (`GreyBbox`, min/max as
  f64, OCCT coords — `grey_box.rs`).
- Unity: transform corners by P ∘ scale. Because P is an axis permutation and
  the scale is positive, min/max map min/max — no corner enumeration needed:
  `Unity.min = (min.x, min.z, min.y)·s`, same for max.
- Set `mesh.bounds` directly instead of `RecalculateBounds()` (skips a Unity
  side scan); per-submesh bounds are auto-computed by `SetSubMesh` unless
  `DontRecalculateBounds` is passed (`unity-mesh.md`).

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
- Buffer bridging from Rust (open decision, gap G11): `SetVertexBufferData`
  has no `IntPtr` overload — the source must be `T[]`, `List<T>` or
  `NativeArray<T>`. Simplest correct path: declare the P/Invoke fill functions
  with `float[]`/`uint[]` parameters; the marshaler pins the blittable array
  for the call, and Rust writes directly into it (no extra copy). Revisit
  `NativeArray`/Burst paths if import time ever matters.

## Gaps

Status: **shim** = exists in C++ shim today; **planned** = agreed next step;
**open** = needs a decision; **deferred** = later, recorded here.

| # | Area | Status | Note |
|---|---|---|---|
| G1 | Normal extraction | done | Shim emits per-vertex unit normals (ABI v3): `BRepLib_ToolTriangulatedShape::ComputeNormals()` when `HasNormals()` is false, location transform, `TopAbs_REVERSED` negation — matching OCCT's own exporters (`RWMesh_FaceIterator::NormalTransformed` reverses the same way; stored normals follow surface-natural orientation, not the face flag). Empirically verified on `rod-clamp-16mm.stp`: 0/5580 vertex normals oppose their triangle winding with the negation, 4572/5580 without (`step_mesh.cpp:141-147`) |
| G2 | UVs | decided | Omit `TexCoord0` for v1 (24 B `[pos][normal]` layout). OCCT UVs are surface *parameters* (arbitrary ranges, per-face space, seam-duplicated), not normalized texture coords, and CAD STEP has no textures to map. Decided 2026-10-07; the shim never reads `UVNode`, the mesh model keeps its `uvs` field for a future texturing pass |
| G3 | Core mesh model + per-face ranges | done | `crates/mesh` model validated at the ABI boundary; shim reports per-face counts (`greyhound_mesh_counts`/`greyhound_mesh_fill`, ABI v2); two-phase tuple return retired |
| G4 | Unity projection | done | `crates/unity-projection` consumes the core model: permutes positions and normals (`(x, z, y)`), bakes the scale parameter into positions, swaps two indices per triangle, emits one submesh per face with `firstVertex`/`vertexCount` from the face ranges, maps the OCCT bbox to Unity bounds; all output types are `repr(C)` for the future C# blit (gap G11). Scalar loops only (`perf.md`) |
| G5 | Submesh grouping | open | Per-face / per-solid / single. Recommend per-solid for v1 (material slots scale). The projection groups per-face today (identity with the core model's ranges); per-solid needs face→solid attribution from the shim — a future ABI addition |
| G6 | Unit scale policy | open | Mechanism decided: bake into vertices, GameObject (1,1,1). The projection takes the factor as `ProjectionSettings` (default 0.001) and `step-stats --unity` uses the default; still open: default factor (0.001 vs 1.0) and import-setting configurability |
| G7 | Vertex welding | deferred | Edge nodes are duplicated across faces; welding by (position, normal) pairs could cut memory but is unnecessary for correctness. Unity `Optimize*` methods are a cheaper post-step |
| G8 | Assembly/instance hierarchy | deferred | `STEPControl_Reader.OneShape()` bakes everything into one compound with locations applied. Unity children-per-instance mapping needs the XCAF reader (`TKDESTEP` has it) — later |
| G9 | Negative-scale locations | deferred | Defensive `det(trsf) < 0` winding flip; theoretical for STEP (see Winding section) |
| G10 | f32 precision for huge models | deferred | Re-origination (subtract pivot before f32, restore via GameObject position) if parts far from origin show jitter |
| G11 | C# buffer bridging | open | The host ABI (`crates/importer-host`) takes raw pointers sized by counts — the pinned `T[]` P/Invoke shape. Remaining: the C# marshaling form (pinned `T[]` recommended vs `NativeArray` copy), decided when the Unity package exists |
| G12 | Progress/cancel, threading | deferred | `BRepMesh` supports `Message_ProgressRange`; unused today. Large assemblies tessellate for seconds |

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

## References

- `occt-mesh.md`, `unity-mesh.md` — the two source docs for both sides.
- `crates/occt-sys/cpp/step_mesh.cpp` — current shim (extraction state).
- `crates/occt-sys/src/step_doc.rs`, `native_api.rs`, `grey_box.rs` —
  current Rust ABI surface (two-phase mesh, f64 bbox).
- Unity Manual *Mesh index data* (winding order):
  <https://docs.unity3d.com/6000.0/Documentation/Manual/mesh-index-data.html>
- Unity Manual *Mesh data* index:
  <https://docs.unity3d.com/6000.0/Documentation/Manual/AnatomyofaMesh.html>
