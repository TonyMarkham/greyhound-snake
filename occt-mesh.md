# OCCT Mesh API summary

Reference for the Open CASCADE mesh data model and tessellation API, focused
on what the STEP importer needs: run BRepMesh over a shape, then pull
triangle soup out of `Poly_Triangulation` into our Rust mesh model.

Facts below are read **directly from the OCCT 8.0.1 headers installed in this
repo** (`dist/occt/x86_64-linux/include/opencascade/`, built from the `V8.0.1`
tag) — not from web documentation. Retrieved 2026-10-07. Header paths are
given as `[Header]` references throughout.

Big picture: OCCT has **no mesh file format of its own**. The in-memory model
is `Poly_Triangulation` (per-face triangulation) attached into the shape's
topological data, produced by `BRepMesh_IncrementalMesh`, and read back via
`BRep_Tool::Triangulation`. Persistence is only through embedding in
`.brep`/BinBRep or XCAF (BinXCAF) documents; interchange uses third-party
formats via writers that consume the same in-memory model.

## `Poly_Triangulation` — the data model `[Poly_Triangulation.hxx]`

A `Standard_Transient` (reference-counted via `occ::handle<Poly_Triangulation>`;
the old `Handle(Poly_Triangulation)` macro still compiles) holding:

| Member | Meaning |
|---|---|
| `myNodes` (`Poly_ArrayOfNodes`) | 3D node positions; float *or* double storage (see below) |
| `myTriangles` (`NCollection_Array1<Poly_Triangle>`) | triangles, each a triplet of node indices |
| `myUVNodes` (`Poly_ArrayOfUVNodes`, optional) | (u, v) parameter-space point per node, parallel to `myNodes` |
| `myNormals` (`NCollection_Array1<NCollection_Vec3<float>>`, optional) | per-node surface normal, **always float**, parallel to `myNodes` |
| `myDeflection` | max distance from true surface to the approximation (model units) |
| `myParams` (`Poly_TriangulationParameters`, optional) | provenance: deflection/angle/minsize the mesh was built for |
| `myPurpose` (`Poly_MeshPurpose`) | `Calculation`, `Presentation`, `Active`, `Loaded`, ... |
| cached min/max `Bnd_Box` | optional cached bounds |

Key API:

- Counts: `NbNodes()`, `NbTriangles()`; presence checks `HasUVNodes()`,
  `HasNormals()`, `HasGeometry()`.
- **1-based indexing everywhere**: `Node(i)`, `UVNode(i)`, `Normal(i)` valid
  for `i ∈ [1, NbNodes()]`; `Triangle(i)` for `i ∈ [1, NbTriangles()]`.
  `Poly_Triangle::Get(n1, n2, n3)` returns node indices in `[1, NbNodes()]`.
  Our shim's `n - 1` subtraction when filling 0-based Unity/Rust buffers is
  mandatory, not stylistic.
- `ComputeNormals()` — computes smooth per-node normals by averaging adjacent
  triangle normals (in place).
- Bulk access: `MapNodeArray()`, `MapTriangleArray()`, `MapUVNodeArray()`,
  `MapNormalArray()` return read-only wrapped arrays; the non-virtual
  `InternalNodes()` / `InternalUVNodes()` / `InternalNormals()` /
  `InternalTriangles()` expose the raw NCollection arrays ("should be used
  instead in portable code" applies to them, but they are the zero-copy path
  our C++ shim can use for fast buffer fills).
- Precision: `IsDoublePrecision()` — node positions (3D and UV) may be stored
  as float or double; set at allocation time via `SetDoublePrecision()` before
  data is allocated (mesher output is double-precision by default). Normals
  are always `float`.
- Deferred data (only relevant when meshes were loaded from files, not when
  produced by BRepMesh): `NbDeferredNodes()` / `LoadDeferredData()` /
  `UnloadDeferredData()` support out-of-core triangulations.
- Multiple triangulations per face are supported in 8.0 (LODs / purposes); a
  deprecated `Poly_ListOfTriangulation` header confirms lists of them.

## Accessing the mesh of a shape `[BRep_Tool.hxx]`

- `BRep_Tool::Triangulation(theFace, theLocation, theMeshPurpose = NONE)`
  → `const occ::handle<Poly_Triangulation>&` — null if the face has no
  triangulation. The **location is returned separately**: node positions are
  in face-local coordinates and must be transformed by
  `theLocation.Transformation()` (`gp_Trsf`) to get shape-global coordinates.
  This is what `step_mesh.cpp:131-137` does. Note: `gp_Trsf` may in principle
  carry scale; transform positions with the full trsf, and transform normals
  with the vectorial part and re-normalize (`gp_Dir::Transform` does this) —
  our shim extracts normals this way since ABI v3 (`step_mesh.cpp:141-147`).
