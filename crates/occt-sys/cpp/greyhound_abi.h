#pragma once
#include <stdint.h>

#define GREYHOUND_API __attribute__((visibility("default")))

// Sentinel solid index for faces that belong to no solid (open shells,
// sheet bodies, loose faces); all such faces merge into one group.
#define GREYHOUND_NO_SOLID UINT32_MAX
// Sentinel node fields: GREYHOUND_NO_PARENT marks root nodes of the scene
// forest; GREYHOUND_NO_MESH marks nodes that carry no geometry (assembly
// grouping nodes).
#define GREYHOUND_NO_MESH UINT32_MAX
#define GREYHOUND_NO_PARENT UINT32_MAX

struct GreyBbox {
  double min[3];
  double max[3];
};

extern "C" {
GREYHOUND_API uint32_t greyhound_abi_version() noexcept;
GREYHOUND_API const char* greyhound_last_error() noexcept;
GREYHOUND_API int32_t greyhound_box_volume(double dx, double dy, double dz, double* volume) noexcept;
GREYHOUND_API void* greyhound_step_open(const char* path) noexcept;
GREYHOUND_API int32_t greyhound_step_info(void* doc, int32_t* solids, int32_t* faces, int32_t* edges,GreyBbox* bbox) noexcept;
// Scene-level queries. nnodes counts all nodes of the assembly forest in
// depth-first pre-order (parents precede children); nmeshes counts unique
// meshed shapes; ncolors counts the global color palette entries;
// nname_bytes counts the total UTF-8 name bytes (no terminators).
GREYHOUND_API int32_t greyhound_scene_counts(void* doc, uint32_t* nnodes, uint32_t* nmeshes, uint32_t* ncolors, uint32_t* nname_bytes) noexcept;
// nodes holds 4 * nnodes uint32_t values per node, in walk order:
// [parent_node_index (GREYHOUND_NO_PARENT for roots), mesh_index
// (GREYHOUND_NO_MESH for geometry-free assembly nodes), name_offset
// (bytes into the names blob), name_length (bytes, no terminator)].
// transforms holds 12 * nnodes float values per node: the local transform
// as a row-major 3x4 matrix [a11 a12 a13 a14 a21 ... a34] mapping mesh-local
// OCCT points into the parent node's frame (a14/a24/a34 = translation).
// names holds nname_bytes UTF-8 bytes: node names concatenated in walk
// order, each addressed by name_offset/name_length; pass NULL when
// nname_bytes is 0.
GREYHOUND_API int32_t greyhound_scene_fill(void* doc, uint32_t* nodes, float* transforms, char* names) noexcept;
// colors holds 4 * ncolors float values: RGBA components in sRGB
// (alpha 1.0), one entry per color index. Color indices are global across
// all meshes of the document.
GREYHOUND_API int32_t greyhound_color_fill(void* doc, float* colors) noexcept;
// Per-mesh tessellation queries; mesh must be < the nmeshes reported by
// greyhound_scene_counts. deflection and angle_rad bound the tessellation
// as in BRepMesh_IncrementalMesh. Returns this mesh's totals: nverts and
// nindices accumulate faces in the walk order (faces without triangulation
// are skipped); nfaces counts the walked faces.
GREYHOUND_API int32_t greyhound_mesh_counts(void* doc, uint32_t mesh, double deflection, double angle_rad, uint32_t* nverts, uint32_t* nindices, uint32_t* nfaces) noexcept;
// out holds 17 double values for the mesh's exact BRep mass properties,
// computed by BRepGProp::VolumeProperties on the mesh's own shape (local
// frame, OCCT units):
//   [0]    volume in mm3 (the density-1 integral; mass = volume * density)
//   [1..3] centre of gravity in the mesh's local frame (mm)
//   [4..12] the three principal axes of inertia as unit direction vectors,
//          row-major (axes[0..2] = first axis, ...), local frame
//   [13..15] the principal moments matched to those axes (mm5 at density
//          1; inertia = moment * density)
//   [16]   the density the STEP file carries for this shape's material
//          (0.0 when the file carries none; units are the file's context)
// default_density is unused by the shim today (kept for ABI stability);
// consumers apply file_density > 0 ? file_density : default_density.
GREYHOUND_API int32_t greyhound_mesh_properties(void* doc, uint32_t mesh, double default_density, double* out) noexcept;
// Buffers must match the last successful mesh_counts call for this mesh.
// face_counts holds 2 * nfaces uint32_t values: [vertex_count, index_count]
// pairs, in the same face walk order as the vertex and index buffers.
// normals holds 3 * nverts float values: one unit normal per vertex, in the
// same walk order, already oriented to match the emitted winding.
// face_attribs holds 2 * nfaces uint32_t values: [solid_index, color_index]
// pairs in the same walk order; solid_index is local to this mesh and
// GREYHOUND_NO_SOLID for the merged non-solid group; color_index is global
// (see greyhound_color_fill).
GREYHOUND_API int32_t greyhound_mesh_fill(void* doc, uint32_t mesh, float* verts, float* normals, uint32_t* indices, uint32_t* face_counts, uint32_t* face_attribs) noexcept;
GREYHOUND_API void greyhound_step_close(void* doc) noexcept;
}

namespace greyhound {
void clear_error() noexcept;
void set_error(const char* message) noexcept;
}
