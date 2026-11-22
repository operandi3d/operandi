//! How each viewport display style (view_display.rs) colours a surface.
//! One formula, written twice: the WGSL from `wgsl()` for meshes kept on
//! the graphics card (gpu.rs splices it into its shader) and `shade` for
//! the projected (CPU) path that draws live primitive and Cone/Ellipsoid
//! previews. The tuning numbers below are spliced into the WGSL, and the
//! theme colours reach the shader through the look uniform and the
//! Matcap style's matcaps as a texture (matcap.rs), so both sides read the
//! same values. The tests pin the CPU
//! side and check the WGSL carries the same numbers.
//!
//! All shading works on gamma-encoded (sRGB, 0–1) colours, the way vertex
//! colours reach the shader, so the numbers below are screen values: the
//! lit styles' ambient and diffuse are fractions of the material colour,
//! chosen so the theme's default grey fill (Solid, 204) lands on the
//! viewport shading spec's targets (VS-2/VS-3, v1.1).
//!
//! Every style is fixed to the camera (VS v1.1): its lights are given in
//! view space (+X right, +Y up, +Z toward the eye) and turn with the view,
//! in perspective and in every orthographic view alike, so orbiting round
//! a model doesn't change what you see. Shaded, Texture, Default, Vertex
//! Map and Advanced are lit by one key light straight above the line of
//! sight, 45° up (`HEADLIGHT`), never by the scene's lights unless the
//! view's Show Scene Lighting is on (Advanced only). A lit style colours a
//! surface `c × (ambient + diffuse × max(0, N·L))`, so the side away from
//! the light is flat ambient. Gooch and Cel have their own view-space
//! lights; Matcap (formerly Reflection) is looked up by the view-space normal.
//! Dot products don't change under a rotation, so the shaders work in world
//! space with each view-space light turned into the world by the camera.
use crate::view_display::Style;
use eframe::egui::Color32;
use glam::Vec3;

/// The view light, in view space: straight above the line of sight, 45° up
/// (VS-2).
pub const HEADLIGHT: [f32; 3] = [
    0.,
    std::f32::consts::FRAC_1_SQRT_2,
    std::f32::consts::FRAC_1_SQRT_2,
];

/// A camera's unit axes in world space: screen right, screen up, and
/// toward the eye (minus the way it looks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewBasis {
    pub right: Vec3,
    pub up: Vec3,
    pub toward: Vec3,
}
impl ViewBasis {
    /// From a camera's right, up and forward (the way it looks).
    pub fn new(right: Vec3, up: Vec3, forward: Vec3) -> Self {
        Self {
            right,
            up,
            toward: -forward,
        }
    }
    /// A view-space direction in world space (normalised).
    pub fn to_world(self, v: [f32; 3]) -> Vec3 {
        (self.right * v[0] + self.up * v[1] + self.toward * v[2]).normalize_or(self.toward)
    }
    /// A world-space direction in view space.
    pub fn to_view(self, v: Vec3) -> Vec3 {
        Vec3::new(v.dot(self.right), v.dot(self.up), v.dot(self.toward))
    }
    /// The view light (`HEADLIGHT`) in world space.
    pub fn headlight(self) -> Vec3 {
        self.to_world(HEADLIGHT)
    }
}

/// The theme's default fill grey (Solid, 204 of 255): the spec's target
/// values are given for it, so the factors below are those values over it.
const GREY: f32 = 204.;
/// Solid: flat, unlit, #999999 for the default grey (VS-3).
pub const SOLID: f32 = 153. / GREY;
/// The lit styles' (ambient, diffuse) as fractions of the material colour
/// (VS-3): Shaded 29 + 184, Texture 22 + 139, Default 98 + 117 for the
/// default grey.
pub const SHADED: (f32, f32) = (29. / GREY, 184. / GREY);
pub const TEXTURE: (f32, f32) = (22. / GREY, 139. / GREY);
pub const DEFAULT: (f32, f32) = (98. / GREY, 117. / GREY);
/// Advanced's diffuse (its ambient is the view's Advanced Options setting,
/// 44 % white by default: 90 for the default grey). Advanced draws each
/// surface as Cycles' principled shader approximated the way Blender's
/// EEVEE does (operandi_core::blender_principled, viewport materials VM-1)
/// in linear light, with these levels as its calibration: the default grey
/// facing away from the light lands on the ambient level, and facing the
/// light on ambient + diffuse, as in VS-3.
pub const ADVANCED_DIFFUSE: f32 = 135. / GREY;
pub const ADVANCED_AMBIENT: f32 = 0.44;

