//! The `operandi_*` node definitions the shader tree's MaterialX form
//! uses (tree.rs), as a MaterialX library document, with standard-node
//! implementations where MaterialX's standard library can do the same
//! sums.
//!
//! Nodes without an implementation here (procedural patterns and their
//! cell randoms, gradient ramps and geometry inputs, occlusion, the
//! general projection nodes for out-of-range numbers, the surface itself)
//! are rendered by
//! Operandi's Cycles bridge; other MaterialX readers see their definitions
//! but can't draw them yet.
//!
//! Every implementation is checked two ways in `tree_tests.rs`: each
//! standard node it uses matches a definition in the bundled MaterialX
//! 1.39.4 standard library, and evaluating it at shading points gives the
//! same numbers as the tree's own sums (`Expr::eval_at`).
use super::tree::{AXES, BOX_FACES, FALLOFFS, MIXES, SURFACE, TILINGS};
use crate::texture_locator::{FalloffType, Tiling};
use std::fmt::Write as _;

const MIX: &[(&str, &str)] = &[("bg", "color3"), ("fg", "color3"), ("mix", "color3")];
const IMAGE: &[(&str, &str)] = &[
    ("image", "string"),
    ("file", "filename"),
    ("filtertype", "string"),
    ("addressmode", "string"),
    ("texcoord", "color3"),
];
const PLANAR_AXIS: &[(&str, &str)] = &[
    ("in", "color3"),
    ("centre", "float"),
    ("repeat", "vector2"),
    ("mirror_u", "boolean"),
    ("mirror_v", "boolean"),
];
const UV: &[(&str, &str)] = &[("repeat", "vector2"), ("rotation", "float")];
const UV_MAP: &[(&str, &str)] = &[
    ("map", "string"),
    ("repeat", "vector2"),
    ("rotation", "float"),
];

/// The nodes with fixed names and their inputs (name, type). The output
/// is `out`, `color3` (`surfaceshader` for the surface).
const FIXED: &[(&str, &[(&str, &str)])] = &[
    ("operandi_attribute", &[("attribute", "string")]),
    ("operandi_attribute_alpha", &[("attribute", "string")]),
    (
        "operandi_bias_gain",
        &[("in", "color3"), ("bias", "float"), ("gain", "float")],
    ),
    (
        "operandi_cell_random",
        &[("in", "color3"), ("axis", "integer"), ("pattern", "string")],
    ),
    (
        "operandi_channel",
        &[("in", "color3"), ("channel", "integer")],
    ),
    ("operandi_constant", &[("value", "vector3")]),
    ("operandi_distance", &[("position", "vector3")]),
    (
        "operandi_face_weight",
        &[
            ("in", "color3"),
            ("face", "integer"),
            ("kind", "string"),
            ("power", "float"),
        ],
    ),
    ("operandi_facing", &[("in", "color3"), ("axis", "integer")]),
    ("operandi_gamma", &[("in", "color3"), ("power", "float")]),
    ("operandi_geometry", &[("kind", "string")]),
    (
        "operandi_hsv",
        &[
            ("in", "color3"),
            ("hue_scale", "float"),
            ("hue_shift", "float"),
            ("saturation_scale", "float"),
        ],
    ),
    (
        "operandi_hsv_vary",
        &[
            ("in", "color3"),
            ("random", "color3"),
            ("amounts", "vector3"),
        ],
    ),
    ("operandi_image", IMAGE),
    ("operandi_image_alpha", IMAGE),
    (
        "operandi_inside",
        &[
            ("in", "color3"),
            ("horizontal", "boolean"),
            ("vertical", "boolean"),
        ],
    ),
    (
        "operandi_locator",
        &[
            ("row0", "vector4"),
            ("row1", "vector4"),
            ("row2", "vector4"),
        ],
    ),
    ("operandi_locator_incidence", &[("position", "vector3")]),
    ("operandi_luma", &[("in", "color3")]),
    (
        "operandi_node_material",
        &[
            ("layer", "string"),
            ("graph", "string"),
            ("coverage", "color3"),
            ("values", "string"),
            ("driver_names", "string"),
        ],
    ),
    (
        "operandi_normal",
        &[
            ("row0", "vector3"),
            ("row1", "vector3"),
            ("row2", "vector3"),
        ],
    ),
    (
        "operandi_occlusion",
        &[
            ("rays", "integer"),
            ("distance", "float"),
            ("same_surface", "boolean"),
            ("inside", "boolean"),
            ("toward", "string"),
        ],
    ),
    (
        "operandi_pattern",
        &[("in", "color3"), ("axis", "integer"), ("pattern", "string")],
    ),
    (
        "operandi_planar",
        &[
            ("in", "color3"),
            ("axis", "integer"),
            ("centre", "float"),
            ("repeat", "vector2"),
            ("mirror", "integer"),
        ],
    ),
    (
        "operandi_probe",
        &[("in", "color3"), ("axis", "integer"), ("repeat", "vector2")],
    ),
    (
        "operandi_ramp",
        &[
            ("in", "color3"),
            ("lo", "float"),
            ("hi", "float"),
            ("stops", "floatarray"),
            ("alpha", "boolean"),
        ],
    ),
    (
        SURFACE,
        &[
            ("shader", "string"),
            ("coat_bump", "float"),
            ("bump", "color3"),
            ("normal_map", "color3"),
            ("displacement", "color3"),
            ("see_through", "string"),
            ("see_through_amount", "color3"),
            ("see_through_color", "color3"),
            ("see_through_ior", "color3"),
            ("see_through_roughness", "color3"),
            ("rounded_edge", "float"),
            ("absorption_color", "vector3"),
            ("absorption_density", "float"),
            ("scatter_color", "vector3"),
            ("scatter_density", "float"),
            ("scatter_anisotropy", "float"),
            ("thin_walled", "boolean"),
            ("glass_principled", "boolean"),
        ],
    ),
    ("operandi_texcoord", UV),
    ("operandi_texcoord_map", UV_MAP),
    (
        "operandi_wrap",
        &[
            ("in", "color3"),
            ("axis", "integer"),
            ("spherical", "boolean"),
            ("centre", "float"),
            ("repeat", "vector2"),
        ],
    ),
];

