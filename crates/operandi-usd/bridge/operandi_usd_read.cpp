/* Reading USD files for import (operandi_usd.h, "Reading").
 *
 * Operandi's own code. It walks a stage once and describes what it finds as
 * JSON (prims, their transforms, meshes, material bindings and shading
 * networks), with every large array (points, polygons, primvars) in a
 * numbered binary buffer the JSON names. The Rust side (src/read.rs and
 * src/import.rs) turns that into a scene; nothing here knows about
 * Operandi's scene. OpenUSD is TOST-1.0 (Apache-2.0 with a different
 * trademark section). */
#include "operandi_usd.h"

#include <pxr/base/gf/matrix4d.h>
#include <pxr/base/gf/vec2f.h>
#include <pxr/base/gf/vec3f.h>
#include <pxr/base/gf/vec4f.h>
#include <pxr/base/gf/half.h>
#include <pxr/base/gf/vec2i.h>
#include <pxr/base/gf/vec3i.h>
#include <pxr/base/gf/vec4d.h>
#include <pxr/base/gf/vec4i.h>
#include <pxr/base/tf/errorMark.h>
#include <pxr/base/tf/stringUtils.h>
#include <pxr/base/vt/array.h>
#include <pxr/usd/ar/asset.h>
#include <pxr/usd/ar/resolvedPath.h>
#include <pxr/usd/ar/resolver.h>
#include <pxr/usd/sdf/assetPath.h>
#include <pxr/usd/sdf/layerUtils.h>
#include <pxr/usd/usd/primRange.h>
#include <pxr/usd/usd/stage.h>
#include <pxr/usd/usd/variantSets.h>
#include <pxr/usd/usdGeom/basisCurves.h>
#include <pxr/usd/usdGeom/camera.h>
#include <pxr/usd/usdGeom/nurbsCurves.h>
#include <pxr/usd/usdGeom/imageable.h>
#include <pxr/usd/usdGeom/mesh.h>
#include <pxr/usd/usdGeom/metrics.h>
#include <pxr/usd/usdGeom/pointInstancer.h>
#include <pxr/usd/usdGeom/primvarsAPI.h>
#include <pxr/usd/usdGeom/subset.h>
#include <pxr/usd/usdGeom/tokens.h>
#include <pxr/usd/usdGeom/xformCommonAPI.h>
#include <pxr/usd/usdGeom/xformable.h>
#include <pxr/usd/usdLux/lightAPI.h>
#include <pxr/usd/usdSkel/bindingAPI.h>
#include <pxr/usd/usdSkel/animation.h>
#include <pxr/usd/usdSkel/blendShape.h>
#include <pxr/usd/usdSkel/skeleton.h>
#include <pxr/usd/usdShade/connectableAPI.h>
#include <pxr/usd/usdShade/material.h>
#include <pxr/usd/usdShade/materialBindingAPI.h>
#include <pxr/usd/usdShade/nodeGraph.h>
#include <pxr/usd/usdShade/shader.h>
#include <pxr/usd/usdShade/tokens.h>
#include <pxr/usd/usdShade/utils.h>

#include <cmath>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <map>
#include <set>
#include <sstream>
#include <string>
#include <vector>

struct OuReader {
  std::string json;
  std::vector<std::vector<char>> buffers;
};

namespace {

using namespace pxr;

void set_text(char *out, size_t len, const std::string &text)
{
  if (out == nullptr || len == 0) {
    return;
  }
  const size_t n = std::min(len - 1, text.size());
  std::memcpy(out, text.data(), n);
  out[n] = '\0';
}

std::string json_string(const std::string &s)
{
  std::string out = "\"";
  for (unsigned char c : s) {
    switch (c) {
      case '"':
        out += "\\\"";
        break;
      case '\\':
        out += "\\\\";
        break;
      case '\n':
        out += "\\n";
        break;
      case '\r':
        out += "\\r";
        break;
      case '\t':
        out += "\\t";
        break;
      default:
        if (c < 0x20) {
          char buf[8];
          std::snprintf(buf, sizeof(buf), "\\u%04x", c);
          out += buf;
        }
        else {
          out += char(c);
        }
    }
  }
  return out + "\"";
}

/* A number JSON can hold (NaN and infinities become 0). */
std::string num(double v)
{
  if (!std::isfinite(v)) {
    return "0";
  }
  char buf[32];
  std::snprintf(buf, sizeof(buf), "%.17g", v);
  return buf;
}

/* Collects one reader's JSON and buffers. */
struct Out {
  OuReader *reader;

