/* Operandi's C interface to the Cycles renderer.
 *
 * This is the only surface Rust sees. Everything C++ (Cycles, OpenImageIO,
 * TBB, Embree...) stays behind it, so the Rust side links one library and
 * never needs a C++ toolchain of its own.
 *
 * All arrays are borrowed for the duration of the call only. */
#ifndef OPERANDI_CYCLES_H
#define OPERANDI_CYCLES_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#  define OC_API __declspec(dllexport)
#else
#  define OC_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* Version of this interface. Bump it whenever any shared struct, enum value
 * or function signature below changes, and bump OC_ABI_VERSION in
 * src/lib.rs to match. Before the first render the app compares it, and
 * every struct's size, with the library it loaded, so a library left over
 * from an older build-cycles.sh run is refused instead of misread. */
/* v18 (Nodes N1, NM-30): OcNode, OcNodeValue, OcNodeLink and the node
 * graph fields at the end of OcSurface. Surfaces without a graph
 * (node_count 0) render exactly as in v17. */
/* v19 (OpenPBR, OPM-03): eight more Principled inputs (coat tint and IOR,
 * sheen tint and roughness, thin film, dispersion) and, at the end of
 * OcSurface, the interior scatter, Thin Wall and the principled glass.
 * Surfaces that leave them unset render exactly as in v18. */
/* v20 (env-texture-locator, ETL-Q02): OcEnvLayer kind 2, a Light Probe
 * (mirror-ball) image file. Layers of kinds 0 and 1 render as in v19. */
/* v26 (material values): OC_VALUE_*, OcSurface.values and, at the end of
 * OcSettings, value_passes / value_out / value_count. A render that asks
 * for no values renders exactly as in v25. */
/* v27 (Cycles built-in Render Outputs): OcMesh's Cryptomatte names and
 * Segment ID colour, OcSettings' manifest callback, and the UV, Segment and
 * cryptomatte passes (OC_PASS_UV ... OC_PASS_COUNT 49). */
/* v28 (render outputs group D): the sample count, volume and background
 * passes (OC_PASS_SAMPLE_COUNT ... OC_PASS_COUNT 53) and, at the end of
 * OcSettings, world_lightgroup and lamps_only. A render that leaves them
 * unset renders exactly as in v27. */
/* v29 (node editing, NE-03): OC_EXPR_NODE and, at the end of OcSurface,
 * expr_sockets / expr_socket_count. Surfaces without free nodes render
 * exactly as in v28. */
#define OC_ABI_VERSION 29

/* Structs shared with Rust, for oc_struct_size. */
enum {
  OC_STRUCT_EXPR = 0,
  OC_STRUCT_SURFACE = 1,
  OC_STRUCT_MESH = 2,
  OC_STRUCT_LIGHT = 3,
  OC_STRUCT_SETTINGS = 4,
  OC_STRUCT_REPORT = 5,
  OC_STRUCT_CONTROL = 6,
  OC_STRUCT_ENVIRONMENT = 7,
  OC_STRUCT_ENV_LAYER = 8,
  OC_STRUCT_NODE = 9,
  OC_STRUCT_NODE_VALUE = 10,
  OC_STRUCT_NODE_LINK = 11,
  OC_STRUCT_COUNT
};

/* One node of a compiled shader tree surface. Nodes only read nodes
 * before them (a, b, fac are indices into the same array). */
