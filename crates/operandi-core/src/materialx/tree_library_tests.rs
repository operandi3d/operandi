//! The library's standard-node implementations against the tree's own
//! sums, through a small MaterialX evaluator written from the MaterialX
//! specification (test only).
use super::*;
use crate::Scene;
use crate::materialx::doc::{self, Document, Element};
use crate::materialx::tree;
use crate::shader_eval::{Expr, ImageLookup, Input, Mix, Principled, ShadingPoint, extended};
use crate::shader_tree::Filtering;
use crate::texture_locator::{FalloffType, Tiling};

const STDLIB: &str = include_str!("../../materialx/stdlib_defs.mtlx");

fn stdlib() -> Document {
    doc::parse(STDLIB).unwrap()
}

/// A standard nodedef the node matches: the same category and output
/// type, and every input it sets declared with that type.
fn matching_def<'a>(std: &'a Document, el: &Element) -> Option<&'a Element> {
    std.all("nodedef").find(|d| {
        d.attrs.get("node") == Some(&el.category)
            && d.outputs.iter().any(|o| o.ty == el.ty)
            && el
                .inputs
                .iter()
                .all(|p| d.inputs.iter().any(|i| i.name == p.name && i.ty == p.ty))
    })
}

#[test]
fn implementations_use_only_standard_nodes_as_defined() {
    let std = stdlib();
    let lib = doc::parse(&library()).unwrap();
    let mut graphs = 0;
    for g in lib.all("nodegraph") {
        graphs += 1;
        let def = lib.element(&g.attrs["nodedef"]).expect("its nodedef");
        for el in &g.children {
            assert!(
                matching_def(&std, el).is_some(),
                "{}: {} ({} {}) isn't a standard node as used",
                g.name,
                el.name,
                el.category,
                el.ty
            );
            for p in &el.inputs {
                if let Some(i) = &p.interfacename {
                    let decl = def
                        .input(i)
                        .unwrap_or_else(|| panic!("{}: {} reads undeclared {i}", g.name, el.name));
                    assert_eq!(decl.ty, p.ty, "{}: {}.{}", g.name, el.name, p.name);
                }
                if let Some(n) = &p.nodename {
                    let src = g.child(n).unwrap_or_else(|| panic!("{}: no {n}", g.name));
                    assert_eq!(src.ty, p.ty, "{}: {}.{} from {n}", g.name, el.name, p.name);
                }
            }
        }
    }
    assert!(graphs >= 50, "{graphs} implementations");
}

#[derive(Clone, Debug)]
enum Val {
    N(Vec<f64>),
    S(String),
}
impl Val {
    fn n(self) -> Vec<f64> {
        match self {
            Val::N(v) => v,
            Val::S(s) => panic!("expected numbers, got '{s}'"),
        }
    }
    fn s(self) -> String {
        match self {
            Val::S(s) => s,
            Val::N(v) => panic!("expected text, got {v:?}"),
        }
    }
}

fn size(ty: &str) -> usize {
    match ty {
        "float" | "integer" | "boolean" => 1,
        "vector2" => 2,
        "color3" | "vector3" => 3,
        "color4" | "vector4" => 4,
        other => panic!("no size for {other}"),
    }
}

fn parse(ty: &str, v: &str) -> Val {
    match ty {
        "string" | "filename" => Val::S(v.into()),
        "boolean" => Val::N(vec![if v == "true" { 1. } else { 0. }]),
        _ => Val::N(if v.trim().is_empty() {
            Vec::new()
        } else {
            v.split(',').map(|s| s.trim().parse().unwrap()).collect()
        }),
    }
}

/// Where names resolve: a graph's nodes, and the node that called the
/// graph (for `interfacename`).
struct Scope<'a> {
    nodes: &'a [Element],
    caller: Option<(&'a Element, &'a Scope<'a>)>,
}

struct Eval<'a> {
    std: &'a Document,
    lib: &'a Document,
    at: ShadingPoint,
    picture: &'a dyn Fn(&str, [f64; 2]) -> [f64; 4],
}

fn broadcast(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| {
            f(
                a[if a.len() == 1 { 0 } else { i }],
                b[if b.len() == 1 { 0 } else { i }],
            )
        })
        .collect()
}

