//! The shader tree's MaterialX form (materials architecture, stage 3;
//! docs/MATERIALX_GRAPHS.md).
//!
//! A surface the layer stack evaluates to ([`Principled`]: every layer's
//! blend, opacity, masks and groups already composed into one expression
//! per input) is written as a MaterialX document: one `operandi_*` node per
//! expression node, an `operandi_surface` holding the principled inputs and
//! the Shader settings, and a `surfacematerial`. [`read`] turns the
//! document back into the same `Principled`, exactly: numbers are written
//! in their shortest form that reads back to the same `f64`.
//!
//! The nodes are Operandi's own (`library.rs` declares them, with
//! standard-node implementations where MaterialX has the same sums), so
//! the form says precisely what Cycles renders. Cycles renders tree
//! materials by reading this form back (`RenderShader::compile_through_graph`
//! in operandi-cycles).
use super::doc::{self, Element, Port};
use crate::Result;
use crate::Scene;
use crate::material_values::MaterialValue;
use crate::shader_eval::{
    AoToward, Expr, FaceKind, GeometryInput, ImageLookup, Input, Mix, NodeUse, PatternLookup,
    Principled, SeeThrough, SeeThroughKind,
};
use crate::shader_tree::Filtering;
use crate::texture_locator::{FalloffType, Tiling};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

/// The surface node's category.
pub const SURFACE: &str = "operandi_surface";
/// The Node Material use's category (NM-31).
pub const NODE_MATERIAL: &str = "operandi_node_material";

/// The principled inputs in `Input` order, with their MaterialX input
/// names on `operandi_surface`.
pub const INPUTS: [(Input, &str); Input::COUNT] = {
    use Input::*;
    [
        (BaseColor, "base_color"),
        (Metallic, "metallic"),
        (Roughness, "roughness"),
        (Ior, "ior"),
        (Alpha, "alpha"),
        (DiffuseRoughness, "diffuse_roughness"),
        (SubsurfaceWeight, "subsurface_weight"),
        (SubsurfaceRadius, "subsurface_radius"),
        (SubsurfaceScale, "subsurface_scale"),
        (SpecularIorLevel, "specular_ior_level"),
        (SpecularTint, "specular_tint"),
        (Anisotropic, "anisotropic"),
        (TransmissionWeight, "transmission_weight"),
        (SheenWeight, "sheen_weight"),
        (CoatWeight, "coat_weight"),
        (CoatRoughness, "coat_roughness"),
        (EmissionColor, "emission_color"),
        (EmissionStrength, "emission_strength"),
        (SubsurfaceAnisotropy, "subsurface_anisotropy"),
        (CoatTint, "coat_tint"),
        (CoatIor, "coat_ior"),
        (SheenTint, "sheen_tint"),
        (SheenRoughness, "sheen_roughness"),
        (ThinFilmThickness, "thin_film_thickness"),
        (ThinFilmIor, "thin_film_ior"),
        (DispersionScale, "dispersion_scale"),
        (DispersionAbbe, "dispersion_abbe"),
    ]
};

/// Mix modes and their node categories (`operandi_mix_<mode>`).
pub const MIXES: [(Mix, &str); 13] = [
    (Mix::Blend, "blend"),
    (Mix::Add, "add"),
    (Mix::Multiply, "multiply"),
    (Mix::Subtract, "subtract"),
    (Mix::Screen, "screen"),
    (Mix::Divide, "divide"),
    (Mix::Difference, "difference"),
    (Mix::Darken, "darken"),
    (Mix::Lighten, "lighten"),
    (Mix::Overlay, "overlay"),
    (Mix::Dodge, "dodge"),
    (Mix::Burn, "burn"),
    (Mix::SoftLight, "soft_light"),
];

const GEOMETRY: [(GeometryInput, &str); 7] = [
    (GeometryInput::Incidence, "incidence"),
    (GeometryInput::Slope, "slope"),
    (GeometryInput::CameraDistance, "camera_distance"),
    (GeometryInput::BackFacing, "back_facing"),
    (GeometryInput::ScreenX, "screen_x"),
    (GeometryInput::ScreenY, "screen_y"),
    (GeometryInput::ScreenR, "screen_r"),
];

const TOWARD: [(AoToward, &str); 4] = [
    (AoToward::Normal, "normal"),
    (AoToward::Up, "up"),
    (AoToward::DownSlope, "down_slope"),
    (AoToward::Reflection, "reflection"),
];

/// Projection axes, in `axis` order.
pub const AXES: [&str; 3] = ["x", "y", "z"];

/// Box projection faces, in `face` order (+X, +Y, +Z, −X, −Y, −Z).
pub const BOX_FACES: [&str; 6] = ["px", "py", "pz", "nx", "ny", "nz"];

/// The one-per-face node for a face weight: `operandi_box_<face>`,
/// `operandi_cubic_<axis>`, `operandi_triplanar_<axis>`; `None` for face
/// numbers outside the kind's range (the general node keeps them).
pub fn face_category(face: u8, kind: FaceKind) -> Option<String> {
    let f = usize::from(face);
    match kind {
        FaceKind::Signed => BOX_FACES.get(f).map(|n| format!("operandi_box_{n}")),
        FaceKind::Dominant => AXES.get(f).map(|a| format!("operandi_cubic_{a}")),
        FaceKind::Power(_) => AXES.get(f).map(|a| format!("operandi_triplanar_{a}")),
    }
}

