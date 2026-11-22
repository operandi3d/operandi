//! Shader-tree surfaces as UsdPreviewSurface materials (USD stage 2).
//!
//! Each polygon context (item, material tag, part, selection sets) is
//! evaluated through the shader tree, as the renderer does; equal surfaces
//! share one Material prim under `/root/_materials` (Blender's place for
//! them). A surface's inputs become UsdPreviewSurface inputs: a constant
//! stays a value; an image read through a UV map, with a constant scale,
//! offset or blend, becomes a UsdUVTexture (with a UsdTransform2d for
//! repeat and rotation). Anything UsdPreviewSurface can't express
//! (procedural patterns, gradients, layered mixes of images) is written as
//! its average value and noted.
use operandi_core::shader_eval::{Expr, ImageLookup, Input, Mix, Principled};
use operandi_core::texture_locator::Tiling;
use std::collections::BTreeSet;

/// A UsdPreviewSurface input's type (`OU_INPUT_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum InputType {
    Float = 0,
    Color = 1,
    Normal = 2,
}

/// Which output of a UsdUVTexture feeds the input (`OU_CHANNEL_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Channel {
    Rgb = 0,
    R = 1,
    G = 2,
    B = 3,
    A = 4,
}

/// UsdUVTexture wrapS/wrapT (`OU_WRAP_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Wrap {
    Repeat = 0,
    Mirror = 1,
    Clamp = 2,
    Black = 3,
}

/// An image feeding an input.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureUse {
    /// `Scene::images` id; the writer turns it into an asset path.
    pub image: u64,
    /// The mesh primvar the texture reads ("st" for the Texture map).
    pub uv: String,
    pub channel: Channel,
    /// UsdUVTexture scale and bias: out = texel × scale + bias.
    pub scale: [f32; 4],
    pub bias: [f32; 4],
    pub wrap: Wrap,
    /// Data (not colour): read without the sRGB curve.
    pub raw: bool,
    /// UsdTransform2d scale and rotation (degrees) on the UVs, when not
    /// the identity.
    pub transform: Option<UvTransform>,
}

/// One UsdPreviewSurface input.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceInput {
    pub name: &'static str,
    pub kind: InputType,
    pub value: [f32; 3],
    pub texture: Option<TextureUse>,
}

/// One Material prim.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialPrim {
    pub name: String,
    pub inputs: Vec<SurfaceInput>,
    /// The polygon tag it shades ("" for the default material).
    pub tag: String,
    /// Its Material Group's MaterialX network (U6), written beside the
    /// Preview Surface.
    pub mtlx: Option<crate::materialx_out::Network>,
}

/// What the conversion couldn't carry, for the export report.
#[derive(Default, Debug)]
pub struct MaterialNotes {
    pub flattened: BTreeSet<String>,
    pub dropped: BTreeSet<&'static str>,
}
impl MaterialNotes {
    pub fn into_lines(self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.flattened.is_empty() {
            let names: Vec<&str> = self.flattened.iter().map(String::as_str).collect();
            out.push(format!(
                "Procedural or layered textures written as plain values in {}",
                names.join(", ")
            ));
        }
        if !self.dropped.is_empty() {
            let names: Vec<&str> = self.dropped.iter().copied().collect();
            out.push(format!(
                "Not in USD's preview material: {}",
                names.join(", ")
            ));
        }
        out
    }
}

/// The primvar a mesh UV map is written as (plan.rs: the Texture map, or
/// a lookup's "first map", is "st").
pub fn uv_primvar(map: Option<&str>) -> String {
    match map {
        None => crate::plan::PRIMARY_UV_NAME.into(),
        Some(m) if m == operandi_core::PRIMITIVE_UV_MAP => crate::plan::PRIMARY_UV_NAME.into(),
        Some(m) => m.into(),
    }
}

/// A value an input can take: a constant, or an image times a scale plus
/// a bias, per colour component.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Const([f64; 3]),
    Texture {
        lookup: ImageLookup,
        alpha: bool,
        /// Some(k): only component k of the image (Channel).
        component: Option<u8>,
        scale: [f64; 3],
        bias: [f64; 3],
    },
}

