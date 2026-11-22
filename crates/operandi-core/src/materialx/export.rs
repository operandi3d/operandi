//! MaterialX export (MX-05): a Material Group (its Material and Image
//! Map layers) as one `open_pbr_surface` material. Constants come from
//! the Material through the OpenPBR table; each image-driven channel is
//! an image chain, layers over it `mix` (Normal blend, its opacity) or
//! `multiply` (Multiply blend) nodes. Whatever has no MaterialX form here
//! (procedurals, gradients, other blends, masks, nested groups, some
//! image adjustments) is left to the Material's constant, and listed in
//! the returned losses. The importer reads every chain written here back
//! into the same layers (MX-06).
use crate::openpbr::{self, Kind, Param, Value};
use crate::shader_tree::{
    BlendMode, ImageSettings, MaterialChannels, Node, NodeId, NodeKind, Swizzle, TextureEffect,
    TextureSource,
};
use crate::{Result, Scene};
use std::fmt::Write as _;
use std::path::Path;

/// The OpenPBR input an Image Map with this effect drives, and the
/// factor between the layer's value and the input's (the importer's
/// table, read the other way).
fn param_of(effect: TextureEffect) -> Option<(Param, f64)> {
    use Param::*;
    use TextureEffect as E;
    Some(match effect {
        E::DiffuseColor => (BaseColor, 1.),
        E::Metallic => (BaseMetalness, 1.),
        E::DiffuseRoughness => (BaseDiffuseRoughness, 1.),
        E::Roughness => (SpecularRoughness, 1.),
        E::SpecularColor => (SpecularColor, 1.),
        E::SpecularAmount => (SpecularWeight, 2.),
        E::ClearcoatAmount => (CoatWeight, 1.),
        E::TransparentAmount => (TransmissionWeight, 1.),
        E::TransparentColor => (TransmissionColor, 1.),
        E::SubsurfaceAmount => (SubsurfaceWeight, 1.),
        E::SubsurfaceColor => (SubsurfaceRadiusScale, 1.),
        E::Sheen => (FuzzWeight, 1.),
        E::LuminousColor => (EmissionColor, 1.),
        E::LuminousAmount => (EmissionLuminance, 1. / openpbr::STRENGTH_PER_NIT),
        E::Dissolve => (GeometryOpacity, 1.),
        E::Normal => (GeometryNormal, 1.),
        _ => return None,
    })
}

/// A MaterialX name: letters, digits and underscores, not starting with
/// a digit.
pub fn mx_name(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, 'M');
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn num(v: f64) -> String {
    let v = if v.abs() < 1e-12 { 0. } else { v };
    let s = format!("{v:.9}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s.into()
    }
}

fn value_text(v: Value, length: bool) -> (&'static str, String) {
    match v {
        // Lengths go out in OpenPBR's default unit, the centimetre.
        Value::Float(f) if length => ("float", num(f / openpbr::DEFAULT_LENGTH_IN_METRES)),
        Value::Float(f) => ("float", num(f)),
        Value::Color(c) => ("color3", c.map(num).join(", ")),
        Value::Bool(b) => ("boolean", b.to_string()),
        Value::Geometry => ("vector3", String::new()),
    }
}

fn ty_of(p: Param) -> &'static str {
    match p.kind() {
        Kind::Float => "float",
        Kind::Color3 => "color3",
        Kind::Boolean => "boolean",
        Kind::Vector3 => "vector3",
    }
}

fn iv(name: &str, ty: &str, value: &str) -> String {
    format!(
        "    <input name=\"{name}\" type=\"{ty}\" value=\"{}\" />\n",
        esc(value)
    )
}

/// Collected nodes and losses while writing.
struct Out<'a> {
    scene: &'a Scene,
    nodes: String,
    losses: Vec<String>,
    base: String,
    folder: &'a Path,
    count: usize,
}

