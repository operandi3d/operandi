/* Operandi's C interface to OpenUSD.
 *
 * This is the only surface Rust sees. Everything C++ (OpenUSD, TBB...)
 * stays behind it, so the Rust side links one library and never needs a
 * C++ toolchain of its own.
 *
 * All arrays are borrowed for the duration of the call only. Functions that
 * can fail return 0 on failure and write a message into `error` (up to
 * `error_len` bytes, always NUL-terminated); they never throw. */
#ifndef OPERANDI_USD_H
#define OPERANDI_USD_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#  define OU_API __declspec(dllexport)
#else
#  define OU_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* Version of this interface. Bump it whenever any shared struct, enum value
 * or function signature below changes, and bump OU_ABI_VERSION in
 * src/ffi.rs to match. The Rust side compares it, and every struct's size,
 * with the library it loaded before the first call. */
#define OU_ABI_VERSION 11

/* Rotation orders, named like USD's rotate ops: OU_ROTATE_ZYX turns about
 * Z first, then Y, then X. */
enum {
  OU_ROTATE_XYZ = 0,
  OU_ROTATE_XZY = 1,
  OU_ROTATE_YXZ = 2,
  OU_ROTATE_YZX = 3,
  OU_ROTATE_ZXY = 4,
  OU_ROTATE_ZYX = 5,
};

/* OuMesh.subdivision */
enum {
  OU_SUBDIVISION_NONE = 0,
  OU_SUBDIVISION_CATMULL_CLARK = 1,
};

/* OuMesh.boundary: USD's interpolateBoundary. */
enum {
  OU_BOUNDARY_EDGE_AND_CORNER = 0,
  OU_BOUNDARY_EDGE_ONLY = 1,
};

/* OuMesh.uv_interpolation: USD's faceVaryingLinearInterpolation. */
enum {
  OU_FACE_VARYING_CORNERS_PLUS1 = 0,
  OU_FACE_VARYING_NONE = 1,
  OU_FACE_VARYING_BOUNDARIES = 2,
  OU_FACE_VARYING_ALL = 3,
};

/* OuSurfaceInput.type */
enum {
  OU_INPUT_FLOAT = 0,
  OU_INPUT_COLOR = 1,
  OU_INPUT_NORMAL = 2,
};

/* OuTexture.channel: the UsdUVTexture output that feeds the input. */
enum {
  OU_CHANNEL_RGB = 0,
  OU_CHANNEL_R = 1,
  OU_CHANNEL_G = 2,
  OU_CHANNEL_B = 3,
  OU_CHANNEL_A = 4,
};

/* OuTexture.wrap: UsdUVTexture wrapS and wrapT. */
enum {
  OU_WRAP_REPEAT = 0,
  OU_WRAP_MIRROR = 1,
  OU_WRAP_CLAMP = 2,
  OU_WRAP_BLACK = 3,
};

/* ou_struct_size `which` values. */
enum {
  OU_STRUCT_XFORM = 0,
  OU_STRUCT_UV_SET = 1,
  OU_STRUCT_SUBSET = 2,
  OU_STRUCT_MESH = 3,
  OU_STRUCT_TEXTURE = 4,
  OU_STRUCT_SURFACE_INPUT = 5,
  OU_STRUCT_MATERIAL = 6,
  OU_STRUCT_CAMERA = 7,
  OU_STRUCT_LIGHT = 8,
};

/* OuLight.type: the UsdLux schema. */
enum {
  OU_LIGHT_DISTANT = 0,
  OU_LIGHT_SPHERE = 1,
  OU_LIGHT_RECT = 2,
  OU_LIGHT_DISK = 3,
  OU_LIGHT_CYLINDER = 4,
  OU_LIGHT_PORTAL = 5,
  OU_LIGHT_DOME = 6,
};

