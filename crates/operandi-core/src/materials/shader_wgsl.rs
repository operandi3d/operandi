//! The shader tree's folded value (`shader_eval::Expr`) as WGSL, so the
//! realtime viewport can draw image layers per pixel with the same sums the
//! renderer and `Expr::eval_at` do. There is one evaluator of the tree (the
//! fold); this only writes its result out as shader code.
//!
//! Every number in the expression (colours, locator matrices, repeats,
//! opacities, gradient stops…) goes into a parameter list the caller uploads
//! as a storage buffer, so the generated code depends only on the
//! expression's shape: moving a Texture Locator or scrubbing a layer's
//! opacity rewrites parameters and never recompiles a shader.
//!
//! The code reads these names, which the caller declares:
//!
//! - `op_params: array<vec4<f32>>` (storage, read), the parameters;
//! - `op_img0` … `op_img{MAX_IMAGES - 1}`: `texture_2d<f32>`, linear
//!   colour (sRGB textures for 8-bit files), row 0 at the top;
//! - samplers `op_smp_{n|l}_{r|m|c}`: nearest / linear filtering with
//!   repeat / mirror-repeat / clamp-to-edge addressing.
//!
//! Generated functions take three shading points (`OpSp`: the point itself
//! and the point one pixel right and one pixel down, from the fragment's
//! derivatives) so image lookups pick their mip level from how fast their
//! coordinates change, without implicit derivatives (which WGSL forbids in
//! the per-polygon branch that chooses a material).
use crate::shader_eval::{Expr, FaceKind, GeometryInput, ImageLookup, Mix};
use crate::shader_tree::Filtering;
use crate::texture_locator::{FalloffType, Tiling};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Image textures one shader can read (wgpu's default limit is 16 sampled
/// textures per stage; the viewport's shadow map takes one).
pub const MAX_IMAGES: usize = 12;

/// The placeholder for a generated function's name in [`Program::body`].
pub const NAME: &str = "op_m__";

/// Past this many statements a material falls back to the CPU colour (a
/// runaway tree would make a shader too slow to compile).
const MAX_STATEMENTS: usize = 4000;

/// One expression as WGSL.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// `fn op_m__(s0: OpSp, s1: OpSp, s2: OpSp, b: u32) -> vec3<f32>` with
    /// parameters at `op_params[b + i]`; the name is [`NAME`].
    pub body: String,
    pub params: Vec<[f32; 4]>,
    /// The UV map MeshUv reads: the first named one met, as the renderer
    /// picks it (`None`: the polygon's own UVs, else the first map).
    pub uv_map: Option<String>,
    /// The one image-free subexpression the GPU can't evaluate (a
    /// procedural pattern), evaluated per mesh corner and passed in as
    /// `OpSp::slot` (0..1, interpolated across the polygon).
    pub slot: Option<Expr>,
    /// Whether the value reads any image (else the CPU colour path is as
    /// good and cheaper).
    pub reads_images: bool,
    /// Whether it reads the view (a Gradient on Incidence): like an image,
    /// it changes across a polygon, so the CPU colour can't stand in.
    pub reads_view: bool,
    /// Whether it runs a graph layer's node sums (`Expr::Func`): they read
    /// positions, normals and procedural textures that change across a
    /// polygon, so they draw per pixel too (MaterialX stage 5).
    pub reads_graph: bool,
    /// Whether it evaluates a procedural pattern (pattern.wgsl): the
    /// pattern changes across a polygon, so per pixel shows it where the
    /// per-corner colour smears it (a low-poly mesh showed next to nothing).
    pub reads_pattern: bool,
}
impl Program {
    /// Whether the GPU program is worth running instead of the CPU colour.
    pub fn per_pixel(&self) -> bool {
        self.reads_images || self.reads_view || self.reads_graph || self.reads_pattern
    }
}

/// `e` as WGSL, or `None` when it can't be (more than one procedural
/// pattern, more images than `image` hands out slots for, too large).
/// `image` gives an image's texture slot (0..MAX_IMAGES).
pub fn translate(e: &Expr, image: &mut dyn FnMut(u64) -> Option<u32>) -> Option<Program> {
    let mut g = Gen {
        lines: Vec::new(),
        memo: HashMap::new(),
        params: Vec::new(),
        param_memo: HashMap::new(),
        uv_map: None,
        slot: None,
        image,
        failed: false,
        reads_images: false,
        reads_view: false,
        reads_graph: false,
        reads_pattern: false,
    };
    let out = g.emit(e, 0);
    if g.failed || g.lines.len() > MAX_STATEMENTS {
        return None;
    }
    let mut body = format!("fn {NAME}(s0: OpSp, s1: OpSp, s2: OpSp, b: u32) -> vec3<f32> {{\n");
    for line in &g.lines {
        let _ = writeln!(body, "  {line}");
    }
    let _ = writeln!(body, "  return {out};\n}}");
    Some(Program {
        body,
        params: g.params,
        uv_map: g.uv_map,
        slot: g.slot,
        reads_images: g.reads_images,
        reads_view: g.reads_view,
        reads_graph: g.reads_graph,
        reads_pattern: g.reads_pattern,
    })
}

