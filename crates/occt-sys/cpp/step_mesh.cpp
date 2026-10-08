#include <STEPCAFControl_Reader.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <XCAFApp_Application.hxx>
#include <TDocStd_Document.hxx>
#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>
#include <XCAFPrs.hxx>
#include <XCAFPrs_Style.hxx>
#include <TDataStd_Name.hxx>
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
#include <NCollection_IndexedMap.hxx>
#include <NCollection_Sequence.hxx>
#include <Quantity_Color.hxx>
#include <Quantity_ColorRGBA.hxx>
#include <gp_Pnt.hxx>
#include <gp_Dir.hxx>
#include "native_guard.h"
#include <cmath>
#include <cstring>
#include <functional>
#include <limits>
#include <string>
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

// One unique meshed shape: a referred simple-shape label tessellated once
// in its own frame; instance placements live on nodes, not here.
struct GreyMesh {
  TopoDS_Shape shape;
  // Located face -> global color palette index; entries exist for faces
  // with a resolved style, lookup fails for faces that then receive the
  // fallback color.
  NCollection_IndexedDataMap<TopoDS_Shape, uint32_t, TopTools_ShapeMapHasher>
    face_color_indices;
  std::vector<FacePlanEntry> plan;
};

// One node of the assembly forest, in depth-first pre-order so parents
// precede children.
struct GreyNode {
  uint32_t parent;
  uint32_t mesh;
  std::string name;
  // Row-major 3x4 affine matrix: a14/a24/a34 carry the translation.
  float transform[12];
};

// Hasher for label identity, for the dedup map of already-meshed labels.
struct LabelHasher {
  size_t operator()(const TDF_Label& label) const noexcept {
    return std::hash<TDF_Label>{}(label);
  }
  bool operator()(const TDF_Label& a, const TDF_Label& b) const noexcept {
    return a == b;
  }
};

struct GreyDoc {
  TopoDS_Shape whole;
  std::vector<GreyMesh> meshes;
  std::vector<GreyNode> nodes;
  std::vector<Quantity_ColorRGBA> color_palette;
  // Referred labels already emitted as meshes, so repeated instances share
  // one geometry payload.
  NCollection_IndexedMap<TDF_Label, LabelHasher> meshed_labels;
};

