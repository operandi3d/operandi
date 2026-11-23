//! Rig pieces (BEHAVIOR-RIG-PHASE4, RG-xx): constraints, dynamic parents
//! and item pivots, as scene data.
//!
//! A piece controls one item (`item`) and reads others (`targets`). Its
//! settings are channels (AR-07): numbers in `values` under names such as
//! `influence`, `weight.0` or `offset.pos.X`, keyable like any channel
//! (`ChannelKey { item: piece.id, channel }`; piece ids come from the
//! scene's item id counter, so they never collide with items). Nothing a
//! piece computes is stored here or on the item: the evaluated result
//! lives in the engine (rig_eval.rs, eval_state.rs) and the item keeps its
//! own channels (ACS-API-31).
//!
//! A piece of a kind this build doesn't know loads as "missing": it is kept
//! and saved back untouched, and evaluates as if disabled (§8 Files).
//!
//! Pivots (RG-14) are kept here too, by item: rotation and scale happen
//! around them (`Scene::local_matrix`).
use glam::{DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The scene's rig: pieces in evaluation order per item, and pivots.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pieces: Vec<Piece>,
    /// Item pivots, by item id (saved as a list: JSON keys are text).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "pivot_list"
    )]
    pub pivots: BTreeMap<u64, Pivot>,
    /// User channels (RG-53), in the order added.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user: Vec<UserChannel>,
    /// On an evaluated copy (rig_eval::apply): the document scene it was
    /// made from, for exports that evaluate other times. Never saved.
    #[serde(skip)]
    pub source: Option<std::sync::Arc<crate::Scene>>,
}

/// A user channel's value type (RG-53).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserType {
    #[default]
    Float,
    Distance,
    Angle,
    Percentage,
    Time,
    Integer,
    Boolean,
    Choice,
    String,
    Divider,
}

impl UserType {
    pub fn parse(s: &str) -> Option<UserType> {
        Some(match s.to_ascii_lowercase().as_str() {
            "float" => UserType::Float,
            "distance" => UserType::Distance,
            "angle" => UserType::Angle,
            "percentage" | "percent" => UserType::Percentage,
            "time" => UserType::Time,
            "integer" | "int" => UserType::Integer,
            "boolean" | "bool" => UserType::Boolean,
            "choice" => UserType::Choice,
            "string" | "text" => UserType::String,
            "divider" => UserType::Divider,
            _ => return None,
        })
    }
    /// Holds a number (keyable, linkable).
    pub fn numeric(self) -> bool {
        !matches!(self, UserType::String | UserType::Divider)
    }
    pub fn name(self) -> &'static str {
        match self {
            UserType::Float => "float",
            UserType::Distance => "distance",
            UserType::Angle => "angle",
            UserType::Percentage => "percentage",
            UserType::Time => "time",
            UserType::Integer => "integer",
            UserType::Boolean => "boolean",
            UserType::Choice => "choice",
            UserType::String => "string",
            UserType::Divider => "divider",
        }
    }
}

/// A user channel's components (RG-53 Mode).
pub fn mode_components(mode: &str) -> Option<&'static [&'static str]> {
    Some(match mode.to_ascii_lowercase().as_str() {
        "scalar" | "" => &[""],
        "xy" | "vectorxy" => &["X", "Y"],
        "xyz" | "vectorxyz" => &["X", "Y", "Z"],
        "rgb" => &["R", "G", "B"],
        "rgba" => &["R", "G", "B", "A"],
        "uv" => &["U", "V"],
        "uvw" => &["U", "V", "W"],
        _ => return None,
    })
}

/// A user channel on an item (RG-53).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserChannel {
    pub item: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub display: String,
    #[serde(default)]
    pub ty: UserType,
    /// scalar, xy, xyz, rgb, rgba, uv or uvw.
    #[serde(default = "scalar", skip_serializing_if = "is_scalar")]
    pub mode: String,
    #[serde(default)]
    pub default: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Choice names (Choice type).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
    /// Values by component ("" for a scalar); missing = the default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f64>,
    /// A String channel's text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
}

fn scalar() -> String {
    "scalar".into()
}
fn is_scalar(m: &str) -> bool {
    m == "scalar"
}