/* An item's transform prim. */
typedef struct OuXform {
  /* Item name; the bridge makes it a valid, unique prim name. */
  const char *name;
  double translate[3];
  /* Degrees, applied in `rotate_order`. */
  double rotate[3];
  uint32_t rotate_order;
  double scale[3];
  /* Non-zero: one matrix op from `matrix` instead of translate/rotate/scale. */
  uint32_t use_matrix;
  /* Row-major, USD's row-vector convention (translation in the last row). */
  double matrix[16];
  /* Zero writes visibility "invisible", which hides everything below too. */
  uint32_t visible;
} OuXform;

/* A UV map: one (u, v) pair per face corner, in face order. */
typedef struct OuUvSet {
  const char *name;
  const float *corner_uvs;
  size_t corner_count;
} OuUvSet;

/* The faces that carry one material tag. */
typedef struct OuSubset {
  const char *name;
  const int32_t *faces;
  size_t face_count;
  /* Path of the Material bound to these faces, or NULL. */
  const char *material;
} OuSubset;

typedef struct OuMesh {
  const char *name;
  /* xyz per point. */
  const float *points;
  size_t point_count;
  const int32_t *face_counts;
  size_t face_count;
  const int32_t *face_indices;
  size_t index_count;
  /* xyz per face corner (`index_count` of them), or NULL. */
  const float *normals;
  uint32_t subdivision;
  uint32_t boundary;
  uint32_t uv_interpolation;
  /* Point pairs, one per creased edge, and their sharpness. */
  const int32_t *crease_points;
  const float *crease_sharpness;
  size_t crease_count;
  const OuUvSet *uv_sets;
  size_t uv_set_count;
  const OuSubset *subsets;
  size_t subset_count;
  uint32_t double_sided;
  /* Zero writes visibility "invisible" on this mesh only. */
  uint32_t visible;
  /* Path of the Material bound to the whole mesh, or NULL. */
  const char *material;
} OuMesh;

/* An image feeding a UsdPreviewSurface input (a UsdUVTexture reading a
 * UV primvar, through a UsdTransform2d when it repeats or turns). */
typedef struct OuTexture {
  /* Asset path as written in the file (relative to it, or absolute). */
  const char *file;
  /* The mesh primvar holding the UVs ("st"...). */
  const char *uv;
  uint32_t channel;
  /* out = texel x scale + bias, per component (r, g, b, a). */
  float scale[4];
  float bias[4];
  uint32_t wrap;
  /* Non-zero: sourceColorSpace "raw" (data), else "sRGB". */
  uint32_t raw;
  /* Non-zero: a UsdTransform2d with this scale and rotation (degrees). */
  uint32_t has_transform;
  float uv_scale[2];
  float uv_rotation;
} OuTexture;

typedef struct OuSurfaceInput {
  /* The UsdPreviewSurface input: "diffuseColor", "roughness"... */
  const char *name;
  uint32_t type;
  /* The value (a float input reads value[0]); used when `texture` is NULL. */
  float value[3];
  const OuTexture *texture;
} OuSurfaceInput;

typedef struct OuMaterial {
  const char *name;
  const OuSurfaceInput *inputs;
  size_t input_count;
} OuMaterial;

/* A camera. Lengths in tenths of a scene unit (USD's camera units). */
typedef struct OuCamera {
  const char *name;
  uint32_t orthographic;
  float focal_length;
  float horizontal_aperture;
  float vertical_aperture;
  float horizontal_offset;
  float vertical_offset;
  /* Near and far, scene units. */
  float clipping[2];
  /* 0: no depth of field. */
  float f_stop;
  float focus_distance;
} OuCamera;

/* A light, shining down its local -Z. */
typedef struct OuLight {
  const char *name;
  uint32_t type;
  float intensity;
  float color[3];
  float diffuse;
  float specular;
  uint32_t normalize;
  /* Sphere, disk and cylinder radius; rect and portal width/height;
   * cylinder length (along local X). */
  float radius;
  float width;
  float height;
  float length;
  /* Distant: angular size, degrees. */
  float angle;
  /* Sphere: a ShapingAPI cone when > 0 (half-angle, degrees). */
  float cone_angle;
  float cone_softness;
  /* Sphere: an IES profile file, or NULL. */
  const char *ies_file;
  /* Dome: a lat-long picture file, or NULL. */
  const char *texture_file;
} OuLight;