enum {
  OC_EXPR_CONST = 0,       /* value */
  OC_EXPR_IMAGE_COLOR = 1, /* images[image] at the mesh's UVs */
  OC_EXPR_IMAGE_ALPHA = 2,
  OC_EXPR_LUMA = 3, /* luminance of a */
  OC_EXPR_MIX = 4,  /* Cycles Mix Color of type `mode` (NodeMix): a, b, fac */
  OC_EXPR_CHANNEL = 5, /* component `mode` (0 R, 1 G, 2 B) of a, as a grey */
  OC_EXPR_GAMMA = 6,   /* a raised to value[0] per component */
  /* Texture Locators. These give vectors (x, y, z). */
  OC_EXPR_LOCATOR = 7, /* the world position in a locator's space: xform (row-major 3x4) applied */
  OC_EXPR_PLANAR = 8,  /* vector a on axis `mode` (0 X, 1 Y, 2 Z): u, v =
                        * ((in-plane + value[0]) * value[1], ... * value[2]), depth 0.
                        * In-plane: X (-z, y), Y (x, -z), Z (x, y). */
  OC_EXPR_UV = 9,      /* the mesh UVs * (value[0], value[1]), turned value[2] radians about 0 */
  OC_EXPR_TILE = 10,   /* vector a tiled per axis: mode = OC_TILE_* (u) | OC_TILE_* (v) << 2 */
  OC_EXPR_INSIDE = 11, /* 1 where vector a is in 0..1 on each clipped axis (mode bit 0 u,
                        * bit 1 v), else 0 */
  /* Procedural textures (operandi-core procedural.rs). */
  OC_EXPR_PATTERN = 12,   /* pattern t (0..1) at vector a; mode 0 Checker, 1 Grid, 2 Dots,
                           * 3 Noise; xform holds [subtype, axis, ...parameters] */
  OC_EXPR_BIAS_GAIN = 13, /* Perlin bias value[0] then gain value[1] of a */
  /* Projections, culling and falloff (stage 3). PLANAR's mode also carries
   * mirror bits << 2 (bit 0 u, bit 1 v). */
  OC_EXPR_WRAP = 14,        /* Cylindrical (mode bit 2 clear) / Spherical coords of locator
                             * vector a around axis mode & 3: ((u0, v0) + value[0]) * value[1..2] */
  OC_EXPR_NORMAL = 15,      /* world normal through 3x3 xform[0..9], normalised */
  OC_EXPR_FACE_WEIGHT = 16, /* face weight from normal a: mode = face | kind << 3 (0 Box signed,
                             * 1 Cubic dominant, 2 Triplanar |n|^value[0], normalised) */
  OC_EXPR_FACING = 17,      /* 1 where normal a component `mode` > 0 */
  OC_EXPR_FALLOFF = 18,     /* falloff opacity of locator point a: mode 0 Cubic, 1 Spherical,
                             * 2..4 Linear X/Y/Z; amounts value[0..2] */
  /* Stage 4. PATTERN modes add 4 Cellular, 5 Wood, 6 Ripples, 7 Weave;
   * stage 5 adds 8 Gabor Noise, 9 Poisson Cells. */
  OC_EXPR_CELL_RANDOM = 19, /* three randoms for the pattern cell at vector a (mode, xform as
                             * PATTERN) */
  OC_EXPR_HSV_VARY = 20,    /* colour a varied by randoms b: value x (1 - value[0] r0), hue +
                             * value[1] (r1 - 0.5), saturation x (1 - value[2] r2) */
  OC_EXPR_PROBE = 21, /* Light Probe (mirror ball) coords of locator vector a, axis `mode`
                       * the picture's up, x value[0], value[1] (shader_eval::probe_coords) */
  /* Gradient family (numbered apart, so other op ranges can grow). */
  OC_EXPR_GEOMETRY = 40,   /* OC_GEOMETRY_* `mode`, as a grey */
  OC_EXPR_DISTANCE = 41,   /* metres from the shading point to value */
  OC_EXPR_RAMP_COLOR = 42, /* ramps[image .. image + mode) (RGBA stops) read at a, mapped
                            * from value[0]..value[1] to the ramp, held at the ends */
  OC_EXPR_RAMP_ALPHA = 43, /* the same ramp's alpha, as a grey */
  OC_EXPR_ATTRIBUTE_COLOR = 44, /* mesh corner attribute attributes[image]: its colour */
  OC_EXPR_ATTRIBUTE_ALPHA = 45, /* ... its alpha */
  OC_EXPR_HSV = 46,       /* a in HSV: hue * value[0] + value[1] degrees (wrapped),
                           * saturation * value[2] (clamped) */
  OC_EXPR_LOCATOR_INCIDENCE = 48, /* degrees between the normal and the direction to value */
  OC_EXPR_OCCLUSION = 47, /* open fraction of `image` AO rays within value[0] m;
                           * mode bit 0 only this object, bit 1 inside, bits 2-3 the
                           * hemisphere: 0 normal, 1 world up, 2 down the slope,
                           * 3 the view's mirror direction (AoToward) */
  OC_EXPR_NODE = 49, /* output socket expr_sockets[image] of nodes[a] (node editing: a
                      * free MaterialX node driving a channel, NE-03) */
};
enum {
  OC_GEOMETRY_INCIDENCE = 0, /* degrees from facing the viewer (0) to edge-on (90) */
  OC_GEOMETRY_SLOPE = 1,     /* degrees from facing +Y (0) to facing down (180) */
  OC_GEOMETRY_CAMERA_DISTANCE = 2, /* metres from the camera */
  OC_GEOMETRY_BACKFACING = 3,      /* 1 on the back side */
  OC_GEOMETRY_SCREEN_X = 4,        /* 0 at the frame's left edge, 1 at its right */
  OC_GEOMETRY_SCREEN_Y = 5,        /* 0 at the frame's bottom edge, 1 at its top */
  OC_GEOMETRY_SCREEN_R = 6,        /* 0 at the frame's centre, 1 at its corners */
};

/* Tiling and image extension (OcSurface.image_extensions, OC_EXPR_TILE).
 * The values are Cycles' ExtensionType. */
enum {
  OC_TILE_REPEAT = 0,
  OC_TILE_EDGE = 1,   /* extend: clamp to the border */
  OC_TILE_RESET = 2,  /* clip: transparent black outside */
  OC_TILE_MIRROR = 3,
};

/* Image filtering (OcSurface.image_filters). */
enum {
  OC_FILTER_NEAREST = 0,
  OC_FILTER_BILINEAR = 1,
  OC_FILTER_BICUBIC = 2,
};
typedef struct OcExpr {
  uint32_t op;
  uint32_t mode;
  int32_t a;
  int32_t b;
  int32_t fac;
  float value[3];
  int32_t image;
  float xform[12]; /* OC_EXPR_LOCATOR: the matrix; OC_EXPR_PATTERN: parameters */
} OcExpr;

