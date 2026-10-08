#include <STEPCAFControl_Reader.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <XCAFApp_Application.hxx>
#include <TDocStd_Document.hxx>
#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>
#include <XCAFPrs.hxx>
#include <XCAFPrs_Style.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <BRepBndLib.hxx>
#include <Bnd_Box.hxx>
#include <Poly_Triangulation.hxx>
#include <BRepLib_ToolTriangulatedShape.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopLoc_Location.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <NCollection_IndexedDataMap.hxx>
#include <NCollection_Sequence.hxx>
#include <Quantity_Color.hxx>
#include <Quantity_ColorRGBA.hxx>
#include <gp_Pnt.hxx>
#include <gp_Dir.hxx>
#include "native_guard.h"
#include <cmath>
#include <limits>
#include <memory>
#include <utility>
#include <vector>

namespace {

// sRGB fallback for faces with no color anywhere in the file.
constexpr double kFallbackColor[3] = {0.72, 0.72, 0.72};

// One face occurrence in the walk order shared by counts and fill.
struct FacePlanEntry {
  TopoDS_Face face;
  uint32_t solid_index;
  uint32_t color_index;
};

} // namespace

struct GreyDoc {
  TopoDS_Shape shape;
  // Located face -> resolved color index; lookup fails for faces with no
  // color anywhere, which then receive the fallback color.
  NCollection_IndexedDataMap<TopoDS_Shape, int, TopTools_ShapeMapHasher> face_color_indices;
  std::vector<Quantity_ColorRGBA> color_palette;
  std::vector<FacePlanEntry> plan;
};

namespace {

// Resolves the effective color of every located face under each free-shape
// label, mirroring OCCT's own mesh exporters (RWMesh_ShapeIterator::
// dispatchStyles): styles are collected per root label with XCAFPrs::
// CollectStyleSettings (assembly/instance/SHUO overrides and cumulative
// locations applied), then expanded onto the located faces top-down with
// first-wins precedence so per-face styles beat whole-part styles.
void resolve_face_colors(
    const TDF_Label& label,
    NCollection_IndexedDataMap<TopoDS_Shape, Quantity_ColorRGBA, TopTools_ShapeMapHasher>& target) {
  NCollection_IndexedDataMap<TopoDS_Shape, XCAFPrs_Style, TopTools_ShapeMapHasher> styles;
  XCAFPrs::CollectStyleSettings(label, TopLoc_Location(), styles);
  for (NCollection_IndexedDataMap<TopoDS_Shape, XCAFPrs_Style, TopTools_ShapeMapHasher>::Iterator
         it(styles);
       it.More();
       it.Next()) {
    const TopoDS_Shape& key = it.Key();
    const XCAFPrs_Style& style = it.Value();
    if (!style.IsSetColorSurf()) {
      continue;
    }
    const Quantity_ColorRGBA& color = style.GetColorSurfRGBA();
    if (key.ShapeType() == TopAbs_FACE) {
      if (!target.Contains(key)) {
        target.Add(key, color);
      }
      continue;
    }
    for (TopExp_Explorer fe(key, TopAbs_FACE); fe.More(); fe.Next()) {
      if (!target.Contains(fe.Current())) {
        target.Add(fe.Current(), color);
      }
    }
  }
}

uint32_t color_index_for(GreyDoc& doc, const Quantity_ColorRGBA& color) {
  for (size_t i = 0; i < doc.color_palette.size(); ++i) {
    const Quantity_Color& existing = doc.color_palette[i].GetRGB();
    if (existing.Red() == color.GetRGB().Red() &&
        existing.Green() == color.GetRGB().Green() &&
        existing.Blue() == color.GetRGB().Blue() &&
        doc.color_palette[i].Alpha() == color.Alpha()) {
      return static_cast<uint32_t>(i);
    }
  }
  doc.color_palette.push_back(color);
  return static_cast<uint32_t>(doc.color_palette.size() - 1);
}

Quantity_ColorRGBA fallback_color() {
  return Quantity_ColorRGBA(Quantity_Color(
    kFallbackColor[0], kFallbackColor[1], kFallbackColor[2], Quantity_TOC_sRGB));
}

// Solid-ordered walk shared by counts and fill: every TopAbs_SOLID
// occurrence contributes its faces in order (occurrence semantics of
// TopExp_Explorer, so shared-face reuse across solids stays per-occurrence);
// all remaining faces not under any solid - open shells, sheet bodies,
// loose faces - merge into one trailing group with GREYHOUND_NO_SOLID.
void build_face_plan(GreyDoc& doc) {
  doc.plan.clear();
  TopoDS_Shape shape = doc.shape;
  uint32_t solid_index = 0;
  for (TopExp_Explorer s(shape, TopAbs_SOLID); s.More(); s.Next()) {
    const uint32_t current_solid = solid_index++;
    for (TopExp_Explorer f(s.Current(), TopAbs_FACE); f.More(); f.Next()) {
      const TopoDS_Face face = TopoDS::Face(f.Current());
      const int found = doc.face_color_indices.FindIndex(face);
      const int color_index = found > 0 ? doc.face_color_indices.FindFromIndex(found) : -1;
      doc.plan.push_back(FacePlanEntry{
        face, current_solid,
        color_index < 0 ? color_index_for(doc, fallback_color())
                        : static_cast<uint32_t>(color_index)});
    }
  }
  const uint32_t no_solid = GREYHOUND_NO_SOLID;
  for (TopExp_Explorer f(shape, TopAbs_FACE, TopAbs_SOLID); f.More(); f.Next()) {
    const TopoDS_Face face = TopoDS::Face(f.Current());
    const int found = doc.face_color_indices.FindIndex(face);
    const int color_index = found > 0 ? doc.face_color_indices.FindFromIndex(found) : -1;
    doc.plan.push_back(FacePlanEntry{
      face, no_solid,
      color_index < 0 ? color_index_for(doc, fallback_color())
                      : static_cast<uint32_t>(color_index)});
  }
}

} // namespace