/// Image filtering and extension in MaterialX's own words (the `image`
/// node's `filtertype` and `uaddressmode`/`vaddressmode`).
pub const FILTERS: [(Filtering, &str); 3] = [
    (Filtering::Nearest, "closest"),
    (Filtering::Bilinear, "linear"),
    (Filtering::Bicubic, "cubic"),
];
pub const ADDRESS: [(Tiling, &str); 4] = [
    (Tiling::Reset, "constant"),
    (Tiling::Repeat, "periodic"),
    (Tiling::Mirror, "mirror"),
    (Tiling::Edge, "clamp"),
];

/// Per-axis tiling in `operandi_tile_<horizontal>_<vertical>`.
pub const TILINGS: [(Tiling, &str); 4] = [
    (Tiling::Reset, "reset"),
    (Tiling::Repeat, "repeat"),
    (Tiling::Mirror, "mirror"),
    (Tiling::Edge, "edge"),
];

/// Falloff shapes in `operandi_falloff_<shape>`.
pub const FALLOFFS: [(FalloffType, &str); 5] = [
    (FalloffType::Cubic, "cubic"),
    (FalloffType::Spherical, "spherical"),
    (FalloffType::LinearX, "linear_x"),
    (FalloffType::LinearY, "linear_y"),
    (FalloffType::LinearZ, "linear_z"),
];

