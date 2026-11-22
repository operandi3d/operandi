/* Operandi's bridge into Cycles: build a small Cycles scene (meshes and
 * their materials, a camera, lights and a uniform sky), render it (or a
 * region of it), report progress while it runs, and copy the pixels out.
 * See operandi_cycles.h. */

#include "operandi_cycles.h"

#include <algorithm>
#include <cctype>
#include <cfloat>
#include <cmath>
#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <exception>
#include <map>
#include <string>
#include <thread>

#include "device/device.h"
#include "scene/attribute.h"
#include "scene/background.h"
#include "scene/camera.h"
#include "scene/film.h"
#include "scene/light.h"
#include "scene/mesh.h"
#include "scene/object.h"
#include "scene/pass.h"
#include "scene/scene.h"
#include "scene/shader.h"
#include "scene/shader_graph.h"
#include "scene/shader_nodes.h"
#include "session/buffers.h"
#include "session/output_driver.h"
#include "session/session.h"
#include "scene/integrator.h"
#include "util/colorspace.h"
#include "util/path.h"
#include "util/set.h"
#include "util/transform.h"
#include "util/version.h"

using namespace ccl;

namespace {

void set_error(char *error, size_t error_len, const std::string &message)
{
  if (error == nullptr || error_len == 0) {
    return;
  }
  std::snprintf(error, error_len, "%s", message.c_str());
}

/* The colour space a picture is read in. Never Cycles' "auto": in this
 * build it reads 8-bit pictures as plain numbers, so colour maps came out
 * pale. Data maps (roughness, metal, heights, normals) stay as numbers;
 * float pictures (EXR, HDR, PFM) are already linear; every other picture
 * (JPEG, PNG, 16-bit TIFF...) is sRGB-encoded and decoded to linear. */
ustring picture_colorspace(const char *path, const bool data)
{
  if (data) {
    return u_colorspace_data;
  }
  std::string ext = path != nullptr ? path : "";
  const size_t dot = ext.find_last_of('.');
  ext = dot == std::string::npos ? std::string() : ext.substr(dot + 1);
  std::transform(ext.begin(), ext.end(), ext.begin(), [](unsigned char c) {
    return char(std::tolower(c));
  });
  if (ext == "exr" || ext == "hdr" || ext == "pfm") {
    return u_colorspace_scene_linear;
  }
  return u_colorspace_srgb;
}

float3 f3(const float v[3])
{
  return make_float3(v[0], v[1], v[2]);
}

uint32_t load_command(const OcControl *control)
{
  if (control == nullptr || control->command == nullptr) {
    return OC_RUN;
  }
#if defined(_MSC_VER)
  return *reinterpret_cast<const volatile uint32_t *>(control->command);
#else
  return __atomic_load_n(control->command, __ATOMIC_ACQUIRE);
#endif
}

/* The rendered area (the region, or the whole picture): Cycles' tiles are
 * gathered into it top-row-first, shown while rendering through
 * OcControl.on_preview, and copied into the picture when finished. */
class BufferOutputDriver : public OutputDriver {
 public:
  BufferOutputDriver(
      float *out, int full_width, int x, int y, int width, int height, const OcControl *control)
  {
    retarget(out, full_width, x, y, width, height, control);
  }

  /* Points the driver at the next render's picture (a kept session renders
   * many times through one driver). */
  void retarget(
      float *out, int full_width, int x, int y, int width, int height, const OcControl *control)
  {
    out_ = out;
    full_width_ = full_width;
    x_ = x;
    y_ = y;
    width_ = width;
    height_ = height;
    control_ = control;
    area_.assign(size_t(width) * height * 4, 0.0f);
    written = false;
    groups.clear();
  }

  bool update_render_tile(const Tile &tile) override
  {
    if (control_ == nullptr || control_->on_preview == nullptr || !gather(tile)) {
      return false;
    }
    control_->on_preview(control_->user, area_.data(), uint32_t(width_), uint32_t(height_));
    return true;
  }

  void write_render_tile(const Tile &tile) override
  {
    if (!gather(tile)) {
      return;
    }
    const size_t row = size_t(width_) * 4;
    for (int y = 0; y < height_; y++) {
      std::memcpy(out_ + (size_t(y_ + y) * full_width_ + x_) * 4,
                  area_.data() + size_t(y) * row,
                  row * sizeof(float));
    }
    /* Light-group passes, into their own pictures. */
    vector<float> group_area(area_.size());
    for (const auto &[name, out] : groups) {
      if (!gather_pass(tile, name.c_str(), group_area)) {
        continue;
      }
      for (int y = 0; y < height_; y++) {
        std::memcpy(out + (size_t(y_ + y) * full_width_ + x_) * 4,
                    group_area.data() + size_t(y) * row,
                    row * sizeof(float));
      }
    }
    if (control_ != nullptr && control_->on_preview != nullptr) {
      control_->on_preview(control_->user, area_.data(), uint32_t(width_), uint32_t(height_));
    }
    written = true;
  }

  bool written = false;
  /* Light-group pass names and where each picture goes. */
  vector<std::pair<std::string, float *>> groups;

 private:
  bool gather(const Tile &tile)
  {
    return gather_pass(tile, "combined", area_);
  }

  /* Copies one tile's pass (rows bottom-first, offset from the area's
   * bottom-left corner) into `area`. */
  bool gather_pass(const Tile &tile, const char *pass, vector<float> &area)
  {
    if (tile.full_size.x != width_ || tile.full_size.y != height_ || tile.offset.x < 0 ||
        tile.offset.y < 0 || tile.offset.x + tile.size.x > width_ ||
        tile.offset.y + tile.size.y > height_)
    {
      return false;
    }
    vector<float> pixels(size_t(tile.size.x) * tile.size.y * 4);
    if (!tile.get_pass_pixels(pass, 4, pixels.data())) {
      return false;
    }
    const size_t tile_row = size_t(tile.size.x) * 4;
    for (int y = 0; y < tile.size.y; y++) {
      const int top_row = height_ - 1 - (tile.offset.y + y);
      std::memcpy(area.data() + (size_t(top_row) * width_ + tile.offset.x) * 4,
                  pixels.data() + size_t(y) * tile_row,
                  tile_row * sizeof(float));
    }
    return true;
  }

  float *out_ = nullptr;
  int full_width_ = 0;
  int x_ = 0;
  int y_ = 0;
  int width_ = 0;
  int height_ = 0;
  const OcControl *control_ = nullptr;
  vector<float> area_;
};

/* A rotation whose -Z axis points along dir (how Cycles aims a sun). */
Transform aim_along(const float3 dir)
{
  const float3 z = -normalize(dir);
  const float3 seed = fabsf(z.y) < 0.99f ? make_float3(0.0f, 1.0f, 0.0f) :
                                           make_float3(1.0f, 0.0f, 0.0f);
  const float3 x = normalize(cross(seed, z));
  const float3 y = cross(z, x);
  return make_transform(x.x, y.x, z.x, 0.0f, x.y, y.y, z.y, 0.0f, x.z, y.z, z.z, 0.0f);
}

/* Principled BSDF socket per OC_INPUT_* (operandi_cycles.h). */
const char *const INPUT_NAMES[OC_INPUT_COUNT] = {
    "Base Color",
    "Metallic",
    "Roughness",
    "IOR",
    "Alpha",
    "Diffuse Roughness",
    "Subsurface Weight",
    "Subsurface Radius",
    "Subsurface Scale",
    "Specular IOR Level",
    "Specular Tint",
    "Anisotropic",
    "Transmission Weight",
    "Sheen Weight",
    "Coat Weight",
    "Coat Roughness",
    "Emission Color",
    "Emission Strength",
    "Subsurface Anisotropy",
    "Coat Tint",
    "Coat IOR",
    "Sheen Tint",
    "Sheen Roughness",
    "Thin Film Thickness",
    "Thin Film IOR",
    "Transmission Dispersion Scale",
    "Transmission Dispersion Abbe Number",
};

/* Builds a shader tree surface: its nodes in order, then the principled
 * inputs they feed. Constants are set on the sockets directly. */
/* A light's visible beam (OcSurface.volume): volume scatter only. */
Shader *build_volume(Scene *scene, const OcSurface &src)
{
  unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
  GeometryNode *geometry = graph->create_node<GeometryNode>();
  VectorMathNode *offset = graph->create_node<VectorMathNode>();
  offset->set_math_type(NODE_VECTOR_MATH_SUBTRACT);
  graph->connect(geometry->output("Position"), offset->input("Vector1"));
  offset->set_vector2(f3(src.volume_origin));
  VectorMathNode *distance = graph->create_node<VectorMathNode>();
  distance->set_math_type(NODE_VECTOR_MATH_LENGTH);
  graph->connect(offset->output("Vector"), distance->input("Vector1"));
  MathNode *fade = graph->create_node<MathNode>();
  fade->set_math_type(NODE_MATH_POWER);
  fade->set_value1(1.0f - src.volume_attenuation);
  graph->connect(distance->output("Value"), fade->input("Value2"));
  MathNode *density = graph->create_node<MathNode>();
  density->set_math_type(NODE_MATH_MULTIPLY);
  density->set_value2(src.volume_density);
  graph->connect(fade->output("Value"), density->input("Value1"));
  ScatterVolumeNode *scatter = graph->create_node<ScatterVolumeNode>();
  scatter->set_color(f3(src.volume_color));
  graph->connect(density->output("Value"), scatter->input("Density"));
  graph->connect(scatter->output("Volume"), graph->output()->input("Volume"));
  Shader *shader = scene->create_node<Shader>();
  shader->set_graph(std::move(graph));
  shader->tag_update(scene);
  return shader;
}

/* Procedural patterns (OC_EXPR_PATTERN) and Bias/Gain, built from stock
 * Math and Noise Texture nodes so every device renders them. Each follows
 * operandi-core's procedural.rs step for step. */

/* A float in the graph: a node output, or a constant when `out` is null. */
struct Val {
  ShaderOutput *out = nullptr;
  float k = 0.0f;
};

struct PatternBuilder {
  ShaderGraph *graph;

  static Val c(float v)
  {
    return Val{nullptr, v};
  }
  void put(ShaderInput *in, Val v, MathNode *m, int slot)
  {
    if (v.out != nullptr) {
      graph->connect(v.out, in);
    }
    else if (slot == 0) {
      m->set_value1(v.k);
    }
    else if (slot == 1) {
      m->set_value2(v.k);
    }
    else {
      m->set_value3(v.k);
    }
  }
  Val op(NodeMathType type, Val a, Val b = c(0.0f), Val d = c(0.0f), bool clamp = false)
  {
    MathNode *m = graph->create_node<MathNode>();
    m->set_math_type(type);
    m->set_use_clamp(clamp);
    put(m->input("Value1"), a, m, 0);
    put(m->input("Value2"), b, m, 1);
    put(m->input("Value3"), d, m, 2);
    return Val{m->output("Value"), 0.0f};
  }
  Val add(Val a, Val b)
  {
    return op(NODE_MATH_ADD, a, b);
  }
  Val sub(Val a, Val b)
  {
    return op(NODE_MATH_SUBTRACT, a, b);
  }
  Val mul(Val a, Val b)
  {
    return op(NODE_MATH_MULTIPLY, a, b);
  }
  /* a * m + k */
  Val madd(Val a, float m, float k, bool clamp = false)
  {
    return op(NODE_MATH_MULTIPLY_ADD, a, c(m), c(k), clamp);
  }
  Val clamp01(Val a)
  {
    return op(NODE_MATH_ADD, a, c(0.0f), c(0.0f), true);
  }
  Val floor_(Val a)
  {
    return op(NODE_MATH_FLOOR, a);
  }
  Val fract(Val a)
  {
    return op(NODE_MATH_FRACTION, a);
  }
  Val abs_(Val a)
  {
    return op(NODE_MATH_ABSOLUTE, a);
  }
  Val min_(Val a, Val b)
  {
    return op(NODE_MATH_MINIMUM, a, b);
  }
  Val max_(Val a, Val b)
  {
    return op(NODE_MATH_MAXIMUM, a, b);
  }
  Val modulo(Val a, float b)
  {
    return op(NODE_MATH_FLOORED_MODULO, a, c(b));
  }
  Val round_(Val a)
  {
    return floor_(add(a, c(0.5f)));
  }
  Val hypot2(Val a, Val b)
  {
    return op(NODE_MATH_SQRT, add(mul(a, a), mul(b, b)));
  }
  /* 0 below e0, 1 above e1, smoothstep between; a hard step at e0 when the
   * ramp has no width (x <= e0 gives 0). */
  Val smoothstep(float e0, float e1, Val x)
  {
    if (e1 <= e0) {
      return op(NODE_MATH_GREATER_THAN, x, c(e0));
    }
    const float s = 1.0f / (e1 - e0);
    Val t = madd(x, s, -e0 * s, true);
    return mul(mul(t, t), madd(t, -2.0f, 3.0f));
  }
  /* procedural.rs soft_parity */
  Val soft_parity(Val x, float width)
  {
    Val p = mul(x, c(2.0f));
    Val fl = floor_(p);
    Val sign = madd(fract(mul(fl, c(0.5f))), -4.0f, 1.0f);
    Val f = sub(p, fl);
    Val d = min_(f, sub(c(1.0f), f));
    return mul(sign, smoothstep(0.0f, width * 0.5f, d));
  }
  Val line_dist(Val x)
  {
    return abs_(sub(fract(x), c(0.5f)));
  }
  Val line_cover(Val d, float half, float transition)
  {
    return sub(c(1.0f), smoothstep(half - transition * 0.5f, half + transition * 0.5f, d));
  }
  /* procedural.rs hex_edge_dist */
  Val hex_edge_dist(Val u, Val v)
  {
    const float r1 = sqrtf(3.0f), h0 = 0.5f, h1 = sqrtf(3.0f) * 0.5f;
    Val ax = sub(modulo(u, 1.0f), c(h0));
    Val ay = sub(modulo(v, r1), c(h1));
    Val bx = sub(modulo(sub(u, c(h0)), 1.0f), c(h0));
    Val by = sub(modulo(sub(v, c(h1)), r1), c(h1));
    Val da = add(mul(ax, ax), mul(ay, ay));
    Val db = add(mul(bx, bx), mul(by, by));
    Val pick_a = op(NODE_MATH_LESS_THAN, da, db);
    Val gx = abs_(add(bx, mul(pick_a, sub(ax, bx))));
    Val gy = abs_(add(by, mul(pick_a, sub(ay, by))));
    Val dist = max_(gx, add(mul(gx, c(0.5f)), mul(gy, c(h1))));
    return sub(c(0.5f), dist);
  }

  /* A vector's three components. */
  void xyz(ShaderOutput *vector, Val out[3])
  {
    SeparateXYZNode *sep = graph->create_node<SeparateXYZNode>();
    graph->connect(vector, sep->input("Vector"));
    out[0] = Val{sep->output("X")};
    out[1] = Val{sep->output("Y")};
    out[2] = Val{sep->output("Z")};
  }
  ShaderOutput *combine(Val x, Val y, Val z)
  {
    CombineXYZNode *xyz = graph->create_node<CombineXYZNode>();
    const char *names[3] = {"X", "Y", "Z"};
    Val v[3] = {x, y, z};
    for (int k = 0; k < 3; k++) {
      if (v[k].out != nullptr) {
        graph->connect(v[k].out, xyz->input(names[k]));
      }
      else {
        xyz->input(names[k])->set(v[k].k);
      }
    }
    return xyz->output("Vector");
  }
  /* shader_eval::wrap_coords, then ((u0, v0) + centre) * repeat. */
  ShaderOutput *wrap(ShaderOutput *vector, uint32_t mode, const float *value)
  {
    Val l[3];
    xyz(vector, l);
    const int axis = int(mode & 3), from = (axis + 1) % 3, toward = (axis + 2) % 3;
    Val u0 = mul(op(NODE_MATH_ARCTAN2, l[toward], l[from]), c(1.0f / (2.0f * float(M_PI))));
    Val v0 = (mode & 4) ? mul(op(NODE_MATH_ARCTAN2, l[axis], hypot2(l[from], l[toward])),
                              c(1.0f / float(M_PI))) :
                          l[axis];
    return combine(madd(u0, value[1], value[0] * value[1]),
                   madd(v0, value[2], value[0] * value[2]),
                   c(0.0f));
  }
  /* shader_eval::probe_coords: Light Probe (mirror ball) coordinates of locator
   * vector a, the Projection Axis (mode) the picture's up; x repeat value[0..2]. */
  ShaderOutput *probe(ShaderOutput *vector, uint32_t mode, const float *value)
  {
    Val l[3];
    xyz(vector, l);
    const int axis = int(mode & 3);
    ShaderOutput *frame = combine(l[(axis + 2) % 3], l[axis], l[(axis + 1) % 3]);
    VectorMathNode *unit = graph->create_node<VectorMathNode>();
    unit->set_math_type(NODE_VECTOR_MATH_NORMALIZE);
    graph->connect(frame, unit->input("Vector1"));
    VectorMathNode *half = graph->create_node<VectorMathNode>();
    half->set_math_type(NODE_VECTOR_MATH_ADD);
    graph->connect(unit->output("Vector"), half->input("Vector1"));
    half->set_vector2(make_float3(0.0f, 0.0f, 1.0f));
    VectorMathNode *n = graph->create_node<VectorMathNode>();
    n->set_math_type(NODE_VECTOR_MATH_NORMALIZE);
    graph->connect(half->output("Vector"), n->input("Vector1"));
    Val d[3];
    xyz(n->output("Vector"), d);
    return combine(madd(d[0], 0.5f * value[0], 0.5f * value[0]),
                   madd(d[1], 0.5f * value[1], 0.5f * value[1]),
                   c(0.0f));
  }
  /* The world normal through a 3x3 matrix, normalised. */
  ShaderOutput *normal(const float *m)
  {
    GeometryNode *geo = graph->create_node<GeometryNode>();
    Val r[3];
    for (int k = 0; k < 3; k++) {
      VectorMathNode *dot = graph->create_node<VectorMathNode>();
      dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
      graph->connect(geo->output("Normal"), dot->input("Vector1"));
      dot->set_vector2(make_float3(m[k * 3], m[k * 3 + 1], m[k * 3 + 2]));
      r[k] = Val{dot->output("Value")};
    }
    VectorMathNode *unit = graph->create_node<VectorMathNode>();
    unit->set_math_type(NODE_VECTOR_MATH_NORMALIZE);
    graph->connect(combine(r[0], r[1], r[2]), unit->input("Vector1"));
    return unit->output("Vector");
  }
  /* shader_eval::face_weight */
  ShaderOutput *face_weight(ShaderOutput *normal, uint32_t mode, float k)
  {
    Val n[3];
    xyz(normal, n);
    Val a[3] = {abs_(n[0]), abs_(n[1]), abs_(n[2])};
    const int face = int(mode & 7), kind = int(mode >> 3);
    Val w;
    if (kind == 2) {
      Val p[3];
      for (int i = 0; i < 3; i++) {
        p[i] = op(NODE_MATH_POWER, a[i], c(k));
      }
      Val total = op(NODE_MATH_MAXIMUM, add(add(p[0], p[1]), p[2]), c(1e-12f));
      w = op(NODE_MATH_DIVIDE, p[std::min(face, 2)], total);
    }
    else {
      /* X on ties with Y or Z, Y on ties with Z. */
      auto not_less = [&](Val x, Val y) { return sub(c(1.0f), op(NODE_MATH_LESS_THAN, x, y)); };
      Val dom[3] = {
          mul(not_less(a[0], a[1]), not_less(a[0], a[2])),
          mul(op(NODE_MATH_GREATER_THAN, a[1], a[0]), not_less(a[1], a[2])),
          mul(op(NODE_MATH_GREATER_THAN, a[2], a[0]), op(NODE_MATH_GREATER_THAN, a[2], a[1])),
      };
      const int axis = face % 3;
      w = dom[axis];
      if (kind == 0) {
        Val positive = sub(c(1.0f), op(NODE_MATH_LESS_THAN, n[axis], c(0.0f)));
        w = mul(w, face < 3 ? positive : sub(c(1.0f), positive));
      }
    }
    if (w.out == nullptr) {
      w = add(w, c(0.0f));
    }
    return w.out;
  }
  ShaderOutput *facing(ShaderOutput *normal, uint32_t axis)
  {
    Val n[3];
    xyz(normal, n);
    return op(NODE_MATH_GREATER_THAN, n[std::min(axis, 2u)], c(0.0f)).out;
  }
  /* shader_eval::falloff */
  ShaderOutput *falloff(ShaderOutput *vector, uint32_t kind, const float *f)
  {
    Val l[3];
    xyz(vector, l);
    Val d[3];
    for (int i = 0; i < 3; i++) {
      d[i] = abs_(mul(l[i], c(f[i])));
    }
    Val reach;
    switch (kind) {
      case 1:
        reach = op(NODE_MATH_SQRT, add(add(mul(d[0], d[0]), mul(d[1], d[1])), mul(d[2], d[2])));
        break;
      case 2:
      case 3:
      case 4:
        reach = d[kind - 2];
        break;
      default:
        reach = max_(max_(d[0], d[1]), d[2]);
        break;
    }
    return madd(reach, -1.0f, 1.0f, true).out;
  }

