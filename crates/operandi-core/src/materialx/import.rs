//! MaterialX import (MX-02..04): a material's surface shader becomes
//! OpenPBR inputs, each a constant or a short image chain, and those
//! become a Material preset (a material group with a Material and Image
//! Map layers), the same shape a texture set gives (CD-25).
//!
//! Simple (MX-03, extended so exports read back, MX-06): a constant, or
//! an `image`/`tiledimage`/`UsdUVTexture` read through `texcoord`/
//! `place2d`/`UsdTransform2d`, optionally `multiply` by a constant,
//! `normalmap`, one channel (`extract`, `separate3/4`, a texture output),
//! `invert`, `dot`/`convert` pass-throughs; also `multiply` by a second
//! image (a Multiply layer) and `mix` of an image over the rest by a
//! constant (a layer at that opacity), which is how layers export.
//! Anything else becomes a MaterialX graph layer (`materialx::graph`);
//! only nodes the Cycles translation can't render still refuse the whole
//! material (MX-04): "needs Node Material (not yet supported): <node>",
//! and the scene is untouched.
use super::colour;
use super::doc::{Document, Element, Port, numbers};
use super::stdlib;
use crate::openpbr::{self, Kind, Param, Value, Values};
use crate::preset::{Preset, PresetHeader, PresetKind};
use crate::shader_tree::{
    BlendMode, GroupMask, ImageSettings, Layer, NewLayer, Node, NodeKind, ShaderTree, Swizzle,
    TextureEffect, TextureLayer, TextureSource,
};
use crate::texture_locator::TextureItem;
use crate::{Result, Scene};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The refusal prefix (MX-04).
pub const NEEDS_NODES: &str = "needs Node Material (not yet supported)";

fn refuse(node: &Element) -> String {
    format!("{NEEDS_NODES}: {} ({})", node.name, node.category)
}

/// A picture read by an image chain.
#[derive(Clone, Debug, PartialEq)]
pub struct Img {
    pub file: PathBuf,
    /// The file's colour space (`None` = the document says nothing).
    pub colorspace: Option<String>,
    /// One channel (0..3 = R, G, B, A), `None` = the whole picture.
    pub channel: Option<u8>,
    /// Multiplied by a constant.
    pub scale: f64,
    pub invert: bool,
    /// Channels turned 1 − v (a DirectX normal's green), written as
    /// multiply by ±1 then add 0 or 1; `negate` holds a multiply waiting
    /// for its add.
    pub flip: [bool; 3],
    pub negate: [bool; 3],
    /// Read through `normalmap` (its scale).
    pub normal: Option<f64>,
    /// UV repeat counts and rotation (degrees) from place2d or tiling.
    pub repeat: [f64; 2],
    pub rotate: f64,
    /// The image node, for layer names and notes.
    pub node: String,
}

/// An input's value: a constant, or image layers (bottom first) over an
/// optional constant.
#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Const(Vec<f64>),
    Layers {
        under: Option<Vec<f64>>,
        layers: Vec<(Img, BlendMode, f64)>,
    },
    /// Texture coordinates (texcoord/place2d), only an image reads them.
    Uv {
        repeat: [f64; 2],
        rotate: f64,
    },
}
impl Src {
    fn single(&self) -> Option<&Img> {
        match self {
            Src::Layers {
                under: None,
                layers,
            } if layers.len() == 1 => {
                let (img, blend, op) = &layers[0];
                (*blend == BlendMode::Normal && *op == 1.).then_some(img)
            }
            _ => None,
        }
    }
    fn single_mut(&mut self) -> Option<&mut Img> {
        match self {
            Src::Layers {
                under: None,
                layers,
            } if layers.len() == 1 => {
                let (img, blend, op) = &mut layers[0];
                (*blend == BlendMode::Normal && *op == 1.).then_some(img)
            }
            _ => None,
        }
    }
}

/// Where names resolve: the document's top level, or a nodegraph's nodes
/// with its interface inputs.
struct Scope<'a> {
    doc: &'a Document,
    graph: Option<&'a Element>,
    interface: Option<&'a BTreeMap<String, Src>>,
    /// The folder relative file names start from.
    folder: &'a Path,
    notes: &'a mut Vec<String>,
    depth: u32,
}