fn name_of<T: PartialEq + Copy>(table: &[(T, &'static str)], v: T) -> &'static str {
    table
        .iter()
        .find(|(t, _)| *t == v)
        .map(|(_, n)| *n)
        .expect("every variant is in its table")
}
fn value_of<T: Copy>(table: &[(T, &'static str)], name: &str, what: &str) -> Result<T> {
    table
        .iter()
        .find(|(_, n)| *n == name)
        .map(|(t, _)| *t)
        .ok_or_else(|| format!("unknown {what} '{name}'"))
}

/// The shortest text that reads back as the same number.
fn num(v: f64) -> String {
    format!("{v}")
}
fn nums(v: &[f64]) -> String {
    v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(", ")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// What an input holds: a value or another node's output.
enum V {
    Value(String),
    Node(String),
}

struct In {
    name: String,
    ty: &'static str,
    v: V,
}

/// The surface and its nodes as MaterialX text; `name` names the
/// material.
pub fn write(p: &Principled, scene: &Scene, name: &str) -> String {
    let mut w = Writer {
        scene,
        free: Default::default(),
        seen: Vec::new(),
        body: String::new(),
    };
    let mut surface: Vec<In> = Vec::new();
    for (input, e) in &p.inputs {
        surface.push(w.expr(name_of(&INPUTS, *input), e));
    }
    surface.push(w.string(
        "shader",
        &serde_json::to_string(&p.shader).unwrap_or_default(),
    ));
    surface.push(w.float("coat_bump", p.coat_bump));
    if let Some(e) = &p.bump {
        surface.push(w.expr("bump", e));
    }
    if let Some(e) = &p.normal_map {
        surface.push(w.expr("normal_map", e));
    }
    if let Some(e) = &p.displacement {
        surface.push(w.expr("displacement", e));
    }
    if let Some(s) = &p.see_through {
        let kind = match s.kind {
            SeeThroughKind::Transparent => "transparent",
            SeeThroughKind::Glass => "glass",
        };
        surface.push(w.string("see_through", kind));
        surface.push(w.expr("see_through_amount", &s.amount));
        surface.push(w.expr("see_through_color", &s.color));
        surface.push(w.expr("see_through_ior", &s.ior));
        surface.push(w.expr("see_through_roughness", &s.roughness));
    }
    if let Some(r) = p.rounded_edge {
        surface.push(w.float("rounded_edge", r));
    }
    if let Some((c, d)) = p.absorption {
        surface.push(w.vector3("absorption_color", c));
        surface.push(w.float("absorption_density", d));
    }
    if let Some((c, d, g)) = p.scatter {
        surface.push(w.vector3("scatter_color", c));
        surface.push(w.float("scatter_density", d));
        surface.push(w.float("scatter_anisotropy", g));
    }
    surface.push(w.boolean("thin_walled", p.thin_walled));
    surface.push(w.boolean("glass_principled", p.glass_principled));
    for (v, e) in &p.values {
        surface.push(w.expr(&format!("value_{}", v.name()), e));
    }
    if let Some(n) = &p.node {
        let mut ins = vec![
            w.string("layer", &n.layer.to_string()),
            w.string("graph", &n.graph.to_string()),
            w.expr("coverage", &n.coverage),
            w.string(
                "values",
                &serde_json::to_string(&n.values).unwrap_or_default(),
            ),
        ];
        let names: Vec<&str> = n.drivers.iter().map(|(d, _)| d.as_str()).collect();
        ins.push(w.string(
            "driver_names",
            &serde_json::to_string(&names).unwrap_or_default(),
        ));
        for (i, (_, e)) in n.drivers.iter().enumerate() {
            ins.push(w.expr(&format!("driver_{i}"), e));
        }
        w.element(NODE_MATERIAL, "node_material", "color3", &ins);
    }
    let shader_name = "surface";
    w.element(SURFACE, shader_name, "surfaceshader", &surface);
    let material = super::export::mx_name(name);
    let mut out = String::from("<?xml version=\"1.0\"?>\n<materialx version=\"1.39\">\n");
    out.push_str(&w.body);
    let _ = writeln!(
        out,
        "  <surfacematerial name=\"{}\" type=\"material\">\n    <input name=\"surfaceshader\" type=\"surfaceshader\" nodename=\"{shader_name}\" />\n  </surfacematerial>",
        esc(&material)
    );
    out.push_str("</materialx>\n");
    out
}

struct Writer<'a> {
    scene: &'a Scene,
    /// Free nodes already written (node editing).
    free: std::collections::BTreeSet<String>,
    /// Expressions already written, with their node names: a shared
    /// subexpression is one node.
    seen: Vec<(Expr, String)>,
    body: String,
}

impl Writer<'_> {
    fn param(&self, name: &str, ty: &'static str, value: String) -> In {
        In {
            name: name.into(),
            ty,
            v: V::Value(value),
        }
    }
    fn string(&self, name: &str, s: &str) -> In {
        self.param(name, "string", s.into())
    }
    fn float(&self, name: &str, v: f64) -> In {
        self.param(name, "float", num(v))
    }
    fn integer(&self, name: &str, v: impl Into<i64>) -> In {
        self.param(name, "integer", v.into().to_string())
    }
    fn boolean(&self, name: &str, v: bool) -> In {
        self.param(name, "boolean", v.to_string())
    }
    fn vector2(&self, name: &str, v: [f64; 2]) -> In {
        self.param(name, "vector2", nums(&v))
    }
    fn vector3(&self, name: &str, v: [f64; 3]) -> In {
        self.param(name, "vector3", nums(&v))
    }
    fn floats(&self, name: &str, v: &[f64]) -> In {
        self.param(name, "floatarray", nums(v))
    }

    /// An expression input: a constant inline, else its node.
    fn expr(&mut self, name: &str, e: &Expr) -> In {
        let v = match e {
            Expr::Const(c) => V::Value(nums(c)),
            _ => V::Node(self.node(e)),
        };
        In {
            name: name.into(),
            ty: "color3",
            v,
        }
    }

    fn element(&mut self, category: &str, name: &str, ty: &str, inputs: &[In]) {
        let _ = writeln!(self.body, "  <{category} name=\"{name}\" type=\"{ty}\">");
        for i in inputs {
            let (attr, v) = match &i.v {
                V::Value(v) => ("value", v),
                V::Node(n) => ("nodename", n),
            };
            let _ = writeln!(
                self.body,
                "    <input name=\"{}\" type=\"{}\" {attr}=\"{}\" />",
                i.name,
                i.ty,
                esc(v)
            );
        }
        let _ = writeln!(self.body, "  </{category}>");
    }

    fn pattern(&mut self, l: &PatternLookup) -> Vec<In> {
        vec![
            self.expr("in", &l.vector),
            self.integer("axis", l.axis),
            self.string(
                "pattern",
                &serde_json::to_string(&l.pattern).unwrap_or_default(),
            ),
        ]
    }

    fn image(&mut self, l: &ImageLookup) -> Vec<In> {
        let file = self
            .scene
            .images
            .iter()
            .find(|i| i.id == l.image)
            .map_or(String::new(), |i| i.path.clone());
        vec![
            self.string("image", &l.image.to_string()),
            self.param("file", "filename", file),
            self.string("filtertype", name_of(&FILTERS, l.filtering)),
            self.string("addressmode", name_of(&ADDRESS, l.extension)),
            self.expr("texcoord", &l.vector),
        ]
    }

    /// A locator-style matrix's rows (row-major, `width` numbers each).
    fn rows(&self, m: &[f64], width: usize) -> Vec<In> {
        let ty = if width == 4 { "vector4" } else { "vector3" };
        m.chunks(width)
            .enumerate()
            .map(|(i, r)| self.param(&format!("row{i}"), ty, nums(r)))
            .collect()
    }

    /// Writes `e`'s node (after the nodes it reads) and names it.
    fn node(&mut self, e: &Expr) -> String {
        if let Some((_, n)) = self.seen.iter().find(|(x, _)| x == e) {
            return n.clone();
        }
        // A free node's output (NE-03): the standard MaterialX nodes as
        // they are, each once, so the form is the combined graph.
        if let Expr::Nodes(n) = e {
            for el in &n.elements {
                if self.free.insert(el.name.clone()) {
                    super::write::element(&mut self.body, el, "  ");
                }
            }
            self.seen.push((e.clone(), n.node.clone()));
            return n.node.clone();
        }
        let (category, inputs): (String, Vec<In>) = match e {
            Expr::Const(c) => ("operandi_constant".into(), vec![self.vector3("value", *c)]),
            // Only graph layers' viewport stand-ins make these, never a
            // tree surface; written as their average if one ever arrives.
            Expr::Nodes(_) => unreachable!("written above"),
            Expr::Func(_) => (
                "operandi_constant".into(),
                vec![self.vector3("value", e.eval(&|_, _| [0.5; 4]))],
            ),
            Expr::ImageColor(l) => ("operandi_image".into(), self.image(l)),
            Expr::ImageAlpha(l) => ("operandi_image_alpha".into(), self.image(l)),
            Expr::Locator(m) => ("operandi_locator".into(), self.rows(&m[..], 4)),
            // One node per projection axis, so each has a MaterialX
            // implementation; other numbers keep the general node.
            Expr::Planar {
                of,
                axis,
                centre,
                repeat,
                mirror,
            } if *axis <= 2 && *mirror <= 3 => (
                format!("operandi_planar_{}", AXES[usize::from(*axis)]),
                vec![
                    self.expr("in", of),
                    self.float("centre", *centre),
                    self.vector2("repeat", *repeat),
                    self.boolean("mirror_u", mirror & 1 != 0),
                    self.boolean("mirror_v", mirror & 2 != 0),
                ],
            ),
            Expr::Planar {
                of,
                axis,
                centre,
                repeat,
                mirror,
            } => (
                "operandi_planar".into(),
                vec![
                    self.expr("in", of),
                    self.integer("axis", *axis),
                    self.float("centre", *centre),
                    self.vector2("repeat", *repeat),
                    self.integer("mirror", *mirror),
                ],
            ),
            Expr::Wrap {
                of,
                axis,
                spherical,
                centre,
                repeat,
            } if *axis <= 2 => (
                format!(
                    "operandi_wrap_{}_{}",
                    if *spherical {
                        "spherical"
                    } else {
                        "cylindrical"
                    },
                    AXES[usize::from(*axis)]
                ),
                vec![
                    self.expr("in", of),
                    self.float("centre", *centre),
                    self.vector2("repeat", *repeat),
                ],
            ),
            Expr::Wrap {
                of,
                axis,
                spherical,
                centre,
                repeat,
            } => (
                "operandi_wrap".into(),
                vec![
                    self.expr("in", of),
                    self.integer("axis", *axis),
                    self.boolean("spherical", *spherical),
                    self.float("centre", *centre),
                    self.vector2("repeat", *repeat),
                ],
            ),
            Expr::Probe { of, axis, repeat } if *axis <= 2 => (
                format!("operandi_probe_{}", AXES[usize::from(*axis)]),
                vec![self.expr("in", of), self.vector2("repeat", *repeat)],
            ),
            Expr::Probe { of, axis, repeat } => (
                "operandi_probe".into(),
                vec![
                    self.expr("in", of),
                    self.integer("axis", *axis),
                    self.vector2("repeat", *repeat),
                ],
            ),
            Expr::Normal(m) => ("operandi_normal".into(), self.rows(&m[..], 3)),
            Expr::FaceWeight { normal, face, kind } if face_category(*face, *kind).is_some() => {
                let mut ins = vec![self.expr("in", normal)];
                if let FaceKind::Power(k) = kind {
                    ins.push(self.float("power", *k));
                }
                (face_category(*face, *kind).expect("checked"), ins)
            }
            Expr::FaceWeight { normal, face, kind } => {
                let mut ins = vec![self.expr("in", normal), self.integer("face", *face)];
                match kind {
                    FaceKind::Signed => ins.push(self.string("kind", "signed")),
                    FaceKind::Dominant => ins.push(self.string("kind", "dominant")),
                    FaceKind::Power(k) => {
                        ins.push(self.string("kind", "power"));
                        ins.push(self.float("power", *k));
                    }
                }
                ("operandi_face_weight".into(), ins)
            }
            Expr::Facing(n, axis) => (
                "operandi_facing".into(),
                vec![self.expr("in", n), self.integer("axis", *axis)],
            ),
            Expr::Falloff { of, kind, amount } => (
                format!("operandi_falloff_{}", name_of(&FALLOFFS, *kind)),
                vec![self.expr("in", of), self.vector3("amount", *amount)],
            ),
            Expr::MeshUv {
                map,
                repeat,
                rotation,
            } => {
                let mut ins = Vec::new();
                if let Some(m) = map {
                    ins.push(self.string("map", m));
                }
                ins.push(self.vector2("repeat", *repeat));
                ins.push(self.float("rotation", *rotation));
                let category = if map.is_some() {
                    "operandi_texcoord_map"
                } else {
                    "operandi_texcoord"
                };
                (category.into(), ins)
            }
            Expr::Tile(v, t) => (
                format!(
                    "operandi_tile_{}_{}",
                    name_of(&TILINGS, t[0]),
                    name_of(&TILINGS, t[1])
                ),
                vec![self.expr("in", v)],
            ),
            Expr::Inside(v, b) => (
                "operandi_inside".into(),
                vec![
                    self.expr("in", v),
                    self.boolean("horizontal", b[0]),
                    self.boolean("vertical", b[1]),
                ],
            ),
            Expr::Pattern(l) => ("operandi_pattern".into(), self.pattern(l)),
            Expr::CellRandom(l) => ("operandi_cell_random".into(), self.pattern(l)),
            Expr::BiasGain(v, b, g) => (
                "operandi_bias_gain".into(),
                vec![
                    self.expr("in", v),
                    self.float("bias", *b),
                    self.float("gain", *g),
                ],
            ),
            Expr::HsvVary {
                color,
                random,
                amounts,
            } => (
                "operandi_hsv_vary".into(),
                vec![
                    self.expr("in", color),
                    self.expr("random", random),
                    self.vector3("amounts", *amounts),
                ],
            ),
            Expr::Luma(v) => ("operandi_luma".into(), vec![self.expr("in", v)]),
            Expr::Channel(v, k) => (
                "operandi_channel".into(),
                vec![self.expr("in", v), self.integer("channel", *k)],
            ),
            Expr::Gamma(v, g) => (
                "operandi_gamma".into(),
                vec![self.expr("in", v), self.float("power", *g)],
            ),
            Expr::Mix { mode, fac, a, b } => (
                format!("operandi_mix_{}", name_of(&MIXES, *mode)),
                vec![
                    self.expr("bg", a),
                    self.expr("fg", b),
                    self.expr("mix", fac),
                ],
            ),
            Expr::Geometry(g) => (
                "operandi_geometry".into(),
                vec![self.string("kind", name_of(&GEOMETRY, *g))],
            ),
            Expr::DistanceTo(p) => (
                "operandi_distance".into(),
                vec![self.vector3("position", *p)],
            ),
            Expr::LocatorIncidence(p) => (
                "operandi_locator_incidence".into(),
                vec![self.vector3("position", *p)],
            ),
            Expr::Hsv {
                of,
                hue_scale,
                hue_shift,
                saturation_scale,
            } => (
                "operandi_hsv".into(),
                vec![
                    self.expr("in", of),
                    self.float("hue_scale", *hue_scale),
                    self.float("hue_shift", *hue_shift),
                    self.float("saturation_scale", *saturation_scale),
                ],
            ),
            Expr::Occlusion {
                rays,
                distance,
                same_surface,
                inside,
                toward,
            } => (
                "operandi_occlusion".into(),
                vec![
                    self.integer("rays", *rays),
                    self.float("distance", *distance),
                    self.boolean("same_surface", *same_surface),
                    self.boolean("inside", *inside),
                    self.string("toward", name_of(&TOWARD, *toward)),
                ],
            ),
            Expr::Attribute { name, alpha } => (
                if *alpha {
                    "operandi_attribute_alpha"
                } else {
                    "operandi_attribute"
                }
                .into(),
                vec![self.string("attribute", name)],
            ),
            Expr::Ramp {
                input,
                lo,
                hi,
                stops,
                alpha,
            } => {
                let flat: Vec<f64> = stops.iter().flatten().copied().collect();
                (
                    "operandi_ramp".into(),
                    vec![
                        self.expr("in", input),
                        self.float("lo", *lo),
                        self.float("hi", *hi),
                        self.floats("stops", &flat),
                        self.boolean("alpha", *alpha),
                    ],
                )
            }
        };
        let name = format!("n{}", self.seen.len() + 1);
        self.element(&category, &name, "color3", &inputs);
        self.seen.push((e.clone(), name.clone()));
        name
    }
}

/// The surface a document [`write`] made, read back.
pub fn read(text: &str) -> Result<Principled> {
    let doc = doc::parse(text)?;
    let by_name: HashMap<&str, &Element> =
        doc.elements.iter().map(|e| (e.name.as_str(), e)).collect();
    let surface = doc
        .all(SURFACE)
        .next()
        .ok_or_else(|| format!("no {SURFACE} in the MaterialX graph"))?;
    let mut r = Reader {
        by_name,
        memo: HashMap::new(),
    };
    let mut p = Principled {
        inputs: Vec::new(),
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
    let mut see_through: Option<SeeThroughKind> = None;
    let mut absorption = (None, None);
    let mut scatter = (None, None, None);
    for port in &surface.inputs {
        let name = port.name.as_str();
        if let Ok(input) = value_of(&INPUTS, name, "input") {
            p.inputs.push((input, r.expr(port)?));
            continue;
        }
        if let Some(v) = name.strip_prefix("value_") {
            let value = MaterialValue::ALL
                .into_iter()
                .find(|m| m.name() == v)
                .ok_or_else(|| format!("unknown material value '{v}'"))?;
            p.values.push((value, r.expr(port)?));
            continue;
        }
        match name {
            "shader" => {
                p.shader = serde_json::from_str(&text_of(port)?)
                    .map_err(|e| format!("{SURFACE} shader: {e}"))?;
            }
            "coat_bump" => p.coat_bump = float(port)?,
            "bump" => p.bump = Some(r.expr(port)?),
            "normal_map" => p.normal_map = Some(r.expr(port)?),
            "displacement" => p.displacement = Some(r.expr(port)?),
            "see_through" => {
                see_through = Some(match text_of(port)?.as_str() {
                    "transparent" => SeeThroughKind::Transparent,
                    "glass" => SeeThroughKind::Glass,
                    other => return Err(format!("unknown see-through kind '{other}'")),
                });
            }
            "see_through_amount"
            | "see_through_color"
            | "see_through_ior"
            | "see_through_roughness" => {}
            "rounded_edge" => p.rounded_edge = Some(float(port)?),
            "absorption_color" => absorption.0 = Some(fixed::<3>(port)?),
            "absorption_density" => absorption.1 = Some(float(port)?),
            "scatter_color" => scatter.0 = Some(fixed::<3>(port)?),
            "scatter_density" => scatter.1 = Some(float(port)?),
            "scatter_anisotropy" => scatter.2 = Some(float(port)?),
            "thin_walled" => p.thin_walled = boolean(port)?,
            "glass_principled" => p.glass_principled = boolean(port)?,
            other => return Err(format!("unknown {SURFACE} input '{other}'")),
        }
    }
    if let Some(kind) = see_through {
        let mut part = |n: &str| -> Result<Expr> {
            let port = surface
                .input(n)
                .ok_or_else(|| format!("{SURFACE} has no {n}"))?;
            r.expr(port)
        };
        p.see_through = Some(SeeThrough {
            kind,
            amount: part("see_through_amount")?,
            color: part("see_through_color")?,
            ior: part("see_through_ior")?,
            roughness: part("see_through_roughness")?,
        });
    }
    if let (Some(c), Some(d)) = absorption {
        p.absorption = Some((c, d));
    }
    if let (Some(c), Some(d), Some(g)) = scatter {
        p.scatter = Some((c, d, g));
    }
    if let Some(n) = doc.all(NODE_MATERIAL).next() {
        let get = |k: &str| {
            n.input(k)
                .ok_or_else(|| format!("{NODE_MATERIAL} has no {k}"))
        };
        let id = |k: &str| -> Result<u64> {
            text_of(get(k)?)?
                .parse()
                .map_err(|e| format!("{NODE_MATERIAL} {k}: {e}"))
        };
        let names: Vec<String> = serde_json::from_str(&text_of(get("driver_names")?)?)
            .map_err(|e| format!("{NODE_MATERIAL} driver_names: {e}"))?;
        let mut drivers = Vec::new();
        for (i, d) in names.into_iter().enumerate() {
            drivers.push((d, r.expr(get(&format!("driver_{i}"))?)?));
        }
        p.node = Some(NodeUse {
            layer: id("layer")?,
            graph: id("graph")?,
            coverage: r.expr(get("coverage")?)?,
            values: serde_json::from_str(&text_of(get("values")?)?)
                .map_err(|e| format!("{NODE_MATERIAL} values: {e}"))?,
            drivers,
        });
    }
    Ok(p)
}

fn text_of(p: &Port) -> Result<String> {
    p.value
        .clone()
        .ok_or_else(|| format!("input '{}' has no value", p.name))
}
fn numbers(p: &Port) -> Result<Vec<f64>> {
    let t = text_of(p)?;
    if t.trim().is_empty() {
        return Ok(Vec::new());
    }
    t.split(',')
        .map(|s| s.trim().parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| format!("input '{}': {e}", p.name))
}
fn fixed<const N: usize>(p: &Port) -> Result<[f64; N]> {
    numbers(p)?
        .try_into()
        .map_err(|_| format!("input '{}' needs {N} numbers", p.name))
}
fn float(p: &Port) -> Result<f64> {
    Ok(fixed::<1>(p)?[0])
}
fn boolean(p: &Port) -> Result<bool> {
    match text_of(p)?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("input '{}': '{other}' isn't a boolean", p.name)),
    }
}
fn integer<T: TryFrom<i64>>(p: &Port) -> Result<T> {
    text_of(p)?
        .parse::<i64>()
        .ok()
        .and_then(|v| T::try_from(v).ok())
        .ok_or_else(|| format!("input '{}' isn't a fitting integer", p.name))
}

struct Reader<'a> {
    by_name: HashMap<&'a str, &'a Element>,
    memo: HashMap<String, Expr>,
}

impl Reader<'_> {
    /// An expression input: its value, or the node it reads.
    fn expr(&mut self, port: &Port) -> Result<Expr> {
        match &port.nodename {
            Some(n) => self.node(n),
            None => Ok(Expr::Const(fixed::<3>(port)?)),
        }
    }

    fn node(&mut self, name: &str) -> Result<Expr> {
        if let Some(e) = self.memo.get(name) {
            return Ok(e.clone());
        }
        let el = *self
            .by_name
            .get(name)
            .ok_or_else(|| format!("no node '{name}' in the MaterialX graph"))?;
        // Not one of the tree's own nodes: a free node and what it reads
        // (node editing, NE-03).
        let e = if !el.category.starts_with("operandi_") {
            let by_name = &self.by_name;
            let elements = crate::node_view::collect(name, &|n| {
                by_name
                    .get(n)
                    .copied()
                    .filter(|e| !e.category.starts_with("operandi_"))
            });
            crate::node_view::node_out(elements, name, "out")
                .ok_or_else(|| format!("{} ({}): can't be rendered", el.name, el.category))?
        } else {
            self.build(el)
                .map_err(|e| format!("{} ({}): {e}", el.name, el.category))?
        };
        self.memo.insert(name.to_owned(), e.clone());
        Ok(e)
    }

    fn build(&mut self, el: &Element) -> Result<Expr> {
        let get = |k: &str| el.input(k).ok_or_else(|| format!("no input '{k}'"));
        let input = |s: &mut Self, k: &str| -> Result<Box<Expr>> { Ok(Box::new(s.expr(get(k)?)?)) };
        let pattern = |s: &mut Self| -> Result<Box<PatternLookup>> {
            Ok(Box::new(PatternLookup {
                pattern: serde_json::from_str(&text_of(get("pattern")?)?)
                    .map_err(|e| format!("pattern: {e}"))?,
                axis: integer(get("axis")?)?,
                vector: s.expr(get("in")?)?,
            }))
        };
        let cat = el.category.as_str();
        if let Some(mode) = cat.strip_prefix("operandi_mix_") {
            return Ok(Expr::Mix {
                mode: value_of(&MIXES, mode, "mix mode")?,
                fac: input(self, "mix")?,
                a: input(self, "bg")?,
                b: input(self, "fg")?,
            });
        }
        let axis_of = |a: &str| -> Result<u8> {
            AXES.iter()
                .position(|x| *x == a)
                .map(|i| i as u8)
                .ok_or_else(|| format!("unknown axis '{a}'"))
        };
        if let Some(rest) = cat.strip_prefix("operandi_wrap_") {
            let (spherical, axis) = if let Some(a) = rest.strip_prefix("spherical_") {
                (true, a)
            } else if let Some(a) = rest.strip_prefix("cylindrical_") {
                (false, a)
            } else {
                return Err(format!("unknown wrap '{rest}'"));
            };
            return Ok(Expr::Wrap {
                of: input(self, "in")?,
                axis: axis_of(axis)?,
                spherical,
                centre: float(get("centre")?)?,
                repeat: fixed::<2>(get("repeat")?)?,
            });
        }
        if let Some(axis) = cat.strip_prefix("operandi_probe_") {
            return Ok(Expr::Probe {
                of: input(self, "in")?,
                axis: axis_of(axis)?,
                repeat: fixed::<2>(get("repeat")?)?,
            });
        }
        if let Some(face) = cat.strip_prefix("operandi_box_") {
            return Ok(Expr::FaceWeight {
                normal: input(self, "in")?,
                face: BOX_FACES
                    .iter()
                    .position(|f| *f == face)
                    .ok_or_else(|| format!("unknown box face '{face}'"))?
                    as u8,
                kind: FaceKind::Signed,
            });
        }
        if let Some(axis) = cat.strip_prefix("operandi_cubic_") {
            return Ok(Expr::FaceWeight {
                normal: input(self, "in")?,
                face: axis_of(axis)?,
                kind: FaceKind::Dominant,
            });
        }
        if let Some(axis) = cat.strip_prefix("operandi_triplanar_") {
            return Ok(Expr::FaceWeight {
                normal: input(self, "in")?,
                face: axis_of(axis)?,
                kind: FaceKind::Power(float(get("power")?)?),
            });
        }
        if let Some(axis) = cat.strip_prefix("operandi_planar_") {
            return Ok(Expr::Planar {
                of: input(self, "in")?,
                axis: AXES
                    .iter()
                    .position(|a| *a == axis)
                    .ok_or_else(|| format!("unknown axis '{axis}'"))? as u8,
                centre: float(get("centre")?)?,
                repeat: fixed::<2>(get("repeat")?)?,
                mirror: u8::from(boolean(get("mirror_u")?)?)
                    | u8::from(boolean(get("mirror_v")?)?) << 1,
            });
        }
        if let Some(shape) = cat.strip_prefix("operandi_falloff_") {
            return Ok(Expr::Falloff {
                of: input(self, "in")?,
                kind: value_of(&FALLOFFS, shape, "falloff")?,
                amount: fixed::<3>(get("amount")?)?,
            });
        }
        if let Some(modes) = cat.strip_prefix("operandi_tile_") {
            let (h, v) = TILINGS
                .iter()
                .find_map(|(t, n)| Some((*t, modes.strip_prefix(n)?.strip_prefix('_')?)))
                .ok_or_else(|| format!("unknown tiling '{modes}'"))?;
            return Ok(Expr::Tile(
                input(self, "in")?,
                [h, value_of(&TILINGS, v, "tiling")?],
            ));
        }
        let rows = |n: usize| -> Result<Vec<f64>> {
            let mut out = Vec::new();
            for i in 0..3 {
                let r = numbers(get(&format!("row{i}"))?)?;
                if r.len() != n {
                    return Err(format!("row{i} needs {n} numbers"));
                }
                out.extend(r);
            }
            Ok(out)
        };
        Ok(match cat {
            "operandi_constant" => Expr::Const(fixed::<3>(get("value")?)?),
            "operandi_image" | "operandi_image_alpha" => {
                let l = Box::new(ImageLookup {
                    image: text_of(get("image")?)?
                        .parse()
                        .map_err(|e| format!("image: {e}"))?,
                    filtering: value_of(&FILTERS, &text_of(get("filtertype")?)?, "filtertype")?,
                    vector: self.expr(get("texcoord")?)?,
                    extension: value_of(&ADDRESS, &text_of(get("addressmode")?)?, "addressmode")?,
                });
                if cat == "operandi_image_alpha" {
                    Expr::ImageAlpha(l)
                } else {
                    Expr::ImageColor(l)
                }
            }
            "operandi_locator" => {
                Expr::Locator(Box::new(rows(4)?.try_into().expect("three rows of four")))
            }
            "operandi_planar" => Expr::Planar {
                of: input(self, "in")?,
                axis: integer(get("axis")?)?,
                centre: float(get("centre")?)?,
                repeat: fixed::<2>(get("repeat")?)?,
                mirror: integer(get("mirror")?)?,
            },
            "operandi_wrap" => Expr::Wrap {
                of: input(self, "in")?,
                axis: integer(get("axis")?)?,
                spherical: boolean(get("spherical")?)?,
                centre: float(get("centre")?)?,
                repeat: fixed::<2>(get("repeat")?)?,
            },
            "operandi_probe" => Expr::Probe {
                of: input(self, "in")?,
                axis: integer(get("axis")?)?,
                repeat: fixed::<2>(get("repeat")?)?,
            },
            "operandi_normal" => {
                Expr::Normal(Box::new(rows(3)?.try_into().expect("three rows of three")))
            }
            "operandi_face_weight" => Expr::FaceWeight {
                normal: input(self, "in")?,
                face: integer(get("face")?)?,
                kind: match text_of(get("kind")?)?.as_str() {
                    "signed" => FaceKind::Signed,
                    "dominant" => FaceKind::Dominant,
                    "power" => FaceKind::Power(float(get("power")?)?),
                    other => return Err(format!("unknown face kind '{other}'")),
                },
            },
            "operandi_facing" => Expr::Facing(input(self, "in")?, integer(get("axis")?)?),
            "operandi_texcoord" | "operandi_texcoord_map" => Expr::MeshUv {
                map: if cat == "operandi_texcoord_map" {
                    Some(text_of(get("map")?)?)
                } else {
                    None
                },
                repeat: fixed::<2>(get("repeat")?)?,
                rotation: float(get("rotation")?)?,
            },
            "operandi_inside" => Expr::Inside(
                input(self, "in")?,
                [boolean(get("horizontal")?)?, boolean(get("vertical")?)?],
            ),
            "operandi_pattern" => Expr::Pattern(pattern(self)?),
            "operandi_cell_random" => Expr::CellRandom(pattern(self)?),
            "operandi_bias_gain" => Expr::BiasGain(
                input(self, "in")?,
                float(get("bias")?)?,
                float(get("gain")?)?,
            ),
            "operandi_hsv_vary" => Expr::HsvVary {
                color: input(self, "in")?,
                random: input(self, "random")?,
                amounts: fixed::<3>(get("amounts")?)?,
            },
            "operandi_luma" => Expr::Luma(input(self, "in")?),
            "operandi_channel" => Expr::Channel(input(self, "in")?, integer(get("channel")?)?),
            "operandi_gamma" => Expr::Gamma(input(self, "in")?, float(get("power")?)?),
            "operandi_geometry" => {
                Expr::Geometry(value_of(&GEOMETRY, &text_of(get("kind")?)?, "geometry")?)
            }
            "operandi_distance" => Expr::DistanceTo(fixed::<3>(get("position")?)?),
            "operandi_locator_incidence" => Expr::LocatorIncidence(fixed::<3>(get("position")?)?),
            "operandi_hsv" => Expr::Hsv {
                of: input(self, "in")?,
                hue_scale: float(get("hue_scale")?)?,
                hue_shift: float(get("hue_shift")?)?,
                saturation_scale: float(get("saturation_scale")?)?,
            },
            "operandi_occlusion" => Expr::Occlusion {
                rays: integer(get("rays")?)?,
                distance: float(get("distance")?)?,
                same_surface: boolean(get("same_surface")?)?,
                inside: boolean(get("inside")?)?,
                toward: value_of(&TOWARD, &text_of(get("toward")?)?, "occlusion direction")?,
            },
            "operandi_attribute" | "operandi_attribute_alpha" => Expr::Attribute {
                name: text_of(get("attribute")?)?,
                alpha: cat == "operandi_attribute_alpha",
            },
            "operandi_ramp" => {
                let flat = numbers(get("stops")?)?;
                if flat.len() % 4 != 0 {
                    return Err("ramp stops aren't RGBA".into());
                }
                Expr::Ramp {
                    input: input(self, "in")?,
                    lo: float(get("lo")?)?,
                    hi: float(get("hi")?)?,
                    stops: Arc::new(flat.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect()),
                    alpha: boolean(get("alpha")?)?,
                }
            }
            other => return Err(format!("not a shader tree node: {other}")),
        })
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
