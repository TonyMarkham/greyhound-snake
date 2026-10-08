#include "greyhound_abi.h"
#include <cstdio>
#include <new>

namespace {
struct FixtureDoc {};
thread_local char error_text[128]{};
void event(const char* text) noexcept {
  std::fprintf(stderr, "fixture:%s\n", text);
  std::fflush(stderr);
}
}

__attribute__((constructor)) static void loaded() { event("load"); }
__attribute__((destructor)) static void unloaded() { event("unload"); }

extern "C" uint32_t greyhound_abi_version() noexcept {
#ifdef FIXTURE_BAD_ABI
  return 42;
#else
  return 6;
#endif
}
extern "C" const char* greyhound_last_error() noexcept { return error_text; }
extern "C" int32_t greyhound_box_volume(
    double dx, double dy, double dz, double* volume) noexcept {
  if (!volume) return 1;
  *volume = dx * dy * dz;
  return 0;
}
extern "C" void* greyhound_step_open(const char*) noexcept {
  auto* doc = new (std::nothrow) FixtureDoc;
  if (!doc) return nullptr;
#ifdef FIXTURE_OPEN_FAIL
  delete doc;
  std::snprintf(error_text, sizeof(error_text), "%s", "fixture open failure");
  event("partial-cleanup");
  return nullptr;
#else
  event("open");
  return doc;
#endif
}
extern "C" int32_t greyhound_step_info(
    void* doc, int32_t* solids, int32_t* faces, int32_t* edges,
    GreyBbox* bbox) noexcept {
  if (!doc || !solids || !faces || !edges || !bbox) return 1;
  *solids = *faces = *edges = 1;
  *bbox = GreyBbox{{0, 0, 0}, {1, 1, 1}};
  event("info");
  return 0;
}
extern "C" int32_t greyhound_scene_counts(
    void* doc, uint32_t* nnodes, uint32_t* nmeshes, uint32_t* ncolors,
    uint32_t* nname_bytes) noexcept {
  if (!doc || !nnodes || !nmeshes || !ncolors || !nname_bytes) return 1;
  *nnodes = 1;
  *nmeshes = 1;
  *ncolors = 1;
  *nname_bytes = 0;
  return 0;
}
extern "C" int32_t greyhound_scene_fill(
    void* doc, uint32_t* nodes, float* transforms, char* names) noexcept {
  if (!doc || !nodes || !transforms || !names) return 1;
  nodes[0] = 0; nodes[1] = 0; nodes[2] = 0; nodes[3] = 0;
  for (int i = 0; i < 12; ++i) transforms[i] = 0.0f;
  transforms[0] = transforms[5] = transforms[10] = 1.0f;
  return 0;
}
extern "C" int32_t greyhound_color_fill(void* doc, float* colors) noexcept {
  if (!doc || !colors) return 1;
  colors[0] = 0.72f; colors[1] = 0.72f; colors[2] = 0.72f; colors[3] = 1.0f;
  return 0;
}
extern "C" int32_t greyhound_mesh_properties(
    void* doc, uint32_t mesh, double, double* out) noexcept {
  if (!doc || mesh != 0 || !out) return 1;
  // A 2x1x1 box (mm3 volume 2) centred at the origin: principal axes are
  // the world axes; gyration radii follow from the box formulas.
  out[0] = 2.0;
  out[1] = 0.0; out[2] = 0.0; out[3] = 0.0;
  out[4] = 0.0; out[5] = 0.0; out[6] = 1.0;
  out[7] = 1.0; out[8] = 0.0; out[9] = 0.0;
  out[10] = 0.0; out[11] = 1.0; out[12] = 0.0;
  out[13] = 0.4166666666666667;  // mass-1 moment about the first axis
  out[14] = 0.8333333333333334;
  out[15] = 0.8333333333333334;
  out[16] = 0.0;
  return 0;
}
extern "C" int32_t greyhound_mesh_counts(
    void* doc, uint32_t mesh, double, double, uint32_t* nverts,
    uint32_t* nindices, uint32_t* nfaces) noexcept {
  if (!doc || mesh != 0 || !nverts || !nindices || !nfaces) return 1;
  *nverts = *nindices = 3;
  *nfaces = 1;
  return 0;
}
#ifndef FIXTURE_MISSING_FILL
extern "C" int32_t greyhound_mesh_fill(
    void* doc, uint32_t mesh, float* verts, float* normals, uint32_t* indices,
    uint32_t* face_counts, uint32_t* face_attribs) noexcept {
  if (!doc || mesh != 0 || !verts || !normals || !indices || !face_counts ||
      !face_attribs) {
    return 1;
  }
  const float triangle[9] = {0, 0, 0, 1, 0, 0, 0, 1, 0};
  for (int i = 0; i < 9; ++i) verts[i] = triangle[i];
  const float normal[9] = {0, 0, 1, 0, 0, 1, 0, 0, 1};
  for (int i = 0; i < 9; ++i) normals[i] = normal[i];
  indices[0] = 0; indices[1] = 1; indices[2] = 2;
  face_counts[0] = 3;
  face_counts[1] = 3;
  face_attribs[0] = 0;
  face_attribs[1] = 0;
  return 0;
}
#endif
extern "C" void greyhound_step_close(void* doc) noexcept {
  delete static_cast<FixtureDoc*>(doc);
  event("close");
}