impl Scope<'_> {
    fn node(&self, name: &str) -> Option<&Element> {
        match self.graph {
            Some(g) => g.child(name),
            None => self.doc.element(name),
        }
    }

    /// A port's value, `None` when it has none (use the default).
    fn port(&mut self, port: &Port) -> Result<Option<Src>> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("MaterialX graph too deep (a loop?)".into());
        }
        let out = self.port_inner(port);
        self.depth -= 1;
        out
    }

    fn port_inner(&mut self, port: &Port) -> Result<Option<Src>> {
        if let Some(i) = &port.interfacename {
            return Ok(self.interface.and_then(|m| m.get(i)).cloned());
        }
        if let Some(g) = &port.nodegraph {
            let graph = self
                .doc
                .element(g)
                .ok_or_else(|| format!("MaterialX nodegraph {g} not found"))?;
            let out = match &port.output {
                Some(o) => graph.outputs.iter().find(|p| &p.name == o),
                None => graph.outputs.first(),
            }
            .ok_or_else(|| format!("MaterialX nodegraph {g} has no such output"))?
            .clone();
            let mut inner = Scope {
                doc: self.doc,
                graph: Some(graph),
                interface: None,
                folder: self.folder,
                notes: self.notes,
                depth: self.depth,
            };
            return inner.port(&out);
        }
        if let Some(n) = &port.nodename {
            let node = self
                .node(n)
                .ok_or_else(|| format!("MaterialX node {n} not found"))?
                .clone();
            return self.node_value(&node, port.output.as_deref()).map(Some);
        }
        Ok(port.value.as_deref().map(|v| self.constant(port, v)))
    }

    fn constant(&mut self, port: &Port, v: &str) -> Src {
        let mut n = numbers(v).unwrap_or_default();
        if matches!(port.ty.as_str(), "color3" | "color4") && n.len() >= 3 {
            let space = port.colorspace.as_deref().unwrap_or("lin_rec709");
            match colour::to_lin_rec709([n[0], n[1], n[2]], space) {
                Some(c) => n[..3].copy_from_slice(&c),
                None => self.notes.push(format!(
                    "colour space {space} unknown: {} used as is",
                    port.name
                )),
            }
        }
        Src::Const(n)
    }

    fn input(&mut self, node: &Element, name: &str) -> Result<Option<Src>> {
        match node.input(name) {
            Some(p) => {
                let p = p.clone();
                self.port(&p)
            }
            None => Ok(None),
        }
    }

    fn need(&mut self, node: &Element, name: &str, default: &[f64]) -> Result<Src> {
        Ok(self
            .input(node, name)?
            .unwrap_or_else(|| Src::Const(default.to_vec())))
    }

    fn consts(&mut self, node: &Element, names: &[(&str, &[f64])]) -> Result<Vec<Vec<f64>>> {
        let mut out = Vec::new();
        for (name, default) in names {
            match self.need(node, name, default)? {
                Src::Const(v) => out.push(v),
                _ => return Err(refuse(node)),
            }
        }
        Ok(out)
    }

    fn node_value(&mut self, node: &Element, output: Option<&str>) -> Result<Src> {
        let size = type_size(&node.ty);
        match node.category.as_str() {
            "constant" => self.need(node, "value", &[0.]),
            "dot" | "convert" => {
                let v = self.need(node, "in", &[0.])?;
                Ok(match v {
                    Src::Const(c) => Src::Const(fit(c, size)),
                    other => other,
                })
            }
            "image" | "tiledimage" | "UsdUVTexture" | "gltf_image" | "gltf_colorimage" => {
                self.image(node, output)
            }
            "texcoord" => {
                let index = self.consts(node, &[("index", &[0.])])?[0][0];
                if index != 0. {
                    return Err(refuse(node));
                }
                Ok(Src::Uv {
                    repeat: [1., 1.],
                    rotate: 0.,
                })
            }
            "UsdPrimvarReader" => Ok(Src::Uv {
                repeat: [1., 1.],
                rotate: 0.,
            }),
            "place2d" | "UsdTransform2d" => self.place(node),
            "normalmap" => {
                let mut v = self.need(node, "in", &[0.5, 0.5, 1.])?;
                let scale = self.consts(node, &[("scale", &[1.])])?[0][0];
                let space = node
                    .input("space")
                    .and_then(|p| p.value.clone())
                    .unwrap_or_else(|| "tangent".into());
                match v.single_mut() {
                    Some(img) if space == "tangent" => {
                        img.normal = Some(scale);
                        Ok(v)
                    }
                    _ => Err(refuse(node)),
                }
            }
            "multiply" => {
                let a = self.need(node, "in1", &[1.])?;
                let b = self.need(node, "in2", &[1.])?;
                multiply(a, b, size).ok_or_else(|| refuse(node))
            }
            "mix" => {
                let fg = self.need(node, "fg", &[0.])?;
                let bg = self.need(node, "bg", &[0.])?;
                let k = self.need(node, "mix", &[0.])?;
                mix(fg, bg, k).ok_or_else(|| refuse(node))
            }
            "invert" => {
                let mut v = self.need(node, "in", &[0.])?;
                let amount = self.consts(node, &[("amount", &[1.])])?[0][0];
                if let Src::Const(c) = &v {
                    return Ok(Src::Const(c.iter().map(|x| amount - x).collect()));
                }
                match v.single_mut() {
                    Some(img) if amount == 1. => {
                        img.invert = !img.invert;
                        Ok(v)
                    }
                    _ => Err(refuse(node)),
                }
            }
            "extract" => {
                let mut v = self.need(node, "in", &[0.])?;
                let index = self.consts(node, &[("index", &[0.])])?[0][0] as usize;
                channel(&mut v, index).ok_or_else(|| refuse(node))
            }
            "separate2" | "separate3" | "separate4" => {
                let mut v = self.need(node, "in", &[0.])?;
                let index = match output.unwrap_or("outr") {
                    "outr" | "outx" => 0,
                    "outg" | "outy" => 1,
                    "outb" | "outz" => 2,
                    "outa" | "outw" => 3,
                    _ => return Err(refuse(node)),
                };
                channel(&mut v, index).ok_or_else(|| refuse(node))
            }
            "ifgreater" | "ifgreatereq" | "ifequal" => {
                // Both branches the same: the test doesn't matter (a
                // textured metalness with the default specular).
                let a = self.need(node, "in1", &[0.])?;
                let b = self.need(node, "in2", &[0.])?;
                if let (Src::Const(x), Src::Const(y)) = (&a, &b)
                    && fit(x.clone(), size) == fit(y.clone(), size)
                {
                    return Ok(Src::Const(fit(x.clone(), size)));
                }
                let t = self.consts(node, &[("value1", &[1.]), ("value2", &[0.])])?;
                let (p, q) = (t[0][0], t[1][0]);
                let yes = match node.category.as_str() {
                    "ifgreater" => p > q,
                    "ifgreatereq" => p >= q,
                    _ => p == q,
                };
                Ok(match if yes { a } else { b } {
                    Src::Const(c) => Src::Const(fit(c, size)),
                    other => other,
                })
            }
            "add" => {
                let a = self.need(node, "in1", &[0.])?;
                let b = self.need(node, "in2", &[0.])?;
                let (mut img, c) = match (a, b) {
                    (Src::Const(_), Src::Const(_)) => return self.arithmetic(node, size),
                    (Src::Const(c), img) | (img, Src::Const(c)) => (img, c),
                    _ => return Err(refuse(node)),
                };
                let c = fit(c, 3);
                let i = img.single_mut().ok_or_else(|| refuse(node))?;
                // 1 − v on the negated channels finishes a flip.
                let fits = (0..3).all(|k| c[k] == if i.negate[k] { 1. } else { 0. });
                if !i.negate.iter().any(|n| *n) || !fits {
                    return Err(refuse(node));
                }
                for k in 0..3 {
                    i.flip[k] ^= i.negate[k];
                }
                i.negate = [false; 3];
                Ok(img)
            }
            // Arithmetic the translation graphs use: constants only.
            _ => self.arithmetic(node, size),
        }
    }

    fn arithmetic(&mut self, node: &Element, size: usize) -> Result<Src> {
        let c = |s: &mut Self, names: &[(&str, &[f64])]| s.consts(node, names);
        let v: Vec<f64> = match node.category.as_str() {
            "add" | "subtract" | "divide" | "power" | "max" | "min" | "modulo" => {
                let i = c(self, &[("in1", &[0.]), ("in2", &[0.])])?;
                let (a, b) = (fit(i[0].clone(), size), fit(i[1].clone(), size));
                a.iter()
                    .zip(&b)
                    .map(|(x, y)| match node.category.as_str() {
                        "add" => x + y,
                        "subtract" => x - y,
                        "divide" => {
                            if *y == 0. {
                                0.
                            } else {
                                x / y
                            }
                        }
                        "power" => x.powf(*y),
                        "max" => x.max(*y),
                        "min" => x.min(*y),
                        _ => x % y,
                    })
                    .collect()
            }
            "clamp" => {
                let i = c(self, &[("in", &[0.]), ("low", &[0.]), ("high", &[1.])])?;
                let (lo, hi) = (fit(i[1].clone(), size), fit(i[2].clone(), size));
                fit(i[0].clone(), size)
                    .iter()
                    .enumerate()
                    .map(|(k, x)| x.clamp(lo[k], hi[k].max(lo[k])))
                    .collect()
            }
            "dotproduct" => {
                let i = c(self, &[("in1", &[0.]), ("in2", &[0.])])?;
                let n = i[0].len().max(i[1].len());
                let (a, b) = (fit(i[0].clone(), n), fit(i[1].clone(), n));
                vec![a.iter().zip(&b).map(|(x, y)| x * y).sum()]
            }
            "magnitude" => {
                let i = c(self, &[("in", &[0.])])?;
                vec![i[0].iter().map(|x| x * x).sum::<f64>().sqrt()]
            }
            "combine2" | "combine3" | "combine4" => {
                let names: &[(&str, &[f64])] = &[
                    ("in1", &[0.]),
                    ("in2", &[0.]),
                    ("in3", &[0.]),
                    ("in4", &[0.]),
                ];
                let i = c(self, &names[..size.max(2)])?;
                i.iter().map(|v| v[0]).collect()
            }
            _ => return Err(refuse(node)),
        };
        Ok(Src::Const(fit(v, size)))
    }

    fn place(&mut self, node: &Element) -> Result<Src> {
        let base = match self.input(node, "texcoord")? {
            Some(s) => Some(s),
            None => self.input(node, "in")?,
        };
        let (mut repeat, mut rotate) = match base {
            Some(Src::Uv { repeat, rotate }) => (repeat, rotate),
            None => ([1., 1.], 0.),
            Some(_) => return Err(refuse(node)),
        };
        let usd = node.category == "UsdTransform2d";
        let i = self.consts(
            node,
            &[
                ("scale", &[1., 1.]),
                (if usd { "rotation" } else { "rotate" }, &[0.]),
                (if usd { "translation" } else { "offset" }, &[0., 0.]),
            ],
        )?;
        let s = fit(i[0].clone(), 2);
        for k in 0..2 {
            // place2d divides the coordinates by its scale; UsdTransform2d
            // multiplies them.
            repeat[k] *= if usd {
                s[k]
            } else if s[k] != 0. {
                1. / s[k]
            } else {
                1.
            };
        }
        rotate += i[1][0];
        if fit(i[2].clone(), 2).iter().any(|v| *v != 0.) {
            self.notes
                .push(format!("{}: UV offset left out", node.name));
        }
        Ok(Src::Uv { repeat, rotate })
    }

    fn image(&mut self, node: &Element, output: Option<&str>) -> Result<Src> {
        let file = node
            .input("file")
            .ok_or_else(|| format!("MaterialX image {} has no file", node.name))?;
        let value = file.value.clone().unwrap_or_default();
        if value.is_empty() {
            return Err(format!("MaterialX image {} has no file", node.name));
        }
        if value.contains("<UDIM>") || value.contains("<UVTILE>") {
            return Err(refuse(node));
        }
        let path = self.folder.join(format!("{}{value}", file.fileprefix));
        let st = node.input("st").or(node.input("texcoord")).cloned();
        let uv = match st {
            Some(p) => self.port(&p)?,
            None => None,
        };
        let (mut repeat, rotate) = match uv {
            Some(Src::Uv { repeat, rotate }) => (repeat, rotate),
            None => ([1., 1.], 0.),
            Some(_) => return Err(refuse(node)),
        };
        if node.category == "tiledimage" {
            let i = self.consts(node, &[("uvtiling", &[1., 1.]), ("uvoffset", &[0., 0.])])?;
            let t = fit(i[0].clone(), 2);
            repeat = [repeat[0] * t[0], repeat[1] * t[1]];
            if fit(i[1].clone(), 2).iter().any(|v| *v != 0.) {
                self.notes
                    .push(format!("{}: UV offset left out", node.name));
            }
        }
        let mut img = Img {
            file: path,
            colorspace: file.colorspace.clone(),
            channel: None,
            scale: 1.,
            invert: false,
            flip: [false; 3],
            negate: [false; 3],
            normal: None,
            repeat,
            rotate,
            node: node.name.clone(),
        };
        if node.category == "UsdUVTexture" {
            img.channel = match output.unwrap_or("rgb") {
                "r" => Some(0),
                "g" => Some(1),
                "b" => Some(2),
                "a" => Some(3),
                _ => None,
            };
            let i = self.consts(
                node,
                &[("scale", &[1., 1., 1., 1.]), ("bias", &[0., 0., 0., 0.])],
            )?;
            let (scale, bias) = (fit(i[0].clone(), 3), fit(i[1].clone(), 3));
            // A normal map written as 2x - 1 (USD's way) is a normal
            // map; normalmap then reads it as it is.
            if bias.iter().all(|b| (*b + 1.).abs() < 1e-6)
                && scale.iter().all(|s| (*s - 2.).abs() < 1e-6)
            {
                img.normal = Some(1.);
            } else if bias.iter().any(|b| *b != 0.) || scale.iter().any(|s| *s != scale[0]) {
                return Err(refuse(node));
            } else {
                img.scale = scale[0];
            }
        }
        Ok(Src::Layers {
            under: None,
            layers: vec![(img, BlendMode::Normal, 1.)],
        })
    }
}