/// The Advanced style's linear ambient for an Advanced Options ambient
/// `a` (a fraction of the surface colour in screen values, VS-3): the
/// default grey lit by it in linear light looks as `a × grey` did.
pub fn advanced_ambient(a: f32) -> f32 {
    use operandi_core::blender_principled::decode;
    let g = GREY / 255.;
    decode(a * g) / decode(g)
}
/// The Advanced style's key light strength (linear): the default grey
/// facing it looks as ambient + diffuse did (VS-3).
pub fn advanced_key() -> f32 {
    use operandi_core::blender_principled::decode;
    let g = GREY / 255.;
    decode(g * (ADVANCED_AMBIENT + ADVANCED_DIFFUSE)) / decode(g)
        - advanced_ambient(ADVANCED_AMBIENT)
}
/// Vertex Map: the map's colours, lightly shaded so shape still reads.
const VERTEX_MAP: (f32, f32) = (0.72, 0.28);

/// Gooch: a view-space light toward (1, 1, 1) (upper right, toward the
/// eye); the colour runs from cool to warm with |N·L|, so a surface facing
/// straight away from it is warm again. A flat face is one tone.
pub const GOOCH_LIGHT: [f32; 3] = [1., 1., 1.];
/// Cel: a view-space light; the eye is the real direction to it per pixel,
/// so the outline follows the silhouette. Tested in order: outline,
/// highlight spot, lit, shade.
pub const CEL_LIGHT: [f32; 3] = [0., 0.67, 0.74];
pub const CEL_OUTLINE_BELOW: f32 = 0.42;
pub const CEL_SPOT_ABOVE: f32 = 0.94;
pub const CEL_LIT_ABOVE: f32 = 0.59;
pub const CEL_OUTLINE: u8 = 0x40;
pub const CEL_SPOT: u8 = 0x0A;
pub const CEL_LIT: u8 = 0x7A;
pub const CEL_SHADE: u8 = 0x62;

/// The user-editable display-style colours (Display > Colors), as 0–1
/// gamma-encoded RGB, the way vertex colours reach the shader, and the
/// view's matcap (the Matcap style).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleColors {
    pub gooch_cool: [f32; 3],
    pub gooch_warm: [f32; 3],
    pub matcap: crate::matcap::Choice,
}

pub fn unit(c: Color32) -> [f32; 3] {
    [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.)
}

impl StyleColors {
    /// The colours to draw with now (the user's overrides, else defaults),
    /// with the default matcap.
    pub fn live() -> Self {
        use crate::theme::viewport as v;
        Self {
            gooch_cool: unit(v::gooch_cool()),
            gooch_warm: unit(v::gooch_warm()),
            matcap: crate::matcap::Choice::default(),
        }
    }
    /// The look uniform's rows after its first two (see `wgsl`'s `Look`):
    /// the two Gooch colours.
    pub fn uniform_rows(&self) -> [[f32; 4]; 2] {
        let row = |c: [f32; 3]| [c[0], c[1], c[2], 1.];
        [row(self.gooch_cool), row(self.gooch_warm)]
    }
}

