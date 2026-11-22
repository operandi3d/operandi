//! Shader tree and subdivision through the bridge: `shader_tree_get`,
//! `material_get` / `material_set`, `shader_layer_add` / `shader_layer_set`
//! and `subdivision_set`.
//!
//! Every edit goes through the Shading panel's own edits
//! (`ShaderTreeEdit::Add`, `Replace`, `SetImage`), so ranges and refusals
//! are the panel's, and one call is one undo step however many settings it
//! changes. Channels are named after the panel's rows in snake_case
//! ("Diffuse Color" is `diffuse_color`), in the bridge's units: fractions
//! 0–1 where the panel shows percent, radians where it shows degrees,
//! metres, linear RGB 0–1.
//!
//! Layer ids come from the tree renders use (`Scene::render_shader_tree`):
//! an older file's materials join the tree on its first edit, and these
//! are the ids that edit gives them.
use crate::bridge::Result;
use operandi_core::gradient::{Curve, GradientInput, Key, KeyShape, Unit};
use operandi_core::shader_tree::{
    BlendMode, Channel, NewLayer, Node, NodeId, NodeKind, ShaderTree, ShaderTreeEdit, ShadingModel,
    TagMask, TextureEffect, TextureLayer, TextureSource,
};
use operandi_core::vertex_map_texture::{VariationSource, VertexMapKind, VertexMapRef};
use operandi_core::{Command, Editor};
use serde_json::{Map, Value, json};