fn rgb_to_hsv(c: &[f64]) -> Vec<f64> {
    let (r, g, b) = (c[0], c[1], c[2]);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max > 0. { d / max } else { 0. };
    let mut h = if d <= 0. {
        0.
    } else if r == max {
        (g - b) / d
    } else if g == max {
        2. + (b - r) / d
    } else {
        4. + (r - g) / d
    };
    h /= 6.;
    if h < 0. {
        h += 1.;
    }
    vec![h, s, max]
}

fn hsv_to_rgb(c: &[f64]) -> Vec<f64> {
    let (h, s, v) = (c[0], c[1], c[2]);
    let h = (h - h.floor()) * 6.;
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (v * (1. - s), v * (1. - s * f), v * (1. - s * (1. - f)));
    match i as i32 {
        0 => vec![v, t, p],
        1 => vec![q, v, p],
        2 => vec![p, v, t],
        3 => vec![p, q, v],
        4 => vec![t, p, v],
        _ => vec![v, p, q],
    }
}

fn address(x: f64, mode: &str) -> Option<f64> {
    match mode {
        "periodic" => Some(x - x.floor()),
        "clamp" => Some(x.clamp(0., 1.)),
        "mirror" => {
            let f = x - 2. * (x / 2.).floor();
            Some(if f > 1. { 2. - f } else { f })
        }
        "constant" => (0.0..=1.0).contains(&x).then_some(x),
        other => panic!("address mode {other}"),
    }
}

