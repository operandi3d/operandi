/* SPDX-FileCopyrightText: 2019 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_writer_transform.cc
 * at commit e1bec63572c7092feb1af6719f8b4f76acce15fc (create_xformable,
 * do_write and set_xform_ops). Operandi hands over its own translate,
 * Euler rotation and scale instead of a matrix to decompose, so the rotate
 * op follows the item's rotation order; a matrix op is kept for the root's
 * up-axis turn. Static scenes only: no time samples. */
#include "blender_usd.hh"

#include <pxr/base/gf/matrix4d.h>
#include <pxr/usd/usdGeom/xform.h>

namespace operandi::blender_usd {

static pxr::UsdGeomXformOp add_rotate_op(const pxr::UsdGeomXformable &xf, uint32_t order)
{
  switch (order) {
    case OU_ROTATE_XZY:
      return xf.AddRotateXZYOp(pxr::UsdGeomXformOp::PrecisionDouble);
    case OU_ROTATE_YXZ:
      return xf.AddRotateYXZOp(pxr::UsdGeomXformOp::PrecisionDouble);
    case OU_ROTATE_YZX:
      return xf.AddRotateYZXOp(pxr::UsdGeomXformOp::PrecisionDouble);
    case OU_ROTATE_ZXY:
      return xf.AddRotateZXYOp(pxr::UsdGeomXformOp::PrecisionDouble);
    case OU_ROTATE_ZYX:
      return xf.AddRotateZYXOp(pxr::UsdGeomXformOp::PrecisionDouble);
    case OU_ROTATE_XYZ:
    default:
      return xf.AddRotateXYZOp(pxr::UsdGeomXformOp::PrecisionDouble);
  }
}

void write_xform(const pxr::UsdStageRefPtr &stage, const pxr::SdfPath &path, const OuXform &xform)
{
  pxr::UsdGeomXform usd_xform = pxr::UsdGeomXform::Define(stage, path);
  pxr::UsdGeomXformable xf(usd_xform.GetPrim());

  if (xform.use_matrix) {
    /* Row-major, USD's row-vector convention (translation in the last row). */
    pxr::GfMatrix4d mat;
    for (int r = 0; r < 4; r++) {
      for (int c = 0; c < 4; c++) {
        mat[r][c] = xform.matrix[r * 4 + c];
      }
    }
    /* USD Xforms are by default the identity transform; only write if necessary. */
    if (mat != pxr::GfMatrix4d(1.0)) {
      xf.AddTransformOp().Set(mat);
    }
  }
  else {
    const bool moved = xform.translate[0] != 0.0 || xform.translate[1] != 0.0 ||
                       xform.translate[2] != 0.0;
    const bool turned = xform.rotate[0] != 0.0 || xform.rotate[1] != 0.0 ||
                        xform.rotate[2] != 0.0;
    const bool scaled = xform.scale[0] != 1.0 || xform.scale[1] != 1.0 || xform.scale[2] != 1.0;
    /* Blender writes all three TRS ops together; so does Operandi, unless
     * the transform is the identity. */
    if (moved || turned || scaled) {
      xf.AddTranslateOp().Set(
          pxr::GfVec3d(xform.translate[0], xform.translate[1], xform.translate[2]));
      add_rotate_op(xf, xform.rotate_order)
          .Set(pxr::GfVec3d(xform.rotate[0], xform.rotate[1], xform.rotate[2]));
      /* Double precision (Blender writes float) so scales read back exactly. */
      xf.AddScaleOp(pxr::UsdGeomXformOp::PrecisionDouble)
          .Set(pxr::GfVec3d(xform.scale[0], xform.scale[1], xform.scale[2]));
    }
  }

  if (!xform.visible) {
    usd_xform.CreateVisibilityAttr().Set(pxr::UsdGeomTokens->invisible);
  }
}

}  // namespace operandi::blender_usd