  /* procedural.rs ripple_hash */
  static uint32_t ripple_hash(uint32_t x)
  {
    x ^= x >> 16;
    x *= 0x7feb352du;
    x ^= x >> 15;
    x *= 0x846ca68bu;
    return x ^ (x >> 16);
  }
  /* Noise Texture, one octave, signed (-1..1). */
  Val snoise(ShaderOutput *vector, float scale)
  {
    NoiseTextureNode *n = graph->create_node<NoiseTextureNode>();
    n->set_dimensions(3);
    n->set_use_normalize(true);
    n->set_detail(0.0f);
    n->set_scale(scale);
    graph->connect(vector, n->input("Vector"));
    return madd(Val{n->output("Fac")}, 2.0f, -1.0f);
  }
  VoronoiTextureNode *voronoi(ShaderOutput *vector,
                              float scale,
                              NodeVoronoiFeature feature,
                              int dimensions = 3,
                              float randomness = 1.0f)
  {
    VoronoiTextureNode *v = graph->create_node<VoronoiTextureNode>();
    v->set_dimensions(dimensions);
    v->set_feature(feature);
    v->set_scale(scale);
    v->set_randomness(randomness);
    graph->connect(vector, v->input("Vector"));
    return v;
  }
  /* procedural.rs weave_shade */
  Val weave_shade(Val u, Val v, float width, float roundness)
  {
    Val pu = mul(u, c(2.0f)), pv = mul(v, c(2.0f));
    Val fu = fract(pu), fv = fract(pv);
    Val sum = add(floor_(pu), floor_(pv));
    Val top_h = op(NODE_MATH_LESS_THAN, sub(sum, mul(c(2.0f), floor_(mul(sum, c(0.5f))))), c(0.5f));
    const float inv_half = 1.0f / std::max(width * 0.5f, 1e-6f);
    Val xh = mul(abs_(sub(fv, c(0.5f))), c(inv_half));
    Val xv = mul(abs_(sub(fu, c(0.5f))), c(inv_half));
    Val in_h = op(NODE_MATH_LESS_THAN, xh, c(1.0f));
    Val in_v = op(NODE_MATH_LESS_THAN, xv, c(1.0f));
    auto profile = [&](Val x) {
      Val tube = op(NODE_MATH_SQRT, max_(sub(c(1.0f), mul(x, x)), c(0.0f)));
      return madd(sub(tube, c(1.0f)), roundness, 1.0f);
    };
    Val not_top = sub(c(1.0f), top_h);
    Val sel_h = mul(in_h, add(top_h, mul(not_top, sub(c(1.0f), in_v))));
    Val sel_v = mul(in_v, add(not_top, mul(top_h, sub(c(1.0f), in_h))));
    return add(mul(sel_h, profile(xh)), mul(sel_v, profile(xv)));
  }
  /* Pattern::cell_random: three randoms per cell (white noise of the cell's
   * id; Cellular: the Voronoi cell colour). */
  ShaderOutput *cell_random(ShaderOutput *vector, uint32_t mode, const float *p)
  {
    if (mode == 4) {
      return voronoi(vector, 1.0f, NODE_VORONOI_F1)->output("Color");
    }
    if (mode == 9) {
      const bool solid = p[5] != 0.0f;
      return voronoi(poisson_input(vector, int(p[1]), solid), p[4], NODE_VORONOI_F1, solid ? 3 : 2, p[10])
          ->output("Color");
    }
    Val l[3];
    xyz(vector, l);
    const int sub_type = int(p[0]);
    const int axis = int(p[1]);
    const Val u = axis == 0 ? mul(l[2], c(-1.0f)) : l[0];
    const Val v = axis == 1 ? mul(l[2], c(-1.0f)) : l[1];
    Val id[3] = {c(0.0f), c(0.0f), c(0.0f)};
    if (mode == 0) { /* Checker */
      if (sub_type == 1) {
        for (int k = 0; k < 3; k++) {
          id[k] = floor_(mul(l[k], c(2.0f)));
        }
      }
      else {
        id[0] = floor_(mul(u, c(2.0f)));
        id[1] = floor_(mul(v, c(2.0f)));
      }
    }
    else if (mode == 1) { /* Grid */
      switch (sub_type) {
        case 1:
          id[0] = round_(u);
          break;
        case 2:
          id[0] = round_(u);
          id[1] = round_(v);
          id[2] = op(NODE_MATH_GREATER_THAN, fract(add(sub(u, v), c(0.5f))), c(0.5f));
          break;
        case 3: {
          /* the hexagonal cell's centre (procedural.rs hex_centre) */
          const float r1 = sqrtf(3.0f), h0 = 0.5f, h1 = sqrtf(3.0f) * 0.5f;
          Val ax = sub(modulo(u, 1.0f), c(h0));
          Val ay = sub(modulo(v, r1), c(h1));
          Val bx = sub(modulo(sub(u, c(h0)), 1.0f), c(h0));
          Val by = sub(modulo(sub(v, c(h1)), r1), c(h1));
          Val pick_a = op(NODE_MATH_LESS_THAN, add(mul(ax, ax), mul(ay, ay)),
                          add(mul(bx, bx), mul(by, by)));
          Val gx = add(bx, mul(pick_a, sub(ax, bx)));
          Val gy = add(by, mul(pick_a, sub(ay, by)));
          id[0] = round_(mul(sub(u, gx), c(2.0f)));
          id[1] = round_(mul(sub(v, gy), c(2.0f / r1)));
          break;
        }
        case 4:
          for (int k = 0; k < 3; k++) {
            id[k] = round_(l[k]);
          }
          break;
        default:
          id[0] = round_(u);
          id[1] = round_(v);
          break;
      }
    }
    else { /* Dots */
      if (sub_type == 2) {
        for (int k = 0; k < 3; k++) {
          id[k] = round_(l[k]);
        }
      }
      else if (sub_type == 1) {
        Val row = round_(v);
        Val shift = mul(c(0.5f), sub(row, mul(c(2.0f), floor_(mul(row, c(0.5f))))));
        id[0] = round_(sub(u, shift));
        id[1] = row;
      }
      else {
        id[0] = round_(u);
        id[1] = round_(v);
      }
    }
    WhiteNoiseTextureNode *noise = graph->create_node<WhiteNoiseTextureNode>();
    noise->set_dimensions(3);
    graph->connect(combine(id[0], id[1], id[2]), noise->input("Vector"));
    return noise->output("Color");
  }
  /* Poisson Cells read the in-plane coordinates (2D) or the full point. */
  ShaderOutput *poisson_input(ShaderOutput *vector, int axis, bool solid)
  {
    if (solid) {
      return vector;
    }
    Val l[3];
    xyz(vector, l);
    const Val u = axis == 0 ? mul(l[2], c(-1.0f)) : l[0];
    const Val v = axis == 1 ? mul(l[2], c(-1.0f)) : l[1];
    return combine(u, v, c(0.0f));
  }

  /* shader_eval::hsv_vary through Cycles' Hue/Saturation/Value node. */
  ShaderOutput *hsv_vary(ShaderOutput *color, ShaderOutput *random, const float *a)
  {
    Val r[3];
    xyz(random, r);
    HSVNode *hsv = graph->create_node<HSVNode>();
    graph->connect(color, hsv->input("Color"));
    auto set = [&](const char *name, Val v) {
      if (v.out != nullptr) {
        graph->connect(v.out, hsv->input(name));
      }
      else {
        hsv->input(name)->set(v.k);
      }
    };
    set("Value", madd(r[0], -a[0], 1.0f));
    set("Hue", madd(r[1], a[1], 0.5f - 0.5f * a[1]));
    set("Saturation", madd(r[2], -a[2], 1.0f));
    hsv->set_fac(1.0f);
    return hsv->output("Color");
  }

  ShaderOutput *pattern(ShaderOutput *vector, uint32_t mode, const float *p)
  {
    SeparateXYZNode *sep = graph->create_node<SeparateXYZNode>();
    graph->connect(vector, sep->input("Vector"));
    const Val x{sep->output("X")}, y{sep->output("Y")}, z{sep->output("Z")};
    const int sub_type = int(p[0]);
    const int axis = int(p[1]);
    /* The plane perpendicular to the axis: X (-z, y), Y (x, -z), Z (x, y). */
    const Val u = axis == 0 ? mul(z, c(-1.0f)) : x;
    const Val v = axis == 1 ? mul(z, c(-1.0f)) : y;
    Val t;
    switch (mode) {
      case 0: { /* Checker */
        const float w = p[2];
        Val s = sub_type == 1 ? mul(mul(soft_parity(x, w), soft_parity(y, w)), soft_parity(z, w)) :
                                mul(soft_parity(u, w), soft_parity(v, w));
        t = madd(s, 0.5f, 0.5f);
        break;
      }
      case 1: { /* Grid */
        const float half = p[2] * 0.5f, tr = p[3];
        auto cover = [&](Val d) { return line_cover(d, half, tr); };
        Val line;
        switch (sub_type) {
          case 1:
            line = cover(line_dist(u));
            break;
          case 2: {
            Val diagonal = mul(line_dist(add(sub(u, v), c(0.5f))), c(1.0f / sqrtf(2.0f)));
            line = max_(max_(cover(line_dist(u)), cover(line_dist(v))), cover(diagonal));
            break;
          }
          case 3:
            line = cover(hex_edge_dist(u, v));
            break;
          case 4:
            line = max_(max_(cover(line_dist(x)), cover(line_dist(y))), cover(line_dist(z)));
            break;
          default:
            line = max_(cover(line_dist(u)), cover(line_dist(v)));
            break;
        }
        t = sub(c(1.0f), line);
        break;
      }
      case 2: { /* Dots */
        Val r;
        if (sub_type == 1) {
          Val row = round_(v);
          Val shift = mul(c(0.5f), sub(row, mul(c(2.0f), floor_(mul(row, c(0.5f))))));
          Val us = sub(u, shift);
          r = hypot2(sub(us, round_(us)), sub(v, row));
        }
        else if (sub_type == 2) {
          Val qx = sub(x, round_(x)), qy = sub(y, round_(y)), qz = sub(z, round_(z));
          r = op(NODE_MATH_SQRT, add(add(mul(qx, qx), mul(qy, qy)), mul(qz, qz)));
        }
        else {
          r = hypot2(sub(u, round_(u)), sub(v, round_(v)));
        }
        t = sub(c(1.0f), line_cover(r, p[2] * 0.5f, p[3]));
        break;
      }
      case 4: { /* Cellular: Voronoi F1 (and F2 for Angular) per octave */
        const bool angular = sub_type == 0;
        const int octaves = std::max(1, std::min(int(p[4]), 8));
        const float width = p[2], tr = p[3], freq_ratio = p[5], amp_ratio = p[6], smooth_tops = p[7];
        float amplitude = 1.0f, frequency = 1.0f, total = 0.0f;
        Val first, rest = c(0.0f);
        for (int i = 0; i < octaves; i++) {
          Val f1{voronoi(vector, frequency, NODE_VORONOI_F1)->output("Distance")};
          Val d;
          if (angular) {
            Val f2{voronoi(vector, frequency, NODE_VORONOI_F2)->output("Distance")};
            /* 1 - clamp(2 (F2 - F1)): 0 deep inside, 1 on the edge */
            d = madd(sub(f2, f1), -2.0f, 1.0f, true);
          }
          else {
            d = mul(f1, c(2.0f));
          }
          Val ti = sub(c(1.0f), line_cover(d, width, tr));
          if (i == 0) {
            first = ti;
          }
          else {
            rest = add(rest, mul(ti, c(amplitude)));
          }
          total += amplitude;
          amplitude *= amp_ratio;
          frequency *= freq_ratio;
        }
        if (smooth_tops != 0.0f && octaves > 1) {
          rest = mul(rest, madd(clamp01(first), -smooth_tops, 1.0f));
        }
        t = mul(add(first, rest), c(1.0f / std::max(total, 1e-12f)));
        break;
      }
      case 5: { /* Wood */
        const float ring = std::max(p[2], 1e-6f), waviness = p[3];
        const float wave = std::max(p[4], 1e-6f), grain_amount = p[5], grain = std::max(p[6], 1e-6f);
        Val r = add(hypot2(u, v), mul(snoise(vector, 1.0f / wave), c(waviness * ring)));
        Val rings = madd(op(NODE_MATH_COSINE, mul(r, c(2.0f * float(M_PI) / ring))), 0.5f, 0.5f);
        Val grains = madd(snoise(vector, 1.0f / grain), 0.5f, 0.5f);
        const float g = grain_amount * 0.5f;
        t = add(mul(rings, c(1.0f - g)), mul(grains, c(g)));
        break;
      }
      case 6: { /* Ripples: sources from ripple_hash, as procedural.rs */
        const int n = std::max(1, std::min(int(p[2]), 32));
        const float wavelength = std::max(p[3], 1e-6f), phase = p[4];
        Val sum = c(0.0f);
        for (int i = 0; i < n; i++) {
          float centre[3];
          for (int k = 0; k < 3; k++) {
            const uint32_t h = ripple_hash(uint32_t(i) * 3u + uint32_t(k));
            centre[k] = float(h >> 8) / float(1u << 24) - 0.5f;
          }
          VectorMathNode *dist = graph->create_node<VectorMathNode>();
          dist->set_math_type(NODE_VECTOR_MATH_DISTANCE);
          graph->connect(vector, dist->input("Vector1"));
          dist->set_vector2(make_float3(centre[0], centre[1], centre[2]));
          Val d{dist->output("Value")};
          Val wave = op(NODE_MATH_COSINE,
                        madd(d, 2.0f * float(M_PI) / wavelength, 2.0f * float(M_PI) * phase));
          sum = add(sum, wave);
        }
        t = madd(sum, -0.5f / float(n), 0.5f);
        break;
      }
      case 7: /* Weave */
        t = sub(c(1.0f), weave_shade(u, v, p[2], p[3]));
        break;
      case 8: { /* Gabor Noise: Cycles' Gabor Texture, 2D in the plane */
        GaborTextureNode *g = graph->create_node<GaborTextureNode>();
        g->set_type(NODE_GABOR_TYPE_2D);
        g->set_scale(p[6]);
        g->set_frequency(p[3]);
        g->set_anisotropy(1.0f - std::min(std::max(p[4], 0.0f), 1.0f));
        g->set_orientation_2d(p[2]);
        const float off = p[5];
        graph->connect(combine(add(u, c(off * 17.31f / p[6])), add(v, c(off * 31.71f / p[6])), c(0.0f)),
                       g->input("Vector"));
        t = sub(c(1.0f), smoothstep(p[7], p[8], abs_(Val{g->output("Value")})));
        break;
      }
      case 9: { /* Poisson Cells: low-randomness Voronoi, Level sets the scale */
        const bool angular = sub_type == 0, solid = p[5] != 0.0f, profile = p[9] != 0.0f;
        const int octaves = std::max(1, std::min(int(p[6]), 8));
        const float width = p[2], tr = p[3];
        ShaderOutput *in = poisson_input(vector, axis, solid);
        float amplitude = 1.0f, frequency = 1.0f, total = 0.0f;
        Val sum = c(0.0f);
        for (int i = 0; i < octaves; i++) {
          const float scale = p[4] * frequency;
          const int dims = solid ? 3 : 2;
          Val f1{voronoi(in, scale, NODE_VORONOI_F1, dims, p[10])->output("Distance")};
          Val d;
          if (angular) {
            Val f2{voronoi(in, scale, NODE_VORONOI_F2, dims, p[10])->output("Distance")};
            d = madd(sub(f2, f1), -2.0f, 1.0f, true);
          }
          else {
            d = mul(f1, c(2.0f));
          }
          Val ti = profile ? smoothstep(0.0f, std::max(width, 1e-6f), d) :
                             sub(c(1.0f), line_cover(d, width, tr));
          sum = add(sum, mul(ti, c(amplitude)));
          total += amplitude;
          amplitude *= p[8];
          frequency *= p[7];
        }
        t = mul(sum, c(1.0f / std::max(total, 1e-12f)));
        break;
      }
      default: { /* Noise: one Noise Texture per octave (procedural.rs noise_value) */
        const bool turbulence = sub_type == 2;
        const int octaves = std::max(1, std::min(int(p[2]), 8));
        const float freq_ratio = p[3], amp_ratio = p[4], smooth_tops = p[5];
        const bool flow = p[6] != 0.0f;
        float amplitude = 1.0f, frequency = 1.0f, total = 0.0f;
        Val first, rest = c(0.0f);
        /* Flow Noise loops every 1.0 of Flow Phase, like the viewport
         * (procedural.rs gradient rotation, PT-Q09): f = the phase's
         * fraction; each octave blends 4D noise at W = f and at W = f - 1
         * with cos/sin weights (keeps the contrast), so f -> 1 lands back
         * on W = 0. */
        const float flow_f = flow ? p[7] - std::floor(p[7]) : 0.0f;
        const float flow_a = std::cos(flow_f * 1.57079633f);
        const float flow_b = std::sin(flow_f * 1.57079633f);
        for (int i = 0; i < octaves; i++) {
          auto octave = [&](float w) {
            NoiseTextureNode *n = graph->create_node<NoiseTextureNode>();
            n->set_dimensions(flow ? 4 : 3);
            n->set_use_normalize(true);
            n->set_detail(0.0f);
            n->set_scale(frequency);
            if (flow) {
              n->set_w(w);
            }
            graph->connect(vector, n->input("Vector"));
            return madd(Val{n->output("Fac")}, 2.0f, -1.0f);
          };
          Val s = octave(flow_f);
          if (flow && flow_f > 0.0f) {
            s = add(mul(s, c(flow_a)), mul(octave(flow_f - 1.0f), c(flow_b)));
          }
          if (turbulence) {
            s = abs_(s);
          }
          if (i == 0) {
            first = s;
          }
          else {
            rest = add(rest, mul(s, c(amplitude)));
          }
          total += amplitude;
          amplitude *= amp_ratio;
          frequency *= freq_ratio;
        }
        if (smooth_tops != 0.0f && octaves > 1) {
          Val t0 = turbulence ? madd(first, 2.0f, 0.0f, true) : madd(first, 0.5f, 0.5f, true);
          rest = mul(rest, madd(t0, -smooth_tops, 1.0f));
        }
        Val sum = mul(add(first, rest), c(1.0f / std::max(total, 1e-12f)));
        t = turbulence ? madd(sum, 2.0f, 0.0f, true) : madd(sum, 0.5f, 0.5f);
        break;
      }
    }
    if (t.out == nullptr) {
      t = add(t, c(0.0f));
    }
    return t.out;
  }