typedef struct OuStage OuStage;

OU_API uint32_t ou_abi_version(void);
OU_API size_t ou_struct_size(uint32_t which);
/* "0.26.8": the OpenUSD version the bridge was built with. */
OU_API const char *ou_usd_version(void);

/* Registers OpenUSD's plugins (file formats and schemas) from `plugin_dir`
 * (the folder holding plugInfo.json), or from next to the USD library when
 * NULL. Safe to call more than once. */
OU_API int ou_init(const char *plugin_dir, char *error, size_t error_len);

/* A new, empty stage that ou_stage_save writes to `path`. The extension
 * picks the format (.usda text, .usdc binary); a .usd file is binary unless
 * `text` is non-zero. */
OU_API OuStage *ou_stage_create(const char *path, uint32_t text, char *error, size_t error_len);
/* Opens a USD file for reading. */
OU_API OuStage *ou_stage_open(const char *path, char *error, size_t error_len);
OU_API void ou_stage_free(OuStage *stage);

/* Stage metadata: up axis ('Y' or 'Z'), metres per unit and the default
 * prim (a prim path, or NULL for none). */
OU_API int ou_stage_set_metadata(OuStage *stage,
                                 char up_axis,
                                 double meters_per_unit,
                                 const char *default_prim,
                                 char *error,
                                 size_t error_len);

/* Defines an Xform under `parent` (a prim path; "/" for the stage root) and
 * writes its path into `path_out`. */
/* Animation (TL-80): framesPerSecond and timeCodesPerSecond = fps, and the
 * start and end time codes (frames). */
OU_API int ou_stage_set_time(OuStage *stage, double fps, double start, double end, char *error, size_t error_len);
/* Animation (TL-81/82): time samples on the Xform at `path`, as JSON:
 * {"times": [t...], "order": rotate order, "translate": [[x,y,z]...] or null,
 *  "rotate": [...] (degrees) or null, "scale": [...] or null,
 *  "attributes": [{"path": prim, "name": attr, "type": "float" | "color3f",
 *                  "values": [...]}]}.
 * An Xform written with no ops (the identity) gets translate, rotate and
 * scale ops first. */
OU_API int ou_set_samples(OuStage *stage, const char *path, const char *json, char *error, size_t error_len);
OU_API int ou_define_xform(OuStage *stage,
                           const char *parent,
                           const OuXform *xform,
                           char *path_out,
                           size_t path_len,
                           char *error,
                           size_t error_len);

/* Defines a Mesh under `parent` and writes its path into `path_out`. */
OU_API int ou_define_mesh(OuStage *stage,
                          const char *parent,
                          const OuMesh *mesh,
                          char *path_out,
                          size_t path_len,
                          char *error,
                          size_t error_len);

/* Defines a Material with a UsdPreviewSurface under `parent` (a Scope is
 * made there if the path's parent exists but it doesn't) and writes its
 * path into `path_out`. */
OU_API int ou_define_material(OuStage *stage,
                              const char *parent,
                              const OuMaterial *material,
                              char *path_out,
                              size_t path_len,
                              char *error,
                              size_t error_len);

/* Adds a MaterialX network to the Material at `material` (a prim path):
 * Shader prims under it, wired to the Material's outputs:mtlx:surface.
 * `json` is {"nodes": [{"name", "id" (the nodedef, info:id), "type" (its
 * output's MaterialX type), "inputs": [{"name", "type", "value" (MaterialX
 * text, or an asset path for a filename) or "connect" (a node name),
 * "colorspace"?}]}], "terminal": the surface shader's node name}. */
OU_API int ou_define_network(OuStage *stage,
                             const char *material,
                             const char *json,
                             char *error,
                             size_t error_len);

/* Writes the render settings (ABI 9): a UsdRender RenderSettings prim
 * `json`'s "name" in Scope "scope" (default "/Render"), with "resolution"
 * [w, h], "pixelAspect", "camera" (a prim path) and "custom" as customData
 * "operandi"; "product" {"name", "productName", "resolution"} adds a
 * RenderProduct in "<scope>/Products" that the settings name. The stage's
 * renderSettingsPrimPath names the settings. */