/// `shader_layer_add` types, by the bridge's name.
fn new_layer(name: &str) -> Result<NewLayer> {
    use operandi_core::procedural::ProceduralKind;
    let key = norm(name);
    let fixed = [
        ("material", NewLayer::Material),
        ("group", NewLayer::Group),
        ("materialgroup", NewLayer::Group),
        ("folder", NewLayer::Folder),
        ("shader", NewLayer::Shader),
        ("constant", NewLayer::Constant),
        ("imagemap", NewLayer::ImageMap),
        ("gradient", NewLayer::Gradient),
        ("vertexmap", NewLayer::VertexMap),
        ("vertexmaptexture", NewLayer::VertexMap),
        ("variation", NewLayer::Variation),
        ("variationtexture", NewLayer::Variation),
        ("process", NewLayer::Process),
        ("occlusion", NewLayer::Occlusion),
    ];
    if let Some((_, layer)) = fixed.iter().find(|(k, _)| *k == key) {
        return Ok(*layer);
    }
    ProceduralKind::ALL
        .iter()
        .find(|k| norm(k.label()) == key)
        .map(|&k| NewLayer::Procedural(k))
        .ok_or_else(|| {
            format!(
                "Unknown layer type {name}; use {} or a procedural ({})",
                LAYER_TYPES.join(", "),
                ProceduralKind::ALL
                    .iter()
                    .map(|k| snake(k.label()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// The fixed `shader_layer_add` types, for the schema.
pub const LAYER_TYPES: &[&str] = &[
    "material",
    "group",
    "folder",
    "shader",
    "constant",
    "image_map",
    "gradient",
    "vertex_map",
    "variation",
    "process",
    "occlusion",
];

/// Keys `shader_layer_set` and `shader_layer_add` take besides channels.
const LAYER_KEYS: &[&str] = &[
    "name",
    "enable",
    "invert",
    "blend",
    "opacity",
    "effect",
    "shading_model",
    "tag",
    "input",
    "value_keys",
    "color_keys",
    "map",
    "source",
    "image",
    "channels",
];

/// Lower case letters and digits only, for forgiving name matches.
fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// A panel label as a bridge key: "Apply to Sub-Group" is
/// `apply_to_sub_group`.
pub fn snake(label: &str) -> String {
    let mut out = String::new();
    for c in label.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_owned()
}

fn channel_json(c: &Channel<'_>) -> (Value, String) {
    match c {
        Channel::Percent(v) => (json!(**v), "fraction 0-1".into()),
        Channel::PercentIn(v, lo, hi) => (json!(**v), format!("fraction {lo} to {hi}")),
        Channel::Color(c) => (json!(**c), "[r, g, b] linear 0-1".into()),
        Channel::Distance(v) => (json!(**v), "m".into()),
        // Stored in degrees; the bridge speaks radians.
        Channel::Angle(v) => (json!(v.to_radians()), "rad, 0 to pi".into()),
        Channel::Number(v, lo, hi) => (json!(**v), format!("number {lo} to {hi}")),
        Channel::Bool(v) => (json!(**v), "bool".into()),
    }
}

/// A layer's channels and their units, by bridge key.
fn channels_json(node: &Node) -> (Value, Value) {
    let mut copy = node.clone();
    let mut values = Map::new();
    let mut units = Map::new();
    for (label, c) in copy.channels() {
        let (v, u) = channel_json(&c);
        values.insert(snake(label), v);
        units.insert(snake(label), json!(u));
    }
    (Value::Object(values), Value::Object(units))
}

fn number(key: &str, v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("{key} must be a number"))
}

fn set_channel(key: &str, c: Channel<'_>, v: &Value) -> Result<()> {
    let range = |n: f64, lo: f64, hi: f64| {
        if (lo..=hi).contains(&n) {
            Ok(n)
        } else {
            Err(format!("{key} must be {lo} to {hi}, not {n}"))
        }
    };
    match c {
        Channel::Percent(x) => *x = range(number(key, v)?, 0., 1.)?,
        Channel::PercentIn(x, lo, hi) => *x = range(number(key, v)?, lo, hi)?,
        Channel::Color(x) => {
            let rgb = v
                .as_array()
                .filter(|a| a.len() == 3)
                .ok_or_else(|| format!("{key} is [r, g, b], each 0-1"))?;
            for (slot, c) in x.iter_mut().zip(rgb) {
                *slot = range(number(key, c)?, 0., 1.)?;
            }
        }
        Channel::Distance(x) => *x = range(number(key, v)?, 0., 1e6)?,
        Channel::Angle(x) => {
            *x = range(number(key, v)?, 0., std::f64::consts::PI)?.to_degrees();
        }
        Channel::Number(x, lo, hi) => *x = range(number(key, v)?, lo, hi)?,
        Channel::Bool(x) => {
            *x = v
                .as_bool()
                .ok_or_else(|| format!("{key} must be true or false"))?;
        }
    }
    Ok(())
}

/// Set channels by bridge key; an unknown key lists the layer's own.
fn set_channels(node: &mut Node, given: &[(&String, &Value)]) -> Result<()> {
    for (key, v) in given {
        let mut channels = node.channels();
        let names: Vec<String> = channels.iter().map(|(l, _)| snake(l)).collect();
        let Some(i) = names.iter().position(|n| n == *key) else {
            return Err(if names.is_empty() {
                format!("{} has no channel {key}", node_type(node))
            } else {
                format!(
                    "{} has no channel {key}; its channels are {}",
                    node_type(node),
                    names.join(", ")
                )
            });
        };
        let (_, c) = channels.swap_remove(i);
        set_channel(key, c, v)?;
    }
    Ok(())
}

fn node_type(node: &Node) -> String {
    format!("{} layer {:?}", node.type_label(), node.name)
}

/// What a material tag shades with, or a Material layer by id or name.
fn find_material(tree: &ShaderTree, args: &Map<String, Value>) -> Result<NodeId> {
    let is_material = |id: NodeId| {
        tree.get(id)
            .is_some_and(|n| matches!(n.kind, NodeKind::Material(_)))
    };
    if let Some(v) = args.get("layer").filter(|v| !v.is_null()) {
        let id = v.as_u64().ok_or("layer must be a layer id")?;
        return if is_material(id) {
            Ok(id)
        } else {
            Err(format!(
                "Layer {id} is not a Material; shader_tree_get lists the layers"
            ))
        };
    }
    let name = match args.get("material").filter(|v| !v.is_null()) {
        Some(v) => v.as_str().ok_or("material must be a name")?,
        None => operandi_core::material::DEFAULT_MATERIAL,
    };
    if let Some(id) = tree.material_for_tag(name) {
        return Ok(id);
    }
    // A Material layer's own name ("Base Material") works too.
    if let Some(n) = tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Material(_)) && n.name == name)
    {
        return Ok(n.id);
    }
    let mut known: Vec<String> = tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Material(_)))
        .map(|n| n.name.clone())
        .collect();
    known.sort();
    Err(format!(
        "No material {name:?}; poly.setMaterial makes one. Material layers: {}",
        known.join(", ")
    ))
}

/// The material tag a Material layer shades, if its group has one.
fn material_tag(tree: &ShaderTree, node: &Node) -> Option<String> {
    match node.parent.and_then(|p| tree.get(p)).map(|g| &g.kind) {
        Some(NodeKind::Group(g)) => match &g.tag {
            TagMask::Name(t) => Some(t.clone()),
            _ => None,
        },
        None if tree.material_for_tag(operandi_core::material::DEFAULT_MATERIAL)
            == Some(node.id) =>
        {
            Some(operandi_core::material::DEFAULT_MATERIAL.into())
        }
        _ => None,
    }
}

fn curve_unit(input: GradientInput) -> &'static str {
    match input.unit() {
        Unit::Degrees => "rad",
        Unit::Metres => "m",
        Unit::Percent => "fraction",
        Unit::Number => "number",
    }
}