  /* Stores `values` as a new buffer; returns its JSON reference. */
  template<typename T> std::string buffer(const T *values, size_t count, const char *type)
  {
    std::vector<char> bytes(sizeof(T) * count);
    if (count > 0) {
      std::memcpy(bytes.data(), values, bytes.size());
    }
    reader->buffers.push_back(std::move(bytes));
    return std::string("{\"buffer\":") + std::to_string(reader->buffers.size() - 1) +
           ",\"type\":\"" + type + "\",\"count\":" + std::to_string(count) + "}";
  }
  std::string ints(const VtIntArray &a)
  {
    return buffer(a.cdata(), a.size(), "i32");
  }
  std::string floats(const std::vector<float> &a)
  {
    return buffer(a.data(), a.size(), "f32");
  }
};

/* A primvar's or attribute's values as flat floats, with the number of
 * components per element; false for types this doesn't read. */
bool flat_floats(const VtValue &v, std::vector<float> &out, int &dims)
{
  out.clear();
  if (v.IsHolding<VtVec2fArray>()) {
    dims = 2;
    for (const GfVec2f &e : v.UncheckedGet<VtVec2fArray>()) {
      out.insert(out.end(), {e[0], e[1]});
    }
    return true;
  }
  if (v.IsHolding<VtVec2dArray>()) {
    dims = 2;
    for (const GfVec2d &e : v.UncheckedGet<VtVec2dArray>()) {
      out.insert(out.end(), {float(e[0]), float(e[1])});
    }
    return true;
  }
  if (v.IsHolding<VtVec3fArray>()) {
    dims = 3;
    for (const GfVec3f &e : v.UncheckedGet<VtVec3fArray>()) {
      out.insert(out.end(), {e[0], e[1], e[2]});
    }
    return true;
  }
  if (v.IsHolding<VtVec3dArray>()) {
    dims = 3;
    for (const GfVec3d &e : v.UncheckedGet<VtVec3dArray>()) {
      out.insert(out.end(), {float(e[0]), float(e[1]), float(e[2])});
    }
    return true;
  }
  if (v.IsHolding<VtVec4fArray>()) {
    dims = 4;
    for (const GfVec4f &e : v.UncheckedGet<VtVec4fArray>()) {
      out.insert(out.end(), {e[0], e[1], e[2], e[3]});
    }
    return true;
  }
  if (v.IsHolding<VtFloatArray>()) {
    dims = 1;
    const VtFloatArray &a = v.UncheckedGet<VtFloatArray>();
    out.assign(a.begin(), a.end());
    return true;
  }
  if (v.IsHolding<VtDoubleArray>()) {
    dims = 1;
    for (double d : v.UncheckedGet<VtDoubleArray>()) {
      out.push_back(float(d));
    }
    return true;
  }
  /* Integer primvars (Houdini's id, Blender's int attributes) come as
   * floats; the type name says they were integers. */
  if (v.IsHolding<VtIntArray>()) {
    dims = 1;
    for (int i : v.UncheckedGet<VtIntArray>()) {
      out.push_back(float(i));
    }
    return true;
  }
  if (v.IsHolding<VtVec2iArray>()) {
    dims = 2;
    for (const GfVec2i &e : v.UncheckedGet<VtVec2iArray>()) {
      out.insert(out.end(), {float(e[0]), float(e[1])});
    }
    return true;
  }
  if (v.IsHolding<VtVec3iArray>()) {
    dims = 3;
    for (const GfVec3i &e : v.UncheckedGet<VtVec3iArray>()) {
      out.insert(out.end(), {float(e[0]), float(e[1]), float(e[2])});
    }
    return true;
  }
  if (v.IsHolding<VtVec4iArray>()) {
    dims = 4;
    for (const GfVec4i &e : v.UncheckedGet<VtVec4iArray>()) {
      out.insert(out.end(), {float(e[0]), float(e[1]), float(e[2]), float(e[3])});
    }
    return true;
  }
  if (v.IsHolding<VtVec4dArray>()) {
    dims = 4;
    for (const GfVec4d &e : v.UncheckedGet<VtVec4dArray>()) {
      out.insert(out.end(), {float(e[0]), float(e[1]), float(e[2]), float(e[3])});
    }
    return true;
  }
  if (v.IsHolding<VtHalfArray>()) {
    dims = 1;
    for (GfHalf h : v.UncheckedGet<VtHalfArray>()) {
      out.push_back(float(h));
    }
    return true;
  }
  return false;
}

/* A float2 primvar without the texture-coordinate role that is still a UV
 * set by name, as older files write `float2[] primvars:st`. */
bool uv_by_name(const std::string &name)
{
  return name == "st" || name == "UVMap" || name == "map1" || TfStringStartsWith(name, "st_") ||
         TfStringStartsWith(name, "st:") ||
         (name.size() > 2 && name.compare(0, 2, "st") == 0 && isdigit((unsigned char)name[2])) ||
         TfStringStartsWith(name, "uv") || TfStringStartsWith(name, "UV");
}

const UsdTimeCode TIME = UsdTimeCode::EarliestTime();

/* A customData value as JSON: strings, booleans, numbers, integer and
 * string lists, and dictionaries; anything else is null. */
std::string custom_json(const VtValue &v)
{
  if (v.IsHolding<std::string>()) return json_string(v.UncheckedGet<std::string>());
  if (v.IsHolding<TfToken>()) return json_string(v.UncheckedGet<TfToken>().GetString());
  if (v.IsHolding<bool>()) return v.UncheckedGet<bool>() ? "true" : "false";
  if (v.IsHolding<int>()) return std::to_string(v.UncheckedGet<int>());
  if (v.IsHolding<int64_t>()) return std::to_string(v.UncheckedGet<int64_t>());
  if (v.IsHolding<double>()) return num(v.UncheckedGet<double>());
  if (v.IsHolding<float>()) return num(v.UncheckedGet<float>());
  if (v.IsHolding<VtIntArray>()) {
    std::string s = "[";
    bool first = true;
    for (int i : v.UncheckedGet<VtIntArray>()) {
      s += (first ? "" : ",") + std::to_string(i);
      first = false;
    }
    return s + "]";
  }
  if (v.IsHolding<VtStringArray>()) {
    std::string s = "[";
    bool first = true;
    for (const std::string &e : v.UncheckedGet<VtStringArray>()) {
      s += (first ? "" : ",") + json_string(e);
      first = false;
    }
    return s + "]";
  }
  if (v.IsHolding<VtDictionary>()) {
    std::string s = "{";
    bool first = true;
    for (const auto &kv : v.UncheckedGet<VtDictionary>()) {
      s += (first ? "" : ",") + json_string(kv.first) + ":" + custom_json(kv.second);
      first = false;
    }
    return s + "}";
  }
  return "null";
}

/* `,"custom":{...}` for what Operandi wrote in `object`'s customData
 * (the "operandi" key), or "". */
std::string operandi_custom(const UsdObject &object)
{
  const VtValue v = object.GetCustomDataByKey(TfToken("operandi"));
  return v.IsHolding<VtDictionary>() ? ",\"custom\":" + custom_json(v) : "";
}

std::string matrix_json(const GfMatrix4d &m)
{
  std::string s = "[";
  for (int r = 0; r < 4; r++) {
    for (int c = 0; c < 4; c++) {
      s += (r || c ? "," : "") + num(m[r][c]);
    }
  }
  return s + "]";
}

/* translate / rotate (degrees) / scale and the rotation order, when the
 * prim's ops are plain TRS without pivots; "" otherwise. */
std::string trs_json(const UsdGeomXformable &xf)
{
  UsdGeomXformCommonAPI api(xf.GetPrim());
  if (!api) {
    return "";
  }
  GfVec3d t;
  GfVec3f r, s, pivot;
  UsdGeomXformCommonAPI::RotationOrder order;
  if (!api.GetXformVectorsByAccumulation(&t, &r, &s, &pivot, &order, TIME)) {
    return "";
  }
  if (pivot != GfVec3f(0)) {
    return "";
  }
  static const char *names[] = {"XYZ", "XZY", "YXZ", "YZX", "ZXY", "ZYX"};
  return "{\"t\":[" + num(t[0]) + "," + num(t[1]) + "," + num(t[2]) + "],\"r\":[" + num(r[0]) +
         "," + num(r[1]) + "," + num(r[2]) + "],\"s\":[" + num(s[0]) + "," + num(s[1]) + "," +
         num(s[2]) + "],\"order\":\"" + names[int(order)] + "\"}";
}

/* The JSON for a material input's or a NodeGraph interface's value. Asset
 * paths carry the path as written, OpenUSD's resolved path (which for a
 * .usdz is a package path "file.usdz[inner]") and the written path made
 * absolute against the layer that wrote it, for files that don't exist. */
std::string value_json(const UsdAttribute &attr)
{
  VtValue v;
  if (!attr.Get(&v, UsdTimeCode::Default()) || v.IsEmpty()) {
    return "";
  }
  std::ostringstream text;
  if (v.IsHolding<SdfAssetPath>()) {
    const SdfAssetPath &a = v.UncheckedGet<SdfAssetPath>();
    std::string anchored;
    for (const SdfPropertySpecHandle &spec : attr.GetPropertyStack(UsdTimeCode::Default())) {
      if (spec && spec->HasDefaultValue()) {
        anchored = SdfComputeAssetPathRelativeToLayer(spec->GetLayer(), a.GetAssetPath());
        break;
      }
    }
    return ",\"value\":" + json_string(a.GetAssetPath()) + ",\"resolved\":" +
           json_string(a.GetResolvedPath()) + ",\"anchored\":" + json_string(anchored);
  }
  if (v.IsHolding<float>()) {
    text << num(v.UncheckedGet<float>());
  }
  else if (v.IsHolding<double>()) {
    text << num(v.UncheckedGet<double>());
  }
  else if (v.IsHolding<int>()) {
    text << v.UncheckedGet<int>();
  }
  else if (v.IsHolding<bool>()) {
    text << (v.UncheckedGet<bool>() ? "true" : "false");
  }
  else if (v.IsHolding<TfToken>()) {
    text << v.UncheckedGet<TfToken>().GetString();
  }
  else if (v.IsHolding<std::string>()) {
    text << v.UncheckedGet<std::string>();
  }
  else if (v.IsHolding<GfVec2f>()) {
    const GfVec2f &e = v.UncheckedGet<GfVec2f>();
    text << num(e[0]) << ", " << num(e[1]);
  }
  else if (v.IsHolding<GfVec3f>()) {
    const GfVec3f &e = v.UncheckedGet<GfVec3f>();
    text << num(e[0]) << ", " << num(e[1]) << ", " << num(e[2]);
  }
  else if (v.IsHolding<GfVec4f>()) {
    const GfVec4f &e = v.UncheckedGet<GfVec4f>();
    text << num(e[0]) << ", " << num(e[1]) << ", " << num(e[2]) << ", " << num(e[3]);
  }
  else if (v.IsHolding<GfVec3d>()) {
    const GfVec3d &e = v.UncheckedGet<GfVec3d>();
    text << num(e[0]) << ", " << num(e[1]) << ", " << num(e[2]);
  }
  else {
    return ",\"unsupported\":" + json_string(v.GetTypeName());
  }
  return ",\"value\":" + json_string(text.str());
}

/* The prim's ColorSpaceAPI colour space, or its attribute's colorSpace
 * metadata, or "". */
std::string colour_space(const UsdPrim &prim, const UsdAttribute &attr)
{
  if (attr && attr.HasColorSpace()) {
    return attr.GetColorSpace().GetString();
  }
  TfToken space;
  UsdAttribute a = prim.GetAttribute(TfToken("colorSpace:name"));
  if (a && a.Get(&space)) {
    return space.GetString();
  }
  return "";
}

/* The camera and light attributes the importer reads (UsdGeomCamera,
 * UsdLux and ShapingAPI names), with their schema fallbacks. */
const char *const CAMERA_ATTRIBUTES[] = {
    "projection", "focalLength", "horizontalAperture", "verticalAperture",
    "horizontalApertureOffset", "verticalApertureOffset", "clippingRange", "fStop",
    "focusDistance", nullptr};
const char *const LIGHT_ATTRIBUTES[] = {
    "inputs:intensity", "inputs:exposure", "inputs:color", "inputs:enableColorTemperature",
    "inputs:colorTemperature", "inputs:normalize", "inputs:diffuse", "inputs:specular",
    "inputs:radius", "inputs:width", "inputs:height", "inputs:length", "inputs:angle",
    "inputs:shaping:cone:angle", "inputs:shaping:cone:softness", "inputs:shaping:focus",
    "inputs:texture:format", "treatAsPoint", "treatAsLine", nullptr};

/* {"name": value...} for the attributes in `names` that have a value
 * (authored or the schema's fallback): numbers, bools and tokens as JSON
 * values, vectors as arrays. */
std::string attributes_json(const UsdPrim &prim, const char *const *names)
{
  std::string s = "{";
  for (; *names != nullptr; names++) {
    UsdAttribute a = prim.GetAttribute(TfToken(*names));
    VtValue v;
    if (!a || !a.Get(&v, TIME) || v.IsEmpty()) {
      continue;
    }
    std::string value;
    if (v.IsHolding<float>()) {
      value = num(v.UncheckedGet<float>());
    }
    else if (v.IsHolding<double>()) {
      value = num(v.UncheckedGet<double>());
    }
    else if (v.IsHolding<bool>()) {
      value = v.UncheckedGet<bool>() ? "true" : "false";
    }
    else if (v.IsHolding<TfToken>()) {
      value = json_string(v.UncheckedGet<TfToken>().GetString());
    }
    else if (v.IsHolding<GfVec2f>()) {
      const GfVec2f &e = v.UncheckedGet<GfVec2f>();
      value = "[" + num(e[0]) + "," + num(e[1]) + "]";
    }
    else if (v.IsHolding<GfVec3f>()) {
      const GfVec3f &e = v.UncheckedGet<GfVec3f>();
      value = "[" + num(e[0]) + "," + num(e[1]) + "," + num(e[2]) + "]";
    }
    else {
      continue;
    }
    s += (s.size() > 1 ? "," : "") + json_string(*names) + ":" + value;
  }
  return s + "}";
}

/* One material's shading network: every shader the surface outputs reach,
 * with inputs as values or connections to another shader's output. */
struct Network {
  UsdShadeMaterial material;
  std::map<SdfPath, std::string> names;
  std::vector<UsdShadeShader> order;

