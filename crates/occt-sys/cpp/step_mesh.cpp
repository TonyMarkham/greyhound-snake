#include <STEPControl_Reader.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <BRepBndLib.hxx>
#include <Bnd_Box.hxx>
#include <Poly_Triangulation.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopLoc_Location.hxx>
#include <gp_Pnt.hxx>
#include "native_guard.h"
#include <cmath>
#include <limits>
#include <memory>
#include <utility>

struct GreyDoc {
  TopoDS_Shape shape;
};

extern "C" {

extern "C" void* greyhound_step_open(const char* path) noexcept {
  return greyhound::guard(static_cast<void*>(nullptr), [&]() -> void* {
    if (!path || path[0] == '\0') {
      greyhound::set_error("empty STEP path");
      return nullptr;
    }
    STEPControl_Reader reader;
    if (reader.ReadFile(path) != IFSelect_RetDone || reader.TransferRoots() <= 0) {
      greyhound::set_error("STEP read or root transfer failed");
      return nullptr;
    }
    auto doc = std::make_unique<GreyDoc>(GreyDoc{reader.OneShape()});
    if (doc->shape.IsNull()) {
      greyhound::set_error("STEP transfer produced no shape");
      return nullptr;
    }
    return doc.release();
  });
}

extern "C" int32_t greyhound_step_info(
    void* doc_in, int32_t* solids, int32_t* faces, int32_t* edges,
    GreyBbox* bbox) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !solids || !faces || !edges || !bbox) {
      greyhound::set_error("invalid STEP info handle or output pointer");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
    *solids = *faces = *edges = 0;
    for (TopExp_Explorer e(doc->shape, TopAbs_SOLID); e.More(); e.Next()) ++*solids;
    for (TopExp_Explorer e(doc->shape, TopAbs_FACE); e.More(); e.Next()) ++*faces;
    for (TopExp_Explorer e(doc->shape, TopAbs_EDGE); e.More(); e.Next()) ++*edges;
    Bnd_Box box;
    BRepBndLib::Add(doc->shape, box);
    box.Get(bbox->min[0], bbox->min[1], bbox->min[2],
            bbox->max[0], bbox->max[1], bbox->max[2]);
    return 0;
  });
}

extern "C" void greyhound_step_close(void* doc_in) noexcept {
  (void)greyhound::guard<int32_t>(1, [&]() -> int32_t {
    delete static_cast<GreyDoc*>(doc_in);
    return 0;
  });
}

extern "C" int32_t greyhound_mesh_counts(
    void* doc_in, double deflection, double angle_rad,
    uint32_t* nverts, uint32_t* nindices) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !nverts || !nindices || !std::isfinite(deflection) ||
        !std::isfinite(angle_rad) || deflection <= 0 || angle_rad <= 0) {
      greyhound::set_error("invalid tessellation arguments");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
    BRepMesh_IncrementalMesh mesh(doc->shape, deflection, false, angle_rad, true);
    uint64_t nv = 0, ni = 0;
    for (TopExp_Explorer e(doc->shape, TopAbs_FACE); e.More(); e.Next()) {
      TopLoc_Location loc;
      const Handle(Poly_Triangulation)& tri =
          BRep_Tool::Triangulation(TopoDS::Face(e.Current()), loc);
      if (tri.IsNull()) continue;
      nv += static_cast<uint64_t>(tri->NbNodes());
      ni += 3ULL * static_cast<uint64_t>(tri->NbTriangles());
      if (nv > std::numeric_limits<uint32_t>::max() ||
          ni > std::numeric_limits<uint32_t>::max()) {
        greyhound::set_error("mesh exceeds the C ABI count range");
        return 1;
      }
    }
    *nverts = static_cast<uint32_t>(nv);
    *nindices = static_cast<uint32_t>(ni);
    return 0;
  });
}

extern "C" int32_t greyhound_mesh_fill(
    void* doc_in, float* verts, uint32_t* indices) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !verts || !indices) {
      greyhound::set_error("invalid mesh handle or output pointer");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
    uint32_t base = 0;
    for (TopExp_Explorer e(doc->shape, TopAbs_FACE); e.More(); e.Next()) {
      TopLoc_Location loc;
      const Handle(Poly_Triangulation)& tri =
          BRep_Tool::Triangulation(TopoDS::Face(e.Current()), loc);
      if (tri.IsNull()) continue;
      const gp_Trsf& trsf = loc.Transformation();
      for (int i = 1; i <= tri->NbNodes(); ++i) {
        gp_Pnt p = tri->Node(i).Transformed(trsf);
        *verts++ = static_cast<float>(p.X());
        *verts++ = static_cast<float>(p.Y());
        *verts++ = static_cast<float>(p.Z());
      }
      const bool reversed = e.Current().Orientation() == TopAbs_REVERSED;
      for (int i = 1; i <= tri->NbTriangles(); ++i) {
        int n1, n2, n3;
        tri->Triangle(i).Get(n1, n2, n3);
        if (reversed) std::swap(n2, n3);
        *indices++ = base + static_cast<uint32_t>(n1 - 1);
        *indices++ = base + static_cast<uint32_t>(n2 - 1);
        *indices++ = base + static_cast<uint32_t>(n3 - 1);
      }
      base += static_cast<uint32_t>(tri->NbNodes());
    }
    return 0;
  });
}

} // extern "C"