impl UserChannel {
    /// The channel names it makes: `name` for a scalar, `name.X`... for
    /// the vector modes; none for a divider or a string.
    pub fn channel_names(&self) -> Vec<String> {
        if !self.ty.numeric() {
            return Vec::new();
        }
        mode_components(&self.mode)
            .unwrap_or(&[""])
            .iter()
            .map(|c| {
                if c.is_empty() {
                    self.name.clone()
                } else {
                    format!("{}.{c}", self.name)
                }
            })
            .collect()
    }
    /// Whether `channel` names this channel (or one of its components).
    pub fn has(&self, channel: &str) -> bool {
        self.component(channel).is_some() || (!self.ty.numeric() && channel == self.name)
    }
    fn component(&self, channel: &str) -> Option<String> {
        let comps = mode_components(&self.mode).unwrap_or(&[""]);
        if channel == self.name {
            return (comps == [""]).then(String::new);
        }
        let rest = channel.strip_prefix(&self.name)?.strip_prefix('.')?;
        comps.contains(&rest).then(|| rest.to_owned())
    }
    pub fn get(&self, channel: &str) -> Option<f64> {
        let c = self.component(channel)?;
        self.ty
            .numeric()
            .then(|| self.values.get(&c).copied().unwrap_or(self.default))
    }
    pub fn set(&mut self, channel: &str, value: f64) -> Result<(), String> {
        if !self.ty.numeric() {
            return Err(format!("{} holds text, not a number", self.name));
        }
        let c = self.component(channel).ok_or("No such user channel")?;
        let mut v = value;
        if let Some(lo) = self.min {
            v = v.max(lo);
        }
        if let Some(hi) = self.max {
            v = v.min(hi);
        }
        v = match self.ty {
            UserType::Integer | UserType::Choice => v.round(),
            UserType::Boolean => {
                if v >= 0.5 {
                    1.
                } else {
                    0.
                }
            }
            _ => v,
        };
        if v == self.default {
            self.values.remove(&c);
        } else {
            self.values.insert(c, v);
        }
        Ok(())
    }
}

impl Rig {
    /// The user channel holding `channel` on `item`.
    pub fn user_channel(&self, item: u64, channel: &str) -> Option<&UserChannel> {
        self.user.iter().find(|u| u.item == item && u.has(channel))
    }
    pub fn user_channel_mut(&mut self, item: u64, channel: &str) -> Option<&mut UserChannel> {
        self.user
            .iter_mut()
            .find(|u| u.item == item && u.has(channel))
    }
}

impl Rig {
    pub fn is_default(&self) -> bool {
        self.pieces.is_empty() && self.pivots.is_empty() && self.user.is_empty()
    }
    pub fn piece(&self, id: u64) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.id == id)
    }
    pub fn piece_mut(&mut self, id: u64) -> Option<&mut Piece> {
        self.pieces.iter_mut().find(|p| p.id == id)
    }
    /// The pieces on `item`, in evaluation order.
    pub fn on(&self, item: u64) -> impl Iterator<Item = &Piece> {
        self.pieces.iter().filter(move |p| p.item == item)
    }
}

/// An item's pivot: position and rotation (radians) in its parent's space,
/// relative to the item's position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pivot {
    #[serde(default)]
    pub position: DVec3,
    #[serde(default)]
    pub rotation: DVec3,
}

impl Pivot {
    /// `local` with rotation and scale about the pivot:
    /// T · P · Rp · (R · S) · Rp⁻¹ · P⁻¹, where `local` = T · R · S.
    pub fn apply(&self, local: DMat4) -> DMat4 {
        if *self == Pivot::default() {
            return local;
        }
        let t = DMat4::from_translation(local.w_axis.truncate());
        let rs = DMat4::from_cols(local.x_axis, local.y_axis, local.z_axis, glam::DVec4::W);
        let p = DMat4::from_translation(self.position);
        let rp = DMat4::from_quat(DQuat::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        ));
        t * p * rp * rs * rp.inverse() * p.inverse()
    }
}

