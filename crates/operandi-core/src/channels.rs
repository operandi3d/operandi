//! `item.channel`: one value of a scene item or shader-tree layer, by
//! channel name (extensibility stage 2; UIC-165 general setter).
//!
//! Each kind of thing registers a table of [`Channel`]s in [`TABLES`]: its
//! name, value type, which targets it applies to, how to read it and the
//! one `Command` that sets it (one undo step through `Editor::dispatch`).
//! The dispatcher here never changes for a new channel: add entries to a
//! table (or a new table to `TABLES`). See docs/COMMAND_REGISTRY.md, "How
//! to add channels".
//!
//! Channel names are Operandi's own for now (provisional, to be matched to
//! Modo's channel names later). Units at the boundary follow the map:
//! distances in metres (or with a unit), percents 0–1 (or `50%`), angles
//! in radians.
use crate::camera_light::{
    AreaShape, LightBlend, LightEdit, LightFalloff, LightRender, ShadowType, VolumetricEdit,
};
use crate::shader_tree::{BlendMode, Node, NodeId, ShaderTreeEdit};
use crate::{Command, Editor, Scene};

pub type Result<T> = std::result::Result<T, String>;

/// What a channel holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelType {
    Bool,
    Int,
    Float,
    Text,
    /// One of these names (or its index).
    Enum(&'static [&'static str]),
    /// Linear RGB, 0–1; `name.r` / `.g` / `.b` set one component.
    Colour,
    /// Another item's id (`none` clears it).
    ItemRef,
    /// Metres, or a value typed with a length unit.
    Distance,
    /// A fraction 0–1 at the command boundary (or `50%`).
    Percent,
    /// Radians at the command boundary (or `30deg`); stored in radians.
    Angle,
}

/// A channel's value.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChannelValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    /// Index into the channel's names.
    Enum(usize),
    Colour([f64; 3]),
    Item(Option<u64>),
}

/// What `item.channel` acts on: a scene item, a shader-tree layer or the
/// scene itself (its time settings, `scene:true`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Item(u64),
    Layer(NodeId),
    Scene,
}

/// Where a channel's value comes from (AR-08, master plan phase 0 door
/// 3). Today every channel holds a constant, stored in the item, layer or
/// scene field its table reads. Later a channel can hold a key curve (A14
/// animation), a link to another channel (A12) or a sim result, and
/// [`Held::value_at`] is where those are read at a time. Saved data will
/// use this type, so it is serialisable now; the file format version
/// covers the new variants when they arrive.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Held {
    /// One value at every time.
    Constant(ChannelValue),
}

impl Held {
    /// The value at `time`.
    pub fn value_at(&self, _time: crate::time_tick::Tick) -> ChannelValue {
        match self {
            Held::Constant(v) => v.clone(),
        }
    }
    /// "constant" (later "keys", "link", "sim").
    pub fn label(&self) -> &'static str {
        match self {
            Held::Constant(_) => "constant",
        }
    }
}

/// How channel `name` of `target` gets its value (AR-08).
pub fn held(scene: &Scene, target: Target, name: &str) -> Result<Held> {
    let channel = find(scene, target, name).ok_or_else(|| unknown(scene, target, name))?;
    let value = (channel.get)(scene, target).ok_or_else(|| format!("{name} has no value here"))?;
    Ok(Held::Constant(value))
}

/// One settable value.
pub struct Channel {
    pub name: &'static str,
    pub ty: ChannelType,
    /// Whether `target` has this channel (a light's, a layer's...).
    pub applies: fn(&Scene, Target) -> bool,
    pub get: fn(&Scene, Target) -> Option<ChannelValue>,
    /// The one command that sets the value (validated by the editor).
    pub set: fn(&Scene, Target, ChannelValue) -> Result<Command>,
}

/// Every channel table. A feature adds its table here.
pub static TABLES: &[&[Channel]] = &[
    ITEM,
    crate::item_common_channels::ITEM,
    LIGHT,
    LAYER,
    crate::environment_channels::ITEM,
    crate::environment_channels::MATERIAL,
    crate::environment_channels::LAYER,
    crate::scene_time::CHANNELS,
];

// ------------------------------------------------------------ lookup

/// The channel `name` that `target` has.
pub fn find(scene: &Scene, target: Target, name: &str) -> Option<&'static Channel> {
    TABLES
        .iter()
        .flat_map(|t| t.iter())
        .find(|c| c.name.eq_ignore_ascii_case(name) && (c.applies)(scene, target))
}

/// The channel names `target` has, for error messages and help.
pub fn names(scene: &Scene, target: Target) -> Vec<&'static str> {
    TABLES
        .iter()
        .flat_map(|t| t.iter())
        .filter(|c| (c.applies)(scene, target))
        .map(|c| c.name)
        .collect()
}

/// `name` split into a channel and a colour component (`color.r`).
fn component(name: &str) -> (&str, Option<usize>) {
    match name.rsplit_once('.') {
        Some((base, "r" | "R")) => (base, Some(0)),
        Some((base, "g" | "G")) => (base, Some(1)),
        Some((base, "b" | "B")) => (base, Some(2)),
        _ => (name, None),
    }
}

fn unknown(scene: &Scene, target: Target, name: &str) -> String {
    let what = match target {
        Target::Item(id) => format!("item {id}"),
        Target::Layer(id) => format!("layer {id}"),
        Target::Scene => "the scene".to_string(),
    };
    let mut list: Vec<String> = names(scene, target)
        .into_iter()
        .map(str::to_owned)
        .collect();
    list.extend(layer_fields(scene, target));
    list.extend(output_fields(scene, target));
    list.extend(
        kit(scene, target)
            .map(|k| k.value_names())
            .unwrap_or_default(),
    );
    if list.is_empty() {
        format!("{what} has no channels item.channel can set yet")
    } else {
        format!(
            "{what} has no channel \"{name}\"; it has {}",
            list.join(", ")
        )
    }
}