  std::string name_of(const UsdShadeShader &s)
  {
    auto it = names.find(s.GetPath());
    if (it != names.end()) {
      return it->second;
    }
    /* Unique within the material: its path below the material. */
    std::string n = s.GetPath().MakeRelativePath(material.GetPath()).GetString();
    for (char &c : n) {
      if (c == '/' || c == '.') {
        c = '_';
      }
    }
    names[s.GetPath()] = n;
    order.push_back(s);
    return n;
  }

  /* The shader output `attr` resolves to: (shader, output name). */
  bool source_of(const UsdAttribute &attr, UsdShadeShader &shader, std::string &output, UsdAttribute &value)
  {
    const UsdShadeAttributeVector sources =
        UsdShadeUtils::GetType(attr.GetName()) == UsdShadeAttributeType::Output ?
            UsdShadeUtils::GetValueProducingAttributes(UsdShadeOutput(attr)) :
            UsdShadeUtils::GetValueProducingAttributes(UsdShadeInput(attr));
    for (const UsdAttribute &s : sources) {
      UsdShadeShader sh(s.GetPrim());
      if (UsdShadeUtils::GetType(s.GetName()) == UsdShadeAttributeType::Output && sh) {
        shader = sh;
        output = UsdShadeUtils::GetBaseNameAndType(s.GetName()).first.GetString();
        return true;
      }
      value = s;
    }
    return false;
  }

  /* The terminal (outputs:surface or outputs:mtlx:surface): the shader
   * name it reaches, or "". */
  std::string terminal(const char *output)
  {
    UsdShadeOutput o = material.GetOutput(TfToken(output));
    if (!o) {
      return "";
    }
    UsdShadeShader s;
    std::string out;
    UsdAttribute value;
    if (!source_of(o.GetAttr(), s, out, value)) {
      return "";
    }
    return name_of(s);
  }