/// One rig piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub id: u64,
    /// `position`, `rotation`, `scale`, `transform`, `direction`,
    /// `distance`, `path`, `pathPosition`, `pathNormal`, `dynamicParent`,
    /// or a kind this build doesn't know (kept as missing).
    pub kind: String,
    /// The controlled item.
    pub item: u64,
    /// Items it reads, in order (targets; parents for a dynamic parent,
    /// where index 0 is World and target `i` is parent index `i + 1`;
    /// curves for a path).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<u64>,
    /// Channel values that differ from the kind's defaults.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f64>,
    /// Item references other than targets (`upItem`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub refs: BTreeMap<String, u64>,
    /// Channel names a piece reads or writes (`driven`, `driver` for a
    /// link) and text settings (`mode`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub names: BTreeMap<String, String>,
    /// A relationship link's curve (RG-52): the driver's value on the
    /// horizontal axis (as seconds), the driven value on the vertical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<crate::anim_curve::Curve>,
    /// Fields a newer build wrote, saved back untouched.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The kinds this build evaluates.
pub const KINDS: &[&str] = &[
    "position",
    "rotation",
    "scale",
    "transform",
    "direction",
    "distance",
    "path",
    "pathPosition",
    "pathNormal",
    "dynamicParent",
    "link",
    "mod.expression",
    "mod.blend",
    "mod.math",
    "mod.clamp",
    "mod.switch",
];

/// Modifier kinds (RG-60..62), in menu order.
pub const MODIFIERS: &[&str] = &[
    "mod.expression",
    "mod.blend",
    "mod.math",
    "mod.clamp",
    "mod.switch",
];

/// Math operations (RG-62).
pub const MATH_OPS: &[&str] = &[
    "add", "subtract", "multiply", "divide", "min", "max", "power",
];

/// Constraint kinds (RG-26..46), in menu order.
pub const CONSTRAINTS: &[&str] = &[
    "position",
    "rotation",
    "scale",
    "transform",
    "direction",
    "distance",
    "path",
    "pathPosition",
    "pathNormal",
];

/// The kind's display name.
pub fn label(kind: &str) -> String {
    match kind {
        "position" => "Position Constraint".into(),
        "rotation" => "Rotation Constraint".into(),
        "scale" => "Scale Constraint".into(),
        "transform" => "Transform Constraint".into(),
        "direction" => "Direction Constraint".into(),
        "distance" => "Distance Constraint".into(),
        "path" => "Path Constraint".into(),
        "pathPosition" => "Path Position Constraint".into(),
        "pathNormal" => "Path Normal Constraint".into(),
        "dynamicParent" => "Dynamic Parent".into(),
        "link" => "Channel Link".into(),
        "mod.expression" => "Expression".into(),
        "mod.blend" => "Linear Blend".into(),
        "mod.math" => "Math".into(),
        "mod.clamp" => "Clamp".into(),
        "mod.switch" => "Switch".into(),
        other => format!("{other} (missing)"),
    }
}

const XYZ: [&str; 3] = ["X", "Y", "Z"];

/// A channel's default; `None` for a name the kind doesn't have. Weights
/// (`weight.<n>`) default to 1 for every target.
pub fn default_value(kind: &str, name: &str) -> Option<f64> {
    let common = match name {
        "enable" | "influence" => Some(1.),
        "usePivots" => Some(0.),
        _ => None,
    };
    if common.is_some() {
        return common;
    }
    if let Some(axis) = name.strip_prefix("affect.") {
        return XYZ.contains(&axis).then_some(1.);
    }
    if let Some(i) = name.strip_prefix("weight.") {
        return i.parse::<usize>().ok().map(|_| 1.);
    }
    for (group, default) in [
        ("offset.pos.", 0.),
        ("offset.rot.", 0.),
        ("offset.scl.", 1.),
    ] {
        if let Some(axis) = name.strip_prefix(group) {
            return XYZ.contains(&axis).then_some(default);
        }
    }
    let up = |axis: &str| match axis {
        "Y" => Some(1.),
        "X" | "Z" => Some(0.),
        _ => None,
    };
    match (kind, name) {
        ("direction" | "path" | "pathNormal", "axis") => Some(2.),
        ("direction" | "path" | "pathNormal", "negative" | "roll") => Some(0.),
        ("direction" | "path" | "pathNormal", n) if n.starts_with("up.") => {
            n.strip_prefix("up.").and_then(up)
        }
        ("direction", "upAxis") => Some(-1.),
        ("distance", "distance") => Some(1.),
        ("distance", "clamp") => Some(0.),
        ("path" | "pathPosition" | "pathNormal", "percent" | "pathOffset" | "wrap") => Some(0.),
        ("path" | "pathNormal", "localUp") => Some(0.),
        ("dynamicParent", "parent") => Some(0.),
        ("mod.blend", "blend") => Some(0.),
        ("mod.clamp", "min") => Some(0.),
        ("mod.clamp", "max") => Some(1.),
        ("mod.switch", "index") => Some(0.),
        _ => None,
    }
}