  /* TwoZone::bias then TwoZone::gain. */
  ShaderOutput *bias_gain(ShaderOutput *in, float bias, float gain)
  {
    auto exponent = [](float b) {
      b = std::min(std::max(b, 1e-6f), 1.0f - 1e-6f);
      return logf(b) / logf(0.5f);
    };
    Val t = clamp01(Val{in});
    if (bias != 0.5f) {
      t = op(NODE_MATH_POWER, t, c(exponent(bias)));
    }
    if (gain != 0.5f) {
      const float k = exponent(1.0f - std::min(std::max(gain, 0.0f), 1.0f));
      Val lo = mul(op(NODE_MATH_POWER, mul(t, c(2.0f)), c(k)), c(0.5f));
      Val hi = sub(c(1.0f), mul(op(NODE_MATH_POWER, madd(t, -2.0f, 2.0f), c(k)), c(0.5f)));
      Val low_half = op(NODE_MATH_LESS_THAN, t, c(0.5f));
      t = add(hi, mul(low_half, sub(lo, hi)));
    }
    return t.out;
  }
};

/* Shader fog (OcSurface.fog_*): the blend factor toward the fog colour,
 * 0 for every ray but camera rays. */
ShaderOutput *fog_factor(ShaderGraph *graph, const OcSurface &src)
{
  CameraNode *camera = graph->create_node<CameraNode>();
  ShaderOutput *d = camera->output("View Distance");
  ShaderOutput *f = nullptr;
  if (src.fog_type == OC_FOG_LINEAR) {
    MapRangeNode *range = graph->create_node<MapRangeNode>();
    range->set_range_type(NODE_MAP_RANGE_LINEAR);
    range->set_clamp(true);
    range->set_from_min(src.fog_start);
    range->set_from_max(src.fog_end > src.fog_start ? src.fog_end : src.fog_start + 1e-6f);
    range->set_to_min(0.0f);
    range->set_to_max(1.0f);
    graph->connect(d, range->input("Value"));
    f = range->output("Result");
  }
  else {
    /* density * d, the density falling off with height for layered fog. */
    MathNode *kd = graph->create_node<MathNode>();
    kd->set_math_type(NODE_MATH_MULTIPLY);
    graph->connect(d, kd->input("Value1"));
    if (src.fog_type == OC_FOG_LAYERED) {
      GeometryNode *geometry = graph->create_node<GeometryNode>();
      SeparateXYZNode *xyz = graph->create_node<SeparateXYZNode>();
      graph->connect(geometry->output("Position"), xyz->input("Vector"));
      MathNode *above = graph->create_node<MathNode>();
      above->set_math_type(NODE_MATH_MAXIMUM);
      above->set_value2(0.0f);
      graph->connect(xyz->output("Y"), above->input("Value1"));
      MathNode *neg = graph->create_node<MathNode>();
      neg->set_math_type(NODE_MATH_MULTIPLY);
      neg->set_value2(-1.0f);
      graph->connect(above->output("Value"), neg->input("Value1"));
      MathNode *thin = graph->create_node<MathNode>();
      thin->set_math_type(NODE_MATH_EXPONENT);
      graph->connect(neg->output("Value"), thin->input("Value1"));
      MathNode *density = graph->create_node<MathNode>();
      density->set_math_type(NODE_MATH_MULTIPLY);
      density->set_value2(src.fog_density);
      graph->connect(thin->output("Value"), density->input("Value1"));
      graph->connect(density->output("Value"), kd->input("Value2"));
    }
    else {
      kd->set_value2(src.fog_density);
    }
    /* 1 - exp(-k d) */
    MathNode *neg = graph->create_node<MathNode>();
    neg->set_math_type(NODE_MATH_MULTIPLY);
    neg->set_value2(-1.0f);
    graph->connect(kd->output("Value"), neg->input("Value1"));
    MathNode *e = graph->create_node<MathNode>();
    e->set_math_type(NODE_MATH_EXPONENT);
    graph->connect(neg->output("Value"), e->input("Value1"));
    MathNode *one_minus = graph->create_node<MathNode>();
    one_minus->set_math_type(NODE_MATH_SUBTRACT);
    one_minus->set_value1(1.0f);
    graph->connect(e->output("Value"), one_minus->input("Value2"));
    f = one_minus->output("Value");
  }
  LightPathNode *path = graph->create_node<LightPathNode>();
  MathNode *camera_only = graph->create_node<MathNode>();
  camera_only->set_math_type(NODE_MATH_MULTIPLY);
  graph->connect(f, camera_only->input("Value1"));
  graph->connect(path->output("Is Camera Ray"), camera_only->input("Value2"));
  return camera_only->output("Value");
}

/* Whether the render asks for Render Output pass `which`. */
bool wants_pass(const OcSettings &s, uint32_t which)
{
  for (uint32_t k = 0; s.passes != nullptr && k < s.pass_count; k++) {
    if (s.passes[k] == which && s.pass_out != nullptr && s.pass_out[k] != nullptr) {
      return true;
    }
  }
  return false;
}

/* Material values (OC_VALUE_*): their AOV names, Rust's
 * MaterialValue::aov_name. */
const char *const OC_VALUE_NAMES[OC_VALUE_COUNT] = {
    "operandi_value_diffuse_amount",
    "operandi_value_diffuse_coefficient",
    "operandi_value_diffuse_energy_conservation",
    "operandi_value_diffuse_roughness",
    "operandi_value_specular_coefficient",
    "operandi_value_reflection_coefficient",
    "operandi_value_roughness",
    "operandi_value_subsurface_amount",
    "operandi_value_subsurface_color",
    "operandi_value_transparent_amount",
    "operandi_value_transparent_color",
    "operandi_value_dpdu",
    "operandi_value_dpdv",
};

/* Whether the render asks for material value `which` (OC_VALUE_*). */
bool wants_value(const OcSettings &s, uint32_t which)
{
  for (uint32_t k = 0; s.value_passes != nullptr && k < s.value_count; k++) {
    if (s.value_passes[k] == which && s.value_out != nullptr && s.value_out[k] != nullptr) {
      return true;
    }
  }
  return false;
}

/* AOVs every surface writes for the Shading Incidence and Object
 * Coordinates outputs, and the dPdu / dPdv material values. */
void add_output_aovs(ShaderGraph *graph, const OcSettings &s, const uint32_t groups = 0)
{
  /* dPdu: the world-space unit tangent along the first UV map's U
   * (mikktspace, orthogonal to the normal); dPdv: cross(normal, dPdu).
   * Raw -1..1 directions; a mesh without UVs writes 0 (MV-Q06). */
  const bool dpdu = wants_value(s, OC_VALUE_DPDU), dpdv = wants_value(s, OC_VALUE_DPDV);
  if (dpdu || dpdv) {
    TangentNode *tangent = graph->create_node<TangentNode>();
    tangent->set_direction_type(NODE_TANGENT_UVMAP);
    if (dpdu) {
      OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
      aov->set_name(ustring(OC_VALUE_NAMES[OC_VALUE_DPDU]));
      graph->connect(tangent->output("Tangent"), aov->input("Color"));
    }
    if (dpdv) {
      GeometryNode *geometry = graph->create_node<GeometryNode>();
      VectorMathNode *cross = graph->create_node<VectorMathNode>();
      cross->set_math_type(NODE_VECTOR_MATH_CROSS_PRODUCT);
      graph->connect(geometry->output("Normal"), cross->input("Vector1"));
      graph->connect(tangent->output("Tangent"), cross->input("Vector2"));
      OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
      aov->set_name(ustring(OC_VALUE_NAMES[OC_VALUE_DPDV]));
      graph->connect(cross->output("Vector"), aov->input("Color"));
    }
  }
  for (uint32_t k = 0; k < 8; k++) {
    if (wants_pass(s, OC_PASS_GROUP_0 + k)) {
      OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
      aov->set_name(ustring("operandi_group_" + std::to_string(k)));
      aov->set_value(((groups >> k) & 1u) != 0 ? 1.0f : 0.0f);
    }
  }
  if (wants_pass(s, OC_PASS_INCIDENCE)) {
    GeometryNode *geometry = graph->create_node<GeometryNode>();
    VectorMathNode *dot = graph->create_node<VectorMathNode>();
    dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
    graph->connect(geometry->output("Normal"), dot->input("Vector1"));
    graph->connect(geometry->output("Incoming"), dot->input("Vector2"));
    MathNode *lo = graph->create_node<MathNode>();
    lo->set_math_type(NODE_MATH_MINIMUM);
    lo->set_value2(1.0f);
    graph->connect(dot->output("Value"), lo->input("Value1"));
    MathNode *angle = graph->create_node<MathNode>();
    angle->set_math_type(NODE_MATH_ARCCOSINE);
    graph->connect(lo->output("Value"), angle->input("Value1"));
    MathNode *frac = graph->create_node<MathNode>();
    frac->set_math_type(NODE_MATH_MULTIPLY);
    frac->set_value2(2.0f / M_PI_F);
    graph->connect(angle->output("Value"), frac->input("Value1"));
    OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
    aov->set_name(ustring("operandi_incidence"));
    graph->connect(frac->output("Value"), aov->input("Value"));
  }
  if (wants_pass(s, OC_PASS_OBJECT_COORDINATES)) {
    TextureCoordinateNode *coords = graph->create_node<TextureCoordinateNode>();
    OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
    aov->set_name(ustring("operandi_object"));
    graph->connect(coords->output("Object"), aov->input("Color"));
  }
  if (wants_pass(s, OC_PASS_UV)) {
    /* The mesh's UVs; Cycles gives 0 where a mesh has none. */
    TextureCoordinateNode *coords = graph->create_node<TextureCoordinateNode>();
    OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
    aov->set_name(ustring("operandi_uv"));
    graph->connect(coords->output("UV"), aov->input("Color"));
  }
  if (wants_pass(s, OC_PASS_SEGMENT)) {
    /* The object colour carries OcMesh.segment_color, so surfaces shared
     * between meshes still give each mesh its own segment. */
    ObjectInfoNode *info = graph->create_node<ObjectInfoNode>();
    OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
    aov->set_name(ustring("operandi_segment"));
    graph->connect(info->output("Color"), aov->input("Color"));
  }
}

/* The Cryptomatte types asked for (CRYPT_* bits) and Cycles' depth: the
 * number of passes of two levels each, the deepest asked for. */
std::pair<int, int> cryptomatte_wanted(const OcSettings &s)
{
  int types = CRYPT_NONE, depth = 0;
  static const uint32_t firsts[3] = {
      OC_PASS_CRYPTO_OBJECT_0, OC_PASS_CRYPTO_MATERIAL_0, OC_PASS_CRYPTO_ASSET_0};
  static const int bits[3] = {CRYPT_OBJECT, CRYPT_MATERIAL, CRYPT_ASSET};
  for (int t = 0; t < 3; t++) {
    for (uint32_t k = 0; k < 8; k++) {
      if (wants_pass(s, firsts[t] + k)) {
        types |= bits[t];
        depth = std::max(depth, int(k) + 1);
      }
    }
  }
  return {types, depth};
}

/* A cryptomatte pass's standard name, "CryptoObject00" etc. */
std::string cryptomatte_pass_name(uint32_t which)
{
  const char *kind = which >= OC_PASS_CRYPTO_ASSET_0    ? "CryptoAsset" :
                     which >= OC_PASS_CRYPTO_MATERIAL_0 ? "CryptoMaterial" :
                                                          "CryptoObject";
  const uint32_t k = (which - OC_PASS_CRYPTO_OBJECT_0) % 8;
  char buffer[32];
  std::snprintf(buffer, sizeof(buffer), "%s%02u", kind, k);
  return buffer;
}

/* One socket value of a general graph node (OC_NODE_VALUE_*), converted
 * to the socket's own type. Values of the wrong kind are skipped. */
/* A node type's input socket by its UI name ("Base Color"), which is what
 * graphs and ShaderNode::input use; NodeType::find_input takes the
 * internal name ("base_color"), tried second. */
const SocketType *find_socket(const NodeType *type, const ustring name)
{
  for (const SocketType &socket : type->inputs) {
    if (socket.ui_name == name) {
      return &socket;
    }
  }
  return type->find_input(name);
}

void set_node_value(ShaderNode *node, const OcNodeValue &v)
{
  if (v.socket == nullptr) {
    return;
  }
  const SocketType *st = find_socket(node->type, ustring(v.socket));
  if (st == nullptr) {
    return;
  }
  const float3 v3 = make_float3(v.value[0], v.value[1], v.value[2]);
  switch (v.kind) {
    case OC_NODE_VALUE_FLOAT:
    case OC_NODE_VALUE_INT:
    case OC_NODE_VALUE_BOOL:
    case OC_NODE_VALUE_VEC3:
      switch (st->type) {
        case SocketType::FLOAT:
          node->set(*st, v.value[0]);
          break;
        case SocketType::INT:
          node->set(*st, int(std::lround(v.value[0])));
          break;
        case SocketType::BOOLEAN:
          node->set(*st, v.value[0] != 0.0f);
          break;
        case SocketType::COLOR:
        case SocketType::VECTOR:
        case SocketType::POINT:
        case SocketType::NORMAL:
          node->set(*st, v.kind == OC_NODE_VALUE_VEC3 ? v3 : make_float3(v.value[0]));
          break;
        case SocketType::ENUM: {
          /* An enum given as a number (noise dimensions "3D" = 3). */
          const int value = int(std::lround(v.value[0]));
          if (st->enum_values != nullptr && st->enum_values->exists(value)) {
            node->set(*st, value);
          }
          break;
        }
        default:
          break;
      }
      break;
    case OC_NODE_VALUE_TEXT: {
      const ustring text(v.text ? v.text : "");
      if (st->type == SocketType::STRING) {
        if (ustring(v.socket) == ustring("Colorspace")) {
          /* The bridge's colour space names (node_graph::cycles_colorspace). */
          node->set(*st, text == ustring("data")         ? u_colorspace_data :
                         text == ustring("scene_linear") ? u_colorspace_scene_linear :
                                                           u_colorspace_srgb);
        }
        else {
          node->set(*st, text);
        }
      }
      else if (st->type == SocketType::ENUM && st->enum_values != nullptr &&
               st->enum_values->exists(text))
      {
        node->set(*st, text);
      }
      break;
    }
    case OC_NODE_VALUE_RAMP: {
      if (v.array == nullptr || st->type != SocketType::COLOR_ARRAY) {
        break;
      }
      array<packed_float3> colors;
      array<float> alphas;
      colors.resize(v.array_count);
      alphas.resize(v.array_count);
      for (uint32_t k = 0; k < v.array_count; k++) {
        colors[k] = make_float3(v.array[k * 4], v.array[k * 4 + 1], v.array[k * 4 + 2]);
        alphas[k] = v.array[k * 4 + 3];
      }
      node->set(*st, colors);
      const SocketType *alpha = find_socket(node->type,
                                            ustring(std::string(v.socket) + " Alpha"));
      if (alpha != nullptr && alpha->type == SocketType::FLOAT_ARRAY) {
        node->set(*alpha, alphas);
      }
      break;
    }
    default:
      break;
  }
}

Shader *build_surface(Scene *scene, const OcSurface &src, const OcSettings &s)
{
  if (src.volume != 0) {
    return build_volume(scene, src);
  }
  unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
  const uint32_t n = src.expr_count;
  vector<ShaderOutput *> outs(n, nullptr);
  std::map<uint64_t, RGBRampNode *> ramps;
  std::map<int32_t, AttributeNode *> attributes;
  /* One image node per image, shared by its colour and alpha. */
  vector<ImageTextureNode *> images(src.image_count, nullptr);
  auto feed = [&](ShaderInput *in, int32_t index) {
    if (in == nullptr || index < 0 || uint32_t(index) >= n) {
      return;
    }
    const OcExpr &e = src.exprs[index];
    if (e.op == OC_EXPR_CONST) {
      if (in->type() == SocketType::FLOAT) {
        in->set(e.value[0]);
      }
      else {
        in->set(f3(e.value));
      }
    }
    else if (outs[index] != nullptr) {
      graph->connect(outs[index], in);
    }
  };
  /* The general node list (NM-30): made first, so ops can read their
   * outputs (OC_EXPR_NODE); their links and op-fed values come after the
   * ops. */
  vector<ShaderNode *> built(src.nodes != nullptr ? src.node_count : 0, nullptr);
  uint32_t output = src.node_count;
  for (uint32_t i = 0; i < built.size(); i++) {
    const OcNode &on = src.nodes[i];
    const ustring type(on.type ? on.type : "");
    if (type == ustring("output")) {
      output = i;
      continue;
    }
    const NodeType *t = NodeType::find(type);
    if (t == nullptr || t->type != NodeType::SHADER || t->create == nullptr) {
      continue;
    }
    ShaderNode *node = graph->create_node(t);
    built[i] = node;
    for (uint32_t k = 0; k < on.value_count; k++) {
      const OcNodeValue &v = on.values[k];
      if (v.kind != OC_NODE_VALUE_EXPR) {
        set_node_value(node, v);
      }
    }
  }
  for (uint32_t i = 0; i < n; i++) {
    const OcExpr &e = src.exprs[i];
    switch (e.op) {
      case OC_EXPR_NODE: {
        if (e.a >= 0 && uint32_t(e.a) < built.size() && built[e.a] != nullptr &&
            e.image >= 0 && uint32_t(e.image) < src.expr_socket_count &&
            src.expr_sockets != nullptr && src.expr_sockets[e.image] != nullptr)
        {
          outs[i] = built[e.a]->output(src.expr_sockets[e.image]);
        }
        break;
      }
      case OC_EXPR_IMAGE_COLOR:
      case OC_EXPR_IMAGE_ALPHA: {
        if (e.image < 0 || uint32_t(e.image) >= src.image_count) {
          break;
        }
        ImageTextureNode *&tex = images[e.image];
        if (tex == nullptr) {
          tex = graph->create_node<ImageTextureNode>();
          tex->set_filename(ustring(src.images[e.image] ? src.images[e.image] : ""));
          const uint32_t filter = src.image_filters ? src.image_filters[e.image] :
                                                      OC_FILTER_BILINEAR;
          tex->set_interpolation(filter == OC_FILTER_NEAREST ? INTERPOLATION_CLOSEST :
                                 filter == OC_FILTER_BICUBIC ? INTERPOLATION_CUBIC :
                                                               INTERPOLATION_LINEAR);
          tex->set_colorspace(picture_colorspace(
              src.images[e.image], src.image_raw != nullptr && src.image_raw[e.image] != 0));
          if (src.image_extensions != nullptr && src.image_extensions[e.image] <= OC_TILE_MIRROR) {
            tex->set_extension(ExtensionType(src.image_extensions[e.image]));
          }
          /* Its vector is an earlier node (Texture Locators). */
          if (src.image_vectors != nullptr && src.image_vectors[e.image] >= 0 &&
              uint32_t(src.image_vectors[e.image]) < i)
          {
            feed(tex->input("Vector"), src.image_vectors[e.image]);
          }
        }
        outs[i] = tex->output(e.op == OC_EXPR_IMAGE_COLOR ? "Color" : "Alpha");
        break;
      }
      case OC_EXPR_LUMA: {
        RGBToBWNode *bw = graph->create_node<RGBToBWNode>();
        feed(bw->input("Color"), e.a);
        outs[i] = bw->output("Val");
        break;
      }
      case OC_EXPR_CHANNEL: {
        SeparateColorNode *sep = graph->create_node<SeparateColorNode>();
        feed(sep->input("Color"), e.a);
        static const char *const names[3] = {"Red", "Green", "Blue"};
        outs[i] = sep->output(names[e.mode < 3 ? e.mode : 0]);
        break;
      }
      case OC_EXPR_GAMMA: {
        GammaNode *gamma = graph->create_node<GammaNode>();
        feed(gamma->input("Color"), e.a);
        gamma->set_gamma(e.value[0]);
        outs[i] = gamma->output("Color");
        break;
      }
      case OC_EXPR_LOCATOR: {
        /* Each locator axis is a dot product of the world position with a
         * row of the matrix, plus its translation. */
        GeometryNode *geo = graph->create_node<GeometryNode>();
        CombineXYZNode *xyz = graph->create_node<CombineXYZNode>();
        static const char *const axes[3] = {"X", "Y", "Z"};
        for (int r = 0; r < 3; r++) {
          VectorMathNode *dot = graph->create_node<VectorMathNode>();
          dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
          graph->connect(geo->output("Position"), dot->input("Vector1"));
          dot->set_vector2(make_float3(e.xform[r * 4], e.xform[r * 4 + 1], e.xform[r * 4 + 2]));
          MathNode *add = graph->create_node<MathNode>();
          add->set_math_type(NODE_MATH_ADD);
          graph->connect(dot->output("Value"), add->input("Value1"));
          add->set_value2(e.xform[r * 4 + 3]);
          graph->connect(add->output("Value"), xyz->input(axes[r]));
        }
        outs[i] = xyz->output("Vector");
        break;
      }
      case OC_EXPR_PLANAR: {
        SeparateXYZNode *sep = graph->create_node<SeparateXYZNode>();
        feed(sep->input("Vector"), e.a);
        /* (sign * c + centre) * repeat = c * (sign * repeat) + centre * repeat */
        auto coord = [&](const char *axis, float sign, float repeat) {
          MathNode *m = graph->create_node<MathNode>();
          m->set_math_type(NODE_MATH_MULTIPLY_ADD);
          graph->connect(sep->output(axis), m->input("Value1"));
          m->set_value2(sign * repeat);
          m->set_value3(e.value[0] * repeat);
          return m->output("Value");
        };
        const uint32_t axis = e.mode & 3;
        const float su = (e.mode & 4) ? -1.0f : 1.0f; /* mirror u */
        const float sv = (e.mode & 8) ? -1.0f : 1.0f; /* mirror v */
        ShaderOutput *u = axis == 0 ? coord("Z", -su, e.value[1]) : coord("X", su, e.value[1]);
        ShaderOutput *v = axis == 1 ? coord("Z", -sv, e.value[2]) : coord("Y", sv, e.value[2]);
        CombineXYZNode *xyz = graph->create_node<CombineXYZNode>();
        graph->connect(u, xyz->input("X"));
        graph->connect(v, xyz->input("Y"));
        outs[i] = xyz->output("Vector");
        break;
      }
      case OC_EXPR_UV: {
        TextureCoordinateNode *coords = graph->create_node<TextureCoordinateNode>();
        VectorMathNode *scale = graph->create_node<VectorMathNode>();
        scale->set_math_type(NODE_VECTOR_MATH_MULTIPLY);
        graph->connect(coords->output("UV"), scale->input("Vector1"));
        scale->set_vector2(make_float3(e.value[0], e.value[1], 1.0f));
        outs[i] = scale->output("Vector");
        if (e.value[2] != 0.0f) {
          VectorRotateNode *turn = graph->create_node<VectorRotateNode>();
          turn->set_rotate_type(NODE_VECTOR_ROTATE_TYPE_AXIS_Z);
          graph->connect(outs[i], turn->input("Vector"));
          turn->set_center(zero_float3());
          turn->set_angle(e.value[2]);
          outs[i] = turn->output("Vector");
        }
        break;
      }
      case OC_EXPR_TILE: {
        SeparateXYZNode *sep = graph->create_node<SeparateXYZNode>();
        feed(sep->input("Vector"), e.a);
        auto tile = [&](const char *axis, uint32_t mode) -> ShaderOutput * {
          MathNode *m = graph->create_node<MathNode>();
          graph->connect(sep->output(axis), m->input("Value1"));
          switch (mode) {
            case OC_TILE_REPEAT:
              m->set_math_type(NODE_MATH_FRACTION);
              break;
            case OC_TILE_MIRROR:
              m->set_math_type(NODE_MATH_PINGPONG);
              m->set_value2(1.0f);
              break;
            case OC_TILE_EDGE:
              m->set_math_type(NODE_MATH_ADD);
              m->set_value2(0.0f);
              m->set_use_clamp(true);
              break;
            default: /* Reset: left as it is; the image clips. */
              return sep->output(axis);
          }
          return m->output("Value");
        };
        CombineXYZNode *xyz = graph->create_node<CombineXYZNode>();
        graph->connect(tile("X", e.mode & 3), xyz->input("X"));
        graph->connect(tile("Y", (e.mode >> 2) & 3), xyz->input("Y"));
        graph->connect(sep->output("Z"), xyz->input("Z"));
        outs[i] = xyz->output("Vector");
        break;
      }
      case OC_EXPR_INSIDE: {
        SeparateXYZNode *sep = graph->create_node<SeparateXYZNode>();
        feed(sep->input("Vector"), e.a);
        ShaderOutput *inside = nullptr;
        static const char *const axes[2] = {"X", "Y"};
        for (int k = 0; k < 2; k++) {
          if ((e.mode & (1u << k)) == 0) {
            continue;
          }
          /* 1 - (c < 0) - (c > 1) */
          MathNode *below = graph->create_node<MathNode>();
          below->set_math_type(NODE_MATH_LESS_THAN);
          graph->connect(sep->output(axes[k]), below->input("Value1"));
          below->set_value2(0.0f);
          MathNode *above = graph->create_node<MathNode>();
          above->set_math_type(NODE_MATH_GREATER_THAN);
          graph->connect(sep->output(axes[k]), above->input("Value1"));
          above->set_value2(1.0f);
          MathNode *in = graph->create_node<MathNode>();
          in->set_math_type(NODE_MATH_SUBTRACT);
          in->set_value1(1.0f);
          graph->connect(below->output("Value"), in->input("Value2"));
          MathNode *in2 = graph->create_node<MathNode>();
          in2->set_math_type(NODE_MATH_SUBTRACT);
          graph->connect(in->output("Value"), in2->input("Value1"));
          graph->connect(above->output("Value"), in2->input("Value2"));
          if (inside == nullptr) {
            inside = in2->output("Value");
          }
          else {
            MathNode *both = graph->create_node<MathNode>();
            both->set_math_type(NODE_MATH_MULTIPLY);
            graph->connect(inside, both->input("Value1"));
            graph->connect(in2->output("Value"), both->input("Value2"));
            inside = both->output("Value");
          }
        }
        if (inside == nullptr) {
          MathNode *one = graph->create_node<MathNode>();
          one->set_math_type(NODE_MATH_ADD);
          one->set_value1(1.0f);
          one->set_value2(0.0f);
          inside = one->output("Value");
        }
        outs[i] = inside;
        break;
      }
      case OC_EXPR_NORMAL: {
        PatternBuilder b{graph.get()};
        outs[i] = b.normal(e.xform);
        break;
      }
      case OC_EXPR_PROBE: {
        if (e.a >= 0 && uint32_t(e.a) < i && outs[e.a] != nullptr) {
          PatternBuilder b{graph.get()};
          outs[i] = b.probe(outs[e.a], e.mode, e.value);
        }
        break;
      }
      case OC_EXPR_WRAP:
      case OC_EXPR_FACE_WEIGHT:
      case OC_EXPR_FACING:
      case OC_EXPR_FALLOFF: {
        if (e.a >= 0 && uint32_t(e.a) < i && outs[e.a] != nullptr) {
          PatternBuilder b{graph.get()};
          outs[i] = e.op == OC_EXPR_WRAP        ? b.wrap(outs[e.a], e.mode, e.value) :
                    e.op == OC_EXPR_FACE_WEIGHT ? b.face_weight(outs[e.a], e.mode, e.value[0]) :
                    e.op == OC_EXPR_FACING      ? b.facing(outs[e.a], e.mode) :
                                                  b.falloff(outs[e.a], e.mode, e.value);
        }
        break;
      }
      case OC_EXPR_PATTERN: {
        if (e.a >= 0 && uint32_t(e.a) < i && outs[e.a] != nullptr) {
          PatternBuilder b{graph.get()};
          outs[i] = b.pattern(outs[e.a], e.mode, e.xform);
        }
        break;
      }
      case OC_EXPR_CELL_RANDOM: {
        if (e.a >= 0 && uint32_t(e.a) < i && outs[e.a] != nullptr) {
          PatternBuilder b{graph.get()};
          outs[i] = b.cell_random(outs[e.a], e.mode, e.xform);
        }
        break;
      }
      case OC_EXPR_HSV_VARY: {
        if (e.a >= 0 && uint32_t(e.a) < i && e.b >= 0 && uint32_t(e.b) < i && outs[e.b] != nullptr) {
          PatternBuilder b{graph.get()};
          ShaderOutput *color = outs[e.a];
          if (color == nullptr) {
            /* a constant colour: feed it through a node so HSV can read it */
            CombineXYZNode *k = graph->create_node<CombineXYZNode>();
            k->set_x(src.exprs[e.a].value[0]);
            k->set_y(src.exprs[e.a].value[1]);
            k->set_z(src.exprs[e.a].value[2]);
            color = k->output("Vector");
          }
          outs[i] = b.hsv_vary(color, outs[e.b], e.value);
        }
        break;
      }
      case OC_EXPR_BIAS_GAIN: {
        if (e.a >= 0 && uint32_t(e.a) < i && outs[e.a] != nullptr) {
          PatternBuilder b{graph.get()};
          outs[i] = b.bias_gain(outs[e.a], e.value[0], e.value[1]);
        }
        break;
      }
      case OC_EXPR_MIX: {
        MixColorNode *mix = graph->create_node<MixColorNode>();
        mix->set_blend_type(NodeMix(e.mode));
        mix->set_use_clamp(true);
        mix->set_use_clamp_result(false);
        feed(mix->input("Factor"), e.fac);
        feed(mix->input("A"), e.a);
        feed(mix->input("B"), e.b);
        outs[i] = mix->output("Result");
        break;
      }
      case OC_EXPR_HSV: {
        SeparateColorNode *sep = graph->create_node<SeparateColorNode>();
        sep->set_color_type(NODE_COMBSEP_COLOR_HSV);
        feed(sep->input("Color"), e.a);
        MathNode *hue = graph->create_node<MathNode>();
        hue->set_math_type(NODE_MATH_MULTIPLY_ADD);
        graph->connect(sep->output("Red"), hue->input("Value1"));
        hue->set_value2(e.value[0]);
        hue->set_value3(e.value[1] / 360.0f);
        MathNode *wrap = graph->create_node<MathNode>();
        wrap->set_math_type(NODE_MATH_FRACTION);
        graph->connect(hue->output("Value"), wrap->input("Value1"));
        MathNode *sat = graph->create_node<MathNode>();
        sat->set_math_type(NODE_MATH_MULTIPLY);
        sat->set_use_clamp(true);
        graph->connect(sep->output("Green"), sat->input("Value1"));
        sat->set_value2(e.value[2]);
        CombineColorNode *comb = graph->create_node<CombineColorNode>();
        comb->set_color_type(NODE_COMBSEP_COLOR_HSV);
        graph->connect(wrap->output("Value"), comb->input("Red"));
        graph->connect(sat->output("Value"), comb->input("Green"));
        graph->connect(sep->output("Blue"), comb->input("Blue"));
        outs[i] = comb->output("Color");
        break;
      }
      case OC_EXPR_OCCLUSION: {
        AmbientOcclusionNode *ao = graph->create_node<AmbientOcclusionNode>();
        ao->set_samples(e.image > 0 ? e.image : 16);
        ao->set_distance(e.value[0]);
        ao->set_only_local((e.mode & 1u) != 0);
        ao->set_inside((e.mode & 2u) != 0);
        const uint32_t toward = (e.mode >> 2) & 3u;
        if (toward != 0) {
          /* World-space hemisphere direction (shader_eval::AoToward). */
          ShaderOutput *dir = nullptr;
          if (toward == 1) {
            /* Meshes keep Operandi's +Y up (as the Slope input reads). */
            CombineXYZNode *up = graph->create_node<CombineXYZNode>();
            up->set_x(0.0f);
            up->set_y(1.0f);
            up->set_z(0.0f);
            dir = up->output("Vector");
          }
          else {
            GeometryNode *geo = graph->create_node<GeometryNode>();
            VectorMathNode *sum = graph->create_node<VectorMathNode>();
            if (toward == 2) {
              /* Halfway between the normal and straight down. */
              sum->set_math_type(NODE_VECTOR_MATH_ADD);
              graph->connect(geo->output("Normal"), sum->input("Vector1"));
              sum->set_vector2(make_float3(0.0f, -1.0f, 0.0f));
            }
            else {
              /* The mirror direction: reflect(-Incoming, Normal). */
              VectorMathNode *back = graph->create_node<VectorMathNode>();
              back->set_math_type(NODE_VECTOR_MATH_SCALE);
              back->set_scale(-1.0f);
              graph->connect(geo->output("Incoming"), back->input("Vector1"));
              sum->set_math_type(NODE_VECTOR_MATH_REFLECT);
              graph->connect(back->output("Vector"), sum->input("Vector1"));
              graph->connect(geo->output("Normal"), sum->input("Vector2"));
            }
            VectorMathNode *unit = graph->create_node<VectorMathNode>();
            unit->set_math_type(NODE_VECTOR_MATH_NORMALIZE);
            graph->connect(sum->output("Vector"), unit->input("Vector1"));
            dir = unit->output("Vector");
          }
          graph->connect(dir, ao->input("Normal"));
        }
        outs[i] = ao->output("AO");
        break;
      }
      case OC_EXPR_ATTRIBUTE_COLOR:
      case OC_EXPR_ATTRIBUTE_ALPHA: {
        if (src.attributes == nullptr || e.image < 0 || uint32_t(e.image) >= src.attribute_count)
        {
          break;
        }
        AttributeNode *&node = attributes[e.image];
        if (node == nullptr) {
          node = graph->create_node<AttributeNode>();
          node->set_attribute(ustring(src.attributes[e.image] ? src.attributes[e.image] : ""));
        }
        outs[i] = node->output(e.op == OC_EXPR_ATTRIBUTE_COLOR ? "Color" : "Alpha");
        break;
      }
      case OC_EXPR_GEOMETRY: {
        /* Degrees from a cosine: acos(clamp(c, -1, 1)) * 180 / pi. */
        auto degrees = [&](ShaderOutput *cosine) {
          MathNode *lo = graph->create_node<MathNode>();
          lo->set_math_type(NODE_MATH_MAXIMUM);
          lo->set_value2(-1.0f);
          graph->connect(cosine, lo->input("Value1"));
          MathNode *hi = graph->create_node<MathNode>();
          hi->set_math_type(NODE_MATH_MINIMUM);
          hi->set_value2(1.0f);
          graph->connect(lo->output("Value"), hi->input("Value1"));
          MathNode *angle = graph->create_node<MathNode>();
          angle->set_math_type(NODE_MATH_ARCCOSINE);
          graph->connect(hi->output("Value"), angle->input("Value1"));
          MathNode *deg = graph->create_node<MathNode>();
          deg->set_math_type(NODE_MATH_MULTIPLY);
          deg->set_value2(180.0f / M_PI_F);
          graph->connect(angle->output("Value"), deg->input("Value1"));
          return deg->output("Value");
        };
        GeometryNode *geometry = graph->create_node<GeometryNode>();
        if (e.mode == OC_GEOMETRY_INCIDENCE) {
          VectorMathNode *dot = graph->create_node<VectorMathNode>();
          dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
          graph->connect(geometry->output("Normal"), dot->input("Vector1"));
          graph->connect(geometry->output("Incoming"), dot->input("Vector2"));
          outs[i] = degrees(dot->output("Value"));
        }
        else if (e.mode == OC_GEOMETRY_SLOPE) {
          SeparateXYZNode *xyz = graph->create_node<SeparateXYZNode>();
          graph->connect(geometry->output("Normal"), xyz->input("Vector"));
          outs[i] = degrees(xyz->output("Y"));
        }
        else if (e.mode == OC_GEOMETRY_CAMERA_DISTANCE) {
          CameraNode *camera = graph->create_node<CameraNode>();
          outs[i] = camera->output("View Distance");
        }
        else if (e.mode == OC_GEOMETRY_SCREEN_X || e.mode == OC_GEOMETRY_SCREEN_Y) {
          /* Window coordinates run 0..1 across the whole frame (a region
           * render included), left to right and bottom to top. */
          TextureCoordinateNode *coords = graph->create_node<TextureCoordinateNode>();
          SeparateXYZNode *xyz = graph->create_node<SeparateXYZNode>();
          graph->connect(coords->output("Window"), xyz->input("Vector"));
          outs[i] = xyz->output(e.mode == OC_GEOMETRY_SCREEN_X ? "X" : "Y");
        }
        else if (e.mode == OC_GEOMETRY_SCREEN_R) {
          /* Pixels from the centre over half the diagonal: with aspect
           * a = width / height, |((2x - 1) a, 2y - 1)| / sqrt(a^2 + 1). */
          const float a = s.height > 0 ? float(s.width) / float(s.height) : 1.0f;
          const float k = 1.0f / sqrtf(a * a + 1.0f);
          TextureCoordinateNode *coords = graph->create_node<TextureCoordinateNode>();
          VectorMathNode *centred = graph->create_node<VectorMathNode>();
          centred->set_math_type(NODE_VECTOR_MATH_SUBTRACT);
          graph->connect(coords->output("Window"), centred->input("Vector1"));
          centred->set_vector2(make_float3(0.5f, 0.5f, 0.0f));
          VectorMathNode *scaled = graph->create_node<VectorMathNode>();
          scaled->set_math_type(NODE_VECTOR_MATH_MULTIPLY);
          graph->connect(centred->output("Vector"), scaled->input("Vector1"));
          scaled->set_vector2(make_float3(2.0f * a * k, 2.0f * k, 0.0f));
          VectorMathNode *length = graph->create_node<VectorMathNode>();
          length->set_math_type(NODE_VECTOR_MATH_LENGTH);
          graph->connect(scaled->output("Vector"), length->input("Vector1"));
          outs[i] = length->output("Value");
        }
        else {
          outs[i] = geometry->output("Backfacing");
        }
        break;
      }
      case OC_EXPR_LOCATOR_INCIDENCE: {
        GeometryNode *geometry = graph->create_node<GeometryNode>();
        VectorMathNode *to = graph->create_node<VectorMathNode>();
        to->set_math_type(NODE_VECTOR_MATH_SUBTRACT);
        to->set_vector1(f3(e.value));
        graph->connect(geometry->output("Position"), to->input("Vector2"));
        VectorMathNode *unit = graph->create_node<VectorMathNode>();
        unit->set_math_type(NODE_VECTOR_MATH_NORMALIZE);
        graph->connect(to->output("Vector"), unit->input("Vector1"));
        VectorMathNode *dot = graph->create_node<VectorMathNode>();
        dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
        graph->connect(geometry->output("Normal"), dot->input("Vector1"));
        graph->connect(unit->output("Vector"), dot->input("Vector2"));
        MathNode *angle = graph->create_node<MathNode>();
        angle->set_math_type(NODE_MATH_ARCCOSINE);
        graph->connect(dot->output("Value"), angle->input("Value1"));
        MathNode *deg = graph->create_node<MathNode>();
        deg->set_math_type(NODE_MATH_MULTIPLY);
        deg->set_value2(180.0f / M_PI_F);
        graph->connect(angle->output("Value"), deg->input("Value1"));
        outs[i] = deg->output("Value");
        break;
      }
      case OC_EXPR_DISTANCE: {
        GeometryNode *geometry = graph->create_node<GeometryNode>();
        VectorMathNode *d = graph->create_node<VectorMathNode>();
        d->set_math_type(NODE_VECTOR_MATH_DISTANCE);
        graph->connect(geometry->output("Position"), d->input("Vector1"));
        d->set_vector2(f3(e.value));
        outs[i] = d->output("Value");
        break;
      }
      case OC_EXPR_RAMP_COLOR:
      case OC_EXPR_RAMP_ALPHA: {
        if (src.ramps == nullptr || e.mode < 2 || e.image < 0 ||
            uint32_t(e.image) + e.mode > src.ramp_count)
        {
          break;
        }
        /* One ramp node per table and input, shared by colour and alpha. */
        const uint64_t key = (uint64_t(uint32_t(e.image)) << 32) | uint32_t(e.a);
        RGBRampNode *&ramp = ramps[key];
        if (ramp == nullptr) {
          MapRangeNode *range = graph->create_node<MapRangeNode>();
          range->set_range_type(NODE_MAP_RANGE_LINEAR);
          range->set_clamp(true);
          range->set_from_min(e.value[0]);
          range->set_from_max(e.value[1]);
          range->set_to_min(0.0f);
          range->set_to_max(1.0f);
          feed(range->input("Value"), e.a);
          ramp = graph->create_node<RGBRampNode>();
          array<packed_float3> colors;
          array<float> alphas;
          for (uint32_t k = 0; k < e.mode; k++) {
            const float *stop = src.ramps + (size_t(e.image) + k) * 4;
            colors.push_back_slow(make_float3(stop[0], stop[1], stop[2]));
            alphas.push_back_slow(stop[3]);
          }
          ramp->set_ramp(colors);
          ramp->set_ramp_alpha(alphas);
          ramp->set_interpolate(true);
          graph->connect(range->output("Result"), ramp->input("Fac"));
        }
        outs[i] = ramp->output(e.op == OC_EXPR_RAMP_COLOR ? "Color" : "Alpha");
        break;
      }
      default:
        break; /* constants are set where they are used */
    }
  }
  PrincipledBsdfNode *bsdf = graph->create_node<PrincipledBsdfNode>();
  const bool see_through = src.transparency_mode == OC_TRANSPARENCY_TRANSPARENT ||
                           src.transparency_mode == OC_TRANSPARENCY_GLASS;
  for (int k = 0; k < OC_INPUT_COUNT; k++) {
    /* With a see-through part, alpha wraps the whole mix instead. */
    if (see_through && k == OC_INPUT_ALPHA) {
      continue;
    }
    feed(bsdf->input(INPUT_NAMES[k]), src.inputs[k]);
  }
  if (src.thin_wall != 0) {
    bsdf->set_thin_wall(1);
  }
  const bool fogged = src.fog_type >= OC_FOG_LINEAR && src.fog_type <= OC_FOG_UNDERWATER;
  ShaderOutput *fog = fogged ? fog_factor(graph.get(), src) : nullptr;
  if (fog != nullptr && src.fog_type == OC_FOG_UNDERWATER) {
    /* Underwater (SH-5): the base colour also loses saturation by f. */
    ShaderInput *base = bsdf->input("Base Color");
    MixColorNode *grey = graph->create_node<MixColorNode>();
    grey->set_blend_type(NODE_MIX_BLEND);
    grey->set_use_clamp(true);
    RGBToBWNode *bw = graph->create_node<RGBToBWNode>();
    const int32_t b = src.inputs[OC_INPUT_BASE_COLOR];
    if (b >= 0 && uint32_t(b) < n && src.exprs[b].op != OC_EXPR_CONST && outs[b] != nullptr) {
      graph->connect(outs[b], grey->input("A"));
      graph->connect(outs[b], bw->input("Color"));
    }
    else {
      const float3 c = (b >= 0 && uint32_t(b) < n) ? f3(src.exprs[b].value) :
                                                     make_float3(0.8f, 0.8f, 0.8f);
      grey->set_a(c);
      bw->set_color(c);
    }
    graph->connect(bw->output("Val"), grey->input("B"));
    graph->connect(fog, grey->input("Factor"));
    graph->disconnect(base);
    graph->connect(grey->output("Result"), base);
  }
  ShaderOutput *surface_normal = nullptr;
  /* Relief (G-07): normal map -> Bump's Normal, Bump -> the shading
   * normal. `strength` < 1 builds a weaker copy for the clear coat. */
  const bool has_normal = src.normal_map >= 0 && uint32_t(src.normal_map) < n;
  const bool has_bump = src.bump >= 0 && uint32_t(src.bump) < n;
  /* Rounded Edge (material extras): a Bevel normal under the relief. */
  ShaderOutput *rounded = nullptr;
  if (src.rounded_edge > 0.0f) {
    BevelNode *bevel = graph->create_node<BevelNode>();
    bevel->set_radius(src.rounded_edge);
    bevel->set_samples(8);
    rounded = bevel->output("Normal");
  }
  auto relief = [&](float strength) -> ShaderOutput * {
    ShaderOutput *normal = rounded;
    if (has_normal) {
      NormalMapNode *map = graph->create_node<NormalMapNode>();
      map->set_space(NODE_NORMAL_MAP_TANGENT);
      map->set_strength(strength);
      feed(map->input("Color"), src.normal_map);
      normal = map->output("Normal");
    }
    if (has_bump) {
      BumpNode *bump = graph->create_node<BumpNode>();
      bump->set_distance(1.0f);
      bump->set_strength(strength);
      feed(bump->input("Height"), src.bump);
      if (normal != nullptr) {
        graph->connect(normal, bump->input("Normal"));
      }
      normal = bump->output("Normal");
    }
    return normal;
  };
  /* A normal map replaces the rounded normal; a bump still adds to it. */
  if (ShaderOutput *normal = relief(1.0f)) {
    surface_normal = normal;
    graph->connect(normal, bsdf->input("Normal"));
    if (src.coat_bump >= 0.999f) {
      graph->connect(normal, bsdf->input("Coat Normal"));
    }
    else if (src.coat_bump > 0.0f) {
      graph->connect(relief(src.coat_bump), bsdf->input("Coat Normal"));
    }
  }
  bool displaced = src.displacement >= 0 && uint32_t(src.displacement) < n;
  /* A Node Material's graph (NM-30): built before the layered surface's
   * displacement so its own displacement, when it has one, wins. */
  ShaderOutput *node_surface = nullptr;
  ShaderOutput *node_displacement = nullptr;
  if (!built.empty()) {
    for (uint32_t i = 0; i < built.size(); i++) {
      if (built[i] == nullptr) {
        continue;
      }
      const OcNode &on = src.nodes[i];
      for (uint32_t k = 0; k < on.value_count; k++) {
        const OcNodeValue &v = on.values[k];
        if (v.kind == OC_NODE_VALUE_EXPR) {
          feed(built[i]->input(v.socket ? v.socket : ""), v.expr);
        }
      }
    }
    for (uint32_t k = 0; k < src.node_link_count; k++) {
      const OcNodeLink &l = src.node_links[k];
      if (l.from >= src.node_count || built[l.from] == nullptr || l.from_socket == nullptr ||
          l.to_socket == nullptr)
      {
        continue;
      }
      ShaderOutput *from = built[l.from]->output(l.from_socket);
      if (from == nullptr) {
        continue;
      }
      if (l.to == output) {
        if (ustring(l.to_socket) == ustring("Surface")) {
          node_surface = from;
        }
        else if (ustring(l.to_socket) == ustring("Displacement")) {
          node_displacement = from;
        }
        continue;
      }
      if (l.to >= src.node_count || built[l.to] == nullptr) {
        continue;
      }
      ShaderInput *to = built[l.to]->input(l.to_socket);
      if (to != nullptr && to->link == nullptr) {
        graph->connect(from, to);
      }
    }
  }
  if (node_displacement != nullptr) {
    graph->connect(node_displacement, graph->output()->input("Displacement"));
    displaced = true;
  }
  else if (displaced) {
    DisplacementNode *disp = graph->create_node<DisplacementNode>();
    disp->set_space(NODE_NORMAL_MAP_OBJECT);
    disp->set_midlevel(0.0f);
    disp->set_scale(1.0f);
    feed(disp->input("Height"), src.displacement);
    graph->connect(disp->output("Displacement"), graph->output()->input("Displacement"));
  }
  ShaderOutput *surface = bsdf->output("BSDF");
  if (see_through) {
    /* Transparency (translation map v0.1, fix 3): a Mix Shader between
     * the principled surface and a Transparent or Glass BSDF coloured by
     * Transparent Color, so Diffuse Color never tints what shows through. */
    ShaderOutput *clear = nullptr;
    if (src.transparency_mode == OC_TRANSPARENCY_GLASS && src.glass_principled != 0) {
      /* OpenPBR dispersion or thin wall: a principled transmission. */
      PrincipledBsdfNode *glass = graph->create_node<PrincipledBsdfNode>();
      glass->set_transmission_weight(1.0f);
      feed(glass->input("Base Color"), src.transparency_color);
      feed(glass->input("IOR"), src.refraction_ior);
      feed(glass->input("Roughness"), src.refraction_roughness);
      for (const int k : {OC_INPUT_THIN_FILM_THICKNESS,
                          OC_INPUT_THIN_FILM_IOR,
                          OC_INPUT_DISPERSION_SCALE,
                          OC_INPUT_DISPERSION_ABBE})
      {
        feed(glass->input(INPUT_NAMES[k]), src.inputs[k]);
      }
      if (src.thin_wall != 0) {
        glass->set_thin_wall(1);
      }
      if (surface_normal != nullptr) {
        graph->connect(surface_normal, glass->input("Normal"));
      }
      clear = glass->output("BSDF");
    }
    else if (src.transparency_mode == OC_TRANSPARENCY_GLASS) {
      GlassBsdfNode *glass = graph->create_node<GlassBsdfNode>();
      feed(glass->input("Color"), src.transparency_color);
      feed(glass->input("IOR"), src.refraction_ior);
      feed(glass->input("Roughness"), src.refraction_roughness);
      /* OpenPBR thin film on the see-through part too. */
      feed(glass->input("Thin Film Thickness"), src.inputs[OC_INPUT_THIN_FILM_THICKNESS]);
      feed(glass->input("Thin Film IOR"), src.inputs[OC_INPUT_THIN_FILM_IOR]);
      if (surface_normal != nullptr) {
        graph->connect(surface_normal, glass->input("Normal"));
      }
      clear = glass->output("BSDF");
    }
    if (clear != nullptr && s.direct_caustics == 0) {
      /* Render fix F7 (Jason, 2026-11-13): with Direct Caustics off, glass
       * casts an even, see-through shadow with no focusing. Shadow rays
       * pass through it as through a Transparent BSDF of the Transparent
       * Color (Cycles' usual Light Path "Is Shadow Ray" setup); the inside's
       * absorption still applies along the way. Rays that have already
       * bounced off a diffuse surface pass straight through too, or the
       * refraction caustics a new scene traces (F5) would still focus the
       * light under the glass on top of the even shadow. Camera, reflection
       * and refraction rays still see the glass. */
      TransparentBsdfNode *shadow = graph->create_node<TransparentBsdfNode>();
      feed(shadow->input("Color"), src.transparency_color);
      LightPathNode *path = graph->create_node<LightPathNode>();
      MathNode *bounced = graph->create_node<MathNode>();
      bounced->set_math_type(NODE_MATH_GREATER_THAN);
      bounced->set_value2(0.5f);
      graph->connect(path->output("Diffuse Depth"), bounced->input("Value1"));
      MathNode *either = graph->create_node<MathNode>();
      either->set_math_type(NODE_MATH_MAXIMUM);
      graph->connect(path->output("Is Shadow Ray"), either->input("Value1"));
      graph->connect(bounced->output("Value"), either->input("Value2"));
      MixClosureNode *pick = graph->create_node<MixClosureNode>();
      graph->connect(either->output("Value"), pick->input("Fac"));
      graph->connect(clear, pick->input("Closure1"));
      graph->connect(shadow->output("BSDF"), pick->input("Closure2"));
      clear = pick->output("Closure");
    }
    if (clear == nullptr) {
      TransparentBsdfNode *transparent = graph->create_node<TransparentBsdfNode>();
      feed(transparent->input("Color"), src.transparency_color);
      clear = transparent->output("BSDF");
    }
    /* The see-through part goes first (Closure1, weighted by Transparent
     * Amount through 1 - Fac), the principled surface second: the same mix
     * as surface-then-clear by Transparent Amount. The order matters for
     * Direct Caustics. Cycles' MNEE evaluates a caster's shader with only
     * CAUSTICS_MAX_CLOSURE (4) closures (surface_shader_eval with
     * use_caustics_storage) and then looks for a refractive or glass one;
     * closures are kept in the order the graph emits them, so a principled
     * surface below 100 % Transparent Amount filled the slots first, the
     * glass was dropped and the caster gave a solid shadow. */
    MathNode *inverse = graph->create_node<MathNode>();
    inverse->set_math_type(NODE_MATH_SUBTRACT);
    inverse->set_use_clamp(true);
    inverse->set_value1(1.0f);
    feed(inverse->input("Value2"), src.transparency_amount);
    MixClosureNode *mix = graph->create_node<MixClosureNode>();
    graph->connect(inverse->output("Value"), mix->input("Fac"));
    graph->connect(clear, mix->input("Closure1"));
    graph->connect(surface, mix->input("Closure2"));
    surface = mix->output("Closure");
    /* Dissolve / alpha over the whole surface: a white Transparent BSDF
     * at alpha 0, the surface at 1. Skipped when the alpha is a plain 1. */
    const int32_t alpha = src.inputs[OC_INPUT_ALPHA];
    const bool opaque = alpha < 0 || uint32_t(alpha) >= n ||
                        (src.exprs[alpha].op == OC_EXPR_CONST &&
                         src.exprs[alpha].value[0] >= 1.0f);
    if (!opaque) {
      TransparentBsdfNode *hole = graph->create_node<TransparentBsdfNode>();
      hole->set_color(make_float3(1.0f, 1.0f, 1.0f));
      MixClosureNode *outer = graph->create_node<MixClosureNode>();
      feed(outer->input("Fac"), alpha);
      graph->connect(hole->output("BSDF"), outer->input("Closure1"));
      graph->connect(surface, outer->input("Closure2"));
      surface = outer->output("Closure");
    }
  }
  if (node_surface != nullptr) {
    /* NM-31: the graph's surface over the layered one by the layer's
     * coverage; at full coverage it replaces it. */
    if (src.node_mix < 0 || uint32_t(src.node_mix) >= n) {
      surface = node_surface;
    }
    else {
      MixClosureNode *mix = graph->create_node<MixClosureNode>();
      feed(mix->input("Fac"), src.node_mix);
      graph->connect(surface, mix->input("Closure1"));
      graph->connect(node_surface, mix->input("Closure2"));
      surface = mix->output("Closure");
    }
  }
  if (fog != nullptr) {
    /* Fog over the finished surface: an emission of the fog colour. */
    EmissionNode *glow = graph->create_node<EmissionNode>();
    glow->set_color(src.fog_use_environment != 0 ? f3(s.environment_fog_color) :
                                                   f3(src.fog_color));
    glow->set_strength(1.0f);
    MixClosureNode *mix = graph->create_node<MixClosureNode>();
    graph->connect(fog, mix->input("Fac"));
    graph->connect(surface, mix->input("Closure1"));
    graph->connect(glow->output("Emission"), mix->input("Closure2"));
    surface = mix->output("Closure");
  }
  if (src.holdout > 0.0f) {
    /* Alpha Type Constant: the picture's alpha is Alpha Value here. */
    HoldoutNode *holdout = graph->create_node<HoldoutNode>();
    MixClosureNode *mix = graph->create_node<MixClosureNode>();
    mix->set_fac(src.holdout < 1.0f ? src.holdout : 1.0f);
    graph->connect(surface, mix->input("Closure1"));
    graph->connect(holdout->output("Holdout"), mix->input("Closure2"));
    surface = mix->output("Closure");
  }
  graph->connect(surface, graph->output()->input("Surface"));
  add_output_aovs(graph.get(), s, src.output_groups);
  /* Material values (material-value Render Outputs): each wanted one's
   * expression into its colour AOV; grey values fill all three. */
  for (uint32_t v = 0; v < OC_VALUE_COUNT; v++) {
    if (v == OC_VALUE_DPDU || v == OC_VALUE_DPDV || !wants_value(s, v)) {
      continue;
    }
    OutputAOVNode *aov = graph->create_node<OutputAOVNode>();
    aov->set_name(ustring(OC_VALUE_NAMES[v]));
    feed(aov->input("Color"), src.values[v]);
  }
  ShaderOutput *volume = nullptr;
  if (src.absorption_density > 0.0f) {
    AbsorptionVolumeNode *absorb = graph->create_node<AbsorptionVolumeNode>();
    absorb->set_color(f3(src.absorption_color));
    absorb->set_density(src.absorption_density);
    volume = absorb->output("Volume");
  }
  if (src.scatter_density > 0.0f) {
    /* OpenPBR transmission_scatter (OPM-03). */
    ScatterVolumeNode *scatter = graph->create_node<ScatterVolumeNode>();
    scatter->set_color(f3(src.scatter_color));
    scatter->set_density(src.scatter_density);
    scatter->set_anisotropy(src.scatter_anisotropy);
    if (volume != nullptr) {
      AddClosureNode *both = graph->create_node<AddClosureNode>();
      graph->connect(volume, both->input("Closure1"));
      graph->connect(scatter->output("Volume"), both->input("Closure2"));
      volume = both->output("Closure");
    }
    else {
      volume = scatter->output("Volume");
    }
  }
  if (volume != nullptr) {
    graph->connect(volume, graph->output()->input("Volume"));
  }
  Shader *shader = scene->create_node<Shader>();
  if (displaced) {
    shader->set_displacement_method(DISPLACE_TRUE);
  }
  shader->set_graph(std::move(graph));
  shader->tag_update(scene);
  return shader;
}

/* The shader tree's Shader layer: ray visibility and shadow catcher. */
void set_visibility(Object *object, const OcMesh &src, bool direct_caustics, bool lamps_only)
{
  /* Direct Caustics: glass casts, everything else receives (DC-Q04: a
   * see-through surface receives none). */
  const bool caster = direct_caustics && src.surface != nullptr &&
                      src.surface->transparency_mode == OC_TRANSPARENCY_GLASS &&
                      src.surface->visible_to_transmission != 0;
  object->set_is_caustics_caster(caster);
  object->set_is_caustics_receiver(direct_caustics && !caster);
  /* Light Linking: which lights this object receives and blocks. */
  object->set_receiver_light_set(src.light_set);
  object->set_blocker_shadow_set(src.light_set);
  /* Lamps only (OcSettings.lamps_only): a glowing surface belongs to no
   * light set, so it lights nothing. */
  object->set_light_set_membership(lamps_only ? 0 : src.light_membership);
  object->set_shadow_set_membership(src.light_membership);
  /* A kept object may have been another mesh's last time: start over. */
  object->set_visibility(PATH_RAY_VISIBILITY_ALL);
  object->set_is_shadow_catcher(false);
  object->set_lightgroup(ustring());
  if (src.surface == nullptr) {
    return;
  }
  const OcSurface &sf = *src.surface;
  uint visibility = PATH_RAY_VISIBILITY_ALL;
  const auto drop = [&](uint32_t keep, uint flag) {
    if (keep == 0) {
      visibility &= ~flag;
    }
  };
  drop(sf.visible_to_camera, PATH_RAY_VISIBILITY_CAMERA);
  drop(sf.visible_to_diffuse, PATH_RAY_VISIBILITY_DIFFUSE);
  drop(sf.visible_to_glossy, PATH_RAY_VISIBILITY_GLOSSY);
  drop(sf.visible_to_transmission, PATH_RAY_VISIBILITY_TRANSMIT);
  drop(sf.cast_shadows, PATH_RAY_VISIBILITY_SHADOW);
  object->set_visibility(visibility);
  object->set_is_shadow_catcher(sf.shadow_catcher != 0);
  /* The light group (LPE Label, LP-22) this glowing surface counts toward. */
  if (sf.lightgroup != nullptr) {
    object->set_lightgroup(ustring(sf.lightgroup));
  }
}

/* Generated texture coordinates for node graphs, as Blender gives them:
 * 0..1 across the mesh's bounding box per axis (a flat axis spans 2 m
 * around its centre, like Blender's texture space). */
void add_generated(AttributeSet &attrs, const OcMesh &src)
{
  if (src.surface == nullptr || src.surface->node_count == 0 || src.vertex_count == 0) {
    return;
  }
  float3 lo = make_float3(FLT_MAX), hi = make_float3(-FLT_MAX);
  for (uint32_t i = 0; i < src.vertex_count; i++) {
    const float3 p = make_float3(
        src.positions[i * 3], src.positions[i * 3 + 1], src.positions[i * 3 + 2]);
    lo = min(lo, p);
    hi = max(hi, p);
  }
  const float3 centre = (lo + hi) * 0.5f;
  float3 half = (hi - lo) * 0.5f;
  for (int k = 0; k < 3; k++) {
    float &h = k == 0 ? half.x : k == 1 ? half.y : half.z;
    if (h == 0.0f) {
      h = 1.0f;
    }
    else if (h < 1e-5f) {
      h = 1e-5f;
    }
  }
  Attribute *attr = attrs.add(ATTR_STD_GENERATED);
  attr->resize(src.vertex_count);
  packed_float3 *g = attr->data_for_write<packed_float3>();
  for (uint32_t i = 0; i < src.vertex_count; i++) {
    const float3 p = make_float3(
        src.positions[i * 3], src.positions[i * 3 + 1], src.positions[i * 3 + 2]);
    g[i] = (p - centre) / (half * 2.0f) + make_float3(0.5f);
  }
}

/* A displaced mesh: its triangles become linear subdivision faces, which
 * Cycles dices to about `dicing_rate` pixels as the camera sees them and
 * moves along the surface normal (true displacement). Linear, so the
 * shape is Operandi's own (subdivision already applied); only the
 * displacement adds detail. */
void build_displaced(Mesh *mesh, const OcMesh &src, const OcSettings &s, const Transform &tfm)
{
  const int verts = int(src.vertex_count), faces = int(src.triangle_count);
  mesh->set_subdivision_type(Mesh::SUBDIVISION_LINEAR);
  mesh->resize_mesh(verts, 0);
  mesh->resize_subd_faces(faces, faces * 3);
  Attribute *attr_p = mesh->subd_attributes.add(ATTR_STD_POSITION);
  attr_p->resize(verts);
  packed_float3 *p = attr_p->data_for_write<packed_float3>();
  for (int i = 0; i < verts; i++) {
    p[i] = make_float3(src.positions[i * 3], src.positions[i * 3 + 1], src.positions[i * 3 + 2]);
  }
  int *corners = mesh->get_subd_face_corners().data();
  int *start = mesh->get_subd_start_corner().data();
  int *count = mesh->get_subd_num_corners().data();
  int *ptex = mesh->get_subd_ptex_offset().data();
  for (int f = 0; f < faces; f++) {
    for (int k = 0; k < 3; k++) {
      corners[f * 3 + k] = int(src.triangles[f * 3 + k]);
    }
    start[f] = f * 3;
    count[f] = 3;
    ptex[f] = f * 3; /* a triangle is three ptex quads */
  }
  std::fill(mesh->get_subd_shader().begin(), mesh->get_subd_shader().end(), 0);
  array<bool> &smooth = mesh->get_subd_smooth();
  for (int f = 0; f < faces; f++) {
    smooth[f] = src.smooth != nullptr && src.smooth[f] != 0;
  }
  if (src.uvs != nullptr) {
    Attribute *attr = mesh->subd_attributes.add(ATTR_STD_UV, ustring("UVMap"));
    float2 *uv = attr->data_for_write<float2>();
    for (int i = 0; i < faces * 3; i++) {
      uv[i] = make_float2(src.uvs[i * 2], src.uvs[i * 2 + 1]);
    }
  }
  add_generated(mesh->subd_attributes, src);
  mesh->tag_subd_face_corners_modified();
  mesh->tag_subd_start_corner_modified();
  mesh->tag_subd_num_corners_modified();
  mesh->tag_subd_shader_modified();
  mesh->tag_subd_smooth_modified();
  mesh->tag_subd_ptex_offset_modified();
  mesh->set_subd_adaptive_space(Mesh::SUBDIVISION_ADAPTIVE_SPACE_PIXEL);
  mesh->set_subd_dicing_rate(s.dicing_rate > 0.0f ? s.dicing_rate : 1.0f);
  /* Each level halves a piece; 12 is Blender's default limit. */
  mesh->set_subd_max_level(12);
  mesh->set_subd_objecttoworld(tfm);
}

/* Math node helpers for the environment graph. */
MathNode *math(ShaderGraph *graph, NodeMathType type, ShaderOutput *a, float b)
{
  MathNode *m = graph->create_node<MathNode>();
  m->set_math_type(type);
  graph->connect(a, m->input("Value1"));
  m->set_value2(b);
  return m;
}

MixColorNode *mix_color(ShaderGraph *graph, float3 a, float3 b, ShaderOutput *fac)
{
  MixColorNode *mix = graph->create_node<MixColorNode>();
  mix->set_blend_type(NODE_MIX_BLEND);
  mix->set_use_clamp(true);
  mix->set_a(a);
  mix->set_b(b);
  graph->connect(fac, mix->input("Factor"));
  return mix;
}

/* The colour an environment shows along a background ray (spec
 * environments-2026-11-09 §4), before Intensity. s is the ray direction's
 * component along env.up (the item's +Y): sin(elevation).
 *   Constant:  zenith.
 *   2 Color:   lerp(nadir, zenith, (1 + s) / 2).
 *   4 Color:   s >= 0: lerp(zenith, sky, (1 - s)^sky_exponent);
 *              s < 0:  lerp(nadir, ground, (1 - |s|)^ground_exponent).
 *   Overcast:  s >= 0: zenith (1 + 2s) / 3; s < 0: nadir.
 * EnvironmentMaterial::color_at_height in operandi-core is the same maths;
 * change both together. nullptr for Constant: the caller sets the colour. */
ShaderOutput *environment_color(ShaderGraph *graph, const OcEnvironment &env)
{
  const float3 zenith = f3(env.zenith), sky = f3(env.sky), ground = f3(env.ground),
               nadir = f3(env.nadir);
  if (env.physical != 0) {
    /* Physically-based Daylight (spec §4): Cycles' Sky Texture, read along
     * the ray in the sky's Z-up frame. */
    GeometryNode *geo = graph->create_node<GeometryNode>();
    CombineXYZNode *xyz = graph->create_node<CombineXYZNode>();
    const char *axes[3] = {"X", "Y", "Z"};
    for (int r = 0; r < 3; r++) {
      VectorMathNode *dot = graph->create_node<VectorMathNode>();
      dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
      graph->connect(geo->output("Position"), dot->input("Vector1"));
      dot->set_vector2(make_float3(env.to_sky[r * 3], env.to_sky[r * 3 + 1], env.to_sky[r * 3 + 2]));
      graph->connect(dot->output("Value"), xyz->input(axes[r]));
    }
    SkyTextureNode *sky = graph->create_node<SkyTextureNode>();
    sky->set_sky_type(NODE_SKY_SINGLE_SCATTERING);
    sky->set_sun_elevation(env.sun_elevation);
    sky->set_sun_rotation(env.sun_rotation);
    sky->set_sun_disc(env.sun_size > 0.0f);
    sky->set_sun_size(env.sun_size > 0.0f ? env.sun_size : 0.0005f);
    sky->set_sun_intensity(1.0f);
    sky->set_ground_albedo(env.ground_albedo);
    sky->set_altitude(0.0f);
    sky->set_air_density(1.0f);
    sky->set_aerosol_density(env.aerosol);
    sky->set_ozone_density(1.0f);
    graph->connect(xyz->output("Vector"), sky->input("Vector"));
    return sky->output("Color");
  }
  if (env.kind == OC_ENV_CONSTANT) {
    return nullptr;
  }
  /* For background shading Cycles sets P to the ray direction (unit). */
  GeometryNode *geometry = graph->create_node<GeometryNode>();
  VectorMathNode *dot = graph->create_node<VectorMathNode>();
  dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
  graph->connect(geometry->output("Position"), dot->input("Vector1"));
  dot->set_vector2(f3(env.up));
  ShaderOutput *s = dot->output("Value");

  if (env.kind == OC_ENV_TWO_COLOR) {
    MathNode *fac = math(graph, NODE_MATH_MULTIPLY_ADD, s, 0.5f);
    fac->set_value3(0.5f);
    return mix_color(graph, nadir, zenith, fac->output("Value"))->output("Result");
  }
  /* Below the horizon (s < 0): 1, else 0. Sky above, ground below, with a
   * hard edge at the horizon. */
  ShaderOutput *below = math(graph, NODE_MATH_LESS_THAN, s, 0.0f)->output("Value");
  ShaderOutput *upper = nullptr;
  ShaderOutput *lower = nullptr;
  if (env.kind == OC_ENV_OVERCAST) {
    MathNode *f = math(graph, NODE_MATH_MULTIPLY_ADD, s, 2.0f / 3.0f);
    f->set_value3(1.0f / 3.0f);
    upper = mix_color(graph, make_float3(0.0f, 0.0f, 0.0f), zenith, f->output("Value"))
                ->output("Result");
    /* lower stays nullptr: a flat nadir below. */
  }
  else {
    ShaderOutput *magnitude = math(graph, NODE_MATH_ABSOLUTE, s, 0.0f)->output("Value");
    /* 1 - |s| (Subtract is Value1 - Value2). */
    MathNode *sub = graph->create_node<MathNode>();
    sub->set_math_type(NODE_MATH_SUBTRACT);
    sub->set_value1(1.0f);
    graph->connect(magnitude, sub->input("Value2"));
    ShaderOutput *t = sub->output("Value");
    ShaderOutput *up_t = math(graph, NODE_MATH_POWER, t, env.sky_exponent)->output("Value");
    ShaderOutput *down_t =
        math(graph, NODE_MATH_POWER, t, env.ground_exponent)->output("Value");
    upper = mix_color(graph, zenith, sky, up_t)->output("Result");
    lower = mix_color(graph, nadir, ground, down_t)->output("Result");
  }
  MixColorNode *pick = graph->create_node<MixColorNode>();
  pick->set_blend_type(NODE_MIX_BLEND);
  graph->connect(upper, pick->input("A"));
  if (lower != nullptr) {
    graph->connect(lower, pick->input("B"));
  }
  else {
    pick->set_b(nadir);
  }
  graph->connect(below, pick->input("Factor"));
  return pick->output("Result");
}

/* An environment's whole colour (spec §5): the Environment Material over
 * black at its opacity (inverted if asked), then each texture layer mixed
 * over it, bottom first, with its Mix Color mode at its opacity. A
 * constant layer is one colour; an image layer is an Environment Texture
 * (equirectangular, or mirror ball for a Light Probe) read along the ray
 * direction taken into its frame by
 * to_image. nullptr for a plain Constant material with no layers (the
 * caller sets env.zenith). */
ShaderOutput *environment_stack(ShaderGraph *graph, const OcEnvironment &env)
{
  ShaderOutput *color = environment_color(graph, env);
  const uint32_t layers = env.layers != nullptr ? env.layer_count : 0;
  if (env.material_invert == 0 && env.material_opacity >= 1.0f && layers == 0) {
    return color;
  }
  /* The colour so far: an output, or (while still constant) a value. */
  float3 flat = f3(env.zenith);
  auto feed = [&](MixColorNode *mix, const char *socket) {
    if (color != nullptr) {
      graph->connect(color, mix->input(socket));
    }
    else if (strcmp(socket, "A") == 0) {
      mix->set_a(flat);
    }
    else {
      mix->set_b(flat);
    }
  };
  if (env.material_invert != 0) {
    InvertNode *inv = graph->create_node<InvertNode>();
    inv->set_fac(1.0f);
    if (color != nullptr) {
      graph->connect(color, inv->input("Color"));
    }
    else {
      inv->set_color(flat);
    }
    color = inv->output("Color");
  }
  if (env.material_opacity < 1.0f) {
    MixColorNode *over = graph->create_node<MixColorNode>();
    over->set_blend_type(NODE_MIX_BLEND);
    over->set_a(make_float3(0.0f, 0.0f, 0.0f));
    feed(over, "B");
    over->set_fac(env.material_opacity);
    color = over->output("Result");
  }
  GeometryNode *geometry = nullptr;
  for (uint32_t k = 0; k < layers; k++) {
    const OcEnvLayer &l = env.layers[k];
    MixColorNode *mix = graph->create_node<MixColorNode>();
    mix->set_blend_type(NodeMix(l.mix));
    mix->set_fac(l.opacity);
    feed(mix, "A");
    ShaderOutput *layer = nullptr;
    if (l.kind == 1 || l.kind == 2) {
      if (geometry == nullptr) {
        geometry = graph->create_node<GeometryNode>();
      }
      CombineXYZNode *xyz = graph->create_node<CombineXYZNode>();
      const char *axes[3] = {"X", "Y", "Z"};
      for (int r = 0; r < 3; r++) {
        VectorMathNode *dot = graph->create_node<VectorMathNode>();
        dot->set_math_type(NODE_VECTOR_MATH_DOT_PRODUCT);
        graph->connect(geometry->output("Position"), dot->input("Vector1"));
        dot->set_vector2(make_float3(l.to_image[r * 3], l.to_image[r * 3 + 1], l.to_image[r * 3 + 2]));
        graph->connect(dot->output("Value"), xyz->input(axes[r]));
      }
      EnvironmentTextureNode *tex = graph->create_node<EnvironmentTextureNode>();
      tex->set_filename(ustring(l.image != nullptr ? l.image : ""));
      tex->set_colorspace(picture_colorspace(l.image, false));
      tex->set_projection(l.kind == 2 ? NODE_ENVIRONMENT_MIRROR_BALL :
                                        NODE_ENVIRONMENT_EQUIRECTANGULAR);
      graph->connect(xyz->output("Vector"), tex->input("Vector"));
      layer = tex->output("Color");
    }
    if (l.invert != 0) {
      InvertNode *inv = graph->create_node<InvertNode>();
      inv->set_fac(1.0f);
      if (layer != nullptr) {
        graph->connect(layer, inv->input("Color"));
      }
      else {
        inv->set_color(f3(l.color));
      }
      layer = inv->output("Color");
    }
    if (layer != nullptr) {
      graph->connect(layer, mix->input("B"));
    }
    else {
      mix->set_b(f3(l.color));
    }
    color = mix->output("Result");
  }
  if (color == nullptr) {
    /* Only reachable with nothing to do; keep the flat colour. */
    MixColorNode *k = graph->create_node<MixColorNode>();
    k->set_a(flat);
    k->set_fac(0.0f);
    color = k->output("Result");
  }
  return color;
}

/* What a kept session built last time, for reuse. This follows the
 * pattern of Blender's Cycles sync (intern/cycles/blender/id_map.h, sync.cpp):
 * look each piece up by a key, build it only when missing, and delete
 * whatever the new scene no longer uses. Objects are kept by position, so
 * when nothing moved their setters change nothing and Cycles does no work. */
struct Kept {
  /* Surfaces by full key (shader_key), meshes by geometry key and surface. */
  std::map<uint64_t, Shader *> shaders;
  std::map<std::pair<uint64_t, Shader *>, Mesh *> meshes;
  /* One object per drawn mesh, in OcMesh order. */
  vector<Object *> objects;
  /* Built without a key: replaced on every render. */
  set<Shader *> loose_shaders;
  set<Geometry *> loose_geometry;
  bool default_built = false;
  std::string default_signature;
  bool world_built = false;
  uint64_t world_key = 0;
  bool lights_built = false;
  uint64_t lights_key = 0;
  bool default_light_built = false;
  set<Object *> light_objects;
  set<Geometry *> light_geometry;
  set<Shader *> light_shaders;
  std::string pass_signature;
  set<Pass *> passes;
};

/* The Render Output passes asked for (with somewhere to go), as text. */
std::string pass_signature(const OcSettings &s)
{
  std::string out;
  for (uint32_t k = 0; s.passes != nullptr && k < s.pass_count; k++) {
    if (s.passes[k] < OC_PASS_COUNT && s.pass_out != nullptr && s.pass_out[k] != nullptr) {
      out += std::to_string(s.passes[k]) + ",";
    }
  }
  out += "|";
  for (uint32_t k = 0; s.value_passes != nullptr && k < s.value_count; k++) {
    if (s.value_passes[k] < OC_VALUE_COUNT && s.value_out != nullptr && s.value_out[k] != nullptr) {
      out += "v" + std::to_string(s.value_passes[k]) + ",";
    }
  }
  out += "|";
  for (uint32_t g = 0; s.light_groups != nullptr && g < s.light_group_count; g++) {
    if (s.light_groups[g] != nullptr && s.light_group_out != nullptr &&
        s.light_group_out[g] != nullptr)
    {
      out += std::string(s.light_groups[g]) + '\0';
    }
  }
  return out;
}

/* A surface's key with what build_surface also reads from the settings:
 * the output AOVs, the picture's aspect (screen coordinates) and the fog
 * colour taken from the environment. 0 = no key. */
uint64_t shader_key(const OcSurface &src, uint64_t surface_key, const OcSettings &s)
{
  if (surface_key == 0) {
    return 0;
  }
  std::string text = std::to_string(surface_key) + "|" + pass_signature(s);
  bool screen = false;
  for (uint32_t i = 0; src.exprs != nullptr && i < src.expr_count; i++) {
    const OcExpr &e = src.exprs[i];
    screen = screen || (e.op == OC_EXPR_GEOMETRY && e.mode >= OC_GEOMETRY_SCREEN_X &&
                        e.mode <= OC_GEOMETRY_SCREEN_R);
  }
  char buffer[96];
  if (screen) {
    std::snprintf(buffer, sizeof(buffer), "|aspect %u %u", s.width, s.height);
    text += buffer;
  }
  if (src.transparency_mode == OC_TRANSPARENCY_GLASS && s.direct_caustics == 0) {
    /* F7: glass shadows change with Direct Caustics. */
    text += "|glass shadow";
  }
  if (src.fog_type != OC_FOG_NONE && src.fog_use_environment != 0) {
    std::snprintf(buffer,
                  sizeof(buffer),
                  "|fog %a %a %a",
                  s.environment_fog_color[0],
                  s.environment_fog_color[1],
                  s.environment_fog_color[2]);
    text += buffer;
  }
  const uint64_t key = uint64_t(std::hash<std::string>{}(text));
  return key == 0 ? 1 : key;
}

/* OcMesh.transform (row-major 3x4) as a Cycles transform. */
Transform mesh_transform(const OcMesh &src)
{
  const float *m = src.transform;
  return make_transform(
      m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11]);
}