fn type_size(ty: &str) -> usize {
    match ty {
        "color3" | "vector3" => 3,
        "color4" | "vector4" => 4,
        "vector2" => 2,
        _ => 1,
    }
}

/// `v` as `n` components: broadcast a scalar, cut longer ones.
fn fit(v: Vec<f64>, n: usize) -> Vec<f64> {
    match v.len() {
        0 => vec![0.; n],
        1 => vec![v[0]; n],
        _ => (0..n).map(|i| v.get(i).copied().unwrap_or(0.)).collect(),
    }
}

fn multiply(a: Src, b: Src, size: usize) -> Option<Src> {
    match (a, b) {
        (Src::Const(x), Src::Const(y)) => {
            let (x, y) = (fit(x, size), fit(y, size));
            Some(Src::Const(x.iter().zip(&y).map(|(p, q)| p * q).collect()))
        }
        (Src::Const(k), mut img) | (mut img, Src::Const(k)) => {
            // A uniform constant scales one picture; ±1 per channel starts
            // a channel flip (see `Img::flip`).
            let uniform = k.iter().all(|v| (*v - k[0]).abs() < 1e-9);
            let i = img.single_mut()?;
            if i.normal.is_some() {
                return None;
            }
            if uniform {
                i.scale *= k.first().copied().unwrap_or(1.);
            } else if k.len() >= 3 && k.iter().take(3).all(|v| (v.abs() - 1.).abs() < 1e-9) {
                for (n, v) in i.negate.iter_mut().zip(&k) {
                    *n ^= *v < 0.;
                }
            } else {
                return None;
            }
            Some(img)
        }
        (Src::Layers { under, mut layers }, over) => {
            // A second picture multiplied over: a Multiply layer.
            let o = over.single()?.clone();
            layers.push((o, BlendMode::Multiply, 1.));
            Some(Src::Layers { under, layers })
        }
        _ => None,
    }
}