/// The shader side: the look uniform and the per-style colour. `look.mode`
/// = (style, smoothing, overlay opacity, overlay kind), `look.wire` = the
/// colored overlay, then the `StyleColors` rows (Gooch cool and warm);
/// `ambient.w` is the Matcap style's layer in `op_matcap` (matcap.rs).
/// Spliced in after the world shader's `cam` uniform, which gives the
/// view's axes; the matcaps share the scene group with the image layers
/// and sample with their linear clamp sampler.
const WGSL: &str = r#"
struct Look {
  mode: vec4<f32>, wire: vec4<f32>,
  cool: vec4<f32>, warm: vec4<f32>,
  plain: vec4<f32>, inactive: vec4<f32>,
  ambient: vec4<f32>,
  t_front: vec4<f32>, t_back: vec4<f32>, t_mode: vec4<f32>,
  // HDRI stage 3 (gpu.rs EnvLook): irradiance harmonics, the world-to-
  // picture rows, then (on, intensity, -, -).
  env_sh0: vec4<f32>, env_sh1: vec4<f32>, env_sh2: vec4<f32>,
  env_sh3: vec4<f32>, env_sh4: vec4<f32>, env_sh5: vec4<f32>,
  env_sh6: vec4<f32>, env_sh7: vec4<f32>, env_sh8: vec4<f32>,
  env_r0: vec4<f32>, env_r1: vec4<f32>, env_r2: vec4<f32>, env_on: vec4<f32>,
  // Fade Wireframe on Small Polygons (gpu.rs Look): (on, gone, full, -).
  wire_fade: vec4<f32>,
  // Show Material Transparency (gpu.rs Look::glass_on), then the Checker
  // (UVT-114, uv_checker.rs): (on, checker, -, -).
  glass: vec4<f32>,
};
@group(1) @binding(0) var<uniform> look: Look;
// A view-space direction in world space (display_shading.rs ViewBasis).
fn view_to_world(v: vec3<f32>) -> vec3<f32> {
  return normalize(cam.right.xyz * v.x + cam.up.xyz * v.y - cam.forward.xyz * v.z);
}
@group(2) @binding({matcap_binding}) var op_matcap: texture_2d_array<f32>;
// The Matcap style's colour for world normal n (matcap::sample).
fn matcap(n: vec3<f32>) -> vec3<f32> {
  var p = vec2<f32>(dot(n, cam.right.xyz), dot(n, cam.up.xyz));
  let r = length(p);
  if (r > {matcap_reach}) { p = p * ({matcap_reach} / r); }
  let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
  return textureSampleLevel(op_matcap, op_smp_l_c, uv, i32(look.ambient.w + 0.5), 0.0).rgb;
}
@group(2) @binding({env_binding}) var op_env: texture_2d<f32>;
// World direction into the environment picture's frame.
fn env_dir(d: vec3<f32>) -> vec3<f32> {
  return normalize(vec3<f32>(dot(look.env_r0.xyz, d), dot(look.env_r1.xyz, d), dot(look.env_r2.xyz, d)));
}
// Irradiance (÷π) of the environment picture along world normal n
// (environment_gpu.rs sh_irradiance).
fn env_irradiance(n: vec3<f32>) -> vec3<f32> {
  let d = env_dir(n);
  let e = look.env_sh0.rgb * 0.282095
    + look.env_sh1.rgb * (0.488603 * d.y) + look.env_sh2.rgb * (0.488603 * d.z)
    + look.env_sh3.rgb * (0.488603 * d.x) + look.env_sh4.rgb * (1.092548 * d.x * d.y)
    + look.env_sh5.rgb * (1.092548 * d.y * d.z)
    + look.env_sh6.rgb * (0.315392 * (3.0 * d.z * d.z - 1.0))
    + look.env_sh7.rgb * (1.092548 * d.x * d.z)
    + look.env_sh8.rgb * (0.546274 * (d.x * d.x - d.y * d.y));
  return max(e, vec3<f32>(0.0)) * look.env_on.y;
}
// The blurred environment picture along world direction d
// (environment_image_uv; v kept off the poles' wrapped row).
fn env_reflected(d: vec3<f32>) -> vec3<f32> {
  let i = env_dir(d);
  let u = 0.5 - atan2(i.x, i.z) / 6.283185307;
  let v = clamp(0.5 - asin(clamp(i.y, -1.0, 1.0)) / 3.141592654, 0.01, 0.99);
  return textureSampleLevel(op_env, op_smp_l_r, vec2<f32>(u, v), 0.0).rgb * look.env_on.y;
}
fn op_decode(c: vec3<f32>) -> vec3<f32> {
  return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}