/// Text for a value, the way `item.channel ... ?` answers.
pub fn text(ty: ChannelType, value: &ChannelValue) -> String {
    match (value, ty) {
        (ChannelValue::Bool(b), _) => b.to_string(),
        (ChannelValue::Int(n), _) => n.to_string(),
        (ChannelValue::Float(f), _) => f.to_string(),
        (ChannelValue::Text(t), _) => t.clone(),
        (ChannelValue::Enum(i), ChannelType::Enum(names)) => names
            .get(*i)
            .map_or_else(|| i.to_string(), |n| n.to_string()),
        (ChannelValue::Enum(i), _) => i.to_string(),
        (ChannelValue::Colour(c), _) => format!("{},{},{}", c[0], c[1], c[2]),
        (ChannelValue::Item(Some(id)), _) => id.to_string(),
        (ChannelValue::Item(None), _) => "none".into(),
    }
}

/// A typed value for a channel of `ty`.
pub fn parse(ty: ChannelType, name: &str, text: &str) -> Result<ChannelValue> {
    use crate::numeric::{Quantity, parse_quantity};
    let text = text.trim();
    Ok(match ty {
        ChannelType::Bool => ChannelValue::Bool(crate::command_args::boolean(name, text)?),
        ChannelType::Int => ChannelValue::Int(
            text.parse()
                .map_err(|_| format!("{name} takes a whole number, not \"{text}\""))?,
        ),
        ChannelType::Float => ChannelValue::Float(parse_quantity(text, Quantity::Scalar)?),
        ChannelType::Text => ChannelValue::Text(text.to_owned()),
        ChannelType::Enum(names) => {
            ChannelValue::Enum(crate::command_args::choice(name, text, names)?)
        }
        ChannelType::Colour => {
            let parts: Vec<f64> = text
                .trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|p| p.trim().parse::<f64>())
                .collect::<std::result::Result<_, _>>()
                .map_err(|_| format!("{name} takes r,g,b (0–1), not \"{text}\""))?;
            match parts.as_slice() {
                [r, g, b] => ChannelValue::Colour([*r, *g, *b]),
                _ => return Err(format!("{name} takes r,g,b (0–1), not \"{text}\"")),
            }
        }
        ChannelType::ItemRef if text.eq_ignore_ascii_case("none") => ChannelValue::Item(None),
        ChannelType::ItemRef => ChannelValue::Item(Some(
            text.parse()
                .map_err(|_| format!("{name} takes an item id or none, not \"{text}\""))?,
        )),
        ChannelType::Distance => ChannelValue::Float(parse_quantity(text, Quantity::Distance)?),
        // Bare numbers are the fraction as given; `50%` via script_percent.
        ChannelType::Percent => ChannelValue::Float(match text.parse::<f64>() {
            Ok(fraction) => fraction,
            Err(_) => crate::command_names::script_percent(text)? / 100.,
        }),
        // Bare numbers are radians as given (no round trip through
        // degrees); `30deg` and the like go through script_angle.
        ChannelType::Angle => ChannelValue::Float(match text.parse::<f64>() {
            Ok(radians) => radians,
            Err(_) => crate::command_names::script_angle(text)?.to_radians(),
        }),
    })
}

/// A kit item's data (kit_item.rs), when `target` is one.
fn kit(scene: &Scene, target: Target) -> Option<&crate::kit_item::KitItem> {
    match target {
        Target::Item(id) => scene.item(id)?.kit.as_deref(),
        _ => None,
    }
}

/// The value of channel `name` (or a colour component) of `target`.
pub fn get(scene: &Scene, target: Target, name: &str) -> Result<String> {
    if let Some(answer) = layer_field_get(scene, target, name) {
        return answer;
    }
    if let Some(answer) = output_field_get(scene, target, name) {
        return answer;
    }
    // A kit item's own values, read-only while its kit is missing.
    if find(scene, target, name).is_none()
        && let Some(text) = kit(scene, target).and_then(|k| k.value_text(name))
    {
        return Ok(text);
    }
    let (base, part) = component(name);
    // Read through how the channel is held, at the scene's current time
    // (AR-08): a constant today, keys or links later.
    let channel = find(scene, target, base).ok_or_else(|| unknown(scene, target, name))?;
    let value = held(scene, target, channel.name)?.value_at(scene.time.current);
    Ok(match (part, &value) {
        (Some(i), ChannelValue::Colour(c)) => c[i].to_string(),
        (Some(_), _) => return Err(format!("{base} isn't a colour")),
        _ => text(channel.ty, &value),
    })
}

/// Sets channel `name` of `target` to `value` (text), as one undo step.
pub fn set(e: &mut Editor, target: Target, name: &str, value: &str) -> Result<()> {
    let command = command_for(&e.scene, target, name, value)?;
    e.dispatch(command)
}