  std::string json(Out &)
  {
    std::string s = "[";
    for (size_t i = 0; i < order.size(); i++) {
      UsdShadeShader sh = order[i];
      TfToken id;
      sh.GetShaderId(&id);
      s += (i ? "," : "") + std::string("{\"name\":") + json_string(name_of(sh)) +
           ",\"id\":" + json_string(id.GetString()) + ",\"colorspace\":" +
           json_string(colour_space(sh.GetPrim(), UsdAttribute())) + ",\"inputs\":[";
      bool first = true;
      for (const UsdShadeInput &in : sh.GetInputs()) {
        UsdShadeShader src;
        std::string output;
        UsdAttribute value;
        std::string entry = "{\"name\":" + json_string(in.GetBaseName().GetString()) + ",\"type\":" +
                            json_string(in.GetTypeName().GetAsToken().GetString());
        if (source_of(in.GetAttr(), src, output, value)) {
          entry += ",\"node\":" + json_string(name_of(src)) + ",\"output\":" + json_string(output);
        }
        else {
          const std::string v = value_json(value ? value : in.GetAttr());
          if (v.empty()) {
            continue;
          }
          entry += v;
        }
        const std::string space = colour_space(sh.GetPrim(), in.GetAttr());
        if (in.GetAttr().HasColorSpace()) {
          entry += ",\"colorspace\":" + json_string(space);
        }
        s += (first ? "" : ",") + entry + "}";
        first = false;
      }
      s += "],\"outputs\":[";
      first = true;
      for (const UsdShadeOutput &o : sh.GetOutputs()) {
        s += (first ? "" : ",") + std::string("{\"name\":") + json_string(o.GetBaseName().GetString()) +
             ",\"type\":" + json_string(o.GetTypeName().GetAsToken().GetString()) + "}";
        first = false;
      }
      s += "]}";
    }
    return s + "]";
  }
};

/* A material's MaterialX network as it is (Operandi writes it so, with
 * customData operandi:materialxGraph): every Shader (but the Preview
 * Surface's Usd* ones) and NodeGraph below the material, by its path
 * relative to the material, with direct connections rather than resolved
 * ones, so NodeGraphs, their interfaces and nodes that feed nothing all
 * come back. */
std::string graph_json(const UsdShadeMaterial &material)
{
  const SdfPath root = material.GetPath();
  auto rel = [&](const SdfPath &p) { return p.MakeRelativePath(root).GetString(); };
  /* `attr`'s one direct source: {"path","kind":"output"|"input","name"}. */
  auto source = [&](const UsdAttribute &attr) -> std::string {
    UsdShadeSourceInfoVector infos = UsdShadeConnectableAPI::GetConnectedSources(attr);
    if (infos.empty()) {
      return "";
    }
    const UsdShadeConnectionSourceInfo &s = infos[0];
    return ",\"source\":{\"path\":" + json_string(rel(s.source.GetPath())) + ",\"kind\":" +
           json_string(s.sourceType == UsdShadeAttributeType::Output ? "output" : "input") +
           ",\"name\":" + json_string(s.sourceName.GetString()) + "}";
  };
  std::string s = "[";
  bool first_prim = true;
  for (const UsdPrim &prim : UsdPrimRange(material.GetPrim())) {
    if (prim == material.GetPrim()) {
      continue;
    }
    const bool graph = prim.IsA<UsdShadeNodeGraph>() && !prim.IsA<UsdShadeMaterial>();
    UsdShadeShader shader(prim);
    TfToken id;
    if (!graph && !(shader && shader.GetShaderId(&id))) {
      continue;
    }
    if (TfStringStartsWith(id.GetString(), "Usd")) {
      continue;
    }
    UsdShadeConnectableAPI c(prim);
    std::string e = std::string(first_prim ? "" : ",") + "{\"path\":" + json_string(rel(prim.GetPath())) +
                    ",\"graph\":" + (graph ? "true" : "false") + ",\"id\":" + json_string(id.GetString()) +
                    ",\"colorspace\":" + json_string(colour_space(prim, UsdAttribute()));
    UsdAttribute pos = prim.GetAttribute(TfToken("ui:nodegraph:node:pos"));
    GfVec2f p;
    if (pos && pos.Get(&p)) {
      e += ",\"pos\":[" + num(p[0]) + "," + num(p[1]) + "]";
    }
    e += ",\"inputs\":[";
    bool first = true;
    for (const UsdShadeInput &in : c.GetInputs()) {
      std::string entry = "{\"name\":" + json_string(in.GetBaseName().GetString()) + ",\"type\":" +
                          json_string(in.GetTypeName().GetAsToken().GetString());
      const std::string src = source(in.GetAttr());
      if (!src.empty()) {
        entry += src;
      }
      else {
        const std::string v = value_json(in.GetAttr());
        if (v.empty()) {
          continue;
        }
        entry += v;
      }
      if (in.GetAttr().HasColorSpace()) {
        entry += ",\"colorspace\":" + json_string(in.GetAttr().GetColorSpace().GetString());
      }
      e += (first ? "" : ",") + entry + "}";
      first = false;
    }
    e += "],\"outputs\":[";
    first = true;
    for (const UsdShadeOutput &o : c.GetOutputs()) {
      e += (first ? "" : ",") + std::string("{\"name\":") + json_string(o.GetBaseName().GetString()) +
           ",\"type\":" + json_string(o.GetTypeName().GetAsToken().GetString()) + source(o.GetAttr()) +
           "}";
      first = false;
    }
    s += e + "]}";
    first_prim = false;
  }
  s += "]";
  std::string terminal;
  UsdShadeOutput out = material.GetOutput(TfToken("mtlx:surface"));
  if (out) {
    UsdShadeSourceInfoVector infos = UsdShadeConnectableAPI::GetConnectedSources(out);
    if (!infos.empty()) {
      terminal = rel(infos[0].source.GetPath());
    }
  }
  return ",\"graph\":" + s + ",\"graph_terminal\":" + json_string(terminal);
}

struct Walker {
  UsdStageRefPtr stage;
  Out out;
  std::vector<std::string> prims;
  std::vector<std::string> meshes;
  std::map<SdfPath, int> mesh_of;
  std::vector<std::string> materials;
  std::map<SdfPath, int> material_of;
  std::vector<std::string> skipped;
  /* UsdSkelSkeletons (BEHAVIOR-RIG-PHASE5 §7). */
  std::vector<std::string> skeletons;

  static std::string matrix_json(const GfMatrix4d &m)
  {
    std::string o = "[";
    for (int i = 0; i < 16; i++) {
      o += (i ? "," : "") + num(m[i / 4][i % 4]);
    }
    return o + "]";
  }

  /* A Skeleton: joints, bind and rest transforms, its world, and its
   * SkelAnimation's samples. Operandi's own code. */
  std::string skeleton_json(const UsdPrim &prim)
  {
    UsdSkelSkeleton skel(prim);
    VtTokenArray joints;
    skel.GetJointsAttr().Get(&joints);
    VtMatrix4dArray bind, rest;
    skel.GetBindTransformsAttr().Get(&bind);
    skel.GetRestTransformsAttr().Get(&rest);
    std::string j = "[", b = "[", r = "[";
    for (size_t i = 0; i < joints.size(); i++) {
      j += (i ? "," : "") + json_string(joints[i].GetString());
    }
    for (size_t i = 0; i < bind.size(); i++) {
      b += (i ? "," : "") + matrix_json(bind[i]);
    }
    for (size_t i = 0; i < rest.size(); i++) {
      r += (i ? "," : "") + matrix_json(rest[i]);
    }
    std::string s = "{\"path\":" + json_string(prim.GetPath().GetString()) + ",\"joints\":" + j +
                    "],\"bind\":" + b + "],\"rest\":" + r + "],\"world\":" +
                    matrix_json(UsdGeomXformable(prim).ComputeLocalToWorldTransform(TIME));
    SdfPathVector sources;
    UsdSkelBindingAPI(prim).GetAnimationSourceRel().GetForwardedTargets(&sources);
    UsdSkelAnimation anim(sources.empty() ? UsdPrim() : prim.GetStage()->GetPrimAtPath(sources[0]));
    if (anim) {
      VtTokenArray order;
      anim.GetJointsAttr().Get(&order);
      std::vector<double> times;
      anim.GetRotationsAttr().GetTimeSamples(&times);
      if (times.empty()) {
        times.push_back(UsdTimeCode::Default().GetValue());
      }
      std::string o = "[";
      for (size_t i = 0; i < order.size(); i++) {
        o += (i ? "," : "") + json_string(order[i].GetString());
      }
      std::string t = "[", tr = "[", ro = "[", sc = "[";
      for (size_t k = 0; k < times.size(); k++) {
        const UsdTimeCode code = std::isnan(times[k]) ? UsdTimeCode::Default() : UsdTimeCode(times[k]);
        VtVec3fArray tv;
        VtQuatfArray rv;
        VtVec3hArray sv;
        anim.GetTranslationsAttr().Get(&tv, code);
        anim.GetRotationsAttr().Get(&rv, code);
        anim.GetScalesAttr().Get(&sv, code);
        t += (k ? "," : "") + num(std::isnan(times[k]) ? 0.0 : times[k]);
        std::string a = "[", q = "[", c = "[";
        for (size_t i = 0; i < tv.size(); i++) {
          a += (i ? "," : "") + std::string("[") + num(tv[i][0]) + "," + num(tv[i][1]) + "," + num(tv[i][2]) + "]";
        }
        for (size_t i = 0; i < rv.size(); i++) {
          const GfVec3f im = rv[i].GetImaginary();
          q += (i ? "," : "") + std::string("[") + num(rv[i].GetReal()) + "," + num(im[0]) + "," + num(im[1]) + "," + num(im[2]) + "]";
        }
        for (size_t i = 0; i < sv.size(); i++) {
          c += (i ? "," : "") + std::string("[") + num(float(sv[i][0])) + "," + num(float(sv[i][1])) + "," + num(float(sv[i][2])) + "]";
        }
        tr += (k ? "," : "") + a + "]";
        ro += (k ? "," : "") + q + "]";
        sc += (k ? "," : "") + c + "]";
      }
      s += ",\"anim\":{\"joints\":" + o + "],\"times\":" + t + "],\"translations\":" + tr +
           "],\"rotations\":" + ro + "],\"scales\":" + sc + "]}";
    }
    return s + "}";
  }