void sync_camera(Scene *scene, const OcSettings &s)
{
  /* Camera. Cycles' camera looks down its local +Z with +Y up, so the
   * matrix columns are simply right, up, forward and the eye position.
   * Displaced meshes are diced as seen through the same camera. */
  for (Camera *cam : {scene->camera, scene->dicing_camera}) {
    cam->set_full_width(int(s.width));
    cam->set_full_height(int(s.height));
    const bool panorama = s.panorama == 1 || s.panorama == 2;
    cam->set_camera_type(panorama                 ? CAMERA_PANORAMA :
                         s.camera_orthographic ? CAMERA_ORTHOGRAPHIC :
                                                 CAMERA_PERSPECTIVE);
    cam->set_fov(s.fov);
    cam->set_nearclip(s.clip_near > 0.0f ? s.clip_near : 0.01f);
    cam->set_farclip(s.clip_far > s.clip_near ? s.clip_far : 1.0e5f);
    if (panorama) {
      /* Cycles' panoramas look down camera +X with +Z up, and the picture
       * runs toward camera -Y left to right: columns forward, -right, up. */
      cam->set_matrix(make_transform(s.camera_forward[0],
                                     -s.camera_right[0],
                                     s.camera_up[0],
                                     s.camera_position[0],
                                     s.camera_forward[1],
                                     -s.camera_right[1],
                                     s.camera_up[1],
                                     s.camera_position[1],
                                     s.camera_forward[2],
                                     -s.camera_right[2],
                                     s.camera_up[2],
                                     s.camera_position[2]));
      if (s.panorama == 1) {
        cam->set_panorama_type(PANORAMA_EQUIRECTANGULAR);
        cam->set_longitude_min(s.panorama_range[0]);
        cam->set_longitude_max(s.panorama_range[1]);
        cam->set_latitude_min(s.panorama_range[2]);
        cam->set_latitude_max(s.panorama_range[3]);
      }
      else {
        cam->set_panorama_type(PANORAMA_CENTRAL_CYLINDRICAL);
        cam->set_central_cylindrical_range_u_min(s.panorama_range[0]);
        cam->set_central_cylindrical_range_u_max(s.panorama_range[1]);
        cam->set_central_cylindrical_range_v_min(s.panorama_range[2]);
        cam->set_central_cylindrical_range_v_max(s.panorama_range[3]);
      }
    }
    else {
      cam->set_matrix(make_transform(s.camera_right[0],
                                     s.camera_up[0],
                                     s.camera_forward[0],
                                     s.camera_position[0],
                                     s.camera_right[1],
                                     s.camera_up[1],
                                     s.camera_forward[1],
                                     s.camera_position[1],
                                     s.camera_right[2],
                                     s.camera_up[2],
                                     s.camera_forward[2],
                                     s.camera_position[2]));
    }
    /* Depth of field (renders only, CP-Q6). */
    cam->set_aperturesize(s.aperture_size > 0.0f ? s.aperture_size : 0.0f);
    cam->set_focaldistance(s.focal_distance > 0.0f ? s.focal_distance : 10.0f);
    cam->set_blades(s.blades >= 3 ? s.blades : 0);
    cam->set_bladesrotation(s.blades_rotation);
    cam->compute_auto_viewplane();
    if (!panorama) {
      /* The auto view plane spans -1..1 across the shorter side. An
       * orthographic camera measures it in metres; a film offset moves it. */
      float left = cam->get_viewplane_left(), right = cam->get_viewplane_right();
      float bottom = cam->get_viewplane_bottom(), top = cam->get_viewplane_top();
      if (s.camera_orthographic) {
        left *= s.ortho_half_short;
        right *= s.ortho_half_short;
        bottom *= s.ortho_half_short;
        top *= s.ortho_half_short;
      }
      const float dx = s.shift[0] * 0.5f * (right - left);
      const float dy = s.shift[1] * 0.5f * (top - bottom);
      cam->set_viewplane_left(left + dx);
      cam->set_viewplane_right(right + dx);
      cam->set_viewplane_bottom(bottom + dy);
      cam->set_viewplane_top(top + dy);
    }
    if (cam->is_modified()) {
      cam->need_flags_update = true;
    }
  }

}

