/* SPDX-FileCopyrightText: 2019 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_writer_light.cc at
 * commit e1bec63572c7092feb1af6719f8b4f76acce15fc (USDLightWriter::do_write),
 * with Blender's world-to-dome export (usd_light_convert.cc) reduced to a
 * DomeLight with a lat-long picture. Operandi hands over the intensity
 * already converted (operandi-usd src/lights.rs) and adds cylinder and
 * portal lights and IES profiles. Static scenes only: no time samples. */
#include "blender_usd.hh"

#include <pxr/usd/sdf/assetPath.h>
#include <pxr/usd/usdLux/cylinderLight.h>
#include <pxr/usd/usdLux/diskLight.h>
#include <pxr/usd/usdLux/distantLight.h>
#include <pxr/usd/usdLux/domeLight.h>
#include <pxr/usd/usdLux/lightAPI.h>
#include <pxr/usd/usdLux/portalLight.h>
#include <pxr/usd/usdLux/rectLight.h>
#include <pxr/usd/usdLux/shapingAPI.h>
#include <pxr/usd/usdLux/sphereLight.h>

namespace operandi::blender_usd {

bool write_light(const pxr::UsdStageRefPtr &stage,
                 const pxr::SdfPath &path,
                 const OuLight &light,
                 std::string &error)
{
  pxr::UsdLuxLightAPI usd_light_api;

  switch (light.type) {
    case OU_LIGHT_RECT: {
      pxr::UsdLuxRectLight rect_light = pxr::UsdLuxRectLight::Define(stage, path);
      rect_light.CreateWidthAttr().Set(light.width);
      rect_light.CreateHeightAttr().Set(light.height);
      usd_light_api = rect_light.LightAPI();
      break;
    }
    case OU_LIGHT_DISK: {
      pxr::UsdLuxDiskLight disk_light = pxr::UsdLuxDiskLight::Define(stage, path);
      disk_light.CreateRadiusAttr().Set(light.radius);
      usd_light_api = disk_light.LightAPI();
      break;
    }
    case OU_LIGHT_SPHERE: {
      pxr::UsdLuxSphereLight sphere_light = pxr::UsdLuxSphereLight::Define(stage, path);
      sphere_light.CreateRadiusAttr().Set(light.radius);
      sphere_light.CreateTreatAsPointAttr().Set(light.radius == 0.0f);

      if (light.cone_angle > 0.0f || (light.ies_file != nullptr && light.ies_file[0] != '\0')) {
        pxr::UsdLuxShapingAPI shaping_api = pxr::UsdLuxShapingAPI::Apply(sphere_light.GetPrim());
        if (shaping_api && light.cone_angle > 0.0f) {
          shaping_api.CreateShapingConeAngleAttr().Set(light.cone_angle);
          shaping_api.CreateShapingConeSoftnessAttr().Set(light.cone_softness);
        }
        if (shaping_api && light.ies_file != nullptr && light.ies_file[0] != '\0') {
          shaping_api.CreateShapingIesFileAttr().Set(pxr::SdfAssetPath(light.ies_file));
        }
      }

      usd_light_api = sphere_light.LightAPI();
      break;
    }
    case OU_LIGHT_DISTANT: {
      pxr::UsdLuxDistantLight distant_light = pxr::UsdLuxDistantLight::Define(stage, path);
      distant_light.CreateAngleAttr().Set(light.angle);
      usd_light_api = distant_light.LightAPI();
      break;
    }
    case OU_LIGHT_CYLINDER: {
      pxr::UsdLuxCylinderLight cylinder = pxr::UsdLuxCylinderLight::Define(stage, path);
      cylinder.CreateLengthAttr().Set(light.length);
      cylinder.CreateRadiusAttr().Set(light.radius);
      usd_light_api = cylinder.LightAPI();
      break;
    }
    case OU_LIGHT_PORTAL: {
      pxr::UsdLuxPortalLight portal = pxr::UsdLuxPortalLight::Define(stage, path);
      portal.CreateWidthAttr().Set(light.width);
      portal.CreateHeightAttr().Set(light.height);
      usd_light_api = portal.LightAPI();
      break;
    }
    case OU_LIGHT_DOME: {
      pxr::UsdLuxDomeLight dome = pxr::UsdLuxDomeLight::Define(stage, path);
      if (light.texture_file != nullptr && light.texture_file[0] != '\0') {
        dome.CreateTextureFileAttr().Set(pxr::SdfAssetPath(light.texture_file));
        dome.CreateTextureFormatAttr().Set(pxr::UsdLuxTokens->latlong);
      }
      usd_light_api = dome.LightAPI();
      break;
    }
    default:
      error = "unknown light type";
      return false;
  }

  usd_light_api.CreateIntensityAttr().Set(light.intensity);
  usd_light_api.CreateExposureAttr().Set(0.0f);
  usd_light_api.CreateColorAttr().Set(pxr::GfVec3f(light.color[0], light.color[1], light.color[2]));
  usd_light_api.CreateEnableColorTemperatureAttr().Set(false);
  usd_light_api.CreateDiffuseAttr().Set(light.diffuse);
  usd_light_api.CreateSpecularAttr().Set(light.specular);
  usd_light_api.CreateNormalizeAttr().Set(light.normalize != 0);
  return true;
}

}  // namespace operandi::blender_usd