/// The one `Command` that sets channel `name` of `target` to `value`.
pub fn command_for(scene: &Scene, target: Target, name: &str, value: &str) -> Result<Command> {
    if let Some(command) = layer_field_command(scene, target, name, value) {
        return command;
    }
    if let Some(command) = output_field_command(scene, target, name, value) {
        return command;
    }
    if find(scene, target, name).is_none()
        && let Some(k) = kit(scene, target)
        && k.value_text(name).is_some()
    {
        return Err(k.needs_text());
    }
    let (base, part) = component(name);
    let channel = find(scene, target, base).ok_or_else(|| unknown(scene, target, name))?;
    let value = match part {
        Some(i) => {
            if channel.ty != ChannelType::Colour {
                return Err(format!("{base} isn't a colour"));
            }
            let Some(ChannelValue::Colour(mut c)) = (channel.get)(scene, target) else {
                return Err(format!("{base} has no value here"));
            };
            c[i] = value
                .trim()
                .parse()
                .map_err(|_| format!("{name} takes a number 0–1, not \"{value}\""))?;
            ChannelValue::Colour(c)
        }
        None => parse(channel.ty, channel.name, value)?,
    };
    (channel.set)(scene, target, value)
}

// ------------------------------------------------------------ layer fields

/// A shader-tree layer's own settings (what its Properties tab edits
/// beyond the common block): every field of the layer's kind, by its
/// stored name (`diffuse_amount`, nested ones with dots), as stored
/// (provisional: names and units are Operandi's own, to be matched to
/// Modo's). Tables above win: `enable`, `opacity` and the like.
fn layer_payload(scene: &Scene, target: Target) -> Option<(Node, String, serde_json::Value)> {
    let node = layer(scene, target)?;
    let (variant, payload) = kind_payload(&node.kind)?;
    Some((node, variant, payload))
}

/// A layer kind as (variant, settings), with the optional blocks that a
/// save leaves out at their defaults (a material's or shader's `more`, a
/// group's `scale`) filled in, so every setting has a name.
pub fn kind_payload(kind: &crate::shader_tree::NodeKind) -> Option<(String, serde_json::Value)> {
    use crate::shader_tree::{GroupScale, MaterialMore, ShaderMore};
    let serde_json::Value::Object(map) = serde_json::to_value(kind).ok()? else {
        return None;
    };
    let (variant, mut payload) = map.into_iter().next()?;
    let fill = |payload: &mut serde_json::Value, key: &str, default: serde_json::Value| {
        if let Some(object) = payload.as_object_mut() {
            object.entry(key).or_insert(default);
        }
    };
    match variant.as_str() {
        "Material" => {
            fill(
                &mut payload,
                "more",
                serde_json::to_value(MaterialMore::default()).ok()?,
            );
            fill(
                &mut payload,
                "openpbr",
                serde_json::to_value(crate::shader_tree::OpenPbrChannels::legacy()).ok()?,
            );
        }
        "Shader" => fill(
            &mut payload,
            "more",
            serde_json::to_value(ShaderMore::default()).ok()?,
        ),
        "Group" => fill(
            &mut payload,
            "scale",
            serde_json::to_value(GroupScale::default()).ok()?,
        ),
        _ => {}
    }
    Some((variant, payload))
}

/// Every leaf of `value` as (dotted path, leaf); lists are leaves.
pub fn json_leaves(
    prefix: &str,
    value: &serde_json::Value,
    out: &mut Vec<(String, serde_json::Value)>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                json_leaves(&path, v, out);
            }
        }
        _ => out.push((prefix.to_owned(), value.clone())),
    }
}

/// A layer's own field names (for the panel's edits and tests).
pub fn layer_fields(scene: &Scene, target: Target) -> Vec<String> {
    layer_payload(scene, target)
        .map(|(_, _, payload)| payload_leaves(&payload))
        .unwrap_or_default()
        .into_iter()
        .map(|(p, _)| p)
        .collect()
}

/// A layer kind's settings as (name, value): its fields, or `effect` for
/// a kind that is one value (a Render Output's effect).
fn payload_leaves(payload: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    let mut out = Vec::new();
    json_leaves("", payload, &mut out);
    for (path, _) in &mut out {
        if path.is_empty() {
            *path = "effect".into();
        }
    }
    out
}

fn layer_leaf(
    scene: &Scene,
    target: Target,
    name: &str,
) -> Option<(Node, String, serde_json::Value, String, serde_json::Value)> {
    if find(scene, target, name).is_some() {
        return None;
    }
    let (node, variant, payload) = layer_payload(scene, target)?;
    let (path, leaf) = payload_leaves(&payload)
        .into_iter()
        .find(|(p, _)| p.eq_ignore_ascii_case(name))?;
    Some((node, variant, payload, path, leaf))
}