// The plain fill's surface.
fn shade(style: i32, c: vec3<f32>, map: vec3<f32>, n: vec3<f32>, to_eye: vec3<f32>, key: vec3<f32>, lit: f32) -> vec3<f32> {
  return shade_s(style, c, map, n, to_eye, key, lit, op_default_surface());
}
// n and to_eye are unit world vectors with dot(n, to_eye) >= 0; to_eye is
// the real direction to the eye at this pixel. Advanced (9) is lit by
// `key` (the view light, or the scene's with Show Scene Lighting) and its
// direct light scaled by `lit` (1 = unshadowed), and draws `s`, the
// surface's principled values (blender_principled.rs).
fn shade_s(style: i32, c: vec3<f32>, map: vec3<f32>, n: vec3<f32>, to_eye: vec3<f32>, key: vec3<f32>, lit: f32, s: OpSurface) -> vec3<f32> {
  let head = view_to_world(vec3<f32>({head}));
  let l = max(dot(n, head), 0.0);
  // Shaded (and Wireframe's unused fill).
  var rgb = c * ({shaded_a} + {shaded_d} * l);
  if (style == 1) {
    // Solid: flat and unlit.
    rgb = c * {solid};
  } else if (style == 3) {
    rgb = map * ({vmap_a} + {vmap_d} * l);
  } else if (style == 7) {
    rgb = c * ({texture_a} + {texture_d} * l);
  } else if (style == 8) {
    rgb = c * ({default_a} + {default_d} * l);
  } else if (style == 9) {
    // Advanced's ambient is the view's Advanced Options setting, in
    // linear light (display_shading.rs advanced_ambient).
    let g = vec3<f32>({grey});
    let ambient = op_decode(look.ambient.rgb * g) / op_decode(g);
    let base = op_decode(c) * s.base_scale;
    var lin = vec3<f32>(0.0);
    if (look.env_on.x > 0.5) {
      // HDRI stage 3: the environment picture lights it.
      lin = op_principled_lit(base, s, n, to_eye, key, lit, env_irradiance(n), env_reflected(reflect(-to_eye, n)), {advanced_key});
    } else {
      lin = op_principled(base, s, n, to_eye, key, lit, ambient, {advanced_key});
    }
    rgb = op_encode(max(lin, vec3<f32>(0.0)));
  } else if (style == 4) {
    // Gooch: cool to warm by |N·L|, the light fixed to the view.
    let t = abs(dot(n, view_to_world(vec3<f32>({gooch_l}))));
    rgb = mix(look.cool.rgb, look.warm.rgb, t);
  } else if (style == 5) {
    // Cel: a view-fixed light, the real eye; hard bands tested in order.
    let cl = view_to_world(vec3<f32>({cel_l}));
    let ch = normalize(cl + to_eye);
    var g = {cel_shade};
    if (dot(n, to_eye) < {cel_outline_below}) { g = {cel_outline}; }
    else if (dot(n, ch) > {cel_spot_above}) { g = {cel_spot}; }
    else if (dot(n, cl) > {cel_lit_above}) { g = {cel_lit}; }
    rgb = vec3<f32>(g);
  } else if (style == 6) {
    rgb = matcap(n);
  }
  return clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}
// A highlighted surface (selected, symmetry-selected, locked or under the
// pointer: `c` is the highlight colour): the style shades a plain grey
// surface first, then the highlight goes on top, scaled by how bright the
// style made that grey, so the style's light, bands and tones still show
// through. In the lit styles this is exactly the highlight lit as a
// surface colour. Vertex Map keeps its own tint of the map.
fn shade_marked(style: i32, c: vec3<f32>, map: vec3<f32>, n: vec3<f32>, to_eye: vec3<f32>, key: vec3<f32>, lit: f32, marked: bool, s: OpSurface) -> vec3<f32> {
  if (!marked || style == 3) { return shade_s(style, c, map, n, to_eye, key, lit, s); }
  let grey = vec3<f32>({grey});
  let luma = vec3<f32>(0.2126, 0.7152, 0.0722);
  var k = dot(shade(style, grey, map, n, to_eye, key, lit), luma) / dot(grey, luma);
  // Gooch, Cel and Reflection: their darkest tones would bury the
  // highlight, so its brightness only follows them part way.
  if (style == 4 || style == 5 || style == 6) { k = {floor} + (1.0 - {floor}) * k; }
  return clamp(c * k, vec3<f32>(0.0), vec3<f32>(1.0));
}
"#;

/// A float as a WGSL literal (always with a decimal point).
fn lit(v: f32) -> String {
    format!("{v:?}")
}
fn lit3(v: [f32; 3]) -> String {
    v.map(lit).join(", ")
}