fn mix(fg: Src, bg: Src, k: Src) -> Option<Src> {
    let Src::Const(k) = k else { return None };
    let k = k.first().copied().unwrap_or(0.).clamp(0., 1.);
    match (fg, bg) {
        (Src::Const(a), Src::Const(b)) => {
            let n = a.len().max(b.len());
            let (a, b) = (fit(a, n), fit(b, n));
            Some(Src::Const(
                a.iter().zip(&b).map(|(x, y)| y + (x - y) * k).collect(),
            ))
        }
        (fg, Src::Const(b)) => {
            let img = fg.single()?.clone();
            Some(Src::Layers {
                under: Some(b),
                layers: vec![(img, BlendMode::Normal, k)],
            })
        }
        (fg, Src::Layers { under, mut layers }) => {
            let img = fg.single()?.clone();
            layers.push((img, BlendMode::Normal, k));
            Some(Src::Layers { under, layers })
        }
        _ => None,
    }
}

fn channel(v: &mut Src, index: usize) -> Option<Src> {
    match v {
        Src::Const(c) => Some(Src::Const(vec![c.get(index).copied().unwrap_or(0.)])),
        _ => {
            let img = v.single_mut()?;
            if img.channel.is_some() || index > 3 {
                return None;
            }
            img.channel = Some(index as u8);
            Some(v.clone())
        }
    }
}

/// A shader's inputs as OpenPBR: constants and image-driven inputs.
#[derive(Clone, Debug, Default)]
pub struct Surface {
    pub values: Values,
    pub images: Vec<(Param, Src)>,
    /// Ambient occlusion (glTF, USD): multiplied over the base colour.
    pub occlusion: Option<Src>,
    /// Displacement height and its distance in metres.
    pub displacement: Option<(Src, f64)>,
    pub notes: Vec<String>,
}

/// One material of a document: its name and surface shader.
pub fn materials(doc: &Document) -> Vec<&Element> {
    doc.all("surfacematerial").collect()
}

/// The OpenPBR surface of material `material` (the first when `None`).
pub fn surface(doc: &Document, material: Option<&str>, folder: &Path) -> Result<(String, Surface)> {
    let mats = materials(doc);
    let mat = match material {
        Some(name) => mats.iter().find(|m| m.name == name).copied(),
        None => mats.first().copied(),
    };
    // A file with only a shader node is read as that shader.
    let (name, shader, displacement) = match mat {
        Some(m) => {
            let shader = m
                .input("surfaceshader")
                .and_then(|p| p.nodename.as_deref())
                .and_then(|n| doc.element(n))
                .ok_or_else(|| format!("MaterialX material {} has no surface shader", m.name))?;
            let disp = m
                .input("displacementshader")
                .and_then(|p| p.nodename.as_deref())
                .and_then(|n| doc.element(n));
            (m.name.clone(), shader, disp)
        }
        None => {
            let shader = doc
                .elements
                .iter()
                .find(|e| e.ty == "surfaceshader" && e.category != "nodedef")
                .ok_or("The MaterialX file has no material")?;
            (shader.name.clone(), shader, None)
        }
    };
    let mut out = Surface::default();
    let mut notes = Vec::new();
    {
        let mut scope = Scope {
            doc,
            graph: None,
            interface: None,
            folder,
            notes: &mut notes,
            depth: 0,
        };
        match shader.category.as_str() {
            "open_pbr_surface" => openpbr_inputs(&mut scope, shader, &mut out)?,
            "standard_surface" => standard_inputs(&mut scope, shader, &mut out)?,
            "gltf_pbr" => gltf_inputs(&mut scope, shader, &mut out)?,
            "UsdPreviewSurface" => usd_inputs(&mut scope, shader, &mut out)?,
            other => return Err(format!("{NEEDS_NODES}: {} ({other})", shader.name)),
        }
        if let Some(d) = displacement {
            if d.category != "displacement" {
                return Err(refuse(d));
            }
            let height = scope.need(d, "displacement", &[0.])?;
            let scale = scope.consts(d, &[("scale", &[1.])])?[0][0];
            if !matches!(height, Src::Const(_)) {
                // MaterialX leaves the unit to the scene; OpenPBR's default
                // length is the centimetre (provisional, OPM-Q13).
                out.displacement = Some((height, scale * openpbr::DEFAULT_LENGTH_IN_METRES));
            }
        }
    }
    out.notes.extend(notes);
    Ok((name, out))
}

