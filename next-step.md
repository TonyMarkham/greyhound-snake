# Next steps: submesh semantics + colors/materials

Handoff notes from the first-import review (2026-10-08, 04:37). The
milestone stands: `rod-clamp-16mm.stp` imported through the full pipeline
(1744 verts / 5580 indices / 26 submeshes / 1860 tris, correct bounds,
clean console). Review found two gaps.

**First action next session: commit the importer bite** (suggested:
`Add the STEP Scripted Importer`). Uncommitted as of writing: `AGENTS.md`,
`justfile`, `occt-to-unity.md`, new `package/com.greyhound.step/{Runtime,
Editor}/`, `step-importer.md`. Note: `../greyhound-snake-csharp` worktree is
stale (predates the public-visibility, `AssetImporters`-using, and
`NoAutoStaticsCleanup` fixes); main is the source of truth — discard or
re-sync it at the start of the next bite.

## Issue 1 — submeshes are per-face artifacts, not semantic meshes

Current state: the shim flattens `TopExp_Explorer(shape, TopAbs_FACE)` and
the projection emits one submesh per face (gap G4 design). The 26 submeshes
of the rod clamp are 26 BRep faces, all sharing one material.

Goal: **one mesh (or submesh) per manifold solid**.

Verified facts:
- OCCT has no single `IsManifold(shape)` API. Nearest stock:
  `BRepCheck_Analyzer::IsValid` (full validity, expensive) and
  `ShapeAnalysis_Shell` free-edge check (`HasFreeEdges`) — every edge
  shared by exactly 2 faces = closed/watertight shell. That cheap test is
  the practical manifold guard.
- A valid `TopAbs_SOLID` is manifold by construction, and the BRep
  hierarchy already groups faces per solid — the checker is only needed as
  a policy filter for non-solid leftovers, not on the happy path. This is
  exactly gap **G5** (face→solid attribution).

To investigate (against the installed headers, `dist/package/…/occt/
x86_64/include/opencascade/` — read, don't guess):
1. Solid walk semantics: `TopExp_Explorer(solid, TopAbs_FACE)` vs shared
   faces between solids (can a face occur in two solids after STEP
   import? dedup or occurrence-count semantics of the explorers).
2. Policy for non-solid geometry: free/open shells, sheet bodies —
   isolate per shell as their own submesh? merge? reject with a warning?
3. Where solid/color attribution lives: shim output gains per-face
   (solid_index, color_index); core mesh model carries it; the projection
   concatenates per-face ranges into per-solid submeshes. Decide exact
   struct/ABI shape (per-submesh color index table → ABI v4, version bump).

## Issue 2 — no colors, no materials

Current state: the C# importer creates a bare `Mesh` asset with no
material; the preview's colors are Unity's default shading, not the file's.

Verified facts:
- The asset **does** carry colors: `assets/rod-clamp-16mm.stp` contains
  styled items with `COLOUR_RGB(0.976, 0.678, 0.122)` (orange). Exact
  inventory of styled items / color definitions still to be taken
  (the first grep combined four entity patterns; 1 unique COLOUR_RGB
  found).
- The shim reads via plain `STEPControl_Reader` (step_mesh.cpp:32), which
  discards all style/color attributes.
- Colors require the XCAF path: `STEPCAFControl_Reader` → `XCAFDoc`
  document → `XCAFDoc_ColorTool`. Verified earlier: the XCAF toolkits
  (`TKXCAF`, `TKCAF`, `TKLCAF`, `TKVCAF`…) are **already** in the runtime
  dependency closure — no new OCCT libraries, but `build.rs`'s link list
  and the shim's `DT_NEEDED` need the XCAF/CAF toolkit names added
  explicitly.

To investigate:
1. `STEPCAFControl_Reader.hxx`: transfer flow (ReadFile → Transfer(doc) →
   one label per root shape; FreeShapes; label → shape via
   `XCAFDoc_ShapeTool::GetShape_s`).
2. Color lookup rules: `XCAFDoc_ColorTool::GetColor_s(label, type)`
   overloads and `XCAFDoc_ColorType` (Gen/Surf/Curv); how colors inherit
   face ← shape ← assembly when attached at different levels; what a face
   with no color anywhere should get (fallback color — decide).
3. sRGB vs linear: STEP `COLOUR_RGB` components are (assumed) sRGB; Unity
   `Color` + URP Lit in linear color space — pick the conversion once
   (likely `Color(r, g, b)` treated as sRGB → linear via
   `UnityEngine.Color` constructor semantics) and note it.
4. C# material strategy: `Shader.Find("Universal Render Pipeline/Lit")`
   (URP project confirmed), one material per distinct color, added as
   asset sub-assets; imported object becomes a **GameObject root** with
   `MeshFilter` + `MeshRenderer` so materials attach (a bare Mesh cannot
   hold materials). Main object switches from Mesh → GameObject.
5. Submesh granularity decision: per-solid vs per-(solid,color) — a solid
   with per-face colors forces per-(solid,color) submeshes; decide after
   seeing how the asset's colors attach.

## Proposed bite shape (after the checklist)

shim: solid walk + XCAF colors (ABI → v4) → core model: per-face
solid/color indices → projection: per-(solid,color) submeshes + color
table → C#: GameObject root, MeshFilter/MeshRenderer, URP Lit materials
per color. G5 closes; a new gap row for colors (G13?) opens and closes in
the same bite; G8 (assembly/instance hierarchy) stays open.