/// Values a list function returns ([`translate_list`]).
pub const LIST_LEN: usize = 16;
/// The placeholder for a list function's name in [`Program::body`].
pub const LIST_NAME: &str = "op_l__";

/// Several expressions, each at one of the three shading points (0 the
/// point, 1 and 2 a pixel across), as one function
/// `fn op_l__(s0: OpSp, s1: OpSp, s2: OpSp, b: u32) ->
/// array<vec3<f32>, LIST_LEN>` (entries past the list are 0), sharing
/// temporaries and parameters. `None` as for [`translate`], and also when
/// an item needs a procedural pattern (the per-corner slot belongs to the
/// colour) or there are more than [`LIST_LEN`] items.
pub fn translate_list(
    items: &[(Expr, u8)],
    image: &mut dyn FnMut(u64) -> Option<u32>,
) -> Option<Program> {
    if items.len() > LIST_LEN {
        return None;
    }
    let mut g = Gen {
        lines: Vec::new(),
        memo: HashMap::new(),
        params: Vec::new(),
        param_memo: HashMap::new(),
        uv_map: None,
        slot: None,
        image,
        failed: false,
        reads_images: false,
        reads_view: false,
        reads_graph: false,
        reads_pattern: false,
    };
    let outs: Vec<String> = items.iter().map(|(e, s)| g.emit(e, (*s).min(2))).collect();
    if g.failed || g.slot.is_some() || g.lines.len() > MAX_STATEMENTS {
        return None;
    }
    let mut body = format!(
        "fn {LIST_NAME}(s0: OpSp, s1: OpSp, s2: OpSp, b: u32) -> array<vec3<f32>, {LIST_LEN}> {{\n"
    );
    for line in &g.lines {
        let _ = writeln!(body, "  {line}");
    }
    let _ = writeln!(body, "  var out: array<vec3<f32>, {LIST_LEN}>;");
    for (k, o) in outs.iter().enumerate() {
        let _ = writeln!(body, "  out[{k}] = {o};");
    }
    let _ = writeln!(body, "  return out;\n}}");
    Some(Program {
        body,
        params: g.params,
        uv_map: g.uv_map,
        slot: None,
        reads_images: g.reads_images,
        reads_view: g.reads_view,
        reads_graph: g.reads_graph,
        reads_pattern: g.reads_pattern,
    })
}

/// The shared helpers, the generated functions (each body's [`NAME`]
/// replaced by `op_m{index + 1}`) and the dispatcher
/// `op_material(code, s0, s1, s2, base)`, code 0 giving black, with an
/// empty list dispatcher (see [`module_with_lists`]).
pub fn module(bodies: &[String]) -> String {
    module_with_lists(bodies, &[])
}

/// [`module`] plus list functions ([`translate_list`], named
/// `op_l{index + 1}`) and their dispatcher `op_list(code, s0, s1, s2,
/// base)`, code 0 giving zeros.
pub fn module_with_lists(bodies: &[String], lists: &[String]) -> String {
    let mut out = String::from(PRELUDE);
    out.push_str(PATTERN_WGSL);
    // Cycles' Voronoi and hashes: the Cellular patterns use them.
    out.push_str(crate::shader_func::VORONOI_WGSL);
    // Graph layers' node sums, only when a program calls them (they are
    // long, and most materials don't).
    if bodies.iter().chain(lists).any(|b| b.contains("op_g_")) {
        out.push_str(crate::shader_func::WGSL);
    }
    for (i, body) in bodies.iter().enumerate() {
        out.push_str(&body.replace(NAME, &format!("op_m{}", i + 1)));
        out.push('\n');
    }
    for (i, body) in lists.iter().enumerate() {
        out.push_str(&body.replace(LIST_NAME, &format!("op_l{}", i + 1)));
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "fn op_list(code: u32, s0: OpSp, s1: OpSp, s2: OpSp, b: u32) -> array<vec3<f32>, {LIST_LEN}> {{\n  switch code {{"
    );
    for i in 0..lists.len() {
        let _ = writeln!(
            out,
            "    case {k}u: {{ return op_l{k}(s0, s1, s2, b); }}",
            k = i + 1
        );
    }
    let _ = writeln!(
        out,
        "    default: {{ return array<vec3<f32>, {LIST_LEN}>(); }}\n  }}\n}}"
    );
    out.push_str(
        "fn op_material(code: u32, s0: OpSp, s1: OpSp, s2: OpSp, b: u32) -> vec3<f32> {\n  switch code {\n",
    );
    for i in 0..bodies.len() {
        let _ = writeln!(
            out,
            "    case {k}u: {{ return op_m{k}(s0, s1, s2, b); }}",
            k = i + 1
        );
    }
    out.push_str("    default: { return vec3<f32>(0.0); }\n  }\n}\n");
    out
}

struct Gen<'a> {
    lines: Vec<String>,
    /// Structurally equal subexpressions share one temporary (the fold
    /// clones the layer below into several places of a blend).
    memo: HashMap<String, String>,
    params: Vec<[f32; 4]>,
    param_memo: HashMap<[u32; 4], usize>,
    uv_map: Option<String>,
    slot: Option<Expr>,
    image: &'a mut dyn FnMut(u64) -> Option<u32>,
    failed: bool,
    reads_images: bool,
    reads_view: bool,
    reads_graph: bool,
    reads_pattern: bool,
}