/// The value of an OpenPBR input from a resolved source.
fn value_of(p: Param, src: &[f64]) -> Value {
    match p.kind() {
        Kind::Float => Value::Float(src.first().copied().unwrap_or(0.)),
        Kind::Color3 => {
            let c = fit(src.to_vec(), 3);
            Value::Color([c[0], c[1], c[2]])
        }
        Kind::Boolean => Value::Bool(src.first().is_some_and(|v| *v != 0.)),
        Kind::Vector3 => Value::Geometry,
    }
}

pub(super) fn metres_per(unit: &str) -> Option<f64> {
    Some(match unit {
        "nanometer" => 1e-9,
        "micron" | "micrometer" => 1e-6,
        "millimeter" => 1e-3,
        "centimeter" => 1e-2,
        "inch" => 0.0254,
        "foot" => 0.3048,
        "yard" => 0.9144,
        "meter" => 1.,
        "kilometer" => 1e3,
        "mile" => 1609.344,
        _ => return None,
    })
}

fn set(out: &mut Surface, p: Param, src: Src) {
    match src {
        Src::Const(v) => out.values.set(p, value_of(p, &v)),
        Src::Uv { .. } => {}
        layers => out.images.push((p, layers)),
    }
}

fn openpbr_inputs(scope: &mut Scope, shader: &Element, out: &mut Surface) -> Result<()> {
    for port in &shader.inputs {
        let Some(p) = Param::from_name(&port.name) else {
            scope
                .notes
                .push(format!("{}: not an OpenPBR 1.1 input, left out", port.name));
            continue;
        };
        let Some(mut src) = scope.port(port)? else {
            continue;
        };
        if p.is_length()
            && let Src::Const(v) = &mut src
        {
            let per = port
                .unit
                .as_deref()
                .map(|u| metres_per(u).ok_or_else(|| format!("{}: unknown unit {u}", port.name)))
                .transpose()?
                .unwrap_or(openpbr::DEFAULT_LENGTH_IN_METRES);
            for x in v.iter_mut() {
                *x *= per;
            }
        }
        set(out, p, src);
    }
    Ok(())
}

/// standard_surface through MaterialX's published translation graph
/// (MX-02): constants are worked out; a picture passes only where the
/// graph passes its input through unchanged.
fn standard_inputs(scope: &mut Scope, shader: &Element, out: &mut Surface) -> Result<()> {
    let defaults = stdlib::defaults("standard_surface");
    let mut interface: BTreeMap<String, Src> = BTreeMap::new();
    for (name, port) in &defaults {
        if let Some(v) = &port.value {
            let src = scope.constant(port, v);
            interface.insert(name.clone(), src);
        }
    }
    for port in &shader.inputs {
        if !defaults.contains_key(&port.name) {
            scope.notes.push(format!(
                "{}: not a standard_surface input, left out",
                port.name
            ));
            continue;
        }
        if let Some(src) = scope.port(port)? {
            interface.insert(port.name.clone(), src);
        }
    }
    // Normals and the opacity colour aren't in the translation's maths.
    let normal = interface.remove("normal");
    let coat_normal = interface.remove("coat_normal");
    let graph = stdlib::standard_to_openpbr();
    let mut inner = Scope {
        doc: scope.doc,
        graph: Some(graph),
        interface: Some(&interface),
        folder: scope.folder,
        notes: scope.notes,
        depth: 0,
    };
    for o in &graph.outputs {
        let Some(name) = o.name.strip_suffix("_out") else {
            continue;
        };
        let Some(p) = Param::from_name(name) else {
            continue;
        };
        let src = match inner.port(o) {
            Ok(Some(s)) => s,
            Ok(None) => continue,
            Err(e) => {
                return Err(if e.starts_with(NEEDS_NODES) {
                    format!("{e} (standard_surface {name})")
                } else {
                    e
                });
            }
        };
        let src = match (p.is_length(), src) {
            // standard_surface lengths have no unit: scene units, which
            // Operandi reads as centimetres like OpenPBR (OPM-Q13).
            (true, Src::Const(v)) => Src::Const(
                v.iter()
                    .map(|x| x * openpbr::DEFAULT_LENGTH_IN_METRES)
                    .collect(),
            ),
            (_, s) => s,
        };
        set(out, p, src);
    }
    if let Some(n) = normal {
        set(out, Param::GeometryNormal, n);
    }
    if let Some(n) = coat_normal {
        set(out, Param::GeometryCoatNormal, n);
    }
    Ok(())
}

/// A shader's inputs, explicit or default, resolved.
fn resolved(scope: &mut Scope, shader: &Element) -> Result<BTreeMap<String, Src>> {
    let mut all = BTreeMap::new();
    for (name, port) in stdlib::defaults(&shader.category) {
        if let Some(v) = &port.value {
            all.insert(name, scope.constant(&port, v));
        }
    }
    for port in &shader.inputs {
        if let Some(src) = scope.port(port)? {
            all.insert(port.name.clone(), src);
        }
    }
    Ok(all)
}

fn konst(src: &Src) -> Option<f64> {
    match src {
        Src::Const(v) => v.first().copied(),
        _ => None,
    }
}