/* Principled BSDF inputs, in this order. */
enum {
  OC_INPUT_BASE_COLOR,
  OC_INPUT_METALLIC,
  OC_INPUT_ROUGHNESS,
  OC_INPUT_IOR,
  OC_INPUT_ALPHA,
  OC_INPUT_DIFFUSE_ROUGHNESS,
  OC_INPUT_SUBSURFACE_WEIGHT,
  OC_INPUT_SUBSURFACE_RADIUS,
  OC_INPUT_SUBSURFACE_SCALE,
  OC_INPUT_SPECULAR_IOR_LEVEL,
  OC_INPUT_SPECULAR_TINT,
  OC_INPUT_ANISOTROPIC,
  OC_INPUT_TRANSMISSION_WEIGHT,
  OC_INPUT_SHEEN_WEIGHT,
  OC_INPUT_COAT_WEIGHT,
  OC_INPUT_COAT_ROUGHNESS,
  OC_INPUT_EMISSION_COLOR,
  OC_INPUT_EMISSION_STRENGTH,
  OC_INPUT_SUBSURFACE_ANISOTROPY,
  OC_INPUT_COAT_TINT,
  OC_INPUT_COAT_IOR,
  OC_INPUT_SHEEN_TINT,
  OC_INPUT_SHEEN_ROUGHNESS,
  OC_INPUT_THIN_FILM_THICKNESS, /* nanometres */
  OC_INPUT_THIN_FILM_IOR,
  OC_INPUT_DISPERSION_SCALE,
  OC_INPUT_DISPERSION_ABBE,
  OC_INPUT_COUNT
};

/* The see-through part of a surface (OcSurface.transparency_mode). */
enum {
  OC_TRANSPARENCY_NONE = 0,
  OC_TRANSPARENCY_TRANSPARENT = 1, /* Transparent BSDF, transparency_color */
  OC_TRANSPARENCY_GLASS = 2,       /* Glass BSDF: colour, refraction IOR, roughness */
};

/* A general node graph (Nodes N1, NM-30): Cycles nodes by their own type
 * names ("noise_texture", "principled_bsdf"...), socket values by Cycles'
 * socket names, and links. A node of type "output" stands for the
 * material output: what links into its "Surface" is the graph's surface,
 * mixed over the layered surface by OcSurface.node_mix; into its
 * "Displacement", the surface's displacement. Unknown types, sockets and
 * enum names are skipped. */
enum {
  OC_NODE_VALUE_FLOAT = 0, /* value[0] */
  OC_NODE_VALUE_VEC3 = 1,  /* value[0..3]: a colour, vector, point or normal */
  OC_NODE_VALUE_INT = 2,   /* value[0], rounded */
  OC_NODE_VALUE_BOOL = 3,  /* value[0] != 0 */
  OC_NODE_VALUE_TEXT = 4,  /* text: a string, or an enum value by name */
  OC_NODE_VALUE_RAMP = 5,  /* array: array_count RGBA entries (sets the colour
                            * array `socket` and the float array "<socket> Alpha") */
  OC_NODE_VALUE_EXPR = 6,  /* linked from OcSurface.exprs[expr] */
};
typedef struct OcNodeValue {
  const char *socket;
  uint32_t kind;
  float value[4];
  const char *text;
  const float *array;
  uint32_t array_count;
  int32_t expr;
} OcNodeValue;
typedef struct OcNode {
  const char *type;
  const OcNodeValue *values;
  uint32_t value_count;
} OcNode;
/* from/to index OcSurface.nodes. */
typedef struct OcNodeLink {
  uint32_t from;
  const char *from_socket;
  uint32_t to;
  const char *to_socket;
} OcNodeLink;

/* A surface compiled from the shader tree. */
/* Material values a render can write (material-value Render Outputs),
 * in the order of Rust's MaterialValue::ALL. Each is an AOV named
 * "operandi_value_<name>" (OC_VALUE_NAMES in the .cpp). */
enum {
  OC_VALUE_DIFFUSE_AMOUNT = 0,
  OC_VALUE_DIFFUSE_COEFFICIENT = 1,
  OC_VALUE_DIFFUSE_ENERGY_CONSERVATION = 2,
  OC_VALUE_DIFFUSE_ROUGHNESS = 3,
  OC_VALUE_SPECULAR_COEFFICIENT = 4,
  OC_VALUE_REFLECTION_COEFFICIENT = 5,
  OC_VALUE_ROUGHNESS = 6,
  OC_VALUE_SUBSURFACE_AMOUNT = 7,
  OC_VALUE_SUBSURFACE_COLOR = 8,
  OC_VALUE_TRANSPARENT_AMOUNT = 9,
  OC_VALUE_TRANSPARENT_COLOR = 10,
  OC_VALUE_DPDU = 11, /* world-space unit tangent along the first UV map's U */
  OC_VALUE_DPDV = 12, /* cross(shading normal, that tangent) */
  OC_VALUE_COUNT = 13
};

