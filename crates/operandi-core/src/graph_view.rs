//! A node graph's surface as the viewport's channels (MaterialX stage 5,
//! docs/MATERIALX_GRAPHS.md): the Advanced style, and the others' colour,
//! draw a graph layer from its nodes instead of one flat colour (NM-40).
//!
//! The graph is read in its flattened, Cycles form (`FlatGraph`), the same
//! one the renderer gets, so MaterialX graphs, Blender Node Materials and
//! anything else that flattens draw alike. Each node's output becomes a
//! [`shader_eval::Expr`](crate::shader_eval::Expr): tree expressions where
//! they fit (images, mixes, ramps, UVs), `Expr::Func` for the rest
//! (`shader_func`), and the closure feeding the Material Output becomes
//! values for the shader tree's channels (Diffuse Color, Roughness...), so
//! the existing per-pixel programs draw it.
//!
//! What can't be drawn falls back input by input to the socket's own
//! value (provisional, MXG-Q07): a Magic texture, a Camera coordinate.
//! The whole graph falls back to NM-40's single colour only when its
//! surface closure itself isn't one of the five handled here.
use crate::cycles_noise::{GradientKind, NoiseKind, WaveKind};
use crate::node_graph::{FlatGraph, FlatValue};
use crate::shader_eval::{Ch, Expr, GeometryInput, ImageLookup, Mix};
use crate::shader_func::{Func, MappingKind, MathOp, MixMore, RangeKind, VectorOp};
use crate::shader_tree::Filtering;
use crate::texture_locator::Tiling;
use std::collections::HashMap;

/// The exposed values a Node Material layer flattens its graph with, and
/// the expressions of the inputs layers above drive (indexed by
/// `FlatValue::Driven`).
pub fn exposed(n: &crate::shader_eval::NodeUse) -> (crate::node_graph::ExposedValues, Vec<Expr>) {
    let mut values = crate::node_graph::ExposedValues {
        values: n.values.clone(),
        ..Default::default()
    };
    let mut drivers = Vec::new();
    for (i, (name, e)) in n.drivers.iter().enumerate() {
        values.drivers.insert(name.clone(), i);
        drivers.push(e.clone());
    }
    (values, drivers)
}

/// Every picture the tree's graphs read (their Image Texture files), for
/// the viewport to load.
pub fn image_paths(tree: &crate::shader_tree::ShaderTree) -> Vec<String> {
    let mut out = Vec::new();
    for g in &tree.node_graphs.graphs {
        let Ok(flat) = g.flatten(&Default::default()) else {
            continue;
        };
        for node in flat.nodes.iter().filter(|n| n.cycles == "image_texture") {
            if let Some((_, FlatValue::Text(p))) = node.values.iter().find(|(s, _)| s == "Filename")
                && !p.is_empty()
                && !out.contains(p)
            {
                out.push(p.clone());
            }
        }
    }
    out
}

/// The viewport's image id for a graph picture: high ids scene images
/// never take (they count up from 1), one per path.
pub fn image_id(path: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    (1 << 63) | (h.finish() >> 1)
}

/// `surface` with a Node Material's channels laid over the layers by its
/// coverage (NM-12).
pub fn apply(
    mut surface: crate::shader_eval::Surface,
    coverage: &Expr,
    view: &ViewGraph,
) -> crate::shader_eval::Surface {
    for (ch, value) in &view.channels {
        let i = *ch as usize;
        let below = std::mem::replace(&mut surface.channels[i], Expr::scalar(0.));
        surface.channels[i] = Expr::mix(Mix::Blend, coverage.clone(), below, value.clone());
    }
    surface
}

/// Where a mesh's coordinates come from, for Texture Coordinate and
/// Geometry outputs. The viewport shades in the item's own space.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphSpace {
    /// Own space to Generated coordinates (row-major 3×4): 0..1 across the
    /// mesh's bounding box, as the renderer computes them (NM-Q08).
    pub generated: [f64; 12],
    /// Own space to world (3×4), for Geometry ▸ Position.
    pub to_world: [f64; 12],
    /// Own-space normals to world (3×3), for Geometry ▸ Normal.
    pub normal_to_world: [f64; 9],
}
impl Default for GraphSpace {
    fn default() -> Self {
        const I12: [f64; 12] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.];
        Self {
            generated: I12,
            to_world: I12,
            normal_to_world: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
        }
    }
}
impl GraphSpace {
    /// Generated coordinates for a mesh whose points span `lo`..`hi`: the
    /// renderer's `add_generated` (a flat axis spans 2 m round its centre;
    /// very thin ones 20 µm).
    pub fn generated_for(lo: [f64; 3], hi: [f64; 3]) -> [f64; 12] {
        let mut m = [0.; 12];
        for k in 0..3 {
            let centre = (lo[k] + hi[k]) * 0.5;
            let mut half = (hi[k] - lo[k]) * 0.5;
            if half == 0. || !half.is_finite() {
                half = 1.;
            } else if half < 1e-5 {
                half = 1e-5;
            }
            // (p − centre) / (2·half) + 0.5
            m[k * 4 + k] = 1. / (2. * half);
            m[k * 4 + 3] = 0.5 - centre / (2. * half);
        }
        m
    }
}

