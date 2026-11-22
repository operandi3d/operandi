/* SPDX-FileCopyrightText: 2019-2024 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_utils.hh,
 * usd_writer_camera.hh, usd_writer_light.hh, usd_writer_mesh.hh,
 * usd_writer_material.hh and usd_writer_transform.hh at commit
 * e1bec63572c7092feb1af6719f8b4f76acce15fc. Blender's writer classes, its
 * depsgraph and its Mesh type are gone: these functions take Operandi's
 * plain arrays (operandi_usd.h) and write one prim each. */
#pragma once

#include "../operandi_usd.h"

#include <pxr/usd/sdf/path.h>
#include <pxr/usd/usd/stage.h>

#include <string>

namespace operandi::blender_usd {

/* A valid USD prim name for `name`: ASCII letters, digits and '_', not
 * starting with a digit (Blender's make_safe_name without Unicode). */
std::string make_safe_name(const std::string &name);

/* `path`, or `path` with 2, 3, ... appended when a prim is already there
 * (Blender's get_unique_path). */
pxr::SdfPath get_unique_path(const pxr::UsdStageRefPtr &stage, const pxr::SdfPath &path);

/* Defines an Xform prim and its translate / rotate / scale ops (or one
 * matrix op), as Blender's USDTransformWriter does. */
void write_xform(const pxr::UsdStageRefPtr &stage, const pxr::SdfPath &path, const OuXform &xform);

/* Defines a Mesh prim: points, faces, creases, UV primvars, normals,
 * subdivision attributes, extent and material-tag subsets, as Blender's
 * USDGenericMeshWriter::write_mesh does. Returns false with `error` set
 * when the arrays don't fit together. */
bool write_mesh(const pxr::UsdStageRefPtr &stage,
                const pxr::SdfPath &path,
                const OuMesh &mesh,
                std::string &error);

/* Defines a Material whose surface is a UsdPreviewSurface built from
 * `material`'s inputs, as Blender's create_usd_preview_surface_material. */
bool write_material(const pxr::UsdStageRefPtr &stage,
                    const pxr::SdfPath &path,
                    const OuMaterial &material,
                    std::string &error);

/* Binds the Material at `material_path` (NULL: nothing) to `prim`. */
bool bind_material(const pxr::UsdStageRefPtr &stage,
                   const pxr::UsdPrim &prim,
                   const char *material_path,
                   std::string &error);

/* Defines a Camera prim (Blender's USDCameraWriter::do_write). */
void write_camera(const pxr::UsdStageRefPtr &stage, const pxr::SdfPath &path, const OuCamera &camera);

/* Defines a UsdLux light prim (Blender's USDLightWriter::do_write). */
bool write_light(const pxr::UsdStageRefPtr &stage,
                 const pxr::SdfPath &path,
                 const OuLight &light,
                 std::string &error);

}  // namespace operandi::blender_usd