/* Default material: a plain principled surface. */
void sync_default_surface(Kept &kept, Scene *scene, const OcSettings &s)
{
  char buffer[128];
  std::snprintf(buffer,
                sizeof(buffer),
                "%a %a %a %a|",
                s.base_color[0],
                s.base_color[1],
                s.base_color[2],
                s.roughness);
  const std::string signature = buffer + pass_signature(s);
  if (kept.default_built && signature == kept.default_signature) {
    return;
  }
  kept.default_built = true;
  kept.default_signature = signature;
  unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
  PrincipledBsdfNode *bsdf = graph->create_node<PrincipledBsdfNode>();
  bsdf->set_base_color(f3(s.base_color));
  bsdf->set_roughness(s.roughness);
  graph->connect(bsdf->output("BSDF"), graph->output()->input("Surface"));
  add_output_aovs(graph.get(), s);
  Shader *surface = scene->default_surface;
  /* Its Cryptomatte Material name (the default material's tag). */
  surface->name = ustring("Default");
  surface->set_graph(std::move(graph));
  surface->tag_update(scene);
}

void sync_world(Kept &kept, Scene *scene, const OcSettings &s)
{
  scene->background->set_transparent(s.transparent_film != 0);
  scene->background->set_visibility(s.world_hidden_from_diffuse != 0 ?
                                        PATH_RAY_VISIBILITY_ALL & ~PATH_RAY_VISIBILITY_DIFFUSE :
                                        PATH_RAY_VISIBILITY_ALL);
  /* Light Path Expressions (group D): the world's light group. */
  scene->background->set_lightgroup(
      ustring(s.world_lightgroup != nullptr ? s.world_lightgroup : ""));
  if (kept.world_built && s.world_key != 0 && s.world_key == kept.world_key) {
    return;
  }
  kept.world_built = true;
  kept.world_key = s.world_key;
  /* World (spec environments-2026-11-09 §6, §10): each ray kind sees the
   * newest environment visible to it; they are not added together. Camera,
   * glossy (reflection) and transmission (refraction) rays are told apart
   * with Light Path outputs; every other ray is indirect (diffuse). Dome
   * lights add to every ray but camera rays. */
  {
    unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
    LightPathNode *path = nullptr;
    auto light_path = [&]() {
      if (path == nullptr) {
        path = graph->create_node<LightPathNode>();
      }
      return path;
    };
    /* One Background closure per environment, built when first used. */
    std::map<uint32_t, ShaderOutput *> closures;
    auto closure = [&](uint32_t i) -> ShaderOutput * {
      auto found = closures.find(i);
      if (found != closures.end()) {
        return found->second;
      }
      const OcEnvironment &env = s.environments[i];
      BackgroundNode *bg = graph->create_node<BackgroundNode>();
      bg->set_strength(env.intensity);
      ShaderOutput *color = environment_stack(graph.get(), env);
      if (color != nullptr) {
        graph->connect(color, bg->input("Color"));
      }
      else {
        bg->set_color(f3(env.zenith));
      }
      closures[i] = bg->output("Background");
      return bg->output("Background");
    };
    /* The environment each ray kind sees (camera, diffuse, glossy,
     * transmission), or -1 for none. */
    int seen[4] = {-1, -1, -1, -1};
    for (int k = 0; k < 4; k++) {
      for (uint32_t i = 0; s.environments != nullptr && i < s.environment_count; i++) {
        if (s.environments[i].visible[k] != 0) {
          seen[k] = int(i);
          break;
        }
      }
    }
    auto of = [&](int k) -> ShaderOutput * {
      return seen[k] < 0 ? nullptr : closure(uint32_t(seen[k]));
    };
    /* fac ? b : a, where a missing closure is black. */
    auto choose = [&](const char *ray, ShaderOutput *a, ShaderOutput *b) -> ShaderOutput * {
      if (a == b) {
        return a;
      }
      MixClosureNode *mix = graph->create_node<MixClosureNode>();
      graph->connect(light_path()->output(ray), mix->input("Fac"));
      if (a != nullptr) {
        graph->connect(a, mix->input("Closure1"));
      }
      if (b != nullptr) {
        graph->connect(b, mix->input("Closure2"));
      }
      return mix->output("Closure");
    };
    /* Indirect rays are everything that isn't one of the other three. */
    ShaderOutput *bounce = choose("Is Glossy Ray", of(1), of(2));
    bounce = choose("Is Transmission Ray", bounce, of(3));
    const float3 dome = f3(s.dome);
    if (dome.x > 0.0f || dome.y > 0.0f || dome.z > 0.0f) {
      /* Dome lights: added to the world for every ray but camera rays,
       * so they light the scene without showing in the background. */
      BackgroundNode *dome_bg = graph->create_node<BackgroundNode>();
      dome_bg->set_color(dome);
      dome_bg->set_strength(1.0f);
      if (bounce == nullptr) {
        bounce = dome_bg->output("Background");
      }
      else {
        AddClosureNode *both = graph->create_node<AddClosureNode>();
        graph->connect(bounce, both->input("Closure1"));
        graph->connect(dome_bg->output("Background"), both->input("Closure2"));
        bounce = both->output("Closure");
      }
    }
    ShaderOutput *world = choose("Is Camera Ray", bounce, of(0));
    if (world != nullptr) {
      graph->connect(world, graph->output()->input("Surface"));
    }
    Shader *background = scene->default_background;
    background->set_graph(std::move(graph));
    background->tag_update(scene);
  }

}