/// The graph's surface as channel values, with the parts left out.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewGraph {
    pub channels: Vec<(Ch, Expr)>,
    /// "Node: what" for each input drawn from its own value instead.
    pub missing: Vec<String>,
}

/// The channels a graph surface sets, each replacing the layers' value by
/// the Node Material's coverage. Values are Cycles' Principled BSDF
/// defaults.
const CHANNELS: [(Ch, [f64; 3]); 27] = [
    (Ch::DiffuseColor, [0.8; 3]),
    (Ch::DiffuseAmount, [1.; 3]),
    (Ch::DiffuseRoughness, [0.; 3]),
    (Ch::Metallic, [0.; 3]),
    (Ch::Roughness, [0.5; 3]),
    (Ch::SpecularAmount, [0.5; 3]),
    (Ch::SpecularColor, [1.; 3]),
    (Ch::Anisotropy, [0.; 3]),
    (Ch::Sheen, [0.; 3]),
    (Ch::ClearcoatAmount, [0.; 3]),
    (Ch::ClearcoatRoughness, [0.03; 3]),
    (Ch::CoatColor, [1.; 3]),
    (Ch::CoatIor, [1.5; 3]),
    (Ch::ReflectionAmount, [0.; 3]),
    (Ch::LuminousColor, [1.; 3]),
    (Ch::LuminousIntensity, [0.; 3]),
    (Ch::TransparentAmount, [0.; 3]),
    (Ch::TransparentColor, [0.8; 3]),
    (Ch::RefractiveIndex, [1.5; 3]),
    (Ch::RefractionRoughness, [0.5; 3]),
    (Ch::Dissolve, [0.; 3]),
    (Ch::SubsurfaceAmount, [0.; 3]),
    (Ch::SubsurfaceColor, [1., 0.2, 0.1]),
    (Ch::SubsurfaceDistance, [0.005; 3]),
    (Ch::SubsurfaceAlbedo, [0.8; 3]),
    (Ch::Bump, [0.; 3]),
    (Ch::BumpAmplitude, [0.; 3]),
];
const FLAT_NORMAL: [f64; 3] = [0.5, 0.5, 1.];
/// Past this many expression nodes the graph is drawn as its stand-in (a
/// graph reusing results many times over would grow without end).
const MAX_SIZE: usize = 20_000;

/// One closure's channel values (in [`CHANNELS`] order, then the normal
/// map colour).
type Closure = Vec<Expr>;