/// The shader side with this module's numbers spliced in.
pub fn wgsl() -> String {
    let byte = |b: u8| lit(f32::from(b) / 255.);
    [
        ("{matcap_binding}", crate::matcap::BINDING.to_string()),
        ("{matcap_reach}", lit(crate::matcap::REACH)),
        ("{env_binding}", crate::gpu::ENV_BINDING.to_string()),
        ("{head}", lit3(HEADLIGHT)),
        ("{grey}", lit(GREY / 255.)),
        ("{floor}", lit(MARKED_FLOOR)),
        ("{shaded_a}", lit(SHADED.0)),
        ("{shaded_d}", lit(SHADED.1)),
        ("{solid}", lit(SOLID)),
        ("{vmap_a}", lit(VERTEX_MAP.0)),
        ("{vmap_d}", lit(VERTEX_MAP.1)),
        ("{texture_a}", lit(TEXTURE.0)),
        ("{texture_d}", lit(TEXTURE.1)),
        ("{default_a}", lit(DEFAULT.0)),
        ("{default_d}", lit(DEFAULT.1)),
        ("{advanced_key}", lit(advanced_key())),
        ("{gooch_l}", lit3(GOOCH_LIGHT)),
        ("{cel_l}", lit3(CEL_LIGHT)),
        ("{cel_outline_below}", lit(CEL_OUTLINE_BELOW)),
        ("{cel_spot_above}", lit(CEL_SPOT_ABOVE)),
        ("{cel_lit_above}", lit(CEL_LIT_ABOVE)),
        ("{cel_shade}", byte(CEL_SHADE)),
        ("{cel_outline}", byte(CEL_OUTLINE)),
        ("{cel_spot}", byte(CEL_SPOT)),
        ("{cel_lit}", byte(CEL_LIT)),
    ]
    .iter()
    .fold(
        WGSL.to_string() + &operandi_core::blender_principled::wgsl(),
        |s, (k, v)| s.replace(k, v),
    )
}

/// The light the Advanced style is lit and shadowed by. Show Scene
/// Lighting on: the first visible scene light that renders; otherwise (or
/// with no such light) `view`, the view's own light (which casts). The
/// bool: whether it casts shadows (a light whose Shadow Type is None
/// doesn't). Only one light for now (tier A1).
pub fn key_light(
    scene: &operandi_core::Scene,
    scene_lighting: bool,
    view: crate::gpu::KeyLight,
) -> (crate::gpu::KeyLight, bool) {
    use crate::gpu::KeyLight;
    use operandi_core::camera_light::{LightKind, ShadowType};
    if !scene_lighting {
        return (view, true);
    }
    for item in &scene.items {
        let Some(light) = &item.light else { continue };
        // LP-10: Render No never lights, Yes always, Default with the eye.
        // A portal makes no light (LP-PO).
        if !light.renders(scene.shows(item)) || !light.kind.emits() {
            continue;
        }
        let Ok(world) = scene.world_matrix(item.id) else {
            continue;
        };
        let key = match light.kind {
            // Lights shine down their local -Z: toward the light is +Z.
            LightKind::Directional | LightKind::Area => {
                let d = world.transform_vector3(glam::DVec3::Z).normalize_or_zero();
                if d == glam::DVec3::ZERO {
                    continue;
                }
                KeyLight::Toward(d.as_vec3().to_array())
            }
            LightKind::Portal | LightKind::Dome => continue,
            LightKind::Point
            | LightKind::Spot
            | LightKind::Cylinder
            | LightKind::Mesh
            | LightKind::Photometric => KeyLight::At(
                world
                    .transform_point3(glam::DVec3::ZERO)
                    .as_vec3()
                    .to_array(),
            ),
        };
        return (key, light.shadow != ShadowType::None);
    }
    (view, true)
}

