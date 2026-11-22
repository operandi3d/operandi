//! MaterialX graphs as Cycles nodes (materials architecture, slice 1;
//! docs/MATERIALX_GRAPHS.md).
//!
//! [`translate`] turns one material of a MaterialX document into the
//! plain Cycles node list the render bridge already builds from
//! ([`FlatGraph`], NM-30), so a MaterialX graph renders without any change
//! on the C++ side. Built from public documentation only: the MaterialX
//! specification and standard library (Apache-2.0, the node definitions
//! bundled in `crates/operandi-core/materialx`), the OpenPBR specification
//! and Cycles' own node definitions (Apache-2.0).
//!
//! - Surfaces: `open_pbr_surface` lands on Principled BSDF through the
//!   same OpenPBR table as the shader tree (`openpbr::cycles_target`);
//!   `standard_surface` first runs through MaterialX's published
//!   standard_surface -> open_pbr_surface graph; `UsdPreviewSurface` and
//!   `gltf_pbr` map their inputs directly. A `mix` of two surfaces is a
//!   Mix Shader.
//! - Everything feeding a surface is translated node by node, constants
//!   folded as they go (a graph of only values gives a Principled BSDF with
//!   values, no maths nodes). Nodegraphs, `interfacename` and nodes whose
//!   definition the document implements as a nodegraph are expanded in
//!   place.
//! - A node with no translation refuses the whole material, naming it:
//!   nothing half-renders.
//!
//! Approximations are listed in [`Translated::notes`]: MaterialX's noises
//! become Cycles' noise (same idea, different pattern), `heighttonormal`
//! becomes a Bump node, and OpenPBR inputs Cycles has no slot for (coat
//! darkening, transmission depth and scatter) are left out.
use super::colour;
use super::doc::{Document, Element, Port, numbers};
use super::stdlib;
use crate::node_graph::{FlatGraph, FlatLink, FlatNode, FlatValue};
use crate::openpbr::{self, Param};
use std::collections::BTreeMap;
use std::path::Path;

/// Cycles nodes one material may become (a sanity budget).
pub const MAX_NODES: usize = 4096;
/// How deep a chain of nodes and nodegraphs may go.
const MAX_DEPTH: usize = 256;

/// Why a material can't be translated: the node it stopped at (empty for
/// a problem with the document itself) and what is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub node: String,
    pub category: String,
    pub reason: String,
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.node.is_empty() {
            f.write_str(&self.reason)
        } else {
            write!(f, "{} ({}): {}", self.node, self.category, self.reason)
        }
    }
}

/// A translated material.
#[derive(Clone, Debug, PartialEq)]
pub struct Translated {
    /// The material's name.
    pub name: String,
    pub graph: FlatGraph,
    /// What was approximated or left out, one line each.
    pub notes: Vec<String>,
}

type R<T> = std::result::Result<T, Refusal>;

fn refuse(el: &Element, reason: impl Into<String>) -> Refusal {
    Refusal {
        node: el.name.clone(),
        category: el.category.clone(),
        reason: reason.into(),
    }
}

fn problem(reason: impl Into<String>) -> Refusal {
    Refusal {
        node: String::new(),
        category: String::new(),
        reason: reason.into(),
    }
}

/// The surface materials a document holds.
pub fn materials(doc: &Document) -> Vec<&Element> {
    doc.all("surfacematerial").collect()
}

/// What one node output gives as Cycles nodes ([`translate_output`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// Socket `socket` of node `node` carries it.
    Socket { node: usize, socket: String },
    /// It folded to a value.
    Const(Vec<f64>),
    /// Nothing sets it (an unconnected geometry default).
    Unset,
}

/// Output `output` of top-level node `node` of `doc` as Cycles nodes,
/// for a free node driving a tree channel (node editing, NE-03): the
/// nodes it needs and where the value comes out.
pub fn translate_output(
    doc: &Document,
    node: &str,
    output: &str,
    folder: &Path,
) -> R<(FlatGraph, Output)> {
    let mut tx = Tx::new(doc, folder);
    let src = tx.node_out(0, node, output)?;
    let out = match src {
        Src::Unset => Output::Unset,
        Src::Const(c) => Output::Const(c),
        Src::Link(o) => Output::Socket {
            node: o.node,
            socket: o.socket.to_string(),
        },
    };
    Ok((
        FlatGraph {
            nodes: tx.nodes,
            links: tx.links,
        },
        out,
    ))
}

/// The material `material` (the first when `None`) as Cycles nodes.
/// Picture paths resolve against `folder`.
pub fn translate(doc: &Document, material: Option<&str>, folder: &Path) -> R<Translated> {
    let mats = materials(doc);
    let mat = match material {
        Some(name) => mats.iter().find(|m| m.name == name).copied(),
        None => mats.first().copied(),
    };
    let mut tx = Tx::new(doc, folder);
    let out = tx.add("output", "Material Output")?;
    let name = match mat {
        Some(m) => {
            let shader = m
                .input("surfaceshader")
                .ok_or_else(|| refuse(m, "the material has no surface shader"))?;
            let surface = tx.port(0, shader)?;
            tx.put(out, "Surface", &surface, 0);
            if let Some(d) = m.input("displacementshader") {
                let disp = tx.port(0, d)?;
                tx.put(out, "Displacement", &disp, 3);
            }
            m.name.clone()
        }
        // A file with only a shader node is read as that shader.
        None => {
            let shader = doc
                .elements
                .iter()
                .find(|e| e.ty == "surfaceshader" && e.category != "nodedef")
                .ok_or_else(|| problem("The MaterialX file has no material"))?;
            let surface = tx.node_out(0, &shader.name, "out")?;
            tx.put(out, "Surface", &surface, 0);
            shader.name.clone()
        }
    };
    if !tx
        .links
        .iter()
        .any(|l| l.to == out && l.to_socket == "Surface")
    {
        return Err(problem(format!("MaterialX material {name} has no surface")));
    }
    let mut notes = tx.notes;
    notes.sort();
    notes.dedup();
    Ok(Translated {
        name,
        graph: FlatGraph {
            nodes: tx.nodes,
            links: tx.links,
        },
        notes,
    })
}

/// A value on its way through the graph.
#[derive(Clone, Debug, PartialEq)]
enum Src {
    /// Left at Cycles' default (an unconnected geometry input).
    Unset,
    Const(Vec<f64>),
    Link(Out),
}