  int material_index(const UsdShadeMaterial &m)
  {
    if (!m) {
      return -1;
    }
    auto it = material_of.find(m.GetPath());
    if (it != material_of.end()) {
      return it->second;
    }
    Network net{m, {}, {}};
    const std::string surface = net.terminal("surface");
    const std::string mtlx = net.terminal("mtlx:surface");
    std::string s = "{\"path\":" + json_string(m.GetPath().GetString()) +
                    ",\"name\":" + json_string(m.GetPrim().GetName().GetString()) +
                    ",\"surface\":" + json_string(surface) + ",\"mtlx\":" + json_string(mtlx) +
                    ",\"nodes\":" + net.json(out);
    bool whole = false;
    VtValue mark = m.GetPrim().GetCustomDataByKey(TfToken("operandi:materialxGraph"));
    if (mark.IsHolding<bool>() && mark.UncheckedGet<bool>()) {
      whole = true;
    }
    if (whole) {
      s += graph_json(m);
    }
    s += "}";
    materials.push_back(s);
    const int index = int(materials.size() - 1);
    material_of[m.GetPath()] = index;
    return index;
  }

  /* The mesh data of `mesh`, shared by every instance of one prototype. */
  int mesh_index(const UsdGeomMesh &mesh)
  {
    const UsdPrim prim = mesh.GetPrim();
    const SdfPath key = prim.IsInstanceProxy() ? prim.GetPrimInPrototype().GetPath() :
                                                 prim.GetPath();
    auto it = mesh_of.find(key);
    if (it != mesh_of.end()) {
      return it->second;
    }
    VtVec3fArray points;
    VtIntArray counts, indices;
    mesh.GetPointsAttr().Get(&points, TIME);
    mesh.GetFaceVertexCountsAttr().Get(&counts, TIME);
    mesh.GetFaceVertexIndicesAttr().Get(&indices, TIME);
    TfToken orientation, scheme, boundary, fvli;
    mesh.GetOrientationAttr().Get(&orientation);
    mesh.GetSubdivisionSchemeAttr().Get(&scheme);
    mesh.GetInterpolateBoundaryAttr().Get(&boundary);
    mesh.GetFaceVaryingLinearInterpolationAttr().Get(&fvli);
    std::string s = "{\"points\":" + out.buffer(points.cdata(), points.size(), "f32x3") +
                    ",\"counts\":" + out.ints(counts) + ",\"indices\":" + out.ints(indices) +
                    ",\"orientation\":" + json_string(orientation.GetString()) +
                    ",\"scheme\":" + json_string(scheme.GetString()) +
                    ",\"boundary\":" + json_string(boundary.GetString()) +
                    ",\"fvli\":" + json_string(fvli.GetString());
    VtIntArray a;
    VtFloatArray f;
    auto ints = [&](const UsdAttribute &attr, const char *key) {
      a.clear();
      if (attr && attr.Get(&a, TIME) && !a.empty()) {
        s += std::string(",\"") + key + "\":" + out.ints(a);
      }
    };
    auto floats = [&](const UsdAttribute &attr, const char *key) {
      f.clear();
      if (attr && attr.Get(&f, TIME) && !f.empty()) {
        s += std::string(",\"") + key + "\":" + out.floats(std::vector<float>(f.begin(), f.end()));
      }
    };
    ints(mesh.GetCreaseIndicesAttr(), "creaseIndices");
    ints(mesh.GetCreaseLengthsAttr(), "creaseLengths");
    floats(mesh.GetCreaseSharpnessesAttr(), "creaseSharpnesses");
    ints(mesh.GetCornerIndicesAttr(), "cornerIndices");
    floats(mesh.GetCornerSharpnessesAttr(), "cornerSharpnesses");
    ints(mesh.GetHoleIndicesAttr(), "holes");

    std::vector<float> values;
    int dims = 0;
    /* Normals: primvars:normals wins over the normals attribute. */
    UsdGeomPrimvarsAPI primvars(prim);
    UsdGeomPrimvar pn = primvars.GetPrimvar(TfToken("normals"));
    VtValue nv;
    if (pn && pn.HasValue() && pn.Get(&nv, TIME) && flat_floats(nv, values, dims) && dims == 3) {
      VtIntArray idx;
      pn.GetIndices(&idx, TIME);
      s += ",\"normals\":{\"interpolation\":" + json_string(pn.GetInterpolation().GetString()) +
           ",\"values\":" + out.floats(values) + (idx.empty() ? "" : ",\"indices\":" + out.ints(idx)) +
           "}";
    }
    else {
      VtVec3fArray normals;
      if (mesh.GetNormalsAttr().Get(&normals, TIME) && !normals.empty()) {
        values.clear();
        for (const GfVec3f &e : normals) {
          values.insert(values.end(), {e[0], e[1], e[2]});
        }
        s += ",\"normals\":{\"interpolation\":" + json_string(mesh.GetNormalsInterpolation().GetString()) +
             ",\"values\":" + out.floats(values) + "}";
      }
    }
    s += ",\"primvars\":[";
    bool first = true;
    for (const UsdGeomPrimvar &pv : primvars.GetPrimvarsWithValues()) {
      const std::string name = pv.GetPrimvarName().GetString();
      /* Maps Operandi wrote say so in their customData; they come in
       * whatever their type, with their unauthored index. */
      const std::string custom = operandi_custom(pv.GetAttr());
      if (name == "normals" || (pv.GetElementSize() != 1 && custom.empty())) {
        continue;
      }
      VtValue v;
      if (!pv.Get(&v, TIME) || !flat_floats(v, values, dims)) {
        continue;
      }
      const SdfValueTypeName type = pv.GetTypeName();
      const TfToken role = type.GetRole();
      std::string kind;
      if (!custom.empty()) {
        kind = "operandi";
        dims *= std::max(1, pv.GetElementSize());
      }
      else if (role == SdfValueRoleNames->TextureCoordinate ||
               (dims == 2 && type.IsArray() && role.IsEmpty() && uv_by_name(name)))
      {
        kind = "uv";
      }
      else if (role == SdfValueRoleNames->Color && (dims == 3 || dims == 4)) {
        kind = "color";
      }
      else if (name == "displayOpacity" && dims == 1) {
        kind = "opacity";
      }
      else if (TfStringStartsWith(name, "skel:") || !type.IsArray() || pv.GetElementSize() != 1) {
        /* Skinning weights belong to a skeleton Operandi doesn't read. */
        continue;
      }
      else {
        /* Another program's map (Blender's vertex groups and attributes,
         * Houdini's point and primitive attributes): its type and role
         * say which Operandi map it becomes (import_extras.rs). */
        kind = "foreign";
      }
      VtIntArray idx;
      pv.GetIndices(&idx, TIME);
      s += (first ? "" : ",") + std::string("{\"name\":") + json_string(name) + ",\"kind\":" +
           json_string(kind) + ",\"dims\":" + std::to_string(dims) +
           ",\"type\":" + json_string(type.GetScalarType().GetAsToken().GetString()) +
           ",\"role\":" + json_string(role.GetString()) +
           ",\"interpolation\":" + json_string(pv.GetInterpolation().GetString()) +
           ",\"values\":" + out.floats(values) +
           (idx.empty() ? "" : ",\"indices\":" + out.ints(idx)) +
           (!custom.empty() && pv.GetUnauthoredValuesIndex() >= 0 ?
                ",\"unauthored\":" + std::to_string(pv.GetUnauthoredValuesIndex()) :
                "") +
           custom + "}";
      first = false;
    }
    s += "]";
    /* Parts and selection sets: subsets of Operandi's own families, and
     * any other program's (Houdini groups...) except material bindings,
     * which `walk` reads. */
    std::string groups;
    for (const UsdGeomSubset &sub : UsdGeomSubset::GetAllGeomSubsets(mesh)) {
      TfToken family, element;
      sub.GetFamilyNameAttr().Get(&family);
      if (family == UsdShadeTokens->materialBind) {
        continue;
      }
      sub.GetElementTypeAttr().Get(&element);
      VtIntArray members;
      sub.GetIndicesAttr().Get(&members, TIME);
      groups += (groups.empty() ? "" : ",") + std::string("{\"name\":") +
                json_string(sub.GetPrim().GetName().GetString()) +
                ",\"family\":" + json_string(family.GetString()) +
                ",\"elementType\":" + json_string(element.GetString()) +
                ",\"indices\":" + out.ints(members) + operandi_custom(sub.GetPrim()) + "}";
    }
    s += ",\"groups\":[" + groups + "]";
    /* Blend shapes bound to the mesh (UsdSkelBindingAPI). */
    std::string shapes;
    UsdSkelBindingAPI skel(prim);
    if (prim.HasAPI<UsdSkelBindingAPI>()) {
      VtTokenArray names;
      skel.GetBlendShapesAttr().Get(&names);
      SdfPathVector targets;
      skel.GetBlendShapeTargetsRel().GetForwardedTargets(&targets);
      for (size_t i = 0; i < targets.size(); i++) {
        UsdSkelBlendShape shape(prim.GetStage()->GetPrimAtPath(targets[i]));
        VtVec3fArray offsets;
        if (!shape || !shape.GetOffsetsAttr().Get(&offsets)) {
          continue;
        }
        VtIntArray point_indices;
        shape.GetPointIndicesAttr().Get(&point_indices);
        const std::string name = i < names.size() ? names[i].GetString() :
                                                    shape.GetPrim().GetName().GetString();
        shapes += (shapes.empty() ? "" : ",") + std::string("{\"name\":") + json_string(name) +
                  ",\"offsets\":" + out.buffer(offsets.cdata(), offsets.size(), "f32x3") +
                  (point_indices.empty() ? "" : ",\"pointIndices\":" + out.ints(point_indices)) +
                  operandi_custom(shape.GetPrim()) + "}";
      }
    }
    s += ",\"blendShapes\":[" + shapes + "]";
    /* Skinning (UsdSkelBindingAPI, §7). */
    if (prim.HasAPI<UsdSkelBindingAPI>()) {
      const UsdSkelSkeleton target = skel.GetInheritedSkeleton();
      VtIntArray joint_indices;
      VtFloatArray joint_weights;
      const UsdGeomPrimvar ip = skel.GetJointIndicesPrimvar();
      const UsdGeomPrimvar wp = skel.GetJointWeightsPrimvar();
      if (target && ip && wp && ip.Get(&joint_indices, TIME) && wp.Get(&joint_weights, TIME)) {
        GfMatrix4d geom(1.0);
        skel.GetGeomBindTransformAttr().Get(&geom);
        TfToken method;
        skel.GetSkinningMethodAttr().Get(&method);
        std::vector<float> w(joint_weights.begin(), joint_weights.end());
        VtTokenArray order;
        skel.GetJointsAttr().Get(&order);
        std::string o = "[";
        for (size_t i = 0; i < order.size(); i++) {
          o += (i ? "," : "") + json_string(order[i].GetString());
        }
        s += ",\"skin\":{\"skeleton\":" + json_string(target.GetPath().GetString()) +
             ",\"elementSize\":" + std::to_string(ip.GetElementSize()) +
             ",\"indices\":" + out.ints(joint_indices) + ",\"weights\":" + out.floats(w) +
             ",\"geomBind\":" + matrix_json(geom) + ",\"method\":" +
             json_string(method.IsEmpty() ? "classicLinear" : method.GetString()) +
             ",\"joints\":" + o + "]}";
      }
    }
    s += operandi_custom(prim) + "}";
    meshes.push_back(s);
    const int index = int(meshes.size() - 1);
    mesh_of[key] = index;
    return index;
  }

