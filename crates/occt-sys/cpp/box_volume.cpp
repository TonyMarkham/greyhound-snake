#include "native_guard.h"
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <gp_Pnt.hxx>
#include <cmath>

extern "C" int32_t greyhound_box_volume(
    double dx, double dy, double dz, double* volume) noexcept {
  return greyhound::guard<int32_t>(1, [&]() -> int32_t {
    if (!volume || !std::isfinite(dx) || !std::isfinite(dy) || !std::isfinite(dz) || dx <= 0 || dy <= 0 || dz <= 0) {
      greyhound::set_error("invalid box dimensions or output pointer");
      return 1;
    }
    BRepPrimAPI_MakeBox box(gp_Pnt(0, 0, 0), dx, dy, dz);
    GProp_GProps props;
    BRepGProp::VolumeProperties(box.Shape(), props);
    *volume = props.Mass();
    return 0;
  });
}