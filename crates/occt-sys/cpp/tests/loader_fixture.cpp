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
  return 2;
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
extern "C" int32_t greyhound_mesh_counts(
    void* doc, double, double, uint32_t* nverts, uint32_t* nindices,
    uint32_t* nfaces) noexcept {
  if (!doc || !nverts || !nindices || !nfaces) return 1;
  *nverts = *nindices = 3;
  *nfaces = 1;
  return 0;
}
#ifndef FIXTURE_MISSING_FILL
extern "C" int32_t greyhound_mesh_fill(
    void* doc, float* verts, uint32_t* indices, uint32_t* face_counts) noexcept {
  if (!doc || !verts || !indices || !face_counts) return 1;
  const float triangle[9] = {0, 0, 0, 1, 0, 0, 0, 1, 0};
  for (int i = 0; i < 9; ++i) verts[i] = triangle[i];
  indices[0] = 0; indices[1] = 1; indices[2] = 2;
  face_counts[0] = 3;
  face_counts[1] = 3;
  return 0;
}
#endif
extern "C" void greyhound_step_close(void* doc) noexcept {
  delete static_cast<FixtureDoc*>(doc);
  event("close");
}