fn leaf_text(leaf: &serde_json::Value) -> String {
    match leaf {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

fn layer_field_get(scene: &Scene, target: Target, name: &str) -> Option<Result<String>> {
    layer_leaf(scene, target, name).map(|(_, _, _, _, leaf)| Ok(leaf_text(&leaf)))
}

fn layer_field_command(
    scene: &Scene,
    target: Target,
    name: &str,
    value: &str,
) -> Option<Result<Command>> {
    let (mut node, variant, payload, path, old) = layer_leaf(scene, target, name)?;
    // The value as the field's own JSON: numbers (whole ones for whole
    // fields), true/false, lists as a,b,c, else the text.
    let text = value.trim();
    let list = || {
        let parts: Option<Vec<serde_json::Value>> = text
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|p| serde_json::from_str::<serde_json::Value>(p.trim()).ok())
            .collect();
        parts.map(serde_json::Value::Array)
    };
    let tries: Vec<serde_json::Value> = match &old {
        serde_json::Value::Array(_) => list().into_iter().collect(),
        serde_json::Value::Bool(_) => crate::command_args::boolean(name, text)
            .map(serde_json::Value::Bool)
            .into_iter()
            .collect(),
        serde_json::Value::Number(n) if n.is_u64() || n.is_i64() => text
            .parse::<f64>()
            .ok()
            .filter(|f| f.fract() == 0.)
            .map(|f| serde_json::Value::from(f as i64))
            .into_iter()
            .collect(),
        serde_json::Value::Number(_) => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .into_iter()
            .collect(),
        _ => vec![serde_json::Value::String(text.to_owned())],
    };
    let kind = tries.into_iter().find_map(|leaf| {
        let mut payload = payload.clone();
        if payload.is_object() {
            let mut at = &mut payload;
            for part in path.split('.') {
                at = at.as_object_mut()?.get_mut(part)?;
            }
            *at = leaf;
        } else {
            payload = leaf;
        }
        serde_json::from_value::<crate::shader_tree::NodeKind>(
            serde_json::json!({ variant.clone(): payload }),
        )
        .ok()
    });
    Some(match kind {
        Some(kind) => {
            node.kind = kind;
            Ok(Command::ShaderTree(ShaderTreeEdit::Replace {
                node,
                continuing: false,
            }))
        }
        None => Err(format!(
            "{name} doesn't take \"{value}\" (it is {})",
            leaf_text(&old)
        )),
    })
}

// ------------------------------------------------------------ render outputs

/// A Render Output layer's settings (Bloom, Exposure, Tone Map…), kept
/// with the tree's outputs rather than on the node: every field by its
/// stored name, as stored (provisional names and units).
fn output_settings(scene: &Scene, target: Target) -> Option<(NodeId, serde_json::Value)> {
    let Target::Layer(id) = target else {
        return None;
    };
    let tree = scene.render_shader_tree();
    let node = tree.get(id)?;
    if !matches!(node.kind, crate::shader_tree::NodeKind::RenderOutput(_)) {
        return None;
    }
    let settings = tree.outputs.get(&id).cloned().unwrap_or_default();
    Some((id, settings.channel_json()))
}

/// A Render Output layer's setting names.
pub fn output_fields(scene: &Scene, target: Target) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((_, value)) = output_settings(scene, target) {
        json_leaves("", &value, &mut out);
    }
    out.into_iter().map(|(p, _)| p).collect()
}

type OutputLeaf = (NodeId, serde_json::Value, String, serde_json::Value);

fn output_leaf(scene: &Scene, target: Target, name: &str) -> Option<OutputLeaf> {
    if find(scene, target, name).is_some() {
        return None;
    }
    let (id, value) = output_settings(scene, target)?;
    let mut all = Vec::new();
    json_leaves("", &value, &mut all);
    let (path, leaf) = all
        .into_iter()
        .find(|(p, _)| p.eq_ignore_ascii_case(name))?;
    Some((id, value, path, leaf))
}

fn output_field_get(scene: &Scene, target: Target, name: &str) -> Option<Result<String>> {
    output_leaf(scene, target, name).map(|(_, _, _, leaf)| Ok(leaf_text(&leaf)))
}

fn output_field_command(
    scene: &Scene,
    target: Target,
    name: &str,
    value: &str,
) -> Option<Result<Command>> {
    let (id, settings, path, old) = output_leaf(scene, target, name)?;
    let built = json_value_tries(name, value, &old)
        .into_iter()
        .find_map(|leaf| {
            let mut settings = settings.clone();
            let mut at = &mut settings;
            for part in path.split('.') {
                at = at.as_object_mut()?.get_mut(part)?;
            }
            *at = leaf;
            serde_json::from_value::<crate::render_outputs::OutputSettings>(settings).ok()
        });
    Some(match built {
        Some(settings) => Ok(Command::ShaderTree(ShaderTreeEdit::SetOutput {
            node: id,
            settings: Box::new(settings),
            continuing: false,
        })),
        None => Err(format!(
            "{name} doesn't take \"{value}\" (it is {})",
            leaf_text(&old)
        )),
    })
}

/// `value` as candidate JSON for a field that is `old` now: numbers (whole
/// ones for whole fields), true/false, lists as a,b,c, else the text.
fn json_value_tries(name: &str, value: &str, old: &serde_json::Value) -> Vec<serde_json::Value> {
    let text = value.trim();
    match old {
        serde_json::Value::Array(_) => {
            let parts: Option<Vec<serde_json::Value>> = text
                .trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|p| serde_json::from_str::<serde_json::Value>(p.trim()).ok())
                .collect();
            parts.map(serde_json::Value::Array).into_iter().collect()
        }
        serde_json::Value::Bool(_) => crate::command_args::boolean(name, text)
            .map(serde_json::Value::Bool)
            .into_iter()
            .collect(),
        serde_json::Value::Number(n) if n.is_u64() || n.is_i64() => text
            .parse::<f64>()
            .ok()
            .filter(|f| f.fract() == 0.)
            .map(|f| serde_json::Value::from(f as i64))
            .into_iter()
            .collect(),
        serde_json::Value::Number(_) => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .into_iter()
            .collect(),
        _ => vec![serde_json::Value::String(text.to_owned())],
    }
}

// ------------------------------------------------------------ helpers

fn light(scene: &Scene, target: Target) -> Option<&crate::camera_light::LightSettings> {
    match target {
        Target::Item(id) => scene.item(id)?.light.as_ref(),
        _ => None,
    }
}

fn is_light(scene: &Scene, target: Target) -> bool {
    light(scene, target).is_some()
}

fn light_edit(target: Target, edit: LightEdit) -> Result<Command> {
    match target {
        Target::Item(item) => Ok(Command::EditLight { item, edit }),
        _ => Err("Not a light".into()),
    }
}

fn float(v: ChannelValue) -> Result<f64> {
    match v {
        ChannelValue::Float(f) => Ok(f),
        _ => Err("Expected a number".into()),
    }
}

fn layer(scene: &Scene, target: Target) -> Option<Node> {
    match target {
        Target::Layer(id) => scene.render_shader_tree().get(id).cloned(),
        _ => None,
    }
}