impl Eval<'_> {
    fn def_default(&self, el: &Element, input: &str) -> Option<Val> {
        let def = if el.category.starts_with("operandi_") {
            self.lib.element(&format!("ND_{}", el.category))
        } else {
            matching_def(self.std, el)
        }?;
        let p = def.input(input)?;
        Some(parse(&p.ty, p.value.as_deref()?))
    }

    fn input(&self, el: &Element, name: &str, scope: &Scope) -> Val {
        let Some(p) = el.input(name) else {
            return self
                .def_default(el, name)
                .unwrap_or_else(|| panic!("{} ({}) has no {name}", el.name, el.category));
        };
        if let Some(n) = &p.nodename {
            let src = scope
                .nodes
                .iter()
                .find(|e| &e.name == n)
                .unwrap_or_else(|| panic!("no node {n}"));
            return self.node(src, scope);
        }
        if let Some(i) = &p.interfacename {
            let (caller, outer) = scope.caller.expect("an interface inside a graph");
            return self.input(caller, i, outer);
        }
        parse(&p.ty, p.value.as_deref().unwrap_or_default())
    }

    fn node(&self, el: &Element, scope: &Scope) -> Val {
        let n = |k: &str| self.input(el, k, scope).n();
        let s = |k: &str| self.input(el, k, scope).s();
        let unary = |f: fn(f64) -> f64| Val::N(n("in").into_iter().map(f).collect());
        let binary = |f: fn(f64, f64) -> f64| Val::N(broadcast(&n("in1"), &n("in2"), f));
        let cond = |f: fn(f64, f64) -> bool| {
            if f(n("value1")[0], n("value2")[0]) {
                Val::N(n("in1"))
            } else {
                Val::N(n("in2"))
            }
        };
        Val::N(match el.category.as_str() {
            "constant" => n("value"),
            "convert" => {
                let v = n("in");
                let to = size(&el.ty);
                if v.len() == 1 {
                    vec![v[0]; to]
                } else {
                    v[..to].to_vec()
                }
            }
            "extract" => vec![n("in")[n("index")[0] as usize]],
            "combine2" => vec![n("in1")[0], n("in2")[0]],
            "combine3" => vec![n("in1")[0], n("in2")[0], n("in3")[0]],
            "combine4" => vec![n("in1")[0], n("in2")[0], n("in3")[0], n("in4")[0]],
            "add" => return binary(|a, b| a + b),
            "subtract" => return binary(|a, b| a - b),
            "multiply" => return binary(|a, b| a * b),
            "divide" => return binary(|a, b| a / b),
            "min" => return binary(f64::min),
            "max" => return binary(f64::max),
            "power" => return binary(f64::powf),
            "atan2" => vec![n("iny")[0].atan2(n("inx")[0])],
            "modulo" => return binary(|a, b| a - b * (a / b).floor()),
            "absval" => return unary(f64::abs),
            "floor" => return unary(f64::floor),
            "sin" => return unary(f64::sin),
            "cos" => return unary(f64::cos),
            "acos" => return unary(f64::acos),
            "ln" => return unary(f64::ln),
            "sqrt" => return unary(f64::sqrt),
            "clamp" => {
                let v = n("in");
                let lo = broadcast(&v, &n("low"), |_, l| l);
                let hi = broadcast(&v, &n("high"), |_, h| h);
                (0..v.len()).map(|i| v[i].max(lo[i]).min(hi[i])).collect()
            }
            "mix" => {
                let m = n("mix");
                let a = broadcast(&n("fg"), &m, |x, m| x * m);
                let b = broadcast(&n("bg"), &m, |x, m| x * (1. - m));
                broadcast(&a, &b, |x, y| x + y)
            }
            "dotproduct" => {
                let (a, b) = (n("in1"), n("in2"));
                vec![(0..a.len()).map(|i| a[i] * b[i]).sum()]
            }
            "magnitude" => vec![n("in").iter().map(|x| x * x).sum::<f64>().sqrt()],
            "ifgreater" => return cond(|a, b| a > b),
            "ifgreatereq" => return cond(|a, b| a >= b),
            "ifequal" => return cond(|a, b| a == b),
            "luminance" => {
                let (c, k) = (n("in"), n("lumacoeffs"));
                vec![c[0] * k[0] + c[1] * k[1] + c[2] * k[2]; 3]
            }
            "rgbtohsv" => rgb_to_hsv(&n("in")),
            "hsvtorgb" => hsv_to_rgb(&n("in")),
            "position" => {
                assert_eq!(s("space"), "world");
                self.at.position.to_vec()
            }
            "normal" => {
                assert_eq!(s("space"), "world");
                self.at.normal.to_vec()
            }
            "texcoord" => self.at.uv.to_vec(),
            // The test mesh has every UV map the same, and no attributes.
            "geompropvalue" if el.ty == "vector2" => self.at.uv.to_vec(),
            "geompropvalue" => vec![0.; size(&el.ty)],
            "image" => {
                let uv = n("texcoord");
                let u = address(uv[0], &s("uaddressmode"));
                let v = address(uv[1], &s("vaddressmode"));
                let rgba = match (u, v) {
                    (Some(u), Some(v)) => (self.picture)(&s("file"), [u, v]).to_vec(),
                    _ => n("default"),
                };
                rgba[..size(&el.ty)].to_vec()
            }
            cat if cat.starts_with("operandi_") => {
                let g = self
                    .lib
                    .element(&format!("NG_{cat}"))
                    .unwrap_or_else(|| panic!("{cat} has no implementation"));
                let inner = Scope {
                    nodes: &g.children,
                    caller: Some((el, scope)),
                };
                let out = g.outputs[0].nodename.as_deref().unwrap();
                return self.node(g.child(out).unwrap(), &inner);
            }
            other => panic!("the test evaluator has no {other}"),
        })
    }
}

/// A smooth picture: RGBA in 0..1 from UVs in 0..1.
fn picture(uv: [f64; 2]) -> [f64; 4] {
    let [u, v] = uv;
    [
        0.5 + 0.4 * (7. * u).sin(),
        0.5 + 0.4 * (5. * v).cos(),
        (u * v * 3. + 0.1).fract(),
        0.2 + 0.6 * u,
    ]
}

fn boxed(e: Expr) -> Box<Expr> {
    Box::new(e)
}