extern "C" {

extern "C" void* greyhound_step_open(const char* path) noexcept {
  return greyhound::guard(static_cast<void*>(nullptr), [&]() -> void* {
    if (!path || path[0] == '\0') {
      greyhound::set_error("empty STEP path");
      return nullptr;
    }
    occ::handle<TDocStd_Document> xdoc;
    XCAFApp_Application::GetApplication()->NewDocument("MDTV-XCAF", xdoc);
    STEPCAFControl_Reader reader;
    reader.SetColorMode(true);
    if (reader.ReadFile(path) != IFSelect_RetDone || !reader.Transfer(xdoc)) {
      XCAFApp_Application::GetApplication()->Close(xdoc);
      greyhound::set_error("STEP read or root transfer failed");
      return nullptr;
    }
    const occ::handle<XCAFDoc_ShapeTool> shape_tool =
      XCAFDoc_DocumentTool::ShapeTool(xdoc->Main());
    NCollection_Sequence<TDF_Label> free_shapes;
    shape_tool->GetFreeShapes(free_shapes);
    if (free_shapes.IsEmpty()) {
      XCAFApp_Application::GetApplication()->Close(xdoc);
      greyhound::set_error("STEP transfer produced no shape");
      return nullptr;
    }
    TopoDS_Shape shape = XCAFDoc_ShapeTool::GetOneShape(free_shapes);
    if (shape.IsNull()) {
      XCAFApp_Application::GetApplication()->Close(xdoc);
      greyhound::set_error("STEP transfer produced no shape");
      return nullptr;
    }
    NCollection_IndexedDataMap<TopoDS_Shape, Quantity_ColorRGBA, TopTools_ShapeMapHasher>
      face_colors;
    for (NCollection_Sequence<TDF_Label>::Iterator it(free_shapes); it.More(); it.Next()) {
      resolve_face_colors(it.Value(), face_colors);
    }
    // Everything the document holds has been resolved into the shape and the
    // face-color map; close the document immediately.
    XCAFApp_Application::GetApplication()->Close(xdoc);
    GreyDoc* doc = new GreyDoc();
    doc->shape = shape;
    for (NCollection_IndexedDataMap<TopoDS_Shape, Quantity_ColorRGBA,
                                    TopTools_ShapeMapHasher>::Iterator it(face_colors);
         it.More();
         it.Next()) {
      const int index = color_index_for(*doc, it.Value());
      doc->face_color_indices.Add(it.Key(), index);
    }
    return doc;
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
    uint32_t* nverts, uint32_t* nindices, uint32_t* nfaces, uint32_t* ncolors) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !nverts || !nindices || !nfaces || !ncolors || !std::isfinite(deflection) ||
        !std::isfinite(angle_rad) || deflection <= 0 || angle_rad <= 0) {
      greyhound::set_error("invalid tessellation arguments");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
    build_face_plan(*doc);
    BRepMesh_IncrementalMesh mesh(doc->shape, deflection, false, angle_rad, true);
    uint64_t nv = 0, ni = 0;
    for (const FacePlanEntry& entry : doc->plan) {
      TopLoc_Location loc;
      const Handle(Poly_Triangulation)& tri =
          BRep_Tool::Triangulation(entry.face, loc);
      if (tri.IsNull()) continue;
      nv += static_cast<uint64_t>(tri->NbNodes());
      ni += 3ULL * static_cast<uint64_t>(tri->NbTriangles());
      if (nv > std::numeric_limits<uint32_t>::max() ||
          ni > std::numeric_limits<uint32_t>::max() ||
          doc->plan.size() > std::numeric_limits<uint32_t>::max() ||
          doc->color_palette.size() > std::numeric_limits<uint32_t>::max()) {
        greyhound::set_error("mesh exceeds the C ABI count range");
        return 1;
      }
    }
    *nverts = static_cast<uint32_t>(nv);
    *nindices = static_cast<uint32_t>(ni);
    *nfaces = static_cast<uint32_t>(doc->plan.size());
    *ncolors = static_cast<uint32_t>(doc->color_palette.size());
    return 0;
  });
}