/// gltf_pbr to OpenPBR (MX-02; MaterialX has no such graph, OPM-Q05):
/// Operandi's own table from the public glTF 2.0 material model.
fn gltf_inputs(scope: &mut Scope, shader: &Element, out: &mut Surface) -> Result<()> {
    use Param::*;
    let mut i = resolved(scope, shader)?;
    let mut take = |name: &str| i.remove(name);
    for (name, p) in [
        ("base_color", BaseColor),
        ("metallic", BaseMetalness),
        ("roughness", SpecularRoughness),
        ("normal", GeometryNormal),
        ("tangent", GeometryTangent),
        ("transmission", TransmissionWeight),
        ("specular", SpecularWeight),
        ("specular_color", SpecularColor),
        ("ior", SpecularIor),
        ("sheen_roughness", FuzzRoughness),
        ("clearcoat", CoatWeight),
        ("clearcoat_roughness", CoatRoughness),
        ("clearcoat_normal", GeometryCoatNormal),
        ("iridescence", ThinFilmWeight),
        ("iridescence_ior", ThinFilmIor),
        ("anisotropy_strength", SpecularRoughnessAnisotropy),
        ("emissive", EmissionColor),
    ] {
        if let Some(src) = take(name) {
            set(out, p, src);
        }
    }
    // Emission: emissive × strength (both glTF and OpenPBR multiply).
    if let Some(s) = take("emissive_strength") {
        set(out, EmissionLuminance, s);
    }
    // glTF thickness 0 is a thin wall.
    let transmission = out.values.get(TransmissionWeight).float();
    let thickness = take("thickness");
    if transmission > 0. && thickness.as_ref().and_then(konst) == Some(0.) {
        out.values.set(GeometryThinWalled, Value::Bool(true));
    }
    // Volume attenuation: distance in metres, the colour reached there.
    if let (Some(d), Some(c)) = (take("attenuation_distance"), take("attenuation_color"))
        && let Some(d) = konst(&d)
        && d > 0.
    {
        out.values.set(TransmissionDepth, Value::Float(d));
        set(out, TransmissionColor, c);
    } else if transmission > 0. {
        // Without a volume glTF tints the transmission by the base colour.
        out.values.set(TransmissionColor, out.values.get(BaseColor));
    }
    if let Some(t) = take("iridescence_thickness") {
        match konst(&t) {
            // Nanometres to micrometres.
            Some(nm) => out.values.set(ThinFilmThickness, Value::Float(nm / 1000.)),
            None => return Err(format!("{NEEDS_NODES}: iridescence_thickness (texture)")),
        }
    }
    // Sheen colour: fuzz weight is its brightest channel.
    if let Some(sc) = take("sheen_color") {
        match sc {
            Src::Const(c) => {
                let c = fit(c, 3);
                let w = c.iter().copied().fold(0., f64::max);
                if w > 0. {
                    out.values.set(FuzzWeight, Value::Float(w.min(1.)));
                    out.values
                        .set(FuzzColor, Value::Color([c[0] / w, c[1] / w, c[2] / w]));
                }
            }
            other => {
                out.values.set(FuzzWeight, Value::Float(1.));
                set(out, FuzzColor, other);
            }
        }
    }
    // Alpha: OPAQUE ignores it.
    let mode = take("alpha_mode").as_ref().and_then(konst).unwrap_or(0.);
    let alpha = take("alpha");
    if mode != 0.
        && let Some(a) = alpha
    {
        set(out, GeometryOpacity, a);
        if mode == 1. {
            scope.notes.push("alpha MASK used as plain opacity".into());
        }
    }
    if let Some(o) = take("occlusion")
        && !matches!(o, Src::Const(_))
    {
        out.occlusion = Some(o);
    }
    if take("anisotropy_rotation")
        .as_ref()
        .and_then(konst)
        .is_some_and(|r| r != 0.)
    {
        scope.notes.push("anisotropy_rotation left out".into());
    }
    Ok(())
}

/// UsdPreviewSurface to OpenPBR (MX-02), Operandi's own table from the
/// public UsdPreviewSurface description.
fn usd_inputs(scope: &mut Scope, shader: &Element, out: &mut Surface) -> Result<()> {
    use Param::*;
    let mut i = resolved(scope, shader)?;
    let mut take = |name: &str| i.remove(name);
    for (name, p) in [
        ("diffuseColor", BaseColor),
        ("metallic", BaseMetalness),
        ("roughness", SpecularRoughness),
        ("clearcoat", CoatWeight),
        ("clearcoatRoughness", CoatRoughness),
        ("opacity", GeometryOpacity),
        ("ior", SpecularIor),
        ("normal", GeometryNormal),
        ("emissiveColor", EmissionColor),
    ] {
        if let Some(src) = take(name) {
            set(out, p, src);
        }
    }
    // A plain colour emits at one nit per unit (STRENGTH_PER_NIT).
    let emits = match out.values.get(EmissionColor) {
        Value::Color(c) => c.iter().any(|v| *v > 0.),
        _ => false,
    } || out.images.iter().any(|(p, _)| *p == EmissionColor);
    if emits {
        out.values.set(EmissionLuminance, Value::Float(1.));
    } else {
        // UsdPreviewSurface's emission defaults to black.
        out.values.set(EmissionColor, Value::Color([1.; 3]));
    }
    // OpenPBR's default coat roughness is 0, USD's 0.01: keep USD's.
    if take("useSpecularWorkflow").as_ref().and_then(konst) == Some(1.) {
        if let Some(s) = take("specularColor") {
            set(out, SpecularColor, s);
        }
        scope
            .notes
            .push("specular workflow: specularColor used as the specular tint".into());
    }
    if take("opacityThreshold")
        .as_ref()
        .and_then(konst)
        .is_some_and(|t| t > 0.)
    {
        scope
            .notes
            .push("opacityThreshold used as plain opacity".into());
    }
    if let Some(o) = take("occlusion")
        && !matches!(o, Src::Const(_))
    {
        out.occlusion = Some(o);
    }
    if let Some(d) = take("displacement")
        && !matches!(d, Src::Const(_))
    {
        out.displacement = Some((d, 1.));
    }
    Ok(())
}