/// A key position from the bridge's units into the input's own.
fn key_at(input: Option<GradientInput>, at: f64) -> f64 {
    match input.map(GradientInput::unit) {
        Some(Unit::Degrees) => at.to_degrees(),
        _ => at,
    }
}
fn key_at_out(input: Option<GradientInput>, at: f64) -> f64 {
    match input.map(GradientInput::unit) {
        Some(Unit::Degrees) => at.to_radians(),
        _ => at,
    }
}

fn curves_json(input: Option<GradientInput>, value: &Curve<f64>, color: &Curve<[f64; 4]>) -> Value {
    json!({
        "value_keys": value.keys.iter().map(|k| json!({
            "at": key_at_out(input, k.at), "value": k.out, "shape": k.shape.label().to_lowercase()
        })).collect::<Vec<_>>(),
        "color_keys": color.keys.iter().map(|k| json!({
            "at": key_at_out(input, k.at), "color": [k.out[0], k.out[1], k.out[2]],
            "alpha": k.out[3], "shape": k.shape.label().to_lowercase()
        })).collect::<Vec<_>>(),
    })
}

fn shape(v: Option<&Value>) -> Result<KeyShape> {
    match v.filter(|v| !v.is_null()) {
        None => Ok(KeyShape::Smooth),
        Some(v) => {
            let s = v.as_str().map(norm).unwrap_or_default();
            KeyShape::ALL
                .into_iter()
                .find(|k| norm(k.label()) == s)
                .ok_or_else(|| "shape is smooth, linear or stepped".into())
        }
    }
}

fn keys_of<'a>(key: &str, v: &'a Value) -> Result<&'a Vec<Value>> {
    v.as_array()
        .ok_or_else(|| format!("{key} is a list of keys"))
}

fn value_keys(input: Option<GradientInput>, v: &Value) -> Result<Curve<f64>> {
    let mut curve = Curve { keys: Vec::new() };
    for k in keys_of("value_keys", v)? {
        let k = k
            .as_object()
            .ok_or("each value key is {at, value, shape?}")?;
        curve.keys.push(Key {
            at: key_at(input, number("at", k.get("at").ok_or("a key needs at")?)?),
            out: number("value", k.get("value").ok_or("a value key needs value")?)?,
            shape: shape(k.get("shape"))?,
        });
    }
    curve.sort();
    Ok(curve)
}

fn color_keys(input: Option<GradientInput>, v: &Value) -> Result<Curve<[f64; 4]>> {
    let mut curve = Curve { keys: Vec::new() };
    for k in keys_of("color_keys", v)? {
        let k = k
            .as_object()
            .ok_or("each color key is {at, color, alpha?, shape?}")?;
        let rgb = k
            .get("color")
            .and_then(Value::as_array)
            .filter(|a| a.len() == 3)
            .ok_or("a color key needs color [r, g, b]")?;
        let alpha = match k.get("alpha").filter(|v| !v.is_null()) {
            Some(a) => number("alpha", a)?,
            None => 1.,
        };
        curve.keys.push(Key {
            at: key_at(input, number("at", k.get("at").ok_or("a key needs at")?)?),
            out: [
                number("color", &rgb[0])?,
                number("color", &rgb[1])?,
                number("color", &rgb[2])?,
                alpha,
            ],
            shape: shape(k.get("shape"))?,
        });
    }
    curve.sort();
    Ok(curve)
}

