//! USD export of Blender Node Material layers (usd-gaps-3): a Material
//! Group shown by a Node Material layer whose graph came from Blender
//! (Cycles nodes, `node_graph`) goes out as the MaterialX nodes those
//! nodes map to, onto an open_pbr_surface, so other programs see the
//! graph rather than a Preview Surface average.
//!
//! Built from the public node definitions on both sides: Cycles' own node
//! sockets (the flattened graph Operandi renders) and MaterialX 1.39's
//! standard library. The Principled BSDF is the inverse of the OpenPBR
//! mapping `materialx::cycles` uses, so a graph that maps reads back as
//! the same Cycles nodes. A node with no MaterialX counterpart (Noise,
//! Voronoi, Wave and Gradient textures, Color Ramp, Bump...) leaves the
//! input it feeds at its default and is named in the notes; a surface
//! shader with none (Glass, Mix Shader...) makes the whole graph an error,
//! and the material keeps its Preview Surface.
use crate::materialx_out::Network;
use operandi_core::Scene;
use operandi_core::materialx::doc::{Document, Element, Port};
use operandi_core::node_graph::{FlatGraph, FlatValue};
use operandi_core::openpbr::{DEFAULT_LENGTH_IN_METRES, STRENGTH_PER_NIT};
use operandi_core::shader_tree::{NodeId, NodeKind};
use std::collections::{BTreeMap, BTreeSet};

/// The network of group `group` when its first enabled layer is a fully
/// opaque Node Material with a Blender graph; `None` for any other group.
pub fn group_network(
    scene: &Scene,
    group: NodeId,
) -> Result<Option<(Network, Vec<String>)>, String> {
    let tree = &scene.shader_tree;
    let Some(top) = tree.children(Some(group)).find(|n| n.layer.enable) else {
        return Ok(None);
    };
    let NodeKind::NodeMaterial(m) = &top.kind else {
        return Ok(None);
    };
    let Some(graph) = m.graph.and_then(|g| tree.node_graphs.get(g)) else {
        return Ok(None);
    };
    if graph.materialx.is_some() {
        return Ok(None);
    }
    if top.layer.opacity < 1. {
        return Err("a Node Material layer below full opacity".into());
    }
    let flat = graph.flatten(&tree.exposed_values(m, &BTreeMap::new()))?;
    let name = graph.material.name.clone();
    let (doc, notes) = document(&flat, &name)?;
    let net =
        crate::materialx_graph_out::flat_network(&doc, &safe(&name), std::path::Path::new(""))?;
    Ok(Some((net, notes)))
}

/// A MaterialX name: letters, digits and underscores, not starting with
/// a digit.
fn safe(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, 'n');
    }
    s
}

/// A value in the graph: a node's output, or a constant.
#[derive(Clone, Debug, PartialEq)]
enum Src {
    Node { name: String, ty: &'static str },
    Value { v: Vec<f64>, ty: &'static str },
}
impl Src {
    fn ty(&self) -> &'static str {
        match self {
            Src::Node { ty, .. } | Src::Value { ty, .. } => ty,
        }
    }
    fn float(v: f64) -> Self {
        Src::Value {
            v: vec![v],
            ty: "float",
        }
    }
}

/// How many numbers a MaterialX type holds.
fn width(ty: &str) -> usize {
    match ty {
        "vector2" => 2,
        "color3" | "vector3" => 3,
        "color4" | "vector4" => 4,
        _ => 1,
    }
}

/// MaterialX text for numbers.
fn text(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{}", *x as f32))
        .collect::<Vec<_>>()
        .join(", ")
}

struct Conv<'a> {
    flat: &'a FlatGraph,
    elements: Vec<Element>,
    names: BTreeSet<String>,
    memo: BTreeMap<(usize, String), Option<Src>>,
    notes: BTreeSet<String>,
}