- `BRep_Tool::Triangulations(face, loc)` → all triangulations of the face.
- Edge approximations (not needed for triangle extraction, part of the family):
  - `PolygonOnTriangulation(edge, tri, loc)` — edge as a polyline of **node
    indices into that face's triangulation** (seam/closure duplicates the
    closure node; an edge on a seam has two such polygons, selected by an
    `Index` overload / detected by `BRep_Tool::IsClosed`).
  - `Polygon3D(edge, loc)` — standalone 3D polyline (for edges without face
    triangulation).
- `BRepTools::Triangulation(shape, ...)` and
  `Load/Unload/ActivateTriangulation(...)` — shape-level helpers for the
  deferred/multiple-triangulation machinery `[BRepTools.hxx]`.

## Tessellating: `BRepMesh_IncrementalMesh` `[BRepMesh_IncrementalMesh.hxx]`

- **"Incremental" means reuse**: faces whose existing triangulation satisfies
  the requested parameters are kept (status flag `Reused`); outdated or
  missing ones are recomputed (`Outdated`, `ReMesh`). To force a fresh mesh,
  remove existing triangulations first with `BRepTools::Clean(shape)`.
- Convenience constructor (what our shim uses at `step_mesh.cpp:84`):
  `BRepMesh_IncrementalMesh(shape, linDeflection, isRelative=false,
  angDeflection=0.5, isInParallel=false)` — **runs `Perform()` automatically**.
- Full constructor takes `IMeshTools_Parameters` (below).
- Meshing results are **attached into the shape's topology**, not returned —
  extraction afterwards is always `TopExp_Explorer(face)` +
  `BRep_Tool::Triangulation`.
- `initParameters()` **throws** `Standard_NumericError` if
  `Deflection < Precision::Confusion()` or `Angle < Precision::Angular()` —
  validate in the shim before calling (we already do).
- Status flags: `GetStatusFlags()` returns an `IMeshData_Status` bitmask:
  `NoError`, `OpenWire`, `SelfIntersectingWire`, `Failure`, `ReMesh`,
  `UnorientedWire`, `TooFewPoints`, `Outdated`, `Reused`, `UserBreak`
  `[IMeshData_Status.hxx]`.
- Progress reporting via `Message_ProgressRange`.

## `IMeshTools_Parameters` `[IMeshTools_Parameters.hxx]`

Defaults in parentheses. The derived defaults are filled in by
`BRepMesh_IncrementalMesh::initParameters()`, not by the struct itself.

| Parameter | Default | Meaning |
|---|---|---|
| `Deflection` | 0.001 | linear (chordal) deflection for **boundary edges**; drives everything else; model units (mm for STEP) |
| `Angle` | 0.5 | angular deflection (rad) for boundary edge tessellation |
| `DeflectionInterior` | −1 → `Deflection` | linear deflection for face interiors |
| `AngleInterior` | −1 → `2 × Angle` | angular deflection for face interiors |
| `MinSize` | −1 → `0.1 × min(Deflection, DeflectionInterior)` | minimum mesh edge length; guards against degenerate triangles on distorted surfaces |
| `Relative` | false | if true, per-edge deflection = `Deflection × edge size`, and face deflection = max of its edges' |
| `InParallel` | false | multi-threaded meshing (shape must not be shared meanwhile) |
| `InternalVerticesMode` | true | insert interior vertices, not just edge/surface boundary |
| `ControlSurfaceDeflection` | true | verify face-interior deviation against deflection |
| `EnableControlSurfaceDeflectionAllSurfaces` | false | extend that check to analytical surfaces |
| `CleanModel` | true | free the internal data model when done |
| `AdjustMinSize` | false | locally adjust MinSize to edge size |
| `ForceFaceDeflection` | false | use shape tolerances for face deflection |
| `AllowQualityDecrease` | false | allow re-meshing to produce a worse mesh than existing |
| `MeshAlgo` | `DEFAULT` | 2D Delaunay triangulation implementation selector |

Practical reads for tuning:

- `Deflection` is the single most important knob: absolute chordal tolerance
  between the true surface and the triangle edges. Model units matter — a
  0.001 default is tuned for mm-scale parts.
- `Relative=true` scales deflection per-edge/per-face by size — good for
  assemblies with widely varying part scales, but less predictable output.
- `Angle` controls how finely curved *edges* are segmented even when the
  deflection tolerance would allow longer segments.

## Conventions that affect our importer

1. **1-based node indexing** → shim subtracts 1 (`step_mesh.cpp:154-156`).
2. **Face-local coordinates + `TopLoc_Location`** → shim transforms nodes and
   normals (`step_mesh.cpp:131-147`).
3. **Winding / orientation** `[Poly_Triangulation.hxx, TopoDS docs]`:
   triangle node order of a FORWARD face is counter-clockwise when viewed
   from outside the material; faces with `TopAbs_REVERSED` orientation are
   stored with inverted orientation, so the shim swaps two indices
   (`step_mesh.cpp:152`). OCCT is right-handed, Z-up; the handedness flip
   for a host (Unity: left-handed Y-up, clockwise front faces) happens in the
   Unity projection layer, not here.