fn pick<T: Copy>(all: &[T], label: fn(T) -> &'static str, key: &str, v: &Value) -> Result<T> {
    let given = v.as_str().map(norm).unwrap_or_default();
    all.iter()
        .copied()
        .find(|t| norm(label(*t)) == given)
        .ok_or_else(|| {
            format!(
                "{key} is one of {}",
                all.iter().map(|t| label(*t)).collect::<Vec<_>>().join(", ")
            )
        })
}

/// One layer in full: its Layer block, type settings and channels.
pub fn layer_json(tree: &ShaderTree, node: &Node) -> Value {
    let (channels, units) = channels_json(node);
    let mut out = json!({
        "layer": node.id,
        "parent": node.parent,
        "name": node.name,
        "type": node.type_label(),
        "enable": node.layer.enable,
        "invert": node.layer.invert,
        "blend": node.layer.blend.label(),
        "opacity": node.layer.opacity,
        "effect": tree.effect_label(node),
        "channels": channels,
        "units": units,
    });
    match &node.kind {
        NodeKind::Material(m) => {
            out["shading_model"] = json!(m.shading_model.label());
            out["material"] = json!(material_tag(tree, node));
        }
        NodeKind::Group(g) => {
            out["tag_type"] = json!(g.tag_type.label());
            out["tag"] = json!(g.tag.label());
        }
        NodeKind::Texture(t) => {
            out["locator"] = json!(t.locator);
            match &t.source {
                TextureSource::Gradient(g) => {
                    out["input"] = json!(g.input.label());
                    out["input_unit"] = json!(curve_unit(g.input));
                    let c = curves_json(Some(g.input), &g.value, &g.color);
                    out["value_keys"] = c["value_keys"].clone();
                    out["color_keys"] = c["color_keys"].clone();
                }
                TextureSource::Variation(v) => {
                    out["source"] = json!(v.source.label());
                    let c = curves_json(None, &v.value, &v.color);
                    out["value_keys"] = c["value_keys"].clone();
                    out["color_keys"] = c["color_keys"].clone();
                }
                TextureSource::VertexMap { map, .. } => {
                    out["map"] = match map {
                        Some(m) => {
                            json!({"name": m.name, "kind": format!("{:?}", m.kind).to_lowercase()})
                        }
                        None => Value::Null,
                    };
                }
                TextureSource::ImageMap { image, .. } => out["image"] = json!(image),
                _ => {}
            }
        }
        _ => {}
    }
    out
}

/// `shader_tree_get`: every layer top to bottom, or one in full.
pub fn tree_get(e: &Editor, args: &Map<String, Value>) -> Result<Value> {
    crate::commands::check_keys("shader_tree_get", args, &["layer"])?;
    let tree = e.scene.render_shader_tree();
    if let Some(v) = args.get("layer").filter(|v| !v.is_null()) {
        let id = v.as_u64().ok_or("layer must be a layer id")?;
        let node = tree.get(id).ok_or_else(|| format!("No layer {id}"))?;
        return Ok(layer_json(&tree, node));
    }
    let rows: Vec<Value> = tree
        .rows(&|_| false)
        .into_iter()
        .map(|(n, depth)| {
            let mut row = json!({
                "layer": n.id, "parent": n.parent, "depth": depth, "name": n.name,
                "type": n.type_label(), "effect": tree.effect_label(n),
                "enable": n.layer.enable,
            });
            if let NodeKind::Group(g) = &n.kind {
                row["tag"] = json!(g.tag.label());
            }
            if let NodeKind::Material(_) = n.kind {
                row["material"] = json!(material_tag(&tree, n));
            }
            row
        })
        .collect();
    Ok(
        json!({"layers": rows, "note": "Top row first; textures apply to the layers below them in their group."}),
    )
}