/// `flat` as a MaterialX document with one material, `name`, and notes on
/// what doesn't map.
pub fn document(flat: &FlatGraph, name: &str) -> Result<(Document, Vec<String>), String> {
    let (from, socket) = flat.feeding("Surface").ok_or("nothing feeds the surface")?;
    let mut c = Conv {
        flat,
        elements: Vec::new(),
        names: BTreeSet::new(),
        memo: BTreeMap::new(),
        notes: BTreeSet::new(),
    };
    let material = safe(name);
    c.names.insert(material.clone());
    let surface = c.surface(from, socket)?;
    if flat.feeding("Displacement").is_some() {
        c.notes.insert("its displacement".into());
    }
    c.elements.push(Element {
        category: "surfacematerial".into(),
        name: material,
        ty: "material".into(),
        inputs: vec![Port {
            name: "surfaceshader".into(),
            ty: "surfaceshader".into(),
            nodename: Some(surface),
            ..Port::default()
        }],
        ..Element::default()
    });
    Ok((
        Document {
            version: "1.39".into(),
            colorspace: Some("lin_rec709".into()),
            elements: c.elements,
        },
        c.notes.into_iter().collect(),
    ))
}

impl Conv<'_> {
    fn label(&self, node: usize) -> String {
        self.flat.nodes[node].name.clone()
    }

    /// A new element name from `base`.
    fn fresh(&mut self, base: &str) -> String {
        let base = safe(base);
        let mut name = base.clone();
        let mut n = 2;
        while !self.names.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        name
    }

    /// Adds node `category` of type `ty` named after `base` with `inputs`
    /// (each converted to the type given), its output as a source.
    fn add(
        &mut self,
        base: &str,
        category: &str,
        ty: &'static str,
        inputs: Vec<(&str, &'static str, Option<Src>)>,
    ) -> Src {
        let name = self.fresh(base);
        let mut ports = Vec::new();
        for (input, want, src) in inputs {
            let Some(src) = src else { continue };
            let src = self.cast(base, src, want);
            ports.push(Self::port(input, want, &src));
        }
        self.elements.push(Element {
            category: category.into(),
            name: name.clone(),
            ty: ty.into(),
            inputs: ports,
            ..Element::default()
        });
        Src::Node { name, ty }
    }

    fn port(name: &str, ty: &str, src: &Src) -> Port {
        match src {
            Src::Node { name: n, .. } => Port {
                name: name.into(),
                ty: ty.into(),
                nodename: Some(n.clone()),
                ..Port::default()
            },
            Src::Value { v, .. } => Port {
                name: name.into(),
                ty: ty.into(),
                value: Some(text(v)),
                ..Port::default()
            },
        }
    }

    /// `src` as type `to`, the way Cycles converts sockets: a number fills
    /// every channel, a colour becomes its luminance, a vector the average
    /// of its parts.
    fn cast(&mut self, base: &str, src: Src, to: &'static str) -> Src {
        let from = src.ty();
        if from == to {
            return src;
        }
        if let Src::Value { v, .. } = &src {
            let n = width(to);
            let out = match (width(from), n) {
                (1, _) => vec![v[0]; n],
                (_, 1) if from.starts_with("color") => {
                    vec![0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]]
                }
                (_, 1) => vec![v.iter().take(3).sum::<f64>() / 3.],
                _ => (0..n).map(|i| v.get(i).copied().unwrap_or(0.)).collect(),
            };
            return Src::Value { v: out, ty: to };
        }
        match (from, to) {
            ("color3", "float") => {
                let l = self.add(
                    &format!("{base}_luma"),
                    "luminance",
                    "color3",
                    vec![
                        ("in", "color3", Some(src)),
                        (
                            "lumacoeffs",
                            "color3",
                            Some(Src::Value {
                                v: vec![0.2126, 0.7152, 0.0722],
                                ty: "color3",
                            }),
                        ),
                    ],
                );
                self.extract(base, l, 0)
            }
            ("vector3", "float") => self.add(
                &format!("{base}_avg"),
                "dotproduct",
                "float",
                vec![
                    ("in1", "vector3", Some(src)),
                    (
                        "in2",
                        "vector3",
                        Some(Src::Value {
                            v: vec![1. / 3.; 3],
                            ty: "vector3",
                        }),
                    ),
                ],
            ),
            ("color4", "float") => self.extract(base, src, 3),
            _ => {
                // convert covers float to anything and the colour/vector
                // pairs.
                let mid = if from == "color4" && to != "color3" {
                    self.cast(base, src, "color3")
                } else {
                    src
                };
                let from = mid.ty();
                if from == to {
                    return mid;
                }
                self.add(
                    &format!("{base}_to_{to}"),
                    "convert",
                    to,
                    vec![("in", from, Some(mid))],
                )
            }
        }
    }

    fn extract(&mut self, base: &str, src: Src, index: i64) -> Src {
        let ty = src.ty();
        let name = self.fresh(&format!("{base}_{index}"));
        self.elements.push(Element {
            category: "extract".into(),
            name: name.clone(),
            ty: "float".into(),
            inputs: vec![
                Self::port("in", ty, &src),
                Port {
                    name: "index".into(),
                    ty: "integer".into(),
                    value: Some(index.to_string()),
                    ..Port::default()
                },
            ],
            ..Element::default()
        });
        Src::Node { name, ty: "float" }
    }

    /// A node's constant setting.
    fn setting(&self, node: usize, socket: &str) -> Option<&FlatValue> {
        self.flat.value(node, socket)
    }
    fn word(&self, node: usize, socket: &str) -> String {
        match self.setting(node, socket) {
            Some(FlatValue::Text(t)) => t.to_ascii_lowercase(),
            _ => String::new(),
        }
    }
    fn flag(&self, node: usize, socket: &str) -> bool {
        matches!(self.setting(node, socket), Some(FlatValue::Bool(true)))
    }

    /// What feeds input `socket` of `node` as type `ty`: its link, else its
    /// value, else `None` (the MaterialX default).
    fn input(&mut self, node: usize, socket: &str, ty: &'static str) -> Option<Src> {
        let link = self
            .flat
            .links
            .iter()
            .find(|l| l.to == node && l.to_socket == socket)
            .map(|l| (l.from, l.from_socket.clone()));
        if let Some((from, out)) = link {
            return match self.output(from, &out) {
                Some(src) => {
                    let base = format!("{}_{socket}", self.label(node));
                    Some(self.cast(&base, src, ty))
                }
                None => {
                    self.notes.insert(format!(
                        "{}'s {socket} (its node has no MaterialX counterpart)",
                        self.label(node)
                    ));
                    None
                }
            };
        }
        let v = match self.setting(node, socket)? {
            FlatValue::Float(x) => Src::float(*x),
            FlatValue::Vec3(v) => Src::Value {
                v: v.to_vec(),
                ty: if ty == "color3" { "color3" } else { "vector3" },
            },
            FlatValue::Int(i) => Src::float(*i as f64),
            FlatValue::Bool(b) => Src::float(f64::from(u8::from(*b))),
            _ => return None,
        };
        Some(self.cast("value", v, ty))
    }

    /// A number input that is `default` when unset.
    fn number(&mut self, node: usize, socket: &str, default: f64) -> Src {
        self.input(node, socket, "float")
            .unwrap_or(Src::float(default))
    }

    /// Output `socket` of `node`, written once; `None` when the node has
    /// no MaterialX counterpart.
    fn output(&mut self, node: usize, socket: &str) -> Option<Src> {
        let key = (node, socket.to_string());
        if let Some(s) = self.memo.get(&key) {
            return s.clone();
        }
        // Guards cycles (none after a flatten) and repeats.
        self.memo.insert(key.clone(), None);
        let s = self.build(node, socket);
        if s.is_none() {
            let n = &self.flat.nodes[node];
            self.notes
                .insert(format!("{} ({}) has no MaterialX node", n.name, n.cycles));
        }
        self.memo.insert(key, s.clone());
        s
    }

    fn build(&mut self, node: usize, socket: &str) -> Option<Src> {
        let cycles = self.flat.nodes[node].cycles;
        let label = self.label(node);
        match cycles {
            "value" => Some(self.input(node, "Value", "float").unwrap_or(Src::float(0.))),
            "color" => Some(self.input(node, "Value", "color3").unwrap_or(Src::Value {
                v: vec![0.; 3],
                ty: "color3",
            })),
            "image_texture" => self.image(node, socket),
            "texture_coordinate" => match socket {
                "UV" => {
                    let uv = self.add(&format!("{label}_uv"), "texcoord", "vector2", vec![]);
                    Some(self.cast(&label, uv, "vector3"))
                }
                "Object" => Some(self.space_node(&label, "position", "object")),
                "Normal" => Some(self.space_node(&label, "normal", "object")),
                _ => None,
            },
            "mapping" => self.mapping(node),
            "math" => self.math(node),
            "mix" | "mix_color" => self.mix(node, cycles == "mix"),
            "mix_float" | "mix_vector" => {
                let ty = if cycles == "mix_float" {
                    "float"
                } else {
                    "vector3"
                };
                let fac = self.number(node, "Factor", 0.5);
                let fac = if self.flag(node, "Use Clamp") {
                    self.clamp(&label, fac)
                } else {
                    fac
                };
                let a = self.input(node, "A", ty);
                let b = self.input(node, "B", ty);
                Some(self.add(
                    &label,
                    "mix",
                    ty,
                    vec![("bg", ty, a), ("fg", ty, b), ("mix", "float", Some(fac))],
                ))
            }
            "invert" => {
                let fac = self.number(node, "Fac", 1.);
                let c = self.input(node, "Color", "color3");
                let inv = self.add(
                    &format!("{label}_invert"),
                    "invert",
                    "color3",
                    vec![("in", "color3", c.clone())],
                );
                Some(self.add(
                    &label,
                    "mix",
                    "color3",
                    vec![
                        ("bg", "color3", c),
                        ("fg", "color3", Some(inv)),
                        ("mix", "float", Some(fac)),
                    ],
                ))
            }
            "separate_color" => {
                if !matches!(self.word(node, "Type").as_str(), "" | "rgb") {
                    return None;
                }
                let index = match socket {
                    "Red" => 0,
                    "Green" => 1,
                    "Blue" => 2,
                    _ => return None,
                };
                let c = self.input(node, "Color", "color3")?;
                Some(self.extract(&label, c, index))
            }
            "combine_color" => {
                if !matches!(self.word(node, "Type").as_str(), "" | "rgb") {
                    return None;
                }
                let r = self.input(node, "Red", "float");
                let g = self.input(node, "Green", "float");
                let b = self.input(node, "Blue", "float");
                Some(self.add(
                    &label,
                    "combine3",
                    "color3",
                    vec![
                        ("in1", "float", r),
                        ("in2", "float", g),
                        ("in3", "float", b),
                    ],
                ))
            }
            "normal_map" => {
                if !matches!(self.word(node, "Space").as_str(), "" | "tangent") {
                    return None;
                }
                let strength = self.number(node, "Strength", 1.);
                let c = self.input(node, "Color", "vector3");
                Some(self.add(
                    &label,
                    "normalmap",
                    "vector3",
                    vec![("in", "vector3", c), ("scale", "float", Some(strength))],
                ))
            }
            _ => None,
        }
    }

    fn space_node(&mut self, label: &str, category: &str, space: &str) -> Src {
        let name = self.fresh(&format!("{label}_{category}"));
        self.elements.push(Element {
            category: category.into(),
            name: name.clone(),
            ty: "vector3".into(),
            inputs: vec![Port {
                name: "space".into(),
                ty: "string".into(),
                value: Some(space.into()),
                ..Port::default()
            }],
            ..Element::default()
        });
        Src::Node {
            name,
            ty: "vector3",
        }
    }

    fn clamp(&mut self, label: &str, src: Src) -> Src {
        let ty = src.ty();
        self.add(
            &format!("{label}_clamp"),
            "clamp",
            ty,
            vec![("in", ty, Some(src))],
        )
    }

    fn image(&mut self, node: usize, socket: &str) -> Option<Src> {
        let label = self.label(node);
        let file = match self.setting(node, "Filename") {
            Some(FlatValue::Text(t)) => t.clone(),
            _ => return None,
        };
        if !matches!(self.word(node, "Projection").as_str(), "" | "flat") {
            return None;
        }
        // One color4 picture node per Image Texture; its colour and alpha
        // are read from it.
        let key = (node, "__picture".to_string());
        let picture = match self.memo.get(&key).cloned().flatten() {
            Some(p) => p,
            None => {
                let space = match self.word(node, "Colorspace").as_str() {
                    "data" => "raw",
                    "scene_linear" => "lin_rec709",
                    _ => "srgb_texture",
                };
                let address = match self.word(node, "Extension").as_str() {
                    "clamp" => "clamp",
                    "black" => "constant",
                    "mirror" => "mirror",
                    _ => "periodic",
                };
                let filter = match self.word(node, "Interpolation").as_str() {
                    "closest" => "closest",
                    "cubic" | "smart" => "cubic",
                    _ => "linear",
                };
                let uv = self.input(node, "Vector", "vector2");
                let name = self.fresh(&label);
                let mut inputs = vec![Port {
                    name: "file".into(),
                    ty: "filename".into(),
                    value: Some(file),
                    colorspace: Some(space.into()),
                    ..Port::default()
                }];
                if let Some(uv) = uv {
                    inputs.push(Self::port("texcoord", "vector2", &uv));
                }
                for (input, value) in [
                    ("uaddressmode", address),
                    ("vaddressmode", address),
                    ("filtertype", filter),
                ] {
                    inputs.push(Port {
                        name: input.into(),
                        ty: "string".into(),
                        value: Some(value.into()),
                        ..Port::default()
                    });
                }
                self.elements.push(Element {
                    category: "image".into(),
                    name: name.clone(),
                    ty: "color4".into(),
                    inputs,
                    ..Element::default()
                });
                let p = Src::Node { name, ty: "color4" };
                self.memo.insert(key, Some(p.clone()));
                p
            }
        };
        match socket {
            "Color" => Some(self.cast(&label, picture, "color3")),
            "Alpha" => Some(self.extract(&label, picture, 3)),
            _ => None,
        }
    }

    /// Mapping: Point, Texture and Vector types with a constant rotation;
    /// Normal and linked rotations have no exact standard form here.
    fn mapping(&mut self, node: usize) -> Option<Src> {
        let label = self.label(node);
        let kind = self.word(node, "Type");
        if self.flat.is_linked(node, "Rotation") {
            return None;
        }
        let rot = match self.setting(node, "Rotation") {
            Some(FlatValue::Vec3(r)) => *r,
            _ => [0.; 3],
        };
        let v = self.input(node, "Vector", "vector3")?;
        let one = Src::Value {
            v: vec![1.; 3],
            ty: "vector3",
        };
        let scale = self.input(node, "Scale", "vector3").unwrap_or(one);
        let loc = self.input(node, "Location", "vector3");
        let rotate = |c: &mut Self, v: Src, axis: usize, angle: f64| -> Src {
            if angle == 0. {
                return v;
            }
            let mut a = vec![0.; 3];
            a[axis] = 1.;
            c.add(
                &format!("{label}_rot{}", ["x", "y", "z"][axis]),
                "rotate3d",
                "vector3",
                vec![
                    ("in", "vector3", Some(v)),
                    ("amount", "float", Some(Src::float(angle.to_degrees()))),
                    (
                        "axis",
                        "vector3",
                        Some(Src::Value {
                            v: a,
                            ty: "vector3",
                        }),
                    ),
                ],
            )
        };
        let bin = |c: &mut Self, op: &str, a: Src, b: Src| -> Src {
            c.add(
                &format!("{label}_{op}"),
                op,
                "vector3",
                vec![("in1", "vector3", Some(a)), ("in2", "vector3", Some(b))],
            )
        };
        match kind.as_str() {
            // T · R · S, rotation X then Y then Z.
            "" | "point" | "vector" => {
                let mut out = bin(self, "multiply", v, scale);
                for (axis, angle) in rot.into_iter().enumerate() {
                    out = rotate(self, out, axis, angle);
                }
                if kind != "vector"
                    && let Some(l) = loc
                {
                    out = bin(self, "add", out, l);
                }
                Some(out)
            }
            // The inverse: S⁻¹ · R⁻¹ · (v − T).
            "texture" => {
                let mut out = match loc {
                    Some(l) => bin(self, "subtract", v, l),
                    None => v,
                };
                for axis in (0..3).rev() {
                    out = rotate(self, out, axis, -rot[axis]);
                }
                Some(bin(self, "divide", out, scale))
            }
            _ => None,
        }
    }

    fn math(&mut self, node: usize) -> Option<Src> {
        let label = self.label(node);
        let op = self.word(node, "Type");
        let a = self.number(node, "Value1", 0.5);
        let b = self.number(node, "Value2", 0.5);
        let bin = |c: &mut Self, cat: &str, a: Src, b: Src| -> Src {
            c.add(
                &label,
                cat,
                "float",
                vec![("in1", "float", Some(a)), ("in2", "float", Some(b))],
            )
        };
        let un = |c: &mut Self, cat: &str, a: Src| -> Src {
            c.add(&label, cat, "float", vec![("in", "float", Some(a))])
        };
        let out = match op.as_str() {
            "" | "add" => bin(self, "add", a, b),
            "subtract" => bin(self, "subtract", a, b),
            "multiply" => bin(self, "multiply", a, b),
            "divide" => bin(self, "divide", a, b),
            "power" => bin(self, "power", a, b),
            "minimum" => bin(self, "min", a, b),
            "maximum" => bin(self, "max", a, b),
            "absolute" => un(self, "absval", a),
            "sine" => un(self, "sin", a),
            "cosine" => un(self, "cos", a),
            "tangent" => un(self, "tan", a),
            "floor" => un(self, "floor", a),
            "ceil" => un(self, "ceil", a),
            "sqrt" => un(self, "sqrt", a),
            "sign" => un(self, "sign", a),
            "exponent" => un(self, "exp", a),
            "multiply_add" => {
                let c = self.number(node, "Value3", 0.5);
                let m = bin(self, "multiply", a, b);
                bin(self, "add", m, c)
            }
            "greater_than" | "less_than" => {
                let (x, y) = if op == "greater_than" { (a, b) } else { (b, a) };
                self.add(
                    &label,
                    "ifgreater",
                    "float",
                    vec![
                        ("value1", "float", Some(x)),
                        ("value2", "float", Some(y)),
                        ("in1", "float", Some(Src::float(1.))),
                        ("in2", "float", Some(Src::float(0.))),
                    ],
                )
            }
            _ => return None,
        };
        Some(if self.flag(node, "Use Clamp") {
            self.clamp(&label, out)
        } else {
            out
        })
    }

    /// Mix Color (MixRGB `mix`, or Mix in colour mode): `Fac`/`Factor`
    /// blends the second colour over the first by the blend mode.
    fn mix(&mut self, node: usize, old: bool) -> Option<Src> {
        let label = self.label(node);
        let (fs, s1, s2) = if old {
            ("Fac", "Color1", "Color2")
        } else {
            ("Factor", "A", "B")
        };
        let mode = self.word(node, "Type");
        let fac = self.number(node, fs, 0.5);
        let fac = if !old && self.flag(node, "Use Clamp") {
            self.clamp(&label, fac)
        } else {
            fac
        };
        let a = self.input(node, s1, "color3");
        let b = self.input(node, s2, "color3");
        let zero = Src::Value {
            v: vec![0.; 3],
            ty: "color3",
        };
        let blended = match mode.as_str() {
            "" | "mix" => b,
            "add" | "multiply" | "subtract" | "divide" | "darken" | "lighten" | "difference" => {
                let cat = match mode.as_str() {
                    "darken" => "min",
                    "lighten" => "max",
                    "difference" => "subtract",
                    m => m,
                };
                let r = self.add(
                    &format!("{label}_{mode}"),
                    cat,
                    "color3",
                    vec![
                        ("in1", "color3", Some(a.clone().unwrap_or(zero.clone()))),
                        ("in2", "color3", Some(b.unwrap_or(zero.clone()))),
                    ],
                );
                Some(if mode == "difference" {
                    self.add(
                        &format!("{label}_abs"),
                        "absval",
                        "color3",
                        vec![("in", "color3", Some(r))],
                    )
                } else {
                    r
                })
            }
            _ => return None,
        };
        let out = self.add(
            &label,
            "mix",
            "color3",
            vec![
                ("bg", "color3", a),
                ("fg", "color3", blended),
                ("mix", "float", Some(fac)),
            ],
        );
        let clamp_result = if old {
            self.flag(node, "Use Clamp")
        } else {
            self.flag(node, "Use Clamp Result")
        };
        Some(if clamp_result {
            self.clamp(&label, out)
        } else {
            out
        })
    }

    /// The surface shader `socket` of `node` reads, as an
    /// open_pbr_surface: a Principled BSDF or an Emission.
    fn surface(&mut self, node: usize, socket: &str) -> Result<String, String> {
        let n = &self.flat.nodes[node];
        let label = n.name.clone();
        let cycles = n.cycles;
        let mut inputs: Vec<(&str, &'static str, Option<Src>)> = Vec::new();
        match (cycles, socket) {
            ("principled_bsdf", _) => {
                let base = self.input(node, "Base Color", "color3");
                let level = self.input(node, "Specular IOR Level", "float");
                let level = level.map(|l| match l {
                    Src::Value { v, .. } => Src::float(v[0] * 2.),
                    l => self.add(
                        &format!("{label}_specular"),
                        "multiply",
                        "float",
                        vec![
                            ("in1", "float", Some(l)),
                            ("in2", "float", Some(Src::float(2.))),
                        ],
                    ),
                });
                let strength = self
                    .input(node, "Emission Strength", "float")
                    .map(|s| match s {
                        Src::Value { v, .. } => Src::float(v[0] / STRENGTH_PER_NIT),
                        s => self.add(
                            &format!("{label}_luminance"),
                            "divide",
                            "float",
                            vec![
                                ("in1", "float", Some(s)),
                                ("in2", "float", Some(Src::float(STRENGTH_PER_NIT))),
                            ],
                        ),
                    });
                let radius = self
                    .input(node, "Subsurface Scale", "float")
                    .map(|s| match s {
                        Src::Value { v, .. } => Src::float(v[0] / DEFAULT_LENGTH_IN_METRES),
                        s => self.add(
                            &format!("{label}_radius"),
                            "divide",
                            "float",
                            vec![
                                ("in1", "float", Some(s)),
                                ("in2", "float", Some(Src::float(DEFAULT_LENGTH_IN_METRES))),
                            ],
                        ),
                    });
                inputs.push(("base_color", "color3", base.clone()));
                // Cycles' transmission and subsurface take the base colour.
                if self.flat.is_linked(node, "Transmission Weight")
                    || self.input(node, "Transmission Weight", "float") != Some(Src::float(0.))
                {
                    inputs.push(("transmission_color", "color3", base.clone()));
                }
                if self.flat.is_linked(node, "Subsurface Weight")
                    || self.input(node, "Subsurface Weight", "float") != Some(Src::float(0.))
                {
                    inputs.push(("subsurface_color", "color3", base));
                }
                inputs.push(("specular_weight", "float", level));
                inputs.push(("emission_luminance", "float", strength));
                inputs.push(("subsurface_radius", "float", radius));
                for (socket, input, ty) in [
                    ("Metallic", "base_metalness", "float"),
                    ("Diffuse Roughness", "base_diffuse_roughness", "float"),
                    ("Specular Tint", "specular_color", "color3"),
                    ("Roughness", "specular_roughness", "float"),
                    ("IOR", "specular_ior", "float"),
                    ("Anisotropic", "specular_roughness_anisotropy", "float"),
                    ("Transmission Weight", "transmission_weight", "float"),
                    ("Subsurface Weight", "subsurface_weight", "float"),
                    ("Subsurface Radius", "subsurface_radius_scale", "color3"),
                    (
                        "Subsurface Anisotropy",
                        "subsurface_scatter_anisotropy",
                        "float",
                    ),
                    ("Sheen Weight", "fuzz_weight", "float"),
                    ("Sheen Tint", "fuzz_color", "color3"),
                    ("Sheen Roughness", "fuzz_roughness", "float"),
                    ("Coat Weight", "coat_weight", "float"),
                    ("Coat Tint", "coat_color", "color3"),
                    ("Coat Roughness", "coat_roughness", "float"),
                    ("Coat IOR", "coat_ior", "float"),
                    ("Emission Color", "emission_color", "color3"),
                    ("Alpha", "geometry_opacity", "float"),
                ] {
                    let src = self.input(node, socket, ty);
                    inputs.push((input, ty, src));
                }
                for (socket, input) in [
                    ("Normal", "geometry_normal"),
                    ("Coat Normal", "geometry_coat_normal"),
                    ("Tangent", "geometry_tangent"),
                ] {
                    if self.flat.is_linked(node, socket) {
                        let src = self.input(node, socket, "vector3");
                        inputs.push((input, "vector3", src));
                    }
                }
                if self.flat.is_linked(node, "Thin Film Thickness")
                    || matches!(self.setting(node, "Thin Film Thickness"), Some(FlatValue::Float(t)) if *t != 0.)
                {
                    self.notes.insert(format!("{label}'s thin film"));
                }
            }
            ("emission", _) => {
                let color = self.input(node, "Color", "color3");
                let strength = self.number(node, "Strength", 1.);
                let strength = match strength {
                    Src::Value { v, .. } => Src::float(v[0] / STRENGTH_PER_NIT),
                    s => self.add(
                        &format!("{label}_luminance"),
                        "divide",
                        "float",
                        vec![
                            ("in1", "float", Some(s)),
                            ("in2", "float", Some(Src::float(STRENGTH_PER_NIT))),
                        ],
                    ),
                };
                inputs.push(("base_weight", "float", Some(Src::float(0.))));
                inputs.push(("specular_weight", "float", Some(Src::float(0.))));
                inputs.push(("emission_color", "color3", color));
                inputs.push(("emission_luminance", "float", Some(strength)));
            }
            _ => {
                return Err(format!(
                    "its surface shader, {label} ({cycles}), has no MaterialX node"
                ));
            }
        }
        match self.add(&label, "open_pbr_surface", "surfaceshader", inputs) {
            Src::Node { name, .. } => Ok(name),
            Src::Value { .. } => unreachable!("add makes a node"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::node_graph::{FlatLink, FlatNode};

    fn node(cycles: &'static str, name: &str, values: Vec<(&str, FlatValue)>) -> FlatNode {
        FlatNode {
            cycles,
            name: name.into(),
            values: values
                .into_iter()
                .map(|(s, v)| (s.to_string(), v))
                .collect(),
        }
    }
    fn link(from: usize, fs: &str, to: usize, ts: &str) -> FlatLink {
        FlatLink {
            from,
            from_socket: fs.into(),
            to,
            to_socket: ts.into(),
        }
    }

    #[test]
    fn principled_math_and_pictures_map_and_the_rest_is_named() {
        let flat = FlatGraph {
            nodes: vec![
                node("output", "Material Output", vec![]),
                node(
                    "principled_bsdf",
                    "Principled BSDF",
                    vec![
                        ("Metallic", FlatValue::Float(1.)),
                        ("Specular IOR Level", FlatValue::Float(0.25)),
                    ],
                ),
                node(
                    "image_texture",
                    "Image Texture",
                    vec![
                        ("Filename", FlatValue::Text("/p/wood.png".into())),
                        ("Colorspace", FlatValue::Text("__builtin_srgb".into())),
                    ],
                ),
                node(
                    "math",
                    "Math",
                    vec![
                        ("Type", FlatValue::Text("multiply".into())),
                        ("Value2", FlatValue::Float(0.5)),
                    ],
                ),
                node("noise_texture", "Noise Texture", vec![]),
            ],
            links: vec![
                link(1, "BSDF", 0, "Surface"),
                link(2, "Color", 1, "Base Color"),
                link(2, "Alpha", 3, "Value1"),
                link(3, "Value", 1, "Roughness"),
                link(4, "Fac", 1, "Coat Weight"),
            ],
        };
        let (doc, notes) = document(&flat, "Wood").unwrap();
        let s = doc
            .elements
            .iter()
            .find(|e| e.category == "open_pbr_surface")
            .unwrap();
        assert_eq!(
            s.input("base_metalness").unwrap().value.as_deref(),
            Some("1")
        );
        assert_eq!(
            s.input("specular_weight").unwrap().value.as_deref(),
            Some("0.5")
        );
        assert!(s.input("base_color").unwrap().nodename.is_some());
        assert!(s.input("coat_weight").is_none());
        let m = doc
            .elements
            .iter()
            .find(|e| e.category == "multiply")
            .unwrap();
        assert_eq!(m.input("in2").unwrap().value.as_deref(), Some("0.5"));
        let image = doc.elements.iter().find(|e| e.category == "image").unwrap();
        assert_eq!(image.ty, "color4");
        assert_eq!(
            image.input("file").unwrap().colorspace.as_deref(),
            Some("srgb_texture")
        );
        assert!(
            notes.iter().any(|n| n.contains("Noise Texture")),
            "{notes:?}"
        );
        // It is a network the USD writer takes.
        let net = crate::materialx_graph_out::flat_network(&doc, "Wood", std::path::Path::new(""))
            .unwrap();
        assert!(net.nodes.iter().any(|n| n.id == "ND_image_color4"));
    }

    #[test]
    fn a_glass_surface_is_refused() {
        let flat = FlatGraph {
            nodes: vec![
                node("output", "Material Output", vec![]),
                node("glass_bsdf", "Glass", vec![]),
            ],
            links: vec![link(1, "BSDF", 0, "Surface")],
        };
        assert!(document(&flat, "G").unwrap_err().contains("Glass"));
    }
}