/// Expressions every implemented node appears in, over leaves that vary
/// from point to point.
fn cases() -> Vec<Expr> {
    let uv = |repeat: [f64; 2], rotation: f64| Expr::MeshUv {
        map: None,
        repeat,
        rotation,
    };
    let locator = Expr::Locator(Box::new([
        0.8, 0.1, -0.3, 0.25, -0.2, 1.1, 0.4, -0.5, 0.3, 0.2, 0.9, 0.1,
    ]));
    let normal = Expr::Normal(Box::new([0.9, 0.1, 0., -0.1, 1., 0.2, 0., 0.3, 1.2]));
    let image = |vector: Expr, extension: Tiling, alpha: bool| {
        let l = Box::new(ImageLookup {
            image: 7,
            filtering: Filtering::Bilinear,
            vector,
            extension,
        });
        if alpha {
            Expr::ImageAlpha(l)
        } else {
            Expr::ImageColor(l)
        }
    };
    let wide = uv([3.5, -2.25], 0.);
    let pic = image(uv([1., 1.], 0.), Tiling::Repeat, false);
    let other = image(uv([2., 1.5], 40.), Tiling::Mirror, false);
    // A factor that runs outside 0..1.
    let fac = Expr::Channel(boxed(locator.clone()), 0);
    let mut out = vec![
        uv([1., 1.], 0.),
        uv([2., 3.], 30.),
        Expr::MeshUv {
            map: Some("Second".into()),
            repeat: [1.5, 0.5],
            rotation: -75.,
        },
        locator.clone(),
        normal.clone(),
        Expr::Normal(Box::new([0.; 9])),
        Expr::Inside(boxed(wide.clone()), [true, true]),
        Expr::Inside(boxed(wide.clone()), [true, false]),
        Expr::Inside(boxed(wide.clone()), [false, true]),
        Expr::Inside(boxed(wide.clone()), [false, false]),
        Expr::Gamma(
            boxed(Expr::Mix {
                mode: Mix::Subtract,
                fac: boxed(Expr::scalar(1.)),
                a: boxed(pic.clone()),
                b: boxed(Expr::scalar(0.5)),
            }),
            2.2,
        ),
        Expr::BiasGain(boxed(Expr::Channel(boxed(pic.clone()), 1)), 0.3, 0.8),
        Expr::BiasGain(boxed(Expr::Channel(boxed(pic.clone()), 2)), 0.7, 0.2),
        Expr::BiasGain(boxed(fac.clone()), 0.5, 0.5),
        Expr::Hsv {
            of: boxed(pic.clone()),
            hue_scale: 1.5,
            hue_shift: 200.,
            saturation_scale: 1.7,
        },
        Expr::HsvVary {
            color: boxed(pic.clone()),
            random: boxed(other.clone()),
            amounts: [0.5, 0.8, 0.3],
        },
        Expr::DistanceTo([0.5, -1., 2.]),
        Expr::LocatorIncidence([0.5, -1., 2.]),
        Expr::Facing(boxed(normal.clone()), 1),
        Expr::Attribute {
            name: "Colour".into(),
            alpha: false,
        },
        Expr::Attribute {
            name: "Colour".into(),
            alpha: true,
        },
        Expr::Luma(boxed(other.clone())),
        Expr::Channel(boxed(other.clone()), 2),
    ];
    for (h, _) in tree::TILINGS {
        for (v, _) in tree::TILINGS {
            out.push(Expr::Tile(boxed(wide.clone()), [h, v]));
        }
    }
    for (t, _) in tree::ADDRESS {
        for alpha in [false, true] {
            out.push(image(wide.clone(), t, alpha));
        }
    }
    for axis in 0..3 {
        for mirror in 0..4 {
            out.push(Expr::Planar {
                of: boxed(locator.clone()),
                axis,
                centre: 0.5,
                repeat: [2., -3.],
                mirror,
            });
        }
    }
    for axis in 0..3 {
        for spherical in [false, true] {
            out.push(Expr::Wrap {
                of: boxed(locator.clone()),
                axis,
                spherical,
                centre: 0.25,
                repeat: [2., 1.5],
            });
        }
        out.push(Expr::Probe {
            of: boxed(locator.clone()),
            axis,
            repeat: [1., 2.],
        });
        out.push(Expr::FaceWeight {
            normal: boxed(normal.clone()),
            face: axis,
            kind: crate::shader_eval::FaceKind::Dominant,
        });
        out.push(Expr::FaceWeight {
            normal: boxed(normal.clone()),
            face: axis,
            kind: crate::shader_eval::FaceKind::Power(3.),
        });
    }
    for face in 0..6 {
        out.push(Expr::FaceWeight {
            normal: boxed(normal.clone()),
            face,
            kind: crate::shader_eval::FaceKind::Signed,
        });
    }
    // The probe's centre and rim, and a zero normal's even split.
    out.push(Expr::Probe {
        of: boxed(Expr::Locator(Box::new([0.; 12]))),
        axis: 1,
        repeat: [1., 1.],
    });
    out.push(Expr::FaceWeight {
        normal: boxed(Expr::Normal(Box::new([0.; 9]))),
        face: 1,
        kind: crate::shader_eval::FaceKind::Power(2.),
    });
    for (kind, _) in tree::FALLOFFS {
        out.push(Expr::Falloff {
            of: boxed(locator.clone()),
            kind,
            amount: [0.4, 0.7, 1.3],
        });
    }
    for (mode, _) in tree::MIXES {
        for a in [pic.clone(), Expr::Const([0., 0.25, 0.75])] {
            out.push(Expr::Mix {
                mode,
                fac: boxed(fac.clone()),
                a: boxed(a),
                b: boxed(other.clone()),
            });
        }
    }
    out
}