/// `material_get`: one Material layer's channels.
pub fn material_get(e: &Editor, args: &Map<String, Value>) -> Result<Value> {
    crate::commands::check_keys("material_get", args, &["material", "layer"])?;
    let tree = e.scene.render_shader_tree();
    let id = find_material(&tree, args)?;
    Ok(layer_json(&tree, tree.get(id).expect("found")))
}

/// `material_set`: channels (flat or under `channels`), the Layer block,
/// name and shading model of one Material, one undo step.
pub fn material_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    let tree = e.scene.render_shader_tree().into_owned();
    let id = find_material(&tree, args)?;
    let mut settings = args.clone();
    settings.remove("material");
    settings.remove("layer");
    // Flat channel keys join any `channels` object.
    let mut channels = match settings.remove("channels") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(m)) => m,
        Some(_) => return Err("channels must be an object".into()),
    };
    for key in settings.keys().cloned().collect::<Vec<_>>() {
        if !LAYER_KEYS.contains(&key.as_str()) {
            channels.insert(key.clone(), settings.remove(&key).expect("present"));
        }
    }
    if !channels.is_empty() {
        settings.insert("channels".into(), Value::Object(channels));
    }
    if settings.is_empty() {
        return Err("Give at least one channel to set; material_get lists them".into());
    }
    edit_layer(e, id, &settings)
}

/// `shader_layer_set`: settings of any layer, one undo step.
pub fn layer_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    let mut known = LAYER_KEYS.to_vec();
    known.push("layer");
    crate::commands::check_keys("shader_layer_set", args, &known)?;
    let id = args
        .get("layer")
        .and_then(Value::as_u64)
        .ok_or("layer is required (a layer id; shader_tree_get lists them)")?;
    let mut settings = args.clone();
    settings.remove("layer");
    edit_layer(e, id, &settings)
}

/// `shader_layer_add`: a new layer with its settings, one undo step.
/// Returns its id.
pub fn layer_add(e: &mut Editor, args: &Map<String, Value>) -> Result<NodeId> {
    let mut known = LAYER_KEYS.to_vec();
    known.extend(["type", "parent", "material"]);
    crate::commands::check_keys("shader_layer_add", args, &known)?;
    let kind = new_layer(
        args.get("type")
            .and_then(Value::as_str)
            .ok_or("type is required")?,
    )?;
    let tree = e.scene.render_shader_tree().into_owned();
    // Where it goes: inside `parent`, or in the group of `material`'s
    // Material (Add Layer with that row selected), else at the top level.
    let selected = match (
        args.get("parent").filter(|v| !v.is_null()),
        args.get("material").filter(|v| !v.is_null()),
    ) {
        (Some(_), Some(_)) => return Err("Give parent or material, not both".into()),
        (Some(p), None) => {
            let id = p.as_u64().ok_or("parent must be a layer id")?;
            let node = tree.get(id).ok_or_else(|| format!("No layer {id}"))?;
            if !node.is_container() {
                return Err(format!(
                    "parent must be a Material Group or Folder; {} is not",
                    node_type(node)
                ));
            }
            Some(id)
        }
        (None, Some(_)) => {
            let mut m = Map::new();
            m.insert("material".into(), args["material"].clone());
            Some(find_material(&tree, &m)?)
        }
        (None, None) => None,
    };
    let start = e.undo.len();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: kind,
        selected,
    }))?;
    let after = e.scene.render_shader_tree();
    let id = after
        .nodes
        .iter()
        .map(|n| n.id)
        .find(|&id| tree.get(id).is_none())
        .ok_or("The layer was not added")?;
    drop(after);
    let mut settings = args.clone();
    for k in ["type", "parent", "material"] {
        settings.remove(k);
    }
    if !settings.is_empty() {
        edit_layer(e, id, &settings)?;
    }
    crate::properties::fold_steps(e, start, &format!("Add {}", kind.label()));
    Ok(id)
}