Quantity_ColorRGBA fallback_color() {
  return Quantity_ColorRGBA(Quantity_Color(
    kFallbackColor[0], kFallbackColor[1], kFallbackColor[2], Quantity_TOC_sRGB));
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

// Resolves the effective color of every located face under the label,
// mirroring OCCT's own mesh exporters (RWMesh_ShapeIterator::
// dispatchStyles): styles are collected with XCAFPrs::CollectStyleSettings
// (SHUO and subshape overrides, later entries overwrite earlier ones), then
// expanded onto the located faces top-down with first-wins precedence so
// per-face styles beat whole-part styles. Per-instance (SHUO) overrides are
// not resolved here: this runs per referred label with no assembly context,
// so instances of one shared mesh carry its colors (gap-table limitation).
void resolve_face_colors(
    GreyDoc& doc,
    const TDF_Label& label,
    NCollection_IndexedDataMap<TopoDS_Shape, uint32_t, TopTools_ShapeMapHasher>& target) {
  NCollection_IndexedDataMap<TopoDS_Shape, XCAFPrs_Style, TopTools_ShapeMapHasher> styles;
  TopoDS_Shape shape = XCAFDoc_ShapeTool::GetShape(label);
  XCAFPrs::CollectStyleSettings(label, shape.Location(), styles);
  for (NCollection_IndexedDataMap<TopoDS_Shape, XCAFPrs_Style, TopTools_ShapeMapHasher>::Iterator
         it(styles);
       it.More();
       it.Next()) {
    const TopoDS_Shape& key = it.Key();
    const XCAFPrs_Style& style = it.Value();
    if (!style.IsSetColorSurf()) {
      continue;
    }
    if (key.ShapeType() == TopAbs_FACE) {
      if (!target.Contains(key)) {
        target.Add(key, color_index_for(doc, style.GetColorSurfRGBA()));
      }
      continue;
    }
    for (TopExp_Explorer fe(key, TopAbs_FACE); fe.More(); fe.Next()) {
      if (!target.Contains(fe.Current())) {
        target.Add(fe.Current(), color_index_for(doc, style.GetColorSurfRGBA()));
      }
    }
  }
}

// Reads the UTF-8 name of a label; returns false when the label has none.
bool label_name(const TDF_Label& label, std::string& out) {
  occ::handle<TDataStd_Name> name;
  if (label.FindAttribute(TDataStd_Name::GetID(), name)) {
    const int bytes = name->Get().LengthOfCString();
    std::vector<char> buffer(static_cast<size_t>(bytes) + 1, '\0');
    Standard_PCharacter cursor = buffer.data();
    name->Get().ToUTF8CString(cursor);
    out.assign(buffer.data(), static_cast<size_t>(bytes));
    return true;
  }
  return false;
}

// Stores the node transform (accumulated location) as a row-major 3x4
// matrix; Value(i, j) already carries the scale factor in its coefficients.
void store_transform(GreyNode& node, const TopLoc_Location& location) {
  const gp_Trsf& trsf = location.Transformation();
  for (int row = 0; row < 3; ++row) {
    for (int col = 0; col < 4; ++col) {
      node.transform[static_cast<size_t>(4 * row + col)] =
        static_cast<float>(trsf.Value(row + 1, col + 1));
    }
  }
}

uint32_t push_node(GreyDoc& doc, uint32_t parent, const std::string& name,
                   const TopLoc_Location& location) {
  GreyNode node;
  node.parent = parent;
  node.mesh = GREYHOUND_NO_MESH;
  node.name = name;
  store_transform(node, location);
  doc.nodes.push_back(std::move(node));
  return static_cast<uint32_t>(doc.nodes.size() - 1);
}

// Meshes the label's simple shape once (or reuses the mesh recorded for a
// previously seen referred label) and returns its index.
uint32_t mesh_for_label(GreyDoc& doc, const TDF_Label& label) {
  if (doc.meshed_labels.Contains(label)) {
    return static_cast<uint32_t>(doc.meshed_labels.FindIndex(label) - 1);
  }
  GreyMesh mesh;
  mesh.shape = XCAFDoc_ShapeTool::GetShape(label);
  resolve_face_colors(doc, label, mesh.face_color_indices);
  doc.meshes.push_back(std::move(mesh));
  const uint32_t index = static_cast<uint32_t>(doc.meshes.size() - 1);
  doc.meshed_labels.Add(label);
  return index;
}

// Recurses the XCAF label tree: reference labels contribute instance nodes
// (name from the reference, falling back to the referred label; transform =
// the reference's own placement, LOCAL to the parent node so the consumer
// composes the hierarchy by parenting); referred assembly labels become
// geometry-free grouping nodes carrying the instance placement composed
// with the referred label's own location, simple labels contribute their
// mesh. Depth-first pre-order keeps every parent before its children. A
// referred shape's own location stays baked into its meshed vertices,
// which is why a simple free-shape root keeps an identity placement.
void walk_label(const TDF_Label& label, uint32_t parent, GreyDoc& doc) {
  if (XCAFDoc_ShapeTool::IsReference(label)) {
    TDF_Label referred;
    if (!XCAFDoc_ShapeTool::GetReferredShape(label, referred)) {
      greyhound::set_error("XCAF reference label without referred shape");
      return;
    }
    std::string name;
    if (!label_name(label, name)) {
      label_name(referred, name);
    }
    const TopLoc_Location own = XCAFDoc_ShapeTool::GetLocation(label);
    if (XCAFDoc_ShapeTool::IsAssembly(referred)) {
      const TopoDS_Shape shape = XCAFDoc_ShapeTool::GetShape(referred);
      const uint32_t node = push_node(doc, parent, name, own * shape.Location());
      NCollection_Sequence<TDF_Label> components;
      XCAFDoc_ShapeTool::GetComponents(referred, components);
      for (NCollection_Sequence<TDF_Label>::Iterator it(components); it.More();
           it.Next()) {
        walk_label(it.Value(), node, doc);
      }
    } else {
      const uint32_t mesh = mesh_for_label(doc, referred);
      const uint32_t node = push_node(doc, parent, name, own);
      doc.nodes[node].mesh = mesh;
    }
    return;
  }
  std::string name;
  label_name(label, name);
  const TopoDS_Shape shape = XCAFDoc_ShapeTool::GetShape(label);
  if (XCAFDoc_ShapeTool::IsAssembly(label)) {
    const uint32_t node = push_node(doc, parent, name, shape.Location());
    NCollection_Sequence<TDF_Label> components;
    XCAFDoc_ShapeTool::GetComponents(label, components);
    for (NCollection_Sequence<TDF_Label>::Iterator it(components); it.More();
         it.Next()) {
      walk_label(it.Value(), node, doc);
    }
    return;
  }
  const uint32_t mesh = mesh_for_label(doc, label);
  const uint32_t node = push_node(doc, parent, name, TopLoc_Location());
  doc.nodes[node].mesh = mesh;
}

// Solid-ordered walk shared by counts and fill, per mesh: every
// TopAbs_SOLID occurrence contributes its faces in order (occurrence
// semantics of TopExp_Explorer, so shared-face reuse across solids stays
// per-occurrence); all remaining faces not under any solid - open shells,
// sheet bodies, loose faces - merge into one trailing group with
// GREYHOUND_NO_SOLID.
void build_face_plan(GreyMesh& mesh, GreyDoc& doc) {
  mesh.plan.clear();
  uint32_t solid_index = 0;
  for (TopExp_Explorer s(mesh.shape, TopAbs_SOLID); s.More(); s.Next()) {
    const uint32_t current_solid = solid_index++;
    for (TopExp_Explorer f(s.Current(), TopAbs_FACE); f.More(); f.Next()) {
      const TopoDS_Face face = TopoDS::Face(f.Current());
      const int found = mesh.face_color_indices.FindIndex(face);
      const uint32_t color_index =
        found > 0 ? mesh.face_color_indices.FindFromIndex(found)
                  : color_index_for(doc, fallback_color());
      mesh.plan.push_back(FacePlanEntry{face, current_solid, color_index});
    }
  }
  const uint32_t no_solid = GREYHOUND_NO_SOLID;
  for (TopExp_Explorer f(mesh.shape, TopAbs_FACE, TopAbs_SOLID); f.More();
       f.Next()) {
    const TopoDS_Face face = TopoDS::Face(f.Current());
    const int found = mesh.face_color_indices.FindIndex(face);
    const uint32_t color_index =
      found > 0 ? mesh.face_color_indices.FindFromIndex(found)
                : color_index_for(doc, fallback_color());
    mesh.plan.push_back(FacePlanEntry{face, no_solid, color_index});
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
    reader.SetNameMode(true);
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
    GreyDoc* doc = new GreyDoc();
    doc->whole = XCAFDoc_ShapeTool::GetOneShape(free_shapes);
    if (doc->whole.IsNull()) {
      XCAFApp_Application::GetApplication()->Close(xdoc);
      delete doc;
      greyhound::set_error("STEP transfer produced no shape");
      return nullptr;
    }
    for (NCollection_Sequence<TDF_Label>::Iterator it(free_shapes); it.More();
         it.Next()) {
      walk_label(it.Value(), GREYHOUND_NO_PARENT, *doc);
    }
    // Everything the document holds has been resolved into the node forest
    // and the per-mesh shapes; close the document immediately.
    XCAFApp_Application::GetApplication()->Close(xdoc);
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
    for (TopExp_Explorer e(doc->whole, TopAbs_SOLID); e.More(); e.Next())
      ++*solids;
    for (TopExp_Explorer e(doc->whole, TopAbs_FACE); e.More(); e.Next())
      ++*faces;
    for (TopExp_Explorer e(doc->whole, TopAbs_EDGE); e.More(); e.Next())
      ++*edges;
    Bnd_Box box;
    BRepBndLib::Add(doc->whole, box);
    box.Get(bbox->min[0], bbox->min[1], bbox->min[2], bbox->max[0],
            bbox->max[1], bbox->max[2]);
    return 0;
  });
}

