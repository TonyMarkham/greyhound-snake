#pragma once
#include <stdint.h>

#define GREYHOUND_API __attribute__((visibility("default")))

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
GREYHOUND_API int32_t greyhound_mesh_counts(void* doc, double deflection, double angle_rad,uint32_t* nverts, uint32_t* nindices, uint32_t* nfaces) noexcept;
// Buffers must match the last successful counts call for this document.
// face_counts holds 2 * nfaces uint32_t values: [vertex_count, index_count]
// pairs, in the same face walk order as the vertex and index buffers.
// normals holds 3 * nverts float values: one unit normal per vertex, in the
// same walk order, already oriented to match the emitted winding.
GREYHOUND_API int32_t greyhound_mesh_fill(void* doc, float* verts, float* normals, uint32_t* indices, uint32_t* face_counts) noexcept;
GREYHOUND_API void greyhound_step_close(void* doc) noexcept;
}

namespace greyhound {
void clear_error() noexcept;
void set_error(const char* message) noexcept;
}