/// The channel names a piece shows, in order: its own settings first,
/// then a weight per target (RG-27: weights at the end).
pub fn channel_names(piece: &Piece) -> Vec<String> {
    let mut out: Vec<String> = vec!["enable".into()];
    fn axes(group: &str) -> Vec<String> {
        XYZ.iter().map(|a| format!("{group}{a}")).collect()
    }
    match piece.kind.as_str() {
        "dynamicParent" => {
            out.push("parent".into());
            out.extend(axes("offset.pos."));
            out.extend(axes("offset.rot."));
            out.extend(axes("offset.scl."));
            return out;
        }
        "link" => return out,
        "mod.blend" => {
            out.push("blend".into());
            return out;
        }
        "mod.clamp" => {
            out.extend(["min", "max"].map(String::from));
            return out;
        }
        "mod.switch" => {
            out.push("index".into());
            return out;
        }
        "mod.expression" | "mod.math" => return out,
        k if !KINDS.contains(&k) => return Vec::new(),
        _ => {}
    }
    out.push("influence".into());
    out.extend(axes("affect."));
    out.push("usePivots".into());
    match piece.kind.as_str() {
        "position" | "distance" | "pathPosition" => out.extend(axes("offset.pos.")),
        "rotation" | "direction" | "pathNormal" => out.extend(axes("offset.rot.")),
        "scale" => out.extend(axes("offset.scl.")),
        "transform" | "path" => {
            out.extend(axes("offset.pos."));
            out.extend(axes("offset.rot."));
            out.extend(axes("offset.scl."));
        }
        _ => {}
    }
    match piece.kind.as_str() {
        "direction" => {
            out.extend(["axis", "negative", "roll"].map(String::from));
            out.extend(axes("up."));
            out.push("upAxis".into());
        }
        "distance" => out.extend(["distance", "clamp"].map(String::from)),
        "path" | "pathNormal" | "pathPosition" => {
            out.extend(["percent", "pathOffset", "wrap"].map(String::from));
            if piece.kind != "pathPosition" {
                out.extend(["axis", "negative", "roll"].map(String::from));
                out.extend(axes("up."));
                out.push("localUp".into());
            }
        }
        _ => {}
    }
    out.extend((0..piece.targets.len()).map(|i| format!("weight.{i}")));
    out
}