typedef struct OcSurface {
  const OcExpr *exprs;
  uint32_t expr_count;
  int32_t inputs[OC_INPUT_COUNT]; /* node per input, -1 = Cycles' default */
  const char *const *images;      /* image file paths */
  uint32_t image_count;
  uint32_t visible_to_camera;
  uint32_t visible_to_diffuse;
  uint32_t visible_to_glossy;
  uint32_t visible_to_transmission;
  uint32_t cast_shadows;
  uint32_t shadow_catcher;
  const uint32_t *image_filters; /* OC_FILTER_* per image; NULL = bilinear */
  /* Per image: 1 = plain numbers (a normal map), 0 = colour. NULL = colour. */
  const uint32_t *image_raw;
  /* Surface relief (G-07). Each is a node index, or -1 for none.
   * bump: the height in scene units, fed to one Bump node (Distance 1,
   * Strength 1). normal_map: a tangent-space normal colour (needs the
   * mesh's UVs), fed to the Bump node's Normal. displacement: how far the
   * surface moves along its normal in scene units; the mesh is then diced
   * finely (OcSettings.dicing_rate) and truly displaced. */
  int32_t bump;
  int32_t normal_map;
  int32_t displacement;
  /* How much of the bump and normal map the clear coat follows, 0..1. */
  float coat_bump;
  /* Transparency (OC_TRANSPARENCY_*). When not NONE the output is a Mix
   * Shader with factor transparency_amount between the principled BSDF
   * (0) and the see-through BSDF (1), and the principled Alpha input is
   * applied as an outer Mix Shader with a white Transparent BSDF (alpha 0)
   * instead of on the principled BSDF. Node indices, -1 = none. */
  uint32_t transparency_mode;
  int32_t transparency_amount;
  int32_t transparency_color;
  int32_t refraction_ior;       /* GLASS only */
  int32_t refraction_roughness; /* GLASS only */
  /* 1 = a light's visible beam (LP-40): no surface, a volume scattering
   * volume_color with density volume_density * (1 - volume_attenuation) ^
   * (metres from volume_origin). */
  uint32_t volume;
  float volume_color[3];
  float volume_density;
  float volume_attenuation;
  float volume_origin[3];
  /* NULL, or the light group (LPE Label, LP-22) this glowing surface's
   * light counts toward. */
  const char *lightgroup;
  /* Per image: the node giving its lookup vector (an earlier node), or -1
   * for the mesh UVs; and its extension (OC_TILE_*). NULL = UVs, repeat. */
  const int32_t *image_vectors;
  const uint32_t *image_extensions;
  /* Shader Alpha Type Constant: this much of the surface is mixed with a
   * Holdout, so the picture's alpha drops by it; 0 = none. */
  float holdout;
  /* Shader fog (OC_FOG_*), for camera rays: the surface blends toward
   * fog_color (the world colour with fog_use_environment) by f, with d
   * the distance from the camera in metres and h the height above Y 0:
   * LINEAR clamp((d - fog_start) / (fog_end - fog_start), 0, 1);
   * EXPONENTIAL 1 - exp(-fog_density d); LAYERED the same with density
   * fog_density exp(-max(h, 0)); UNDERWATER as EXPONENTIAL, and the base
   * colour also loses its saturation by f. */
  uint32_t fog_type;
  float fog_color[3];
  uint32_t fog_use_environment;
  float fog_start;
  float fog_end;
  float fog_density;
  /* Gradient ramp stops, 4 floats (RGBA) each, ramp_count of them. */
  const float *ramps;
  uint32_t ramp_count;
  /* Names of the RGBA corner attributes the nodes read (vertex maps,
   * variation numbers); each mesh with this surface gives their values. */
  const char *const *attributes;
  uint32_t attribute_count;
  /* Rounded Edge: a Bevel normal of this radius (m); 0 = none. */
  float rounded_edge;
  /* Absorption Distance on glass: a Volume Absorption of this colour and
   * density (per metre); density 0 = none. */
  float absorption_color[3];
  float absorption_density;
  /* A Node Material (NM-31): the graph's surface is mixed over the
   * layered one (after its transparency, before fog and holdout) with a
   * Mix Shader whose factor is exprs[node_mix], or 1 for -1. node_count 0 =
   * none. */
  const OcNode *nodes;
  uint32_t node_count;
  const OcNodeLink *node_links;
  uint32_t node_link_count;
  int32_t node_mix;
  /* OpenPBR transmission_scatter inside a GLASS see-through part: a
   * Volume Scatter of this colour, density (per metre) and anisotropy,
   * added to the absorption; density 0 = none. */
  float scatter_color[3];
  float scatter_density;
  float scatter_anisotropy;
  /* 1 = the principled BSDF's Thin Wall (OpenPBR geometry_thin_walled). */
  uint32_t thin_wall;
  /* 1 = the GLASS see-through part is a second principled BSDF at full
   * Transmission Weight (Base Color = transparency_color, IOR, Roughness,
   * and the thin film, dispersion and Thin Wall inputs of the surface)
   * instead of a Glass BSDF, which has no dispersion or thin wall. */
  uint32_t glass_principled;
  /* Bit k: this surface fills grouped Render Output k (OC_PASS_GROUP_0 + k). */
  uint32_t output_groups;
  /* Material values (OC_VALUE_*): the expression each writes into its
   * "operandi_value_<name>" AOV, -1 = not written. dPdu and dPdv are
   * never read from here: every surface writes them from its UVs. */
  int32_t values[OC_VALUE_COUNT];
  /* Output socket names OC_EXPR_NODE reads (ABI 29). Graph nodes are made
   * before the ops, so ops may read them and they may read ops. */
  const char *const *expr_sockets;
  uint32_t expr_socket_count;
} OcSurface;

