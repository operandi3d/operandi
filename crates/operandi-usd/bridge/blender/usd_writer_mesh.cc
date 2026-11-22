/* SPDX-FileCopyrightText: 2019 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_writer_mesh.cc at
 * commit e1bec63572c7092feb1af6719f8b4f76acce15fc: write_mesh,
 * write_uv_data, write_normals, write_subdiv, get_edge_creases and the
 * subset and binding parts of assign_materials. The UV de-duplication follows
 * the idea of Blender's bke::mesh::calc_uv_verts (corners of one point
 * with the same UV share one value). Operandi gives the arrays directly,
 * already in its own crease sharpness (10 x edge weight), and there are no
 * time samples. */
#include "blender_usd.hh"

#include <pxr/usd/usdGeom/mesh.h>
#include <pxr/usd/usdGeom/primvarsAPI.h>
#include <pxr/usd/usdGeom/subset.h>
#include <pxr/usd/usdShade/materialBindingAPI.h>

#include <algorithm>
#include <cmath>
#include <cstring>
#include <map>
#include <tuple>

namespace operandi::blender_usd {

static pxr::TfToken boundary_token(uint32_t boundary)
{
  /* For reference, see
   * https://graphics.pixar.com/opensubdiv/docs/subdivision_surfaces.html#boundary-interpolation-rules
   */
  return boundary == OU_BOUNDARY_EDGE_ONLY ? pxr::UsdGeomTokens->edgeOnly :
                                             pxr::UsdGeomTokens->edgeAndCorner;
}

static pxr::TfToken face_varying_token(uint32_t mode)
{
  /* For reference, see
   * https://graphics.pixar.com/opensubdiv/docs/subdivision_surfaces.html#face-varying-interpolation-rules
   */
  switch (mode) {
    case OU_FACE_VARYING_NONE:
      return pxr::UsdGeomTokens->none;
    case OU_FACE_VARYING_BOUNDARIES:
      return pxr::UsdGeomTokens->boundaries;
    case OU_FACE_VARYING_ALL:
      return pxr::UsdGeomTokens->all;
    case OU_FACE_VARYING_CORNERS_PLUS1:
    default:
      return pxr::UsdGeomTokens->cornersPlus1;
  }
}

static void write_uv_data(const pxr::UsdGeomMesh &usd_mesh,
                          const pxr::VtIntArray &face_indices,
                          const OuUvSet &set)
{
  /* Find the unique UVs and the index of the unique UV used by each face corner:
   * corners of the same point with the same UV share one value, so the
   * primvar stays continuous across faces wherever the map is. */
  pxr::VtIntArray indices(set.corner_count);
  pxr::VtArray<pxr::GfVec2f> unique_uvs;
  std::map<std::tuple<int, float, float>, int> seen;
  for (size_t corner = 0; corner < set.corner_count; corner++) {
    const float u = set.corner_uvs[corner * 2];
    const float v = set.corner_uvs[corner * 2 + 1];
    const auto key = std::make_tuple(face_indices[corner], u, v);
    auto found = seen.find(key);
    if (found == seen.end()) {
      found = seen.emplace(key, int(unique_uvs.size())).first;
      unique_uvs.push_back(pxr::GfVec2f(u, v));
    }
    indices[corner] = found->second;
  }

  const pxr::TfToken pv_name(make_safe_name(set.name));
  const pxr::UsdGeomPrimvarsAPI pv_api = pxr::UsdGeomPrimvarsAPI(usd_mesh);
  pxr::UsdGeomPrimvar pv_uv = pv_api.CreatePrimvar(
      pv_name, pxr::SdfValueTypeNames->TexCoord2fArray, pxr::UsdGeomTokens->faceVarying);
  pv_uv.Set(unique_uvs);
  pv_uv.CreateIndicesAttr().Set(indices);
}

static void write_normals(pxr::UsdGeomMesh &usd_mesh, const OuMesh &mesh)
{
  pxr::VtVec3fArray loop_normals(mesh.index_count);
  std::memcpy(loop_normals.data(), mesh.normals, sizeof(float) * 3 * mesh.index_count);
  usd_mesh.CreateNormalsAttr().Set(loop_normals);
  usd_mesh.SetNormalsInterpolation(pxr::UsdGeomTokens->faceVarying);
}

static void write_subdiv(const pxr::UsdGeomMesh &usd_mesh, const OuMesh &mesh)
{
  if (mesh.subdivision != OU_SUBDIVISION_CATMULL_CLARK) {
    usd_mesh.CreateSubdivisionSchemeAttr().Set(pxr::UsdGeomTokens->none);
    return;
  }
  usd_mesh.CreateSubdivisionSchemeAttr().Set(pxr::UsdGeomTokens->catmullClark);
  usd_mesh.CreateFaceVaryingLinearInterpolationAttr().Set(
      face_varying_token(mesh.uv_interpolation));
  usd_mesh.CreateInterpolateBoundaryAttr().Set(boundary_token(mesh.boundary));
}

static void write_creases(const pxr::UsdGeomMesh &usd_mesh, const OuMesh &mesh)
{
  if (mesh.crease_count == 0) {
    return;
  }
  /* One crease per edge, two points long, as Blender writes them. */
  pxr::VtIntArray crease_lengths(mesh.crease_count, 2);
  pxr::VtIntArray crease_vertex_indices(mesh.crease_points,
                                        mesh.crease_points + mesh.crease_count * 2);
  pxr::VtFloatArray crease_sharpnesses(mesh.crease_sharpness,
                                       mesh.crease_sharpness + mesh.crease_count);
  usd_mesh.CreateCreaseLengthsAttr().Set(crease_lengths);
  usd_mesh.CreateCreaseIndicesAttr().Set(crease_vertex_indices);
  usd_mesh.CreateCreaseSharpnessesAttr().Set(crease_sharpnesses);
}

static bool assign_materials(const pxr::UsdStageRefPtr &stage,
                             const pxr::UsdGeomMesh &usd_mesh,
                             const OuMesh &mesh,
                             std::string &error)
{
  /* Binding a material to a geometry subset isn't supported by the Hydra GL viewport yet,
   * which is why we always bind the first material to the entire mesh. See
   * https://github.com/PixarAnimationStudios/USD/issues/542 for more info. */
  if (!bind_material(stage, usd_mesh.GetPrim(), mesh.material, error)) {
    return false;
  }
  if (mesh.subset_count == 0) {
    return true;
  }
  /* Define a geometry subset per material tag. */
  pxr::UsdShadeMaterialBindingAPI material_binding_api(usd_mesh.GetPrim());
  for (size_t i = 0; i < mesh.subset_count; i++) {
    const OuSubset &subset = mesh.subsets[i];
    pxr::VtIntArray face_indices(subset.faces, subset.faces + subset.face_count);
    const pxr::TfToken name(make_safe_name(subset.name));
    pxr::UsdGeomSubset usd_face_subset = material_binding_api.CreateMaterialBindSubset(
        name, face_indices);
    /* Keep the tag's own spelling for the way back in. */
    usd_face_subset.GetPrim().SetCustomDataByKey(pxr::TfToken("operandi:materialTag"),
                                                 pxr::VtValue(std::string(subset.name)));
    if (!bind_material(stage, usd_face_subset.GetPrim(), subset.material, error)) {
      return false;
    }
  }
  return true;
}

bool write_mesh(const pxr::UsdStageRefPtr &stage,
                const pxr::SdfPath &path,
                const OuMesh &mesh,
                std::string &error)
{
  size_t corners = 0;
  for (size_t f = 0; f < mesh.face_count; f++) {
    if (mesh.face_counts[f] < 3) {
      error = "a polygon has fewer than three corners";
      return false;
    }
    corners += size_t(mesh.face_counts[f]);
  }
  if (corners != mesh.index_count) {
    error = "face corner counts don't add up to the index list";
    return false;
  }
  for (size_t i = 0; i < mesh.index_count; i++) {
    if (mesh.face_indices[i] < 0 || size_t(mesh.face_indices[i]) >= mesh.point_count) {
      error = "a face corner names a point that doesn't exist";
      return false;
    }
  }
  for (size_t i = 0; i < mesh.crease_count * 2; i++) {
    if (mesh.crease_points[i] < 0 || size_t(mesh.crease_points[i]) >= mesh.point_count) {
      error = "a crease names a point that doesn't exist";
      return false;
    }
  }
  for (size_t i = 0; i < mesh.uv_set_count; i++) {
    if (mesh.uv_sets[i].corner_count != mesh.index_count) {
      error = "a UV map doesn't have one value per face corner";
      return false;
    }
  }

  pxr::UsdGeomMesh usd_mesh = pxr::UsdGeomMesh::Define(stage, path);
  if (!mesh.visible) {
    usd_mesh.CreateVisibilityAttr().Set(pxr::UsdGeomTokens->invisible);
  }

  pxr::VtArray<pxr::GfVec3f> points(mesh.point_count);
  std::memcpy(points.data(), mesh.points, sizeof(float) * 3 * mesh.point_count);
  pxr::VtIntArray face_vertex_counts(mesh.face_counts, mesh.face_counts + mesh.face_count);
  pxr::VtIntArray face_indices(mesh.face_indices, mesh.face_indices + mesh.index_count);
  usd_mesh.CreatePointsAttr().Set(points);
  usd_mesh.CreateFaceVertexCountsAttr().Set(face_vertex_counts);
  usd_mesh.CreateFaceVertexIndicesAttr().Set(face_indices);

  /* Operandi change (2026-11-18): creases on plain meshes too. They only
   * shape a subdivision surface, but Blender writes and reads them on any
   * mesh, and a round trip keeps them. */
  write_creases(usd_mesh, mesh);

  for (size_t i = 0; i < mesh.uv_set_count; i++) {
    write_uv_data(usd_mesh, face_indices, mesh.uv_sets[i]);
  }

  /* Normals are written unless the mesh is a subdivision surface, whose
   * normals are computed, not stored with the mesh. */
  if (mesh.normals != nullptr && mesh.subdivision == OU_SUBDIVISION_NONE) {
    write_normals(usd_mesh, mesh);
  }

  pxr::VtArray<pxr::GfVec3f> extent(2);
  if (mesh.point_count > 0) {
    pxr::GfVec3f lo = points[0];
    pxr::GfVec3f hi = points[0];
    for (const pxr::GfVec3f &p : points) {
      for (int a = 0; a < 3; a++) {
        lo[a] = std::min(lo[a], p[a]);
        hi[a] = std::max(hi[a], p[a]);
      }
    }
    extent[0] = lo;
    extent[1] = hi;
  }
  usd_mesh.CreateExtentAttr().Set(extent);

  /* The subdivision scheme is a uniform according to spec,
   * so this value cannot be animated. */
  write_subdiv(usd_mesh, mesh);

  /* Blender defaults to double-sided, but USD to single-sided. */
  usd_mesh.CreateDoubleSidedAttr(pxr::VtValue(mesh.double_sided != 0));

  return assign_materials(stage, usd_mesh, mesh, error);
}

}  // namespace operandi::blender_usd