/// The graph's surface, or `None` when what feeds the Material Output's
/// Surface isn't a closure this can draw. `drivers` are the exposed inputs
/// texture layers drive (`FlatValue::Driven` indexes them); `image` gives
/// the scene image id the viewport loaded for a picture's path.
pub fn surface(
    flat: &FlatGraph,
    drivers: &[Expr],
    space: &GraphSpace,
    image: &mut dyn FnMut(&str) -> Option<u64>,
) -> Option<ViewGraph> {
    let (node, socket) = flat.feeding("Surface")?;
    let mut tx = Tx {
        g: flat,
        drivers,
        space,
        image,
        memo: HashMap::new(),
        sizes: vec![0],
        missing: Vec::new(),
        depth: 0,
    };
    let socket = socket.to_string();
    let closure = tx.closure(node, &socket, 0)?;
    let mut channels: Vec<(Ch, Expr)> = CHANNELS
        .iter()
        .map(|(ch, _)| *ch)
        .chain([Ch::NormalMap])
        .zip(closure)
        .collect();
    if tx.sizes[0] > MAX_SIZE {
        return None;
    }
    // Unchanged normal map colours leave the channel as the layers have it.
    channels.retain(|(ch, e)| *ch != Ch::NormalMap || *e != Expr::Const(FLAT_NORMAL));
    Some(ViewGraph {
        channels,
        missing: tx.missing,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Float,
    Color,
    Vector,
}

#[derive(Clone, Debug)]
struct Val {
    e: Expr,
    kind: Kind,
}

struct Tx<'a> {
    g: &'a FlatGraph,
    drivers: &'a [Expr],
    space: &'a GraphSpace,
    image: &'a mut dyn FnMut(&str) -> Option<u64>,
    memo: HashMap<(usize, String), (Option<Val>, usize)>,
    /// Expression sizes being added up, one per output under way.
    sizes: Vec<usize>,
    missing: Vec<String>,
    depth: usize,
}

fn c3(v: [f64; 3]) -> Expr {
    Expr::Const(v)
}
fn s(v: f64) -> Expr {
    Expr::scalar(v)
}
fn func(f: Func, args: Vec<Expr>) -> Expr {
    Expr::func(f, args)
}

impl Tx<'_> {
    fn cycles(&self, node: usize) -> &str {
        self.g.nodes[node].cycles
    }
    fn name(&self, node: usize) -> &str {
        &self.g.nodes[node].name
    }
    fn link(&self, node: usize, socket: &str) -> Option<(usize, String)> {
        self.g
            .links
            .iter()
            .find(|l| l.to == node && l.to_socket == socket)
            .map(|l| (l.from, l.from_socket.clone()))
    }
    fn text(&self, node: usize, socket: &str, default: &str) -> String {
        match self.g.value(node, socket) {
            Some(FlatValue::Text(t)) => t.clone(),
            _ => default.to_string(),
        }
    }
    fn flag(&self, node: usize, socket: &str, default: bool) -> bool {
        match self.g.value(node, socket) {
            Some(FlatValue::Bool(b)) => *b,
            Some(FlatValue::Int(i)) => *i != 0,
            Some(FlatValue::Float(f)) => *f != 0.,
            _ => default,
        }
    }
    fn int(&self, node: usize, socket: &str, default: i64) -> i64 {
        match self.g.value(node, socket) {
            Some(FlatValue::Int(i)) => *i,
            Some(FlatValue::Float(f)) => *f as i64,
            _ => default,
        }
    }
    fn note(&mut self, node: usize, what: &str) {
        let line = format!("{}: {what}", self.name(node));
        if !self.missing.contains(&line) {
            self.missing.push(line);
        }
    }

    /// An input as `want`: its link's value converted the way Cycles
    /// converts sockets, else the node's own value, else `default`.
    fn input(&mut self, node: usize, socket: &str, want: Kind, default: [f64; 3]) -> Expr {
        if let Some((from, out)) = self.link(node, socket) {
            if let Some(v) = self.output(from, &out) {
                return convert(v, want);
            }
            self.note(node, &format!("{socket} drawn from its own value"));
        }
        self.own(node, socket, want, default)
    }
    fn own(&self, node: usize, socket: &str, want: Kind, default: [f64; 3]) -> Expr {
        match self.g.value(node, socket) {
            Some(FlatValue::Float(f)) => s(*f),
            Some(FlatValue::Vec3(v)) => convert(
                Val {
                    e: c3(*v),
                    kind: Kind::Color,
                },
                want,
            ),
            Some(FlatValue::Int(i)) => s(*i as f64),
            Some(FlatValue::Bool(b)) => s(f64::from(u8::from(*b))),
            Some(FlatValue::Driven(i)) => match self.drivers.get(*i) {
                Some(e) => convert(
                    Val {
                        e: e.clone(),
                        kind: Kind::Color,
                    },
                    want,
                ),
                None => c3(default),
            },
            _ => c3(default),
        }
    }
    fn float(&mut self, node: usize, socket: &str, default: f64) -> Expr {
        self.input(node, socket, Kind::Float, [default; 3])
    }
    fn color(&mut self, node: usize, socket: &str, default: [f64; 3]) -> Expr {
        self.input(node, socket, Kind::Color, default)
    }
    fn vector(&mut self, node: usize, socket: &str, default: [f64; 3]) -> Expr {
        self.input(node, socket, Kind::Vector, default)
    }
    /// A texture's Vector: its link, else the coordinates Cycles links in
    /// by itself (Generated, or UV for images).
    fn coords(&mut self, node: usize, uv: bool) -> Expr {
        if let Some((from, out)) = self.link(node, "Vector")
            && let Some(v) = self.output(from, &out)
        {
            return convert(v, Kind::Vector);
        }
        if uv {
            Expr::mesh_uv(None, [1., 1.], 0.)
        } else {
            Expr::Locator(Box::new(self.space.generated))
        }
    }

    /// An output's value, `None` when it can't be drawn.
    fn output(&mut self, node: usize, socket: &str) -> Option<Val> {
        let key = (node, socket.to_string());
        if let Some((v, size)) = self.memo.get(&key) {
            let (v, size) = (v.clone(), *size);
            *self.sizes.last_mut().unwrap_or(&mut 0) += size;
            return v;
        }
        // A cycle (a broken file) or a runaway chain.
        if self.depth > 256 {
            return None;
        }
        self.depth += 1;
        self.sizes.push(1);
        let v = self.build(node, socket);
        let size = self.sizes.pop().unwrap_or(1);
        self.depth -= 1;
        let v = v.filter(|_| size <= MAX_SIZE);
        if let Some(top) = self.sizes.last_mut() {
            *top += size;
        }
        self.memo.insert(key, (v.clone(), size));
        v
    }

    fn build(&mut self, n: usize, out: &str) -> Option<Val> {
        use Kind::*;
        let val = |e: Expr, kind: Kind| Some(Val { e, kind });
        match (self.cycles(n), out) {
            ("value", "Value") => val(self.own(n, "Value", Float, [0.; 3]), Float),
            ("color", "Color") => val(self.own(n, "Value", Color, [0.; 3]), Color),
            ("math", "Value") => {
                let op = MathOp::from_name(&self.text(n, "Type", "add"))?;
                let clamp = self.flag(n, "Use Clamp", false);
                let a = self.float(n, "Value1", 0.5);
                let b = self.float(n, "Value2", 0.5);
                let c = self.float(n, "Value3", 0.);
                val(func(Func::Math { op, clamp }, vec![a, b, c]), Float)
            }
            ("vector_math", "Value" | "Vector") => {
                let op = VectorOp::from_name(&self.text(n, "Type", "add"))?;
                if op.gives_value() != (out == "Value") {
                    // The other output of this operation is zero.
                    return val(s(0.), if out == "Value" { Float } else { Vector });
                }
                let a = self.vector(n, "Vector1", [0.; 3]);
                let b = self.vector(n, "Vector2", [0.; 3]);
                let c = self.vector(n, "Vector3", [0.; 3]);
                let k = self.float(n, "Scale", 1.);
                let e = func(Func::VectorMath(op), vec![a, b, c, k]);
                val(e, if out == "Value" { Float } else { Vector })
            }
            ("mix", "Color") => {
                let mode = self.text(n, "Type", "mix");
                let fac = self.float(n, "Fac", 0.5);
                let a = self.color(n, "Color1", [0.; 3]);
                let b = self.color(n, "Color2", [0.; 3]);
                let e = self.blend(&mode, true, fac, a, b)?;
                let e = if self.flag(n, "Use Clamp", false) {
                    func(Func::Clamp01, vec![e])
                } else {
                    e
                };
                val(e, Color)
            }
            ("mix_color", "Result") => {
                let mode = self.text(n, "Type", "mix");
                let fac = self.float(n, "Factor", 0.5);
                let a = self.color(n, "A", [0.; 3]);
                let b = self.color(n, "B", [0.; 3]);
                let clamp = self.flag(n, "Use Clamp", true);
                let e = self.blend(&mode, clamp, fac, a, b)?;
                let e = if self.flag(n, "Use Clamp Result", false) {
                    func(Func::Clamp01, vec![e])
                } else {
                    e
                };
                val(e, Color)
            }
            (kind @ ("mix_float" | "mix_vector" | "mix_vector_non_uniform"), "Result") => {
                let (want, uniform) = match kind {
                    "mix_float" => (Float, true),
                    "mix_vector" => (Vector, true),
                    _ => (Vector, false),
                };
                let fac = if uniform {
                    self.float(n, "Factor", 0.5)
                } else {
                    self.vector(n, "Factor", [0.5; 3])
                };
                let a = self.input(n, "A", want, [0.; 3]);
                let b = self.input(n, "B", want, [0.; 3]);
                let fac = if self.flag(n, "Use Clamp", true) {
                    func(Func::Clamp01, vec![fac])
                } else {
                    fac
                };
                val(func(Func::Lerp, vec![fac, a, b]), want)
            }
            ("rgb_ramp", "Color" | "Alpha") => {
                let Some(FlatValue::Ramp(table)) = self.g.value(n, "Ramp") else {
                    return None;
                };
                let stops = std::sync::Arc::new(table.clone());
                let fac = self.float(n, "Fac", 0.);
                let alpha = out == "Alpha";
                let e = Expr::Ramp {
                    input: Box::new(fac),
                    lo: 0.,
                    hi: 1.,
                    stops,
                    alpha,
                };
                val(e, if alpha { Float } else { Color })
            }
            ("invert", "Color") => {
                let fac = self.float(n, "Fac", 1.);
                let c = self.color(n, "Color", [0.; 3]);
                let inv = Expr::one_minus(c.clone());
                val(Expr::mix(Mix::Blend, fac, c, inv), Color)
            }
            ("gamma", "Color") => {
                let c = self.color(n, "Color", [0.; 3]);
                let g = self.float(n, "Gamma", 1.);
                let e = match g.constant() {
                    Some([p, ..]) if p != 0. => Expr::gamma(c, p),
                    _ => func(Func::Gamma, vec![c, g]),
                };
                val(e, Color)
            }
            ("hsv", "Color") => {
                let hue = self.float(n, "Hue", 0.5);
                let sat = self.float(n, "Saturation", 1.);
                let v = self.float(n, "Value", 1.);
                let fac = self.float(n, "Fac", 1.);
                let c = self.color(n, "Color", [0.; 3]);
                val(func(Func::HueSatVal, vec![hue, sat, v, fac, c]), Color)
            }
            ("separate_color", "Red" | "Green" | "Blue") => {
                let c = self.color(n, "Color", [0.; 3]);
                let c = match self.text(n, "Type", "rgb").as_str() {
                    "hsv" => func(Func::ToHsv, vec![c]),
                    "hsl" => func(Func::ToHsl, vec![c]),
                    _ => c,
                };
                let k = match out {
                    "Red" => 0,
                    "Green" => 1,
                    _ => 2,
                };
                val(Expr::channel(c, k), Float)
            }
            ("combine_color", "Color") => {
                let r = self.float(n, "Red", 0.);
                let g = self.float(n, "Green", 0.);
                let b = self.float(n, "Blue", 0.);
                let c = func(Func::Combine, vec![r, g, b]);
                let c = match self.text(n, "Type", "rgb").as_str() {
                    "hsv" => func(Func::FromHsv, vec![c]),
                    "hsl" => func(Func::FromHsl, vec![c]),
                    _ => c,
                };
                val(c, Color)
            }
            ("separate_xyz", "X" | "Y" | "Z") => {
                let v = self.vector(n, "Vector", [0.; 3]);
                let k = match out {
                    "X" => 0,
                    "Y" => 1,
                    _ => 2,
                };
                val(Expr::channel(v, k), Float)
            }
            ("combine_xyz", "Vector") => {
                let x = self.float(n, "X", 0.);
                let y = self.float(n, "Y", 0.);
                let z = self.float(n, "Z", 0.);
                val(func(Func::Combine, vec![x, y, z]), Vector)
            }
            ("map_range", "Result") => {
                let kind = RangeKind::from_name(&self.text(n, "Type", "linear"))?;
                let clamp = self.flag(n, "Clamp", false);
                let args = vec![
                    self.float(n, "Value", 1.),
                    self.float(n, "From Min", 0.),
                    self.float(n, "From Max", 1.),
                    self.float(n, "To Min", 0.),
                    self.float(n, "To Max", 1.),
                    self.float(n, "Steps", 4.),
                ];
                val(func(Func::MapRange { kind, clamp }, args), Float)
            }
            ("mapping", "Vector") => {
                let kind = MappingKind::from_name(&self.text(n, "Type", "point"))?;
                let args = vec![
                    self.vector(n, "Vector", [0.; 3]),
                    self.vector(n, "Location", [0.; 3]),
                    self.vector(n, "Rotation", [0.; 3]),
                    self.vector(n, "Scale", [1.; 3]),
                ];
                val(func(Func::Mapping(kind), args), Vector)
            }
            ("texture_coordinate", out) => {
                let e = match out {
                    "Generated" => Expr::Locator(Box::new(self.space.generated)),
                    "UV" => Expr::mesh_uv(None, [1., 1.], 0.),
                    "Object" => Expr::Locator(Box::new(GraphSpace::default().generated)),
                    "Normal" => Expr::Normal(Box::new(GraphSpace::default().normal_to_world)),
                    _ => return None,
                };
                val(e, Vector)
            }
            ("geometry", out) => {
                let e = match out {
                    "Position" => Expr::Locator(Box::new(self.space.to_world)),
                    "Normal" | "True Normal" => Expr::Normal(Box::new(self.space.normal_to_world)),
                    "Backfacing" => return val(Expr::Geometry(GeometryInput::BackFacing), Float),
                    _ => return None,
                };
                val(e, Vector)
            }
            ("fresnel", "Fac") => {
                let ior = self.float(n, "IOR", 1.5);
                let inc = Expr::Geometry(GeometryInput::Incidence);
                val(func(Func::Fresnel, vec![ior, inc]), Float)
            }
            ("layer_weight", "Fresnel" | "Facing") => {
                let blend = self.float(n, "Blend", 0.5);
                let inc = Expr::Geometry(GeometryInput::Incidence);
                let facing = out == "Facing";
                val(func(Func::LayerWeight { facing }, vec![blend, inc]), Float)
            }
            ("image_texture", "Color" | "Alpha") => {
                let Some(FlatValue::Text(path)) = self.g.value(n, "Filename") else {
                    return None;
                };
                let path = path.clone();
                let Some(id) = (self.image)(&path) else {
                    self.note(n, "picture not loaded");
                    return None;
                };
                let filtering = match self.text(n, "Interpolation", "linear").as_str() {
                    "closest" => Filtering::Nearest,
                    _ => Filtering::Bilinear,
                };
                let extension = match self.text(n, "Extension", "periodic").as_str() {
                    "clamp" => Tiling::Edge,
                    "black" => Tiling::Reset,
                    "mirror" => Tiling::Mirror,
                    _ => Tiling::Repeat,
                };
                if self.text(n, "Projection", "flat") != "flat" {
                    self.note(n, "drawn with flat projection");
                }
                let vector = self.coords(n, true);
                let lookup = Box::new(ImageLookup {
                    image: id,
                    filtering,
                    vector,
                    extension,
                });
                if out == "Alpha" {
                    return val(Expr::ImageAlpha(lookup), Float);
                }
                // The viewport decodes every 8-bit picture as sRGB; a data
                // picture (roughness, normals) wants its numbers back.
                let colour = Expr::ImageColor(lookup);
                let data = self.text(n, "Colorspace", "__builtin_srgb") == "data";
                val(
                    if data {
                        func(Func::SrgbEncode, vec![colour])
                    } else {
                        colour
                    },
                    Color,
                )
            }
            ("noise_texture", "Fac" | "Color") => {
                let dims = self.int(n, "Dimensions", 3).clamp(1, 4) as u8;
                let kind = NoiseKind::from_name(&self.text(n, "Type", "fBM"))?;
                let normalize = self.flag(n, "Normalize", true);
                let color = out == "Color";
                let args = vec![
                    self.coords(n, false),
                    self.float(n, "W", 0.),
                    self.float(n, "Scale", 1.),
                    self.float(n, "Detail", 2.),
                    self.float(n, "Roughness", 0.5),
                    self.float(n, "Lacunarity", 2.),
                    self.float(n, "Offset", 0.),
                    self.float(n, "Gain", 1.),
                    self.float(n, "Distortion", 0.),
                ];
                let f = Func::Noise {
                    dims,
                    kind,
                    normalize,
                    color,
                };
                val(func(f, args), if color { Color } else { Float })
            }
            ("white_noise_texture", "Value" | "Color") => {
                let dims = self.int(n, "Dimensions", 3).clamp(1, 4) as u8;
                let color = out == "Color";
                let args = vec![self.vector(n, "Vector", [0.; 3]), self.float(n, "W", 0.)];
                let e = func(Func::WhiteNoise { dims, color }, args);
                val(e, if color { Color } else { Float })
            }
            ("checker_texture", "Color" | "Fac") => {
                let color = out == "Color";
                let args = vec![
                    self.coords(n, false),
                    self.color(n, "Color1", [0.; 3]),
                    self.color(n, "Color2", [0.; 3]),
                    self.float(n, "Scale", 1.),
                ];
                let e = func(Func::Checker { color }, args);
                val(e, if color { Color } else { Float })
            }
            ("gradient_texture", "Fac" | "Color") => {
                let kind = GradientKind::from_name(&self.text(n, "Type", "linear"))?;
                let v = self.coords(n, false);
                let e = func(Func::Gradient(kind), vec![v]);
                val(e, if out == "Color" { Color } else { Float })
            }
            ("wave_texture", "Fac" | "Color") => {
                let rings = self.text(n, "Type", "bands") == "rings";
                let dir = if rings {
                    self.text(n, "Rings Direction", "x")
                } else {
                    self.text(n, "Bands Direction", "x")
                };
                let direction = match dir.as_str() {
                    "x" => 0,
                    "y" => 1,
                    "z" => 2,
                    _ => 3,
                };
                let profile = match self.text(n, "Profile", "sine").as_str() {
                    "saw" => 1,
                    "tri" => 2,
                    _ => 0,
                };
                let args = vec![
                    self.coords(n, false),
                    self.float(n, "Scale", 1.),
                    self.float(n, "Distortion", 0.),
                    self.float(n, "Detail", 2.),
                    self.float(n, "Detail Scale", 0.),
                    self.float(n, "Detail Roughness", 0.5),
                    self.float(n, "Phase Offset", 0.),
                ];
                let k = WaveKind {
                    rings,
                    direction,
                    profile,
                };
                let e = func(Func::Wave(k), args);
                val(e, if out == "Color" { Color } else { Float })
            }
            ("voronoi_texture", socket) => {
                use crate::cycles_voronoi::{VoronoiFeature, VoronoiMetric, VoronoiOut};
                let which = VoronoiOut::from_socket(socket)?;
                let f = Func::Voronoi {
                    dims: self.int(n, "Dimensions", 3).clamp(1, 4) as u8,
                    feature: VoronoiFeature::from_name(&self.text(n, "Feature", "f1"))?,
                    metric: VoronoiMetric::from_name(&self.text(
                        n,
                        "Distance Metric",
                        "euclidean",
                    ))?,
                    normalize: self.flag(n, "Normalize", false),
                    out: which,
                };
                let args = vec![
                    self.coords(n, false),
                    self.float(n, "W", 0.),
                    self.float(n, "Scale", 5.),
                    self.float(n, "Detail", 0.),
                    self.float(n, "Roughness", 0.5),
                    self.float(n, "Lacunarity", 2.),
                    self.float(n, "Smoothness", 5.),
                    self.float(n, "Exponent", 0.5),
                    self.float(n, "Randomness", 1.),
                ];
                let kind = match which {
                    VoronoiOut::Color => Color,
                    VoronoiOut::Position => Vector,
                    _ => Float,
                };
                val(func(f, args), kind)
            }
            _ => None,
        }
    }

    /// A colour blend by Cycles' name; `clamp` keeps the factor in 0..1.
    fn blend(&mut self, mode: &str, clamp: bool, fac: Expr, a: Expr, b: Expr) -> Option<Expr> {
        let simple = match mode {
            "mix" => Some(Mix::Blend),
            "add" => Some(Mix::Add),
            "multiply" => Some(Mix::Multiply),
            "subtract" => Some(Mix::Subtract),
            "screen" => Some(Mix::Screen),
            "divide" => Some(Mix::Divide),
            "difference" => Some(Mix::Difference),
            "darken" => Some(Mix::Darken),
            "lighten" => Some(Mix::Lighten),
            "overlay" => Some(Mix::Overlay),
            "dodge" => Some(Mix::Dodge),
            "burn" => Some(Mix::Burn),
            "soft_light" => Some(Mix::SoftLight),
            _ => None,
        };
        match simple {
            // The tree's mixes keep the factor in 0..1 already; an
            // unclamped plain blend runs past the ends.
            Some(Mix::Blend) if !clamp => Some(func(Func::Lerp, vec![fac, a, b])),
            Some(m) => Some(Expr::mix(m, fac, a, b)),
            None => {
                let more = MixMore::from_name(mode)?;
                let fac = if clamp {
                    func(Func::Clamp01, vec![fac])
                } else {
                    fac
                };
                Some(func(Func::MixMore(more), vec![fac, a, b]))
            }
        }
    }

    /// The channel values of the closure `node` gives at `out`.
    fn closure(&mut self, node: usize, out: &str, depth: usize) -> Option<Closure> {
        if depth > 64 {
            return None;
        }
        let mut v: Vec<Expr> = CHANNELS.iter().map(|(_, d)| c3(*d)).collect();
        v.push(c3(FLAT_NORMAL));
        let at = |ch: Ch| CHANNELS.iter().position(|(c, _)| *c == ch).unwrap_or(0);
        let set = |v: &mut Vec<Expr>, ch: Ch, e: Expr| v[at(ch)] = e;
        match (self.cycles(node), out) {
            ("principled_bsdf", "BSDF") => {
                let base = self.color(node, "Base Color", [0.8; 3]);
                let roughness = self.float(node, "Roughness", 0.5);
                set(&mut v, Ch::DiffuseColor, base.clone());
                set(&mut v, Ch::SubsurfaceAlbedo, base.clone());
                set(&mut v, Ch::TransparentColor, base);
                set(&mut v, Ch::Roughness, roughness.clone());
                set(&mut v, Ch::RefractionRoughness, roughness);
                for (ch, socket, d) in [
                    (Ch::Metallic, "Metallic", 0.),
                    (Ch::DiffuseRoughness, "Diffuse Roughness", 0.),
                    (Ch::SpecularAmount, "Specular IOR Level", 0.5),
                    (Ch::Anisotropy, "Anisotropic", 0.),
                    (Ch::Sheen, "Sheen Weight", 0.),
                    (Ch::ClearcoatAmount, "Coat Weight", 0.),
                    (Ch::ClearcoatRoughness, "Coat Roughness", 0.03),
                    (Ch::CoatIor, "Coat IOR", 1.5),
                    (Ch::LuminousIntensity, "Emission Strength", 0.),
                    (Ch::TransparentAmount, "Transmission Weight", 0.),
                    (Ch::RefractiveIndex, "IOR", 1.5),
                    (Ch::SubsurfaceAmount, "Subsurface Weight", 0.),
                    (Ch::SubsurfaceDistance, "Subsurface Scale", 0.005),
                ] {
                    let e = self.float(node, socket, d);
                    set(&mut v, ch, e);
                }
                for (ch, socket, d) in [
                    (Ch::SpecularColor, "Specular Tint", [1.; 3]),
                    (Ch::CoatColor, "Coat Tint", [1.; 3]),
                    (Ch::LuminousColor, "Emission Color", [1.; 3]),
                ] {
                    let e = self.color(node, socket, d);
                    set(&mut v, ch, e);
                }
                let radius = self.vector(node, "Subsurface Radius", [1., 0.2, 0.1]);
                set(&mut v, Ch::SubsurfaceColor, radius);
                let alpha = self.float(node, "Alpha", 1.);
                set(&mut v, Ch::Dissolve, Expr::one_minus(alpha));
                self.normal(node, &mut v);
            }
            ("emission", "Emission") => {
                let colour = self.color(node, "Color", [0.8; 3]);
                let strength = self.float(node, "Strength", 10.);
                set(&mut v, Ch::DiffuseColor, s(0.));
                set(&mut v, Ch::SpecularAmount, s(0.));
                set(&mut v, Ch::LuminousColor, colour);
                set(&mut v, Ch::LuminousIntensity, strength);
            }
            ("transparent_bsdf", "BSDF") => {
                let colour = self.color(node, "Color", [1.; 3]);
                set(&mut v, Ch::DiffuseColor, s(0.));
                set(&mut v, Ch::SpecularAmount, s(0.));
                set(&mut v, Ch::TransparentAmount, s(1.));
                set(&mut v, Ch::TransparentColor, colour);
                set(&mut v, Ch::RefractiveIndex, s(1.));
                set(&mut v, Ch::RefractionRoughness, s(0.));
            }
            ("glass_bsdf", "BSDF") => {
                let colour = self.color(node, "Color", [0.8; 3]);
                let roughness = self.float(node, "Roughness", 0.);
                let ior = self.float(node, "IOR", 1.5);
                set(&mut v, Ch::DiffuseColor, colour.clone());
                set(&mut v, Ch::TransparentAmount, s(1.));
                set(&mut v, Ch::TransparentColor, colour);
                set(&mut v, Ch::RefractiveIndex, ior);
                set(&mut v, Ch::Roughness, roughness.clone());
                set(&mut v, Ch::RefractionRoughness, roughness);
                self.normal(node, &mut v);
            }
            ("mix_closure", "Closure") => {
                let fac = self.float(node, "Fac", 0.5);
                let side = |tx: &mut Self, socket: &str| -> Option<Closure> {
                    match tx.link(node, socket) {
                        Some((from, out)) => tx.closure(from, &out, depth + 1),
                        // Nothing linked: no light from that side (black,
                        // see-through nothing): Cycles' empty closure.
                        None => {
                            let mut v: Vec<Expr> = CHANNELS.iter().map(|(_, d)| c3(*d)).collect();
                            v.push(c3(FLAT_NORMAL));
                            v[at(Ch::DiffuseColor)] = s(0.);
                            v[at(Ch::SpecularAmount)] = s(0.);
                            Some(v)
                        }
                    }
                };
                let a = side(self, "Closure1")?;
                let b = side(self, "Closure2")?;
                v = a
                    .into_iter()
                    .zip(b)
                    .map(|(x, y)| Expr::mix(Mix::Blend, fac.clone(), x, y))
                    .collect();
            }
            _ => return None,
        }
        Some(v)
    }

    /// A shader's Normal input: a tangent-space Normal Map's colour, and a
    /// Bump's height (with any normal map under it).
    fn normal(&mut self, node: usize, v: &mut [Expr]) {
        let normal_at = v.len() - 1;
        let at = |ch: Ch| CHANNELS.iter().position(|(c, _)| *c == ch).unwrap_or(0);
        let Some((from, _)) = self.link(node, "Normal") else {
            return;
        };
        let map_from = match self.cycles(from) {
            "bump" => {
                let height = self.float(from, "Height", 1.);
                let height = if self.flag(from, "Invert", false) {
                    func(
                        Func::Math {
                            op: MathOp::Multiply,
                            clamp: false,
                        },
                        vec![height, s(-1.), s(0.)],
                    )
                } else {
                    height
                };
                let strength = self.float(from, "Strength", 1.);
                let distance = self.float(from, "Distance", 0.1);
                // Bump heights are in scene units at Distance 1 (G-07);
                // Strength blends toward the plain normal, drawn here as a
                // smaller height (provisional).
                let amount = Expr::times(strength, distance);
                v[at(Ch::Bump)] = height;
                v[at(Ch::BumpAmplitude)] = amount;
                self.link(from, "Normal").map(|(f, _)| f)
            }
            _ => Some(from),
        };
        let Some(m) = map_from.filter(|&m| self.cycles(m) == "normal_map") else {
            if map_from.is_some_and(|m| self.cycles(m) != "bump") {
                self.note(node, "Normal input not drawn");
            }
            return;
        };
        if self.text(m, "Space", "tangent") != "tangent" {
            self.note(m, "only tangent-space normal maps draw");
            return;
        }
        let colour = self.color(m, "Color", FLAT_NORMAL);
        let colour = if self.text(m, "Convention", "opengl") == "directx" {
            func(
                Func::Combine,
                vec![
                    Expr::channel(colour.clone(), 0),
                    Expr::one_minus(Expr::channel(colour.clone(), 1)),
                    Expr::channel(colour, 2),
                ],
            )
        } else {
            colour
        };
        let strength = self.float(m, "Strength", 1.);
        v[normal_at] = Expr::mix(Mix::Blend, strength, c3(FLAT_NORMAL), colour);
    }
}