4. **No vertex sharing between faces**: each face owns its own
   `Poly_Triangulation` with its own node array. Seam edges on closed
   surfaces deliberately get duplicated nodes (UV seam). Whole-shape
   concatenation (as in our shim) is therefore natural; deduping shared edge
   nodes is an optional post-step, and conflicts with per-face UV seams.
5. **Per-node normals** `[BRepLib_ToolTriangulatedShape.cxx, verified in the
   V8.0.1 source]`: BRepMesh itself does **not** populate normals (no
   `SetNormal` call in the mesher); consumers do it lazily.
   `BRepLib_ToolTriangulatedShape::ComputeNormals(face, tris)` fills
   surface-aware per-node normals when UV nodes exist (analytic
   `GeomLib::NormEstim`, flat-averaged fallback), does nothing if normals
   already exist. Crucially, both it and the winding-averaged
   `Poly_Triangulation::ComputeNormals()` produce normals in the **surface's
   natural orientation, ignoring the face orientation flag** — OCCT's own
   mesh exporters therefore reverse normals for `TopAbs_REVERSED` faces
   (`RWMesh_FaceIterator::NormalTransformed` negates exactly like its
   `TriangleOriented` swaps winding). Our shim does the same
   (`step_mesh.cpp:141-147`). Verified empirically on `rod-clamp-16mm.stp`:
   with the negation 0 of 5580 vertex normals oppose their triangle's
   winding; without it 4572 of 5580 do.
6. **`float` vs `double`**: node positions are typically double; we convert to
   `f32` when filling the Rust mesh model (standard for GPU meshes; CAD
   coordinates can exceed f32 precision for large models — a per-part
   origin/translation strategy may be needed later, same as Unity's own
   float precision guidance). Normals are always `float` in OCCT and
   converted unchanged.

## Mesh interchange in this build (writers/readers present)

Verified from headers + installed libs (`dist/occt/x86_64-linux/`):

| Format | Toolkit | API |
|---|---|---|
| STL | `TKDESTL` | `RWStl` (read/write, binary+ascii), `StlAPI` |
| OBJ | `TKDEOBJ` | `RWObj` (read/write, `RWObj_CafWriter` for XCAF scenes) |
| PLY | `TKDEPLY` | `RWPly_CafWriter` (write) |
| VRML | `TKDEVRML` | `VrmlData` (read/write) |
| glTF | — | **absent in this 8.0.1 build** (no `RWGltf` headers, no glTF toolkit) |

All of these consume/produce `Poly_Triangulation` (or XCAF documents of them),
which confirms `Poly_Triangulation` as the mesh data model. The earlier
suggestion of glTF as a future file-interchange format needs revisiting for
this build: OBJ/PLY are the available neighbors (Blender and Unity both read
both). glTF support existed in the OCCT 7.x series; if we ever want it, check
a later 8.x release before relying on it.

## Other members of the Poly family (for later)

- `Poly_Connect` — adjacency queries over a `Poly_Triangulation` (triangle
  neighbors, boundary loops).
- `Poly_CoherentTriangulation` — editable triangulation with links/nodes for
  constructing/optimizing meshes.
- `MeshVS` — visualization framework over arbitrary meshes (not needed).
- `Poly_Polygon2D` — 2D polyline (parametric-space edge approximation).
- `Poly_TriangulationParameters` — provenance triple
  (deflection, angle, minsize) per triangulation; useful to detect "mesh was
  built with other settings" before reuse.

## References

Primary source for this summary — actual headers in this repo's OCCT 8.0.1
install:

- `dist/occt/x86_64-linux/include/opencascade/Poly_Triangulation.hxx`
- `dist/occt/x86_64-linux/include/opencascade/Poly_Triangle.hxx`
- `dist/occt/x86_64-linux/include/opencascade/Poly_Polygon3D.hxx`
- `dist/occt/x86_64-linux/include/opencascade/Poly_PolygonOnTriangulation.hxx`
- `dist/occt/x86_64-linux/include/opencascade/Poly_MeshPurpose.hxx`
- `dist/occt/x86_64-linux/include/opencascade/Poly_TriangulationParameters.hxx`
- `dist/occt/x86_64-linux/include/opencascade/BRep_Tool.hxx`
- `dist/occt/x86_64-linux/include/opencascade/BRepTools.hxx`
- `dist/occt/x86_64-linux/include/opencascade/BRepMesh_IncrementalMesh.hxx`
- `dist/occt/x86_64-linux/include/opencascade/IMeshTools_Parameters.hxx`
- `dist/occt/x86_64-linux/include/opencascade/IMeshData_Status.hxx`

Upstream:

- Repo at the release tag used here:
  <https://github.com/Open-Cascade-SAS/OCCT/tree/V8.0.1>
