/* Operandi's C bridge to OpenUSD (operandi_usd.h).
 *
 * Stage handling, plugin registration, errors and the read-back helpers
 * are Operandi's own. The prim writers adapted from Blender are in
 * blender/ (GPL-2.0-or-later). OpenUSD is TOST-1.0 (Apache-2.0 with a
 * different trademark section). */
#include "operandi_usd.h"

#include "blender/blender_usd.hh"

#include <pxr/base/js/json.h>
#include <pxr/base/plug/registry.h>
#include <pxr/base/tf/errorMark.h>
#include <pxr/base/tf/stringUtils.h>
#include <pxr/pxr.h>
#include <pxr/usd/sdf/fileFormat.h>
#include <pxr/usd/sdf/layer.h>
#include <pxr/usd/usd/stage.h>
#include <pxr/usd/usdGeom/basisCurves.h>
#include <pxr/usd/usdGeom/mesh.h>
#include <pxr/usd/usdGeom/pointInstancer.h>
#include <pxr/usd/usdGeom/nurbsCurves.h>
#include <pxr/usd/usdGeom/metrics.h>
#include <pxr/usd/usdGeom/primvarsAPI.h>
#include <pxr/usd/usdGeom/subset.h>
#include <pxr/usd/usdSkel/bindingAPI.h>
#include <pxr/usd/usdSkel/blendShape.h>
#include <pxr/usd/usdGeom/tokens.h>
#include <pxr/usd/usdGeom/scope.h>
#include <pxr/usd/usdGeom/xform.h>
#include <pxr/usd/usdGeom/xformable.h>
#include <pxr/usd/usdLux/lightAPI.h>
#include <pxr/usd/usdLux/meshLightAPI.h>
#include <pxr/usd/usdLux/tokens.h>
#include <pxr/usd/usdRender/product.h>
#include <pxr/usd/usdRender/settings.h>
#include <pxr/usd/usdShade/material.h>
#include <pxr/usd/usdShade/materialBindingAPI.h>
#include <pxr/usd/usdShade/nodeGraph.h>
#include <pxr/usd/usdShade/shader.h>
#include <pxr/usd/usdUI/nodeGraphNodeAPI.h>
#include <pxr/usd/usdUtils/usdzPackage.h>

#include <cstdlib>
#include <cstring>
#include <exception>
#include <mutex>
#include <string>

struct OuStage {
  pxr::UsdStageRefPtr stage;
  pxr::SdfLayerRefPtr layer;
};

namespace {

void set_text(char *out, size_t len, const std::string &text)
{
  if (out == nullptr || len == 0) {
    return;
  }
  const size_t n = std::min(len - 1, text.size());
  std::memcpy(out, text.data(), n);
  out[n] = '\0';
}

/* The messages of the USD errors raised since `mark`, or `fallback`. */
std::string usd_errors(const pxr::TfErrorMark &mark, const std::string &fallback)
{
  std::string text;
  for (const pxr::TfError &e : mark) {
    if (!text.empty()) {
      text += "; ";
    }
    text += e.GetCommentary();
  }
  return text.empty() ? fallback : text;
}

/* Runs `body`, turning exceptions and USD errors into a message. */
template<typename F> int guarded(char *error, size_t error_len, const char *what, F &&body)
{
  pxr::TfErrorMark mark;
  try {
    std::string message;
    if (!body(message)) {
      set_text(error, error_len, usd_errors(mark, message.empty() ? what : message));
      mark.Clear();
      return 0;
    }
    if (!mark.IsClean()) {
      set_text(error, error_len, usd_errors(mark, what));
      mark.Clear();
      return 0;
    }
    return 1;
  }
  catch (const std::exception &e) {
    set_text(error, error_len, std::string(what) + ": " + e.what());
  }
  catch (...) {
    set_text(error, error_len, what);
  }
  mark.Clear();
  return 0;
}

bool valid_parent(const OuStage *stage, const char *parent, pxr::SdfPath &out, std::string &message)
{
  if (stage == nullptr || parent == nullptr) {
    message = "no stage or parent";
    return false;
  }
  out = pxr::SdfPath(parent);
  if (out.IsEmpty() || !out.IsAbsolutePath()) {
    message = std::string("not an absolute prim path: ") + parent;
    return false;
  }
  if (out != pxr::SdfPath::AbsoluteRootPath() && !stage->stage->GetPrimAtPath(out).IsValid()) {
    message = std::string("no prim at ") + parent;
    return false;
  }
  return true;
}

}  // namespace