fn is_layer(scene: &Scene, target: Target) -> bool {
    layer(scene, target).is_some()
}

/// A layer changed by `edit`, as one Replace of the shader tree.
fn layer_edit(scene: &Scene, target: Target, edit: impl FnOnce(&mut Node)) -> Result<Command> {
    let mut node = layer(scene, target).ok_or("Not a shader layer")?;
    edit(&mut node);
    Ok(Command::ShaderTree(ShaderTreeEdit::Replace {
        node,
        continuing: false,
    }))
}

const SHADOWS: &[&str] = &["none", "raytrace"];
/// `RotationOrder::ALL`'s labels, in its order.
const ROTATION_ORDERS: &[&str] = &["XYZ", "ZXY", "XZY", "YXZ", "YZX", "ZYX"];

fn item(scene: &Scene, target: Target) -> Option<&crate::scene::Item> {
    match target {
        Target::Item(id) => scene.item(id),
        _ => None,
    }
}

// ------------------------------------------------------------ every item

/// An item with a transform: every kind but environments
/// (ENVIRONMENT-PLACEMENT §2).
fn has_transform(scene: &Scene, target: Target) -> bool {
    item(scene, target).is_some_and(|i| i.environment.is_none())
}

/// Channels every item with a transform has (all but environments).
static ITEM: &[Channel] = &[Channel {
    // Item Transform > Rotation Order; changing it keeps the angles.
    name: "rotOrder",
    ty: ChannelType::Enum(ROTATION_ORDERS),
    applies: has_transform,
    get: |s, t| {
        item(s, t).map(|i| {
            let order = i.transform.rotation_order();
            let index = crate::scene::RotationOrder::ALL
                .iter()
                .position(|o| *o == order);
            ChannelValue::Enum(index.unwrap_or(0))
        })
    },
    set: |s, t, v| {
        let i = item(s, t).ok_or("Not an item")?;
        match v {
            ChannelValue::Enum(order) => {
                let order = *crate::scene::RotationOrder::ALL
                    .get(order)
                    .ok_or("Expected XYZ, ZXY, XZY, YXZ, YZX or ZYX")?;
                let mut transform = i.transform.clone();
                transform.set_rotation_order(order);
                Ok(Command::ItemTransform {
                    item: i.id,
                    transform,
                })
            }
            _ => Err("Expected XYZ, ZXY, XZY, YXZ, YZX or ZYX".into()),
        }
    },
}];
const RENDERS: &[&str] = &["default", "on", "off"];

// ------------------------------------------------------------ lights

const FALLOFFS: &[&str] = &["None", "Inverse Distance", "Inverse Distance Squared"];
const SHAPES: &[&str] = &["Rectangle", "Ellipse"];
const BLENDS: &[&str] = &["Normal", "Add", "Subtract", "Multiply", "Screen", "Overlay"];

/// A light channel that is one LightEdit of one field.
macro_rules! light_channel {
    ($name:literal, Bool, $($field:ident).+, $edit:path) => {
        Channel {
            name: $name,
            ty: ChannelType::Bool,
            applies: is_light,
            get: |s, t| light(s, t).map(|l| ChannelValue::Bool(l.$($field).+)),
            set: |_, t, v| match v {
                ChannelValue::Bool(b) => light_edit(t, $edit(b)),
                _ => Err("Expected true or false".into()),
            },
        }
    };
    ($name:literal, $ty:ident, $($field:ident).+, $edit:path) => {
        Channel {
            name: $name,
            ty: ChannelType::$ty,
            applies: is_light,
            get: |s, t| light(s, t).map(|l| ChannelValue::Float(l.$($field).+)),
            set: |_, t, v| light_edit(t, $edit(float(v)?)),
        }
    };
    (enum $name:literal, $names:expr, $all:expr, $($field:ident).+, $edit:path) => {
        Channel {
            name: $name,
            ty: ChannelType::Enum($names),
            applies: is_light,
            get: |s, t| {
                light(s, t).map(|l| {
                    ChannelValue::Enum($all.iter().position(|&x| x == l.$($field).+).unwrap_or(0))
                })
            },
            set: |_, t, v| match v {
                ChannelValue::Enum(i) => {
                    light_edit(t, $edit(*$all.get(i).ok_or("Unknown choice")?))
                }
                _ => Err("Expected a choice".into()),
            },
        }
    };
}