impl Out<'_> {
    fn fresh(&mut self, what: &str) -> String {
        self.count += 1;
        format!("{}_{what}_{}", self.base, self.count)
    }

    /// One Image Map layer as a node chain; returns the last node's name.
    fn chain(&mut self, layer: &Node, param: Param, factor: f64) -> Option<String> {
        let NodeKind::Texture(t) = &layer.kind else {
            return None;
        };
        let TextureSource::ImageMap {
            image: Some(image),
            settings,
        } = &t.source
        else {
            return None;
        };
        let pic = self.scene.images.iter().find(|i| i.id == *image)?;
        let s: &ImageSettings = settings;
        let colour = matches!(param.kind(), Kind::Color3);
        let normal = param == Param::GeometryNormal;
        // Settings with no node here are losses.
        let plain = ImageSettings::default();
        let opacity_invert = param == Param::GeometryOpacity;
        let inverted = s.invert_red && s.invert_green && s.invert_blue;
        let some_inverted = s.invert_red || s.invert_green || s.invert_blue;
        if s.gamma != plain.gamma
            || s.contrast != plain.contrast
            || s.low_value != plain.low_value
            || s.high_value != plain.high_value
            || s.clamp
            || s.alpha == crate::shader_tree::ImageAlpha::AlphaOnly
        {
            self.losses
                .push(format!("{}: image adjustments left out", layer.name));
        }
        let path = Path::new(&pic.path);
        let file = path
            .strip_prefix(self.folder)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| pic.path.clone());
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let space = if !colour || normal {
            ""
        } else if matches!(ext.as_str(), "exr" | "hdr") {
            " colorspace=\"lin_rec709\""
        } else {
            " colorspace=\"srgb_texture\""
        };
        // Texture coordinates: the locator's repeat and rotation.
        let mut texcoord = None;
        if let Some(l) = t.locator.and_then(|id| self.scene.texture_locator(id)) {
            if l.projection != crate::shader_tree::Projection::UvMap {
                self.losses.push(format!(
                    "{}: {} projection written as UV",
                    layer.name,
                    l.projection.label()
                ));
            }
            if l.repeat != [1., 1.] || l.uv_rotation != 0. {
                let tc = self.fresh("texcoord");
                let _ = writeln!(self.nodes, "  <texcoord name=\"{tc}\" type=\"vector2\" />");
                let pl = self.fresh("place2d");
                let scale = l.repeat.map(|r| if r != 0. { 1. / r } else { 1. });
                let _ = write!(
                    self.nodes,
                    "  <place2d name=\"{pl}\" type=\"vector2\">\n    <input name=\"texcoord\" type=\"vector2\" nodename=\"{tc}\" />\n{}{}  </place2d>\n",
                    iv(
                        "scale",
                        "vector2",
                        &format!("{}, {}", num(scale[0]), num(scale[1]))
                    ),
                    iv("rotate", "float", &num(l.uv_rotation)),
                );
                texcoord = Some(pl);
            }
        }
        let channel = match s.swizzle {
            Swizzle::Red => Some(0),
            Swizzle::Green => Some(1),
            Swizzle::Blue => Some(2),
            Swizzle::Alpha => Some(3),
            Swizzle::Rgba | Swizzle::Rgb => None,
        };
        let image_ty = if channel.is_some() {
            if channel == Some(3) {
                "color4"
            } else {
                "color3"
            }
        } else if normal {
            "vector3"
        } else {
            ty_of(param)
        };
        let img = self.fresh("image");
        let _ = write!(
            self.nodes,
            "  <image name=\"{img}\" type=\"{image_ty}\">\n    <input name=\"file\" type=\"filename\" value=\"{}\"{space} />\n",
            esc(&file)
        );
        if let Some(tc) = &texcoord {
            let _ = writeln!(
                self.nodes,
                "    <input name=\"texcoord\" type=\"vector2\" nodename=\"{tc}\" />"
            );
        }
        self.nodes.push_str("  </image>\n");
        let mut last = img;
        let mut last_ty = image_ty;
        if let Some(c) = channel {
            let ex = self.fresh("extract");
            let _ = write!(
                self.nodes,
                "  <extract name=\"{ex}\" type=\"float\">\n    <input name=\"in\" type=\"{last_ty}\" nodename=\"{last}\" />\n{}  </extract>\n",
                iv("index", "integer", &c.to_string())
            );
            last = ex;
            last_ty = "float";
        }
        // Some channels inverted (a DirectX normal's green): × ±1, + 0/1.
        if some_inverted && !inverted {
            let flips = [s.invert_red, s.invert_green, s.invert_blue];
            let vty = if last_ty == "float" {
                "color3"
            } else {
                last_ty
            };
            let m = self.fresh("multiply");
            let k = flips.map(|f| if f { "-1" } else { "1" }).join(", ");
            let _ = write!(
                self.nodes,
                "  <multiply name=\"{m}\" type=\"{last_ty}\">\n    <input name=\"in1\" type=\"{last_ty}\" nodename=\"{last}\" />\n{}  </multiply>\n",
                iv("in2", vty, &k)
            );
            let a = self.fresh("add");
            let c = flips.map(|f| if f { "1" } else { "0" }).join(", ");
            let _ = write!(
                self.nodes,
                "  <add name=\"{a}\" type=\"{last_ty}\">\n    <input name=\"in1\" type=\"{last_ty}\" nodename=\"{m}\" />\n{}  </add>\n",
                iv("in2", vty, &c)
            );
            last = a;
        }
        // Dissolve layers read opacity inverted, so an inverted picture
        // is the opacity as it is.
        if inverted != opacity_invert {
            let inv = self.fresh("invert");
            let _ = write!(
                self.nodes,
                "  <invert name=\"{inv}\" type=\"{last_ty}\">\n    <input name=\"in\" type=\"{last_ty}\" nodename=\"{last}\" />\n  </invert>\n"
            );
            last = inv;
        }
        let scale = s.brightness * factor;
        if (scale - 1.).abs() > 1e-9 {
            let m = self.fresh("multiply");
            let _ = write!(
                self.nodes,
                "  <multiply name=\"{m}\" type=\"{last_ty}\">\n    <input name=\"in1\" type=\"{last_ty}\" nodename=\"{last}\" />\n{}  </multiply>\n",
                iv("in2", "float", &num(scale))
            );
            last = m;
        }
        if normal {
            let n = self.fresh("normalmap");
            let _ = write!(
                self.nodes,
                "  <normalmap name=\"{n}\" type=\"vector3\">\n    <input name=\"in\" type=\"vector3\" nodename=\"{last}\" />\n  </normalmap>\n"
            );
            last = n;
        }
        Some(last)
    }
}