OU_API int ou_define_render_settings(OuStage *stage, const char *json, char *error, size_t error_len);

/* Writes skeletons and skinning (ABI 11, BEHAVIOR-RIG-PHASE5 §7): the prim
 * "skelRoot" becomes a SkelRoot; each of "skeletons" [{"path", "joints":
 * [joint paths], "bind": [[16]...] (world, row-major, USD's row-vector
 * convention), "rest": [[16]...] (local), "anim"?: {"path", "times": [...],
 * "translations": [[[x,y,z] per joint] per time], "rotations": [[[w,x,y,z]
 * ...]...], "scales": [[[x,y,z]...]...]}}] a UsdSkelSkeleton (with its
 * SkelAnimation as animationSource); each of "bindings" [{"mesh",
 * "skeleton", "elementSize", "indices": [...], "weights": [...],
 * "geomBind": [16], "method": "classicLinear" | "dualQuaternion"}] binds a
 * Mesh with UsdSkelBindingAPI. */
OU_API int ou_define_skel(OuStage *stage, const char *json, char *error, size_t error_len);

/* Defines a Camera / a light under `parent` and writes its path into
 * `path_out`. */
OU_API int ou_define_camera(OuStage *stage,
                            const char *parent,
                            const OuCamera *camera,
                            char *path_out,
                            size_t path_len,
                            char *error,
                            size_t error_len);
OU_API int ou_define_light(OuStage *stage,
                           const char *parent,
                           const OuLight *light,
                           char *path_out,
                           size_t path_len,
                           char *error,
                           size_t error_len);

/* Adds what a mesh carries beyond its shape (ABI 6) to the Mesh at `mesh`
 * (a prim path): extra primvars, geometry subsets, blend shapes and
 * customData. `json` describes them and names `buffers` (with their
 * `sizes` in bytes) by index:
 *   {"primvars": [{"name", "type": "float" | "float2" | "float3" | "float4" |
 *                  "color3" | "color4", "elementSize"?: n (a "float" with n
 *                  numbers per element), "interpolation": "vertex" |
 *                  "faceVarying", "values": buffer (f32), "indices"?: buffer
 *                  (i32), "unauthored"?: index (UsdGeomPrimvar's
 *                  unauthoredValuesIndex), "custom"?: {...}}],
 *    "subsets": [{"name", "elementType": "face" | "point", "family",
 *                 "familyType", "indices": buffer (i32), "custom"?: {...}}],
 *    "blendShapes": [{"name", "offsets": buffer (f32, xyz per point),
 *                     "pointIndices"?: buffer (i32), "custom"?: {...}}],
 *    "light"?: {"intensity", "color": [r, g, b], "diffuse", "specular"}
 *              (ABI 9: UsdLux MeshLightAPI, not normalized, emission only),
 *    "custom"?: {...}}
 * Each "custom" object becomes customData under the key "operandi":
 * strings, booleans, numbers, integer lists and nested objects. */
OU_API int ou_define_mesh_extras(OuStage *stage,
                                 const char *mesh,
                                 const char *json,
                                 const void *const *buffers,
                                 const size_t *sizes,
                                 size_t buffer_count,
                                 char *error,
                                 size_t error_len);

/* Writes a BasisCurves or NurbsCurves prim (ABI 7) named `json`'s "name"
 * (made unique) under `parent`, and puts its path in `path_out`. `json`:
 *   {"name", "schema": "BasisCurves" | "NurbsCurves",
 *    "type"?: "linear" | "cubic", "basis"?: "bezier" | "bspline" |
 *    "catmullRom", "wrap"?: "nonperiodic" | "periodic" | "pinned",
 *    "counts": buffer (i32, points per curve), "points": buffer (f32, xyz),
 *    "widths"?: buffer (f32, one per point), "order"?: buffer (i32, one per
 *    curve), "knots"?: buffer (f64), "ranges"?: buffer (f64, two per
 *    curve), "visible": bool, "material"?: a Material's prim path,
 *    "custom"?: {...}}
 * "type", "basis" and "wrap" are BasisCurves', "order", "knots" and
 * "ranges" NurbsCurves'. "custom" becomes customData "operandi". */