impl Piece {
    pub fn new(id: u64, kind: &str, item: u64, targets: Vec<u64>) -> Piece {
        Piece {
            id,
            kind: kind.into(),
            item,
            targets,
            values: BTreeMap::new(),
            refs: BTreeMap::new(),
            names: BTreeMap::new(),
            curve: None,
            extra: serde_json::Map::new(),
        }
    }
    /// A kind this build doesn't know (§8 Files).
    pub fn is_missing(&self) -> bool {
        !KINDS.contains(&self.kind.as_str())
    }
    /// A channel's value (its default when not set); `None` for a name
    /// the piece doesn't have.
    pub fn get(&self, name: &str) -> Option<f64> {
        if let Some(v) = self.values.get(name) {
            return Some(*v);
        }
        if let Some(i) = name.strip_prefix("weight.") {
            return i
                .parse::<usize>()
                .ok()
                .filter(|i| *i < self.targets.len())
                .map(|_| 1.);
        }
        default_value(&self.kind, name)
    }
    /// The value or 0 (for names known to exist).
    pub fn num(&self, name: &str) -> f64 {
        self.get(name).unwrap_or(0.)
    }
    pub fn flag(&self, name: &str) -> bool {
        self.num(name) >= 0.5
    }
    pub fn vec3(&self, group: &str) -> DVec3 {
        DVec3::new(
            self.num(&format!("{group}X")),
            self.num(&format!("{group}Y")),
            self.num(&format!("{group}Z")),
        )
    }
    /// Sets a channel, refusing names the piece doesn't have and values
    /// out of range. A value equal to the default is dropped from
    /// `values`, so files only carry what changed.
    pub fn set(&mut self, name: &str, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("Not a number".into());
        }
        if !channel_names(self).iter().any(|n| n == name) {
            return Err(format!("{} has no channel {name}", label(&self.kind)));
        }
        let value = match name {
            "enable" | "usePivots" | "negative" | "wrap" | "localUp" => {
                if value >= 0.5 {
                    1.
                } else {
                    0.
                }
            }
            n if n.starts_with("affect.") => {
                if value >= 0.5 {
                    1.
                } else {
                    0.
                }
            }
            "influence" => value.clamp(0., 1.),
            "axis" => (value.round()).clamp(0., 2.),
            "clamp" => (value.round()).clamp(0., 2.),
            "upAxis" => (value.round()).clamp(-1., 2.),
            "parent" => (value.round()).clamp(0., self.targets.len() as f64),
            n if n.starts_with("weight.") => value.max(0.),
            "distance" => value.max(0.),
            _ => value,
        };
        if default_value(&self.kind, name) == Some(value)
            || (name.starts_with("weight.") && value == 1.)
        {
            self.values.remove(name);
        } else {
            self.values.insert(name.into(), value);
        }
        Ok(())
    }
    /// The piece's offset as a matrix (position, rotation XYZ radians,
    /// scale).
    pub fn offset_matrix(&self) -> DMat4 {
        let r = self.vec3("offset.rot.");
        DMat4::from_scale_rotation_translation(
            self.vec3("offset.scl."),
            DQuat::from_euler(glam::EulerRot::XYZ, r.x, r.y, r.z),
            self.vec3("offset.pos."),
        )
    }
    /// Sets the offset channels from a matrix (compensation).
    pub fn set_offset_matrix(&mut self, m: DMat4) {
        let (s, q, t) = m.to_scale_rotation_translation();
        let (rx, ry, rz) = q.to_euler(glam::EulerRot::XYZ);
        for (i, a) in XYZ.iter().enumerate() {
            let _ = self.set(&format!("offset.pos.{a}"), clean(t[i]));
            let _ = self.set(&format!("offset.rot.{a}"), clean([rx, ry, rz][i]));
            let _ = self.set(&format!("offset.scl.{a}"), clean(s[i]));
        }
    }
}

/// Rounds away float noise from a decomposition (1e-12).
pub fn clean(v: f64) -> f64 {
    let r = (v * 1e9).round() / 1e9;
    if (r - v).abs() < 1e-12 { r } else { v }
}

/// Channel reads and writes for piece and pivot channels (animation.rs
/// routes `ChannelKey`s here first). `None`: not a rig channel.
pub fn read(scene: &crate::Scene, item: u64, name: &str) -> Option<Option<f64>> {
    if let Some(piece) = scene.animation.rig.piece(item) {
        return Some(piece.get(name));
    }
    if let Some(u) = scene.animation.rig.user_channel(item, name) {
        return Some(u.get(name));
    }
    if name.starts_with("world.") || name.starts_with("local.") {
        return Some(matrix_channel(scene, item, name));
    }
    let (group, axis) = pivot_part(name)?;
    scene.item(item)?;
    let p = scene
        .animation
        .rig
        .pivots
        .get(&item)
        .copied()
        .unwrap_or_default();
    Some(Some(if group == "pos" {
        p.position[axis]
    } else {
        p.rotation[axis]
    }))
}