/// The Material Group holding `node` (itself when it is one).
fn group_of(scene: &Scene, node: NodeId) -> Result<NodeId> {
    let n = scene
        .shader_tree
        .get(node)
        .ok_or("That shader tree layer doesn't exist")?;
    match n.kind {
        NodeKind::Group(_) => Ok(n.id),
        NodeKind::Material(_) => n
            .parent
            .ok_or_else(|| "The material isn't in a group".into()),
        _ => Err("Pick a Material Group or a Material to export".into()),
    }
}

/// The group's material as MaterialX text, and what was left out.
pub fn material_x(scene: &Scene, node: NodeId, folder: &Path) -> Result<(String, Vec<String>)> {
    let group = group_of(scene, node)?;
    let tree = &scene.shader_tree;
    let gnode = tree.get(group).ok_or("Missing group")?;
    let name = mx_name(&gnode.name);
    // Top of the group first.
    let children: Vec<&Node> = tree.children(Some(group)).collect();
    let material: MaterialChannels = children
        .iter()
        .rev()
        .find_map(|n| match &n.kind {
            NodeKind::Material(m) if n.layer.enable => Some((**m).clone()),
            _ => None,
        })
        .ok_or("The group has no Material")?;
    let (values, mut losses) = openpbr::from_material(&material);
    // Free nodes (node editing) need the standard form (NE-10).
    if tree.free_node_count(group) > 0 {
        losses.push("the material's free nodes".into());
    }
    let mut out = Out {
        scene,
        nodes: String::new(),
        losses: Vec::new(),
        base: name.clone(),
        folder,
        count: 0,
    };
    // Each channel's Image Maps, bottom first.
    let mut wired: Vec<(Param, String)> = Vec::new();
    let mut displacement = None;
    let mut params: Vec<Param> = Vec::new();
    for n in children.iter().rev() {
        match &n.kind {
            NodeKind::Texture(t) if n.layer.enable => {
                let is_image = matches!(t.source, TextureSource::ImageMap { image: Some(_), .. });
                if t.effect == TextureEffect::Displacement && is_image {
                    displacement = Some(*n);
                    continue;
                }
                match (param_of(t.effect), is_image) {
                    (Some((p, _)), true) => {
                        if !params.contains(&p) {
                            params.push(p);
                        }
                    }
                    _ => out.losses.push(format!("{}: left out", n.name)),
                }
            }
            NodeKind::Material(_) | NodeKind::Texture(_) => {}
            _ => out.losses.push(format!("{}: left out", n.name)),
        }
    }
    for p in params {
        let mut current: Option<String> = None;
        for n in children.iter().rev() {
            let NodeKind::Texture(t) = &n.kind else {
                continue;
            };
            if !n.layer.enable
                || !matches!(t.source, TextureSource::ImageMap { image: Some(_), .. })
                || param_of(t.effect).map(|x| x.0) != Some(p)
            {
                continue;
            }
            let factor = param_of(t.effect).map_or(1., |x| x.1);
            let Some(chain) = out.chain(n, p, factor) else {
                continue;
            };
            let ty = if p == Param::GeometryNormal {
                "vector3"
            } else {
                ty_of(p)
            };
            let op = n.layer.opacity;
            let blend = n.layer.blend;
            current = Some(match (current.take(), blend) {
                (None, BlendMode::Normal) if op >= 1. => chain,
                (None, BlendMode::Normal) => {
                    let (vty, v) = value_text(values.get(p), p.is_length());
                    let m = out.fresh("mix");
                    let _ = write!(
                        out.nodes,
                        "  <mix name=\"{m}\" type=\"{ty}\">\n    <input name=\"fg\" type=\"{ty}\" nodename=\"{chain}\" />\n    <input name=\"bg\" type=\"{vty}\" value=\"{v}\" />\n    <input name=\"mix\" type=\"float\" value=\"{}\" />\n  </mix>\n",
                        num(op)
                    );
                    m
                }
                (Some(below), BlendMode::Normal) => {
                    let m = out.fresh("mix");
                    let _ = write!(
                        out.nodes,
                        "  <mix name=\"{m}\" type=\"{ty}\">\n    <input name=\"fg\" type=\"{ty}\" nodename=\"{chain}\" />\n    <input name=\"bg\" type=\"{ty}\" nodename=\"{below}\" />\n    <input name=\"mix\" type=\"float\" value=\"{}\" />\n  </mix>\n",
                        num(op)
                    );
                    m
                }
                (Some(below), BlendMode::Multiply) if op >= 1. => {
                    let m = out.fresh("multiply");
                    let _ = write!(
                        out.nodes,
                        "  <multiply name=\"{m}\" type=\"{ty}\">\n    <input name=\"in1\" type=\"{ty}\" nodename=\"{below}\" />\n    <input name=\"in2\" type=\"{ty}\" nodename=\"{chain}\" />\n  </multiply>\n"
                    );
                    m
                }
                (below, _) => {
                    out.losses.push(format!(
                        "{}: {} blend at {:.0} % left out",
                        n.name,
                        blend.label(),
                        op * 100.
                    ));
                    match below {
                        Some(b) => b,
                        None => continue,
                    }
                }
            });
        }
        if let Some(c) = current {
            wired.push((p, c));
        }
    }
    let mut disp = None;
    if let Some(n) = displacement
        && let Some(chain) = out.chain(n, Param::SpecularRoughness, 1.)
    {
        let d = format!("{name}_displacement");
        let _ = write!(
            out.nodes,
            "  <displacement name=\"{d}\" type=\"displacementshader\">\n    <input name=\"displacement\" type=\"float\" nodename=\"{chain}\" />\n    <input name=\"scale\" type=\"float\" value=\"{}\" />\n  </displacement>\n",
            num(material.displacement_distance / openpbr::DEFAULT_LENGTH_IN_METRES)
        );
        disp = Some(d);
    }
    losses.append(&mut out.losses);

    let shader = format!("{name}_open_pbr_surface");
    let mut xml = String::from("<?xml version=\"1.0\"?>\n");
    let _ = writeln!(
        xml,
        "<!-- Written by Operandi: OpenPBR Surface {} -->",
        openpbr::OPENPBR_VERSION
    );
    xml.push_str("<materialx version=\"1.39\" colorspace=\"lin_rec709\">\n");
    let _ = writeln!(
        xml,
        "  <open_pbr_surface name=\"{shader}\" type=\"surfaceshader\">"
    );
    for p in Param::ALL {
        if let Some((_, node)) = wired.iter().find(|(q, _)| q == p) {
            let _ = writeln!(
                xml,
                "    <input name=\"{}\" type=\"{}\" nodename=\"{node}\" />",
                p.name(),
                ty_of(*p)
            );
            continue;
        }
        let v = values.get(*p);
        let same = match (v, p.default()) {
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < 1e-9,
            (Value::Color(a), Value::Color(b)) => (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9),
            (a, b) => a == b,
        };
        if same || matches!(v, Value::Geometry) {
            continue;
        }
        let (ty, text) = value_text(v, p.is_length());
        let _ = writeln!(
            xml,
            "    <input name=\"{}\" type=\"{ty}\" value=\"{text}\" />",
            p.name()
        );
    }
    xml.push_str("  </open_pbr_surface>\n");
    let _ = writeln!(xml, "  <surfacematerial name=\"{name}\" type=\"material\">");
    let _ = writeln!(
        xml,
        "    <input name=\"surfaceshader\" type=\"surfaceshader\" nodename=\"{shader}\" />"
    );
    if let Some(d) = &disp {
        let _ = writeln!(
            xml,
            "    <input name=\"displacementshader\" type=\"displacementshader\" nodename=\"{d}\" />"
        );
    }
    xml.push_str("  </surfacematerial>\n");
    xml.push_str(&out.nodes);
    xml.push_str("</materialx>\n");
    Ok((xml, losses))
}

/// Write the material holding `node` to `path` (File > Export Material
/// as MaterialX..., and a preset's .mtlx beside it). Picture paths are
/// relative when they're under the file's folder.
pub fn write(scene: &Scene, node: NodeId, path: &Path) -> Result<Vec<String>> {
    let folder = path.parent().unwrap_or(Path::new(""));
    let (xml, losses) = material_x(scene, node, folder)?;
    crate::persistence::write_atomic(path, xml.as_bytes())?;
    Ok(losses)
}
