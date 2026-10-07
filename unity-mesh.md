# Unity Mesh API summary

Reference for the Unity `UnityEngine.Mesh` API, focused on the data-oriented
("advanced") surface we plan to use for the STEP importer's Unity projection.
Facts below are from the **Unity 6.0 (6000.0)** scripting API docs, retrieved
2026-10-07. Links to the exact pages are at the end.

Scope note: this documents what Unity *expects on its side*, so the Rust
"Unity projection" layer can emit buffers C# can use without any rearranging.

## Two API families

The `Mesh` class has two sets of methods for assigning data:

- **Simple**: `SetVertices`, `SetNormals`, `SetUVs`, `SetTriangles`,
  `SetIndices`, `SetColors`, `SetTangents`, ... Per-attribute arrays with
  validation. Convenient, but does format conversion and extra copies, and
  implies C#-side assembly of data.
- **Advanced (data-oriented)**: `SetVertexBufferParams`,
  `SetVertexBufferData`, `SetIndexBufferParams`, `SetIndexBufferData`,
  `SetSubMesh`/`SetSubMeshes`, plus `MeshUpdateFlags` to control validation.
  Operates on raw vertex/index buffers and submesh descriptors; little or no
  validation; fastest path. **This is the family we target** — it lets C# be a
  pure blit: our Rust projection emits exactly the bytes Unity wants.

Validation difference matters: with the advanced API Unity will not check for
out-of-bounds indices or bad submesh ranges for us (unless we leave the
relevant `MeshUpdateFlags` off). The Rust/C# boundary must guarantee valid
data.

## Required call order

Building a mesh from scratch with the advanced API:

1. `SetVertexBufferParams(vertexCount, attributes...)` — allocates the vertex
   buffer and declares the layout.
2. `SetVertexBufferData(data, dataStart, meshBufferStart, count, stream, flags)`
   — copies vertex bytes.
3. `SetIndexBufferParams(indexCount, format)` — allocates the index buffer.
   **Resets `subMeshCount` to 1 and leaves the index data uninitialized** when
   the size/format changes.
4. `SetIndexBufferData(data, dataStart, meshBufferStart, count, flags)` —
   copies index bytes.
5. `mesh.subMeshCount = n; mesh.SetSubMesh(i, desc, flags)` — or
   `SetSubMeshes(descriptors)` to define all submeshes at once.
6. `mesh.RecalculateBounds()` — recomputes the *whole-mesh* bounds. (Submesh
   bounds are computed by `SetSubMesh` automatically unless
   `MeshUpdateFlags.DontRecalculateBounds` is passed.)

## Vertex layout: `VertexAttributeDescriptor`

Declared per attribute as `(attribute, format, dimension, stream)`.

- `attribute`: semantic slot — `Position`, `Normal`, `Tangent`, `Color`,
  `TexCoord0`..`TexCoord7`, `BlendWeight`, `BlendIndices`.
- `format`: `VertexAttributeFormat` — `Float32`, `Float16`, `UNorm8`,
  `SNorm16`, etc.
- `dimension`: component count.
- `stream`: 0–3. Each stream becomes a separate GPU vertex buffer. Most uses
  are a single stream (stream 0).

Rules and conventions:

- Within a stream, attributes are ordered
  `Position, Normal, Tangent, Color, TexCoord0..7, BlendWeight,
  BlendIndices` regardless of declaration order.
- **Attribute size must be a multiple of 4 bytes.** e.g. `Float16` with
  dimension 3 is invalid; `Float16` dimension 2 (4 bytes) is valid.
- Format support can be checked with `SystemInfo.SupportsVertexAttributeFormat`.
- If `BlendWeight`/`BlendIndices` are present, Unity's default 3-stream
  layout should be used so skinned rendering does not reorder attributes.
- A C# struct mirroring the layout must use
  `[StructLayout(LayoutKind.Sequential)]` so marshalled copy matches exactly.

Planned importer layout (single stream, 32 bytes/vertex):

```csharp
new VertexAttributeDescriptor(VertexAttribute.Position, VertexAttributeFormat.Float32, 3), // 12
new VertexAttributeDescriptor(VertexAttribute.Normal,   VertexAttributeFormat.Float32, 3), // 12
new VertexAttributeDescriptor(VertexAttribute.TexCoord0,VertexAttributeFormat.Float32, 2), // 8
```

```csharp
[StructLayout(LayoutKind.Sequential)]
struct ImportVertex
{
    public Vector3 position; // Unity-space, transformed in Rust
    public Vector3 normal;   // Unity-space, transformed in Rust
    public Vector2 uv;
}
```

Every attribute component is a multiple of 4 bytes, so this is valid.

## Data transfer: `SetVertexBufferData` / `SetIndexBufferData`

Signatures (same shape for both):

```csharp
void SetVertexBufferData(NativeArray<T>/T[]/List<T> data,
                         int dataStart,       // first source element
                         int meshBufferStart, // first destination vertex
                         int count,           // element count
                         int stream,          // 0..3
                         MeshUpdateFlags flags);
```

- `T` must be a blittable struct matching the declared layout byte-for-byte.
  There is **no `IntPtr` overload**; the source must be a managed array,
  `List<T>`, or `NativeArray<T>`. (How we bridge from a Rust-owned raw pointer
  to one of these is a design point for later; simplest correct path is
  `Marshal.Copy`-style transfer into a `T[]`, zero-copy options exist via
  `Unity.Collections` unsafe utilities but are undocumented surface.)