fn points() -> Vec<ShadingPoint> {
    let p = |position: [f64; 3], uv: [f64; 2], normal: [f64; 3]| ShadingPoint {
        position,
        uv,
        normal,
    };
    vec![
        p([0.1, 0.2, 0.3], [0.25, 0.75], [0., 1., 0.]),
        p([-1.3, 0.7, 2.1], [0.9, 0.1], [0.6, -0.8, 0.]),
        p([2.4, -1.9, -0.6], [0.43, 0.61], [-0.3, 0.2, 0.93]),
        p([0.5, 0.5, 0.5], [0., 1.], [0., 0., -1.]),
        p([-0.05, 3.2, 1.0], [0.77, 0.33], [0.1, 0.99, 0.1]),
    ]
}

#[test]
fn implementations_match_the_tree_at_shading_points() {
    let std = stdlib();
    let lib = doc::parse(&library()).unwrap();
    let mut scene = Scene::default();
    scene.images.push(crate::backdrop::SceneImage {
        id: 7,
        name: "pic".into(),
        path: "pic.png".into(),
        width: 8,
        height: 8,
    });
    let mut seen = std::collections::BTreeSet::new();
    for e in cases() {
        let p = Principled {
            inputs: vec![(Input::BaseColor, e.clone())],
            shader: Default::default(),
            bump: None,
            normal_map: None,
            displacement: None,
            coat_bump: 0.,
            see_through: None,
            rounded_edge: None,
            absorption: None,
            scatter: None,
            thin_walled: false,
            glass_principled: false,
            node: None,
            values: Vec::new(),
        };
        let text = tree::write(&p, &scene, "Case");
        let d = doc::parse(&text).unwrap();
        for el in &d.elements {
            seen.insert(el.category.clone());
        }
        let surface = d.all(tree::SURFACE).next().unwrap();
        let top = Scope {
            nodes: &d.elements,
            caller: None,
        };
        for at in points() {
            let pic = |file: &str, uv: [f64; 2]| {
                assert_eq!(file, "pic.png");
                picture(uv)
            };
            let ev = Eval {
                std: &std,
                lib: &lib,
                at,
                picture: &pic,
            };
            let got = ev.input(surface, "base_color", &top).n();
            let want = e.eval_at(&at, &|l, raw| {
                extended(raw, l.extension).map_or([0.; 4], picture)
            });
            for k in 0..3 {
                let tol = 1e-9 * (1. + want[k].abs());
                assert!(
                    (got[k] - want[k]).abs() <= tol,
                    "{e:?}\nat {at:?}: MaterialX {got:?}, tree {want:?}"
                );
            }
        }
    }
    // Every node with an implementation was exercised (constants are
    // written inline; tree_tests checks operandi_constant's graph).
    for (category, _) in nodes() {
        if implemented(&category) && category != "operandi_constant" {
            assert!(seen.contains(&category), "no case uses {category}");
        }
    }
}

#[test]
fn falloff_and_tiling_families_cover_every_mode() {
    let names: Vec<String> = nodes().into_iter().map(|(c, _)| c).collect();
    for kind in FalloffType::ALL {
        let n = tree::FALLOFFS.iter().find(|(k, _)| k == kind).unwrap().1;
        assert!(names.contains(&format!("operandi_falloff_{n}")));
    }
    assert_eq!(
        names
            .iter()
            .filter(|n| n.starts_with("operandi_tile_"))
            .count(),
        Tiling::ALL.len() * Tiling::ALL.len()
    );
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}