extern "C" {

uint32_t ou_abi_version(void)
{
  return OU_ABI_VERSION;
}

size_t ou_struct_size(uint32_t which)
{
  switch (which) {
    case OU_STRUCT_XFORM:
      return sizeof(OuXform);
    case OU_STRUCT_UV_SET:
      return sizeof(OuUvSet);
    case OU_STRUCT_SUBSET:
      return sizeof(OuSubset);
    case OU_STRUCT_MESH:
      return sizeof(OuMesh);
    case OU_STRUCT_TEXTURE:
      return sizeof(OuTexture);
    case OU_STRUCT_SURFACE_INPUT:
      return sizeof(OuSurfaceInput);
    case OU_STRUCT_MATERIAL:
      return sizeof(OuMaterial);
    case OU_STRUCT_CAMERA:
      return sizeof(OuCamera);
    case OU_STRUCT_LIGHT:
      return sizeof(OuLight);
    default:
      return 0;
  }
}

const char *ou_usd_version(void)
{
  static const std::string version = std::to_string(PXR_MAJOR_VERSION) + "." +
                                     std::to_string(PXR_MINOR_VERSION) + "." +
                                     std::to_string(PXR_PATCH_VERSION);
  return version.c_str();
}

int ou_init(const char *plugin_dir, char *error, size_t error_len)
{
  static std::mutex lock;
  std::lock_guard<std::mutex> guard(lock);
  return guarded(error, error_len, "Couldn't load OpenUSD's plugins", [&](std::string &message) {
    if (plugin_dir != nullptr && plugin_dir[0] != '\0') {
      pxr::PlugRegistry::GetInstance().RegisterPlugins(plugin_dir);
    }
    /* Without the file formats nothing can be read or written. */
    if (!pxr::SdfFileFormat::FindByExtension("usda")) {
      message = "OpenUSD's plugins weren't found";
      if (plugin_dir != nullptr) {
        message += std::string(" in ") + plugin_dir;
      }
      return false;
    }
    return true;
  });
}

OuStage *ou_stage_create(const char *path, uint32_t text, char *error, size_t error_len)
{
  OuStage *out = nullptr;
  guarded(error, error_len, "Couldn't create the USD file", [&](std::string &message) {
    if (path == nullptr) {
      message = "no path";
      return false;
    }
    pxr::SdfLayer::FileFormatArguments args;
    if (pxr::TfStringToLower(pxr::TfStringGetSuffix(path)) == "usd") {
      args["format"] = text ? "usda" : "usdc";
    }
    pxr::SdfLayerRefPtr layer = pxr::SdfLayer::CreateNew(path, args);
    if (!layer) {
      return false;
    }
    pxr::UsdStageRefPtr stage = pxr::UsdStage::Open(layer);
    if (!stage) {
      return false;
    }
    out = new OuStage{stage, layer};
    return true;
  });
  return out;
}

OuStage *ou_stage_open(const char *path, char *error, size_t error_len)
{
  OuStage *out = nullptr;
  guarded(error, error_len, "Couldn't open the USD file", [&](std::string &message) {
    if (path == nullptr) {
      message = "no path";
      return false;
    }
    pxr::UsdStageRefPtr stage = pxr::UsdStage::Open(std::string(path));
    if (!stage) {
      return false;
    }
    out = new OuStage{stage, stage->GetRootLayer()};
    return true;
  });
  return out;
}

void ou_stage_free(OuStage *stage)
{
  delete stage;
}

int ou_stage_set_metadata(OuStage *stage,
                          char up_axis,
                          double meters_per_unit,
                          const char *default_prim,
                          char *error,
                          size_t error_len)
{
  return guarded(error, error_len, "Couldn't write the stage settings", [&](std::string &message) {
    if (stage == nullptr) {
      message = "no stage";
      return false;
    }
    if (up_axis != 'Y' && up_axis != 'Z') {
      message = "the up axis must be Y or Z";
      return false;
    }
    pxr::UsdGeomSetStageUpAxis(stage->stage,
                               up_axis == 'Z' ? pxr::UsdGeomTokens->z : pxr::UsdGeomTokens->y);
    pxr::UsdGeomSetStageMetersPerUnit(stage->stage, meters_per_unit);
    if (default_prim != nullptr) {
      pxr::UsdPrim prim = stage->stage->GetPrimAtPath(pxr::SdfPath(default_prim));
      if (!prim) {
        message = std::string("no prim at ") + default_prim;
        return false;
      }
      stage->stage->SetDefaultPrim(prim);
    }
    return true;
  });
}

int ou_define_xform(OuStage *stage,
                    const char *parent,
                    const OuXform *xform,
                    char *path_out,
                    size_t path_len,
                    char *error,
                    size_t error_len)
{
  return guarded(error, error_len, "Couldn't write an item", [&](std::string &message) {
    pxr::SdfPath parent_path;
    if (xform == nullptr || !valid_parent(stage, parent, parent_path, message)) {
      return false;
    }
    using namespace operandi::blender_usd;
    const pxr::SdfPath path = get_unique_path(
        stage->stage, parent_path.AppendChild(pxr::TfToken(make_safe_name(xform->name ? xform->name : ""))));
    write_xform(stage->stage, path, *xform);
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

int ou_define_mesh(OuStage *stage,
                   const char *parent,
                   const OuMesh *mesh,
                   char *path_out,
                   size_t path_len,
                   char *error,
                   size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a mesh", [&](std::string &message) {
    pxr::SdfPath parent_path;
    if (mesh == nullptr || !valid_parent(stage, parent, parent_path, message)) {
      return false;
    }
    using namespace operandi::blender_usd;
    const pxr::SdfPath path = get_unique_path(
        stage->stage, parent_path.AppendChild(pxr::TfToken(make_safe_name(mesh->name ? mesh->name : ""))));
    if (!write_mesh(stage->stage, path, *mesh, message)) {
      stage->stage->RemovePrim(path);
      return false;
    }
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

int ou_define_material(OuStage *stage,
                       const char *parent,
                       const OuMaterial *material,
                       char *path_out,
                       size_t path_len,
                       char *error,
                       size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a material", [&](std::string &message) {
    if (stage == nullptr || parent == nullptr || material == nullptr) {
      message = "no stage, parent or material";
      return false;
    }
    pxr::SdfPath parent_path(parent);
    if (parent_path.IsEmpty() || !parent_path.IsAbsolutePath()) {
      message = std::string("not an absolute prim path: ") + parent;
      return false;
    }
    /* Materials live in a Scope (Blender's /root/_materials). */
    if (!stage->stage->GetPrimAtPath(parent_path).IsValid()) {
      if (!stage->stage->GetPrimAtPath(parent_path.GetParentPath()).IsValid()) {
        message = std::string("no prim above ") + parent;
        return false;
      }
      pxr::UsdGeomScope::Define(stage->stage, parent_path);
    }
    using namespace operandi::blender_usd;
    const pxr::SdfPath path = get_unique_path(
        stage->stage,
        parent_path.AppendChild(pxr::TfToken(make_safe_name(material->name ? material->name : ""))));
    if (!write_material(stage->stage, path, *material, message)) {
      stage->stage->RemovePrim(path);
      return false;
    }
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

/* The USD value type of a MaterialX type. */
static pxr::SdfValueTypeName usd_type(const std::string &mx)
{
  using namespace pxr;
  if (mx == "float") return SdfValueTypeNames->Float;
  if (mx == "integer") return SdfValueTypeNames->Int;
  if (mx == "boolean") return SdfValueTypeNames->Bool;
  if (mx == "color3") return SdfValueTypeNames->Color3f;
  if (mx == "color4") return SdfValueTypeNames->Color4f;
  if (mx == "vector2") return SdfValueTypeNames->Float2;
  if (mx == "vector3") return SdfValueTypeNames->Float3;
  if (mx == "vector4") return SdfValueTypeNames->Float4;
  if (mx == "filename") return SdfValueTypeNames->Asset;
  if (mx == "string") return SdfValueTypeNames->String;
  return SdfValueTypeNames->Token;
}

/* MaterialX text ("1, 0.5, 0") as a value of `type`. */
static pxr::VtValue usd_value(const std::string &mx, const std::string &text)
{
  using namespace pxr;
  std::vector<float> n;
  for (const std::string &part : TfStringSplit(text, ",")) {
    n.push_back(float(std::atof(TfStringTrim(part).c_str())));
  }
  auto at = [&](size_t i) { return i < n.size() ? n[i] : 0.0f; };
  if (mx == "float") return VtValue(at(0));
  if (mx == "integer") return VtValue(int(at(0)));
  if (mx == "boolean") return VtValue(TfStringTrim(text) == "true");
  if (mx == "color3" || mx == "vector3") return VtValue(GfVec3f(at(0), at(1), at(2)));
  if (mx == "color4" || mx == "vector4") return VtValue(GfVec4f(at(0), at(1), at(2), at(3)));
  if (mx == "vector2") return VtValue(GfVec2f(at(0), at(1)));
  if (mx == "filename") return VtValue(SdfAssetPath(text));
  if (mx == "string") return VtValue(text);
  return VtValue(TfToken(text));
}

int ou_define_network(OuStage *stage,
                      const char *material,
                      const char *json,
                      char *error,
                      size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a MaterialX network", [&](std::string &message) {
    using namespace pxr;
    if (stage == nullptr || material == nullptr || json == nullptr) {
      message = "no stage, material or network";
      return false;
    }
    UsdShadeMaterial mat(stage->stage->GetPrimAtPath(SdfPath(material)));
    if (!mat) {
      message = std::string("no material at ") + material;
      return false;
    }
    JsParseError parse;
    const JsValue doc = JsParseString(json, &parse);
    if (!doc.IsObject()) {
      message = "the network isn't JSON: " + parse.reason;
      return false;
    }
    const JsObject &root = doc.GetJsObject();
    const JsArray nodes = root.count("nodes") ? root.at("nodes").GetJsArray() : JsArray();
    const JsArray graphs = root.count("graphs") ? root.at("graphs").GetJsArray() : JsArray();
    using namespace operandi::blender_usd;
    /* A child prim path under `parent`, kept clear of prims already there
     * (the Preview Surface's shaders). */
    auto child = [&](const SdfPath &parent, const std::string &name) {
      const SdfPath path = parent.AppendChild(TfToken(make_safe_name(name)));
      return stage->stage->GetPrimAtPath(path) ? get_unique_path(stage->stage, path) : path;
    };
    auto place = [&](const UsdPrim &prim, const JsObject &o) {
      if (!o.count("pos") || !o.at("pos").IsArray()) {
        return;
      }
      const JsArray p = o.at("pos").GetJsArray();
      if (p.size() < 2) {
        return;
      }
      UsdUINodeGraphNodeAPI::Apply(prim).CreatePosAttr(
          VtValue(GfVec2f(float(p[0].GetReal()), float(p[1].GetReal()))));
    };
    /* NodeGraphs first (shaders sit inside them), then shaders with their
     * outputs, then every input and wire. Shaders are keyed by
     * "<graph>/<name>" ("/<name>" at the top level). */
    std::map<std::string, UsdShadeNodeGraph> node_graphs;
    for (const JsValue &g : graphs) {
      const JsObject &graph = g.GetJsObject();
      const std::string name = graph.at("name").GetString();
      UsdShadeNodeGraph ng = UsdShadeNodeGraph::Define(stage->stage, child(mat.GetPath(), name));
      place(ng.GetPrim(), graph);
      for (const JsValue &i : graph.at("inputs").GetJsArray()) {
        const JsObject &in = i.GetJsObject();
        ng.CreateInput(TfToken(in.at("name").GetString()), usd_type(in.at("type").GetString()));
      }
      for (const JsValue &o : graph.at("outputs").GetJsArray()) {
        const JsObject &out = o.GetJsObject();
        ng.CreateOutput(TfToken(out.at("name").GetString()), usd_type(out.at("type").GetString()));
      }
      node_graphs[name] = ng;
    }
    std::map<std::string, UsdShadeShader> shaders;
    auto scope_of = [](const JsObject &o) {
      return o.count("graph") && o.at("graph").IsString() ? o.at("graph").GetString() : std::string();
    };
    for (const JsValue &n : nodes) {
      const JsObject &node = n.GetJsObject();
      const std::string name = node.at("name").GetString();
      const std::string scope = scope_of(node);
      SdfPath parent = mat.GetPath();
      if (!scope.empty()) {
        auto g = node_graphs.find(scope);
        if (g == node_graphs.end()) {
          message = "node " + name + " names a missing node graph " + scope;
          return false;
        }
        parent = g->second.GetPath();
      }
      UsdShadeShader shader = UsdShadeShader::Define(stage->stage, child(parent, name));
      shader.CreateIdAttr(VtValue(TfToken(node.at("id").GetString())));
      if (node.count("outputs")) {
        for (const JsValue &o : node.at("outputs").GetJsArray()) {
          const JsObject &out = o.GetJsObject();
          shader.CreateOutput(TfToken(out.at("name").GetString()),
                              usd_type(out.at("type").GetString()));
        }
      }
      else {
        shader.CreateOutput(TfToken("out"), usd_type(node.at("type").GetString()));
      }
      place(shader.GetPrim(), node);
      shaders[scope + "/" + name] = shader;
    }
    /* Wires `input` (in scope `scope`) as JSON `in` says: a node's output,
     * a NodeGraph's output, or the enclosing NodeGraph's interface. */
    auto wire = [&](const UsdShadeInput &input, const JsObject &in, const std::string &scope,
                    const std::string &type) {
      const std::string output = in.count("output") ? in.at("output").GetString() : "out";
      if (in.count("connect")) {
        auto src = shaders.find(scope + "/" + in.at("connect").GetString());
        if (src != shaders.end()) {
          input.ConnectToSource(src->second.GetOutput(TfToken(output)));
        }
      }
      else if (in.count("graph")) {
        auto g = node_graphs.find(in.at("graph").GetString());
        if (g != node_graphs.end()) {
          input.ConnectToSource(g->second.GetOutput(TfToken(output)));
        }
      }
      else if (in.count("interface")) {
        auto g = node_graphs.find(scope);
        if (g != node_graphs.end()) {
          input.ConnectToSource(g->second.GetInput(TfToken(in.at("interface").GetString())));
        }
      }
      else if (in.count("value")) {
        input.Set(usd_value(type, in.at("value").GetString()));
      }
      if (in.count("colorspace")) {
        input.GetAttr().SetColorSpace(TfToken(in.at("colorspace").GetString()));
      }
    };
    for (const JsValue &n : nodes) {
      const JsObject &node = n.GetJsObject();
      const std::string scope = scope_of(node);
      UsdShadeShader shader = shaders[scope + "/" + node.at("name").GetString()];
      for (const JsValue &i : node.at("inputs").GetJsArray()) {
        const JsObject &in = i.GetJsObject();
        const std::string type = in.at("type").GetString();
        wire(shader.CreateInput(TfToken(in.at("name").GetString()), usd_type(type)), in, scope, type);
      }
    }
    for (const JsValue &g : graphs) {
      const JsObject &graph = g.GetJsObject();
      const std::string name = graph.at("name").GetString();
      UsdShadeNodeGraph ng = node_graphs[name];
      /* Interface inputs read the top level. */
      for (const JsValue &i : graph.at("inputs").GetJsArray()) {
        const JsObject &in = i.GetJsObject();
        wire(ng.GetInput(TfToken(in.at("name").GetString())), in, "", in.at("type").GetString());
      }
      for (const JsValue &o : graph.at("outputs").GetJsArray()) {
        const JsObject &out = o.GetJsObject();
        UsdShadeOutput output = ng.GetOutput(TfToken(out.at("name").GetString()));
        const std::string from = out.count("output") ? out.at("output").GetString() : "out";
        if (out.count("connect")) {
          auto src = shaders.find(name + "/" + out.at("connect").GetString());
          if (src != shaders.end()) {
            output.ConnectToSource(src->second.GetOutput(TfToken(from)));
          }
        }
        else if (out.count("interface")) {
          output.ConnectToSource(ng.GetInput(TfToken(out.at("interface").GetString())));
        }
      }
    }
    auto terminal = shaders.find("/" + (root.count("terminal") ? root.at("terminal").GetString() : ""));
    if (terminal == shaders.end()) {
      message = "the network names no surface shader";
      return false;
    }
    mat.CreateOutput(TfToken("mtlx:surface"), SdfValueTypeNames->Token)
        .ConnectToSource(terminal->second.GetOutput(TfToken("out")));
    /* Tells Operandi's reader to take the network as it is: NodeGraphs,
     * interfaces and nodes that feed nothing included. */
    mat.GetPrim().SetCustomDataByKey(TfToken("operandi:materialxGraph"), VtValue(true));
    return true;
  });
}

int ou_define_camera(OuStage *stage,
                     const char *parent,
                     const OuCamera *camera,
                     char *path_out,
                     size_t path_len,
                     char *error,
                     size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a camera", [&](std::string &message) {
    pxr::SdfPath parent_path;
    if (camera == nullptr || !valid_parent(stage, parent, parent_path, message)) {
      return false;
    }
    using namespace operandi::blender_usd;
    const pxr::SdfPath path = get_unique_path(
        stage->stage,
        parent_path.AppendChild(pxr::TfToken(make_safe_name(camera->name ? camera->name : ""))));
    write_camera(stage->stage, path, *camera);
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

int ou_define_light(OuStage *stage,
                    const char *parent,
                    const OuLight *light,
                    char *path_out,
                    size_t path_len,
                    char *error,
                    size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a light", [&](std::string &message) {
    pxr::SdfPath parent_path;
    if (light == nullptr || !valid_parent(stage, parent, parent_path, message)) {
      return false;
    }
    using namespace operandi::blender_usd;
    const pxr::SdfPath path = get_unique_path(
        stage->stage,
        parent_path.AppendChild(pxr::TfToken(make_safe_name(light->name ? light->name : ""))));
    if (!write_light(stage->stage, path, *light, message)) {
      stage->stage->RemovePrim(path);
      return false;
    }
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

/* A "custom" JSON value as customData: strings, booleans, numbers,
 * integer (or string) lists and nested objects. */
static pxr::VtValue custom_value(const pxr::JsValue &v)
{
  using namespace pxr;
  if (v.IsString()) return VtValue(v.GetString());
  if (v.IsBool()) return VtValue(v.GetBool());
  if (v.IsInt()) return VtValue(int(v.GetInt64()));
  if (v.IsReal()) return VtValue(v.GetReal());
  if (v.IsArray()) {
    const JsArray &a = v.GetJsArray();
    bool strings = !a.empty() && a.front().IsString();
    if (strings) {
      VtStringArray out;
      for (const JsValue &e : a) {
        out.push_back(e.IsString() ? e.GetString() : std::string());
      }
      return VtValue(out);
    }
    VtIntArray out;
    out.reserve(a.size());
    for (const JsValue &e : a) {
      out.push_back(e.IsInt() ? int(e.GetInt64()) : 0);
    }
    return VtValue(out);
  }
  if (v.IsObject()) {
    VtDictionary d;
    for (const auto &kv : v.GetJsObject()) {
      d[kv.first] = custom_value(kv.second);
    }
    return VtValue(d);
  }
  return VtValue();
}

/* Puts `entry`'s "custom" object, if any, in `object`'s customData under
 * "operandi". */
static void set_custom(const pxr::UsdObject &object, const pxr::JsObject &entry)
{
  auto it = entry.find("custom");
  if (it == entry.end() || !it->second.IsObject()) {
    return;
  }
  for (const auto &kv : it->second.GetJsObject()) {
    object.SetCustomDataByKey(pxr::TfToken("operandi:" + kv.first), custom_value(kv.second));
  }
}

int ou_define_render_settings(OuStage *stage, const char *json, char *error, size_t error_len)
{
  return guarded(error, error_len, "Couldn't write the render settings", [&](std::string &message) {
    using namespace pxr;
    if (stage == nullptr || json == nullptr) {
      message = "no stage or settings";
      return false;
    }
    JsParseError parse;
    const JsValue doc = JsParseString(json, &parse);
    if (!doc.IsObject()) {
      message = "the settings aren't JSON: " + parse.reason;
      return false;
    }
    const JsObject &root = doc.GetJsObject();
    auto text = [](const JsObject &o, const char *key, const char *fallback) {
      return o.count(key) && o.at(key).IsString() ? o.at(key).GetString() : std::string(fallback);
    };
    auto size = [](const JsObject &o, GfVec2i &out) {
      if (!o.count("resolution") || !o.at("resolution").IsArray()) {
        return false;
      }
      const JsArray &r = o.at("resolution").GetJsArray();
      if (r.size() < 2 || !r[0].IsInt() || !r[1].IsInt()) {
        return false;
      }
      out = GfVec2i(int(r[0].GetInt64()), int(r[1].GetInt64()));
      return true;
    };
    const SdfPath scope(text(root, "scope", "/Render"));
    UsdGeomScope::Define(stage->stage, scope);
    UsdRenderSettings settings = UsdRenderSettings::Define(
        stage->stage, scope.AppendChild(TfToken(text(root, "name", "Settings"))));
    GfVec2i res;
    if (size(root, res)) {
      settings.CreateResolutionAttr(VtValue(res));
    }
    if (root.count("pixelAspect") && (root.at("pixelAspect").IsReal() || root.at("pixelAspect").IsInt())) {
      settings.CreatePixelAspectRatioAttr(VtValue(float(root.at("pixelAspect").GetReal())));
    }
    const std::string camera = text(root, "camera", "");
    if (!camera.empty()) {
      settings.CreateCameraRel().SetTargets({SdfPath(camera)});
    }
    if (root.count("product") && root.at("product").IsObject()) {
      const JsObject &p = root.at("product").GetJsObject();
      const SdfPath products = scope.AppendChild(TfToken("Products"));
      UsdGeomScope::Define(stage->stage, products);
      UsdRenderProduct product = UsdRenderProduct::Define(
          stage->stage, products.AppendChild(TfToken(text(p, "name", "Final"))));
      product.CreateProductNameAttr(VtValue(TfToken(text(p, "productName", "render.exr"))));
      GfVec2i pres;
      if (size(p, pres)) {
        product.CreateResolutionAttr(VtValue(pres));
      }
      if (!camera.empty()) {
        product.CreateCameraRel().SetTargets({SdfPath(camera)});
      }
      settings.CreateProductsRel().SetTargets({product.GetPath()});
    }
    set_custom(settings.GetPrim(), root);
    /* The settings a renderer of this stage uses. */
    stage->stage->SetMetadata(TfToken("renderSettingsPrimPath"), settings.GetPath().GetString());
    return true;
  });
}

int ou_define_mesh_extras(OuStage *stage,
                          const char *mesh,
                          const char *json,
                          const void *const *buffers,
                          const size_t *sizes,
                          size_t buffer_count,
                          char *error,
                          size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a mesh's maps and sets", [&](std::string &message) {
    using namespace pxr;
    if (stage == nullptr || mesh == nullptr || json == nullptr) {
      message = "no stage, mesh or description";
      return false;
    }
    UsdGeomMesh target(stage->stage->GetPrimAtPath(SdfPath(mesh)));
    if (!target) {
      message = std::string("no mesh at ") + mesh;
      return false;
    }
    JsParseError parse;
    const JsValue doc = JsParseString(json, &parse);
    if (!doc.IsObject()) {
      message = "the description isn't JSON: " + parse.reason;
      return false;
    }
    const JsObject &root = doc.GetJsObject();
    /* Buffer `key` of `entry` as floats or ints; false when it's missing or
     * out of range. */
    auto buffer = [&](const JsObject &entry, const char *key, const void *&data, size_t &bytes) {
      auto it = entry.find(key);
      if (it == entry.end() || !it->second.IsInt()) {
        return false;
      }
      const int64_t i = it->second.GetInt64();
      if (i < 0 || size_t(i) >= buffer_count) {
        return false;
      }
      data = buffers[i];
      bytes = sizes[i];
      return true;
    };
    auto floats = [&](const JsObject &entry, const char *key, std::vector<float> &out) {
      const void *data = nullptr;
      size_t bytes = 0;
      out.clear();
      if (!buffer(entry, key, data, bytes)) {
        return false;
      }
      const float *f = static_cast<const float *>(data);
      out.assign(f, f + bytes / sizeof(float));
      return true;
    };
    auto ints = [&](const JsObject &entry, const char *key, VtIntArray &out) {
      const void *data = nullptr;
      size_t bytes = 0;
      out.clear();
      if (!buffer(entry, key, data, bytes)) {
        return false;
      }
      const int32_t *n = static_cast<const int32_t *>(data);
      out.assign(n, n + bytes / sizeof(int32_t));
      return true;
    };
    auto text = [](const JsObject &entry, const char *key) {
      auto it = entry.find(key);
      return it != entry.end() && it->second.IsString() ? it->second.GetString() : std::string();
    };
    using operandi::blender_usd::make_safe_name;

    /* A mesh light (usd-gaps-3): MeshLightAPI on this mesh, its emission
     * only (the mesh itself doesn't render in Operandi). */
    if (root.count("light") && root.at("light").IsObject()) {
      const JsObject &l = root.at("light").GetJsObject();
      UsdLuxMeshLightAPI::Apply(target.GetPrim());
      UsdLuxLightAPI light(target.GetPrim());
      auto real = [&](const char *key, double fallback) {
        return l.count(key) && (l.at(key).IsReal() || l.at(key).IsInt()) ? l.at(key).GetReal() :
                                                                            fallback;
      };
      light.CreateIntensityAttr(VtValue(float(real("intensity", 1.0))));
      light.CreateExposureAttr(VtValue(0.0f));
      light.CreateNormalizeAttr(VtValue(false));
      light.CreateDiffuseAttr(VtValue(float(real("diffuse", 1.0))));
      light.CreateSpecularAttr(VtValue(float(real("specular", 1.0))));
      if (l.count("color") && l.at("color").IsArray()) {
        const JsArray &c = l.at("color").GetJsArray();
        if (c.size() >= 3) {
          light.CreateColorAttr(VtValue(GfVec3f(float(c[0].GetReal()), float(c[1].GetReal()),
                                                float(c[2].GetReal()))));
        }
      }
      light.CreateMaterialSyncModeAttr(VtValue(UsdLuxTokens->noMaterialResponse));
    }

    UsdGeomPrimvarsAPI primvars(target.GetPrim());
    std::vector<float> values;
    VtIntArray indices;
    const JsArray none;
    const JsArray &pvs = root.count("primvars") ? root.at("primvars").GetJsArray() : none;
    for (const JsValue &p : pvs) {
      const JsObject &pv = p.GetJsObject();
      const std::string type = text(pv, "type");
      const TfToken interpolation(text(pv, "interpolation"));
      if (!floats(pv, "values", values)) {
        message = "a map has no values";
        return false;
      }
      int element = 1;
      if (pv.count("elementSize") && pv.at("elementSize").IsInt()) {
        element = std::max(1, int(pv.at("elementSize").GetInt64()));
      }
      SdfValueTypeName vtype;
      VtValue value;
      auto vec = [&](auto tag, size_t n) {
        using V = decltype(tag);
        VtArray<V> a(values.size() / n);
        for (size_t i = 0; i < a.size(); i++) {
          for (size_t k = 0; k < n; k++) {
            a[i][k] = values[i * n + k];
          }
        }
        value = VtValue(a);
      };
      if (type == "float") {
        vtype = SdfValueTypeNames->FloatArray;
        value = VtValue(VtFloatArray(values.begin(), values.end()));
      }
      else if (type == "float2") {
        vtype = SdfValueTypeNames->Float2Array;
        vec(GfVec2f(), 2);
      }
      else if (type == "float3") {
        vtype = SdfValueTypeNames->Float3Array;
        vec(GfVec3f(), 3);
      }
      else if (type == "float4") {
        vtype = SdfValueTypeNames->Float4Array;
        vec(GfVec4f(), 4);
      }
      else if (type == "color3") {
        vtype = SdfValueTypeNames->Color3fArray;
        vec(GfVec3f(), 3);
      }
      else if (type == "color4") {
        vtype = SdfValueTypeNames->Color4fArray;
        vec(GfVec4f(), 4);
      }
      else {
        message = "unknown map type " + type;
        return false;
      }
      UsdGeomPrimvar primvar = primvars.CreatePrimvar(
          TfToken(make_safe_name(text(pv, "name"))), vtype, interpolation, element);
      primvar.Set(value);
      if (ints(pv, "indices", indices)) {
        primvar.SetIndices(indices);
      }
      if (pv.count("unauthored") && pv.at("unauthored").IsInt()) {
        primvar.SetUnauthoredValuesIndex(int(pv.at("unauthored").GetInt64()));
      }
      set_custom(primvar.GetAttr(), pv);
    }

    const JsArray &subs = root.count("subsets") ? root.at("subsets").GetJsArray() : none;
    for (const JsValue &s : subs) {
      const JsObject &sub = s.GetJsObject();
      if (!ints(sub, "indices", indices)) {
        message = "a set has no indices";
        return false;
      }
      const TfToken element = text(sub, "elementType") == "point" ? UsdGeomTokens->point :
                                                                     UsdGeomTokens->face;
      UsdGeomSubset subset = UsdGeomSubset::CreateUniqueGeomSubset(
          target,
          TfToken(make_safe_name(text(sub, "name"))),
          element,
          indices,
          TfToken(text(sub, "family")),
          TfToken(text(sub, "familyType")));
      if (!subset) {
        message = "couldn't write set " + text(sub, "name");
        return false;
      }
      set_custom(subset.GetPrim(), sub);
    }

    const JsArray &shapes = root.count("blendShapes") ? root.at("blendShapes").GetJsArray() : none;
    if (!shapes.empty()) {
      VtTokenArray names;
      SdfPathVector targets;
      for (const JsValue &b : shapes) {
        const JsObject &shape = b.GetJsObject();
        if (!floats(shape, "offsets", values)) {
          message = "a morph has no offsets";
          return false;
        }
        const SdfPath path = operandi::blender_usd::get_unique_path(
            stage->stage,
            target.GetPath().AppendChild(TfToken(make_safe_name(text(shape, "name")))));
        UsdSkelBlendShape blend = UsdSkelBlendShape::Define(stage->stage, path);
        VtVec3fArray offsets(values.size() / 3);
        for (size_t i = 0; i < offsets.size(); i++) {
          offsets[i] = GfVec3f(values[i * 3], values[i * 3 + 1], values[i * 3 + 2]);
        }
        blend.CreateOffsetsAttr(VtValue(offsets));
        if (ints(shape, "pointIndices", indices)) {
          blend.CreatePointIndicesAttr(VtValue(indices));
        }
        set_custom(blend.GetPrim(), shape);
        names.push_back(path.GetNameToken());
        targets.push_back(path);
      }
      UsdSkelBindingAPI binding = UsdSkelBindingAPI::Apply(target.GetPrim());
      binding.CreateBlendShapesAttr(VtValue(names));
      binding.CreateBlendShapeTargetsRel().SetTargets(targets);
    }

    set_custom(target.GetPrim(), root);
    return true;
  });
}

int ou_make_class(OuStage *stage, const char *path, char *error, size_t error_len)
{
  return guarded(error, error_len, "Couldn't write the instance prototypes", [&](std::string &message) {
    if (stage == nullptr || path == nullptr) {
      message = "no stage or path";
      return false;
    }
    pxr::UsdPrim prim = stage->stage->GetPrimAtPath(pxr::SdfPath(path));
    if (!prim) {
      message = std::string("no prim at ") + path;
      return false;
    }
    return prim.SetSpecifier(pxr::SdfSpecifierClass);
  });
}

int ou_define_curves(OuStage *stage,
                     const char *parent,
                     const char *json,
                     const void *const *buffers,
                     const size_t *sizes,
                     size_t buffer_count,
                     char *path_out,
                     size_t path_len,
                     char *error,
                     size_t error_len)
{
  return guarded(error, error_len, "Couldn't write curves", [&](std::string &message) {
    using namespace pxr;
    SdfPath parent_path;
    if (json == nullptr || !valid_parent(stage, parent, parent_path, message)) {
      return false;
    }
    JsParseError parse;
    const JsValue doc = JsParseString(json, &parse);
    if (!doc.IsObject()) {
      message = "the description isn't JSON: " + parse.reason;
      return false;
    }
    const JsObject &root = doc.GetJsObject();
    auto text = [&](const char *key) {
      auto it = root.find(key);
      return it != root.end() && it->second.IsString() ? it->second.GetString() : std::string();
    };
    /* Buffer `key` as `T`s; false when it's missing or out of range. */
    auto get = [&](const char *key, auto &out) {
      using T = typename std::decay_t<decltype(out)>::value_type;
      out.clear();
      auto it = root.find(key);
      if (it == root.end() || !it->second.IsInt()) {
        return false;
      }
      const int64_t i = it->second.GetInt64();
      if (i < 0 || size_t(i) >= buffer_count) {
        return false;
      }
      const T *data = static_cast<const T *>(buffers[i]);
      out.assign(data, data + sizes[i] / sizeof(T));
      return true;
    };
    VtIntArray counts;
    std::vector<float> flat;
    if (!get("counts", counts) || !get("points", flat)) {
      message = "curves need counts and points";
      return false;
    }
    VtVec3fArray points(flat.size() / 3);
    for (size_t i = 0; i < points.size(); i++) {
      points[i] = GfVec3f(flat[3 * i], flat[3 * i + 1], flat[3 * i + 2]);
    }
    using operandi::blender_usd::get_unique_path;
    using operandi::blender_usd::make_safe_name;
    const SdfPath path = get_unique_path(
        stage->stage, parent_path.AppendChild(TfToken(make_safe_name(text("name")))));
    UsdGeomCurves curves;
    if (text("schema") == "NurbsCurves") {
      UsdGeomNurbsCurves nurbs = UsdGeomNurbsCurves::Define(stage->stage, path);
      VtIntArray order;
      std::vector<double> knots, ranges;
      get("order", order);
      get("knots", knots);
      get("ranges", ranges);
      nurbs.CreateOrderAttr().Set(order);
      nurbs.CreateKnotsAttr().Set(VtDoubleArray(knots.begin(), knots.end()));
      if (!ranges.empty()) {
        VtVec2dArray r(ranges.size() / 2);
        for (size_t i = 0; i < r.size(); i++) {
          r[i] = GfVec2d(ranges[2 * i], ranges[2 * i + 1]);
        }
        nurbs.CreateRangesAttr().Set(r);
      }
      curves = nurbs;
    }
    else {
      UsdGeomBasisCurves basis = UsdGeomBasisCurves::Define(stage->stage, path);
      const std::string type = text("type");
      basis.CreateTypeAttr().Set(TfToken(type.empty() ? "linear" : type));
      const std::string b = text("basis");
      if (!b.empty()) {
        basis.CreateBasisAttr().Set(TfToken(b));
      }
      const std::string wrap = text("wrap");
      basis.CreateWrapAttr().Set(TfToken(wrap.empty() ? "nonperiodic" : wrap));
      curves = basis;
    }
    if (!curves) {
      message = "couldn't define " + path.GetString();
      return false;
    }
    curves.CreateCurveVertexCountsAttr().Set(counts);
    curves.CreatePointsAttr().Set(points);
    VtVec3fArray extent;
    if (UsdGeomPointBased::ComputeExtent(points, &extent)) {
      curves.CreateExtentAttr().Set(extent);
    }
    std::vector<float> widths;
    if (get("widths", widths) && widths.size() == points.size()) {
      curves.CreateWidthsAttr().Set(VtFloatArray(widths.begin(), widths.end()));
      curves.SetWidthsInterpolation(UsdGeomTokens->vertex);
    }
    auto visible = root.find("visible");
    if (visible != root.end() && visible->second.IsBool() && !visible->second.GetBool()) {
      curves.CreateVisibilityAttr().Set(UsdGeomTokens->invisible);
    }
    const std::string material = text("material");
    if (!material.empty()) {
      UsdShadeMaterial m(stage->stage->GetPrimAtPath(SdfPath(material)));
      if (m) {
        UsdShadeMaterialBindingAPI::Apply(curves.GetPrim()).Bind(m);
      }
    }
    set_custom(curves.GetPrim(), root);
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

int ou_define_point_instancer(OuStage *stage,
                              const char *parent,
                              const char *json,
                              const void *const *buffers,
                              const size_t *sizes,
                              size_t buffer_count,
                              char *path_out,
                              size_t path_len,
                              char *error,
                              size_t error_len)
{
  return guarded(error, error_len, "Couldn't write a point instancer", [&](std::string &message) {
    using namespace pxr;
    SdfPath parent_path;
    if (json == nullptr || !valid_parent(stage, parent, parent_path, message)) {
      return false;
    }
    JsParseError parse;
    const JsValue doc = JsParseString(json, &parse);
    if (!doc.IsObject()) {
      message = "the description isn't JSON: " + parse.reason;
      return false;
    }
    const JsObject &root = doc.GetJsObject();
    /* Buffer `key` as `T`s; false when it's missing or out of range. */
    auto get = [&](const char *key, auto &out) {
      using T = typename std::decay_t<decltype(out)>::value_type;
      out.clear();
      auto it = root.find(key);
      if (it == root.end() || !it->second.IsInt()) {
        return false;
      }
      const int64_t i = it->second.GetInt64();
      if (i < 0 || size_t(i) >= buffer_count) {
        return false;
      }
      const T *data = static_cast<const T *>(buffers[i]);
      out.assign(data, data + sizes[i] / sizeof(T));
      return true;
    };
    VtIntArray proto_indices;
    std::vector<float> positions, orientations, scales;
    if (!get("protoIndices", proto_indices) || !get("positions", positions) ||
        !get("orientations", orientations) || !get("scales", scales))
    {
      message = "an instancer needs protoIndices, positions, orientations and scales";
      return false;
    }
    const size_t n = proto_indices.size();
    if (positions.size() != 3 * n || orientations.size() != 4 * n || scales.size() != 3 * n) {
      message = "an instancer's arrays don't match its protoIndices";
      return false;
    }
    using operandi::blender_usd::get_unique_path;
    using operandi::blender_usd::make_safe_name;
    auto name = root.find("name");
    const SdfPath path = get_unique_path(
        stage->stage,
        parent_path.AppendChild(TfToken(make_safe_name(
            name != root.end() && name->second.IsString() ? name->second.GetString() : "instances"))));
    UsdGeomPointInstancer instancer = UsdGeomPointInstancer::Define(stage->stage, path);
    const SdfPath scope = path.AppendChild(TfToken("Prototypes"));
    UsdGeomScope::Define(stage->stage, scope);
    auto protos = root.find("prototypes");
    if (protos == root.end() || !protos->second.IsArray()) {
      message = "an instancer needs prototypes";
      return false;
    }
    UsdRelationship rel = instancer.CreatePrototypesRel();
    for (const JsValue &p : protos->second.GetJsArray()) {
      const SdfPath source(p.IsString() ? p.GetString() : std::string());
      if (source.IsEmpty() || !stage->stage->GetPrimAtPath(source)) {
        message = "no prototype at " + source.GetString();
        return false;
      }
      const SdfPath holder = get_unique_path(stage->stage, scope.AppendChild(source.GetNameToken()));
      UsdPrim prim = UsdGeomXform::Define(stage->stage, holder).GetPrim();
      /* Instanceable, so it shares OpenUSD's prototype with the source
       * item's own instanceable reference (one mesh on import). */
      if (!prim.GetReferences().AddInternalReference(source) || !prim.SetInstanceable(true)) {
        message = "couldn't reference " + source.GetString();
        return false;
      }
      rel.AddTarget(holder);
    }
    VtVec3fArray p(n), s(n);
    VtQuatfArray qf(n);
    VtQuathArray qh(n);
    VtInt64Array ids(n);
    for (size_t i = 0; i < n; i++) {
      p[i] = GfVec3f(positions[3 * i], positions[3 * i + 1], positions[3 * i + 2]);
      s[i] = GfVec3f(scales[3 * i], scales[3 * i + 1], scales[3 * i + 2]);
      const float *q = &orientations[4 * i];
      qf[i] = GfQuatf(q[3], q[0], q[1], q[2]);
      qh[i] = GfQuath(GfHalf(q[3]), GfHalf(q[0]), GfHalf(q[1]), GfHalf(q[2]));
      ids[i] = int64_t(i);
    }
    instancer.CreateProtoIndicesAttr().Set(proto_indices);
    instancer.CreatePositionsAttr().Set(p);
    /* orientationsf keeps full precision where it's read (OpenUSD 24.11
     * and later); orientations is for older readers. */
    instancer.CreateOrientationsAttr().Set(qh);
    instancer.CreateOrientationsfAttr().Set(qf);
    instancer.CreateScalesAttr().Set(s);
    instancer.CreateIdsAttr().Set(ids);
    std::vector<int64_t> invisible;
    if (get("invisibleIds", invisible) && !invisible.empty()) {
      instancer.CreateInvisibleIdsAttr().Set(VtInt64Array(invisible.begin(), invisible.end()));
    }
    set_custom(instancer.GetPrim(), root);
    set_text(path_out, path_len, path.GetString());
    return true;
  });
}

int ou_make_instance(OuStage *stage,
                     const char *path,
                     const char *prototype,
                     char *error,
                     size_t error_len)
{
  return guarded(error, error_len, "Couldn't write an instance", [&](std::string &message) {
    if (stage == nullptr || path == nullptr || prototype == nullptr) {
      message = "no stage, path or prototype";
      return false;
    }
    pxr::UsdPrim prim = stage->stage->GetPrimAtPath(pxr::SdfPath(path));
    if (!prim || !stage->stage->GetPrimAtPath(pxr::SdfPath(prototype))) {
      message = std::string("no prim at ") + path + " or " + prototype;
      return false;
    }
    if (!prim.GetReferences().AddInternalReference(pxr::SdfPath(prototype))) {
      return false;
    }
    /* Marks the holder, so the import takes its meshes as its parent
     * item's own (other programs' instances stay items of their own). */
    prim.SetCustomDataByKey(pxr::TfToken("operandi:instanceHolder"), pxr::VtValue(true));
    return prim.SetInstanceable(true);
  });
}

int ou_package_usdz(const char *source, const char *usdz_path, char *error, size_t error_len)
{
  return guarded(error, error_len, "Couldn't make the USDZ package", [&](std::string &message) {
    if (source == nullptr || usdz_path == nullptr) {
      message = "no source or destination";
      return false;
    }
    return pxr::UsdUtilsCreateNewUsdzPackage(pxr::SdfAssetPath(source), usdz_path);
  });
}

int ou_stage_save(OuStage *stage, char *error, size_t error_len)
{
  return guarded(error, error_len, "Couldn't save the USD file", [&](std::string &message) {
    if (stage == nullptr) {
      message = "no stage";
      return false;
    }
    return stage->layer->Save();
  });
}

char *ou_stage_text(OuStage *stage)
{
  if (stage == nullptr) {
    return nullptr;
  }
  try {
    std::string text;
    if (!stage->stage->ExportToString(&text)) {
      return nullptr;
    }
    char *out = static_cast<char *>(std::malloc(text.size() + 1));
    if (out != nullptr) {
      std::memcpy(out, text.c_str(), text.size() + 1);
    }
    return out;
  }
  catch (...) {
    return nullptr;
  }
}

void ou_string_free(char *text)
{
  std::free(text);
}

int ou_world_matrix(OuStage *stage, const char *path, double out[16])
{
  if (stage == nullptr || path == nullptr || out == nullptr) {
    return 0;
  }
  try {
    pxr::UsdGeomXformable xf(stage->stage->GetPrimAtPath(pxr::SdfPath(path)));
    if (!xf) {
      return 0;
    }
    const pxr::GfMatrix4d m = xf.ComputeLocalToWorldTransform(pxr::UsdTimeCode::Default());
    for (int r = 0; r < 4; r++) {
      for (int c = 0; c < 4; c++) {
        out[r * 4 + c] = m[r][c];
      }
    }
    return 1;
  }
  catch (...) {
    return 0;
  }
}

}  /* extern "C" */