const AXES: [&str; 3] = ["x", "y", "z"];

impl Gen<'_> {
    /// A temporary holding `code` (a `vec3<f32>` unless `ty` says else),
    /// shared with an earlier one of the same `key`.
    fn bind(&mut self, key: String, ty: &str, code: String) -> String {
        if let Some(t) = self.memo.get(&key) {
            return t.clone();
        }
        let name = format!("t{}", self.lines.len());
        self.lines.push(format!("let {name}: {ty} = {code};"));
        self.memo.insert(key, name.clone());
        name
    }
    /// The parameter row holding `v`, as WGSL.
    fn param(&mut self, v: [f64; 4]) -> String {
        let v = v.map(|x| x as f32);
        let bits = v.map(f32::to_bits);
        let i = *self.param_memo.entry(bits).or_insert_with(|| {
            self.params.push(v);
            self.params.len() - 1
        });
        format!("op_params[b + {i}u]")
    }
    /// Rows that must stay together (a gradient's table), not shared.
    fn param_block(&mut self, rows: impl IntoIterator<Item = [f64; 4]>) -> usize {
        let start = self.params.len();
        self.params
            .extend(rows.into_iter().map(|r| r.map(|x| x as f32)));
        start
    }
    /// The value of `e` at shading point `s` (0 the point, 1 and 2 the
    /// points a pixel across), as the name of a `vec3<f32>`.
    fn emit(&mut self, e: &Expr, s: u8) -> String {
        if self.failed {
            return "vec3<f32>(0.0)".into();
        }
        let sp = format!("s{s}");
        match e {
            // A free node's output draws as its lowered expression (NE-03).
            Expr::Nodes(n) => self.emit(&n.lowered, s),
            Expr::Const(c) => {
                let p = self.param([c[0], c[1], c[2], 0.]);
                self.bind(format!("c{p}"), "vec3<f32>", format!("{p}.xyz"))
            }
            Expr::ImageColor(l) => {
                let t = self.sample(l, s);
                self.bind(format!("rgb|{t}"), "vec3<f32>", format!("{t}.rgb"))
            }
            Expr::ImageAlpha(l) => {
                let t = self.sample(l, s);
                self.bind(format!("a|{t}"), "vec3<f32>", format!("vec3<f32>({t}.a)"))
            }
            Expr::Locator(m) => {
                let r: Vec<String> = (0..3)
                    .map(|k| self.param([m[k * 4], m[k * 4 + 1], m[k * 4 + 2], m[k * 4 + 3]]))
                    .collect();
                let code = format!("op_affine({}, {}, {}, {sp}.p)", r[0], r[1], r[2]);
                self.bind(
                    format!("loc|{}|{}|{}|{sp}", r[0], r[1], r[2]),
                    "vec3<f32>",
                    code,
                )
            }
            Expr::Planar {
                of,
                axis,
                centre,
                repeat,
                mirror,
            } => {
                let l = self.emit(of, s);
                let (mut u, mut v) = match axis {
                    0 => (format!("-{l}.z"), format!("{l}.y")),
                    1 => (format!("{l}.x"), format!("-{l}.z")),
                    _ => (format!("{l}.x"), format!("{l}.y")),
                };
                if mirror & 1 != 0 {
                    u = format!("-({u})");
                }
                if mirror & 2 != 0 {
                    v = format!("-({v})");
                }
                let q = self.param([*centre, repeat[0], repeat[1], 0.]);
                let code = format!("vec3<f32>(({u} + {q}.x) * {q}.y, ({v} + {q}.x) * {q}.z, 0.0)");
                self.bind(format!("pl|{axis}|{mirror}|{l}|{q}"), "vec3<f32>", code)
            }
            Expr::Wrap {
                of,
                axis,
                spherical,
                centre,
                repeat,
            } => {
                let l = self.emit(of, s);
                let a = usize::from((*axis).min(2));
                let (f, t, a) = (AXES[(a + 1) % 3], AXES[(a + 2) % 3], AXES[a]);
                let u0 = format!("atan2({l}.{t}, {l}.{f}) / 6.283185307179586");
                let v0 = if *spherical {
                    format!(
                        "atan2({l}.{a}, length(vec2<f32>({l}.{f}, {l}.{t}))) / 3.141592653589793"
                    )
                } else {
                    format!("{l}.{a}")
                };
                let q = self.param([*centre, repeat[0], repeat[1], 0.]);
                let code =
                    format!("vec3<f32>(({u0} + {q}.x) * {q}.y, ({v0} + {q}.x) * {q}.z, 0.0)");
                self.bind(format!("wr|{a}|{spherical}|{l}|{q}"), "vec3<f32>", code)
            }
            Expr::Probe { of, axis, repeat } => {
                // `shader_eval::probe_coords`.
                let l = self.emit(of, s);
                let a = usize::from((*axis).min(2));
                let (r, u, z) = (AXES[(a + 2) % 3], AXES[a], AXES[(a + 1) % 3]);
                let q = self.param([repeat[0], repeat[1], 0., 0.]);
                let code = format!("op_probe(vec3<f32>({l}.{r}, {l}.{u}, {l}.{z})) * {q}.xy");
                self.bind(
                    format!("pr|{a}|{l}|{q}"),
                    "vec3<f32>",
                    format!("vec3<f32>({code}, 0.0)"),
                )
            }
            Expr::Normal(m) => {
                let r: Vec<String> = (0..3)
                    .map(|k| self.param([m[k * 3], m[k * 3 + 1], m[k * 3 + 2], 0.]))
                    .collect();
                let code = format!(
                    "op_normalize_or_keep(op_affine({}, {}, {}, {sp}.n))",
                    r[0], r[1], r[2]
                );
                self.bind(
                    format!("n|{}|{}|{}|{sp}", r[0], r[1], r[2]),
                    "vec3<f32>",
                    code,
                )
            }
            Expr::FaceWeight { normal, face, kind } => {
                let n = self.emit(normal, s);
                let code = match kind {
                    FaceKind::Signed => format!(
                        "vec3<f32>(op_face_signed({n}, {}, {}))",
                        face % 3,
                        face < &3
                    ),
                    FaceKind::Dominant => {
                        format!("vec3<f32>(select(0.0, 1.0, op_dominant({n}) == {}))", face)
                    }
                    FaceKind::Power(k) => {
                        let q = self.param([*k, 0., 0., 0.]);
                        format!("vec3<f32>(op_face_power({n}, {}, {q}.x))", (*face).min(2))
                    }
                };
                self.bind(format!("fw|{face}|{kind:?}|{n}"), "vec3<f32>", code)
            }
            Expr::Facing(normal, axis) => {
                let n = self.emit(normal, s);
                let a = AXES[usize::from((*axis).min(2))];
                self.bind(
                    format!("fc|{a}|{n}"),
                    "vec3<f32>",
                    format!("vec3<f32>(select(0.0, 1.0, {n}.{a} > 0.0))"),
                )
            }
            Expr::Falloff { of, kind, amount } => {
                let l = self.emit(of, s);
                let q = self.param([amount[0], amount[1], amount[2], 0.]);
                let d = format!("abs({l} * {q}.xyz)");
                let reach = match kind {
                    FalloffType::Cubic => format!("op_max3({d})"),
                    FalloffType::Spherical => format!("length({d})"),
                    FalloffType::LinearX => format!("{d}.x"),
                    FalloffType::LinearY => format!("{d}.y"),
                    FalloffType::LinearZ => format!("{d}.z"),
                };
                self.bind(
                    format!("fo|{kind:?}|{l}|{q}"),
                    "vec3<f32>",
                    format!("vec3<f32>(clamp(1.0 - {reach}, 0.0, 1.0))"),
                )
            }
            Expr::MeshUv {
                map,
                repeat,
                rotation,
            } => {
                if self.uv_map.is_none() {
                    self.uv_map = map.clone();
                }
                let (sin, cos) = if *rotation == 0. {
                    (0., 1.)
                } else {
                    rotation.to_radians().sin_cos()
                };
                let q = self.param([repeat[0], repeat[1], sin, cos]);
                let code = format!("op_uv({sp}.uv, {q})");
                self.bind(format!("uv|{q}|{sp}"), "vec3<f32>", code)
            }
            Expr::Tile(a, tiling) => {
                let v = self.emit(a, s);
                let m = tiling.map(tiling_code);
                self.bind(
                    format!("ti|{m:?}|{v}"),
                    "vec3<f32>",
                    format!(
                        "vec3<f32>(op_tile({v}.x, {}), op_tile({v}.y, {}), {v}.z)",
                        m[0], m[1]
                    ),
                )
            }
            Expr::Inside(a, clip) => {
                let v = self.emit(a, s);
                let test = |k: usize| {
                    if clip[k] {
                        let c = AXES[k];
                        format!("({v}.{c} >= 0.0 && {v}.{c} <= 1.0)")
                    } else {
                        "true".to_string()
                    }
                };
                let code = format!("vec3<f32>(select(0.0, 1.0, {} && {}))", test(0), test(1));
                self.bind(format!("in|{clip:?}|{v}"), "vec3<f32>", code)
            }
            // Per pixel (VM-Q11): pattern.wgsl, from Pattern::packed.
            Expr::Pattern(l) | Expr::CellRandom(l) => {
                self.reads_pattern = true;
                let v = self.emit(&l.vector, s);
                let (mode, k) = l.pattern.packed(l.axis);
                let k = k.map(f64::from);
                let q: Vec<String> = (0..3)
                    .map(|r| self.param([k[r * 4], k[r * 4 + 1], k[r * 4 + 2], k[r * 4 + 3]]))
                    .collect();
                let (tag, code) = if matches!(e, Expr::Pattern(_)) {
                    (
                        "pt",
                        format!(
                            "vec3<f32>(op_pat({mode}u, {}, {}, {}, {v}))",
                            q[0], q[1], q[2]
                        ),
                    )
                } else {
                    (
                        "cr",
                        format!("op_cell_random({mode}u, {}, {}, {}, {v})", q[0], q[1], q[2]),
                    )
                };
                self.bind(
                    format!("{tag}|{mode}|{}|{}|{}|{v}", q[0], q[1], q[2]),
                    "vec3<f32>",
                    code,
                )
            }
            Expr::BiasGain(a, bias, gain) => {
                let v = self.emit(a, s);
                let q = self.param([*bias, *gain, 0., 0.]);
                self.bind(
                    format!("bg|{v}|{q}"),
                    "vec3<f32>",
                    format!("vec3<f32>(op_gain({q}.y, op_bias({q}.x, {v}.x)))"),
                )
            }
            Expr::HsvVary {
                color,
                random,
                amounts,
            } => {
                let c = self.emit(color, s);
                let r = self.emit(random, s);
                let q = self.param([amounts[0], amounts[1], amounts[2], 0.]);
                self.bind(
                    format!("hv|{c}|{r}|{q}"),
                    "vec3<f32>",
                    format!("op_hsv_vary({c}, {r}, {q}.xyz)"),
                )
            }
            Expr::Luma(a) => {
                let v = self.emit(a, s);
                self.bind(
                    format!("lu|{v}"),
                    "vec3<f32>",
                    format!("vec3<f32>(dot({v}, vec3<f32>(0.2126, 0.7152, 0.0722)))"),
                )
            }
            Expr::Channel(a, k) => {
                let v = self.emit(a, s);
                let c = AXES[usize::from((*k).min(2))];
                self.bind(
                    format!("ch|{c}|{v}"),
                    "vec3<f32>",
                    format!("vec3<f32>({v}.{c})"),
                )
            }
            Expr::Gamma(a, p) => {
                let v = self.emit(a, s);
                let q = self.param([*p, 0., 0., 0.]);
                self.bind(
                    format!("ga|{v}|{q}"),
                    "vec3<f32>",
                    format!("op_gamma({v}, {q}.x)"),
                )
            }
            Expr::Mix { mode, fac, a, b } => {
                let f = self.emit(fac, s);
                let x = self.emit(a, s);
                let y = self.emit(b, s);
                let m = mix_name(*mode);
                self.bind(
                    format!("mx|{m}|{f}|{x}|{y}"),
                    "vec3<f32>",
                    format!("op_mix_{m}({x}, {y}, clamp({f}.x, 0.0, 1.0))"),
                )
            }
            // The viewport passes the angle to the eye per pixel
            // (`OpSp::inc`, degrees), so Incidence gradients shade across
            // a surface as they render (LXP stage A).
            Expr::Geometry(GeometryInput::Incidence) => {
                self.reads_view = true;
                self.bind(
                    format!("inc|{sp}"),
                    "vec3<f32>",
                    format!("vec3<f32>({sp}.inc)"),
                )
            }
            // As `eval_at`: a point has no other view inputs, no mesh
            // attributes and sees nothing around it (provisional in the
            // viewport too).
            Expr::Geometry(_) | Expr::Attribute { .. } => {
                self.bind("zero".into(), "vec3<f32>", "vec3<f32>(0.0)".into())
            }
            Expr::Occlusion { .. } => self.bind("one".into(), "vec3<f32>", "vec3<f32>(1.0)".into()),
            Expr::LocatorIncidence(p) => {
                let q = self.param([p[0], p[1], p[2], 0.]);
                self.bind(
                    format!("li|{q}|{sp}"),
                    "vec3<f32>",
                    format!(
                        "vec3<f32>(degrees(acos(clamp(dot(op_normalize_or_keep({sp}.n), op_normalize_or_keep({q}.xyz - {sp}.p)), -1.0, 1.0))))"
                    ),
                )
            }
            Expr::DistanceTo(p) => {
                let q = self.param([p[0], p[1], p[2], 0.]);
                self.bind(
                    format!("di|{q}|{sp}"),
                    "vec3<f32>",
                    format!("vec3<f32>(length({sp}.p - {q}.xyz))"),
                )
            }
            Expr::Hsv {
                of,
                hue_scale,
                hue_shift,
                saturation_scale,
            } => {
                let v = self.emit(of, s);
                let q = self.param([*hue_scale, *hue_shift / 360., *saturation_scale, 0.]);
                self.bind(
                    format!("hs|{v}|{q}"),
                    "vec3<f32>",
                    format!("op_hsv({v}, {q}.xyz)"),
                )
            }
            Expr::Ramp {
                input,
                lo,
                hi,
                stops,
                alpha,
            } => {
                let x = self.emit(input, s);
                let h = self.param_block(
                    std::iter::once([*lo, *hi, stops.len() as f64, 0.])
                        .chain(stops.iter().copied()),
                );
                // The table is the ramp's own: no sharing by value.
                let r = self.bind(
                    format!("rp|{x}|{h}"),
                    "vec4<f32>",
                    format!("op_ramp({x}.x, b + {h}u)"),
                );
                self.bind(
                    format!("rs|{r}|{alpha}"),
                    "vec3<f32>",
                    if *alpha {
                        format!("vec3<f32>({r}.a)")
                    } else {
                        format!("{r}.rgb")
                    },
                )
            }
            // Graph layers' node sums (shader_func.wgsl).
            Expr::Func(c) => {
                self.reads_graph = true;
                let args: Vec<String> = c.args.iter().map(|a| self.emit(a, s)).collect();
                if args.len() < c.func.arity() {
                    self.failed = true;
                    return "vec3<f32>(0.0)".into();
                }
                let code = c.func.wgsl(&args);
                self.bind(
                    format!("fn|{:?}|{}", c.func, args.join("|")),
                    "vec3<f32>",
                    code,
                )
            }
        }
    }
    /// An image's RGBA at a lookup (a `vec4<f32>` name).
    fn sample(&mut self, l: &ImageLookup, s: u8) -> String {
        let Some(slot) = (self.image)(l.image).filter(|&k| (k as usize) < MAX_IMAGES) else {
            self.failed = true;
            return "vec4<f32>(0.0)".into();
        };
        self.reads_images = true;
        let filter = match l.filtering {
            Filtering::Nearest => "n",
            // Bicubic draws bilinear in the viewport (provisional).
            Filtering::Bilinear | Filtering::Bicubic => "l",
        };
        let (address, clip) = match l.extension {
            Tiling::Repeat => ("r", false),
            Tiling::Mirror => ("m", false),
            Tiling::Edge => ("c", false),
            Tiling::Reset => ("c", true),
        };
        let v0 = self.emit(&l.vector, s);
        let code = if s == 0 {
            // How fast the coordinates change across a pixel: the mip level.
            let vx = self.emit(&l.vector, 1);
            let vy = self.emit(&l.vector, 2);
            format!(
                "op_tex(op_img{slot}, op_smp_{filter}_{address}, {v0}.xy, {vx}.xy - {v0}.xy, {vy}.xy - {v0}.xy, {clip})"
            )
        } else {
            format!(
                "op_tex(op_img{slot}, op_smp_{filter}_{address}, {v0}.xy, vec2<f32>(0.0), vec2<f32>(0.0), {clip})"
            )
        };
        self.bind(
            format!("im|{slot}|{filter}|{address}|{clip}|{v0}|{s}"),
            "vec4<f32>",
            code,
        )
    }
}