enum {
  OC_FOG_NONE = 0,
  OC_FOG_LINEAR = 1,
  OC_FOG_EXPONENTIAL = 2,
  OC_FOG_LAYERED = 3,
  OC_FOG_UNDERWATER = 4,
};

/* One triangulated mesh, in its own space: `transform` places it in the
 * world. Meshes that share geometry_key and surface_key share one copy of
 * their points in Cycles (instancing). */
typedef struct OcMesh {
  const float *positions; /* xyz per vertex */
  uint32_t vertex_count;
  const uint32_t *triangles; /* three vertex indices per triangle */
  const uint8_t *smooth;     /* one flag per triangle; 0 = flat */
  uint32_t triangle_count;
  /* 1 = render with the material below; 0 = the default material. */
  uint32_t has_material;
  float base_color[3]; /* linear RGB */
  float ior;           /* sets the straight-on specular reflectance */
  /* When not NULL, replaces the material above. */
  const OcSurface *surface;
  /* NULL, or two floats per triangle corner (triangle_count * 3 * 2). */
  const float *uvs;
  /* Light Linking (LP-30..32): the receiver and shadow-blocker light set
   * (0 = lit by every light), and for a glowing light surface the sets it
   * lights (all bits = every set). */
  uint32_t light_set;
  uint64_t light_membership;
  /* NULL, or the surface's corner attributes: per triangle corner, 4
   * floats (RGBA) for each of surface->attributes in turn. */
  const float *attribute_values;
  /* NULL (Cycles averages the triangles around each point), or the shading
   * normal at each triangle corner: 3 floats each, triangle_count * 3 of
   * them, in the mesh's own space. Smooth triangles use them. */
  const float *normals;
  /* Row-major 3x4 matrix from the mesh's space to the world. */
  float transform[12];
  /* Content keys, 0 = none. Equal geometry_key: the same points,
   * triangles, smooth flags, UVs, normals and attribute values. Equal
   * surface_key: the same surface (OcSurface contents and image files).
   * An OcSession keeps what it built under these keys and reuses it. */
  uint64_t geometry_key;
  uint64_t surface_key;
  /* Cryptomatte names (NULL = ""): the Object id is name, the Asset id
   * asset_name, and the Material id material_name, which names the mesh's
   * own surface (the shared default surface stays "Default"). */
  const char *name;
  const char *asset_name;
  const char *material_name;
  /* The Segment ID pass's colour for this mesh (linear RGB). */
  float segment_color[3];
} OcMesh;

/* One scene light. kind: 0 sun, 1 point, 2 spot, 3 area, 4 portal (an
 * area-light portal sized by `size`/`ellipse`; it makes no light and guides
 * world-light sampling, strength ignored). The light travels along
 * `direction` (its -Z); `right` and `up` complete its frame. */
typedef struct OcLight {
  uint32_t kind;
  float position[3];
  float right[3];
  float up[3];
  float direction[3];
  float strength[3]; /* per colour channel: W/m2 for a sun, W otherwise */
  float angle;       /* sun: angular diameter, radians */
  float radius;      /* point and spot */
  float spot_size;   /* spot: full cone angle, radians */
  float spot_blend;  /* spot: 0..1 */
  float size[2];     /* area: width and height, metres */
  uint32_t ellipse;  /* area: 1 = ellipse/disk */
  uint32_t cast_shadow;
  uint32_t visible_to_camera;
  uint32_t affects_diffuse;
  uint32_t affects_specular;
  /* 1 = bounced (reflection / refraction) rays can hit the light's shape:
   * Cycles' multiple importance sampling on for it. */
  uint32_t visible_to_reflection;
  uint32_t visible_to_refraction;
  /* NULL, or an IES photometry file's text: the light's emission is
   * shaped by it (point/spot; strength multiplies the file's watts). */
  const char *ies;
  /* NULL, or the light group (LPE Label, LP-22) for its own pass. */
  const char *lightgroup;
  /* Light-material image layers (gobos), bottom first: image file,
   * NodeMix blend type and opacity per layer, projected through a spot's
   * cone; gobo_tan_half = tan(half the cone), 0 = layers not projected. */
  const char *const *layer_images;
  const uint32_t *layer_blend;
  const float *layer_opacity;
  uint32_t layer_count;
  float gobo_tan_half;
  /* Light Linking: the receiver light sets this light lights and whose
   * blockers shadow it (all bits = every set). */
  uint64_t light_membership;
  /* The blocker sets that shadow it (usually light_membership; 0 for a
   * copy lighting surfaces whose shader has Receive Shadows off). */
  uint64_t shadow_membership;
  /* Direct Caustics (OcSettings.direct_caustics): 0 = none, 1 = this light
   * also casts shadow caustics (MNEE) through glass, 2 = it lights nothing
   * else, it only casts caustics (Cycles patch caustics-only-lights). */
  uint32_t caustics;
} OcLight;