/// A value converted to the kind an input wants, as Cycles converts
/// sockets: a colour read as a float is its luminance, a vector its mean;
/// a float read as a colour or vector repeats.
/// Output `socket` of node `node` of a flattened graph as a tree
/// expression, a colour when `colour`, else a number; `None` when it
/// can't be drawn (node editing, NE-03: a free node driving a channel).
pub fn output_expr(
    flat: &FlatGraph,
    node: usize,
    socket: &str,
    colour: bool,
    space: &GraphSpace,
) -> Option<Expr> {
    let mut tx = Tx {
        g: flat,
        drivers: &[],
        space,
        image: &mut |_| None,
        memo: HashMap::new(),
        sizes: vec![0],
        missing: Vec::new(),
        depth: 0,
    };
    let v = tx.output(node, socket)?;
    if tx.sizes[0] > MAX_SIZE {
        return None;
    }
    Some(convert(v, if colour { Kind::Color } else { Kind::Float }))
}

fn convert(v: Val, want: Kind) -> Expr {
    match (v.kind, want) {
        (Kind::Color, Kind::Float) => Expr::luma(v.e),
        (Kind::Vector, Kind::Float) => func(Func::Average, vec![v.e]),
        _ => v.e,
    }
}

#[cfg(test)]
#[path = "graph_view_tests.rs"]
mod tests;
