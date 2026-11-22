/* SPDX-FileCopyrightText: 2019 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_writer_material.cc
 * at commit e1bec63572c7092feb1af6719f8b4f76acce15fc: the UsdPreviewSurface
 * network (create_usd_preview_shader, process_inputs' texture and constant
 * cases, create_uvmap_shader, create_transform2d_shader, set_scale_bias,
 * the wrap and colour-space inputs) and the material binding in
 * USDGenericMeshWriter::assign_materials. Blender walks its node tree to
 * find the images; Operandi hands over each input already worked out
 * (operandi_usd.h OuMaterial), so there is no node traversal here. */
#include "blender_usd.hh"

#include <pxr/usd/sdf/assetPath.h>
#include <pxr/usd/usdShade/material.h>
#include <pxr/usd/usdShade/materialBindingAPI.h>
#include <pxr/usd/usdShade/shader.h>

#include <map>

namespace operandi::blender_usd {

namespace usdtokens {
static const pxr::TfToken preview_surface("UsdPreviewSurface", pxr::TfToken::Immortal);
static const pxr::TfToken preview_shader("previewShader", pxr::TfToken::Immortal);
static const pxr::TfToken UsdTransform2d("UsdTransform2d", pxr::TfToken::Immortal);
static const pxr::TfToken uv_texture("UsdUVTexture", pxr::TfToken::Immortal);
static const pxr::TfToken primvar_float2("UsdPrimvarReader_float2", pxr::TfToken::Immortal);
static const pxr::TfToken surface("surface", pxr::TfToken::Immortal);
static const pxr::TfToken rgb("rgb", pxr::TfToken::Immortal);
static const pxr::TfToken r("r", pxr::TfToken::Immortal);
static const pxr::TfToken g("g", pxr::TfToken::Immortal);
static const pxr::TfToken b("b", pxr::TfToken::Immortal);
static const pxr::TfToken a("a", pxr::TfToken::Immortal);
static const pxr::TfToken st("st", pxr::TfToken::Immortal);
static const pxr::TfToken result("result", pxr::TfToken::Immortal);
static const pxr::TfToken varname("varname", pxr::TfToken::Immortal);
static const pxr::TfToken file("file", pxr::TfToken::Immortal);
static const pxr::TfToken raw("raw", pxr::TfToken::Immortal);
static const pxr::TfToken scale("scale", pxr::TfToken::Immortal);
static const pxr::TfToken bias("bias", pxr::TfToken::Immortal);
static const pxr::TfToken sRGB("sRGB", pxr::TfToken::Immortal);
static const pxr::TfToken sourceColorSpace("sourceColorSpace", pxr::TfToken::Immortal);
static const pxr::TfToken black("black", pxr::TfToken::Immortal);
static const pxr::TfToken clamp("clamp", pxr::TfToken::Immortal);
static const pxr::TfToken repeat("repeat", pxr::TfToken::Immortal);
static const pxr::TfToken mirror("mirror", pxr::TfToken::Immortal);
static const pxr::TfToken wrapS("wrapS", pxr::TfToken::Immortal);
static const pxr::TfToken wrapT("wrapT", pxr::TfToken::Immortal);
static const pxr::TfToken in("in", pxr::TfToken::Immortal);
static const pxr::TfToken rotation("rotation", pxr::TfToken::Immortal);
}  // namespace usdtokens

static pxr::UsdShadeShader define_shader(const pxr::UsdStageRefPtr &stage,
                                         const pxr::UsdShadeMaterial &material,
                                         const std::string &name,
                                         const pxr::TfToken &id)
{
  pxr::SdfPath shader_path = material.GetPath().AppendChild(pxr::TfToken(make_safe_name(name)));
  pxr::UsdShadeShader shader = pxr::UsdShadeShader::Define(stage, shader_path);
  shader.CreateIdAttr(pxr::VtValue(id));
  return shader;
}

static pxr::TfToken wrap_token(uint32_t wrap)
{
  switch (wrap) {
    case OU_WRAP_MIRROR:
      return usdtokens::mirror;
    case OU_WRAP_CLAMP:
      return usdtokens::clamp;
    case OU_WRAP_BLACK:
      return usdtokens::black;
    case OU_WRAP_REPEAT:
    default:
      return usdtokens::repeat;
  }
}

static pxr::TfToken channel_token(uint32_t channel)
{
  switch (channel) {
    case OU_CHANNEL_R:
      return usdtokens::r;
    case OU_CHANNEL_G:
      return usdtokens::g;
    case OU_CHANNEL_B:
      return usdtokens::b;
    case OU_CHANNEL_A:
      return usdtokens::a;
    case OU_CHANNEL_RGB:
    default:
      return usdtokens::rgb;
  }
}

static pxr::SdfValueTypeName input_type(uint32_t type)
{
  switch (type) {
    case OU_INPUT_COLOR:
      return pxr::SdfValueTypeNames->Color3f;
    case OU_INPUT_NORMAL:
      return pxr::SdfValueTypeNames->Normal3f;
    case OU_INPUT_FLOAT:
    default:
      return pxr::SdfValueTypeNames->Float;
  }
}

/* The UV source of a texture: one primvar reader per UV map, shared by the
 * material's textures (Blender's create_uvmap_shader), through a
 * UsdTransform2d when the texture repeats or turns (Blender's
 * create_transform2d_shader). */
static void create_uv_input(const pxr::UsdStageRefPtr &stage,
                            const pxr::UsdShadeMaterial &material,
                            const std::string &input_name,
                            const OuTexture &texture,
                            pxr::UsdShadeInput st_input,
                            std::map<std::string, pxr::UsdShadeShader> &readers)
{
  const std::string uv_name = make_safe_name(texture.uv ? texture.uv : "st");
  auto found = readers.find(uv_name);
  if (found == readers.end()) {
    pxr::UsdShadeShader reader = define_shader(
        stage, material, "uvmap_" + uv_name, usdtokens::primvar_float2);
    reader.CreateInput(usdtokens::varname, pxr::SdfValueTypeNames->String).Set(uv_name);
    reader.CreateOutput(usdtokens::result, pxr::SdfValueTypeNames->Float2);
    found = readers.emplace(uv_name, reader).first;
  }
  pxr::UsdShadeShader reader = found->second;

  if (!texture.has_transform) {
    st_input.ConnectToSource(reader.ConnectableAPI(), usdtokens::result);
    return;
  }
  pxr::UsdShadeShader transform2d = define_shader(
      stage, material, "transform_" + input_name, usdtokens::UsdTransform2d);
  transform2d.CreateInput(usdtokens::in, pxr::SdfValueTypeNames->Float2)
      .ConnectToSource(reader.ConnectableAPI(), usdtokens::result);
  transform2d.CreateInput(usdtokens::scale, pxr::SdfValueTypeNames->Float2)
      .Set(pxr::GfVec2f(texture.uv_scale[0], texture.uv_scale[1]));
  transform2d.CreateInput(usdtokens::rotation, pxr::SdfValueTypeNames->Float)
      .Set(texture.uv_rotation);
  transform2d.CreateOutput(usdtokens::result, pxr::SdfValueTypeNames->Float2);
  st_input.ConnectToSource(transform2d.ConnectableAPI(), usdtokens::result);
}

static void create_texture_input(const pxr::UsdStageRefPtr &stage,
                                 const pxr::UsdShadeMaterial &material,
                                 pxr::UsdShadeShader &preview_surface,
                                 const OuSurfaceInput &input,
                                 std::map<std::string, pxr::UsdShadeShader> &readers)
{
  const OuTexture &texture = *input.texture;
  pxr::UsdShadeShader usd_shader = define_shader(
      stage, material, std::string("tex_") + input.name, usdtokens::uv_texture);
  if (texture.file != nullptr && texture.file[0] != '\0') {
    usd_shader.CreateInput(usdtokens::file, pxr::SdfValueTypeNames->Asset)
        .Set(pxr::SdfAssetPath(texture.file));
  }
  usd_shader.CreateInput(usdtokens::sourceColorSpace, pxr::SdfValueTypeNames->Token)
      .Set(texture.raw ? usdtokens::raw : usdtokens::sRGB);
  const pxr::TfToken wrap = wrap_token(texture.wrap);
  usd_shader.CreateInput(usdtokens::wrapS, pxr::SdfValueTypeNames->Token).Set(wrap);
  usd_shader.CreateInput(usdtokens::wrapT, pxr::SdfValueTypeNames->Token).Set(wrap);

  /* Scale-bias: only written when it isn't the identity. */
  const pxr::GfVec4f scale(texture.scale[0], texture.scale[1], texture.scale[2], texture.scale[3]);
  const pxr::GfVec4f bias(texture.bias[0], texture.bias[1], texture.bias[2], texture.bias[3]);
  if (scale != pxr::GfVec4f(1.0f) || bias != pxr::GfVec4f(0.0f)) {
    usd_shader.CreateInput(usdtokens::scale, pxr::SdfValueTypeNames->Float4).Set(scale);
    usd_shader.CreateInput(usdtokens::bias, pxr::SdfValueTypeNames->Float4).Set(bias);
  }

  /* Create the UsdUVTexture node output attribute that should be connected to this input. */
  const pxr::TfToken source_name = channel_token(texture.channel);
  usd_shader.CreateOutput(source_name,
                          source_name == usdtokens::rgb ? pxr::SdfValueTypeNames->Float3 :
                                                          pxr::SdfValueTypeNames->Float);

  /* Create the preview surface input and connect it to the shader. */
  pxr::UsdShadeConnectionSourceInfo source_info(
      usd_shader.ConnectableAPI(), source_name, pxr::UsdShadeAttributeType::Output);
  preview_surface.CreateInput(pxr::TfToken(input.name), input_type(input.type))
      .ConnectToSource(source_info);

  if (pxr::UsdShadeInput st_input = usd_shader.CreateInput(usdtokens::st,
                                                           pxr::SdfValueTypeNames->Float2))
  {
    create_uv_input(stage, material, input.name, texture, st_input, readers);
  }
}

bool write_material(const pxr::UsdStageRefPtr &stage,
                    const pxr::SdfPath &path,
                    const OuMaterial &material,
                    std::string &error)
{
  for (size_t i = 0; i < material.input_count; i++) {
    if (material.inputs[i].name == nullptr || material.inputs[i].name[0] == '\0') {
      error = "a material input has no name";
      return false;
    }
  }
  pxr::UsdShadeMaterial usd_material = pxr::UsdShadeMaterial::Define(stage, path);
  pxr::UsdShadeShader preview_surface = define_shader(
      stage, usd_material, usdtokens::preview_shader.GetString(), usdtokens::preview_surface);
  usd_material.CreateSurfaceOutput().ConnectToSource(preview_surface.ConnectableAPI(),
                                                     usdtokens::surface);
  if (material.name != nullptr) {
    /* Keep the material's own spelling for the way back in. */
    usd_material.GetPrim().SetCustomDataByKey(pxr::TfToken("operandi:name"),
                                              pxr::VtValue(std::string(material.name)));
  }

  std::map<std::string, pxr::UsdShadeShader> readers;
  for (size_t i = 0; i < material.input_count; i++) {
    const OuSurfaceInput &input = material.inputs[i];
    if (input.texture != nullptr) {
      create_texture_input(stage, usd_material, preview_surface, input, readers);
      continue;
    }
    /* No upstream texture, just set a constant. */
    const pxr::TfToken name(input.name);
    if (input.type == OU_INPUT_FLOAT) {
      preview_surface.CreateInput(name, pxr::SdfValueTypeNames->Float).Set(input.value[0]);
    }
    else {
      preview_surface.CreateInput(name, input_type(input.type))
          .Set(pxr::GfVec3f(input.value[0], input.value[1], input.value[2]));
    }
  }
  return true;
}

bool bind_material(const pxr::UsdStageRefPtr &stage,
                   const pxr::UsdPrim &prim,
                   const char *material_path,
                   std::string &error)
{
  if (material_path == nullptr) {
    return true;
  }
  pxr::UsdShadeMaterial usd_material(stage->GetPrimAtPath(pxr::SdfPath(material_path)));
  if (!usd_material) {
    error = std::string("no material at ") + material_path;
    return false;
  }
  /* USD will require that prims with material bindings have the #MaterialBindingAPI applied
   * schema. While Bind() above will create the binding attribute, Apply() needs to be called as
   * well to add the #MaterialBindingAPI schema to the prim itself. */
  pxr::UsdShadeMaterialBindingAPI binding = pxr::UsdShadeMaterialBindingAPI::Apply(prim);
  binding.Bind(usd_material);
  return true;
}

}  // namespace operandi::blender_usd