/// Apply settings to layer `id` through Replace (and SetImage), folded
/// into one undo step.
fn edit_layer(e: &mut Editor, id: NodeId, settings: &Map<String, Value>) -> Result<()> {
    crate::commands::check_keys("shader layer settings", settings, LAYER_KEYS)?;
    let tree = e.scene.render_shader_tree().into_owned();
    let mut node = tree
        .get(id)
        .cloned()
        .ok_or_else(|| format!("No layer {id}; shader_tree_get lists the layers"))?;
    let get = |k: &str| settings.get(k).filter(|v| !v.is_null());
    let label = node_type(&node);
    if let Some(v) = get("name") {
        node.name = v.as_str().ok_or("name must be text")?.into();
    }
    if let Some(v) = get("enable") {
        node.layer.enable = v.as_bool().ok_or("enable must be true or false")?;
    }
    if let Some(v) = get("invert") {
        node.layer.invert = v.as_bool().ok_or("invert must be true or false")?;
    }
    if let Some(v) = get("blend") {
        node.layer.blend = pick(BlendMode::ALL, BlendMode::label, "blend", v)?;
    }
    if let Some(v) = get("opacity") {
        node.layer.opacity = number("opacity", v)?;
        if !(0.0..=1.0).contains(&node.layer.opacity) {
            return Err("opacity must be 0 to 1".into());
        }
    }
    if let Some(v) = get("shading_model") {
        let NodeKind::Material(m) = &mut node.kind else {
            return Err(format!("{label} has no shading_model"));
        };
        m.shading_model = pick(ShadingModel::ALL, ShadingModel::label, "shading_model", v)?;
    }
    if let Some(v) = get("tag") {
        let NodeKind::Group(g) = &mut node.kind else {
            return Err(format!("{label} has no tag (only Material Groups do)"));
        };
        let t = v.as_str().ok_or("tag is a polygon tag name, or (all)")?;
        g.tag = match t {
            "(all)" | "" => TagMask::All,
            "(none)" => TagMask::None,
            t => TagMask::Name(t.into()),
        };
    }
    texture_settings(&mut node, settings, &label)?;
    if let Some(v) = get("channels") {
        let m = v.as_object().ok_or("channels must be an object")?;
        set_channels(&mut node, &m.iter().collect::<Vec<_>>())?;
    }
    let start = e.undo.len();
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))?;
    if let Some(v) = settings.get("image") {
        if !matches!(
            tree.get(id).map(|n| &n.kind),
            Some(NodeKind::Texture(TextureLayer {
                source: TextureSource::ImageMap { .. },
                ..
            }))
        ) {
            return Err(format!("{label} has no image (only Image Maps do)"));
        }
        let image = match v {
            Value::Null => None,
            Value::String(path) => {
                let (w, h) = picture_size(std::path::Path::new(path))?;
                Some((path.clone(), w, h))
            }
            Value::Number(n) => {
                let want = n
                    .as_u64()
                    .ok_or("image is a file path or a scene image id")?;
                let i = e
                    .scene
                    .images
                    .iter()
                    .find(|i| i.id == want)
                    .ok_or_else(|| format!("No scene image {want}"))?;
                Some((i.path.clone(), i.width, i.height))
            }
            _ => return Err("image is a file path, a scene image id, or null".into()),
        };
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetImage {
            node: id,
            image,
        }))?;
    }
    let name = e
        .scene
        .shader_tree
        .get(id)
        .map_or_else(|| "Shader Layer".to_owned(), |n| n.name.clone());
    crate::properties::fold_steps(e, start, &format!("Edit {name}"));
    Ok(())
}