extern "C" int32_t greyhound_mesh_fill(
    void* doc_in, float* verts, float* normals, uint32_t* indices,
    uint32_t* face_counts, uint32_t* face_attribs, float* colors) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !verts || !normals || !indices || !face_counts || !face_attribs || !colors) {
      greyhound::set_error("invalid mesh handle or output pointer");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
    uint32_t base = 0;
    for (size_t entry_index = 0; entry_index < doc->plan.size(); ++entry_index) {
      const FacePlanEntry& entry = doc->plan[entry_index];
      TopLoc_Location loc;
      const TopoDS_Face face = entry.face;
      const Handle(Poly_Triangulation)& tri = BRep_Tool::Triangulation(face, loc);
      if (tri.IsNull()) continue;
      BRepLib_ToolTriangulatedShape::ComputeNormals(face, tri);
      if (!tri->HasNormals()) {
        greyhound::set_error("face triangulation has no normals");
        return 1;
      }
      const gp_Trsf& trsf = loc.Transformation();
      const bool reversed = face.Orientation() == TopAbs_REVERSED;
      for (int i = 1; i <= tri->NbNodes(); ++i) {
        gp_Pnt p = tri->Node(i).Transformed(trsf);
        *verts++ = static_cast<float>(p.X());
        *verts++ = static_cast<float>(p.Y());
        *verts++ = static_cast<float>(p.Z());
        // Stored normals follow the surface's natural orientation, not the
        // face orientation flag; mirror what OCCT's own mesh exporters do
        // (RWMesh_FaceIterator::NormalTransformed): transform by the location,
        // then reverse for REVERSED faces so normals match the emitted
        // (orientation-corrected) winding.
        gp_Dir n = tri->Normal(i);
        if (!loc.IsIdentity()) n.Transform(trsf);
        if (reversed) n.Reverse();
        *normals++ = static_cast<float>(n.X());
        *normals++ = static_cast<float>(n.Y());
        *normals++ = static_cast<float>(n.Z());
      }
      uint32_t tri_count = 0;
      for (int i = 1; i <= tri->NbTriangles(); ++i) {
        int n1, n2, n3;
        tri->Triangle(i).Get(n1, n2, n3);
        if (reversed) std::swap(n2, n3);
        *indices++ = base + static_cast<uint32_t>(n1 - 1);
        *indices++ = base + static_cast<uint32_t>(n2 - 1);
        *indices++ = base + static_cast<uint32_t>(n3 - 1);
        ++tri_count;
      }
      face_counts[2 * entry_index] = static_cast<uint32_t>(tri->NbNodes());
      face_counts[2 * entry_index + 1] = 3U * tri_count;
      face_attribs[2 * entry_index] = entry.solid_index;
      face_attribs[2 * entry_index + 1] = entry.color_index;
      base += static_cast<uint32_t>(tri->NbNodes());
    }
    for (const Quantity_ColorRGBA& color : doc->color_palette) {
      // STEP colors decode as sRGB (STEPConstruct_Styles::DecodeColor uses
      // Quantity_TOC_sRGB); hand the sRGB components to the host and let the
      // renderer apply its own sRGB-to-linear conversion.
      double r = 0.0, g = 0.0, b = 0.0;
      color.GetRGB().Values(r, g, b, Quantity_TOC_sRGB);
      *colors++ = static_cast<float>(r);
      *colors++ = static_cast<float>(g);
      *colors++ = static_cast<float>(b);
      *colors++ = static_cast<float>(color.Alpha());
    }
    return 0;
  });
}

} // extern "C"