  /* A BasisCurves or NurbsCurves prim as a "mesh" with no polygons and a
   * "curves" description (import_curves.rs), once per prim like meshes. */
  int curves_index(const UsdGeomCurves &curves)
  {
    const UsdPrim prim = curves.GetPrim();
    const SdfPath key = prim.IsInstanceProxy() ? prim.GetPrimInPrototype().GetPath() :
                                                 prim.GetPath();
    auto it = mesh_of.find(key);
    if (it != mesh_of.end()) {
      return it->second;
    }
    VtVec3fArray points;
    VtIntArray counts;
    curves.GetPointsAttr().Get(&points, TIME);
    curves.GetCurveVertexCountsAttr().Get(&counts, TIME);
    const VtIntArray none;
    std::string c = "{\"schema\":" + json_string(prim.GetTypeName().GetString()) +
                    ",\"vertexCounts\":" + out.ints(counts);
    VtFloatArray widths;
    if (curves.GetWidthsAttr().Get(&widths, TIME) && !widths.empty()) {
      c += ",\"widths\":" + out.floats(std::vector<float>(widths.begin(), widths.end())) +
           ",\"widthsInterpolation\":" + json_string(curves.GetWidthsInterpolation().GetString());
    }
    auto doubles = [&](const VtDoubleArray &a, const char *name) {
      if (!a.empty()) {
        std::vector<double> v(a.begin(), a.end());
        c += std::string(",\"") + name + "\":" + out.buffer(v.data(), v.size(), "f64");
      }
    };
    if (UsdGeomBasisCurves basis = UsdGeomBasisCurves(prim)) {
      TfToken type, b, wrap;
      basis.GetTypeAttr().Get(&type);
      basis.GetBasisAttr().Get(&b);
      basis.GetWrapAttr().Get(&wrap);
      c += ",\"type\":" + json_string(type.GetString()) + ",\"basis\":" +
           json_string(b.GetString()) + ",\"wrap\":" + json_string(wrap.GetString());
    }
    else if (UsdGeomNurbsCurves nurbs = UsdGeomNurbsCurves(prim)) {
      VtIntArray order;
      VtDoubleArray knots, weights;
      VtVec2dArray ranges;
      nurbs.GetOrderAttr().Get(&order, TIME);
      nurbs.GetKnotsAttr().Get(&knots, TIME);
      nurbs.GetPointWeightsAttr().Get(&weights, TIME);
      nurbs.GetRangesAttr().Get(&ranges, TIME);
      c += ",\"order\":" + out.ints(order);
      doubles(knots, "knots");
      doubles(weights, "pointWeights");
      VtDoubleArray flat;
      for (const GfVec2d &r : ranges) {
        flat.push_back(r[0]);
        flat.push_back(r[1]);
      }
      doubles(flat, "ranges");
    }
    c += operandi_custom(prim) + "}";
    std::string s = "{\"points\":" + out.buffer(points.cdata(), points.size(), "f32x3") +
                    ",\"counts\":" + out.ints(none) + ",\"indices\":" + out.ints(none) +
                    ",\"orientation\":\"rightHanded\",\"scheme\":\"none\",\"boundary\":\"\"," +
                    "\"fvli\":\"\",\"primvars\":[],\"groups\":[],\"blendShapes\":[],\"curves\":" + c +
                    "}";
    meshes.push_back(s);
    const int index = int(meshes.size() - 1);
    mesh_of[key] = index;
    return index;
  }