/// See [`read`]; `None` when not a rig channel.
pub fn write(
    scene: &mut crate::Scene,
    item: u64,
    name: &str,
    value: f64,
) -> Option<Result<(), String>> {
    if let Some(piece) = scene.animation.rig.piece_mut(item) {
        return Some(piece.set(name, value));
    }
    if let Some(u) = scene.animation.rig.user_channel_mut(item, name) {
        return Some(u.set(name, value));
    }
    if name.starts_with("world.") || name.starts_with("local.") {
        return Some(Err(format!("{name} is read-only (an output, RG-63)")));
    }
    let (group, axis) = pivot_part(name)?;
    scene.item(item)?;
    let pivots = &mut scene.animation.rig.pivots;
    let p = pivots.entry(item).or_default();
    if group == "pos" {
        p.position[axis] = value;
    } else {
        p.rotation[axis] = value;
    }
    if *p == Pivot::default() {
        pivots.remove(&item);
    }
    Some(Ok(()))
}

/// `pivot.pos.X` → ("pos", 0).
pub fn pivot_part(name: &str) -> Option<(&'static str, usize)> {
    let rest = name.strip_prefix("pivot.")?;
    let (group, axis) = rest.split_once('.')?;
    let group = match group {
        "pos" => "pos",
        "rot" => "rot",
        _ => return None,
    };
    Some((group, XYZ.iter().position(|a| *a == axis)?))
}

/// The pivot channels every item has (RG-14).
pub const PIVOT_CHANNELS: [&str; 6] = [
    "pivot.pos.X",
    "pivot.pos.Y",
    "pivot.pos.Z",
    "pivot.rot.X",
    "pivot.rot.Y",
    "pivot.rot.Z",
];

/// Whether `name` on the item is driven by one of its enabled pieces
/// (RG-04), and by which.
pub fn driven_by<'a>(scene: &'a crate::Scene, item: u64, name: &str) -> Option<&'a Piece> {
    // A channel link or modifier output on exactly this channel (RG-51).
    if let Some(p) = scene
        .animation
        .rig
        .on(item)
        .find(|p| p.flag("enable") && p.names.get("driven").is_some_and(|d| d == name))
    {
        return Some(p);
    }
    let (group, axis) = crate::animation::transform_part(name)?;
    scene.animation.rig.on(item).find(|p| {
        !p.is_missing()
            && p.flag("enable")
            && match p.kind.as_str() {
                "dynamicParent" | "transform" | "path" => true,
                "position" | "distance" | "pathPosition" => group == "pos",
                "rotation" | "direction" | "pathNormal" => group == "rot",
                "scale" => group == "scl",
                _ => false,
            }
            && (p.kind == "dynamicParent" || p.flag(&format!("affect.{}", XYZ[axis])))
    })
}

/// Whether `item` feeds a piece (the driver light, RG-04).
pub fn drives(scene: &crate::Scene, item: u64) -> bool {
    scene
        .animation
        .rig
        .pieces
        .iter()
        .any(|p| p.targets.contains(&item) || p.refs.values().any(|r| *r == item))
}

/// Whether `item`'s channel `name` feeds a link (the driver light, RG-04).
pub fn drives_channel(scene: &crate::Scene, item: u64, name: &str) -> bool {
    scene.animation.rig.pieces.iter().any(|p| {
        p.kind == "link"
            && p.targets.first() == Some(&item)
            && p.names.get("driver").is_some_and(|d| d == name)
    })
}