/* Environment Type (OcEnvironment.kind). */
enum {
  OC_ENV_CONSTANT = 0,
  OC_ENV_TWO_COLOR = 1,
  OC_ENV_FOUR_COLOR = 2,
  OC_ENV_OVERCAST = 3,
};

/* One texture layer of an environment (spec §5), blended over what is
 * below it with Cycles' Mix Color `mix` mode at `opacity`. */
typedef struct OcEnvLayer {
  /* 0 = constant colour, 1 = latitude-longitude image file, 2 = Light
   * Probe (mirror-ball) image file. */
  uint32_t kind;
  float color[3];
  float opacity;
  uint32_t mix;
  uint32_t invert;
  const char *image;
  /* Rows of the 3x3 matrix taking a world ray direction into the image's
   * Z-up equirectangular frame (kind 1) or Cycles' mirror-ball frame
   * (kind 2). */
  float to_image[9];
} OcEnvLayer;

/* One environment of the render world (spec environments-2026-11-09). The
 * colour along a ray depends on s, the ray direction's component along
 * `up` (the environment item's +Y, unit): formulas in environment_color in
 * operandi_cycles.cpp. Colours are linear RGB, times intensity. */
typedef struct OcEnvironment {
  uint32_t kind;
  float zenith[3];
  float sky[3];
  float ground[3];
  float nadir[3];
  float sky_exponent;
  float ground_exponent;
  float intensity;
  float up[3];
  /* Seen by camera, diffuse (indirect), glossy (reflection) and
   * transmission (refraction) rays: 1 = yes. */
  uint32_t visible[4];
  /* The Environment Material over black: opacity, and 1 = inverted. */
  float material_opacity;
  uint32_t material_invert;
  /* Texture layers over it, bottom first. */
  const OcEnvLayer *layers;
  uint32_t layer_count;
  /* 1 = Physically-based Daylight: Cycles' single-scattering sky replaces
   * the gradient. to_sky (rows) takes a world direction into the sky's
   * Z-up frame; the sun's elevation and rotation are in that frame;
   * sun_size is its angular diameter (0 = no disc). */
  uint32_t physical;
  float to_sky[9];
  float sun_elevation;
  float sun_rotation;
  float sun_size;
  float ground_albedo;
  float aerosol;
} OcEnvironment;