extern "C" void greyhound_step_close(void* doc_in) noexcept {
  (void)greyhound::guard<int32_t>(1, [&]() -> int32_t {
    delete static_cast<GreyDoc*>(doc_in);
    return 0;
  });
}

extern "C" int32_t greyhound_scene_counts(void* doc_in, uint32_t* nnodes,
                                          uint32_t* nmeshes, uint32_t* ncolors,
                                          uint32_t* nname_bytes) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !nnodes || !nmeshes || !ncolors || !nname_bytes) {
      greyhound::set_error("invalid scene handle or output pointer");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
    uint64_t names = 0;
    for (const GreyNode& node : doc->nodes) {
      names += node.name.size();
    }
    if (doc->nodes.size() > std::numeric_limits<uint32_t>::max() ||
        doc->meshes.size() > std::numeric_limits<uint32_t>::max() ||
        doc->color_palette.size() > std::numeric_limits<uint32_t>::max() ||
        names > std::numeric_limits<uint32_t>::max()) {
      greyhound::set_error("scene exceeds the C ABI count range");
      return 1;
    }
    *nnodes = static_cast<uint32_t>(doc->nodes.size());
    *nmeshes = static_cast<uint32_t>(doc->meshes.size());
    *ncolors = static_cast<uint32_t>(doc->color_palette.size());
    *nname_bytes = static_cast<uint32_t>(names);
    return 0;
  });
}