/// Texture-only settings: effect, Gradient input and keys, Variation
/// source and keys, Vertex Map map.
fn texture_settings(node: &mut Node, settings: &Map<String, Value>, label: &str) -> Result<()> {
    let get = |k: &str| settings.get(k).filter(|v| !v.is_null());
    let texture_only = [
        "effect",
        "input",
        "value_keys",
        "color_keys",
        "map",
        "source",
    ];
    let NodeKind::Texture(t) = &mut node.kind else {
        if let Some(k) = texture_only.iter().find(|k| get(k).is_some()) {
            return Err(format!("{label} has no {k} (texture layers only)"));
        }
        return Ok(());
    };
    if let Some(v) = get("effect") {
        t.effect = pick(TextureEffect::ALL, TextureEffect::label, "effect", v)?;
    }
    let wrong = |k: &str| format!("{label} has no {k}");
    match &mut t.source {
        TextureSource::Gradient(g) => {
            if let Some(v) = get("input") {
                g.input = pick(GradientInput::ALL, GradientInput::label, "input", v)?;
            }
            if let Some(v) = get("value_keys") {
                g.value = value_keys(Some(g.input), v)?;
            }
            if let Some(v) = get("color_keys") {
                g.color = color_keys(Some(g.input), v)?;
            }
            if get("map").is_some() || get("source").is_some() {
                return Err(wrong("map or source (Gradient takes input)"));
            }
        }
        TextureSource::Variation(var) => {
            if let Some(v) = get("source") {
                var.source = pick(&VariationSource::ALL, VariationSource::label, "source", v)?;
            }
            if let Some(v) = get("value_keys") {
                var.value = value_keys(None, v)?;
            }
            if let Some(v) = get("color_keys") {
                var.color = color_keys(None, v)?;
            }
            if get("map").is_some() || get("input").is_some() {
                return Err(wrong("map or input"));
            }
        }
        TextureSource::VertexMap { map, .. } => {
            if let Some(v) = settings.get("map") {
                *map = match v {
                    Value::Null => None,
                    Value::Object(m) => Some(VertexMapRef {
                        name: m
                            .get("name")
                            .and_then(Value::as_str)
                            .ok_or("map is {name, kind}")?
                            .into(),
                        kind: match m.get("kind").and_then(Value::as_str).map(norm).as_deref() {
                            Some("weight") | None => VertexMapKind::Weight,
                            Some("color" | "colour" | "rgb" | "rgba") => VertexMapKind::Color,
                            _ => return Err("map kind is weight or color".into()),
                        },
                    }),
                    _ => return Err("map is {name, kind (weight or color)} or null".into()),
                };
            }
            if let Some(k) = ["input", "value_keys", "color_keys", "source"]
                .iter()
                .find(|k| get(k).is_some())
            {
                return Err(wrong(k));
            }
        }
        _ => {
            if let Some(k) = ["input", "value_keys", "color_keys", "map", "source"]
                .iter()
                .find(|k| get(k).is_some())
            {
                return Err(wrong(k));
            }
        }
    }
    Ok(())
}

/// Width and height of a PNG or JPEG from its header (core's reader).
fn picture_size(path: &std::path::Path) -> Result<(u32, u32)> {
    operandi_core::node_graph::picture_size(path)
}

/// Every subdivision setting `subdivision_set` takes, with its meaning.
pub const SUBDIVISION_FIELDS: &[(&str, &str)] = &[
    (
        "toggle",
        "\"subdivision\" (Tab: Face <-> Subdivision) or \"catmull_clark\" (Shift+Tab: Face <-> Catmull-Clark). Polygon mode: the selected polygons, or all when none are; other modes: every polygon of the mesh",
    ),
    (
        "subdivision_level",
        "display level of Subdivision polygons, 0-6",
    ),
    (
        "catmull_clark_level",
        "display level of Catmull-Clark polygons, 0-6",
    ),
    (
        "subdivision_render_level",
        "render level of Subdivision polygons, 0-6",
    ),
    (
        "catmull_clark_render_level",
        "render level of Catmull-Clark polygons, 0-6",
    ),
    ("boundary_rule", "crease, smooth or crease_edges"),
    ("uv_boundary_rule", "crease, smooth or crease_edges"),
    ("linear_uvs", "bool"),
];

fn boundary(key: &str, v: &Value) -> Result<operandi_core::subdivision::BoundaryRule> {
    use operandi_core::subdivision::BoundaryRule as B;
    match v.as_str().map(norm).as_deref() {
        Some("crease") => Ok(B::Crease),
        Some("smooth") => Ok(B::Smooth),
        Some("creaseedges") => Ok(B::CreaseEdges),
        _ => Err(format!("{key} is crease, smooth or crease_edges")),
    }
}
fn boundary_name(b: operandi_core::subdivision::BoundaryRule) -> &'static str {
    use operandi_core::subdivision::BoundaryRule as B;
    match b {
        B::Crease => "crease",
        B::Smooth => "smooth",
        B::CreaseEdges => "crease_edges",
    }
}