mod pivot_list {
    use super::Pivot;
    use glam::DVec3;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry {
        item: u64,
        #[serde(default)]
        position: DVec3,
        #[serde(default)]
        rotation: DVec3,
    }
    pub fn serialize<S: Serializer>(m: &BTreeMap<u64, Pivot>, s: S) -> Result<S::Ok, S::Error> {
        m.iter()
            .map(|(&item, p)| Entry {
                item,
                position: p.position,
                rotation: p.rotation,
            })
            .collect::<Vec<_>>()
            .serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<u64, Pivot>, D::Error> {
        Ok(Vec::<Entry>::deserialize(d)?
            .into_iter()
            .map(|e| {
                (
                    e.item,
                    Pivot {
                        position: e.position,
                        rotation: e.rotation,
                    },
                )
            })
            .collect())
    }
}

/// A modifier's wired inputs, by input name (`A`, `B`, `Blend`, an
/// expression's variable), in name order: `names` entries `in.<name>` =
/// `item:channel`.
pub fn modifier_inputs(p: &Piece) -> Vec<(String, crate::animation::ChannelKey)> {
    p.names
        .iter()
        .filter_map(|(k, v)| {
            let name = k.strip_prefix("in.")?;
            Some((
                name.to_owned(),
                crate::animation::ChannelKey::parse(v).ok()?,
            ))
        })
        .collect()
}

/// A modifier's output (RG-60..62) from its inputs' values (`value(name)`,
/// `None` when unwired). Unwired inputs read 0, as a syntax error does
/// (RG-60: the output holds 0).
pub fn modifier_value(
    p: &Piece,
    value: &dyn Fn(&str) -> Option<f64>,
    scene: &crate::Scene,
) -> Option<f64> {
    let v = |n: &str| value(n).unwrap_or(0.);
    Some(match p.kind.as_str() {
        "mod.blend" => {
            let t = value("Blend")
                .unwrap_or_else(|| p.num("blend"))
                .clamp(0., 1.);
            v("A") + (v("B") - v("A")) * t
        }
        "mod.math" => {
            let (a, b) = (v("A"), v("B"));
            match p.names.get("op").map(String::as_str).unwrap_or("add") {
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => a / b,
                "min" => a.min(b),
                "max" => a.max(b),
                "power" => a.powf(b),
                _ => a + b,
            }
        }
        "mod.clamp" => {
            let lo = value("Min").unwrap_or_else(|| p.num("min"));
            let hi = value("Max").unwrap_or_else(|| p.num("max"));
            v("A").max(lo.min(hi)).min(hi.max(lo))
        }
        "mod.switch" => {
            let index = value("Index")
                .unwrap_or_else(|| p.num("index"))
                .round()
                .max(0.) as usize;
            let mut names: Vec<String> = modifier_inputs(p)
                .into_iter()
                .map(|(n, _)| n)
                .filter(|n| n != "Index")
                .collect();
            names.sort();
            names.get(index).map_or(0., |n| v(n))
        }
        "mod.expression" => expression_value(p, value, scene),
        _ => return None,
    })
}

/// RG-60: the expression with its inputs, `time` (seconds) and `frame`
/// built in; a syntax error gives 0.
fn expression_value(p: &Piece, value: &dyn Fn(&str) -> Option<f64>, scene: &crate::Scene) -> f64 {
    let Some(source) = p.names.get("expr") else {
        return 0.;
    };
    let Ok(expr) = crate::rig_expression::Expr::parse(source) else {
        return 0.;
    };
    let t = scene.time.current;
    let frame = crate::animation::frames(scene.time.rate, t);
    expr.eval(&|n| match n {
        "time" => Some(t.seconds()),
        "frame" => Some(frame),
        other => value(other),
    })
}

/// RG-63: every item's world and local transform as read-only channels
/// (`world.pos.X`, `local.rot.Y`...), so a link or modifier can follow an
/// item without a script. Worlds are the document's (before the rig), so
/// reading them never makes a loop.
pub fn matrix_channel(scene: &crate::Scene, item: u64, name: &str) -> Option<f64> {
    let (space, rest) = name.split_once('.')?;
    let (group, axis) = rest.split_once('.')?;
    let axis = XYZ.iter().position(|a| *a == axis)?;
    let item_ref = scene.item(item)?;
    let m = match space {
        "world" => scene.world_matrix(item).ok()?,
        "local" => scene.local_matrix(item_ref),
        _ => return None,
    };
    let (s, q, t) = m.to_scale_rotation_translation();
    Some(match group {
        "pos" => t[axis],
        "rot" => {
            let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
            [x, y, z][axis]
        }
        "scl" => s[axis],
        _ => return None,
    })
}

/// The matrix output channels every item has (RG-63).
pub fn matrix_channel_names() -> Vec<String> {
    let mut out = Vec::new();
    for space in ["world", "local"] {
        for group in ["pos", "rot", "scl"] {
            for a in XYZ {
                out.push(format!("{space}.{group}.{a}"));
            }
        }
    }
    out
}