extern "C" int32_t greyhound_scene_fill(void* doc_in, uint32_t* nodes,
                                        float* transforms,
                                        char* names) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    auto* doc = static_cast<GreyDoc*>(doc_in);
    size_t name_bytes = 0;
    if (doc) {
      for (const GreyNode& node : doc->nodes) {
        name_bytes += node.name.size();
      }
    }
    if (!doc || !nodes || !transforms || (!names && name_bytes != 0)) {
      greyhound::set_error("invalid scene handle or output pointer");
      return 1;
    }
    uint32_t offset = 0;
    for (const GreyNode& node : doc->nodes) {
      nodes[0] = node.parent;
      nodes[1] = node.mesh;
      nodes[2] = offset;
      nodes[3] = static_cast<uint32_t>(node.name.size());
      nodes += 4;
      for (int i = 0; i < 12; ++i) {
        *transforms++ = node.transform[i];
      }
      if (!node.name.empty()) {
        std::memcpy(names, node.name.data(), node.name.size());
        names += node.name.size();
      }
      offset += static_cast<uint32_t>(node.name.size());
    }
    return 0;
  });
}

extern "C" int32_t greyhound_color_fill(void* doc_in, float* colors) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!doc_in || !colors) {
      greyhound::set_error("invalid scene handle or output pointer");
      return 1;
    }
    auto* doc = static_cast<GreyDoc*>(doc_in);
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

extern "C" int32_t greyhound_mesh_counts(
    void* doc_in, uint32_t mesh_index, double deflection, double angle_rad,
    uint32_t* nverts, uint32_t* nindices, uint32_t* nfaces) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    auto* doc = static_cast<GreyDoc*>(doc_in);
    if (!doc || mesh_index >= doc->meshes.size() || !nverts || !nindices ||
        !nfaces || !std::isfinite(deflection) ||
        !std::isfinite(angle_rad) || deflection <= 0 || angle_rad <= 0) {
      greyhound::set_error("invalid tessellation arguments");
      return 1;
    }
    GreyMesh& mesh = doc->meshes[mesh_index];
    build_face_plan(mesh, *doc);
    BRepMesh_IncrementalMesh tessellator(mesh.shape, deflection, false,
                                         angle_rad, true);
    uint64_t nv = 0, ni = 0;
    for (const FacePlanEntry& entry : mesh.plan) {
      TopLoc_Location loc;
      const occ::handle<Poly_Triangulation>& tri =
        BRep_Tool::Triangulation(entry.face, loc);
      if (tri.IsNull()) {
        continue;
      }
      nv += static_cast<uint64_t>(tri->NbNodes());
      ni += 3ULL * static_cast<uint64_t>(tri->NbTriangles());
      if (nv > std::numeric_limits<uint32_t>::max() ||
          ni > std::numeric_limits<uint32_t>::max() ||
          mesh.plan.size() > std::numeric_limits<uint32_t>::max()) {
        greyhound::set_error("mesh exceeds the C ABI count range");
        return 1;
      }
    }
    *nverts = static_cast<uint32_t>(nv);
    *nindices = static_cast<uint32_t>(ni);
    *nfaces = static_cast<uint32_t>(mesh.plan.size());
    return 0;
  });
}

extern "C" int32_t greyhound_mesh_fill(
    void* doc_in, uint32_t mesh_index, float* verts, float* normals,
    uint32_t* indices, uint32_t* face_counts,
    uint32_t* face_attribs) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    auto* doc = static_cast<GreyDoc*>(doc_in);
    if (!doc || mesh_index >= doc->meshes.size() || !verts || !normals ||
        !indices || !face_counts || !face_attribs) {
      greyhound::set_error("invalid mesh handle or output pointer");
      return 1;
    }
    const GreyMesh& mesh = doc->meshes[mesh_index];
    uint32_t base = 0;
    for (size_t entry_index = 0; entry_index < mesh.plan.size(); ++entry_index) {
      const FacePlanEntry& entry = mesh.plan[entry_index];
      TopLoc_Location loc;
      const TopoDS_Face face = entry.face;
      const occ::handle<Poly_Triangulation>& tri =
        BRep_Tool::Triangulation(face, loc);
      if (tri.IsNull()) {
        continue;
      }
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
        // (RWMesh_FaceIterator::NormalTransformed): transform by the
        // location, then reverse for REVERSED faces so normals match the
        // emitted (orientation-corrected) winding.
        gp_Dir n = tri->Normal(i);
        if (!loc.IsIdentity()) {
          n.Transform(trsf);
        }
        if (reversed) {
          n.Reverse();
        }
        *normals++ = static_cast<float>(n.X());
        *normals++ = static_cast<float>(n.Y());
        *normals++ = static_cast<float>(n.Z());
      }
      uint32_t tri_count = 0;
      for (int i = 1; i <= tri->NbTriangles(); ++i) {
        int n1, n2, n3;
        tri->Triangle(i).Get(n1, n2, n3);
        if (reversed) {
          std::swap(n2, n3);
        }
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
    return 0;
  });
}

} // extern "C"