/// The Texture Effect an image-driven OpenPBR input drives, with the
/// scale it needs, `None` for inputs Operandi can't texture.
fn effect(p: Param) -> Option<(TextureEffect, f64)> {
    use Param::*;
    use TextureEffect as E;
    Some(match p {
        BaseColor => (E::DiffuseColor, 1.),
        BaseMetalness => (E::Metallic, 1.),
        BaseDiffuseRoughness => (E::DiffuseRoughness, 1.),
        SpecularRoughness => (E::Roughness, 1.),
        SpecularColor => (E::SpecularColor, 1.),
        // Specular Amount is half the weight.
        SpecularWeight => (E::SpecularAmount, 0.5),
        CoatWeight => (E::ClearcoatAmount, 1.),
        TransmissionWeight => (E::TransparentAmount, 1.),
        TransmissionColor => (E::TransparentColor, 1.),
        SubsurfaceWeight => (E::SubsurfaceAmount, 1.),
        SubsurfaceRadiusScale => (E::SubsurfaceColor, 1.),
        FuzzWeight => (E::Sheen, 1.),
        EmissionColor => (E::LuminousColor, 1.),
        EmissionLuminance => (E::LuminousAmount, openpbr::STRENGTH_PER_NIT),
        GeometryOpacity => (E::Dissolve, 1.),
        GeometryNormal => (E::Normal, 1.),
        _ => return None,
    })
}

/// The order layers go in, top of the group first, like a texture set's.
const ORDER: &[Param] = &[
    Param::BaseColor,
    Param::BaseWeight,
    Param::SpecularRoughness,
    Param::BaseMetalness,
    Param::SpecularColor,
    Param::SpecularWeight,
    Param::BaseDiffuseRoughness,
    Param::GeometryNormal,
    Param::CoatWeight,
    Param::TransmissionWeight,
    Param::TransmissionColor,
    Param::SubsurfaceWeight,
    Param::SubsurfaceRadiusScale,
    Param::FuzzWeight,
    // The height goes here, then:
    Param::GeometryOpacity,
    Param::EmissionColor,
    Param::EmissionLuminance,
];

/// Image settings for a picture driving `effect`.
fn settings(
    img: &Img,
    effect: TextureEffect,
    extra: f64,
    notes: &mut Vec<String>,
) -> ImageSettings {
    let colour = crate::shader_eval::Ch::of_effect(effect).is_some_and(|c| c.is_color())
        && effect != TextureEffect::Normal;
    let mut s = ImageSettings {
        swizzle: match img.channel {
            Some(0) => Swizzle::Red,
            Some(1) => Swizzle::Green,
            Some(2) => Swizzle::Blue,
            Some(_) => Swizzle::Alpha,
            None => Swizzle::Rgba,
        },
        ..ImageSettings::default()
    };
    // Opacity drives Dissolve: white is solid, so the picture inverts.
    let invert = img.invert != (effect == TextureEffect::Dissolve);
    s.invert_red = invert != img.flip[0];
    s.invert_green = invert != img.flip[1];
    s.invert_blue = invert != img.flip[2];
    let scale = img.scale * extra;
    if (scale - 1.).abs() > 1e-9 {
        if scale.abs() > 10. {
            notes.push(format!("{}: scale {scale:.3} capped at 10", img.node));
        }
        s.brightness = scale.clamp(0., 10.);
    }
    if effect == TextureEffect::Normal && img.normal.is_some_and(|k| (k - 1.).abs() > 1e-9) {
        notes.push(format!("{}: normal map scale left out", img.node));
    }
    // Colour maps are read as sRGB, the rest as plain numbers; another
    // encoding is approximated with a gamma (MX-03).
    if let Some(space) = img.colorspace.as_deref() {
        let srgb = colour::is_srgb_encoded(space);
        let linear = matches!(space, "lin_rec709" | "lin_srgb" | "raw" | "none" | "");
        let ext = img
            .file
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let float_file = matches!(ext.as_str(), "exr" | "hdr");
        if colour && linear && !float_file {
            s.gamma = 2.2;
            notes.push(format!("{}: linear colour map approximated", img.node));
        } else if !colour && srgb && effect != TextureEffect::Normal {
            s.gamma = 1. / 2.2;
            notes.push(format!("{}: sRGB data map approximated", img.node));
        } else if !srgb && !linear {
            notes.push(format!("{}: colour space {space} read as is", img.node));
        }
    }
    s
}