  /* Adds `prim` (and, unless it's skipped, what's below it) to the prim
   * list. `proto_of` marks a PointInstancer's prototype subtree. */
  void walk(const UsdPrim &prim, int parent, int proto_of)
  {
    if (!prim.IsActive() || prim.IsAbstract() || !prim.IsDefined()) {
      return;
    }
    const std::string type = prim.GetTypeName().GetString();
    if (prim.IsA<UsdShadeMaterial>() || prim.IsA<UsdShadeShader>() ||
        prim.IsA<UsdShadeNodeGraph>() || prim.IsA<UsdGeomSubset>() ||
        prim.IsA<UsdSkelBlendShape>() || prim.GetTypeName() == TfToken("RenderSettings") ||
        prim.GetTypeName() == TfToken("RenderProduct") || prim.GetTypeName() == TfToken("RenderVar"))
    {
      /* Render prims are read on their own (render_settings_json). */
      return;
    }
    if (prim.IsA<UsdSkelSkeleton>()) {
      /* Read as joints (import_skel.rs), not as a transform. */
      skeletons.push_back(skeleton_json(prim));
      return;
    }
    UsdGeomImageable img(prim);
    std::string purpose = "default";
    std::string visible = "true";
    if (img) {
      TfToken p, v;
      img.GetPurposeAttr().Get(&p);
      if (!p.IsEmpty()) {
        purpose = p.GetString();
      }
      img.GetVisibilityAttr().Get(&v, TIME);
      visible = v == UsdGeomTokens->invisible ? "false" : "true";
    }
    std::string s = "{\"path\":" + json_string(prim.GetPath().GetString()) + ",\"name\":" +
                    json_string(prim.GetName().GetString()) + ",\"type\":" + json_string(type) +
                    ",\"parent\":" + std::to_string(parent) + ",\"purpose\":" + json_string(purpose) +
                    ",\"visible\":" + visible;
    if (proto_of >= 0) {
      s += ",\"protoOf\":" + std::to_string(proto_of);
    }
    if (prim.IsInstance()) {
      s += ",\"instance\":true";
    }
    /* Variant sets as composed (their selections) and payloads, which
     * UsdStage::Open loads, for the report (usd-gaps-3). */
    if (prim.HasVariantSets()) {
      UsdVariantSets sets = prim.GetVariantSets();
      std::string v;
      for (const std::string &name : sets.GetNames()) {
        v += (v.empty() ? "" : ",") + json_string(name) + ":" +
             json_string(sets.GetVariantSelection(name));
      }
      s += ",\"variants\":{" + v + "}";
    }
    if (prim.HasAuthoredPayloads()) {
      s += ",\"payload\":true";
    }
    s += operandi_custom(prim);
    UsdGeomXformable xf(prim);
    if (xf) {
      GfMatrix4d m;
      bool resets = false;
      xf.GetLocalTransformation(&m, &resets, TIME);
      s += ",\"local\":" + matrix_json(m) + (resets ? ",\"resetsXform\":true" : "");
      const std::string trs = trs_json(xf);
      if (!trs.empty()) {
        s += ",\"trs\":" + trs;
      }
    }
    if (prim.IsA<UsdGeomCamera>()) {
      s += ",\"camera\":" + attributes_json(prim, CAMERA_ATTRIBUTES);
    }
    if (prim.HasAPI<UsdLuxLightAPI>()) {
      std::string light = attributes_json(prim, LIGHT_ATTRIBUTES);
      /* The dome's picture: written path, resolved path and anchored path,
       * like a shader's asset input. */
      UsdAttribute file = prim.GetAttribute(TfToken("inputs:texture:file"));
      const std::string picture = file ? value_json(file) : "";
      if (!picture.empty()) {
        light.pop_back();
        light += std::string(light.size() > 1 ? "," : "") + "\"texture\":{" + picture.substr(1) + "}}";
      }
      /* ShapingAPI's IES profile, the same way. */
      UsdAttribute ies = prim.GetAttribute(TfToken("inputs:shaping:ies:file"));
      const std::string profile = ies ? value_json(ies) : "";
      if (!profile.empty()) {
        light.pop_back();
        light += std::string(light.size() > 1 ? "," : "") + "\"ies\":{" + profile.substr(1) + "}}";
      }
      s += ",\"light\":" + light;
    }
    UsdGeomMesh mesh(prim);
    if (mesh) {
      s += ",\"mesh\":" + std::to_string(mesh_index(mesh));
      UsdShadeMaterialBindingAPI binding(prim);
      s += ",\"material\":" + std::to_string(material_index(binding.ComputeBoundMaterial()));
      std::string subsets;
      for (const UsdGeomSubset &sub : binding.GetMaterialBindSubsets()) {
        VtIntArray faces;
        sub.GetIndicesAttr().Get(&faces, TIME);
        const int m = material_index(UsdShadeMaterialBindingAPI(sub.GetPrim()).ComputeBoundMaterial());
        subsets += (subsets.empty() ? "" : ",") + std::string("{\"name\":") +
                   json_string(sub.GetPrim().GetName().GetString()) + ",\"faces\":" + out.ints(faces) +
                   ",\"material\":" + std::to_string(m) + "}";
      }
      s += ",\"subsets\":[" + subsets + "]";
    }
    else if (UsdGeomCurves curves = UsdGeomCurves(prim)) {
      s += ",\"mesh\":" + std::to_string(curves_index(curves));
      UsdShadeMaterialBindingAPI binding(prim);
      s += ",\"material\":" + std::to_string(material_index(binding.ComputeBoundMaterial())) +
           ",\"subsets\":[]";
    }
    const int index = int(prims.size());
    UsdGeomPointInstancer instancer(prim);
    prims.push_back("");
    if (instancer) {
      /* The prototypes go in once each, marked; the Rust side copies them
       * per instance. Nothing else below the instancer is drawn. */
      SdfPathVector targets;
      instancer.GetPrototypesRel().GetForwardedTargets(&targets);
      VtIntArray proto_indices;
      instancer.GetProtoIndicesAttr().Get(&proto_indices, TIME);
      VtMatrix4dArray xforms;
      /* Every instance's transform, masked or not, so they line up with
       * protoIndices; the mask comes separately ("hidden", "inactive"). */
      instancer.ComputeInstanceTransformsAtTime(&xforms, TIME, TIME,
                                                UsdGeomPointInstancer::ExcludeProtoXform,
                                                UsdGeomPointInstancer::IgnoreMask);
      std::vector<double> flat;
      for (const GfMatrix4d &m : xforms) {
        flat.insert(flat.end(), m.data(), m.data() + 16);
      }
      std::string protos;
      for (const SdfPath &t : targets) {
        const UsdPrim p = stage->GetPrimAtPath(t);
        if (!p) {
          continue;
        }
        protos += (protos.empty() ? "" : ",") + std::to_string(prims.size());
        walk(p, -1, index);
      }
      /* Instances hidden by invisibleIds, by their index. */
      VtInt64Array ids, invisible;
      instancer.GetIdsAttr().Get(&ids, TIME);
      instancer.GetInvisibleIdsAttr().Get(&invisible, TIME);
      VtIntArray hidden, inactive;
      const std::set<int64_t> off(invisible.begin(), invisible.end());
      const std::vector<bool> mask = instancer.ComputeMaskAtTime(TIME);
      for (size_t i = 0; i < proto_indices.size(); i++) {
        if (off.count(i < ids.size() ? ids[i] : int64_t(i))) {
          hidden.push_back(int(i));
        }
        else if (i < mask.size() && !mask[i]) {
          /* Masked by inactiveIds: not there at all. */
          inactive.push_back(int(i));
        }
      }
      s += ",\"instancer\":{\"prototypes\":[" + protos + "],\"protoIndices\":" +
           out.ints(proto_indices) + ",\"matrices\":" + out.buffer(flat.data(), flat.size(), "f64") +
           (hidden.empty() ? "" : ",\"hidden\":" + out.ints(hidden)) +
           (inactive.empty() ? "" : ",\"inactive\":" + out.ints(inactive)) + "}";
      prims[index] = s + "}";
      return;
    }
    if (!xf && !prim.IsPseudoRoot() && type != "Scope" && !type.empty()) {
      /* Not geometry and not a transform: say what was left out. */
      skipped.push_back("{\"path\":" + json_string(prim.GetPath().GetString()) + ",\"type\":" + json_string(type) + "}");
      prims.pop_back();
      return;
    }
    prims[index] = s + "}";
    for (const UsdPrim &child : prim.GetFilteredChildren(UsdTraverseInstanceProxies())) {
      walk(child, index, proto_of);
    }
  }