void sync_lights(Kept &kept, Scene *scene, const OcSettings &s)
{
  if (kept.lights_built && s.lights_key != 0 && s.lights_key == kept.lights_key) {
    return;
  }
  kept.lights_built = true;
  kept.lights_key = s.lights_key;
  /* Out with the old lights, then note every node the new ones add. */
  scene->delete_nodes(kept.light_objects);
  scene->delete_nodes(kept.light_geometry);
  scene->delete_nodes(kept.light_shaders);
  kept.light_objects.clear();
  kept.light_geometry.clear();
  kept.light_shaders.clear();
  const size_t objects_before = scene->objects.size();
  const size_t geometry_before = scene->geometry.size();
  const size_t shaders_before = scene->shaders.size();
  /* Sun(s). Light colour comes from the default light shader (emission 1)
   * times each sun's strength. */
  const bool any_light = s.use_scene_lights ? (s.lights != nullptr && s.light_count > 0) :
                                              s.sun_strength > 0.0f;
  if (any_light && !kept.default_light_built) {
    kept.default_light_built = true;
    unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
    EmissionNode *emission = graph->create_node<EmissionNode>();
    emission->set_color(make_float3(1.0f, 1.0f, 1.0f));
    emission->set_strength(1.0f);
    graph->connect(emission->output("Emission"), graph->output()->input("Surface"));
    scene->default_light->set_graph(std::move(graph));
    scene->default_light->tag_update(scene);
  }
  /* Stage 5 (HDRI): a world with an environment picture is importance
   * sampled, so its bright parts (a sun in the picture) light the scene
   * with little noise, as Blender does for every world. */
  bool world_image = false;
  for (uint32_t e = 0; s.environments != nullptr && e < s.environment_count; e++) {
    const OcEnvironment &env = s.environments[e];
    for (uint32_t k = 0; env.layers != nullptr && k < env.layer_count; k++) {
      if (env.layers[k].kind == 1 && env.layers[k].image != nullptr && env.layers[k].image[0] != 0) {
        world_image = true;
      }
    }
  }
  bool any_portal = false;
  if (s.use_scene_lights) {
    for (uint32_t i = 0; s.lights != nullptr && i < s.light_count; i++) {
      const OcLight &l = s.lights[i];
      Light *light = nullptr;
      if (l.kind == 4) {
        /* Portal: an area light flagged as a portal. Cycles samples the
         * world through it and never adds light of its own. */
        AreaLight *area = scene->create_node<AreaLight>();
        area->set_sizeu(l.size[0]);
        area->set_sizev(l.size[1]);
        area->set_ellipse(l.ellipse != 0);
        area->set_is_portal(true);
        any_portal = true;
        light = area;
      }
      else if (l.kind == 1 || l.kind == 2) {
        PointLight *point = l.kind == 2 ? scene->create_node<SpotLight>() :
                                          scene->create_node<PointLight>();
        point->set_radius(l.radius);
        if (l.kind == 2) {
          SpotLight *spot = static_cast<SpotLight *>(point);
          spot->set_angle(l.spot_size);
          spot->set_smooth(l.spot_blend);
        }
        light = point;
      }
      else if (l.kind == 3) {
        AreaLight *area = scene->create_node<AreaLight>();
        area->set_sizeu(l.size[0]);
        area->set_sizev(l.size[1]);
        area->set_ellipse(l.ellipse != 0);
        light = area;
      }
      else {
        SunLight *sun = scene->create_node<SunLight>();
        sun->set_angle(l.angle);
        light = sun;
      }
      light->set_strength(f3(l.strength));
      light->set_cast_shadow(l.cast_shadow != 0 && s.no_shadows == 0);
      /* Bounced rays only hit a light with MIS on (Cycles' light
       * intersection), so this is how reflections/refractions see it. */
      light->set_use_mis(l.visible_to_reflection != 0 || l.visible_to_refraction != 0);
      light->set_use_caustics(s.direct_caustics != 0 && l.caustics != 0);
      light->set_caustics_only(s.direct_caustics != 0 && l.caustics == 2);
      array<Node *> light_shaders;
      const bool layered = l.layer_count > 0 && l.gobo_tan_half > 0.0f && l.layer_images != nullptr;
      if (l.ies != nullptr || layered) {
        /* Photometric: emission strength from the IES profile (watts,
         * Cycles converts candela itself), white, times the light's
         * strength. */
        unique_ptr<ShaderGraph> ies_graph = make_unique<ShaderGraph>();
        EmissionNode *ies_emission = ies_graph->create_node<EmissionNode>();
        ies_emission->set_color(make_float3(1.0f, 1.0f, 1.0f));
        ies_emission->set_strength(1.0f);
        if (l.ies != nullptr) {
          IESLightNode *ies_node = ies_graph->create_node<IESLightNode>();
          ies_node->set_ies(ustring(l.ies));
          ies_graph->connect(ies_node->output("Fac"), ies_emission->input("Strength"));
        }
        if (layered) {
          /* Gobo: the light-space direction (texture coordinate Normal)
           * projected through the cone, u, v = 0.5 + 0.5 * (x, y) /
           * (-z * tan(half cone)), each layer mixed over white. */
          TextureCoordinateNode *coords = ies_graph->create_node<TextureCoordinateNode>();
          SeparateXYZNode *xyz = ies_graph->create_node<SeparateXYZNode>();
          ies_graph->connect(coords->output("Normal"), xyz->input("Vector"));
          MathNode *depth = ies_graph->create_node<MathNode>();
          depth->set_math_type(NODE_MATH_MULTIPLY);
          depth->set_value2(-l.gobo_tan_half * 2.0f);
          ies_graph->connect(xyz->output("Z"), depth->input("Value1"));
          auto axis = [&](const char *name) {
            MathNode *div = ies_graph->create_node<MathNode>();
            div->set_math_type(NODE_MATH_DIVIDE);
            ies_graph->connect(xyz->output(name), div->input("Value1"));
            ies_graph->connect(depth->output("Value"), div->input("Value2"));
            MathNode *add = ies_graph->create_node<MathNode>();
            add->set_math_type(NODE_MATH_ADD);
            add->set_value2(0.5f);
            ies_graph->connect(div->output("Value"), add->input("Value1"));
            return add->output("Value");
          };
          CombineXYZNode *uv = ies_graph->create_node<CombineXYZNode>();
          ies_graph->connect(axis("X"), uv->input("X"));
          ies_graph->connect(axis("Y"), uv->input("Y"));
          ShaderOutput *color = nullptr;
          for (uint32_t k = 0; k < l.layer_count; k++) {
            ImageTextureNode *tex = ies_graph->create_node<ImageTextureNode>();
            tex->set_filename(ustring(l.layer_images[k] ? l.layer_images[k] : ""));
            tex->set_colorspace(picture_colorspace(l.layer_images[k], false));
            tex->set_extension(EXTENSION_CLIP);
            ies_graph->connect(uv->output("Vector"), tex->input("Vector"));
            MixColorNode *mix = ies_graph->create_node<MixColorNode>();
            mix->set_blend_type(NodeMix(l.layer_blend ? l.layer_blend[k] : 0));
            mix->set_use_clamp(true);
            mix->set_fac(l.layer_opacity ? l.layer_opacity[k] : 1.0f);
            if (color != nullptr) {
              ies_graph->connect(color, mix->input("A"));
            }
            else {
              mix->set_a(make_float3(1.0f, 1.0f, 1.0f));
            }
            ies_graph->connect(tex->output("Color"), mix->input("B"));
            color = mix->output("Result");
          }
          if (color != nullptr) {
            ies_graph->connect(color, ies_emission->input("Color"));
          }
        }
        ies_graph->connect(ies_emission->output("Emission"),
                           ies_graph->output()->input("Surface"));
        Shader *ies_shader = scene->create_node<Shader>();
        ies_shader->name = "photometric";
        ies_shader->set_graph(std::move(ies_graph));
        ies_shader->tag_update(scene);
        light_shaders.push_back_slow(ies_shader);
      }
      else {
        light_shaders.push_back_slow(scene->default_light);
      }
      light->set_used_shaders(light_shaders);
      Object *sun_object = scene->create_node<Object>();
      sun_object->set_geometry(light);
      if (l.lightgroup != nullptr) {
        sun_object->set_lightgroup(ustring(l.lightgroup));
      }
      sun_object->set_light_set_membership(l.light_membership);
      sun_object->set_shadow_set_membership(l.shadow_membership);
      /* Columns: right, up, back (+Z; the light travels along -Z), position. */
      const float3 x = f3(l.right), y = f3(l.up), z = -f3(l.direction), p = f3(l.position);
      sun_object->set_tfm(
          make_transform(x.x, y.x, z.x, p.x, x.y, y.y, z.y, p.y, x.z, y.z, z.z, p.z));
      uint visibility = PATH_RAY_VISIBILITY_ALL;
      if (!l.visible_to_camera) {
        visibility &= ~PATH_RAY_VISIBILITY_CAMERA;
      }
      if (!l.affects_diffuse) {
        visibility &= ~PATH_RAY_VISIBILITY_DIFFUSE;
      }
      if (!l.affects_specular) {
        visibility &= ~PATH_RAY_VISIBILITY_GLOSSY;
      }
      sun_object->set_visibility(visibility);
    }
  }
  else if (s.sun_strength > 0.0f) {
    SunLight *sun = scene->create_node<SunLight>();
    sun->set_strength(make_float3(s.sun_strength, s.sun_strength, s.sun_strength));
    sun->set_angle(0.05f);
    sun->set_cast_shadow(s.no_shadows == 0);
    array<Node *> light_shaders;
    light_shaders.push_back_slow(scene->default_light);
    sun->set_used_shaders(light_shaders);
    Object *sun_object = scene->create_node<Object>();
    sun_object->set_geometry(sun);
    sun_object->set_tfm(aim_along(f3(s.sun_direction)));
    sun_object->set_visibility(PATH_RAY_VISIBILITY_ALL & ~PATH_RAY_VISIBILITY_CAMERA);
  }

  if (any_portal || world_image) {
    /* Portals and importance sampling work through the world's background
     * light, which Cycles only creates when asked (Blender adds it for
     * every world). Cycles picks its map resolution itself (auto). */
    BackgroundLight *background_light = scene->create_node<BackgroundLight>();
    background_light->set_strength(make_float3(1.0f, 1.0f, 1.0f));
    array<Node *> background_shaders;
    background_shaders.push_back_slow(scene->default_background);
    background_light->set_used_shaders(background_shaders);
    Object *background_object = scene->create_node<Object>();
    background_object->set_geometry(background_light);
  }

  kept.light_objects.insert(scene->objects.begin() + objects_before, scene->objects.end());
  kept.light_geometry.insert(scene->geometry.begin() + geometry_before, scene->geometry.end());
  kept.light_shaders.insert(scene->shaders.begin() + shaders_before, scene->shaders.end());
}