/// A document's material as a Material preset, named after the
/// material: layers when it is simple (MX-03), else the whole MaterialX
/// graph in a Node Material layer (materials architecture slice 1,
/// `materialx::graph`), which renders in Cycles. Only a graph with nodes
/// that can't be translated is still refused whole (MX-04). `size_of`
/// gives a picture's pixel size; a picture it can't read is left out
/// with a note, as for texture sets.
pub fn preset(
    doc: &Document,
    material: Option<&str>,
    folder: &Path,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> Result<(Preset, Vec<String>)> {
    match layers_preset(doc, material, folder, size_of) {
        Err(e) if e.starts_with(NEEDS_NODES) => super::graph::preset(doc, material, folder),
        other => other,
    }
}

/// The material as layers (MX-03), or `NEEDS_NODES` when it isn't simple.
fn layers_preset(
    doc: &Document,
    material: Option<&str>,
    folder: &Path,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> Result<(Preset, Vec<String>)> {
    let (name, surface) = surface(doc, material, folder)?;
    let mut notes = surface.notes.clone();
    // Every picture must have an effect before anything is built (MX-04).
    for (p, src) in &surface.images {
        if effect(*p).is_none() && *p != Param::BaseWeight {
            return Err(format!("{NEEDS_NODES}: {} is textured", p.name()));
        }
        if let Src::Layers { layers, .. } = src
            && let Some((img, _, _)) = layers.iter().find(|(i, _, _)| i.negate.iter().any(|n| *n))
        {
            return Err(format!("{NEEDS_NODES}: {} (a negative picture)", img.node));
        }
    }
    let (mut material_channels, more) = openpbr::to_material(&surface.values);
    notes.extend(more);
    let mut scene = Scene {
        shader_tree: ShaderTree {
            node_views: Default::default(),
            nodes: Vec::new(),
            next_id: 1,
            outputs: Default::default(),
            node_graphs: Default::default(),
        },
        ..Scene::default()
    };
    let mut nodes = vec![node(1, None, &name, NodeKind::Group(GroupMask::default()))];
    let mut next = 2;
    let mut repeats: Vec<(u64, [f64; 2], f64)> = Vec::new();
    let mut add = |nodes: &mut Vec<Node>,
                   scene: &mut Scene,
                   notes: &mut Vec<String>,
                   img: &Img,
                   effect: TextureEffect,
                   extra: f64,
                   blend: BlendMode,
                   opacity: f64|
     -> Result<()> {
        let Some((w, h)) = size_of(&img.file) else {
            notes.push(format!(
                "{} left out: the picture can't be read",
                img.file
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default()
            ));
            return Ok(());
        };
        let image = scene.image_for_path(&img.file.to_string_lossy(), w, h)?;
        let mut kind = NewLayer::ImageMap.kind();
        if let NodeKind::Texture(TextureLayer {
            source, effect: e, ..
        }) = &mut kind
        {
            *source = TextureSource::ImageMap {
                image: Some(image),
                settings: Box::new(settings(img, effect, extra, notes)),
            };
            *e = effect;
        }
        let mut n = node(next, Some(1), &format!("{name} {}", effect.label()), kind);
        n.layer = Layer {
            blend,
            opacity,
            ..Layer::default()
        };
        if img.repeat != [1., 1.] || img.rotate != 0. {
            repeats.push((next, img.repeat, img.rotate));
        }
        next += 1;
        nodes.push(n);
        Ok(())
    };
    // AO multiplies the base colour from the top, as in a texture set.
    if let Some(ao) = &surface.occlusion {
        let img = ao
            .single()
            .ok_or_else(|| format!("{NEEDS_NODES}: occlusion"))?;
        add(
            &mut nodes,
            &mut scene,
            &mut notes,
            img,
            TextureEffect::DiffuseColor,
            1.,
            BlendMode::Multiply,
            1.,
        )?;
    }
    for p in ORDER {
        if *p == Param::GeometryOpacity {
            if let Some((height, distance)) = &surface.displacement {
                let img = height
                    .single()
                    .ok_or_else(|| format!("{NEEDS_NODES}: displacement"))?;
                add(
                    &mut nodes,
                    &mut scene,
                    &mut notes,
                    img,
                    TextureEffect::Displacement,
                    1.,
                    BlendMode::Normal,
                    1.,
                )?;
                material_channels.displacement_distance = distance.clamp(0., 1e6);
            } else {
                material_channels.displacement_distance = 0.;
            }
        }
        for (_, src) in surface.images.iter().filter(|(q, _)| q == p) {
            let Src::Layers { under, layers } = src else {
                continue;
            };
            // base_weight multiplies the base colour.
            let (fx, extra, base_blend) = match effect(*p) {
                Some((fx, extra)) => (fx, extra, BlendMode::Normal),
                None => (TextureEffect::DiffuseColor, 1., BlendMode::Multiply),
            };
            if let Some(c) = under {
                set_constant(&mut material_channels, *p, c);
            }
            // Top of the group first: the last layer goes in first.
            for (img, blend, opacity) in layers.iter().rev() {
                let blend =
                    if *blend == BlendMode::Normal && *opacity == 1. && *p == Param::BaseWeight {
                        base_blend
                    } else {
                        *blend
                    };
                add(
                    &mut nodes, &mut scene, &mut notes, img, fx, extra, blend, *opacity,
                )?;
            }
        }
    }
    material_channels.bump_amplitude = 0.;
    nodes.push(node(
        next,
        Some(1),
        &format!("{name} Material"),
        NodeKind::Material(Box::new(material_channels)),
    ));
    next += 1;
    scene.shader_tree = ShaderTree {
        node_views: Default::default(),
        nodes,
        next_id: next,
        outputs: Default::default(),
        node_graphs: Default::default(),
    };
    let mut next_item = scene.next_id;
    scene.give_layers_locators(&mut next_item)?;
    scene.next_id = scene.next_id.max(next_item);
    for (layer, repeat, rotate) in repeats {
        let locator = scene.shader_tree.nodes.iter().find_map(|n| match &n.kind {
            NodeKind::Texture(t) if n.id == layer => t.locator,
            _ => None,
        });
        if let Some(item) = scene.items.iter_mut().find(|i| Some(i.id) == locator)
            && let Some(TextureItem::Locator(l)) = &mut item.texture
        {
            l.repeat = repeat;
            l.uv_rotation = rotate;
        }
    }
    scene.validate()?;
    Ok((
        Preset {
            header: PresetHeader {
                kind: PresetKind::Material,
                thumbnail: None,
                source: None,
            },
            scene,
        },
        notes,
    ))
}

/// A constant under an input's pictures goes into the Material.
fn set_constant(m: &mut crate::shader_tree::MaterialChannels, p: Param, c: &[f64]) {
    let mut v = Values::default();
    v.set(p, value_of(p, c));
    let (one, _) = openpbr::to_material(&v);
    match p {
        Param::BaseColor => m.diffuse_color = one.diffuse_color,
        Param::SpecularRoughness => m.roughness = one.roughness,
        Param::BaseMetalness => m.metallic = one.metallic,
        Param::SpecularColor => m.specular_color = one.specular_color,
        Param::EmissionColor => m.luminous_color = one.luminous_color,
        Param::GeometryOpacity => m.dissolve = one.dissolve,
        Param::TransmissionColor => m.transparent_color = one.transparent_color,
        Param::TransmissionWeight => m.transparent_amount = one.transparent_amount,
        Param::CoatWeight => m.clearcoat_amount = one.clearcoat_amount,
        Param::SubsurfaceWeight => m.subsurface_amount = one.subsurface_amount,
        Param::FuzzWeight => m.sheen = one.sheen,
        _ => {}
    }
}

fn node(id: u64, parent: Option<u64>, name: &str, kind: NodeKind) -> Node {
    Node {
        id,
        parent,
        name: name.into(),
        layer: Layer::default(),
        kind,
        extras: Default::default(),
    }
}