  /* The camera the stage renders through (USD-IQ21, provisional): the
   * `camera` relationship of the RenderSettings prim the stage's
   * renderSettingsPrimPath metadata names, else of the first
   * RenderSettings prim that has one. "" when the stage names none. */
  std::string render_camera()
  {
    auto camera_of = [](const UsdPrim &p) -> std::string {
      if (!p || p.GetTypeName() != TfToken("RenderSettings")) {
        return "";
      }
      SdfPathVector targets;
      p.GetRelationship(TfToken("camera")).GetForwardedTargets(&targets);
      return targets.empty() ? "" : targets.front().GetString();
    };
    std::string named;
    if (stage->GetMetadata(TfToken("renderSettingsPrimPath"), &named) && !named.empty() &&
        SdfPath::IsValidPathString(named))
    {
      const std::string c = camera_of(stage->GetPrimAtPath(SdfPath(named)));
      if (!c.empty()) {
        return c;
      }
    }
    for (const UsdPrim &p : stage->Traverse()) {
      const std::string c = camera_of(p);
      if (!c.empty()) {
        return c;
      }
    }
    return "";
  }


  /* The RenderSettings prim the stage renders with: the one the stage's
   * renderSettingsPrimPath names, else the first one. */
  UsdPrim render_settings_prim()
  {
    std::string named;
    if (stage->GetMetadata(TfToken("renderSettingsPrimPath"), &named) && !named.empty() &&
        SdfPath::IsValidPathString(named))
    {
      UsdPrim p = stage->GetPrimAtPath(SdfPath(named));
      if (p && p.GetTypeName() == TfToken("RenderSettings")) {
        return p;
      }
    }
    for (const UsdPrim &p : stage->Traverse()) {
      if (p.GetTypeName() == TfToken("RenderSettings")) {
        return p;
      }
    }
    return UsdPrim();
  }

  /* `,"renderSettings":{...}`: its resolution, pixel aspect, camera,
   * Operandi's customData and its first product's resolution and camera,
   * or "" for none. */
  std::string render_settings_json()
  {
    UsdPrim p = render_settings_prim();
    if (!p) {
      return "";
    }
    auto settings_of = [](const UsdPrim &prim) {
      std::string s;
      GfVec2i res;
      UsdAttribute r = prim.GetAttribute(TfToken("resolution"));
      if (r && r.Get(&res)) {
        s += ",\"resolution\":[" + std::to_string(res[0]) + "," + std::to_string(res[1]) + "]";
      }
      float aspect;
      UsdAttribute a = prim.GetAttribute(TfToken("pixelAspectRatio"));
      if (a && a.Get(&aspect)) {
        s += ",\"pixelAspect\":" + num(aspect);
      }
      SdfPathVector targets;
      prim.GetRelationship(TfToken("camera")).GetForwardedTargets(&targets);
      if (!targets.empty()) {
        s += ",\"camera\":" + json_string(targets.front().GetString());
      }
      return s;
    };
    std::string s = ",\"renderSettings\":{\"path\":" + json_string(p.GetPath().GetString()) +
                    settings_of(p) + operandi_custom(p);
    SdfPathVector products;
    p.GetRelationship(TfToken("products")).GetForwardedTargets(&products);
    if (!products.empty()) {
      UsdPrim product = stage->GetPrimAtPath(products.front());
      if (product) {
        s += ",\"product\":{\"path\":" + json_string(product.GetPath().GetString()) +
             settings_of(product) + "}";
      }
    }
    return s + "}";
  }

  std::string json()
  {
    TfToken up = UsdGeomGetStageUpAxis(stage);
    std::string s = "{\"upAxis\":" + json_string(up.GetString()) + ",\"metersPerUnit\":" +
                    num(UsdGeomGetStageMetersPerUnit(stage)) + ",\"defaultPrim\":" +
                    json_string(stage->GetDefaultPrim() ? stage->GetDefaultPrim().GetPath().GetString() : "");
    auto list = [](const std::vector<std::string> &items) {
      std::string out = "[";
      for (size_t i = 0; i < items.size(); i++) {
        out += (i ? "," : "") + items[i];
      }
      return out + "]";
    };
    s += ",\"renderCamera\":" + json_string(render_camera());
    s += render_settings_json();
    return s + ",\"prims\":" + list(prims) + ",\"meshes\":" + list(meshes) +
           ",\"materials\":" + list(materials) + ",\"skipped\":" + list(skipped) +
           ",\"skeletons\":" + list(skeletons) + "}";
  }
};

}  // namespace

extern "C" {

OuReader *ou_read_open(const char *path, char *error, size_t error_len)
{
  TfErrorMark mark;
  try {
    if (path == nullptr) {
      set_text(error, error_len, "no path");
      return nullptr;
    }
    UsdStageRefPtr stage = UsdStage::Open(std::string(path));
    if (!stage) {
      std::string text;
      for (const TfError &e : mark) {
        text += (text.empty() ? "" : "; ") + e.GetCommentary();
      }
      mark.Clear();
      set_text(error, error_len, text.empty() ? "Couldn't open the USD file" : text);
      return nullptr;
    }
    /* OpenUSD crashes reading time samples of a stage whose
     * timeCodesPerSecond is 0 or negative (usd-wg assets'
     * timeCodesPerSecond_0.usda; Blender's OpenUSD too), so such files are
     * refused here. */
    const double tcps = stage->GetTimeCodesPerSecond();
    if (!(tcps > 0.0) || !std::isfinite(tcps)) {
      set_text(error,
               error_len,
               "The file's timeCodesPerSecond is " + std::to_string(tcps) +
                   "; OpenUSD can't read a stage's animation unless it is positive");
      return nullptr;
    }
    auto *reader = new OuReader();
    Walker walker{stage, Out{reader}, {}, {}, {}, {}, {}, {}};
    for (const UsdPrim &child : stage->GetPseudoRoot().GetFilteredChildren(UsdTraverseInstanceProxies())) {
      walker.walk(child, -1, -1);
    }
    reader->json = walker.json();
    /* Warnings raised while reading (an unresolved asset...) are not
     * failures; the Rust side reports what didn't come across. */
    mark.Clear();
    return reader;
  }
  catch (const std::exception &e) {
    set_text(error, error_len, std::string("Couldn't read the USD file: ") + e.what());
  }
  catch (...) {
    set_text(error, error_len, "Couldn't read the USD file");
  }
  mark.Clear();
  return nullptr;
}

void ou_read_free(OuReader *reader)
{
  delete reader;
}

const char *ou_read_json(const OuReader *reader)
{
  return reader ? reader->json.c_str() : nullptr;
}

size_t ou_read_buffer(const OuReader *reader, size_t index, const void **data)
{
  if (reader == nullptr || index >= reader->buffers.size() || data == nullptr) {
    return 0;
  }
  /* An empty buffer still gets a non-null pointer, so a caller can tell
   * it from "no such buffer". */
  static const char none = 0;
  *data = reader->buffers[index].empty() ? &none : reader->buffers[index].data();
  return reader->buffers[index].size();
}

int ou_copy_asset(const char *resolved, const char *destination, char *error, size_t error_len)
{
  TfErrorMark mark;
  try {
    if (resolved == nullptr || destination == nullptr) {
      set_text(error, error_len, "no asset or destination");
      return 0;
    }
    std::shared_ptr<ArAsset> asset = ArGetResolver().OpenAsset(ArResolvedPath(resolved));
    if (!asset) {
      set_text(error, error_len, std::string("Couldn't open ") + resolved);
      mark.Clear();
      return 0;
    }
    std::shared_ptr<const char> bytes = asset->GetBuffer();
    std::ofstream file(destination, std::ios::binary);
    if (!bytes || !file) {
      set_text(error, error_len, std::string("Couldn't write ") + destination);
      mark.Clear();
      return 0;
    }
    file.write(bytes.get(), std::streamsize(asset->GetSize()));
    file.close();
    mark.Clear();
    return file ? 1 : 0;
  }
  catch (const std::exception &e) {
    set_text(error, error_len, e.what());
  }
  catch (...) {
    set_text(error, error_len, "Couldn't copy the asset");
  }
  mark.Clear();
  return 0;
}

}  /* extern "C" */