/// Light item channels (Light Properties). One of each value type the
/// lights have, more to come with the stage 4 backfill.
static LIGHT: &[Channel] = &[
    light_channel!(enum "falloff", FALLOFFS, LightFalloff::ALL, falloff, LightEdit::Falloff),
    light_channel!("spread", Angle, spread_angle, LightEdit::SpreadAngle),
    light_channel!("length", Distance, length, LightEdit::Length),
    light_channel!("width", Distance, width, LightEdit::Width),
    light_channel!("height", Distance, height, LightEdit::Height),
    light_channel!(enum "shape", SHAPES, AreaShape::ALL, shape, LightEdit::Shape),
    light_channel!(
        "envOnly",
        Bool,
        environment_only,
        LightEdit::EnvironmentOnly
    ),
    light_channel!(
        "visReflection",
        Bool,
        stored.visible_to_reflection,
        LightEdit::VisibleToReflection
    ),
    light_channel!(
        "visRefraction",
        Bool,
        stored.visible_to_refraction,
        LightEdit::VisibleToRefraction
    ),
    light_channel!("coneAngle", Angle, cone_angle, LightEdit::ConeAngle),
    light_channel!("softEdge", Angle, soft_edge, LightEdit::SoftEdge),
    light_channel!("diffuse", Percent, affect_diffuse, LightEdit::AffectDiffuse),
    light_channel!(
        "specular",
        Percent,
        affect_specular,
        LightEdit::AffectSpecular
    ),
    light_channel!(
        "importance",
        Percent,
        stored.importance,
        LightEdit::Importance
    ),
    light_channel!(
        "usePrototypeTransform",
        Bool,
        use_prototype_transform,
        LightEdit::UsePrototypeTransform
    ),
    light_channel!(enum "blendMode", BLENDS, LightBlend::ALL, blend_mode, LightEdit::BlendMode),
    // The light material's Enable and Opacity (stored, LM-01, LM-04).
    light_channel!(
        "enable",
        Bool,
        stored.material_enabled,
        LightEdit::MaterialEnabled
    ),
    light_channel!("opacity", Percent, stored.opacity, LightEdit::Opacity),
    Channel {
        name: "intensity",
        ty: ChannelType::Float,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Float(l.intensity)),
        set: |_, t, v| light_edit(t, LightEdit::Intensity(float(v)?)),
    },
    Channel {
        name: "color",
        ty: ChannelType::Colour,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Colour(l.color)),
        set: |_, t, v| match v {
            ChannelValue::Colour(c) => light_edit(t, LightEdit::Color(c)),
            _ => Err("Expected r,g,b".into()),
        },
    },
    Channel {
        name: "visCamera",
        ty: ChannelType::Bool,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Bool(l.visible_to_camera)),
        set: |_, t, v| match v {
            ChannelValue::Bool(b) => light_edit(t, LightEdit::VisibleToCamera(b)),
            _ => Err("Expected true or false".into()),
        },
    },
    Channel {
        name: "shadowType",
        ty: ChannelType::Enum(SHADOWS),
        applies: is_light,
        get: |s, t| {
            light(s, t).map(|l| {
                ChannelValue::Enum(
                    ShadowType::ALL
                        .iter()
                        .position(|&x| x == l.shadow)
                        .unwrap_or(0),
                )
            })
        },
        set: |_, t, v| match v {
            ChannelValue::Enum(i) => light_edit(
                t,
                LightEdit::Shadow(*ShadowType::ALL.get(i).ok_or("Unknown shadow type")?),
            ),
            _ => Err("Expected a shadow type".into()),
        },
    },
    Channel {
        name: "render",
        ty: ChannelType::Enum(RENDERS),
        applies: is_light,
        get: |s, t| {
            light(s, t).map(|l| {
                ChannelValue::Enum(match l.render {
                    LightRender::Default => 0,
                    LightRender::On => 1,
                    LightRender::Off => 2,
                })
            })
        },
        set: |_, t, v| match v {
            ChannelValue::Enum(i) => light_edit(
                t,
                LightEdit::Render(
                    [LightRender::Default, LightRender::On, LightRender::Off]
                        .get(i)
                        .copied()
                        .ok_or("Unknown render setting")?,
                ),
            ),
            _ => Err("Expected a render setting".into()),
        },
    },
    Channel {
        name: "radius",
        ty: ChannelType::Distance,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Float(l.radius)),
        set: |_, t, v| light_edit(t, LightEdit::Radius(float(v)?)),
    },
    Channel {
        // Fraction 0–1, like the panel's Dissolve.
        name: "dissolve",
        ty: ChannelType::Percent,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Float(l.dissolve)),
        set: |_, t, v| light_edit(t, LightEdit::Dissolve(float(v)?)),
    },
    Channel {
        name: "lpeLabel",
        ty: ChannelType::Text,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Text(l.lpe_label.clone())),
        set: |_, t, v| match v {
            ChannelValue::Text(text) => light_edit(t, LightEdit::LpeLabel(text)),
            _ => Err("Expected text".into()),
        },
    },
    Channel {
        // Mesh Light's Prototype mesh.
        name: "prototype",
        ty: ChannelType::ItemRef,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Item(l.prototype)),
        set: |_, t, v| match v {
            ChannelValue::Item(id) => light_edit(t, LightEdit::Prototype(id)),
            _ => Err("Expected an item id".into()),
        },
    },
    Channel {
        // Volumetrics on or off (LP-40).
        name: "volumetric",
        ty: ChannelType::Bool,
        applies: is_light,
        get: |s, t| light(s, t).map(|l| ChannelValue::Bool(l.volumetrics.is_some())),
        set: |_, t, v| match v {
            ChannelValue::Bool(b) => {
                light_edit(t, LightEdit::Volumetric(VolumetricEdit::Enabled(b)))
            }
            _ => Err("Expected true or false".into()),
        },
    },
    Channel {
        // Volumetrics samples (LP-41).
        name: "volSamples",
        ty: ChannelType::Int,
        applies: is_light,
        get: |s, t| {
            light(s, t).map(|l| {
                ChannelValue::Int(l.volumetrics.as_ref().map_or(40, |v| i64::from(v.samples)))
            })
        },
        set: |_, t, v| match v {
            ChannelValue::Int(n) => light_edit(
                t,
                LightEdit::Volumetric(VolumetricEdit::Samples(
                    u32::try_from(n).map_err(|_| "volSamples must be 0 or more")?,
                )),
            ),
            _ => Err("Expected a whole number".into()),
        },
    },
];

// ------------------------------------------------------------ layers

const BLEND_NAMES: &[&str] = BlendMode::LABELS;