/* Fills a (new) mesh from its OcMesh: points, triangles, smooth flags,
 * UVs, corner normals and attributes. */
void fill_mesh(Mesh *mesh, const OcMesh &src)
{
  mesh->resize_mesh(int(src.vertex_count), int(src.triangle_count));
  packed_float3 *p = mesh->get_position_for_write();
  for (uint32_t i = 0; i < src.vertex_count; i++) {
    p[i] = make_float3(src.positions[i * 3], src.positions[i * 3 + 1], src.positions[i * 3 + 2]);
  }
  int *tris = mesh->get_triangles().data();
  for (uint32_t i = 0; i < src.triangle_count * 3; i++) {
    tris[i] = int(src.triangles[i]);
  }
  array<bool> &smooth = mesh->get_smooth();
  for (uint32_t i = 0; i < src.triangle_count; i++) {
    smooth[i] = src.smooth != nullptr && src.smooth[i] != 0;
  }
  std::fill(mesh->get_shader().begin(), mesh->get_shader().end(), 0);
  if (src.uvs != nullptr) {
    Attribute *attr = mesh->attributes.add(ATTR_STD_UV, ustring("UVMap"));
    float2 *uv = attr->data_for_write<float2>();
    for (uint32_t i = 0; i < src.triangle_count * 3; i++) {
      uv[i] = make_float2(src.uvs[i * 2], src.uvs[i * 2 + 1]);
    }
  }
  /* Exact shading normals per triangle corner, as Blender's sync passes
   * its corner normals (blender/mesh.cpp): smooth triangles shade with
   * these instead of Cycles' average around each point. */
  if (src.normals != nullptr) {
    Attribute *attr = mesh->attributes.add(ATTR_STD_CORNER_NORMAL);
    packed_normal *n = attr->data_for_write<packed_normal>();
    for (uint32_t i = 0; i < src.triangle_count * 3; i++) {
      n[i] = packed_normal(
          make_float3(src.normals[i * 3], src.normals[i * 3 + 1], src.normals[i * 3 + 2]));
    }
  }
  /* Vertex maps and variation numbers: RGBA per corner. */
  if (src.attribute_values != nullptr && src.surface != nullptr &&
      src.surface->attributes != nullptr)
  {
    const uint32_t count = src.surface->attribute_count;
    for (uint32_t k = 0; k < count; k++) {
      const char *name = src.surface->attributes[k];
      Attribute *attr = mesh->attributes.add(
          ustring(name ? name : ""), TypeRGBA, ATTR_ELEMENT_CORNER);
      float4 *d = attr->data_for_write<float4>();
      for (uint32_t c = 0; c < src.triangle_count * 3; c++) {
        const float *v = src.attribute_values + (size_t(c) * count + k) * 4;
        d[c] = make_float4(v[0], v[1], v[2], v[3]);
      }
    }
  }
  add_generated(mesh->attributes, src);
  mesh->tag_triangles_modified();
  mesh->tag_smooth_modified();
  mesh->tag_shader_modified();
}

/* The meshes: each surface built once (by key) and shared by every mesh
 * that uses it, each mesh's points sent once (by key) and placed by its
 * object's transform, so equal meshes are instances of one. */
void sync_meshes(Kept &kept, Scene *scene, const OcMesh *meshes, uint32_t mesh_count, const OcSettings &s)
{
  std::map<uint64_t, Shader *> shaders;
  std::map<std::pair<uint64_t, Shader *>, Mesh *> kept_meshes;
  set<Shader *> loose_shaders;
  set<Geometry *> loose_geometry;
  auto build_material = [&](const OcMesh &src) {
    /* A mesh with its own material gets its own principled surface, with
     * the default material's roughness. */
    unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
    PrincipledBsdfNode *bsdf = graph->create_node<PrincipledBsdfNode>();
    bsdf->set_base_color(f3(src.base_color));
    bsdf->set_roughness(s.roughness);
    bsdf->set_ior(src.ior);
    graph->connect(bsdf->output("BSDF"), graph->output()->input("Surface"));
    add_output_aovs(graph.get(), s);
    Shader *surface = scene->create_node<Shader>();
    surface->set_graph(std::move(graph));
    surface->tag_update(scene);
    return surface;
  };
  size_t slot = 0;
  for (uint32_t m = 0; m < mesh_count; m++) {
    const OcMesh &src = meshes[m];
    if (src.vertex_count == 0 || src.triangle_count == 0) {
      continue;
    }
    Shader *surface = scene->default_surface;
    /* Cryptomatte Material names surfaces, but clay meshes share the
     * default surface: with a Material cryptomatte asked for, a clay mesh
     * of another material gets a clay surface of its own by that name. */
    const bool named_clay = src.surface == nullptr && src.has_material == 0 &&
                            (cryptomatte_wanted(s).first & CRYPT_MATERIAL) != 0 &&
                            src.material_name != nullptr && src.material_name[0] != '\0' &&
                            std::strcmp(src.material_name, "Default") != 0;
    if (named_clay) {
      char buffer[96];
      std::snprintf(buffer,
                    sizeof(buffer),
                    "clay %a %a %a %a|",
                    s.base_color[0],
                    s.base_color[1],
                    s.base_color[2],
                    s.roughness);
      const uint64_t key =
          std::hash<std::string>()(buffer + pass_signature(s) + src.material_name) | 1u;
      if (auto found = shaders.find(key); found != shaders.end()) {
        surface = found->second;
      }
      else if (auto old = kept.shaders.find(key); old != kept.shaders.end()) {
        surface = old->second;
        shaders[key] = surface;
      }
      else {
        unique_ptr<ShaderGraph> graph = make_unique<ShaderGraph>();
        PrincipledBsdfNode *bsdf = graph->create_node<PrincipledBsdfNode>();
        bsdf->set_base_color(f3(s.base_color));
        bsdf->set_roughness(s.roughness);
        graph->connect(bsdf->output("BSDF"), graph->output()->input("Surface"));
        add_output_aovs(graph.get(), s);
        surface = scene->create_node<Shader>();
        surface->set_graph(std::move(graph));
        surface->tag_update(scene);
        shaders[key] = surface;
      }
    }
    if (src.surface != nullptr) {
      const uint64_t key = shader_key(*src.surface, src.surface_key, s);
      if (key == 0) {
        surface = build_surface(scene, *src.surface, s);
        loose_shaders.insert(surface);
      }
      else if (auto found = shaders.find(key); found != shaders.end()) {
        surface = found->second;
      }
      else {
        auto old = kept.shaders.find(key);
        surface = old != kept.shaders.end() ? old->second : build_surface(scene, *src.surface, s);
        shaders[key] = surface;
      }
    }
    else if (src.has_material != 0) {
      surface = build_material(src);
      loose_shaders.insert(surface);
    }
    const Transform tfm = mesh_transform(src);
    const bool displaced = src.surface != nullptr && src.surface->displacement >= 0;
    Mesh *mesh = nullptr;
    const std::pair<uint64_t, Shader *> mesh_key(src.geometry_key, surface);
    if (!displaced && src.geometry_key != 0) {
      if (auto found = kept_meshes.find(mesh_key); found != kept_meshes.end()) {
        mesh = found->second;
      }
      else if (auto old = kept.meshes.find(mesh_key); old != kept.meshes.end()) {
        mesh = old->second;
        kept_meshes[mesh_key] = mesh;
      }
      else {
        /* The same points with another surface (a material change): keep
         * the points, swap the surface, if that mesh isn't claimed yet. */
        for (auto it = kept.meshes.lower_bound({src.geometry_key, nullptr});
             it != kept.meshes.end() && it->first.first == src.geometry_key;
             ++it)
        {
          Mesh *candidate = it->second;
          const bool claimed = std::any_of(kept_meshes.begin(),
                                           kept_meshes.end(),
                                           [&](const auto &e) { return e.second == candidate; });
          if (claimed || kept.meshes.count(mesh_key) != 0) {
            continue;
          }
          array<Node *> used_shaders;
          used_shaders.push_back_slow(surface);
          candidate->set_used_shaders(used_shaders);
          mesh = candidate;
          kept_meshes[mesh_key] = mesh;
          break;
        }
      }
    }
    if (mesh == nullptr) {
      mesh = scene->create_node<Mesh>();
      array<Node *> used_shaders;
      used_shaders.push_back_slow(surface);
      mesh->set_used_shaders(used_shaders);
      if (displaced) {
        /* Diced as the camera sees it, so never reused. */
        build_displaced(mesh, src, s, tfm);
      }
      else {
        fill_mesh(mesh, src);
      }
      if (!displaced && src.geometry_key != 0) {
        kept_meshes[mesh_key] = mesh;
      }
      else {
        loose_geometry.insert(mesh);
      }
    }
    Object *object = nullptr;
    if (slot < kept.objects.size()) {
      object = kept.objects[slot];
    }
    else {
      object = scene->create_node<Object>();
      kept.objects.push_back(object);
    }
    slot++;
    object->set_geometry(mesh);
    object->set_tfm(tfm);
    set_visibility(object, src, s.direct_caustics != 0, s.lamps_only != 0);
    /* Cryptomatte names and the Segment ID colour. Cycles rehashes the
     * object name only when the asset name changes, so a kept object
     * that takes another mesh's name is tagged. */
    const ustring name(src.name != nullptr ? src.name : "");
    if (object->name != name) {
      object->name = name;
      object->tag_asset_name_modified();
    }
    object->set_asset_name(ustring(src.asset_name != nullptr ? src.asset_name : ""));
    object->set_color(f3(src.segment_color));
    if (surface != scene->default_surface && src.material_name != nullptr) {
      surface->name = ustring(src.material_name);
    }
  }
  /* Drop what the new scene doesn't use: spare objects first, then the
   * geometry and surfaces nothing points at any more. */
  if (slot < kept.objects.size()) {
    set<Object *> spare(kept.objects.begin() + slot, kept.objects.end());
    scene->delete_nodes(spare);
    kept.objects.resize(slot);
  }
  set<Geometry *> unused_geometry = kept.loose_geometry;
  set<Mesh *> still_used;
  for (const auto &[key, mesh] : kept_meshes) {
    still_used.insert(mesh);
  }
  for (const auto &[key, mesh] : kept.meshes) {
    if (still_used.count(mesh) == 0) {
      unused_geometry.insert(mesh);
    }
  }
  scene->delete_nodes(unused_geometry);
  set<Shader *> unused_shaders = kept.loose_shaders;
  for (const auto &[key, shader] : kept.shaders) {
    if (shaders.find(key) == shaders.end()) {
      unused_shaders.insert(shader);
    }
  }
  scene->delete_nodes(unused_shaders);
  kept.shaders = std::move(shaders);
  kept.meshes = std::move(kept_meshes);
  kept.loose_shaders = std::move(loose_shaders);
  kept.loose_geometry = std::move(loose_geometry);
}