typedef struct OcSettings {
  uint32_t width;
  uint32_t height;
  uint32_t samples;
  /* Camera placement: right, up and forward axes plus the eye position,
   * all in world space. forward points into the scene. */
  float camera_right[3];
  float camera_up[3];
  float camera_forward[3];
  float camera_position[3];
  /* Field of view in radians across the image's shorter side. */
  float fov;
  /* Direction the sunlight travels (from the sun toward the scene). */
  float sun_direction[3];
  float sun_strength;
  /* The world: environments, newest first. Each ray kind sees the first
   * one visible to it; none = black. */
  const OcEnvironment *environments;
  uint32_t environment_count;
  /* The colour fog with "use environment" takes. */
  float environment_fog_color[3];
  /* Default material: a plain principled surface. */
  float base_color[3];
  float roughness;
  /* 1 = use the GPU (Metal on macOS) when there is one; 0 = CPU. */
  uint32_t use_gpu;
  /* 1 = clean up noise with OpenImageDenoise after the last sample. */
  uint32_t denoise;
  /* Folder holding Cycles' kernel sources ("source/"), which GPU devices
   * compile at first use. NULL or "" when only the CPU is used. */
  const char *resource_path;
  /* 1 = orthographic camera; ortho_half_short is then half the picture's
   * shorter side in metres. */
  uint32_t camera_orthographic;
  float ortho_half_short;
  /* Film offset: the view plane moved by this many half-widths (x) and
   * half-heights (y). */
  float shift[2];
  float clip_near;
  float clip_far;
  /* The scene's lights. With use_scene_lights = 1 they replace the sun
   * above (an empty list then means no lights). */
  const OcLight *lights;
  uint32_t light_count;
  uint32_t use_scene_lights;
  /* Region render: only this rectangle of the picture is rendered.
   * x, y (from the top-left corner), width and height in pixels; a width
   * or height of 0 renders the whole picture. */
  uint32_t region[4];
  /* Light paths. Bounce limits per kind; -1 keeps Cycles' default, 0 turns
   * that kind of indirect light off (the Preview's Global Illumination,
   * Reflections and Refractions options). */
  int32_t max_diffuse_bounces;
  int32_t max_glossy_bounces;
  int32_t max_transmission_bounces;
  /* 1 = no light casts shadows (the Preview's Shadows option off). */
  uint32_t no_shadows;
  /* Adaptive sampling noise threshold; 0 = adaptive sampling off. */
  float adaptive_threshold;
  /* CPU render threads; 0 = all cores. GPU devices ignore it. */
  uint32_t threads;
  /* 1 = transparent film: the world still lights the scene but shows as
   * alpha 0, so alpha is coverage (the Preview's Alpha effect). */
  uint32_t transparent_film;
  /* Displaced meshes are diced into pieces about this many pixels across
   * (0 = 1 pixel). */
  float dicing_rate;
  /* 1 = the world lights only glossy and transmission, not diffuse
   * surfaces (Cycles' world ray visibility; used by calibration renders). */
  uint32_t world_hidden_from_diffuse;
  /* Dome lights (LP-DM-01): extra even world light (linear RGB radiance)
   * that lights the scene but isn't seen by camera rays. */
  float dome[3];
  /* Light groups (LPE Labels): a combined pass per name, written into
   * light_group_out[i] (width * height * 4 floats each, laid out like
   * rgba_out). */
  const char *const *light_groups;
  float *const *light_group_out;
  uint32_t light_group_count;
  /* Panoramic camera (CP-V13): 0 = none (camera_orthographic decides),
   * 1 = equirectangular, 2 = central cylindrical. The camera axes stay
   * right/up/forward; the bridge turns them into Cycles' panorama frame. */
  uint32_t panorama;
  /* Equirectangular: longitude min/max and latitude min/max, radians.
   * Cylindrical: angle min/max (radians) and height min/max on the unit
   * cylinder. Longitude/angle 0 is the forward axis. */
  float panorama_range[4];
  /* Depth of field (CP-E01): aperture radius in metres (0 = off), focus
   * distance in metres, iris blades (0 = round) and their rotation. */
  float aperture_size;
  float focal_distance;
  uint32_t blades;
  float blades_rotation;
  /* Render Output passes (OC_PASS_*), each written into pass_out[i]
   * (width * height * 4 floats, laid out like rgba_out; one-value passes
   * in the first float of each pixel). */
  const uint32_t *passes;
  float *const *pass_out;
  uint32_t pass_count;
  /* Ambient Occlusion pass ray length, metres. */
  float ao_distance;
  /* The Render item's ray settings (render item spec v1). Total bounce
   * limit and transparent bounces; -1 keeps Cycles' default. */
  int32_t max_bounces;
  int32_t max_transparent_bounces;
  /* Maximum Radiance: bounced light is clamped to this; 0 = off. */
  float sample_clamp_indirect;
  /* Indirect Caustics: 1 = trace light focused by mirrors / by glass. */
  uint32_t caustics_reflective;
  uint32_t caustics_refractive;
  /* Direct Caustics: 1 = lights with OcLight.caustics cast caustics through
   * see-through Glass surfaces (casters) onto every other mesh (receivers),
   * with Cycles' manifold next event estimation. */
  uint32_t direct_caustics;
  /* Stop after this many seconds and keep what is done; 0 = no limit
   * (the Preview's Max Render Time). */
  float time_limit;
  /* Content keys, 0 = none: equal world_key, the same environments and
   * dome; equal lights_key, the same lights, sun and shadow option. An
   * OcSession keeps the world and lights it built under them. */
  uint64_t world_key;
  uint64_t lights_key;
  /* Material values (OC_VALUE_*), each written into value_out[i] like the
   * passes (colour AOVs; grey values in all three colour floats). */
  const uint32_t *value_passes;
  float *const *value_out;
  uint32_t value_count;
  /* Cryptomatte manifests: after a finished render with cryptomatte
   * passes, called once per type asked for (0 object, 1 material, 2 asset)
   * with Cycles' manifest, a JSON object of name to 8-digit hex hash.
   * NULL = not wanted. */
  void *manifest_user;
  void (*on_cryptomatte_manifest)(void *user, uint32_t type, const char *json);
  /* The light group (LPE Label) the world counts toward; NULL = none. */
  const char *world_lightgroup;
  /* 1 = only light items light the scene: glowing surfaces light nothing
   * (light linking: they belong to no light set). Used with an empty world
   * by the Illumination (Unshadowed) helper render. */
  uint32_t lamps_only;
} OcSettings;