- Partial updates are supported via the start/count parameters.
- The source layout must match `SetVertexBufferParams`/`GetVertexAttributes`
  exactly — no per-attribute conversion happens.

## Index format

`SetIndexBufferParams(indexCount, IndexFormat)`:

- `IndexFormat.UInt16`: up to 65,535 vertices. Default (smaller, faster).
- `IndexFormat.UInt32`: up to ~4 billion vertices. **GPU support is not
  universal** (docs example: Mali-400 Android logs a warning and does not
  render). Desktop targets are fine.
- **Caveat from docs:** the maximum possible index value (e.g. `0xFFFF` for
  UInt16) may not be usable — some APIs/GPUs skip triangles that reference the
  max index. So UInt16 is really "65,534 usable vertex indices" in the worst
  case.

For CAD tessellation (hundreds of thousands of vertices is normal) we plan
**`UInt32` indices end to end**; the Rust mesh model and C ABI already use
`u32`.

## SubMeshes: `SubMeshDescriptor`

A submesh is the unit of material assignment: one submesh renders with one
material. Fields:

| Field | Meaning |
|---|---|
| `indexStart` | offset into the mesh's single shared index buffer |
| `indexCount` | indices for this submesh (3 per triangle) |
| `topology` | `MeshTopology` of this submesh, normally `Triangles` |
| `baseVertex` | value added to every index to reach the real vertex |
| `firstVertex`, `vertexCount` | vertex range referenced by this submesh |
| `bounds` | submesh-local bounding box |

Notes:

- `SetSubMesh` auto-computes `bounds`, `firstVertex`, `vertexCount` unless
  `MeshUpdateFlags.DontRecalculateBounds` is passed. Whole-mesh bounds still
  need an explicit `RecalculateBounds()`.
- Submesh index ranges (`indexStart`/`indexCount`) **must not overlap**.
- Setting `subMeshCount` smaller than before *shrinks* the index buffer to the
  `indexStart` of the first removed submesh — order of operations matters.
- Our mapping: one submesh per face group (ultimately per solid/part), so
  materials can be assigned per part in Unity without per-face splits.

## `MeshUpdateFlags`

Optional flags on the advanced setters; combinable with `|`:

| Flag | Effect |
|---|---|
| `Default` | all default checks/validation |
| `DontValidateIndices` | skip out-of-bounds index checking (`SetIndexBufferData`) |
| `DontRecalculateBounds` | skip submesh bounds computation (`SetSubMesh`) |
| `DontNotifyMeshUsers` | skip notifying Renderers of possible bounds change |
| `DontResetBoneBounds` | skip skinned bone bounds reset (not relevant to us yet) |

Plan: start with `Default` (validation on) for correctness while we shake out
the Rust projection; switch to `DontValidateIndices` (and keep bounds
recalculation) once data is known-good, for speed.

## Topology and winding

- `MeshTopology.Triangles` is an indexed **triangle list**: 3 indices per
  triangle, from the shared index buffer. (Line/point topologies exist;
  strips/quads are legacy and not our concern.)
- Unity is **left-handed, Y-up**, and front faces are **clockwise** when
  viewed from outside. OCCT is right-handed, Z-up with counter-clockwise
  front faces — the Rust Unity projection must handle both the axis mapping
  and the winding flip. (OCCT's own per-face orientation fix for
  `TopAbs_REVERSED` belongs in the C++ shim, before this projection.)

## Other API pieces (for later)

- `RecalculateNormals()` / `RecalculateTangents()`: Unity-side recomputation.
  We expect OCCT to give us per-node normals from `Poly_Triangulation`
  directly, so we should not need `RecalculateNormals`, but it is a fallback.
- `Mesh.isReadable`: meshes created at runtime by script are readable;
  imported assets depend on their import setting. Relevant only if we later
  read Unity meshes back.
- Jobs/Burst path: `AllocateWritableMeshData` → fill `Mesh.MeshData` in jobs →
  `ApplyAndDisposeWritableMeshData`. An alternative fill mechanism to
  `SetVertexBufferData` if we later want parallel tessellation on the C# side.
- `SystemInfo.maxGraphicsBufferSize`: exceeding it throws in
  `SetVertexBufferParams`/`SetIndexBufferParams` — a per-part mesh strategy
  also dodges this.
- `MarkDynamic`, `Optimize*`, `UploadMeshData`: performance controls,
  not needed for a one-shot import.

## References

- Mesh class overview (simple vs advanced API):
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Mesh.html>
- `SetVertexBufferParams`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Mesh.SetVertexBufferParams.html>
- `SetVertexBufferData`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Mesh.SetVertexBufferData.html>
- `VertexAttributeDescriptor`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Rendering.VertexAttributeDescriptor.html>
- `SetIndexBufferParams`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Mesh.SetIndexBufferParams.html>
- `SetIndexBufferData`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Mesh.SetIndexBufferData.html>
- `IndexFormat`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Rendering.IndexFormat.html>
- `SubMeshDescriptor`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Rendering.SubMeshDescriptor.html>
- `SetSubMesh`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Mesh.SetSubMesh.html>
- `MeshUpdateFlags`:
  <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Rendering.MeshUpdateFlags.html>