void sync_scene(Kept &kept, Scene *scene, const OcMesh *meshes, uint32_t mesh_count, const OcSettings &s)
{
  sync_camera(scene, s);
  sync_default_surface(kept, scene, s);
  sync_world(kept, scene, s);
  sync_lights(kept, scene, s);
  sync_meshes(kept, scene, meshes, mesh_count, s);
}

/* The GPU when asked for and present, otherwise the CPU. */
DeviceInfo pick_device(const OcSettings &s)
{
  if (s.use_gpu) {
    const vector<DeviceInfo> gpus = Device::available_devices(DEVICE_MASK_METAL | DEVICE_MASK_CUDA |
                                                              DEVICE_MASK_OPTIX | DEVICE_MASK_HIP |
                                                              DEVICE_MASK_ONEAPI);
    if (!gpus.empty()) {
      return gpus.front();
    }
  }
  const vector<DeviceInfo> cpus = Device::available_devices(DEVICE_MASK_CPU);
  return cpus.empty() ? DeviceInfo() : cpus.front();
}

}  // namespace

/* A kept renderer (operandi_cycles.h). */
struct OcSession {
  unique_ptr<Session> session;
  SessionParams params;
  /* Owned by `session`. */
  BufferOutputDriver *driver = nullptr;
  Kept kept;
};

namespace {

int render(OcSession &state,
           const OcMesh *meshes,
           uint32_t mesh_count,
           const OcSettings &s,
           float *rgba_out,
           OcControl *control,
           OcReport *report,
           char *error,
           size_t error_len)
{
  if (s.width == 0 || s.height == 0 || s.width > 16384 || s.height > 16384) {
    set_error(error, error_len, "Image size must be between 1 and 16384 pixels");
    return 1;
  }
  /* The rendered area, in pixels from the picture's top-left corner. */
  const bool has_region = s.region[2] != 0 && s.region[3] != 0;
  const uint32_t rx = has_region ? s.region[0] : 0, ry = has_region ? s.region[1] : 0;
  const uint32_t rw = has_region ? s.region[2] : s.width, rh = has_region ? s.region[3] : s.height;
  if (rx >= s.width || ry >= s.height || rw > s.width - rx || rh > s.height - ry) {
    set_error(error, error_len, "The render region must lie inside the picture");
    return 1;
  }

  /* GPU kernels are compiled from Cycles' sources on first use (then
   * cached), so Cycles must know where they are. */
  if (s.resource_path != nullptr && s.resource_path[0] != '\0') {
    path_init(s.resource_path);
  }

  const DeviceInfo device = pick_device(s);
  if (device.type == DEVICE_NONE) {
    set_error(error, error_len, "Cycles found no render device");
    return 1;
  }
  const bool on_gpu = device.type != DEVICE_CPU;
  const bool denoise = s.denoise && device.denoisers != 0;

  SessionParams session_params;
  session_params.device = device;
  session_params.denoise_device = device;
  session_params.background = true;
  /* Not headless, so Cycles hands over the picture while it renders. */
  session_params.headless = false;
  session_params.samples = int(s.samples == 0 ? 1 : s.samples);
  session_params.threads = int(s.threads); /* 0 = all cores */
  session_params.time_limit = s.time_limit > 0.0f ? double(s.time_limit) : 0.0;

  SceneParams scene_params;
  scene_params.shadingsystem = SHADINGSYSTEM_SVM;

  /* Keep the session (device, kernels, scene) unless what it was made
   * with changed; Cycles' own test, plus the denoising device. */
  if (state.session == nullptr || state.params.modified(session_params) ||
      !(state.params.denoise_device == session_params.denoise_device))
  {
    state.session.reset();
    state.kept = Kept();
    state.session = make_unique<Session>(session_params, scene_params);
    auto driver = make_unique<BufferOutputDriver>(
        rgba_out, int(s.width), int(rx), int(ry), int(rw), int(rh), control);
    state.driver = driver.get();
    state.session->set_output_driver(std::move(driver));
  }
  state.params = session_params;
  Session &session = *state.session;
  BufferOutputDriver *driver_ptr = state.driver;
  driver_ptr->retarget(rgba_out, int(s.width), int(rx), int(ry), int(rw), int(rh), control);
  session.progress.reset();
  /* A failed or cancelled render leaves Cycles in an unknown state: start
   * afresh next time. */
  struct DropOnFailure {
    OcSession &state;
    bool ok = false;
    ~DropOnFailure()
    {
      if (!ok) {
        state.session.reset();
        state.driver = nullptr;
        state.kept = Kept();
      }
    }
  } drop_on_failure{state};

  Scene *scene = session.scene.get();
  {
    const thread_scoped_lock scene_lock(scene->mutex);
    sync_scene(state.kept, scene, meshes, mesh_count, s);
    Integrator *integrator = scene->integrator;
    integrator->set_use_denoise(denoise);
    /* -1 keeps Cycles' default (the integrator socket's default value),
     * set again so a kept session forgets an earlier render's limit. */
    {
      auto limit = [&](int32_t wanted, const char *socket) {
        if (wanted >= 0) {
          return int(wanted);
        }
        const SocketType *type = integrator->type->find_input(ustring(socket));
        return type != nullptr ? *static_cast<const int *>(type->default_value) : 7;
      };
      integrator->set_max_diffuse_bounce(limit(s.max_diffuse_bounces, "max_diffuse_bounce"));
      integrator->set_max_glossy_bounce(limit(s.max_glossy_bounces, "max_glossy_bounce"));
      integrator->set_max_transmission_bounce(
          limit(s.max_transmission_bounces, "max_transmission_bounce"));
      integrator->set_max_bounce(limit(s.max_bounces, "max_bounce"));
      integrator->set_transparent_max_bounce(
          limit(s.max_transparent_bounces, "transparent_max_bounce"));
    }
    integrator->set_sample_clamp_indirect(s.sample_clamp_indirect > 0.0f ? s.sample_clamp_indirect
                                                                         : 0.0f);
    integrator->set_caustics_reflective(s.caustics_reflective != 0);
    integrator->set_caustics_refractive(s.caustics_refractive != 0);
    integrator->set_use_adaptive_sampling(s.adaptive_threshold > 0.0f);
    if (s.adaptive_threshold > 0.0f) {
      integrator->set_adaptive_threshold(s.adaptive_threshold);
    }
    if (denoise) {
      integrator->set_denoiser_type(DENOISER_OPENIMAGEDENOISE);
      integrator->set_denoise_use_gpu(on_gpu);
      integrator->set_denoiser_quality(DENOISER_QUALITY_HIGH);
    }
    /* Passes stay while the same ones are asked for; only where their
     * pictures go changes. */
    const std::string passes_wanted = pass_signature(s);
    const bool new_passes = state.kept.passes.empty() || passes_wanted != state.kept.pass_signature;
    if (new_passes) {
      scene->delete_nodes(state.kept.passes);
      state.kept.passes.clear();
      state.kept.pass_signature = passes_wanted;
      Pass *pass = scene->create_node<Pass>();
      pass->set_name(ustring("combined"));
      pass->set_type(PASS_COMBINED);
      state.kept.passes.insert(pass);
    }
    for (uint32_t g = 0; s.light_groups != nullptr && g < s.light_group_count; g++) {
      if (s.light_groups[g] == nullptr || s.light_group_out == nullptr ||
          s.light_group_out[g] == nullptr)
      {
        continue;
      }
      if (new_passes) {
        Pass *group = scene->create_node<Pass>();
        group->set_name(ustring(s.light_groups[g]));
        group->set_type(PASS_COMBINED);
        group->set_lightgroup(ustring(s.light_groups[g]));
        state.kept.passes.insert(group);
      }
      driver_ptr->groups.emplace_back(s.light_groups[g], s.light_group_out[g]);
    }
    /* Render Output passes, gathered like the light groups. */
    static const PassType types[OC_PASS_COUNT] = {
        PASS_DEPTH,
        PASS_NORMAL,
        PASS_DIFFUSE_DIRECT,
        PASS_DIFFUSE_INDIRECT,
        PASS_DIFFUSE_COLOR,
        PASS_GLOSSY_DIRECT,
        PASS_GLOSSY_INDIRECT,
        PASS_GLOSSY_COLOR,
        PASS_TRANSMISSION_DIRECT,
        PASS_TRANSMISSION_INDIRECT,
        PASS_TRANSMISSION_COLOR,
        PASS_EMISSION,
        PASS_AO,
        PASS_AOV_VALUE,
        PASS_AOV_COLOR,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_VALUE,
        PASS_AOV_COLOR, /* UV */
        PASS_AOV_COLOR, /* Segment */
        PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE,
        PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE,
        PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE,
        PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE,
        PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE,
        PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE, PASS_CRYPTOMATTE,
        PASS_SAMPLE_COUNT,
        PASS_VOLUME_DIRECT,
        PASS_VOLUME_INDIRECT,
        PASS_BACKGROUND,
    };
    /* Cycles writes every enabled cryptomatte type at one depth, object
     * first, then material, then asset, into consecutive passes; the
     * passes come in that order (OC_PASS_* order, sorted by the caller). */
    const auto [crypto_types, crypto_depth] = cryptomatte_wanted(s);
    scene->film->set_cryptomatte_passes(CryptomatteType(crypto_types));
    scene->film->set_cryptomatte_depth(crypto_depth);
    for (uint32_t k = 0; s.passes != nullptr && k < s.pass_count; k++) {
      const uint32_t which = s.passes[k];
      if (which >= OC_PASS_COUNT || s.pass_out == nullptr || s.pass_out[k] == nullptr) {
        continue;
      }
      const std::string name = which == OC_PASS_INCIDENCE ? "operandi_incidence" :
                               which == OC_PASS_OBJECT_COORDINATES ? "operandi_object" :
                               which == OC_PASS_UV ? "operandi_uv" :
                               which == OC_PASS_SEGMENT ? "operandi_segment" :
                               which >= OC_PASS_SAMPLE_COUNT ?
                                   "operandi_pass_" + std::to_string(which) :
                               which >= OC_PASS_CRYPTO_OBJECT_0 ? cryptomatte_pass_name(which) :
                               which >= OC_PASS_GROUP_0 ?
                                   "operandi_group_" + std::to_string(which - OC_PASS_GROUP_0) :
                                   "operandi_pass_" + std::to_string(which);
      if (new_passes) {
        Pass *p = scene->create_node<Pass>();
        p->set_name(ustring(name));
        p->set_type(types[which]);
        state.kept.passes.insert(p);
      }
      driver_ptr->groups.emplace_back(name, s.pass_out[k]);
    }
    /* Material values: one colour AOV each, read back like the passes. */
    for (uint32_t k = 0; s.value_passes != nullptr && k < s.value_count; k++) {
      const uint32_t which = s.value_passes[k];
      if (which >= OC_VALUE_COUNT || s.value_out == nullptr || s.value_out[k] == nullptr) {
        continue;
      }
      if (new_passes) {
        Pass *p = scene->create_node<Pass>();
        p->set_name(ustring(OC_VALUE_NAMES[which]));
        p->set_type(PASS_AOV_COLOR);
        state.kept.passes.insert(p);
      }
      driver_ptr->groups.emplace_back(OC_VALUE_NAMES[which], s.value_out[k]);
    }
    integrator->set_ao_distance(wants_pass(s, OC_PASS_AO) && s.ao_distance > 0.0f ? s.ao_distance :
                                                                                   1e10f);
  }

  /* A region is a smaller buffer placed inside the full picture; Cycles
   * counts full_y from the bottom. */
  BufferParams buffer_params;
  buffer_params.width = int(rw);
  buffer_params.height = int(rh);
  buffer_params.window_width = int(rw);
  buffer_params.window_height = int(rh);
  buffer_params.full_x = int(rx);
  buffer_params.full_y = int(s.height - ry - rh);
  buffer_params.full_width = int(s.width);
  buffer_params.full_height = int(s.height);

  /* Steering. Cycles' offline renders have no pause of their own, so a
   * pause holds the render thread in the progress callback, which Cycles
   * calls between sample batches (and while it prepares the scene). Finish
   * lowers the sample count to what is done, so Cycles wraps up normally:
   * last pass, denoise, picture. Cancel stops without a picture. */
  const uint32_t total_samples = uint32_t(session_params.samples);
  const std::thread::id caller = std::this_thread::get_id();
  bool finishing = false;
  std::atomic<bool> cancelled{false};
  /* Called from any thread (Cycles polls it while it works). */
  auto cancel_if_asked = [&]() {
    if (load_command(control) == OC_CANCEL && !cancelled.exchange(true)) {
      session.progress.set_cancel("Cancelled");
    }
  };
  /* The callbacks borrow these locals: drop them before the locals go. */
  struct DropCallbacks {
    Progress &progress;
    ~DropCallbacks()
    {
      progress.set_update_callback(nullptr);
      progress.set_cancel_callback(nullptr);
    }
  } drop_callbacks{session.progress};
  if (control != nullptr) {
    session.progress.set_cancel_callback(cancel_if_asked);
    session.progress.set_update_callback([&]() {
      if (std::this_thread::get_id() == caller) {
        return;
      }
      auto report_progress = [&](uint32_t paused) {
        if (control->on_progress != nullptr) {
          const int done = session.progress.get_current_sample();
          control->on_progress(control->user, uint32_t(done > 0 ? done : 0), total_samples, paused);
        }
      };
      report_progress(0);
      if (load_command(control) == OC_PAUSE) {
        report_progress(1);
        while (load_command(control) == OC_PAUSE) {
          std::this_thread::sleep_for(std::chrono::milliseconds(30));
        }
        report_progress(0);
      }
      if (load_command(control) == OC_FINISH && !finishing) {
        finishing = true;
        const int done = session.progress.get_current_sample();
        session.set_samples(done > 1 ? done : 1);
      }
      cancel_if_asked();
    });
  }

  session.reset(session_params, buffer_params);
  session.start();
  session.wait();

  if (cancelled) {
    /* A Preview restart cancels the render it replaces. Keep the session
     * when Cycles had finished taking in the scene (the cancel only cut
     * the sampling short); otherwise its state is unknown, so drop it. */
    {
      const thread_scoped_lock scene_lock(scene->mutex);
      drop_on_failure.ok = !session.progress.get_error() && !scene->need_reset(false);
    }
    set_error(error, error_len, "Cancelled");
    return 1;
  }

  if (session.progress.get_error()) {
    set_error(error, error_len, "Cycles: " + session.progress.get_error_message());
    return 1;
  }
  if (!driver_ptr->written) {
    set_error(error, error_len, "Cycles finished without producing an image");
    return 1;
  }
  drop_on_failure.ok = true;
  /* Cycles' manifests of the names it hashed (RO-Q16). */
  if (s.on_cryptomatte_manifest != nullptr) {
    const int crypto_types = cryptomatte_wanted(s).first;
    const thread_scoped_lock scene_lock(scene->mutex);
    if (crypto_types & CRYPT_OBJECT) {
      s.on_cryptomatte_manifest(
          s.manifest_user, 0, scene->object_manager->get_cryptomatte_objects(scene).c_str());
    }
    if (crypto_types & CRYPT_MATERIAL) {
      s.on_cryptomatte_manifest(
          s.manifest_user, 1, scene->shader_manager->get_cryptomatte_materials(scene).c_str());
    }
    if (crypto_types & CRYPT_ASSET) {
      s.on_cryptomatte_manifest(
          s.manifest_user, 2, scene->object_manager->get_cryptomatte_assets(scene).c_str());
    }
  }
  if (report != nullptr) {
    std::snprintf(report->device, sizeof(report->device), "%s", device.description.c_str());
    report->used_gpu = on_gpu;
    report->denoised = denoise;
    const int done = session.progress.get_current_sample();
    report->samples = uint32_t(done > 0 ? done : 0);
  }
  return 0;
}

}  // namespace

extern "C" OcSession *oc_session_new(void)
{
  try {
    return new OcSession();
  }
  catch (...) {
    return nullptr;
  }
}

extern "C" void oc_session_free(OcSession *session)
{
  try {
    delete session;
  }
  catch (...) {
  }
}

extern "C" int oc_session_render(OcSession *session,
                                 const OcMesh *meshes,
                                 uint32_t mesh_count,
                                 const OcSettings *settings,
                                 float *rgba_out,
                                 OcControl *control,
                                 OcReport *report,
                                 char *error,
                                 size_t error_len)
{
  if (session == nullptr || settings == nullptr || rgba_out == nullptr ||
      (meshes == nullptr && mesh_count != 0))
  {
    set_error(error, error_len, "Missing render input");
    return 1;
  }
  /* Exceptions must not cross into Rust. */
  try {
    return render(*session, meshes, mesh_count, *settings, rgba_out, control, report, error, error_len);
  }
  catch (const std::exception &e) {
    set_error(error, error_len, std::string("Cycles failed: ") + e.what());
  }
  catch (...) {
    set_error(error, error_len, "Cycles failed with an unknown error");
  }
  session->session.reset();
  session->driver = nullptr;
  session->kept = Kept();
  return 1;
}

/* One render with a session of its own, freed straight after. */
extern "C" int oc_render(const OcMesh *meshes,
                         uint32_t mesh_count,
                         const OcSettings *settings,
                         float *rgba_out,
                         OcControl *control,
                         OcReport *report,
                         char *error,
                         size_t error_len)
{
  OcSession session;
  return oc_session_render(
      &session, meshes, mesh_count, settings, rgba_out, control, report, error, error_len);
}

extern "C" const char *oc_cycles_version(void)
{
  return CYCLES_VERSION_STRING;
}

/* Structs without pointers have the same size everywhere; pin them so an
 * edit that changes one fails here first, as a reminder to bump
 * OC_ABI_VERSION. (Rust compares every struct's size at run time.) */
static_assert(sizeof(OcExpr) == 84, "OcExpr changed: bump OC_ABI_VERSION");
/* OcLight holds pointers since light properties Tier 2, so its size is
 * pinned for 64-bit builds (every supported target); Rust also checks every
 * struct's size at run time. */
static_assert(sizeof(void *) != 8 || sizeof(OcLight) == 192,
              "OcLight changed: bump OC_ABI_VERSION");
static_assert(sizeof(void *) != 8 || sizeof(OcEnvironment) == 176,
              "OcEnvironment changed: bump OC_ABI_VERSION");
static_assert(sizeof(void *) != 8 || sizeof(OcEnvLayer) == 80,
              "OcEnvLayer changed: bump OC_ABI_VERSION");
static_assert(sizeof(void *) != 8 || sizeof(OcMesh) == 208, "OcMesh changed: bump OC_ABI_VERSION");
static_assert(sizeof(OcReport) == 140, "OcReport changed: bump OC_ABI_VERSION");
static_assert(OC_INPUT_COUNT == 27, "Principled inputs changed: bump OC_ABI_VERSION");

extern "C" uint32_t oc_abi_version(void)
{
  return OC_ABI_VERSION;
}

extern "C" size_t oc_struct_size(uint32_t which)
{
  switch (which) {
    case OC_STRUCT_EXPR:
      return sizeof(OcExpr);
    case OC_STRUCT_SURFACE:
      return sizeof(OcSurface);
    case OC_STRUCT_MESH:
      return sizeof(OcMesh);
    case OC_STRUCT_LIGHT:
      return sizeof(OcLight);
    case OC_STRUCT_SETTINGS:
      return sizeof(OcSettings);
    case OC_STRUCT_REPORT:
      return sizeof(OcReport);
    case OC_STRUCT_CONTROL:
      return sizeof(OcControl);
    case OC_STRUCT_ENVIRONMENT:
      return sizeof(OcEnvironment);
    case OC_STRUCT_ENV_LAYER:
      return sizeof(OcEnvLayer);
    case OC_STRUCT_NODE:
      return sizeof(OcNode);
    case OC_STRUCT_NODE_VALUE:
      return sizeof(OcNodeValue);
    case OC_STRUCT_NODE_LINK:
      return sizeof(OcNodeLink);
    default:
      return 0;
  }
}