/// Every node category the form writes, with its inputs, sorted by name:
/// the fixed ones and the families with one node per mode (blends,
/// planar axes, falloff shapes, tilings).
pub fn nodes() -> Vec<(String, Vec<(&'static str, &'static str)>)> {
    let mut out: Vec<(String, Vec<(&str, &str)>)> = FIXED
        .iter()
        .map(|(c, ins)| ((*c).to_owned(), ins.to_vec()))
        .collect();
    for (_, m) in MIXES {
        out.push((format!("operandi_mix_{m}"), MIX.to_vec()));
    }
    for a in AXES {
        out.push((format!("operandi_planar_{a}"), PLANAR_AXIS.to_vec()));
    }
    for (_, f) in FALLOFFS {
        out.push((
            format!("operandi_falloff_{f}"),
            vec![("in", "color3"), ("amount", "vector3")],
        ));
    }
    for (_, h) in TILINGS {
        for (_, v) in TILINGS {
            out.push((format!("operandi_tile_{h}_{v}"), vec![("in", "color3")]));
        }
    }
    for a in AXES {
        for kind in ["cylindrical", "spherical"] {
            out.push((
                format!("operandi_wrap_{kind}_{a}"),
                vec![("in", "color3"), ("centre", "float"), ("repeat", "vector2")],
            ));
        }
        out.push((
            format!("operandi_probe_{a}"),
            vec![("in", "color3"), ("repeat", "vector2")],
        ));
        out.push((format!("operandi_cubic_{a}"), vec![("in", "color3")]));
        out.push((
            format!("operandi_triplanar_{a}"),
            vec![("in", "color3"), ("power", "float")],
        ));
    }
    for f in BOX_FACES {
        out.push((format!("operandi_box_{f}"), vec![("in", "color3")]));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Inputs every surface may also carry beyond its table entry: the
/// principled inputs (tree::INPUTS) and material values (`value_<name>`),
/// all `color3`; a Node Material's drivers are `driver_<n>`.
fn open_inputs(category: &str, input: &str) -> bool {
    match category {
        SURFACE => {
            super::tree::INPUTS.iter().any(|(_, n)| *n == input) || input.starts_with("value_")
        }
        "operandi_node_material" => input.starts_with("driver_"),
        _ => false,
    }
}

/// Whether `category` declares `input` with type `ty`.
pub fn declares(category: &str, input: &str, ty: &str) -> bool {
    nodes()
        .iter()
        .find(|(c, _)| c == category)
        .is_some_and(|(_, ins)| {
            ins.iter().any(|(n, t)| *n == input && *t == ty)
                || (ty == "color3" && open_inputs(category, input))
        })
}

fn default_of(ty: &str) -> &'static str {
    match ty {
        "color3" | "vector3" => "0, 0, 0",
        "vector4" => "0, 0, 0, 0",
        "vector2" => "0, 0",
        "float" | "integer" => "0",
        "boolean" => "false",
        _ => "",
    }
}

fn num(v: f64) -> String {
    format!("{v}")
}

/// Where a nodegraph input comes from.
#[derive(Clone)]
enum S {
    /// The node's own input (`interfacename`).
    I(&'static str),
    /// Another node in the graph.
    N(String),
    /// A value, as written.
    V(String),
}
fn f(v: f64) -> S {
    S::V(num(v))
}

/// A nodegraph body under construction; every node is named after its
/// category and a counter.
#[derive(Default)]
struct G {
    body: String,
    n: usize,
}

impl G {
    fn node(&mut self, category: &str, ty: &str, inputs: &[(&str, &str, S)]) -> S {
        self.n += 1;
        let name = format!("{category}{}", self.n);
        let _ = writeln!(self.body, "    <{category} name=\"{name}\" type=\"{ty}\">");
        for (input, t, s) in inputs {
            let (attr, v) = match s {
                S::I(i) => ("interfacename", (*i).to_owned()),
                S::N(n) => ("nodename", n.clone()),
                S::V(v) => ("value", v.clone()),
            };
            let _ = writeln!(
                self.body,
                "      <input name=\"{input}\" type=\"{t}\" {attr}=\"{v}\" />"
            );
        }
        let _ = writeln!(self.body, "    </{category}>");
        S::N(name)
    }
    fn finish(mut self, out: S, ty: &str) -> String {
        let S::N(n) = out else {
            unreachable!("a graph's output is a node")
        };
        let _ = writeln!(
            self.body,
            "    <output name=\"out\" type=\"{ty}\" nodename=\"{n}\" />"
        );
        self.body
    }

    /// Component `k` of a `ty` value, as a float.
    fn ext(&mut self, of: S, ty: &str, k: usize) -> S {
        self.node(
            "extract",
            "float",
            &[("in", ty, of), ("index", "integer", S::V(k.to_string()))],
        )
    }
    fn convert(&mut self, of: S, from: &str, to: &str) -> S {
        self.node("convert", to, &[("in", from, of)])
    }
    fn grey(&mut self, of: S) -> S {
        self.convert(of, "float", "color3")
    }
    /// A two-input float maths node.
    fn op(&mut self, category: &str, a: S, b: S) -> S {
        self.node(
            category,
            "float",
            &[("in1", "float", a), ("in2", "float", b)],
        )
    }
    fn add(&mut self, a: S, b: S) -> S {
        self.op("add", a, b)
    }
    fn sub(&mut self, a: S, b: S) -> S {
        self.op("subtract", a, b)
    }
    fn mul(&mut self, a: S, b: S) -> S {
        self.op("multiply", a, b)
    }
    fn div(&mut self, a: S, b: S) -> S {
        self.op("divide", a, b)
    }
    fn unary(&mut self, category: &str, a: S) -> S {
        self.node(category, "float", &[("in", "float", a)])
    }
    fn clamp(&mut self, a: S, low: f64, high: f64) -> S {
        self.node(
            "clamp",
            "float",
            &[
                ("in", "float", a),
                ("low", "float", f(low)),
                ("high", "float", f(high)),
            ],
        )
    }
    /// `when` (ifgreater, ifgreatereq, ifequal): `then` if value1 compares
    /// to value2, else `otherwise`.
    fn cond(&mut self, when: &str, ty: &str, v1: S, v2: S, then: S, otherwise: S) -> S {
        self.node(
            when,
            ty,
            &[
                ("value1", "float", v1),
                ("value2", "float", v2),
                ("in1", ty, then),
                ("in2", ty, otherwise),
            ],
        )
    }
    fn combine3(&mut self, ty: &str, c: [S; 3]) -> S {
        let [a, b, d] = c;
        self.node(
            "combine3",
            ty,
            &[
                ("in1", "float", a),
                ("in2", "float", b),
                ("in3", "float", d),
            ],
        )
    }
    /// A boolean input as 0 or 1.
    fn bool01(&mut self, of: S) -> S {
        self.convert(of, "boolean", "float")
    }
    /// The blend factor: the first component of `mix`, clamped to 0..1
    /// (`Mix::apply`).
    fn fac(&mut self) -> S {
        let first = self.ext(S::I("mix"), "color3", 0);
        self.clamp(first, 0., 1.)
    }
    /// Perlin bias of `t` (`TwoZone::bias`).
    fn bias(&mut self, b: S, t: S) -> S {
        let b = self.clamp(b, 1e-6, 1. - 1e-6);
        let ln = self.unary("ln", b);
        let e = self.div(ln, f(0.5f64.ln()));
        let t = self.clamp(t, 0., 1.);
        // t^e is 0 at t = 0 for every exponent here (e > 0).
        self.op("power", t, e)
    }
    /// One component of a blend with a per-component formula.
    fn per_component(&mut self, formula: impl Fn(&mut G, S, S, S) -> S) -> S {
        let t = self.fac();
        let parts = [0, 1, 2].map(|k| {
            let a = self.ext(S::I("bg"), "color3", k);
            let b = self.ext(S::I("fg"), "color3", k);
            formula(self, a, b, t.clone())
        });
        self.combine3("color3", parts)
    }
    /// bg + (x − bg) × t for the colour node `x`.
    fn lerp(&mut self, x: S) -> S {
        let t = self.fac();
        self.node(
            "mix",
            "color3",
            &[
                ("fg", "color3", x),
                ("bg", "color3", S::I("bg")),
                ("mix", "float", t),
            ],
        )
    }
    fn colour_op(&mut self, category: &str, a: S, b: S) -> S {
        self.node(
            category,
            "color3",
            &[("in1", "color3", a), ("in2", "color3", b)],
        )
    }
    /// The world shading position (`vector3`).
    fn position(&mut self) -> S {
        self.node(
            "position",
            "vector3",
            &[("space", "string", S::V("world".into()))],
        )
    }
    fn normal(&mut self) -> S {
        self.node(
            "normal",
            "vector3",
            &[("space", "string", S::V("world".into()))],
        )
    }
}

fn tile_axis(g: &mut G, x: S, mode: Tiling) -> S {
    match mode {
        Tiling::Reset => x,
        Tiling::Repeat => {
            let fl = g.unary("floor", x.clone());
            g.sub(x, fl)
        }
        // A triangle wave (texture_locator::tile).
        Tiling::Mirror => {
            let half = g.mul(x, f(0.5));
            let fl = g.unary("floor", half.clone());
            let fr = g.sub(half, fl);
            let w = g.mul(fr, f(2.));
            let back = g.sub(f(2.), w.clone());
            g.cond("ifgreater", "float", w.clone(), f(1.), back, w)
        }
        Tiling::Edge => g.clamp(x, 0., 1.),
    }
}

/// The standard-node implementation of a node, where there is one: the
/// nodegraph's body.
fn implementation(category: &str) -> Option<String> {
    let mut g = G::default();
    if let Some(mode) = category.strip_prefix("operandi_mix_") {
        let out = match mode {
            "blend" => {
                let t = g.fac();
                g.node(
                    "mix",
                    "color3",
                    &[
                        ("fg", "color3", S::I("fg")),
                        ("bg", "color3", S::I("bg")),
                        ("mix", "float", t),
                    ],
                )
            }
            // bg ± t·fg.
            "add" | "subtract" => {
                let t = g.fac();
                let tf = g.node(
                    "multiply",
                    "color3",
                    &[("in1", "color3", S::I("fg")), ("in2", "float", t)],
                );
                g.colour_op(mode, S::I("bg"), tf)
            }
            "multiply" | "darken" | "lighten" => {
                let op = match mode {
                    "multiply" => "multiply",
                    "darken" => "min",
                    _ => "max",
                };
                let x = g.colour_op(op, S::I("bg"), S::I("fg"));
                g.lerp(x)
            }
            "difference" => {
                let d = g.colour_op("subtract", S::I("bg"), S::I("fg"));
                let x = g.node("absval", "color3", &[("in", "color3", d)]);
                g.lerp(x)
            }
            // 1 − (1 − bg)(1 − fg).
            "screen" => {
                let one = S::V("1, 1, 1".into());
                let ib = g.colour_op("subtract", one.clone(), S::I("bg"));
                let inv_fg = g.colour_op("subtract", one.clone(), S::I("fg"));
                let p = g.colour_op("multiply", ib, inv_fg);
                let x = g.colour_op("subtract", one, p);
                g.lerp(x)
            }
            // (1 − bg)·fg·bg + bg·screen.
            "soft_light" => {
                let one = S::V("1, 1, 1".into());
                let ib = g.colour_op("subtract", one.clone(), S::I("bg"));
                let inv_fg = g.colour_op("subtract", one.clone(), S::I("fg"));
                let p = g.colour_op("multiply", inv_fg, ib.clone());
                let screen = g.colour_op("subtract", one, p);
                let ab = g.colour_op("multiply", ib, S::I("fg"));
                let aba = g.colour_op("multiply", ab, S::I("bg"));
                let as_ = g.colour_op("multiply", S::I("bg"), screen);
                let x = g.colour_op("add", aba, as_);
                g.lerp(x)
            }
            // a·(1 − t) + t·a / b, or a where b is 0.
            "divide" => g.per_component(|g, a, b, t| {
                let om = g.sub(f(1.), t.clone());
                let p1 = g.mul(a.clone(), om);
                let ta = g.mul(t, a.clone());
                let p2 = g.div(ta, b.clone());
                let s = g.add(p1, p2);
                g.cond("ifequal", "float", b, f(0.), a, s)
            }),
            "overlay" => g.per_component(|g, a, b, t| {
                let om = g.sub(f(1.), t.clone());
                let tt = g.mul(f(2.), t);
                let tb = g.mul(tt.clone(), b.clone());
                let k = g.add(om.clone(), tb);
                let lo = g.mul(a.clone(), k);
                let ib = g.sub(f(1.), b);
                let tib = g.mul(tt, ib);
                let k2 = g.add(om, tib);
                let ia = g.sub(f(1.), a.clone());
                let p = g.mul(k2, ia);
                let hi = g.sub(f(1.), p);
                g.cond("ifgreatereq", "float", a, f(0.5), hi, lo)
            }),
            "dodge" => g.per_component(|g, a, b, t| {
                let tb = g.mul(t, b);
                let tmp = g.sub(f(1.), tb);
                let q = g.div(a.clone(), tmp.clone());
                let m = g.op("min", q, f(1.));
                let inner = g.cond("ifgreater", "float", tmp, f(0.), m, f(1.));
                g.cond("ifequal", "float", a.clone(), f(0.), a, inner)
            }),
            "burn" => g.per_component(|g, a, b, t| {
                let om = g.sub(f(1.), t.clone());
                let tb = g.mul(t, b);
                let tmp = g.add(om, tb);
                let ia = g.sub(f(1.), a);
                let q = g.div(ia, tmp.clone());
                let r = g.sub(f(1.), q);
                let c = g.clamp(r, 0., 1.);
                g.cond("ifgreater", "float", tmp, f(0.), c, f(0.))
            }),
            _ => return None,
        };
        return Some(g.finish(out, "color3"));
    }
    if let Some(axis) = category.strip_prefix("operandi_planar_") {
        let axis = AXES.iter().position(|a| *a == axis)?;
        let l = [0, 1, 2].map(|k| g.ext(S::I("in"), "color3", k));
        let [x, y, z] = l;
        let neg = |g: &mut G, s: S| g.sub(f(0.), s);
        let (u, v) = match axis {
            0 => (neg(&mut g, z), y),
            1 => (x, neg(&mut g, z)),
            _ => (x, y),
        };
        let side = |g: &mut G, c: S, mirror: &'static str, k: usize| {
            let m = g.bool01(S::I(mirror));
            let m2 = g.mul(m, f(2.));
            let sign = g.sub(f(1.), m2);
            let c = g.mul(c, sign);
            let c = g.add(c, S::I("centre"));
            let r = g.ext(S::I("repeat"), "vector2", k);
            g.mul(c, r)
        };
        let u = side(&mut g, u, "mirror_u", 0);
        let v = side(&mut g, v, "mirror_v", 1);
        let out = g.combine3("color3", [u, v, f(0.)]);
        return Some(g.finish(out, "color3"));
    }
    if let Some(shape) = category.strip_prefix("operandi_falloff_") {
        let shape = FALLOFFS.iter().find(|(_, n)| *n == shape)?.0;
        let d = [0, 1, 2].map(|k| {
            let l = g.ext(S::I("in"), "color3", k);
            let a = g.ext(S::I("amount"), "vector3", k);
            let p = g.mul(l, a);
            g.unary("absval", p)
        });
        let [d0, d1, d2] = d;
        let reach = match shape {
            FalloffType::Cubic => {
                let m = g.op("max", d0, d1);
                g.op("max", m, d2)
            }
            FalloffType::Spherical => {
                let s0 = g.mul(d0.clone(), d0);
                let s1 = g.mul(d1.clone(), d1);
                let s2 = g.mul(d2.clone(), d2);
                let s = g.add(s0, s1);
                let s = g.add(s, s2);
                g.unary("sqrt", s)
            }
            FalloffType::LinearX => d0,
            FalloffType::LinearY => d1,
            FalloffType::LinearZ => d2,
        };
        let r = g.sub(f(1.), reach);
        let c = g.clamp(r, 0., 1.);
        let out = g.grey(c);
        return Some(g.finish(out, "color3"));
    }
    if let Some(modes) = category.strip_prefix("operandi_tile_") {
        let (h, v) = TILINGS.iter().find_map(|(t, n)| {
            let rest = modes.strip_prefix(n)?.strip_prefix('_')?;
            Some((*t, TILINGS.iter().find(|(_, m)| *m == rest)?.0))
        })?;
        let x = g.ext(S::I("in"), "color3", 0);
        let y = g.ext(S::I("in"), "color3", 1);
        let z = g.ext(S::I("in"), "color3", 2);
        let x = tile_axis(&mut g, x, h);
        let y = tile_axis(&mut g, y, v);
        let out = g.combine3("color3", [x, y, z]);
        return Some(g.finish(out, "color3"));
    }
    if let Some(rest) = category.strip_prefix("operandi_wrap_") {
        let (spherical, axis) = match rest.split_once('_')? {
            ("spherical", a) => (true, a),
            ("cylindrical", a) => (false, a),
            _ => return None,
        };
        let a = AXES.iter().position(|x| *x == axis)?;
        let (from, toward) = ((a + 1) % 3, (a + 2) % 3);
        let l = [0, 1, 2].map(|k| g.ext(S::I("in"), "color3", k));
        let atan2 = |g: &mut G, y: S, x: S| {
            g.node(
                "atan2",
                "float",
                &[("iny", "float", y), ("inx", "float", x)],
            )
        };
        // wrap_coords: the angle from the next axis toward the one after.
        let ang = atan2(&mut g, l[toward].clone(), l[from].clone());
        let u0 = g.div(ang, f(std::f64::consts::TAU));
        let v0 = if spherical {
            let a2 = g.mul(l[from].clone(), l[from].clone());
            let b2 = g.mul(l[toward].clone(), l[toward].clone());
            let s = g.add(a2, b2);
            let h = g.unary("sqrt", s);
            let lat = atan2(&mut g, l[a].clone(), h);
            g.div(lat, f(std::f64::consts::PI))
        } else {
            l[a].clone()
        };
        let side = |g: &mut G, c: S, k: usize| {
            let c = g.add(c, S::I("centre"));
            let r = g.ext(S::I("repeat"), "vector2", k);
            g.mul(c, r)
        };
        let u = side(&mut g, u0, 0);
        let v = side(&mut g, v0, 1);
        let out = g.combine3("color3", [u, v, f(0.)]);
        return Some(g.finish(out, "color3"));
    }
    if let Some(axis) = category.strip_prefix("operandi_probe_") {
        let a = AXES.iter().position(|x| *x == axis)?;
        // probe_coords: right, up (the axis), toward the viewer.
        let (r, up, z) = ((a + 2) % 3, a, (a + 1) % 3);
        let l = [0, 1, 2].map(|k| g.ext(S::I("in"), "color3", k));
        let sq = |g: &mut G, x: S| g.mul(x.clone(), x);
        let s0 = sq(&mut g, l[0].clone());
        let s1 = sq(&mut g, l[1].clone());
        let s2 = sq(&mut g, l[2].clone());
        let s = g.add(s0, s1);
        let s = g.add(s, s2);
        let len = g.unary("sqrt", s);
        let dr = g.div(l[r].clone(), len.clone());
        let du = g.div(l[up].clone(), len.clone());
        let dz = g.div(l[z].clone(), len.clone());
        let dz = g.add(dz, f(1.));
        let q0 = sq(&mut g, dr.clone());
        let q1 = sq(&mut g, du.clone());
        let q2 = sq(&mut g, dz);
        let q = g.add(q0, q1);
        let q = g.add(q, q2);
        let n = g.unary("sqrt", q);
        let coord = |g: &mut G, d: S, rim: f64| {
            let x = g.div(d, n.clone());
            let x = g.mul(f(0.5), x);
            let x = g.add(f(0.5), x);
            let x = g.cond("ifgreater", "float", n.clone(), f(1e-12), x, f(rim));
            g.cond("ifgreater", "float", len.clone(), f(0.), x, f(0.5))
        };
        let u = coord(&mut g, dr, 0.5);
        let v = coord(&mut g, du, 0.);
        let ru = g.ext(S::I("repeat"), "vector2", 0);
        let rv = g.ext(S::I("repeat"), "vector2", 1);
        let u = g.mul(u, ru);
        let v = g.mul(v, rv);
        let out = g.combine3("color3", [u, v, f(0.)]);
        return Some(g.finish(out, "color3"));
    }
    // dominant_axis: X on ties with Y or Z, Y on ties with Z; as 0/1.
    let dominant = |g: &mut G| -> [S; 3] {
        let a = [0, 1, 2].map(|k| {
            let c = g.ext(S::I("in"), "color3", k);
            g.unary("absval", c)
        });
        let xz = g.cond(
            "ifgreatereq",
            "float",
            a[0].clone(),
            a[2].clone(),
            f(1.),
            f(0.),
        );
        let is_x = g.cond(
            "ifgreatereq",
            "float",
            a[0].clone(),
            a[1].clone(),
            xz,
            f(0.),
        );
        let yz = g.cond(
            "ifgreatereq",
            "float",
            a[1].clone(),
            a[2].clone(),
            f(1.),
            f(0.),
        );
        let not_x = g.sub(f(1.), is_x.clone());
        let is_y = g.mul(not_x.clone(), yz);
        let not_y = g.sub(f(1.), is_y.clone());
        let is_z = g.mul(not_x, not_y);
        [is_x, is_y, is_z]
    };
    if let Some(face) = category.strip_prefix("operandi_box_") {
        let face = BOX_FACES.iter().position(|x| *x == face)?;
        let axis = face % 3;
        let d = dominant(&mut g);
        let c = g.ext(S::I("in"), "color3", axis);
        let facing = g.cond("ifgreatereq", "float", c, f(0.), f(1.), f(0.));
        let side = if face < 3 {
            facing
        } else {
            g.sub(f(1.), facing)
        };
        let w = g.mul(d[axis].clone(), side);
        let out = g.grey(w);
        return Some(g.finish(out, "color3"));
    }
    if let Some(axis) = category.strip_prefix("operandi_cubic_") {
        let axis = AXES.iter().position(|x| *x == axis)?;
        let d = dominant(&mut g);
        let out = g.grey(d[axis].clone());
        return Some(g.finish(out, "color3"));
    }
    if let Some(axis) = category.strip_prefix("operandi_triplanar_") {
        let axis = AXES.iter().position(|x| *x == axis)?;
        // |n_face|^k / Σ |n_i|^k, a third each for a zero normal.
        let w = [0, 1, 2].map(|k| {
            let c = g.ext(S::I("in"), "color3", k);
            let a = g.unary("absval", c);
            g.op("power", a, S::I("power"))
        });
        let t = g.add(w[0].clone(), w[1].clone());
        let t = g.add(t, w[2].clone());
        let q = g.div(w[axis].clone(), t.clone());
        let r = g.cond("ifgreater", "float", t, f(0.), q, f(1. / 3.));
        let out = g.grey(r);
        return Some(g.finish(out, "color3"));
    }
    let out = match category {
        "operandi_constant" => g.convert(S::I("value"), "vector3", "color3"),
        "operandi_luma" => g.node(
            "luminance",
            "color3",
            &[
                ("in", "color3", S::I("in")),
                (
                    "lumacoeffs",
                    "color3",
                    S::V("0.2126, 0.7152, 0.0722".into()),
                ),
            ],
        ),
        "operandi_channel" => {
            let e = g.node(
                "extract",
                "float",
                &[
                    ("in", "color3", S::I("in")),
                    ("index", "integer", S::I("channel")),
                ],
            );
            g.grey(e)
        }
        // Components at or below 0 stay as they are (Cycles' Gamma).
        "operandi_gamma" => {
            let parts = [0, 1, 2].map(|k| {
                let e = g.ext(S::I("in"), "color3", k);
                let p = g.op("power", e.clone(), S::I("power"));
                g.cond("ifgreater", "float", e.clone(), f(0.), p, e)
            });
            g.combine3("color3", parts)
        }
        "operandi_bias_gain" => {
            let t = g.ext(S::I("in"), "color3", 0);
            let b = g.bias(S::I("bias"), t);
            // TwoZone::gain.
            let gc = g.clamp(S::I("gain"), 1e-6, 1. - 1e-6);
            let gg = g.sub(f(1.), gc);
            let t = g.clamp(b, 0., 1.);
            let t2 = g.mul(f(2.), t.clone());
            let lo = g.bias(gg.clone(), t2.clone());
            let lo = g.div(lo, f(2.));
            let rest = g.sub(f(2.), t2);
            let hi = g.bias(gg, rest);
            let hi = g.div(hi, f(2.));
            let hi = g.sub(f(1.), hi);
            let r = g.cond("ifgreatereq", "float", t, f(0.5), hi, lo);
            g.grey(r)
        }
        "operandi_texcoord" | "operandi_texcoord_map" => {
            let uv = if category == "operandi_texcoord" {
                g.node(
                    "texcoord",
                    "vector2",
                    &[("index", "integer", S::V("0".into()))],
                )
            } else {
                g.node(
                    "geompropvalue",
                    "vector2",
                    &[("geomprop", "string", S::I("map"))],
                )
            };
            let su = g.ext(uv.clone(), "vector2", 0);
            let sv = g.ext(uv, "vector2", 1);
            let ru = g.ext(S::I("repeat"), "vector2", 0);
            let rv = g.ext(S::I("repeat"), "vector2", 1);
            let u = g.mul(su, ru);
            let v = g.mul(sv, rv);
            // Counter-clockwise by `rotation` degrees (uv_transform).
            let rad = g.mul(S::I("rotation"), f(std::f64::consts::PI / 180.));
            let s = g.unary("sin", rad.clone());
            let c = g.unary("cos", rad);
            let uc = g.mul(u.clone(), c.clone());
            let vs = g.mul(v.clone(), s.clone());
            let us = g.mul(u, s);
            let vc = g.mul(v, c);
            let u2 = g.sub(uc, vs);
            let v2 = g.add(us, vc);
            g.combine3("color3", [u2, v2, f(0.)])
        }
        // 1 where the coordinate lies in 0..1 on every clipped axis.
        "operandi_inside" => {
            let w = [(0, "horizontal"), (1, "vertical")].map(|(k, clip)| {
                let x = g.ext(S::I("in"), "color3", k);
                let below = g.cond("ifgreatereq", "float", f(1.), x.clone(), f(1.), f(0.));
                let inside = g.cond("ifgreatereq", "float", x, f(0.), below, f(0.));
                let c = g.bool01(S::I(clip));
                let out = g.sub(f(1.), inside);
                let p = g.mul(c, out);
                g.sub(f(1.), p)
            });
            let [a, b] = w;
            let both = g.mul(a, b);
            g.grey(both)
        }
        // The rows times (x, y, z, 1).
        "operandi_locator" => {
            let p = g.position();
            let c = [0, 1, 2].map(|k| g.ext(p.clone(), "vector3", k));
            let [x, y, z] = c;
            let p4 = g.node(
                "combine4",
                "vector4",
                &[
                    ("in1", "float", x),
                    ("in2", "float", y),
                    ("in3", "float", z),
                    ("in4", "float", f(1.)),
                ],
            );
            let rows = ["row0", "row1", "row2"].map(|r| {
                g.node(
                    "dotproduct",
                    "float",
                    &[("in1", "vector4", p4.clone()), ("in2", "vector4", S::I(r))],
                )
            });
            g.combine3("color3", rows)
        }
        // The rows times the world normal, normalised (a zero stays zero).
        "operandi_normal" => {
            let n = g.normal();
            let rows = ["row0", "row1", "row2"].map(|r| {
                g.node(
                    "dotproduct",
                    "float",
                    &[("in1", "vector3", n.clone()), ("in2", "vector3", S::I(r))],
                )
            });
            let o = g.combine3("vector3", rows);
            let len = g.node("magnitude", "float", &[("in", "vector3", o.clone())]);
            let unit = g.node(
                "divide",
                "vector3",
                &[("in1", "vector3", o.clone()), ("in2", "float", len.clone())],
            );
            let r = g.cond("ifgreater", "vector3", len, f(0.), unit, o);
            g.convert(r, "vector3", "color3")
        }
        // 1 where the normal's component `axis` is above 0.
        "operandi_facing" => {
            let e = g.node(
                "extract",
                "float",
                &[
                    ("in", "color3", S::I("in")),
                    ("index", "integer", S::I("axis")),
                ],
            );
            let r = g.cond("ifgreater", "float", e, f(0.), f(1.), f(0.));
            g.grey(r)
        }
        "operandi_distance" => {
            let p = g.position();
            let d = g.node(
                "subtract",
                "vector3",
                &[("in1", "vector3", p), ("in2", "vector3", S::I("position"))],
            );
            let m = g.node("magnitude", "float", &[("in", "vector3", d)]);
            g.grey(m)
        }
        // Degrees between the normal and the direction to `position`.
        "operandi_locator_incidence" => {
            let p = g.position();
            let d = g.node(
                "subtract",
                "vector3",
                &[("in1", "vector3", S::I("position")), ("in2", "vector3", p)],
            );
            let n = g.normal();
            let ld = g.node("magnitude", "float", &[("in", "vector3", d.clone())]);
            let ln = g.node("magnitude", "float", &[("in", "vector3", n.clone())]);
            let dot = g.node(
                "dotproduct",
                "float",
                &[("in1", "vector3", d), ("in2", "vector3", n)],
            );
            let den = g.mul(ld.clone(), ln.clone());
            let c = g.div(dot, den);
            let c = g.clamp(c, -1., 1.);
            let a = g.unary("acos", c);
            let deg = g.mul(a, f(180. / std::f64::consts::PI));
            let inner = g.cond("ifgreater", "float", ln, f(0.), deg, f(0.));
            let r = g.cond("ifgreater", "float", ld, f(0.), inner, f(0.));
            g.grey(r)
        }
        "operandi_image" | "operandi_image_alpha" => {
            let u = g.ext(S::I("texcoord"), "color3", 0);
            let v = g.ext(S::I("texcoord"), "color3", 1);
            let tc = g.node(
                "combine2",
                "vector2",
                &[("in1", "float", u), ("in2", "float", v)],
            );
            let alpha = category == "operandi_image_alpha";
            let ty = if alpha { "color4" } else { "color3" };
            let img = g.node(
                "image",
                ty,
                &[
                    ("file", "filename", S::I("file")),
                    ("texcoord", "vector2", tc),
                    ("uaddressmode", "string", S::I("addressmode")),
                    ("vaddressmode", "string", S::I("addressmode")),
                    ("filtertype", "string", S::I("filtertype")),
                ],
            );
            if alpha {
                let a = g.ext(img, "color4", 3);
                g.grey(a)
            } else {
                img
            }
        }
        "operandi_attribute" | "operandi_attribute_alpha" => {
            let a = g.node(
                "geompropvalue",
                "color4",
                &[("geomprop", "string", S::I("attribute"))],
            );
            if category == "operandi_attribute_alpha" {
                let x = g.ext(a, "color4", 3);
                g.grey(x)
            } else {
                g.convert(a, "color4", "color3")
            }
        }
        // Process: hue × scale + shift°, saturation × scale (clamped).
        "operandi_hsv" => {
            let hsv = g.node("rgbtohsv", "color3", &[("in", "color3", S::I("in"))]);
            let h = g.ext(hsv.clone(), "color3", 0);
            let s = g.ext(hsv.clone(), "color3", 1);
            let v = g.ext(hsv, "color3", 2);
            let hs = g.mul(h, S::I("hue_scale"));
            let shift = g.div(S::I("hue_shift"), f(360.));
            let h = g.add(hs, shift);
            let h = g.op("modulo", h, f(1.));
            let s = g.mul(s, S::I("saturation_scale"));
            let s = g.clamp(s, 0., 1.);
            let c = g.combine3("color3", [h, s, v]);
            g.node("hsvtorgb", "color3", &[("in", "color3", c)])
        }
        // shader_eval::hsv_vary.
        "operandi_hsv_vary" => {
            let hsv = g.node("rgbtohsv", "color3", &[("in", "color3", S::I("in"))]);
            let [h, s, v] = [0, 1, 2].map(|k| g.ext(hsv.clone(), "color3", k));
            let [r0, r1, r2] = [0, 1, 2].map(|k| g.ext(S::I("random"), "color3", k));
            let [a0, a1, a2] = [0, 1, 2].map(|k| g.ext(S::I("amounts"), "vector3", k));
            let r1 = g.sub(r1, f(0.5));
            let dh = g.mul(a1, r1);
            let h = g.add(h, dh);
            let h = g.op("modulo", h, f(1.));
            let ds = g.mul(a2, r2);
            let ks = g.sub(f(1.), ds);
            let s = g.mul(s, ks);
            let s = g.clamp(s, 0., 1.);
            let dv = g.mul(a0, r0);
            let kv = g.sub(f(1.), dv);
            let v = g.mul(v, kv);
            let v = g.op("max", v, f(0.));
            let c = g.combine3("color3", [h, s, v]);
            g.node("hsvtorgb", "color3", &[("in", "color3", c)])
        }
        _ => return None,
    };
    Some(g.finish(out, "color3"))
}

/// Whether a node category has a standard-node implementation.
pub fn implemented(category: &str) -> bool {
    implementation(category).is_some()
}

/// The library as MaterialX text: a nodedef per node (`ND_<category>`),
/// and a nodegraph (`NG_<category>`) for those with a standard-node
/// implementation.
pub fn library() -> String {
    let mut out = String::from("<?xml version=\"1.0\"?>\n<materialx version=\"1.39\">\n");
    for (category, _) in nodes() {
        out.push_str(&definition(&category).unwrap_or_default());
    }
    out.push_str("</materialx>\n");
    out
}

/// One node's nodedef and, when it has one, its implementation, as
/// MaterialX elements; `None` for a category the form never writes.
pub fn definition(category: &str) -> Option<String> {
    let inputs = nodes().into_iter().find(|(c, _)| c == category)?.1;
    let mut out = String::new();
    {
        let ty = if category == SURFACE {
            "surfaceshader"
        } else {
            "color3"
        };
        let _ = writeln!(
            out,
            "  <nodedef name=\"ND_{category}\" node=\"{category}\" nodegroup=\"operandi\">"
        );
        for (name, t) in inputs {
            let uniform = if t == "color3" {
                ""
            } else {
                " uniform=\"true\""
            };
            let _ = writeln!(
                out,
                "    <input name=\"{name}\" type=\"{t}\" value=\"{}\"{uniform} />",
                default_of(t)
            );
        }
        let _ = writeln!(
            out,
            "    <output name=\"out\" type=\"{ty}\" />\n  </nodedef>"
        );
        if let Some(body) = implementation(category) {
            let _ = write!(
                out,
                "  <nodegraph name=\"NG_{category}\" nodedef=\"ND_{category}\">\n{body}  </nodegraph>\n"
            );
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "tree_library_tests.rs"]
mod tests;