/// The common block every shader-tree layer has (Enable, Invert, Blend
/// Mode, Opacity). Layer-kind channels (Effect, material values) come in
/// their own tables.
static LAYER: &[Channel] = &[
    Channel {
        name: "enable",
        ty: ChannelType::Bool,
        applies: is_layer,
        get: |s, t| layer(s, t).map(|n| ChannelValue::Bool(n.layer.enable)),
        set: |s, t, v| match v {
            ChannelValue::Bool(b) => layer_edit(s, t, |n| n.layer.enable = b),
            _ => Err("Expected true or false".into()),
        },
    },
    Channel {
        name: "invert",
        ty: ChannelType::Bool,
        applies: is_layer,
        get: |s, t| layer(s, t).map(|n| ChannelValue::Bool(n.layer.invert)),
        set: |s, t, v| match v {
            ChannelValue::Bool(b) => layer_edit(s, t, |n| n.layer.invert = b),
            _ => Err("Expected true or false".into()),
        },
    },
    Channel {
        name: "blend",
        ty: ChannelType::Enum(BLEND_NAMES),
        applies: is_layer,
        get: |s, t| {
            layer(s, t).map(|n| {
                ChannelValue::Enum(
                    BlendMode::ALL
                        .iter()
                        .position(|&b| b == n.layer.blend)
                        .unwrap_or(0),
                )
            })
        },
        set: |s, t, v| match v {
            ChannelValue::Enum(i) => {
                let blend = *BlendMode::ALL.get(i).ok_or("Unknown blend mode")?;
                layer_edit(s, t, |n| n.layer.blend = blend)
            }
            _ => Err("Expected a blend mode".into()),
        },
    },
    Channel {
        name: "opacity",
        ty: ChannelType::Percent,
        applies: is_layer,
        get: |s, t| layer(s, t).map(|n| ChannelValue::Float(n.layer.opacity)),
        set: |s, t, v| {
            let o = float(v)?;
            if !(0.0..=1.0).contains(&o) {
                return Err("opacity must be 0 to 1 (or 0% to 100%)".into());
            }
            layer_edit(s, t, |n| n.layer.opacity = o)
        },
    },
];