/// Recognises the expressions UsdPreviewSurface can hold.
/// The picture an image read (whole, one channel, or its luminance) looks
/// up. A data map layer reads one channel of its picture, so its blend by
/// its own alpha is written as that channel (found exporting imported
/// roughness maps, USD-IQ14).
fn picture_of(e: &Expr) -> Option<&operandi_core::shader_eval::ImageLookup> {
    match e {
        Expr::ImageColor(l) => Some(l),
        Expr::Channel(inner, _) | Expr::Luma(inner) => picture_of(inner),
        _ => None,
    }
}

fn value_of(e: &Expr) -> Option<Value> {
    match e {
        Expr::Const(c) => Some(Value::Const(*c)),
        Expr::ImageColor(l) => Some(Value::Texture {
            lookup: (**l).clone(),
            alpha: false,
            component: None,
            scale: [1.0; 3],
            bias: [0.0; 3],
        }),
        Expr::ImageAlpha(l) => Some(Value::Texture {
            lookup: (**l).clone(),
            alpha: true,
            component: None,
            scale: [1.0; 3],
            bias: [0.0; 3],
        }),
        // A grey from one component, or (approximately) the luminance as red.
        Expr::Channel(inner, k) => component_of(inner, *k),
        Expr::Luma(inner) => component_of(inner, 0),
        // An image layer over what's below it, blended by its own alpha:
        // written as the image, which is exact for opaque pictures (USD's
        // preview material has no per-pixel blend; provisional, USD-Q05).
        Expr::Mix {
            mode: Mix::Blend,
            fac,
            a,
            b,
        } if matches!(&**fac, Expr::ImageAlpha(x) if picture_of(b) == Some(&**x))
            && a.constant().is_some() =>
        {
            value_of(b)
        }
        Expr::Mix { mode, fac, a, b } => {
            let t = fac.constant()?[0].clamp(0.0, 1.0);
            let (a, b) = (value_of(a)?, value_of(b)?);
            // lerp(a, mode(a, b), t), with exactly one side an image.
            let affine = |v: Value, s: [f64; 3], o: [f64; 3]| match v {
                Value::Texture {
                    lookup,
                    alpha,
                    component,
                    scale,
                    bias,
                } => Some(Value::Texture {
                    lookup,
                    alpha,
                    component,
                    scale: [0, 1, 2].map(|i| scale[i] * s[i]),
                    bias: [0, 1, 2].map(|i| bias[i] * s[i] + o[i]),
                }),
                Value::Const(_) => None,
            };
            match (mode, a, b) {
                (Mix::Blend, Value::Const(c), tex) => affine(tex, [t; 3], c.map(|c| c * (1.0 - t))),
                (Mix::Blend, tex, Value::Const(c)) => affine(tex, [1.0 - t; 3], c.map(|c| c * t)),
                // a × (1 − t + t b)
                (Mix::Multiply, Value::Const(c), tex) => {
                    affine(tex, c.map(|c| c * t), c.map(|c| c * (1.0 - t)))
                }
                (Mix::Multiply, tex, Value::Const(c)) => {
                    affine(tex, [0, 1, 2].map(|i| 1.0 - t + t * c[i]), [0.0; 3])
                }
                // a + t b, a − t b
                (Mix::Add, Value::Const(c), tex) => affine(tex, [t; 3], c),
                (Mix::Add, tex, Value::Const(c)) => affine(tex, [1.0; 3], c.map(|c| c * t)),
                (Mix::Subtract, Value::Const(c), tex) => affine(tex, [-t; 3], c),
                (Mix::Subtract, tex, Value::Const(c)) => affine(tex, [1.0; 3], c.map(|c| -c * t)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn component_of(inner: &Expr, k: u8) -> Option<Value> {
    match value_of(inner)? {
        Value::Texture {
            lookup,
            alpha: false,
            component: None,
            scale,
            bias,
        } => Some(Value::Texture {
            lookup,
            alpha: false,
            component: Some(k),
            scale: [scale[k as usize]; 3],
            bias: [bias[k as usize]; 3],
        }),
        // Already one channel, the same in r, g and b: its luminance and
        // any of its channels are itself.
        grey @ Value::Texture {
            alpha: false,
            component: Some(_),
            ..
        } => Some(grey),
        _ => None,
    }
}

/// A UsdTransform2d's scale and rotation (degrees).
pub type UvTransform = ([f32; 2], f32);

/// The UVs an image lookup reads, when they are a mesh UV map (with
/// repeat and rotation) that USD can name.
fn uv_of(vector: &Expr) -> Option<(String, Option<UvTransform>)> {
    let inner = match vector {
        Expr::Tile(inner, _) => inner,
        other => other,
    };
    let Expr::MeshUv {
        map,
        repeat,
        rotation,
    } = inner
    else {
        return None;
    };
    let transform = (*repeat != [1.0, 1.0] || *rotation != 0.0)
        .then(|| (repeat.map(|r| r as f32), *rotation as f32));
    Some((uv_primvar(map.as_deref()), transform))
}

fn wrap_of(t: Tiling) -> Wrap {
    match t {
        Tiling::Repeat => Wrap::Repeat,
        Tiling::Mirror => Wrap::Mirror,
        Tiling::Edge => Wrap::Clamp,
        Tiling::Reset => Wrap::Black,
    }
}

/// One input from an expression; `None` when nothing should be written.
#[allow(clippy::too_many_arguments)]
fn input(
    name: &'static str,
    kind: InputType,
    expr: &Expr,
    // Applied after the expression: out = v × post_scale + post_bias.
    post_scale: f64,
    post_bias: f64,
    raw: bool,
    material: &str,
    notes: &mut MaterialNotes,
) -> SurfaceInput {
    let scalar = kind == InputType::Float;
    let finish = |c: [f64; 3]| c.map(|v| (v * post_scale + post_bias) as f32);
    let flat = |notes: &mut MaterialNotes| {
        notes.flattened.insert(material.to_string());
        let v = expr.eval(&|_, _| [0.5; 4]);
        SurfaceInput {
            name,
            kind,
            value: finish(v),
            texture: None,
        }
    };
    match value_of(expr) {
        Some(Value::Const(c)) => SurfaceInput {
            name,
            kind,
            value: finish(c),
            texture: None,
        },
        Some(Value::Texture {
            lookup,
            alpha,
            component,
            scale,
            bias,
        }) => {
            let Some((uv, transform)) = uv_of(&lookup.vector) else {
                return flat(notes);
            };
            let channel = match (alpha, component, scalar) {
                (true, _, _) => Channel::A,
                (false, Some(1), _) => Channel::G,
                (false, Some(2), _) => Channel::B,
                (false, Some(_), _) | (false, None, true) => Channel::R,
                (false, None, false) => Channel::Rgb,
            };
            let s = scale.map(|v| (v * post_scale) as f32);
            let b = bias.map(|v| (v * post_scale + post_bias) as f32);
            // UsdUVTexture applies scale and bias per output component; a
            // single-channel read uses that channel's slot.
            let (scale4, bias4) = match channel {
                Channel::Rgb => ([s[0], s[1], s[2], 1.0], [b[0], b[1], b[2], 0.0]),
                Channel::R => ([s[0], 1.0, 1.0, 1.0], [b[0], 0.0, 0.0, 0.0]),
                Channel::G => ([1.0, s[0], 1.0, 1.0], [0.0, b[0], 0.0, 0.0]),
                Channel::B => ([1.0, 1.0, s[0], 1.0], [0.0, 0.0, b[0], 0.0]),
                Channel::A => ([1.0, 1.0, 1.0, s[0]], [0.0, 0.0, 0.0, b[0]]),
            };
            SurfaceInput {
                name,
                kind,
                value: finish(expr.eval(&|_, _| [0.5; 4])),
                texture: Some(TextureUse {
                    image: lookup.image,
                    uv,
                    channel,
                    scale: scale4,
                    bias: bias4,
                    wrap: wrap_of(lookup.extension),
                    raw,
                    transform,
                }),
            }
        }
        None => flat(notes),
    }
}

fn get(p: &Principled, which: Input) -> Option<&Expr> {
    p.inputs.iter().find(|(i, _)| *i == which).map(|(_, e)| e)
}

/// The UsdPreviewSurface inputs of a surface.
pub fn preview_surface(p: &Principled, name: &str, notes: &mut MaterialNotes) -> MaterialPrim {
    let mut inputs: Vec<SurfaceInput> = Vec::new();
    let add = |inputs: &mut Vec<SurfaceInput>,
               input_name,
               kind,
               which: Input,
               scale: f64,
               notes: &mut MaterialNotes| {
        if let Some(e) = get(p, which) {
            inputs.push(input(
                input_name,
                kind,
                e,
                scale,
                0.0,
                !which.is_colour(),
                name,
                notes,
            ));
        }
    };
    use InputType::{Color, Float};
    add(
        &mut inputs,
        "diffuseColor",
        Color,
        Input::BaseColor,
        1.0,
        notes,
    );
    add(&mut inputs, "metallic", Float, Input::Metallic, 1.0, notes);
    add(
        &mut inputs,
        "roughness",
        Float,
        Input::Roughness,
        1.0,
        notes,
    );
    add(&mut inputs, "ior", Float, Input::Ior, 1.0, notes);
    add(
        &mut inputs,
        "clearcoat",
        Float,
        Input::CoatWeight,
        1.0,
        notes,
    );
    add(
        &mut inputs,
        "clearcoatRoughness",
        Float,
        Input::CoatRoughness,
        1.0,
        notes,
    );
    // Emission Color × Emission Strength, left out when dark.
    match get(p, Input::EmissionStrength).map(Expr::constant) {
        Some(Some(s)) if s[0] > 0.0 => {
            add(
                &mut inputs,
                "emissiveColor",
                Color,
                Input::EmissionColor,
                s[0],
                notes,
            );
        }
        Some(None) => {
            notes.flattened.insert(name.to_string());
            add(
                &mut inputs,
                "emissiveColor",
                Color,
                Input::EmissionColor,
                1.0,
                notes,
            );
        }
        _ => {}
    }
    inputs.retain(|i| !(i.name == "emissiveColor" && i.texture.is_none() && i.value == [0.0; 3]));
    // Opacity: Alpha, times what Transparent Amount leaves (1 − amount).
    let keep = match p.see_through.as_ref().map(|s| s.amount.constant()) {
        None => 1.0,
        Some(Some(a)) => 1.0 - a[0].clamp(0.0, 1.0),
        Some(None) => {
            notes.flattened.insert(name.to_string());
            0.5
        }
    };
    add(&mut inputs, "opacity", Float, Input::Alpha, keep, notes);
    inputs.retain(|i| !(i.name == "opacity" && i.texture.is_none() && i.value[0] >= 1.0));
    // The tangent-space normal map, stored 0..1, read as -1..1 (Blender's
    // set_normal_texture_range). Only an image can be written.
    if let Some(normal) = &p.normal_map {
        let n = input(
            "normal",
            InputType::Normal,
            normal,
            2.0,
            -1.0,
            true,
            name,
            notes,
        );
        if n.texture.is_some() {
            inputs.push(n);
        } else {
            notes.dropped.insert("procedural normal maps");
        }
    }
    if p.bump.is_some() {
        notes.dropped.insert("bump");
    }
    if p.displacement.is_some() {
        notes.dropped.insert("displacement");
    }
    if [
        Input::SubsurfaceWeight,
        Input::SheenWeight,
        Input::TransmissionWeight,
    ]
    .into_iter()
    .any(|i| get(p, i).is_some_and(|e| e.constant().is_none_or(|c| c[0] > 0.0)))
    {
        notes.dropped.insert("subsurface, sheen and transmission");
    }
    MaterialPrim {
        name: name.to_string(),
        inputs,
        tag: String::new(),
        mtlx: None,
    }
}

/// Gathers the scene's materials while the plan walks its meshes.
pub struct Collector<'s> {
    tree: std::borrow::Cow<'s, operandi_core::shader_tree::ShaderTree>,
    /// Each prim's surface, in `prims` order.
    pub surfaces: Vec<Principled>,
    pub prims: Vec<MaterialPrim>,
    names: BTreeSet<String>,
    pub notes: MaterialNotes,
}
impl<'s> Collector<'s> {
    pub fn new(scene: &'s operandi_core::Scene) -> Self {
        Self {
            tree: scene.shading_tree(),
            surfaces: Vec::new(),
            prims: Vec::new(),
            names: BTreeSet::new(),
            notes: MaterialNotes::default(),
        }
    }

    /// The material index of each polygon of `item` (polylines included,
    /// so indices line up with the mesh's faces).
    pub fn per_face(
        &mut self,
        scene: &operandi_core::Scene,
        item: &operandi_core::Item,
    ) -> Vec<usize> {
        use operandi_core::shader_eval::Context;
        let mesh = &item.mesh;
        let ancestors = scene.item_ancestors(item.id);
        let locators = scene.locator_frames(item.id);
        // Selection sets per polygon (G-01), as the renderer reads them.
        let mut sets: Vec<Vec<&str>> = vec![Vec::new(); mesh.faces.len()];
        for (name, faces) in &mesh.attributes.polygon_sets {
            for &f in faces {
                if let Some(list) = sets.get_mut(f as usize) {
                    list.push(name.as_str());
                }
            }
        }
        let mut seen: std::collections::HashMap<(&str, Option<&str>, &[&str]), usize> =
            std::collections::HashMap::new();
        let mut out = Vec::with_capacity(mesh.faces.len());
        for (f, face) in mesh.faces.iter().enumerate() {
            let key = (
                face.material.as_str(),
                face.part.as_deref(),
                sets[f].as_slice(),
            );
            if let Some(&m) = seen.get(&key) {
                out.push(m);
                continue;
            }
            let surface = self
                .tree
                .evaluate(&Context {
                    item: item.id,
                    ancestors: &ancestors,
                    material: key.0,
                    part: key.1,
                    sets: key.2,
                    locators: &locators,
                })
                .principled();
            let m = match self.surfaces.iter().position(|s| *s == surface) {
                Some(m) => m,
                None => {
                    let tag = if key.0.is_empty() {
                        operandi_core::material::DEFAULT_MATERIAL
                    } else {
                        key.0
                    };
                    let mut name = tag.to_string();
                    let mut n = 2;
                    while self.names.contains(&name) {
                        name = format!("{tag} {n}");
                        n += 1;
                    }
                    self.names.insert(name.clone());
                    let mut prim = preview_surface(&surface, &name, &mut self.notes);
                    prim.tag = key.0.to_string();
                    self.surfaces.push(surface);
                    self.prims.push(prim);
                    self.prims.len() - 1
                }
            };
            seen.insert(key, m);
            out.push(m);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::shader_tree::Filtering;

    fn image() -> Expr {
        Expr::ImageColor(Box::new(ImageLookup::uv(3, Filtering::Bilinear)))
    }

    #[test]
    fn constant_scales_offsets_and_blends_become_scale_and_bias() {
        let tinted = Expr::times(Expr::Const([0.5, 1.0, 0.25]), image());
        let Some(Value::Texture { scale, bias, .. }) = value_of(&tinted) else {
            panic!()
        };
        assert_eq!((scale, bias), ([0.5, 1.0, 0.25], [0.0; 3]));
        // 30 % of the image over grey 0.2: 0.2 × 0.7 + texel × 0.3.
        let blend = Expr::mix(Mix::Blend, Expr::scalar(0.3), Expr::scalar(0.2), image());
        let Some(Value::Texture { scale, bias, .. }) = value_of(&blend) else {
            panic!()
        };
        assert!((scale[0] - 0.3).abs() < 1e-12 && (bias[0] - 0.14).abs() < 1e-12);
        // 1 − image (Dissolve from a picture).
        let Some(Value::Texture { scale, bias, .. }) = value_of(&Expr::one_minus(image())) else {
            panic!()
        };
        assert_eq!((scale, bias), ([-1.0; 3], [1.0; 3]));
    }

    #[test]
    fn single_channels_read_the_matching_output_and_patterns_flatten() {
        let mut notes = MaterialNotes::default();
        let red = input(
            "roughness",
            InputType::Float,
            &Expr::channel(image(), 1),
            1.0,
            0.0,
            true,
            "M",
            &mut notes,
        );
        let t = red.texture.unwrap();
        assert_eq!((t.channel, t.raw), (Channel::G, true));
        // A screen-space value has no USD form: written as its average.
        let flat = input(
            "roughness",
            InputType::Float,
            &Expr::Geometry(operandi_core::shader_eval::GeometryInput::ScreenX),
            1.0,
            0.0,
            true,
            "M",
            &mut notes,
        );
        assert!(flat.texture.is_none());
        assert!(notes.flattened.contains("M"));
    }
}