fn tiling_code(t: Tiling) -> u32 {
    match t {
        Tiling::Reset => 0,
        Tiling::Repeat => 1,
        Tiling::Mirror => 2,
        Tiling::Edge => 3,
    }
}

fn mix_name(m: Mix) -> &'static str {
    match m {
        Mix::Blend => "blend",
        Mix::Add => "add",
        Mix::Multiply => "multiply",
        Mix::Subtract => "subtract",
        Mix::Screen => "screen",
        Mix::Divide => "divide",
        Mix::Difference => "difference",
        Mix::Darken => "darken",
        Mix::Lighten => "lighten",
        Mix::Overlay => "overlay",
        Mix::Dodge => "dodge",
        Mix::Burn => "burn",
        Mix::SoftLight => "soft_light",
    }
}

/// Helpers the generated functions call: the same sums as `shader_eval`,
/// `texture_locator` and `process_occlusion`, in f32.
/// The procedural patterns per pixel (`op_pat`, `op_cell_random`): a port
/// of procedural.rs (VM-Q11).
const PATTERN_WGSL: &str = include_str!("pattern.wgsl");

const PRELUDE: &str = r#"
struct OpSp { p: vec3<f32>, uv: vec2<f32>, n: vec3<f32>, slot: vec3<f32>, inc: f32 };
fn op_affine(r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>, p: vec3<f32>) -> vec3<f32> {
  return vec3<f32>(dot(r0.xyz, p) + r0.w, dot(r1.xyz, p) + r1.w, dot(r2.xyz, p) + r2.w);
}
fn op_normalize_or_keep(v: vec3<f32>) -> vec3<f32> {
  let l = length(v);
  if (l > 0.0) { return v / l; }
  return v;
}
fn op_max3(v: vec3<f32>) -> f32 { return max(v.x, max(v.y, v.z)); }
// X on ties with Y or Z, Y on ties with Z (shader_eval::dominant_axis).
fn op_dominant(n: vec3<f32>) -> u32 {
  let a = abs(n);
  if (a.x >= a.y && a.x >= a.z) { return 0u; }
  if (a.y >= a.z) { return 1u; }
  return 2u;
}
// Light Probe: a direction (right, up, toward the viewer) on a mirror ball.
fn op_probe(l: vec3<f32>) -> vec2<f32> {
    let len = length(l);
    if (len <= 0.0) { return vec2<f32>(0.5); }
    let d = l / len + vec3<f32>(0.0, 0.0, 1.0);
    let n = length(d);
    if (n <= 1e-6) { return vec2<f32>(0.5, 0.0); }
    return vec2<f32>(0.5) + 0.5 * d.xy / n;
}
fn op_face_signed(n: vec3<f32>, axis: u32, positive: bool) -> f32 {
  let on = op_dominant(n) == axis && (n[axis] >= 0.0) == positive;
  return select(0.0, 1.0, on);
}
fn op_face_power(n: vec3<f32>, face: u32, k: f32) -> f32 {
  let w = pow(abs(n), vec3<f32>(k));
  let total = w.x + w.y + w.z;
  if (total > 0.0) { return w[face] / total; }
  return 1.0 / 3.0;
}
fn op_uv(uv: vec2<f32>, q: vec4<f32>) -> vec3<f32> {
  let u = uv.x * q.x;
  let v = uv.y * q.y;
  return vec3<f32>(u * q.w - v * q.z, u * q.z + v * q.w, 0.0);
}
// texture_locator::tile: 0 Reset (as is), 1 Repeat, 2 Mirror, 3 Edge.
fn op_tile(t: f32, mode: u32) -> f32 {
  switch mode {
    case 1u: { return t - floor(t); }
    case 2u: {
      let f = (t * 0.5 - floor(t * 0.5)) * 2.0;
      return select(f, 2.0 - f, f > 1.0);
    }
    case 3u: { return clamp(t, 0.0, 1.0); }
    default: { return t; }
  }
}
// An image read at raw coordinates (v up; texture rows run down) with the
// sampler's addressing as the image's extension; `clip`: Reset, transparent
// black outside 0..1.
fn op_tex(t: texture_2d<f32>, sm: sampler, uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, clip: bool) -> vec4<f32> {
  let q = vec2<f32>(uv.x, 1.0 - uv.y);
  let c = textureSampleGrad(t, sm, q, vec2<f32>(dx.x, -dx.y), vec2<f32>(dy.x, -dy.y));
  let inside = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
  return select(c, vec4<f32>(0.0), clip && !inside);
}
fn op_bias(b0: f32, t0: f32) -> f32 {
  let b = clamp(b0, 1e-6, 1.0 - 1e-6);
  let t = clamp(t0, 0.0, 1.0);
  if (t == 0.0) { return 0.0; }
  return pow(t, log(b) / log(0.5));
}
fn op_gain(g0: f32, t0: f32) -> f32 {
  let t = clamp(t0, 0.0, 1.0);
  let g = 1.0 - clamp(g0, 1e-6, 1.0 - 1e-6);
  if (t < 0.5) { return op_bias(g, 2.0 * t) / 2.0; }
  return 1.0 - op_bias(g, 2.0 - 2.0 * t) / 2.0;
}
fn op_rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
  let mx = max(c.x, max(c.y, c.z));
  let mn = min(c.x, min(c.y, c.z));
  let d = mx - mn;
  var s = 0.0;
  if (mx > 0.0) { s = d / mx; }
  var h = 0.0;
  if (d <= 0.0) {
    h = 0.0;
  } else if (mx == c.x) {
    let x = (c.y - c.z) / d;
    h = (x - 6.0 * floor(x / 6.0)) / 6.0;
  } else if (mx == c.y) {
    h = ((c.z - c.x) / d + 2.0) / 6.0;
  } else {
    h = ((c.x - c.y) / d + 4.0) / 6.0;
  }
  return vec3<f32>(h, s, mx);
}
fn op_hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
  let h6 = (hsv.x - floor(hsv.x)) * 6.0;
  let i = floor(h6);
  let f = h6 - i;
  let v = hsv.z;
  let s = hsv.y;
  let p = v * (1.0 - s);
  let q = v * (1.0 - s * f);
  let t = v * (1.0 - s * (1.0 - f));
  switch i32(i) {
    case 0: { return vec3<f32>(v, t, p); }
    case 1: { return vec3<f32>(q, v, p); }
    case 2: { return vec3<f32>(p, v, t); }
    case 3: { return vec3<f32>(p, q, v); }
    case 4: { return vec3<f32>(t, p, v); }
    default: { return vec3<f32>(v, p, q); }
  }
}
fn op_hsv_vary(c: vec3<f32>, r: vec3<f32>, a: vec3<f32>) -> vec3<f32> {
  if (all(a == vec3<f32>(0.0))) { return c; }
  let hsv = op_rgb_to_hsv(c);
  let x = hsv.x + a.y * (r.y - 0.5);
  let h = x - floor(x);
  let s = clamp(hsv.y * (1.0 - a.z * r.z), 0.0, 1.0);
  let v = max(hsv.z * (1.0 - a.x * r.x), 0.0);
  return op_hsv_to_rgb(vec3<f32>(h, s, v));
}
// Process: hue × scale + shift (in turns), saturation × scale, value kept.
fn op_hsv(c: vec3<f32>, q: vec3<f32>) -> vec3<f32> {
  let hsv = op_rgb_to_hsv(c);
  let h = hsv.x * q.x + q.y;
  return op_hsv_to_rgb(vec3<f32>(h - floor(h), clamp(hsv.y * q.z, 0.0, 1.0), hsv.z));
}
fn op_gamma(a: vec3<f32>, p: f32) -> vec3<f32> {
  return select(a, pow(max(a, vec3<f32>(1e-30)), vec3<f32>(p)), a > vec3<f32>(0.0));
}
// shader_eval::ramp_at from a table in op_params at h: (lo, hi, n), stops.
fn op_ramp(x: f32, h: u32) -> vec4<f32> {
  let q = op_params[h];
  let n = u32(q.z);
  if (n == 0u) { return vec4<f32>(0.0); }
  var t = 0.0;
  if (q.y > q.x) { t = clamp((x - q.x) / (q.y - q.x), 0.0, 1.0); }
  let f = t * f32(n - 1u);
  let i = min(u32(floor(f)), n - 1u);
  let j = min(i + 1u, n - 1u);
  let w = f - f32(i);
  let a = op_params[h + 1u + i];
  return a + (op_params[h + 1u + j] - a) * w;
}
// Cycles' Mix Color modes, as shader_eval::Mix::apply.
fn op_mix_blend(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return a + (b - a) * t; }
fn op_mix_add(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return a + t * b; }
fn op_mix_multiply(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return a * (1.0 - t + t * b); }
fn op_mix_subtract(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return a - t * b; }
fn op_mix_screen(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  return 1.0 - (1.0 - t + t * (1.0 - b)) * (1.0 - a);
}
fn op_mix_divide(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  let safe = select(vec3<f32>(1.0), b, b != vec3<f32>(0.0));
  return select(a, a * (1.0 - t) + t * a / safe, b != vec3<f32>(0.0));
}
fn op_mix_difference(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return op_mix_blend(a, abs(a - b), t); }
fn op_mix_darken(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return op_mix_blend(a, min(a, b), t); }
fn op_mix_lighten(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return op_mix_blend(a, max(a, b), t); }
fn op_mix_overlay(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  let low = a * (1.0 - t + 2.0 * t * b);
  let high = 1.0 - (1.0 - t + 2.0 * t * (1.0 - b)) * (1.0 - a);
  return select(high, low, a < vec3<f32>(0.5));
}
fn op_dodge1(a: f32, b: f32, t: f32) -> f32 {
  if (a == 0.0) { return a; }
  let tmp = 1.0 - t * b;
  if (tmp <= 0.0) { return 1.0; }
  return min(a / tmp, 1.0);
}
fn op_mix_dodge(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  return vec3<f32>(op_dodge1(a.x, b.x, t), op_dodge1(a.y, b.y, t), op_dodge1(a.z, b.z, t));
}
fn op_burn1(a: f32, b: f32, t: f32) -> f32 {
  let tmp = 1.0 - t + t * b;
  if (tmp <= 0.0) { return 0.0; }
  return clamp(1.0 - (1.0 - a) / tmp, 0.0, 1.0);
}
fn op_mix_burn(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  return vec3<f32>(op_burn1(a.x, b.x, t), op_burn1(a.y, b.y, t), op_burn1(a.z, b.z, t));
}
fn op_mix_soft_light(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  let screen = 1.0 - (1.0 - b) * (1.0 - a);
  return op_mix_blend(a, (1.0 - a) * b * a + a * screen, t);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_eval::ImageLookup;

    fn image(id: u64) -> Expr {
        Expr::ImageColor(Box::new(ImageLookup::uv(id, Filtering::Bilinear)))
    }

    #[test]
    fn numbers_go_to_parameters_not_code() {
        let a = Expr::mix(
            Mix::Blend,
            Expr::scalar(0.3),
            Expr::Const([1., 0., 0.]),
            image(7),
        );
        let b = Expr::mix(
            Mix::Blend,
            Expr::scalar(0.6),
            Expr::Const([0., 1., 0.]),
            image(7),
        );
        let mut slots = |_| Some(0);
        let pa = translate(&a, &mut slots).unwrap();
        let pb = translate(&b, &mut slots).unwrap();
        assert_eq!(pa.body, pb.body);
        assert_ne!(pa.params, pb.params);
        assert!(pa.reads_images);
    }

    #[test]
    fn shared_subexpressions_are_written_once() {
        let below = Expr::mix(
            Mix::Multiply,
            Expr::scalar(0.5),
            image(1),
            Expr::Const([0.2; 3]),
        );
        let e = Expr::mix(Mix::Blend, Expr::scalar(0.5), below.clone(), below);
        let p = translate(&e, &mut |_| Some(0)).unwrap();
        assert_eq!(p.body.matches("op_tex(").count(), 1);
    }

    #[test]
    fn images_without_a_slot_fall_back() {
        let mut slots = |id: u64| (id < 2).then_some(id as u32);
        assert!(translate(&image(5), &mut slots).is_none());
        assert!(translate(&image(1), &mut slots).is_some());
    }

    #[test]
    fn uv_map_is_the_first_named_one() {
        let lookup = ImageLookup {
            vector: Expr::mesh_uv(Some("Second".into()), [2., 2.], 0.),
            ..ImageLookup::uv(1, Filtering::Nearest)
        };
        let p = translate(&Expr::ImageColor(Box::new(lookup)), &mut |_| Some(0)).unwrap();
        assert_eq!(p.uv_map.as_deref(), Some("Second"));
    }

    #[test]
    fn module_dispatches_by_code() {
        let p = translate(&image(1), &mut |_| Some(3)).unwrap();
        let m = module(&[p.body.clone(), p.body]);
        assert!(m.contains("case 2u: { return op_m2(s0, s1, s2, b); }"));
        assert!(m.contains("op_img3"));
        assert!(!m.contains(NAME));
    }
}