/// A Cycles output socket, with the MaterialX value size it carries and,
/// for a four-channel value, where its fourth channel comes from.
#[derive(Clone, Debug, PartialEq)]
struct Out {
    node: usize,
    socket: &'static str,
    size: usize,
    alpha: Option<(usize, &'static str)>,
}

fn link(node: usize, socket: &'static str, size: usize) -> Src {
    Src::Link(Out {
        node,
        socket,
        size,
        alpha: None,
    })
}

/// The number of channels of a MaterialX type (shaders 0).
fn size_of(ty: &str) -> usize {
    match ty {
        "float" | "integer" | "boolean" => 1,
        "vector2" => 2,
        "color3" | "vector3" => 3,
        "color4" | "vector4" => 4,
        _ => 0,
    }
}

/// `v` as `n` numbers: a single number repeats, missing ones are 0.
fn fit(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    match v.len() {
        0 => vec![0.; n],
        1 => vec![v[0]; n],
        _ => (0..n).map(|i| v.get(i).copied().unwrap_or(0.)).collect(),
    }
}

fn konst(s: &Src) -> Option<&[f64]> {
    match s {
        Src::Const(c) => Some(c),
        _ => None,
    }
}

fn is_all(s: &Src, x: f64) -> bool {
    konst(s).is_some_and(|c| !c.is_empty() && c.iter().all(|v| *v == x))
}

fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    let n = a.len().max(b.len());
    let (a, b) = (fit(a, n), fit(b, n));
    a.iter().zip(&b).map(|(x, y)| f(*x, *y)).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Min,
    Max,
    Mod,
}
impl Op {
    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div => {
                if b == 0. {
                    0.
                } else {
                    a / b
                }
            }
            Op::Pow => a.powf(b),
            Op::Min => a.min(b),
            Op::Max => a.max(b),
            Op::Mod => {
                if b == 0. {
                    0.
                } else {
                    a - b * (a / b).floor()
                }
            }
        }
    }
    /// The Cycles Math / Vector Math type.
    fn cycles(self, scalar: bool) -> &'static str {
        match self {
            Op::Add => "add",
            Op::Sub => "subtract",
            Op::Mul => "multiply",
            Op::Div => "divide",
            Op::Pow => "power",
            Op::Min => "minimum",
            Op::Max => "maximum",
            // MaterialX's modulo is floored; Vector Math has only fmod.
            Op::Mod if scalar => "floored_modulo",
            Op::Mod => "modulo",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Un {
    Abs,
    Floor,
    Ceil,
    Sqrt,
    Sin,
    Cos,
    Sign,
    Exp,
    Ln,
}
impl Un {
    fn apply(self, a: f64) -> f64 {
        match self {
            Un::Abs => a.abs(),
            Un::Floor => a.floor(),
            Un::Ceil => a.ceil(),
            Un::Sqrt => a.max(0.).sqrt(),
            Un::Sin => a.sin(),
            Un::Cos => a.cos(),
            Un::Sign => {
                if a > 0. {
                    1.
                } else if a < 0. {
                    -1.
                } else {
                    0.
                }
            }
            Un::Exp => a.exp(),
            Un::Ln => {
                if a > 0. {
                    a.ln()
                } else {
                    0.
                }
            }
        }
    }
    fn cycles(self) -> &'static str {
        match self {
            Un::Abs => "absolute",
            Un::Floor => "floor",
            Un::Ceil => "ceil",
            Un::Sqrt => "sqrt",
            Un::Sin => "sine",
            Un::Cos => "cosine",
            Un::Sign => "sign",
            Un::Exp => "exponent",
            Un::Ln => "logarithm",
        }
    }
    /// Vector Math has these per component.
    fn per_component(self) -> bool {
        matches!(
            self,
            Un::Abs | Un::Floor | Un::Ceil | Un::Sin | Un::Cos | Un::Sign
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmp {
    Greater,
    GreaterEq,
    Equal,
}

/// Where `interfacename` inputs of a node list resolve.
enum Iface<'a> {
    /// The document's top level: there is no interface.
    Top,
    /// A top-level nodegraph: its inputs, resolved at the top level.
    Graph(&'a Element),
    /// The implementation nodegraph of a node: that node's inputs (in its
    /// own frame), else the definition's defaults.
    Instance {
        node: &'a Element,
        at: usize,
        def: &'a Element,
    },
}

struct Frame<'a> {
    elements: &'a [Element],
    iface: Iface<'a>,
}

struct Tx<'a> {
    doc: &'a Document,
    folder: &'a Path,
    frames: Vec<Frame<'a>>,
    graph_frames: BTreeMap<String, usize>,
    instance_frames: BTreeMap<(usize, String), usize>,
    memo: BTreeMap<(usize, String, String), Src>,
    busy: Vec<(usize, String, String)>,
    nodes: Vec<FlatNode>,
    links: Vec<FlatLink>,
    notes: Vec<String>,
    texcoord: Option<usize>,
    geometry: Option<usize>,
}

impl<'a> Tx<'a> {
    fn new(doc: &'a Document, folder: &'a Path) -> Self {
        Self {
            doc,
            folder,
            frames: vec![Frame {
                elements: &doc.elements,
                iface: Iface::Top,
            }],
            graph_frames: BTreeMap::new(),
            instance_frames: BTreeMap::new(),
            memo: BTreeMap::new(),
            busy: Vec::new(),
            nodes: Vec::new(),
            links: Vec::new(),
            notes: Vec::new(),
            texcoord: None,
            geometry: None,
        }
    }

    // -- Building Cycles nodes ------------------------------------------

    fn add(&mut self, cycles: &'static str, name: &str) -> R<usize> {
        if self.nodes.len() >= MAX_NODES {
            return Err(problem("The MaterialX material is too big"));
        }
        self.nodes.push(FlatNode {
            cycles,
            name: name.to_string(),
            values: Vec::new(),
        });
        Ok(self.nodes.len() - 1)
    }

    fn set(&mut self, node: usize, socket: &str, v: FlatValue) {
        self.nodes[node].values.push((socket.to_string(), v));
    }

    fn text(&mut self, node: usize, socket: &str, v: &str) {
        self.set(node, socket, FlatValue::Text(v.to_string()));
    }

    /// Feed `src` to `socket` of `node`; `want` is the socket's channel
    /// count (0 for a shader, 1 a number, 3 a colour or vector).
    fn put(&mut self, node: usize, socket: &str, src: &Src, want: usize) {
        match src {
            Src::Unset => {}
            Src::Const(c) if want > 0 => {
                let v = fit(c, want);
                let value = if want == 1 {
                    FlatValue::Float(v[0])
                } else {
                    FlatValue::Vec3([v[0], v[1], v[2]])
                };
                self.set(node, socket, value);
            }
            Src::Const(_) => {}
            Src::Link(o) => self.links.push(FlatLink {
                from: o.node,
                from_socket: o.socket.to_string(),
                to: node,
                to_socket: socket.to_string(),
            }),
        }
    }

    // -- Resolving MaterialX references ---------------------------------

    /// What a port gives: a connection, an interface input or its value.
    /// Strings and filenames give `Unset` (read them with [`Self::word`]).
    fn port(&mut self, f: usize, p: &'a Port) -> R<Src> {
        if let Some(name) = &p.interfacename {
            return self.interface(f, name);
        }
        if let Some(g) = &p.nodegraph {
            return self.graph_out(g, p.output.as_deref());
        }
        if let Some(n) = &p.nodename {
            return self.node_out(f, n, p.output.as_deref().unwrap_or("out"));
        }
        let Some(v) = &p.value else {
            return Ok(Src::Unset);
        };
        if size_of(&p.ty) == 0 {
            return Ok(Src::Unset);
        }
        let mut n = numbers(v).ok_or_else(|| {
            problem(format!(
                "MaterialX input {} has a value that isn't a number",
                p.name
            ))
        })?;
        if matches!(p.ty.as_str(), "color3" | "color4")
            && n.len() >= 3
            && let Some(space) = &p.colorspace
        {
            match colour::to_lin_rec709([n[0], n[1], n[2]], space) {
                Some(c) => n[..3].copy_from_slice(&c),
                None => self
                    .notes
                    .push(format!("colour space {space} not known; used as it is")),
            }
        }
        Ok(Src::Const(n))
    }

    fn interface(&mut self, f: usize, name: &str) -> R<Src> {
        match self.frames[f].iface {
            Iface::Top => Err(problem(format!(
                "MaterialX input reads interface “{name}” outside a nodegraph"
            ))),
            Iface::Graph(ng) => match ng.input(name) {
                Some(p) => self.port(0, p),
                None => Ok(Src::Unset),
            },
            Iface::Instance { node, at, def } => match node.input(name) {
                Some(p) => self.port(at, p),
                None => match def.input(name) {
                    Some(p) => self.port(0, p),
                    None => Ok(Src::Unset),
                },
            },
        }
    }

    /// Output `output` (or the first) of top-level nodegraph `name`.
    fn graph_out(&mut self, name: &str, output: Option<&str>) -> R<Src> {
        let doc = self.doc;
        let ng = doc
            .elements
            .iter()
            .find(|e| e.category == "nodegraph" && e.name == name)
            .ok_or_else(|| problem(format!("MaterialX nodegraph {name} isn't in the file")))?;
        let frame = match self.graph_frames.get(name) {
            Some(&k) => k,
            None => {
                self.frames.push(Frame {
                    elements: &ng.children,
                    iface: Iface::Graph(ng),
                });
                let k = self.frames.len() - 1;
                self.graph_frames.insert(name.to_string(), k);
                k
            }
        };
        let out = match output {
            Some(o) => ng.outputs.iter().find(|p| p.name == o),
            None => ng.outputs.first(),
        }
        .ok_or_else(|| refuse(ng, "the nodegraph has no such output"))?;
        self.port(frame, out)
    }

    /// Output `output` of node `name` in frame `f`, translated once.
    fn node_out(&mut self, f: usize, name: &str, output: &str) -> R<Src> {
        let key = (f, name.to_string(), output.to_string());
        if let Some(s) = self.memo.get(&key) {
            return Ok(s.clone());
        }
        if self.busy.contains(&key) {
            return Err(problem(format!("MaterialX node {name} feeds itself")));
        }
        if self.busy.len() >= MAX_DEPTH {
            return Err(problem("The MaterialX graph is too deep"));
        }
        let elements = self.frames[f].elements;
        let el = elements
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| problem(format!("MaterialX node {name} isn't in the file")))?;
        self.busy.push(key.clone());
        let r = self.node(f, el, output);
        self.busy.pop();
        let s = r?;
        self.memo.insert(key, s.clone());
        Ok(s)
    }

    /// Input `name` of `el`: its connection or value, else `default`.
    fn inp(&mut self, f: usize, el: &'a Element, name: &str, default: &[f64]) -> R<Src> {
        let s = match el.input(name) {
            Some(p) => self.port(f, p)?,
            None => Src::Unset,
        };
        Ok(match s {
            Src::Unset => Src::Const(default.to_vec()),
            s => s,
        })
    }

    /// Input `name` of `el`, `Unset` when it has none.
    fn opt(&mut self, f: usize, el: &'a Element, name: &str) -> R<Src> {
        match el.input(name) {
            Some(p) => self.port(f, p),
            None => Ok(Src::Unset),
        }
    }

    /// A string input's value.
    fn word(el: &Element, name: &str) -> Option<String> {
        el.input(name)
            .and_then(|p| p.value.clone())
            .filter(|v| !v.is_empty())
    }

    /// A constant input: refused when something drives it.
    fn fixed(&mut self, f: usize, el: &'a Element, name: &str, default: &[f64]) -> R<Vec<f64>> {
        match self.inp(f, el, name, default)? {
            Src::Const(c) => Ok(c),
            _ => Err(refuse(el, format!("{name} must be a value here"))),
        }
    }

    // -- Maths, folding constants ---------------------------------------

    fn bin(&mut self, name: &str, op: Op, a: Src, b: Src, size: usize) -> R<Src> {
        let a = if a == Src::Unset {
            Src::Const(vec![0.])
        } else {
            a
        };
        let b = if b == Src::Unset {
            Src::Const(vec![0.])
        } else {
            b
        };
        if let (Some(x), Some(y)) = (konst(&a), konst(&b)) {
            return Ok(Src::Const(zip(x, y, |p, q| op.apply(p, q))));
        }
        // Identities keep graphs small.
        match op {
            Op::Mul | Op::Div | Op::Pow if is_all(&b, 1.) => return Ok(a),
            Op::Mul if is_all(&a, 1.) => return Ok(b),
            Op::Add | Op::Sub if is_all(&b, 0.) => return Ok(a),
            Op::Add if is_all(&a, 0.) => return Ok(b),
            _ => {}
        }
        if size <= 1 {
            let n = self.add("math", name)?;
            self.text(n, "Type", op.cycles(true));
            self.put(n, "Value1", &a, 1);
            self.put(n, "Value2", &b, 1);
            return Ok(link(n, "Value", 1));
        }
        if op == Op::Pow {
            // A colour to a single power is Cycles' Gamma node.
            let scalar = match &b {
                Src::Const(c) => c.iter().all(|v| *v == c[0]),
                Src::Link(o) => o.size == 1,
                Src::Unset => true,
            };
            if !scalar {
                return Err(problem(format!(
                    "MaterialX node {name}: a power per channel can't render yet"
                )));
            }
            let n = self.add("gamma", name)?;
            self.put(n, "Color", &a, 3);
            self.put(n, "Gamma", &b, 1);
            return Ok(link(n, "Color", size));
        }
        let n = self.add("vector_math", name)?;
        self.text(n, "Type", op.cycles(false));
        self.put(n, "Vector1", &a, 3);
        self.put(n, "Vector2", &b, 3);
        Ok(link(n, "Vector", size))
    }

    fn un(&mut self, el: &Element, op: Un, a: Src, size: usize) -> R<Src> {
        if let Some(c) = konst(&a) {
            return Ok(Src::Const(c.iter().map(|v| op.apply(*v)).collect()));
        }
        if size <= 1 {
            let n = self.add("math", &el.name)?;
            self.text(n, "Type", op.cycles());
            self.put(n, "Value1", &a, 1);
            if op == Un::Ln {
                self.set(n, "Value2", FlatValue::Float(std::f64::consts::E));
            }
            return Ok(link(n, "Value", 1));
        }
        if !op.per_component() {
            return Err(refuse(el, "can't render on colours or vectors yet"));
        }
        let n = self.add("vector_math", &el.name)?;
        self.text(n, "Type", op.cycles());
        self.put(n, "Vector1", &a, 3);
        Ok(link(n, "Vector", size))
    }

    /// `bg` to `fg` by `k` (MaterialX `mix`).
    fn mix(&mut self, name: &str, bg: Src, fg: Src, k: Src, size: usize, clamp: bool) -> R<Src> {
        if let (Some(b), Some(f), Some(t)) = (konst(&bg), konst(&fg), konst(&k)) {
            let t = if clamp {
                t.iter().map(|v| v.clamp(0., 1.)).collect()
            } else {
                t.to_vec()
            };
            let d = zip(f, b, |x, y| x - y);
            let s = zip(&d, &t, |x, y| x * y);
            return Ok(Src::Const(zip(b, &s, |x, y| x + y)));
        }
        if is_all(&k, 0.) {
            return Ok(bg);
        }
        if is_all(&k, 1.) {
            return Ok(fg);
        }
        if konst(&k).is_some_and(|c| c.len() > 1 && c.iter().any(|v| *v != c[0]))
            || matches!(&k, Src::Link(o) if o.size > 1)
        {
            return Err(problem(format!(
                "MaterialX node {name}: a mix per channel can't render yet"
            )));
        }
        let n = if size <= 1 {
            self.add("mix_float", name)?
        } else {
            let n = self.add("mix_color", name)?;
            self.text(n, "Type", "mix");
            n
        };
        self.set(n, "Use Clamp", FlatValue::Bool(clamp));
        self.put(n, "Factor", &k, 1);
        self.put(n, "A", &bg, size.clamp(1, 3));
        self.put(n, "B", &fg, size.clamp(1, 3));
        Ok(link(n, "Result", size))
    }

    /// 1 where the comparison holds, else 0.
    fn cond(&mut self, name: &str, cmp: Cmp, a: Src, b: Src) -> R<Src> {
        if let (Some(x), Some(y)) = (konst(&a), konst(&b)) {
            let (x, y) = (
                x.first().copied().unwrap_or(0.),
                y.first().copied().unwrap_or(0.),
            );
            let holds = match cmp {
                Cmp::Greater => x > y,
                Cmp::GreaterEq => x >= y,
                Cmp::Equal => x == y,
            };
            return Ok(Src::Const(vec![f64::from(u8::from(holds))]));
        }
        let n = self.add("math", name)?;
        match cmp {
            Cmp::Greater => self.text(n, "Type", "greater_than"),
            Cmp::GreaterEq => self.text(n, "Type", "less_than"),
            Cmp::Equal => {
                self.text(n, "Type", "compare");
                self.set(n, "Value3", FlatValue::Float(0.));
            }
        }
        self.put(n, "Value1", &a, 1);
        self.put(n, "Value2", &b, 1);
        let out = link(n, "Value", 1);
        if cmp == Cmp::GreaterEq {
            // a >= b is 1 - (a < b).
            return self.bin(name, Op::Sub, Src::Const(vec![1.]), out, 1);
        }
        Ok(out)
    }

    /// Channel `index` of `s`.
    fn channel(&mut self, name: &str, s: Src, index: usize) -> R<Src> {
        match s {
            Src::Unset => Ok(Src::Const(vec![0.])),
            Src::Const(c) => Ok(Src::Const(vec![c.get(index).copied().unwrap_or(0.)])),
            Src::Link(o) if index == 3 => match o.alpha {
                Some((node, socket)) => Ok(link(node, socket, 1)),
                None => Ok(Src::Const(vec![1.])),
            },
            Src::Link(o) if o.size <= 1 => Ok(Src::Link(o)),
            Src::Link(o) => {
                let n = self.add("separate_xyz", name)?;
                self.put(n, "Vector", &Src::Link(o), 3);
                Ok(link(n, ["X", "Y", "Z"][index.min(2)], 1))
            }
        }
    }

    /// `s` as a value of `size` channels.
    fn resize(s: Src, size: usize) -> Src {
        match s {
            Src::Const(c) => Src::Const(fit(&c, size)),
            Src::Link(mut o) => {
                if size < 4 {
                    o.alpha = None;
                }
                o.size = size;
                Src::Link(o)
            }
            Src::Unset => Src::Unset,
        }
    }

    // -- Geometry --------------------------------------------------------

    /// The mesh UVs (`texcoord` 0).
    fn uv(&mut self) -> R<Src> {
        let n = match self.texcoord {
            Some(n) => n,
            None => {
                let n = self.add("texture_coordinate", "Texture Coordinate")?;
                self.texcoord = Some(n);
                n
            }
        };
        Ok(link(n, "UV", 2))
    }

    fn object_coords(&mut self, socket: &'static str) -> R<Src> {
        let n = match self.texcoord {
            Some(n) => n,
            None => {
                let n = self.add("texture_coordinate", "Texture Coordinate")?;
                self.texcoord = Some(n);
                n
            }
        };
        Ok(link(n, socket, 3))
    }

    fn world(&mut self, socket: &'static str) -> R<Src> {
        let n = match self.geometry {
            Some(n) => n,
            None => {
                let n = self.add("geometry", "Geometry")?;
                self.geometry = Some(n);
                n
            }
        };
        Ok(link(n, socket, 3))
    }

    /// `v` scaled, rotated `degrees` about Z and moved (Cycles' Mapping
    /// node, Point type: R(v × scale) + location).
    fn mapping(&mut self, name: &str, v: Src, scale: Src, degrees: f64, location: Src) -> R<Src> {
        if is_all(&scale, 1.) && degrees == 0. && is_all(&location, 0.) {
            return Ok(v);
        }
        let n = self.add("mapping", name)?;
        self.text(n, "Type", "point");
        self.put(n, "Vector", &v, 3);
        self.put(n, "Scale", &Self::pad_scale(scale), 3);
        self.set(
            n,
            "Rotation",
            FlatValue::Vec3([0., 0., degrees.to_radians()]),
        );
        self.put(n, "Location", &location, 3);
        Ok(link(n, "Vector", 2))
    }

    /// A two-channel scale with Z kept at 1.
    fn pad_scale(s: Src) -> Src {
        match s {
            Src::Const(c) => {
                let c = fit(&c, 2);
                Src::Const(vec![c[0], c[1], 1.])
            }
            s => s,
        }
    }

    // -- Nodes -----------------------------------------------------------

    fn node(&mut self, f: usize, el: &'a Element, output: &str) -> R<Src> {
        let n = size_of(&el.ty);
        let zero = vec![0.; n.max(1)];
        let one = vec![1.; n.max(1)];
        let name = el.name.as_str();
        match el.category.as_str() {
            "constant" => self.inp(f, el, "value", &zero),
            "dot" => {
                if el.ty == "surfaceshader" || el.ty == "displacementshader" {
                    return match self.opt(f, el, "in")? {
                        Src::Unset => Err(refuse(el, "nothing feeds it")),
                        s => Ok(s),
                    };
                }
                self.inp(f, el, "in", &zero)
            }
            "convert" => {
                let s = self.inp(f, el, "in", &[0.])?;
                Ok(Self::resize(s, n))
            }
            "extract" => {
                let s = self.inp(f, el, "in", &[0.])?;
                let i = self.fixed(f, el, "index", &[0.])?[0].max(0.) as usize;
                self.channel(name, s, i)
            }
            "separate2" | "separate3" | "separate4" => {
                let s = self.inp(f, el, "in", &[0.])?;
                let i = match output.chars().last() {
                    Some('r' | 'x') => 0,
                    Some('g' | 'y') => 1,
                    Some('b' | 'z') => 2,
                    Some('a' | 'w') => 3,
                    _ => return Err(refuse(el, format!("no output {output}"))),
                };
                self.channel(name, s, i)
            }
            "combine2" | "combine3" | "combine4" => {
                let parts: Vec<Src> = (1..=n.min(4))
                    .map(|k| self.inp(f, el, &format!("in{k}"), &[0.]))
                    .collect::<R<_>>()?;
                if parts.iter().all(|p| konst(p).is_some()) {
                    return Ok(Src::Const(
                        parts
                            .iter()
                            .map(|p| konst(p).and_then(|c| c.first().copied()).unwrap_or(0.))
                            .collect(),
                    ));
                }
                let c = self.add("combine_xyz", name)?;
                for (k, axis) in ["X", "Y", "Z"].iter().enumerate().take(n.min(3)) {
                    self.put(c, axis, &parts[k], 1);
                }
                let alpha = match parts.get(3) {
                    Some(Src::Link(o)) => Some((o.node, o.socket)),
                    _ => None,
                };
                Ok(Src::Link(Out {
                    node: c,
                    socket: "Vector",
                    size: n,
                    alpha,
                }))
            }
            cat @ ("add" | "subtract" | "multiply" | "divide" | "power" | "min" | "max"
            | "modulo") => {
                let op = match cat {
                    "add" => Op::Add,
                    "subtract" => Op::Sub,
                    "multiply" => Op::Mul,
                    "divide" => Op::Div,
                    "power" => Op::Pow,
                    "min" => Op::Min,
                    "max" => Op::Max,
                    _ => Op::Mod,
                };
                let second = if matches!(op, Op::Mul | Op::Div | Op::Pow | Op::Mod) {
                    &one
                } else {
                    &zero
                };
                let a = self.inp(f, el, "in1", &zero)?;
                let b = self.inp(f, el, "in2", second)?;
                self.bin(name, op, a, b, n)
            }
            "invert" => {
                let a = self.inp(f, el, "in", &zero)?;
                let amount = self.inp(f, el, "amount", &one)?;
                self.bin(name, Op::Sub, amount, a, n)
            }
            cat
            @ ("absval" | "floor" | "ceil" | "sqrt" | "sin" | "cos" | "sign" | "exp" | "ln") => {
                let op = match cat {
                    "absval" => Un::Abs,
                    "floor" => Un::Floor,
                    "ceil" => Un::Ceil,
                    "sqrt" => Un::Sqrt,
                    "sin" => Un::Sin,
                    "cos" => Un::Cos,
                    "sign" => Un::Sign,
                    "exp" => Un::Exp,
                    _ => Un::Ln,
                };
                let a = self.inp(f, el, "in", &zero)?;
                self.un(el, op, a, n)
            }
            "clamp" => {
                let a = self.inp(f, el, "in", &zero)?;
                let low = self.inp(f, el, "low", &zero)?;
                let high = self.inp(f, el, "high", &one)?;
                let up = self.bin(name, Op::Max, a, low, n)?;
                self.bin(name, Op::Min, up, high, n)
            }
            "mix" if el.ty == "surfaceshader" => {
                let bg = self.opt(f, el, "bg")?;
                let fg = self.opt(f, el, "fg")?;
                let k = self.inp(f, el, "mix", &[0.])?;
                if is_all(&k, 0.) || fg == Src::Unset {
                    return Ok(bg);
                }
                if is_all(&k, 1.) || bg == Src::Unset {
                    return Ok(fg);
                }
                let m = self.add("mix_closure", name)?;
                self.put(m, "Fac", &k, 1);
                self.put(m, "Closure1", &bg, 0);
                self.put(m, "Closure2", &fg, 0);
                Ok(link(m, "Closure", 0))
            }
            "mix" => {
                let fg = self.inp(f, el, "fg", &zero)?;
                let bg = self.inp(f, el, "bg", &zero)?;
                let k = self.inp(f, el, "mix", &[0.])?;
                self.mix(name, bg, fg, k, n, false)
            }
            "dotproduct" => {
                let a = self.inp(f, el, "in1", &[0.; 3])?;
                let b = self.inp(f, el, "in2", &[0.; 3])?;
                if let (Some(x), Some(y)) = (konst(&a), konst(&b)) {
                    return Ok(Src::Const(vec![zip(x, y, |p, q| p * q).iter().sum()]));
                }
                let v = self.add("vector_math", name)?;
                self.text(v, "Type", "dot_product");
                self.put(v, "Vector1", &a, 3);
                self.put(v, "Vector2", &b, 3);
                Ok(link(v, "Value", 1))
            }
            cat @ ("crossproduct" | "normalize" | "magnitude") => {
                let a = self.inp(
                    f,
                    el,
                    if cat == "crossproduct" { "in1" } else { "in" },
                    &[0.; 3],
                )?;
                let v = self.add("vector_math", name)?;
                self.put(v, "Vector1", &a, 3);
                match cat {
                    "crossproduct" => {
                        let b = self.inp(f, el, "in2", &[0.; 3])?;
                        self.text(v, "Type", "cross_product");
                        self.put(v, "Vector2", &b, 3);
                        Ok(link(v, "Vector", 3))
                    }
                    "normalize" => {
                        self.text(v, "Type", "normalize");
                        Ok(link(v, "Vector", n))
                    }
                    _ => {
                        self.text(v, "Type", "length");
                        Ok(link(v, "Value", 1))
                    }
                }
            }
            cat @ ("ifgreater" | "ifgreatereq" | "ifequal") => {
                let cmp = match cat {
                    "ifgreater" => Cmp::Greater,
                    "ifgreatereq" => Cmp::GreaterEq,
                    _ => Cmp::Equal,
                };
                let first = if cmp == Cmp::Equal { [0.] } else { [1.] };
                let a = self.inp(f, el, "value1", &first)?;
                let b = self.inp(f, el, "value2", &[0.])?;
                let c = self.cond(name, cmp, a, b)?;
                if el.ty == "boolean" {
                    return Ok(c);
                }
                let yes = self.inp(f, el, "in1", &zero)?;
                let no = self.inp(f, el, "in2", &zero)?;
                self.mix(name, no, yes, c, n, false)
            }
            "remap" | "range" => {
                let a = self.inp(f, el, "in", &zero)?;
                let inlow = self.inp(f, el, "inlow", &zero)?;
                let inhigh = self.inp(f, el, "inhigh", &one)?;
                let outlow = self.inp(f, el, "outlow", &zero)?;
                let outhigh = self.inp(f, el, "outhigh", &one)?;
                let from = self.bin(name, Op::Sub, a, inlow.clone(), n)?;
                let span = self.bin(name, Op::Sub, inhigh, inlow, n)?;
                let mut t = self.bin(name, Op::Div, from, span, n)?;
                if el.category == "range" {
                    let gamma = self.inp(f, el, "gamma", &one)?;
                    let inv = self.bin(name, Op::Div, Src::Const(vec![1.]), gamma, n)?;
                    t = self.bin(name, Op::Pow, t, inv, n)?;
                }
                let width = self.bin(name, Op::Sub, outhigh.clone(), outlow.clone(), n)?;
                let scaled = self.bin(name, Op::Mul, t, width, n)?;
                let out = self.bin(name, Op::Add, scaled, outlow.clone(), n)?;
                let clamp = el.category == "range"
                    && Self::word(el, "doclamp").is_some_and(|w| w == "true");
                if clamp {
                    let up = self.bin(name, Op::Max, out, outlow, n)?;
                    return self.bin(name, Op::Min, up, outhigh, n);
                }
                Ok(out)
            }
            "smoothstep" => {
                let a = self.inp(f, el, "in", &zero)?;
                let low = self.inp(f, el, "low", &zero)?;
                let high = self.inp(f, el, "high", &one)?;
                if let (Some(x), Some(l), Some(h)) = (konst(&a), konst(&low), konst(&high)) {
                    let t = zip(
                        &zip(x, l, |p, q| p - q),
                        &zip(h, l, |p, q| p - q),
                        |p, q| {
                            if q == 0. { 0. } else { (p / q).clamp(0., 1.) }
                        },
                    );
                    return Ok(Src::Const(
                        t.iter().map(|t| t * t * (3. - 2. * t)).collect(),
                    ));
                }
                if n > 1 {
                    return Err(refuse(el, "can't render on colours or vectors yet"));
                }
                let m = self.add("map_range", name)?;
                self.text(m, "Type", "smoothstep");
                self.put(m, "Value", &a, 1);
                self.put(m, "From Min", &low, 1);
                self.put(m, "From Max", &high, 1);
                self.set(m, "To Min", FlatValue::Float(0.));
                self.set(m, "To Max", FlatValue::Float(1.));
                Ok(link(m, "Result", 1))
            }
            "luminance" => {
                let a = self.inp(f, el, "in", &zero)?;
                let w = self.inp(
                    f,
                    el,
                    "lumacoeffs",
                    &[0.272_228_7, 0.674_081_8, 0.053_689_5],
                )?;
                if let (Some(c), Some(w)) = (konst(&a), konst(&w)) {
                    let y: f64 = zip(&fit(c, 3), &fit(w, 3), |p, q| p * q).iter().sum();
                    return Ok(Src::Const(fit(&[y], n)));
                }
                let v = self.add("vector_math", name)?;
                self.text(v, "Type", "dot_product");
                self.put(v, "Vector1", &a, 3);
                self.put(v, "Vector2", &w, 3);
                Ok(link(v, "Value", 1))
            }
            // Cycles' Separate / Combine Color in HSV mode (hue 0..1, as
            // MaterialX's).
            "rgbtohsv" | "hsvtorgb" => {
                let a = self.inp(f, el, "in", &zero)?;
                let to_hsv = el.category == "rgbtohsv";
                if let Some(c) = konst(&a) {
                    let c = fit(c, 3);
                    let out = if to_hsv {
                        crate::process_occlusion::rgb_to_hsv([c[0], c[1], c[2]])
                    } else {
                        crate::process_occlusion::hsv_to_rgb([c[0], c[1], c[2]])
                    };
                    return Ok(Src::Const(fit(&out, n)));
                }
                let s = self.add("separate_color", name)?;
                self.text(s, "Type", if to_hsv { "hsv" } else { "rgb" });
                self.put(s, "Color", &a, 3);
                let c = self.add("combine_color", name)?;
                self.text(c, "Type", if to_hsv { "rgb" } else { "hsv" });
                for k in ["Red", "Green", "Blue"] {
                    self.put(c, k, &link(s, k, 1), 1);
                }
                Ok(link(c, "Color", n))
            }
            "hsvadjust" | "saturate" => {
                let a = self.inp(f, el, "in", &zero)?;
                let (hue, sat, val) = if el.category == "hsvadjust" {
                    let m = fit(&self.fixed(f, el, "amount", &[0., 1., 1.])?, 3);
                    (m[0], m[1], m[2])
                } else {
                    (0., self.fixed(f, el, "amount", &[1.])?[0], 1.)
                };
                if (hue, sat, val) == (0., 1., 1.) {
                    return Ok(a);
                }
                let h = self.add("hsv", name)?;
                self.put(h, "Color", &a, 3);
                self.set(h, "Hue", FlatValue::Float(0.5 + hue));
                self.set(h, "Saturation", FlatValue::Float(sat));
                self.set(h, "Value", FlatValue::Float(val));
                self.set(h, "Fac", FlatValue::Float(1.));
                Ok(link(h, "Color", n))
            }
            "texcoord" => {
                if self.fixed(f, el, "index", &[0.])?[0] != 0. {
                    self.notes
                        .push(format!("{name}: only the first UV map is read"));
                }
                self.uv()
            }
            "geompropvalue" | "UsdPrimvarReader" => {
                let key = if el.category == "geompropvalue" {
                    "geomprop"
                } else {
                    "varname"
                };
                let prop = Self::word(el, key).unwrap_or_default();
                if n == 2 {
                    if !matches!(prop.as_str(), "st" | "uv" | "UV" | "UVMap" | "st0" | "uv0") {
                        self.notes
                            .push(format!("{name}: reads “{prop}” as the first UV map"));
                    }
                    return self.uv();
                }
                Err(refuse(
                    el,
                    format!("reads “{prop}”, which can't render yet"),
                ))
            }
            "position" | "normal" => {
                let space = Self::word(el, "space").unwrap_or_else(|| "object".into());
                match (el.category.as_str(), space.as_str()) {
                    ("position", "world") => self.world("Position"),
                    ("position", _) => self.object_coords("Object"),
                    ("normal", "object") => self.object_coords("Normal"),
                    _ => self.world("Normal"),
                }
            }
            "place2d" => {
                let uv = match self.opt(f, el, "texcoord")? {
                    Src::Unset => self.uv()?,
                    s => s,
                };
                let pivot = fit(&self.fixed(f, el, "pivot", &[0., 0.])?, 2);
                let scale = fit(&self.fixed(f, el, "scale", &[1., 1.])?, 2);
                let rotate = self.fixed(f, el, "rotate", &[0.])?[0];
                let offset = fit(&self.fixed(f, el, "offset", &[0., 0.])?, 2);
                if self.fixed(f, el, "operationorder", &[0.])?[0] != 0. {
                    self.notes
                        .push(format!("{name}: transforms applied in the default order"));
                }
                // ((uv - pivot) / scale, turned by rotate degrees clockwise,
                // which is MaterialX's rotate2d) - offset + pivot.
                let moved = self.bin(name, Op::Sub, uv, Src::Const(pivot.clone()), 2)?;
                let inv: Vec<f64> = scale
                    .iter()
                    .map(|s| if *s == 0. { 1. } else { 1. / s })
                    .collect();
                let location = vec![pivot[0] - offset[0], pivot[1] - offset[1], 0.];
                self.mapping(name, moved, Src::Const(inv), -rotate, Src::Const(location))
            }
            "UsdTransform2d" => {
                let uv = match self.opt(f, el, "in")? {
                    Src::Unset => self.uv()?,
                    s => s,
                };
                let scale = fit(&self.fixed(f, el, "scale", &[1., 1.])?, 2);
                let rotation = self.fixed(f, el, "rotation", &[0.])?[0];
                let t = fit(&self.fixed(f, el, "translation", &[0., 0.])?, 2);
                self.mapping(
                    name,
                    uv,
                    Src::Const(scale),
                    rotation,
                    Src::Const(vec![t[0], t[1], 0.]),
                )
            }
            "image" | "tiledimage" => self.image(f, el),
            "UsdUVTexture" => self.usd_texture(f, el, output),
            "normalmap" => {
                let a = self.inp(f, el, "in", &[0.5, 0.5, 1.])?;
                let scale = self.inp(f, el, "scale", &[1.])?;
                let m = self.add("normal_map", name)?;
                let space = Self::word(el, "space").unwrap_or_else(|| "tangent".into());
                self.text(
                    m,
                    "Space",
                    if space == "object" {
                        "object"
                    } else {
                        "tangent"
                    },
                );
                self.put(m, "Color", &a, 3);
                let strength = match scale {
                    Src::Const(c) => Src::Const(vec![c.first().copied().unwrap_or(1.)]),
                    s => s,
                };
                self.put(m, "Strength", &strength, 1);
                Ok(link(m, "Normal", 3))
            }
            "heighttonormal" => {
                let a = self.inp(f, el, "in", &[0.])?;
                let scale = self.inp(f, el, "scale", &[1.])?;
                let b = self.add("bump", name)?;
                self.put(b, "Height", &a, 1);
                self.put(b, "Distance", &scale, 1);
                self.set(b, "Strength", FlatValue::Float(1.));
                self.notes.push(format!(
                    "{name}: heighttonormal approximated by a Bump node"
                ));
                Ok(link(b, "Normal", 3))
            }
            cat @ ("noise2d" | "noise3d" | "fractal2d" | "fractal3d") => {
                let two = cat.ends_with("2d");
                let at = match self.opt(f, el, if two { "texcoord" } else { "position" })? {
                    Src::Unset if two => self.uv()?,
                    Src::Unset => self.object_coords("Object")?,
                    s => s,
                };
                let amplitude = self.inp(f, el, "amplitude", &one)?;
                let noise = self.add("noise_texture", name)?;
                self.set(noise, "Dimensions", FlatValue::Int(if two { 2 } else { 3 }));
                self.put(noise, "Vector", &at, 3);
                self.set(noise, "Scale", FlatValue::Float(1.));
                if cat.starts_with("fractal") {
                    let octaves = self.fixed(f, el, "octaves", &[3.])?[0];
                    let lac = self.inp(f, el, "lacunarity", &[2.])?;
                    let dim = self.inp(f, el, "diminish", &[0.5])?;
                    self.set(noise, "Detail", FlatValue::Float((octaves - 1.).max(0.)));
                    self.put(noise, "Lacunarity", &lac, 1);
                    self.put(noise, "Roughness", &dim, 1);
                    self.set(noise, "Normalize", FlatValue::Bool(false));
                } else {
                    self.set(noise, "Detail", FlatValue::Float(0.));
                }
                self.notes.push(format!(
                    "{name}: MaterialX {cat} rendered as Cycles noise (a different pattern)"
                ));
                // Cycles' noise runs 0..1 around 0.5; MaterialX's -1..1
                // around 0, times amplitude, plus pivot.
                let raw = link(noise, if n > 1 { "Color" } else { "Fac" }, n);
                let centred = self.bin(name, Op::Sub, raw, Src::Const(vec![0.5]), n)?;
                let wide = self.bin(name, Op::Mul, centred, Src::Const(vec![2.]), n)?;
                let scaled = self.bin(name, Op::Mul, wide, amplitude, n)?;
                let pivot = self.inp(f, el, "pivot", &zero)?;
                self.bin(name, Op::Add, scaled, pivot, n)
            }
            cat @ ("cellnoise2d" | "cellnoise3d") => {
                let two = cat == "cellnoise2d";
                let at = match self.opt(f, el, if two { "texcoord" } else { "position" })? {
                    Src::Unset if two => self.uv()?,
                    Src::Unset => self.object_coords("Object")?,
                    s => s,
                };
                let cell = self.un(el, Un::Floor, Self::resize(at, 3), 3)?;
                let w = self.add("white_noise_texture", name)?;
                self.set(w, "Dimensions", FlatValue::Int(if two { 2 } else { 3 }));
                self.put(w, "Vector", &cell, 3);
                Ok(link(w, if n > 1 { "Color" } else { "Value" }, n))
            }
            cat @ ("ramplr" | "ramptb") => {
                let (a, b, axis) = if cat == "ramplr" {
                    ("valuel", "valuer", 0)
                } else {
                    ("valueb", "valuet", 1)
                };
                let start = self.inp(f, el, a, &zero)?;
                let end = self.inp(f, el, b, &zero)?;
                let uv = match self.opt(f, el, "texcoord")? {
                    Src::Unset => self.uv()?,
                    s => s,
                };
                let t = self.channel(name, uv, axis)?;
                self.mix(name, start, end, t, n, true)
            }
            "checkerboard" => {
                let c1 = self.inp(f, el, "color1", &[1.; 3])?;
                let c2 = self.inp(f, el, "color2", &[0.; 3])?;
                let tiling = self.fixed(f, el, "uvtiling", &[8., 8.])?;
                let offset = fit(&self.fixed(f, el, "uvoffset", &[0., 0.])?, 2);
                let uv = match self.opt(f, el, "texcoord")? {
                    Src::Unset => self.uv()?,
                    s => s,
                };
                let at = self.mapping(
                    name,
                    uv,
                    Src::Const(tiling),
                    0.,
                    Src::Const(vec![-offset[0], -offset[1], 0.]),
                )?;
                let c = self.add("checker_texture", name)?;
                self.put(c, "Vector", &at, 3);
                self.put(c, "Color1", &c1, 3);
                self.put(c, "Color2", &c2, 3);
                self.set(c, "Scale", FlatValue::Float(1.));
                Ok(link(c, "Color", 3))
            }
            "displacement" => {
                let scale = self.inp(f, el, "scale", &[1.])?;
                // MaterialX leaves the unit to the scene; OpenPBR's default
                // length is the centimetre, as the importer reads it
                // (provisional, OPM-Q13).
                let scale = self.bin(
                    name,
                    Op::Mul,
                    scale,
                    Src::Const(vec![openpbr::DEFAULT_LENGTH_IN_METRES]),
                    1,
                )?;
                let vector = el.input("displacement").is_some_and(|p| p.ty == "vector3");
                let d = self.add(
                    if vector {
                        "vector_displacement"
                    } else {
                        "displacement"
                    },
                    name,
                )?;
                let h = self.inp(f, el, "displacement", &[0.])?;
                self.put(
                    d,
                    if vector { "Vector" } else { "Height" },
                    &h,
                    if vector { 3 } else { 1 },
                );
                self.set(d, "Midlevel", FlatValue::Float(0.));
                self.put(d, "Scale", &scale, 1);
                if vector {
                    self.text(d, "Space", "tangent");
                }
                Ok(link(d, "Displacement", 3))
            }
            "open_pbr_surface" => {
                let mut map = BTreeMap::new();
                for p in Param::ALL {
                    let s = match el.input(p.name()) {
                        Some(port) => {
                            let s = self.port(f, port)?;
                            if p.is_length() {
                                self.length(name, s, port.unit.as_deref())?
                            } else {
                                s
                            }
                        }
                        None => Src::Unset,
                    };
                    map.insert(*p, s);
                }
                self.openpbr_bsdf(name, map)
            }
            "standard_surface" => self.standard_surface(f, el),
            "UsdPreviewSurface" => self.usd_preview(f, el),
            "gltf_pbr" => self.gltf(f, el),
            _ => self.implemented(f, el, output),
        }
    }

    /// A length input as metres: its own unit, else OpenPBR's centimetre.
    fn length(&mut self, name: &str, s: Src, unit: Option<&str>) -> R<Src> {
        let factor = unit
            .and_then(super::import::metres_per)
            .unwrap_or(openpbr::DEFAULT_LENGTH_IN_METRES);
        match s {
            Src::Unset => Ok(Src::Unset),
            s => self.bin(name, Op::Mul, s, Src::Const(vec![factor]), 1),
        }
    }

    /// A node whose definition the document implements as a nodegraph.
    fn implemented(&mut self, f: usize, el: &'a Element, output: &str) -> R<Src> {
        let doc = self.doc;
        let def = doc.all("nodedef").find(|d| {
            d.attrs.get("node").map(String::as_str) == Some(el.category.as_str())
                && (el.ty.is_empty()
                    || d.outputs.iter().any(|o| o.ty == el.ty)
                    || d.attrs.get("type") == Some(&el.ty))
        });
        let graph = def.and_then(|d| {
            doc.all("nodegraph")
                .find(|g| g.attrs.get("nodedef").map(String::as_str) == Some(d.name.as_str()))
        });
        // Else MaterialX's own implementation, from the bundled libraries.
        let (def, graph) = match (def, graph) {
            (Some(d), Some(g)) => (d, g),
            _ => super::stdlib::implementation(el)
                .ok_or_else(|| refuse(el, "this MaterialX node can't render yet"))?,
        };
        let key = (f, el.name.clone());
        let frame = match self.instance_frames.get(&key) {
            Some(&k) => k,
            None => {
                self.frames.push(Frame {
                    elements: &graph.children,
                    iface: Iface::Instance {
                        node: el,
                        at: f,
                        def,
                    },
                });
                let k = self.frames.len() - 1;
                self.instance_frames.insert(key, k);
                k
            }
        };
        let out = graph
            .outputs
            .iter()
            .find(|o| o.name == output)
            .or(graph.outputs.first())
            .ok_or_else(|| refuse(el, "its nodegraph has no output"))?;
        self.port(frame, out)
    }

    fn image(&mut self, f: usize, el: &'a Element) -> R<Src> {
        let name = el.name.as_str();
        let n = size_of(&el.ty);
        let file = el
            .input("file")
            .ok_or_else(|| refuse(el, "the image has no file"))?;
        let value = file.value.clone().unwrap_or_default();
        if value.is_empty() {
            return self.inp(f, el, "default", &vec![0.; n.max(1)]);
        }
        if value.contains("<UDIM>") || value.contains("<UVTILE>") {
            return Err(refuse(el, "UDIM tiles can't render yet"));
        }
        let path = self.folder.join(format!("{}{value}", file.fileprefix));
        let mut uv = self.opt(f, el, "texcoord")?;
        if el.category == "tiledimage" {
            if uv == Src::Unset {
                uv = self.uv()?;
            }
            let tiling = self.fixed(f, el, "uvtiling", &[1., 1.])?;
            let offset = fit(&self.fixed(f, el, "uvoffset", &[0., 0.])?, 2);
            uv = self.mapping(
                name,
                uv,
                Src::Const(tiling),
                0.,
                Src::Const(vec![-offset[0], -offset[1], 0.]),
            )?;
        }
        let space = file.colorspace.clone().or_else(|| el.colorspace.clone());
        let colorspace = match space.as_deref() {
            Some(s) if colour::is_srgb_encoded(s) => "__builtin_srgb",
            Some("raw" | "none") => "data",
            Some(s) => {
                if !matches!(s, "lin_rec709" | "lin_srgb") {
                    self.notes
                        .push(format!("{name}: pictures in {s} read as linear Rec.709"));
                }
                "scene_linear"
            }
            // No colour space: the values are data unless they are colours.
            None if el.ty.starts_with("color") => "scene_linear",
            None => "data",
        };
        let address = |w: Option<String>| match w.as_deref() {
            Some("clamp") => "clamp",
            Some("constant") => "black",
            Some("mirror") => "mirror",
            _ => "periodic",
        };
        let img = self.add("image_texture", name)?;
        self.text(img, "Filename", &path.to_string_lossy());
        self.text(img, "Colorspace", colorspace);
        let u = address(Self::word(el, "uaddressmode"));
        let v = address(Self::word(el, "vaddressmode"));
        if u != v {
            self.notes
                .push(format!("{name}: one wrap mode for both directions"));
        }
        self.text(img, "Extension", u);
        let filter = match Self::word(el, "filtertype").as_deref() {
            Some("closest") => "closest",
            Some("cubic") => "cubic",
            _ => "linear",
        };
        self.text(img, "Interpolation", filter);
        self.put(img, "Vector", &uv, 3);
        Ok(match n {
            1 => {
                let s = self.add("separate_color", name)?;
                self.text(s, "Type", "rgb");
                self.put(s, "Color", &link(img, "Color", 3), 3);
                link(s, "Red", 1)
            }
            4 => Src::Link(Out {
                node: img,
                socket: "Color",
                size: 4,
                alpha: Some((img, "Alpha")),
            }),
            n => link(img, "Color", n),
        })
    }

    fn usd_texture(&mut self, f: usize, el: &'a Element, output: &str) -> R<Src> {
        let name = el.name.as_str();
        let file = el
            .input("file")
            .ok_or_else(|| refuse(el, "the texture has no file"))?;
        let value = file.value.clone().unwrap_or_default();
        if value.is_empty() {
            return Ok(Src::Const(vec![0.; if output == "rgb" { 3 } else { 1 }]));
        }
        let path = self.folder.join(format!("{}{value}", file.fileprefix));
        let uv = self.opt(f, el, "st")?;
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        // USD's "auto": 8-bit colour pictures are sRGB (provisional: the
        // channel count isn't known here, so only the rgb output counts).
        let colorspace = match Self::word(el, "sourceColorSpace").as_deref() {
            Some("raw") => "data",
            Some("sRGB") => "__builtin_srgb",
            _ if matches!(ext.as_str(), "exr" | "hdr") => "scene_linear",
            _ if output == "rgb" => "__builtin_srgb",
            _ => "data",
        };
        let wrap = |w: Option<String>| match w.as_deref() {
            Some("clamp") => "clamp",
            Some("black") => "black",
            Some("mirror") => "mirror",
            _ => "periodic",
        };
        let img = self.add("image_texture", name)?;
        self.text(img, "Filename", &path.to_string_lossy());
        self.text(img, "Colorspace", colorspace);
        self.text(img, "Extension", wrap(Self::word(el, "wrapS")));
        self.put(img, "Vector", &uv, 3);
        let scale = fit(&self.fixed(f, el, "scale", &[1.; 4])?, 4);
        let bias = fit(&self.fixed(f, el, "bias", &[0.; 4])?, 4);
        let (raw, k) = match output {
            "a" => (link(img, "Alpha", 1), 3),
            "r" | "g" | "b" => {
                let k = match output {
                    "r" => 0,
                    "g" => 1,
                    _ => 2,
                };
                let c = self.channel(name, link(img, "Color", 3), k)?;
                (c, k)
            }
            _ => {
                let scaled = self.bin(
                    name,
                    Op::Mul,
                    link(img, "Color", 3),
                    Src::Const(scale[..3].to_vec()),
                    3,
                )?;
                return self.bin(name, Op::Add, scaled, Src::Const(bias[..3].to_vec()), 3);
            }
        };
        let scaled = self.bin(name, Op::Mul, raw, Src::Const(vec![scale[k]]), 1)?;
        self.bin(name, Op::Add, scaled, Src::Const(vec![bias[k]]), 1)
    }

    /// Every input a surface declares (or the document sets), resolved;
    /// declared defaults where nothing is set.
    fn surface_inputs(&mut self, f: usize, el: &'a Element) -> R<BTreeMap<String, Src>> {
        let mut out = BTreeMap::new();
        for (name, port) in stdlib::defaults(&el.category) {
            let s = match &port.value {
                Some(v) if size_of(&port.ty) > 0 => numbers(v).map_or(Src::Unset, Src::Const),
                _ => Src::Unset,
            };
            out.insert(name, s);
        }
        for p in &el.inputs {
            let s = self.port(f, p)?;
            if s != Src::Unset {
                out.insert(p.name.clone(), s);
            }
        }
        Ok(out)
    }

    /// OpenPBR inputs on Cycles' Principled BSDF (`openpbr::cycles_target`).
    fn openpbr_bsdf(&mut self, name: &str, mut map: BTreeMap<Param, Src>) -> R<Src> {
        use Param::*;
        for p in Param::ALL {
            let s = map.entry(*p).or_insert(Src::Unset);
            if *s == Src::Unset {
                *s = match p.default() {
                    openpbr::Value::Float(v) => Src::Const(vec![v]),
                    openpbr::Value::Color(c) => Src::Const(c.to_vec()),
                    openpbr::Value::Bool(b) => Src::Const(vec![f64::from(u8::from(b))]),
                    openpbr::Value::Geometry => Src::Unset,
                };
            }
        }
        let g = |p: Param| map[&p].clone();
        let b = self.add("principled_bsdf", name)?;
        let mut base = self.bin(name, Op::Mul, g(BaseColor), g(BaseWeight), 3)?;
        if !is_all(&g(SubsurfaceWeight), 0.) {
            base = self.mix(
                name,
                base,
                g(SubsurfaceColor),
                g(SubsurfaceWeight),
                3,
                false,
            )?;
        }
        if !is_all(&g(TransmissionWeight), 0.) {
            // Cycles' transmission takes its colour from the base colour.
            base = self.mix(
                name,
                base,
                g(TransmissionColor),
                g(TransmissionWeight),
                3,
                false,
            )?;
        }
        self.put(b, "Base Color", &base, 3);
        self.put(b, "Metallic", &g(BaseMetalness), 1);
        self.put(b, "Diffuse Roughness", &g(BaseDiffuseRoughness), 1);
        let level = self.bin(name, Op::Mul, g(SpecularWeight), Src::Const(vec![0.5]), 1)?;
        self.put(b, "Specular IOR Level", &level, 1);
        self.put(b, "Specular Tint", &g(SpecularColor), 3);
        self.put(b, "Roughness", &g(SpecularRoughness), 1);
        self.put(b, "IOR", &g(SpecularIor), 1);
        self.put(b, "Anisotropic", &g(SpecularRoughnessAnisotropy), 1);
        self.put(b, "Transmission Weight", &g(TransmissionWeight), 1);
        self.put(
            b,
            "Transmission Dispersion Scale",
            &g(TransmissionDispersionScale),
            1,
        );
        self.put(
            b,
            "Transmission Dispersion Abbe Number",
            &g(TransmissionDispersionAbbeNumber),
            1,
        );
        self.put(b, "Subsurface Weight", &g(SubsurfaceWeight), 1);
        self.put(b, "Subsurface Scale", &g(SubsurfaceRadius), 1);
        self.put(b, "Subsurface Radius", &g(SubsurfaceRadiusScale), 3);
        self.put(
            b,
            "Subsurface Anisotropy",
            &g(SubsurfaceScatterAnisotropy),
            1,
        );
        self.put(b, "Sheen Weight", &g(FuzzWeight), 1);
        self.put(b, "Sheen Tint", &g(FuzzColor), 3);
        self.put(b, "Sheen Roughness", &g(FuzzRoughness), 1);
        self.put(b, "Coat Weight", &g(CoatWeight), 1);
        self.put(b, "Coat Tint", &g(CoatColor), 3);
        self.put(b, "Coat Roughness", &g(CoatRoughness), 1);
        self.put(b, "Coat IOR", &g(CoatIor), 1);
        let film = self.bin(
            name,
            Op::Mul,
            g(ThinFilmThickness),
            Src::Const(vec![openpbr::NANOMETRES_PER_MICROMETRE]),
            1,
        )?;
        let film = self.bin(name, Op::Mul, film, g(ThinFilmWeight), 1)?;
        self.put(b, "Thin Film Thickness", &film, 1);
        self.put(b, "Thin Film IOR", &g(ThinFilmIor), 1);
        self.put(b, "Emission Color", &g(EmissionColor), 3);
        let strength = self.bin(
            name,
            Op::Mul,
            g(EmissionLuminance),
            Src::Const(vec![openpbr::STRENGTH_PER_NIT]),
            1,
        )?;
        self.put(b, "Emission Strength", &strength, 1);
        self.put(b, "Alpha", &g(GeometryOpacity), 1);
        self.put(b, "Thin Wall", &g(GeometryThinWalled), 1);
        for (p, socket) in [
            (GeometryNormal, "Normal"),
            (GeometryCoatNormal, "Coat Normal"),
            (GeometryTangent, "Tangent"),
        ] {
            if matches!(g(p), Src::Link(_)) {
                self.put(b, socket, &g(p), 3);
            }
        }
        for (p, default) in [
            (CoatDarkening, 1.),
            (CoatRoughnessAnisotropy, 0.),
            (TransmissionDepth, 0.),
            (TransmissionScatterAnisotropy, 0.),
        ] {
            if !is_all(&g(p), default) {
                self.notes
                    .push(format!("{name}: {} has no Cycles slot; left out", p.name()));
            }
        }
        if !is_all(&g(TransmissionScatter), 0.) {
            self.notes.push(format!(
                "{name}: transmission_scatter has no Cycles slot; left out"
            ));
        }
        Ok(link(b, "BSDF", 0))
    }

    /// standard_surface through MaterialX's published translation graph
    /// to OpenPBR (as the importer reads it).
    fn standard_surface(&mut self, f: usize, el: &'a Element) -> R<Src> {
        let graph = stdlib::standard_to_openpbr();
        let def = graph
            .attrs
            .get("nodedef")
            .and_then(|d| stdlib::nodedef_named(d))
            .ok_or_else(|| problem("The bundled standard_surface translation is missing"))?;
        self.frames.push(Frame {
            elements: &graph.children,
            iface: Iface::Instance {
                node: el,
                at: f,
                def,
            },
        });
        let frame = self.frames.len() - 1;
        let mut map = BTreeMap::new();
        for o in &graph.outputs {
            let Some(p) = o.name.strip_suffix("_out").and_then(Param::from_name) else {
                continue;
            };
            let s = self.port(frame, o)?;
            // standard_surface lengths have no unit: scene units, which
            // Operandi reads as centimetres like OpenPBR (OPM-Q13).
            let s = if p.is_length() {
                self.length(&el.name, s, None)?
            } else {
                s
            };
            map.insert(p, s);
        }
        for (input, p) in [
            ("normal", Param::GeometryNormal),
            ("coat_normal", Param::GeometryCoatNormal),
            ("tangent", Param::GeometryTangent),
        ] {
            let s = self.opt(f, el, input)?;
            map.insert(p, s);
        }
        self.openpbr_bsdf(&el.name, map)
    }

    fn usd_preview(&mut self, f: usize, el: &'a Element) -> R<Src> {
        let name = el.name.as_str();
        let i = self.surface_inputs(f, el)?;
        let g = |k: &str| i.get(k).cloned().unwrap_or(Src::Unset);
        let b = self.add("principled_bsdf", name)?;
        self.put(b, "Base Color", &g("diffuseColor"), 3);
        self.put(b, "Metallic", &g("metallic"), 1);
        self.put(b, "Roughness", &g("roughness"), 1);
        self.put(b, "IOR", &g("ior"), 1);
        self.put(b, "Coat Weight", &g("clearcoat"), 1);
        self.put(b, "Coat Roughness", &g("clearcoatRoughness"), 1);
        self.put(b, "Alpha", &g("opacity"), 1);
        self.put(b, "Emission Color", &g("emissiveColor"), 3);
        self.set(b, "Emission Strength", FlatValue::Float(1.));
        if is_all(&g("useSpecularWorkflow"), 1.) {
            self.notes
                .push(format!("{name}: the specular workflow read as metalness"));
        }
        if let n @ Src::Link(_) = g("normal") {
            // USD normals are -1..1; a normal map reads 0..1.
            let half = self.bin(name, Op::Mul, n, Src::Const(vec![0.5]), 3)?;
            let colour = self.bin(name, Op::Add, half, Src::Const(vec![0.5]), 3)?;
            let m = self.add("normal_map", name)?;
            self.text(m, "Space", "tangent");
            self.put(m, "Color", &colour, 3);
            self.put(b, "Normal", &link(m, "Normal", 3), 3);
        }
        if matches!(g("displacement"), Src::Link(_)) {
            self.notes
                .push(format!("{name}: displacement on the surface left out"));
        }
        Ok(link(b, "BSDF", 0))
    }

    fn gltf(&mut self, f: usize, el: &'a Element) -> R<Src> {
        let name = el.name.as_str();
        let i = self.surface_inputs(f, el)?;
        let g = |k: &str| i.get(k).cloned().unwrap_or(Src::Unset);
        let b = self.add("principled_bsdf", name)?;
        self.put(b, "Base Color", &g("base_color"), 3);
        self.put(b, "Alpha", &g("alpha"), 1);
        self.put(b, "Metallic", &g("metallic"), 1);
        self.put(b, "Roughness", &g("roughness"), 1);
        self.put(b, "Transmission Weight", &g("transmission"), 1);
        let level = self.bin(name, Op::Mul, g("specular"), Src::Const(vec![0.5]), 1)?;
        self.put(b, "Specular IOR Level", &level, 1);
        self.put(b, "Specular Tint", &g("specular_color"), 3);
        self.put(b, "IOR", &g("ior"), 1);
        let sheen = g("sheen_color");
        if !is_all(&sheen, 0.) {
            self.set(b, "Sheen Weight", FlatValue::Float(1.));
            self.put(b, "Sheen Tint", &sheen, 3);
            self.put(b, "Sheen Roughness", &g("sheen_roughness"), 1);
        }
        self.put(b, "Coat Weight", &g("clearcoat"), 1);
        self.put(b, "Coat Roughness", &g("clearcoat_roughness"), 1);
        self.put(b, "Emission Color", &g("emissive"), 3);
        self.put(b, "Emission Strength", &g("emissive_strength"), 1);
        for (input, socket) in [("normal", "Normal"), ("clearcoat_normal", "Coat Normal")] {
            if let n @ Src::Link(_) = g(input) {
                self.put(b, socket, &n, 3);
            }
        }
        Ok(link(b, "BSDF", 0))
    }
}

#[cfg(test)]
#[path = "cycles_tests.rs"]
mod tests;