OU_API int ou_define_curves(OuStage *stage,
                            const char *parent,
                            const char *json,
                            const void *const *buffers,
                            const size_t *sizes,
                            size_t buffer_count,
                            char *path_out,
                            size_t path_len,
                            char *error,
                            size_t error_len);

/* Writes a PointInstancer (ABI 8) named `json`'s "name" (made unique)
 * under `parent`, and puts its path in `path_out`. Each prototype is an
 * instanceable Xform in the instancer's "Prototypes" Scope referencing a
 * prim of the same stage. `json`:
 *   {"name", "prototypes": [prim paths], "protoIndices": buffer (i32),
 *    "positions": buffer (f32, xyz), "orientations": buffer (f32, xyzw,
 *    written both as orientations and orientationsf), "scales": buffer
 *    (f32, xyz), "invisibleIds"?: buffer (i64), "custom"?: {...}}
 * "ids" are written as 0, 1, 2... "custom" becomes customData
 * "operandi". */
OU_API int ou_define_point_instancer(OuStage *stage,
                                     const char *parent,
                                     const char *json,
                                     const void *const *buffers,
                                     const size_t *sizes,
                                     size_t buffer_count,
                                     char *path_out,
                                     size_t path_len,
                                     char *error,
                                     size_t error_len);

/* Makes the prim at `path` a class: abstract, so it isn't drawn and only
 * what references it shows it (ABI 6). */
OU_API int ou_make_class(OuStage *stage, const char *path, char *error, size_t error_len);

/* Makes the prim at `path` a native instance of `prototype` (a prim path
 * in the same stage): an internal reference to it, instanceable, and
 * customData operandi:instanceHolder = true (ABI 6). */
OU_API int ou_make_instance(OuStage *stage,
                            const char *path,
                            const char *prototype,
                            char *error,
                            size_t error_len);

OU_API int ou_stage_save(OuStage *stage, char *error, size_t error_len);

/* Packs the saved USD file `source` and every file it references into a
 * new .usdz archive at `usdz_path`. */
OU_API int ou_package_usdz(const char *source,
                           const char *usdz_path,
                           char *error,
                           size_t error_len);

/* The flattened stage as .usda text. Free it with ou_string_free. */
OU_API char *ou_stage_text(OuStage *stage);
OU_API void ou_string_free(char *text);

/* The local-to-world matrix of the prim at `path`, row-major in USD's
 * row-vector convention. */
OU_API int ou_world_matrix(OuStage *stage, const char *path, double out[16]);

/* Reading (import).
 *
 * ou_read_open walks a whole stage once (instances expanded, prototypes
 * read once) and keeps a JSON description and the binary buffers it
 * names; both stay valid until ou_read_free. A buffer reference in the
 * JSON is {"buffer": n, "type": "i32" | "f32" | "f32x3" | "f64",
 * "count": elements}. src/read.rs documents the JSON's fields. */
typedef struct OuReader OuReader;
OU_API OuReader *ou_read_open(const char *path, char *error, size_t error_len);
OU_API void ou_read_free(OuReader *reader);
/* NUL-terminated UTF-8 JSON, owned by the reader. */
OU_API const char *ou_read_json(const OuReader *reader);
/* Points `data` at buffer `index` and returns its size in bytes (0 and
 * no change for an index out of range). */
OU_API size_t ou_read_buffer(const OuReader *reader, size_t index, const void **data);
/* Writes the asset at OpenUSD resolved path `resolved` (a file, or a file
 * inside a .usdz: "pack.usdz[inner/name.png]") to the file `destination`. */
OU_API int ou_copy_asset(const char *resolved,
                         const char *destination,
                         char *error,
                         size_t error_len);

#ifdef __cplusplus
}
#endif

#endif