/// A mesh item's subdivision settings and polygon kinds, for `item_get`.
pub fn subdivision_json(item: &operandi_core::Item) -> Value {
    use operandi_core::PolygonRepresentation as P;
    let s = item.subdivision.clone().unwrap_or_default();
    let reps = &item.mesh.attributes.representations;
    let count = |want: fn(&P) -> bool| {
        (0..item.mesh.faces.len())
            .filter(|&f| want(reps.get(f).unwrap_or(&P::Face)))
            .count()
    };
    json!({
        "subdivision_level": s.subdivision_level,
        "catmull_clark_level": s.catmull_clark_level,
        "subdivision_render_level": s.subdivision_render_level,
        "catmull_clark_render_level": s.catmull_clark_render_level,
        "boundary_rule": boundary_name(s.boundary_rule),
        "uv_boundary_rule": boundary_name(s.uv_boundary_rule),
        "linear_uvs": s.linear_uvs,
        "face_polygons": count(|p| matches!(p, P::Face)),
        "subdivision_polygons": count(|p| matches!(p, P::Subdivs)),
        "catmull_clark_polygons": count(|p| matches!(p, P::CatmullClark)),
    })
}

/// `subdivision_set`: toggle polygons and set the mesh's levels, one undo
/// step.
pub fn subdivision_set(e: &mut Editor, args: &Map<String, Value>) -> Result<()> {
    let mut known: Vec<&str> = SUBDIVISION_FIELDS.iter().map(|(k, _)| *k).collect();
    known.push("item");
    crate::commands::check_keys("subdivision_set", args, &known)?;
    let get = |k: &str| args.get(k).filter(|v| !v.is_null());
    if !SUBDIVISION_FIELDS.iter().any(|(k, _)| get(k).is_some()) {
        return Err("Nothing to change: give toggle or a setting".into());
    }
    let start = e.undo.len();
    if let Some(v) = get("item") {
        let id = v.as_u64().ok_or("item must be an item id")?;
        if !e
            .scene
            .items
            .iter()
            .any(|i| i.id == id && i.camera.is_none() && i.light.is_none())
        {
            return Err(format!("No mesh item {id}"));
        }
        if e.scene.selection.item != Some(id) {
            e.dispatch(Command::SelectItem(Some(id)))?;
        }
    }
    if let Some(v) = get("toggle") {
        use operandi_core::subdivision::Toggle;
        let toggle = match v.as_str().map(norm).as_deref() {
            Some("subdivision" | "subdivs" | "tab") => Toggle::Subdivision,
            Some("catmullclark" | "shifttab") => Toggle::CatmullClark,
            _ => return Err("toggle is \"subdivision\" or \"catmull_clark\"".into()),
        };
        e.dispatch(Command::ToggleSubdivision(toggle))?;
    }
    let mut s = e
        .scene
        .active()
        .ok_or("No active mesh; pass item")?
        .subdivision
        .clone()
        .unwrap_or_default();
    let before = s.clone();
    let level = |k: &str, slot: &mut u8| -> Result<()> {
        if let Some(v) = get(k) {
            *slot = v
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| format!("{k} must be a whole number"))?;
        }
        Ok(())
    };
    level("subdivision_level", &mut s.subdivision_level)?;
    level("catmull_clark_level", &mut s.catmull_clark_level)?;
    level("subdivision_render_level", &mut s.subdivision_render_level)?;
    level(
        "catmull_clark_render_level",
        &mut s.catmull_clark_render_level,
    )?;
    if let Some(v) = get("boundary_rule") {
        s.boundary_rule = boundary("boundary_rule", v)?;
    }
    if let Some(v) = get("uv_boundary_rule") {
        s.uv_boundary_rule = boundary("uv_boundary_rule", v)?;
    }
    if let Some(v) = get("linear_uvs") {
        s.linear_uvs = v.as_bool().ok_or("linear_uvs must be true or false")?;
    }
    if s != before {
        e.dispatch(Command::SetSubdivision(s))?;
    }
    crate::properties::fold_steps(e, start, "Subdivision");
    Ok(())
}
