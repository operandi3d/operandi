/* SPDX-FileCopyrightText: 2019 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_writer_camera.cc at
 * commit e1bec63572c7092feb1af6719f8b4f76acce15fc (USDCameraWriter::do_write).
 * Operandi hands over the lens and film already in USD's tenths of a scene
 * unit (operandi-usd src/lights.rs), and an orthographic projection too.
 * Static scenes only: no time samples. */
#include "blender_usd.hh"

#include <pxr/usd/usdGeom/camera.h>

namespace operandi::blender_usd {

void write_camera(const pxr::UsdStageRefPtr &stage, const pxr::SdfPath &path, const OuCamera &camera)
{
  pxr::UsdGeomCamera usd_camera = pxr::UsdGeomCamera::Define(stage, path);

  usd_camera.CreateProjectionAttr().Set(camera.orthographic ? pxr::UsdGeomTokens->orthographic :
                                                              pxr::UsdGeomTokens->perspective);

  /*
   * For USD, these camera properties are in tenths of a world unit.
   * https://graphics.pixar.com/usd/release/api/class_usd_geom_camera.html#UsdGeom_CameraUnits
   */
  usd_camera.CreateFocalLengthAttr().Set(camera.focal_length);
  usd_camera.CreateHorizontalApertureAttr().Set(camera.horizontal_aperture);
  usd_camera.CreateVerticalApertureAttr().Set(camera.vertical_aperture);
  usd_camera.CreateHorizontalApertureOffsetAttr().Set(camera.horizontal_offset);
  usd_camera.CreateVerticalApertureOffsetAttr().Set(camera.vertical_offset);
  usd_camera.CreateClippingRangeAttr().Set(pxr::GfVec2f(camera.clipping[0], camera.clipping[1]));

  /* Write DoF-related attributes. */
  usd_camera.CreateFStopAttr().Set(camera.f_stop);
  if (camera.f_stop > 0.0f) {
    usd_camera.CreateFocusDistanceAttr().Set(camera.focus_distance);
  }
}

}  // namespace operandi::blender_usd