/// The typed line that makes `edit` on light `item` (UIC-164/165): an
/// `item.channel`, or the map's own `item.setType` / `light.temperature`
/// / `light.mesh`. `None` for edits no command covers yet.
pub fn light_line(item: u64, edit: &LightEdit) -> Option<String> {
    let brace = |t: String| {
        if t.contains(' ') || t.is_empty() {
            format!("{{{t}}}")
        } else {
            t
        }
    };
    let pick =
        |names: &[&str], i: Option<usize>| i.and_then(|i| names.get(i)).map(|n| n.to_string());
    let (name, value) = match edit {
        LightEdit::Kind(kind) => {
            return Some(format!(
                "item.setType {} item:{item}",
                brace(kind.label().to_owned())
            ));
        }
        LightEdit::Temperature(k) => return Some(format!("light.temperature {k} item:{item}")),
        LightEdit::Prototype(mesh) => {
            let mesh = mesh.map_or_else(|| "none".to_owned(), |m| m.to_string());
            return Some(format!("light.mesh mesh:{mesh} light:{item}"));
        }
        LightEdit::Render(r) => (
            "render",
            pick(
                RENDERS,
                [LightRender::Default, LightRender::On, LightRender::Off]
                    .iter()
                    .position(|x| x == r),
            )?,
        ),
        LightEdit::Dissolve(v) => ("dissolve", v.to_string()),
        LightEdit::Intensity(v) => ("intensity", v.to_string()),
        LightEdit::Shadow(sh) => (
            "shadowType",
            pick(SHADOWS, ShadowType::ALL.iter().position(|x| x == sh))?,
        ),
        LightEdit::VisibleToCamera(b) => ("visCamera", b.to_string()),
        LightEdit::SpreadAngle(v) => ("spread", v.to_string()),
        LightEdit::Color(c) => ("color", format!("{},{},{}", c[0], c[1], c[2])),
        LightEdit::AffectDiffuse(v) => ("diffuse", v.to_string()),
        LightEdit::AffectSpecular(v) => ("specular", v.to_string()),
        LightEdit::Radius(v) => ("radius", v.to_string()),
        LightEdit::Falloff(f) => (
            "falloff",
            pick(FALLOFFS, LightFalloff::ALL.iter().position(|x| x == f))?,
        ),
        LightEdit::ConeAngle(v) => ("coneAngle", v.to_string()),
        LightEdit::SoftEdge(v) => ("softEdge", v.to_string()),
        LightEdit::Shape(sh) => (
            "shape",
            pick(SHAPES, AreaShape::ALL.iter().position(|x| x == sh))?,
        ),
        LightEdit::Width(v) => ("width", v.to_string()),
        LightEdit::Height(v) => ("height", v.to_string()),
        LightEdit::VisibleToReflection(b) => ("visReflection", b.to_string()),
        LightEdit::VisibleToRefraction(b) => ("visRefraction", b.to_string()),
        LightEdit::EnvironmentOnly(b) => ("envOnly", b.to_string()),
        LightEdit::Length(v) => ("length", v.to_string()),
        LightEdit::UsePrototypeTransform(b) => ("usePrototypeTransform", b.to_string()),
        LightEdit::LpeLabel(t) => ("lpeLabel", t.clone()),
        LightEdit::Importance(v) => ("importance", v.to_string()),
        LightEdit::MaterialEnabled(b) => ("enable", b.to_string()),
        LightEdit::Opacity(v) => ("opacity", v.to_string()),
        LightEdit::BlendMode(b) => (
            "blendMode",
            pick(BLENDS, LightBlend::ALL.iter().position(|x| x == b))?,
        ),
        _ => return None,
    };
    Some(format!("item.channel {name} {} item:{item}", brace(value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_exec::run;
    use serde_json::{Map, Value, json};

    fn args(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    /// An editor with a mesh, a Mesh Light (active) and the default
    /// shader tree; returns (editor, mesh id, light id).
    fn scene() -> (Editor, u64, u64) {
        let mut e = Editor::default();
        run(&mut e, "mesh.new", &Map::new()).unwrap();
        let mesh = e.scene.selection.item.unwrap();
        run(&mut e, "item.create", &args(json!({"type": "light.mesh"}))).unwrap();
        let light = e.scene.selection.item.unwrap();
        (e, mesh, light)
    }

    fn round_trip(e: &mut Editor, target: Target, name: &str, value: &str, back: &str) {
        let undo = e.undo.len();
        set(e, target, name, value).unwrap_or_else(|err| panic!("{name} {value}: {err}"));
        // One undo step (none when the value was already that).
        assert!(e.undo.len() <= undo + 1, "{name}: one undo step");
        assert_eq!(get(&e.scene, target, name).unwrap(), back, "{name}");
    }

    /// One channel of each value type, set and read back.
    #[test]
    fn every_value_type_sets_and_reads_back() {
        let (mut e, mesh, light) = scene();
        let l = Target::Item(light);
        let undo = e.undo.len();
        round_trip(&mut e, l, "intensity", "2.5", "2.5"); // float
        assert_eq!(e.undo.len(), undo + 1, "a change is one undo step");
        round_trip(&mut e, Target::Item(mesh), "rotOrder", "ZXY", "ZXY"); // every item
        round_trip(&mut e, l, "rotOrder", "0", "XYZ");
        round_trip(&mut e, l, "visCamera", "false", "false"); // bool
        assert!(
            set(&mut e, l, "volSamples", "12")
                .unwrap_err()
                .contains("Volumetrics")
        );
        round_trip(&mut e, l, "lpeLabel", "key", "key"); // text
        round_trip(&mut e, l, "shadowType", "none", "none"); // enum by name
        round_trip(&mut e, l, "render", "2", "off"); // enum by index
        round_trip(&mut e, l, "color", "1,0.5,0", "1,0.5,0"); // colour
        round_trip(&mut e, l, "color.g", "0.25", "0.25"); // colour part
        assert_eq!(get(&e.scene, l, "color").unwrap(), "1,0.25,0");
        round_trip(&mut e, l, "radius", "50cm", "0.5"); // distance
        round_trip(&mut e, l, "dissolve", "25%", "0.25"); // percent
        round_trip(&mut e, l, "dissolve", "0.5", "0.5");
        round_trip(&mut e, l, "prototype", &mesh.to_string(), &mesh.to_string()); // item ref
        round_trip(&mut e, l, "prototype", "none", "none");
        // Int: a Point Light's Volumetrics samples (Mesh Lights have none).
        run(&mut e, "item.create", &args(json!({"type": "light.point"}))).unwrap();
        let point = Target::Item(e.scene.selection.item.unwrap());
        round_trip(&mut e, point, "volumetric", "true", "true");
        round_trip(&mut e, point, "volSamples", "12", "12"); // int
        // A shader-tree layer's common block.
        let layer = Target::Layer(e.scene.shader_tree.nodes[3].id);
        round_trip(&mut e, layer, "enable", "off", "false");
        round_trip(&mut e, layer, "invert", "true", "true");
        round_trip(&mut e, layer, "blend", "Multiply", "Multiply");
        round_trip(&mut e, layer, "opacity", "40%", "0.4");
    }

    #[test]
    fn bad_names_and_values_change_nothing() {
        let (mut e, mesh, light) = scene();
        let before = e.scene.clone();
        let err = set(&mut e, Target::Item(light), "brightness", "1").unwrap_err();
        assert!(err.contains("intensity"), "{err}");
        assert!(set(&mut e, Target::Item(mesh), "intensity", "1").is_err());
        assert!(set(&mut e, Target::Item(mesh), "rotOrder", "XXY").is_err());
        assert!(set(&mut e, Target::Item(light), "visCamera", "maybe").is_err());
        assert!(set(&mut e, Target::Item(light), "shadowType", "soft").is_err());
        assert!(set(&mut e, Target::Item(light), "intensity.r", "1").is_err());
        let layer = Target::Layer(e.scene.shader_tree.nodes[3].id);
        assert!(set(&mut e, layer, "opacity", "2").is_err());
        assert_eq!(e.scene, before);
    }

    /// The command through the runner (MCP and typed lines): set, query,
    /// refusals.
    #[test]
    fn item_channel_runs_and_answers() {
        let (mut e, _, light) = scene();
        run(
            &mut e,
            "item.channel",
            &args(json!({"name": "intensity", "value": 3})),
        )
        .unwrap();
        let q = args(json!({"name": "intensity", "value": "?"}));
        assert_eq!(
            crate::command_exec::answer(&e, "item.channel", &q)
                .unwrap()
                .unwrap(),
            "3"
        );
        run(
            &mut e,
            "item.channel",
            &args(json!({"name": "color", "value": [0, 1, 0], "item": light})),
        )
        .unwrap();
        assert_eq!(
            get(&e.scene, Target::Item(light), "color").unwrap(),
            "0,1,0"
        );
        let layer = e.scene.shader_tree.nodes[3].id;
        run(
            &mut e,
            "item.channel",
            &args(json!({"name": "opacity", "value": 0.5, "layer": layer})),
        )
        .unwrap();
        assert!(
            run(
                &mut e,
                "item.channel",
                &args(json!({"name": "intensity", "value": 1, "mode": 1}))
            )
            .is_err()
        );
        assert!(run(&mut e, "item.channel", &args(json!({"name": "intensity"}))).is_err());
    }
}