/// The colour `style` gives a surface of base colour `c` (0–1) with vertex
/// map colour `map`, unit world normal `n`, unit direction to the eye
/// `to_eye` (the real one at that point) and the camera's axes `view`.
/// Advanced draws unshadowed here (the shadow map is the graphics card's)
/// with the default ambient. Wireframe and Shaded both return the Shaded
/// fill.
pub fn shade(
    style: Style,
    c: [f32; 3],
    map: [f32; 3],
    n: Vec3,
    to_eye: Vec3,
    view: &ViewBasis,
    k: &StyleColors,
) -> [f32; 3] {
    let c = Vec3::from(c);
    let head = view.headlight();
    let l = n.dot(head).max(0.);
    let lit = |(a, d): (f32, f32)| c * (a + d * l);
    let rgb = match style {
        Style::Wireframe | Style::Shaded => lit(SHADED),
        Style::Texture => lit(TEXTURE),
        Style::Default => lit(DEFAULT),
        Style::Advanced => {
            use operandi_core::blender_principled as bp;
            let lights = bp::Lights {
                ambient: [advanced_ambient(ADVANCED_AMBIENT); 3],
                key: advanced_key(),
            };
            let base = c.to_array().map(bp::decode);
            let lin = bp::shade(
                base,
                &bp::Params::default(),
                n.to_array(),
                to_eye.to_array(),
                head.to_array(),
                1.,
                &lights,
            );
            Vec3::from(lin.map(|v| bp::encode(v.max(0.))))
        }
        Style::Solid => c * SOLID,
        Style::VertexMap => Vec3::from(map) * (VERTEX_MAP.0 + VERTEX_MAP.1 * l),
        Style::Gooch => {
            let t = n.dot(view.to_world(GOOCH_LIGHT)).abs();
            Vec3::from(k.gooch_cool).lerp(Vec3::from(k.gooch_warm), t)
        }
        Style::Cel => {
            let cl = view.to_world(CEL_LIGHT);
            let ch = (cl + to_eye).normalize();
            let g = if n.dot(to_eye) < CEL_OUTLINE_BELOW {
                CEL_OUTLINE
            } else if n.dot(ch) > CEL_SPOT_ABOVE {
                CEL_SPOT
            } else if n.dot(cl) > CEL_LIT_ABOVE {
                CEL_LIT
            } else {
                CEL_SHADE
            };
            Vec3::splat(f32::from(g) / 255.)
        }
        Style::Reflection => {
            let v = view.to_view(n);
            Vec3::from(crate::matcap::sample(k.matcap, v.x, v.y))
        }
    };
    rgb.clamp(Vec3::ZERO, Vec3::ONE).to_array()
}

/// Rec. 709 weights for a colour's brightness.
const LUMA: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);
/// Gooch, Cel and Reflection: a highlight's brightness follows the style
/// only from this floor up (their dark tones would bury it). Operandi's
/// own value, by eye.
pub const MARKED_FLOOR: f32 = 0.45;

/// `shade` for a highlighted surface (the shader's `shade_marked`): `c` is
/// the highlight colour (selection, symmetry selection, locked or
/// rollover). The style shades a plain grey surface first, then the
/// highlight goes on top, scaled by how bright the style made the grey, so
/// its light, bands and tones still show (from `MARKED_FLOOR` up in Gooch,
/// Cel and Reflection). In the lit styles that is the highlight lit as a
/// surface colour. Vertex Map is shaded as it is (it
/// tints the map itself).
#[allow(clippy::too_many_arguments)]
pub fn shade_marked(
    style: Style,
    c: [f32; 3],
    map: [f32; 3],
    n: Vec3,
    to_eye: Vec3,
    view: &ViewBasis,
    k: &StyleColors,
    marked: bool,
) -> [f32; 3] {
    if !marked || style == Style::VertexMap {
        return shade(style, c, map, n, to_eye, view, k);
    }
    let grey = Vec3::splat(GREY / 255.);
    let styled = Vec3::from(shade(style, grey.to_array(), map, n, to_eye, view, k));
    let mut scale = styled.dot(LUMA) / grey.dot(LUMA);
    if matches!(style, Style::Gooch | Style::Cel | Style::Reflection) {
        scale = MARKED_FLOOR + (1. - MARKED_FLOOR) * scale;
    }
    (Vec3::from(c) * scale)
        .clamp(Vec3::ZERO, Vec3::ONE)
        .to_array()
}

/// `shade` for the projected path: one colour per face from its world
/// normal (flipped toward the eye, as the shader does), with an opaque
/// Color32 in and out. Vertex Map previews stay neutral (the theme's No
/// Vertex Map colour). `marked`: `base` is a highlight colour
/// (`shade_marked`).
pub fn face_color(
    style: Style,
    base: Color32,
    normal: Vec3,
    to_eye: Vec3,
    view: &ViewBasis,
    k: &StyleColors,
    marked: bool,
) -> Color32 {
    let to_eye = to_eye.normalize_or_zero();
    let mut n = normal.normalize_or_zero();
    if n == Vec3::ZERO {
        n = to_eye;
    }
    if n.dot(to_eye) < 0. {
        n = -n;
    }
    let map = unit(crate::theme::viewport::vertex_map_none());
    let [r, g, b] = shade_marked(style, unit(base), map, n, to_eye, view, k, marked)
        .map(|v| (v * 255. + 0.5).clamp(0., 255.) as u8);
    Color32::from_rgb(r, g, b)
}

#[cfg(test)]
#[path = "display_shading_tests.rs"]
mod tests;