enum {
  OC_PASS_DEPTH = 0,
  OC_PASS_NORMAL = 1,
  OC_PASS_DIFFUSE_DIRECT = 2,
  OC_PASS_DIFFUSE_INDIRECT = 3,
  OC_PASS_DIFFUSE_COLOR = 4,
  OC_PASS_GLOSSY_DIRECT = 5,
  OC_PASS_GLOSSY_INDIRECT = 6,
  OC_PASS_GLOSSY_COLOR = 7,
  OC_PASS_TRANSMISSION_DIRECT = 8,
  OC_PASS_TRANSMISSION_INDIRECT = 9,
  OC_PASS_TRANSMISSION_COLOR = 10,
  OC_PASS_EMISSION = 11,
  OC_PASS_AO = 12,
  OC_PASS_INCIDENCE = 13,          /* AOV: 0 facing the camera, 1 edge-on */
  OC_PASS_OBJECT_COORDINATES = 14, /* AOV: object-space position */
  OC_PASS_GROUP_0 = 15, /* AOVs 15..22: 1 where the k-th grouped Render Output's
                         * Material Group applies (OcSurface.output_groups bit k) */
  OC_PASS_UV = 23,      /* AOV: the mesh UVs (u, v, 0) */
  OC_PASS_SEGMENT = 24, /* AOV: OcMesh.segment_color */
  /* Cycles' cryptomatte passes, 8 per type: pass k holds levels 2k and 2k+1
   * as (id, coverage, id, coverage). The deepest k asked for sets Cycles'
   * cryptomatte depth; ask for passes 0..k of every type wanted. */
  OC_PASS_CRYPTO_OBJECT_0 = 25,
  OC_PASS_CRYPTO_MATERIAL_0 = 33,
  OC_PASS_CRYPTO_ASSET_0 = 41,
  /* Samples each pixel took over the samples rendered (0..1); only with
   * adaptive sampling on. */
  OC_PASS_SAMPLE_COUNT = 49,
  OC_PASS_VOLUME_DIRECT = 50,
  OC_PASS_VOLUME_INDIRECT = 51,
  OC_PASS_BACKGROUND = 52,
  OC_PASS_COUNT = 53
};

/* What actually happened, filled by oc_render when not NULL. */
typedef struct OcReport {
  char device[128]; /* e.g. "Apple M1 Max (Metal)" or "CPU" */
  uint32_t used_gpu;
  uint32_t denoised;
  uint32_t samples; /* samples per pixel actually rendered */
} OcReport;

/* Commands for a running render (OcControl.command). */
enum {
  OC_RUN = 0,
  OC_PAUSE = 1,  /* hold between sample batches until the command changes */
  OC_FINISH = 2, /* stop adding samples; denoise and deliver what is there */
  OC_CANCEL = 3, /* stop now; oc_render returns an error */
};

/* Lets the caller steer a render while it runs and watch it come in.
 * Every field may be 0/NULL. Callbacks run on Cycles' render thread. */
typedef struct OcControl {
  /* The caller's command word (OC_RUN...), which it may change at any
   * time; read atomically. NULL = just run. */
  const uint32_t *command;
  void *user;
  /* Samples done so far, the total asked for, and 1 while paused. */
  void (*on_progress)(void *user, uint32_t samples_done, uint32_t samples_total, uint32_t paused);
  /* The rendered area so far: width * height * 4 linear floats, top row
   * first, covering the region only (the whole picture without one).
   * Borrowed for the call. */
  void (*on_preview)(void *user, const float *rgba, uint32_t width, uint32_t height);
} OcControl;

/* Renders and fills rgba_out (width * height * 4 linear floats,
 * top row first). With a region, only the region's pixels are written and
 * the rest of rgba_out is left as it was. control may be NULL. Returns 0
 * on success; otherwise writes a message to error (always NUL-terminated
 * when error_len > 0). */
OC_API int oc_render(const OcMesh *meshes,
              uint32_t mesh_count,
              const OcSettings *settings,
              float *rgba_out,
              OcControl *control,
              OcReport *report,
              char *error,
              size_t error_len);

/* A renderer kept between renders (after Blender's interactive sync): its
 * device, compiled kernels, and the meshes, surfaces, images, world and
 * lights the last render built stay loaded. Each oc_session_render then
 * only rebuilds what changed (by the content keys above) and moves the
 * rest. Image files are read once: free the session when one changes.
 * One render at a time per session. */
typedef struct OcSession OcSession;

OC_API OcSession *oc_session_new(void);
OC_API void oc_session_free(OcSession *session);

/* oc_render through `session`. After a failed render, or one cancelled
 * before Cycles took in the whole scene, the session starts afresh next
 * time. */
OC_API int oc_session_render(OcSession *session,
                             const OcMesh *meshes,
                             uint32_t mesh_count,
                             const OcSettings *settings,
                             float *rgba_out,
                             OcControl *control,
                             OcReport *report,
                             char *error,
                             size_t error_len);

/* Cycles version string, e.g. "5.1.0". */
OC_API const char *oc_cycles_version(void);

/* OC_ABI_VERSION as this library was built. */
OC_API uint32_t oc_abi_version(void);

/* sizeof() of the shared struct `which` (OC_STRUCT_*) as this library was
 * built; 0 for an unknown one. */
OC_API size_t oc_struct_size(uint32_t which);

#ifdef __cplusplus
}
#endif

#endif
