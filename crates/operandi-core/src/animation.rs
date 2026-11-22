//! Animation data and the pose at a time (BEHAVIOR-TIME-KEYS TK-01..53,
//! BEHAVIOR-TIMELINE TL-13/50..54): which channels hold key curves, the
//! scene's time settings beyond `SceneTime`, the markers, auto key and the
//! "edited, not keyed" pending edits.
//!
//! ## Where values live (rest / curve / evaluated)
//!
//! Each animated channel is an [`AnimChannel`] beside the item: its key
//! [`Curve`] and its **rest** value, the channel's own setup value from
//! before it was first keyed. The item's field (an item's
//! `transform.translation.x`, a light's intensity…) holds the **evaluated**
//! value at the current time, which is what every tool, panel, render and
//! export reads and edits (TK-50/51). Posing ([`pose`]) writes curve values
//! into those fields only; it never writes the rest value, and nothing
//! writes a curve value into the rest value. A field that differs from its
//! curve at the current time is a **pending edit** (TK-40/41): it sits in
//! the field beside the curve, and the next time change poses it away
//! unless Apply keys it. Rigging's later setup mode can show and edit the
//! rest values without a change of layout. [`evaluate_at`] gives the pose
//! at any other time as a copy, leaving the document scene alone.
//!
//! Channel names: transforms are `pos.X`, `rot.Y`, `scl.Z` (the map's
//! `transform.channel` names; rotations stored in radians, keyed per Euler
//! axis in the item's order, TK-15); every other keyable channel is an
//! `item.channel` name (colours by component, `color.r`).
use crate::anim_curve::{Curve, Key};
use crate::channels::{self, ChannelType, ChannelValue, Target};
use crate::time_tick::{FrameRate, Tick};
use crate::{Result, Scene};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One animated scalar channel: an item and a channel name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChannelKey {
    pub item: u64,
    pub channel: String,
}

impl ChannelKey {
    pub fn new(item: u64, channel: &str) -> Self {
        ChannelKey {
            item,
            channel: channel.to_owned(),
        }
    }
    /// `item:channel`, as commands take it.
    pub fn text(&self) -> String {
        format!("{}:{}", self.item, self.channel)
    }
    pub fn parse(text: &str) -> Result<Self> {
        let (item, channel) = text
            .trim()
            .split_once(':')
            .ok_or_else(|| format!("A channel is item:name, not \"{text}\""))?;
        let item = item
            .trim()
            .parse()
            .map_err(|_| format!("A channel is item:name, not \"{text}\""))?;
        Ok(ChannelKey::new(item, channel.trim()))
    }
    /// The transform group (`pos`, `rot`, `scl`) and axis, for those.
    pub fn transform_part(&self) -> Option<(&str, usize)> {
        transform_part(&self.channel)
    }
}

/// The nine transform channel names, position first.
pub const TRANSFORM_CHANNELS: [&str; 9] = [
    "pos.X", "pos.Y", "pos.Z", "rot.X", "rot.Y", "rot.Z", "scl.X", "scl.Y", "scl.Z",
];

fn transform_part(name: &str) -> Option<(&str, usize)> {
    let (group, axis) = name.split_once('.')?;
    let group = ["pos", "rot", "scl"]
        .into_iter()
        .find(|g| g.eq_ignore_ascii_case(group))?;
    let axis = match axis {
        "X" | "x" => 0,
        "Y" | "y" => 1,
        "Z" | "z" => 2,
        _ => return None,
    };
    Some((group, axis))
}

/// A channel's keys and its rest (setup) value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimChannel {
    pub curve: Curve,
    /// The channel's value before it was first keyed (its setup value).
    /// Posing never writes it; Reset and later rig setup mode read it.
    pub rest: f64,
}

/// Auto Key (TK-30).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoKey {
    Off,
    /// Edits to channels that already have keys key them.
    #[default]
    Animated,
    /// Every edit to an item's channel keys it.
    All,
}

impl AutoKey {
    pub const NAMES: [&'static str; 3] = ["off", "animated", "all"];
    pub fn parse(word: &str) -> Option<AutoKey> {
        match word.to_ascii_lowercase().as_str() {
            "off" => Some(AutoKey::Off),
            "animated" => Some(AutoKey::Animated),
            "all" => Some(AutoKey::All),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Off", "Animated", "All"][self as usize]
    }
}

/// How times are written (TK-03 Time Display).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeDisplay {
    #[default]
    Frames,
    Seconds,
    Smpte,
    FilmCode,
}

impl TimeDisplay {
    pub const NAMES: [&'static str; 4] = ["frames", "seconds", "smpte", "film"];
    pub fn parse(word: &str) -> Option<TimeDisplay> {
        Self::NAMES
            .iter()
            .position(|n| n.eq_ignore_ascii_case(word))
            .map(|i| [Self::Frames, Self::Seconds, Self::Smpte, Self::FilmCode][i])
    }
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Frames", "Seconds", "SMPTE", "Film Code"][self as usize]
    }
}

/// What playback does at the end of its range (TK-62).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndBehavior {
    Hold,
    #[default]
    Repeat,
    Wrap,
}

impl EndBehavior {
    pub const NAMES: [&'static str; 3] = ["hold", "repeat", "wrap"];
    pub fn parse(word: &str) -> Option<EndBehavior> {
        Self::NAMES
            .iter()
            .position(|n| n.eq_ignore_ascii_case(word))
            .map(|i| [Self::Hold, Self::Repeat, Self::Wrap][i])
    }
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Hold", "Repeat", "Wrap"][self as usize]
    }
}

fn yes() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}
fn is_false(b: &bool) -> bool {
    !*b
}

/// The scene's time settings beyond rate and range (TK-03). Channels of
/// the Scene item (AR-07), saved with the scene; changing them is one undo
/// step (TK-04).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeSettings {
    #[serde(default, skip_serializing_if = "is_default_display")]
    pub display: TimeDisplay,
    /// The current range (a working window inside the scene range);
    /// `None` follows the scene range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_range: Option<(Tick, Tick)>,
    #[serde(default, skip_serializing_if = "is_default_auto")]
    pub auto_key: AutoKey,
    /// Current time and dragged keys land on whole frames.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub snap: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub real_time: bool,
    #[serde(default, skip_serializing_if = "is_default_end")]
    pub end: EndBehavior,
    /// With Auto Key Off, an edit made exactly on a key updates it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_update_keys: bool,
}

fn is_default_display(d: &TimeDisplay) -> bool {
    *d == TimeDisplay::Frames
}
fn is_default_auto(a: &AutoKey) -> bool {
    *a == AutoKey::Animated
}
fn is_default_end(e: &EndBehavior) -> bool {
    *e == EndBehavior::Repeat
}

impl Default for TimeSettings {
    fn default() -> Self {
        TimeSettings {
            display: TimeDisplay::Frames,
            current_range: None,
            auto_key: AutoKey::Animated,
            snap: true,
            real_time: true,
            end: EndBehavior::Repeat,
            auto_update_keys: false,
        }
    }
}

/// A marker's look (TL-52).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerShape {
    #[default]
    Triangle,
    Square,
    Diamond,
    Bar,
}

impl MarkerShape {
    pub const NAMES: [&'static str; 4] = ["triangle", "square", "diamond", "bar"];
    pub fn parse(word: &str) -> Option<MarkerShape> {
        Self::NAMES
            .iter()
            .position(|n| n.eq_ignore_ascii_case(word))
            .map(|i| [Self::Triangle, Self::Square, Self::Diamond, Self::Bar][i])
    }
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

fn is_zero_tick(t: &Tick) -> bool {
    *t == Tick::ZERO
}
fn default_marker_color() -> [f32; 3] {
    [0.95, 0.75, 0.2]
}
fn is_default_marker_color(c: &[f32; 3]) -> bool {
    *c == default_marker_color()
}
fn is_default_shape(s: &MarkerShape) -> bool {
    *s == MarkerShape::Triangle
}

/// A time marker or range marker (TL-50..54). Scene data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub id: u64,
    pub time: Tick,
    /// A range marker's length; zero for a plain marker.
    #[serde(default, skip_serializing_if = "is_zero_tick")]
    pub length: Tick,
    /// The item it belongs to; `None` is the scene (TL-51).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_label: bool,
    #[serde(default, skip_serializing_if = "is_default_shape")]
    pub shape: MarkerShape,
    #[serde(
        default = "default_marker_color",
        skip_serializing_if = "is_default_marker_color"
    )]
    pub color: [f32; 3],
    /// Restricts playback and stepping to the range while the current time
    /// is inside it (TL-53).
    #[serde(default, skip_serializing_if = "is_false")]
    pub frame_limits: bool,
}

impl Marker {
    pub fn end(&self) -> Tick {
        self.time + self.length
    }
    pub fn is_range(&self) -> bool {
        self.length > Tick::ZERO
    }
}

/// Everything animated in a scene (saved; native version 60).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    /// Animated channels, saved as a list (JSON keys are text).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "channel_list"
    )]
    pub channels: BTreeMap<ChannelKey, AnimChannel>,
    #[serde(default, skip_serializing_if = "is_default_settings")]
    pub settings: TimeSettings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    /// GE-80: shadow copies of curves, by channel (saved; native v61).
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "shadow_list"
    )]
    pub shadows: BTreeMap<ChannelKey, Curve>,
}

mod shadow_list {
    use super::ChannelKey;
    use crate::anim_curve::Curve;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry {
        item: u64,
        channel: String,
        curve: Curve,
    }
    pub fn serialize<S: Serializer>(
        map: &BTreeMap<ChannelKey, Curve>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter()
            .map(|(k, c)| Entry {
                item: k.item,
                channel: k.channel.clone(),
                curve: c.clone(),
            })
            .collect::<Vec<_>>()
            .serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<ChannelKey, Curve>, D::Error> {
        Ok(Vec::<Entry>::deserialize(d)?
            .into_iter()
            .map(|e| {
                (
                    ChannelKey {
                        item: e.item,
                        channel: e.channel,
                    },
                    e.curve,
                )
            })
            .collect())
    }
}

impl Animation {
    /// Anything phase 3 added to the file (behaviours, cycles, shadows):
    /// saved as native version 61. Weights, manual and broken handles were
    /// already in version 60's keys.
    pub fn has_graph_editor_data(&self) -> bool {
        !self.shadows.is_empty()
            || self
                .channels
                .values()
                .any(|c| !c.curve.pre.is_constant() || !c.curve.post.is_constant() || c.curve.cycle)
    }
}

fn is_default_settings(s: &TimeSettings) -> bool {
    *s == TimeSettings::default()
}

mod channel_list {
    use super::{AnimChannel, ChannelKey};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry {
        item: u64,
        channel: String,
        #[serde(flatten)]
        anim: AnimChannelFields,
    }
    #[derive(Serialize, Deserialize)]
    struct AnimChannelFields {
        curve: crate::anim_curve::Curve,
        rest: f64,
    }

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<ChannelKey, AnimChannel>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let list: Vec<Entry> = map
            .iter()
            .map(|(k, v)| Entry {
                item: k.item,
                channel: k.channel.clone(),
                anim: AnimChannelFields {
                    curve: v.curve.clone(),
                    rest: v.rest,
                },
            })
            .collect();
        list.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<ChannelKey, AnimChannel>, D::Error> {
        let list = Vec::<Entry>::deserialize(d)?;
        Ok(list
            .into_iter()
            .map(|e| {
                (
                    ChannelKey {
                        item: e.item,
                        channel: e.channel,
                    },
                    AnimChannel {
                        curve: e.anim.curve,
                        rest: e.anim.rest,
                    },
                )
            })
            .collect())
    }
}

impl Animation {
    pub fn is_default(&self) -> bool {
        *self == Animation::default()
    }
    /// The channels of item `id` that have keys.
    pub fn item_channels(&self, id: u64) -> impl Iterator<Item = (&ChannelKey, &AnimChannel)> {
        self.channels
            .range(ChannelKey::new(id, "")..)
            .take_while(move |(k, _)| k.item == id)
    }
    pub fn is_animated(&self, key: &ChannelKey) -> bool {
        self.channels.get(key).is_some_and(|c| !c.curve.is_empty())
    }
    pub fn item_is_animated(&self, id: u64) -> bool {
        self.item_channels(id).next().is_some()
    }
    /// Every key time of these items (all items when `None`), sorted.
    pub fn key_times(&self, items: Option<&[u64]>) -> Vec<Tick> {
        let mut out: Vec<Tick> = self
            .channels
            .iter()
            .filter(|(k, _)| items.is_none_or(|ids| ids.contains(&k.item)))
            .flat_map(|(_, c)| c.curve.times())
            .collect();
        out.sort();
        out.dedup();
        out
    }
    pub fn validate(&self) -> Result<()> {
        for (k, c) in &self.channels {
            if !c.rest.is_finite() || c.curve.keys.iter().any(|key| !key.value.is_finite()) {
                return Err(format!(
                    "Channel {} has a value that isn't a number",
                    k.text()
                ));
            }
            if c.curve.keys.windows(2).any(|w| w[0].time >= w[1].time) {
                return Err(format!("Channel {}'s keys are out of order", k.text()));
            }
        }
        if let Some((a, b)) = self.settings.current_range
            && a > b
        {
            return Err("The current range's start is after its end".into());
        }
        Ok(())
    }
}

// ------------------------------------------------------------ channel values

/// Whether `name` is a channel of item `id` that can hold keys (TK-10): a
/// transform axis, or a number, on/off, choice or colour component channel
/// of its `item.channel` tables whose setter posing can apply.
pub fn keyable(scene: &Scene, id: u64, name: &str) -> bool {
    if transform_part(name).is_some() {
        return read(scene, &ChannelKey::new(id, name)).is_some();
    }
    let (base, part) = split_component(name);
    let target = Target::Item(id);
    let Some(channel) = channels::find(scene, target, base) else {
        return false;
    };
    let numeric = match (channel.ty, part) {
        (ChannelType::Colour, Some(_)) => true,
        (ChannelType::Text | ChannelType::ItemRef | ChannelType::Colour, _) | (_, Some(_)) => false,
        _ => true,
    };
    // Its setter's command is one posing can apply (`apply_quietly`).
    numeric
        && (channel.get)(scene, target)
            .and_then(|v| (channel.set)(scene, target, v).ok())
            .is_some_and(|c| posable(&c))
}

/// The commands posing writes straight into the scene.
fn posable(c: &crate::Command) -> bool {
    use crate::Command;
    matches!(
        c,
        Command::ItemTransform { .. }
            | Command::EditLight { .. }
            | Command::EditCamera { .. }
            | Command::EditItemCommon { .. }
    )
}

/// The keyable channel a property row's edit `command` sets (UIC-112):
/// the item and channel name (a colour gives its three components). The
/// app's shared row helpers ask this so every keyable row gets its light.
pub fn channel_of_command(scene: &Scene, command: &crate::Command) -> Option<(u64, Vec<String>)> {
    use crate::Command;
    let item = match command {
        Command::EditLight { item, .. } | Command::EditCamera { item, .. } => *item,
        Command::EditItemCommon { items, .. } => *items.first()?,
        _ => return None,
    };
    // A setter's command and the row's are the same edit when their debug
    // text matches up to the edit's value.
    // The edit's variant path (`Extra/DrawOptions/Enable`, `Intensity`):
    // the names that open a bracket, values left out.
    fn path(debug: &str) -> String {
        let mut out = Vec::new();
        let mut word = String::new();
        for ch in debug.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                word.push(ch);
                continue;
            }
            if (ch == '(' || ch == '{') && word.starts_with(|c: char| c.is_uppercase()) {
                out.push(std::mem::take(&mut word));
            } else if ch != ' ' {
                if !word.is_empty() && out.is_empty() {
                    // A bare unit variant: its own name.
                    out.push(std::mem::take(&mut word));
                }
                if ch != ' ' && !out.is_empty() && !word.is_empty() {
                    break;
                }
            }
            word.clear();
        }
        if out.is_empty() && !word.is_empty() {
            out.push(word);
        }
        out.join("/")
    }
    let shape = |c: &Command| match c {
        Command::EditLight { edit, .. } => Some(format!("light/{}", path(&format!("{edit:?}")))),
        Command::EditCamera { edit, .. } => Some(format!("camera/{}", path(&format!("{edit:?}")))),
        Command::EditItemCommon { edit, .. } => {
            Some(format!("item/{}", path(&format!("{edit:?}"))))
        }
        _ => None,
    };
    let wanted = shape(command)?;
    let target = Target::Item(item);
    channels::TABLES
        .iter()
        .flat_map(|t| t.iter())
        .filter(|c| (c.applies)(scene, target))
        .find(|c| {
            (c.get)(scene, target)
                .and_then(|v| (c.set)(scene, target, v).ok())
                .is_some_and(|cmd| shape(&cmd).as_ref() == Some(&wanted))
        })
        .and_then(|c| {
            let names = if c.ty == ChannelType::Colour {
                ["r", "g", "b"].map(|p| format!("{}.{p}", c.name)).to_vec()
            } else {
                vec![c.name.to_string()]
            };
            names
                .iter()
                .all(|n| keyable(scene, item, n))
                .then_some((item, names))
        })
}

/// Every keyable channel of item `id`, transforms first.
pub fn keyable_channels(scene: &Scene, id: u64) -> Vec<String> {
    let Some(item) = scene.item(id) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    if item.environment.is_none() {
        out.extend(TRANSFORM_CHANNELS.iter().map(|s| s.to_string()));
    }
    for c in channels::TABLES.iter().flat_map(|t| t.iter()) {
        if !(c.applies)(scene, Target::Item(id)) || c.name == "rotOrder" {
            continue;
        }
        match c.ty {
            ChannelType::Colour => {
                for part in ["r", "g", "b"] {
                    out.push(format!("{}.{part}", c.name));
                }
            }
            ChannelType::Text | ChannelType::ItemRef => {}
            _ => out.push(c.name.to_string()),
        }
    }
    out.retain(|n| transform_part(n).is_some() || keyable(scene, id, n));
    out.dedup();
    out
}

/// Choice and on/off channels are stepped (TK-14).
pub fn is_stepped(scene: &Scene, key: &ChannelKey) -> bool {
    if transform_part(&key.channel).is_some() {
        return false;
    }
    let base = key.channel.split('.').next().unwrap_or("");
    channels::find(scene, Target::Item(key.item), base).is_some_and(|c| {
        matches!(
            c.ty,
            ChannelType::Bool | ChannelType::Enum(_) | ChannelType::Int
        )
    })
}

/// The default value Reset puts back (TK-24): transforms identity, other
/// channels their rest value.
pub fn default_value(scene: &Scene, key: &ChannelKey) -> f64 {
    match key.transform_part() {
        Some(("scl", _)) => 1.,
        Some(_) => 0.,
        None => scene
            .animation
            .channels
            .get(key)
            .map_or_else(|| read(scene, key).unwrap_or(0.), |c| c.rest),
    }
}

/// The value channel `key` holds now (its field: the evaluated value plus
/// any pending edit).
pub fn read(scene: &Scene, key: &ChannelKey) -> Option<f64> {
    let item = scene.item(key.item)?;
    if let Some((group, axis)) = transform_part(&key.channel) {
        if item.environment.is_some() {
            return None;
        }
        let t = &item.transform;
        return Some(match group {
            "pos" => t.translation[axis],
            "rot" => t.rotation[axis],
            _ => t.scale[axis],
        });
    }
    let (base, part) = split_component(&key.channel);
    let target = Target::Item(key.item);
    let channel = channels::find(scene, target, base)?;
    let value = (channel.get)(scene, target)?;
    match (value, part) {
        (ChannelValue::Colour(c), Some(i)) => Some(c[i]),
        (ChannelValue::Colour(_), None) | (_, Some(_)) => None,
        (ChannelValue::Float(f), None) => Some(f),
        (ChannelValue::Int(n), None) => Some(n as f64),
        (ChannelValue::Bool(b), None) => Some(if b { 1. } else { 0. }),
        (ChannelValue::Enum(i), None) => Some(i as f64),
        _ => None,
    }
}

fn split_component(name: &str) -> (&str, Option<usize>) {
    match name.rsplit_once('.') {
        Some((base, "r" | "R")) => (base, Some(0)),
        Some((base, "g" | "G")) => (base, Some(1)),
        Some((base, "b" | "B")) => (base, Some(2)),
        _ => (name, None),
    }
}

/// Writes `value` into channel `key`'s field, with no undo step (posing).
/// Refuses channels whose setter isn't one posing can apply.
pub fn write(scene: &mut Scene, key: &ChannelKey, value: f64) -> Result<()> {
    if !value.is_finite() {
        return Err("Not a number".into());
    }
    if let Some((group, axis)) = transform_part(&key.channel) {
        let item = scene.item_mut(key.item).ok_or("Missing item")?;
        let t = &mut item.transform;
        match group {
            "pos" => t.translation[axis] = value,
            "rot" => t.rotation[axis] = value,
            _ => t.scale[axis] = value,
        }
        return Ok(());
    }
    let (base, part) = split_component(&key.channel);
    let target = Target::Item(key.item);
    let channel = channels::find(scene, target, base).ok_or("No such channel")?;
    let typed = match (channel.ty, part) {
        (ChannelType::Colour, Some(i)) => {
            let Some(ChannelValue::Colour(mut c)) = (channel.get)(scene, target) else {
                return Err("No colour here".into());
            };
            c[i] = value;
            ChannelValue::Colour(c)
        }
        (ChannelType::Bool, None) => ChannelValue::Bool(value >= 0.5),
        (ChannelType::Int, None) => ChannelValue::Int(value.round() as i64),
        (ChannelType::Enum(names), None) => {
            ChannelValue::Enum((value.round().max(0.) as usize).min(names.len().saturating_sub(1)))
        }
        (ChannelType::Text | ChannelType::ItemRef | ChannelType::Colour, _) | (_, Some(_)) => {
            return Err("This channel can't hold keys".into());
        }
        _ => ChannelValue::Float(value),
    };
    let command = (channel.set)(scene, target, typed)?;
    apply_quietly(scene, command)
}

/// Applies a channel setter's command straight to the scene (no undo),
/// for the commands channel setters make.
fn apply_quietly(scene: &mut Scene, command: crate::Command) -> Result<()> {
    use crate::Command;
    match command {
        Command::ItemTransform { item, transform } => {
            scene.item_mut(item).ok_or("Missing item")?.transform = transform;
            Ok(())
        }
        Command::EditLight { item, edit } => scene
            .item_mut(item)
            .and_then(|i| i.light.as_mut())
            .ok_or("Not a light")?
            .apply(edit),
        Command::EditCamera { item, edit } => scene
            .item_mut(item)
            .and_then(|i| i.camera.as_mut())
            .ok_or("Not a camera")?
            .apply(edit),
        Command::EditItemCommon { items, edit } => scene.apply_item_common(&items, &edit),
        _ => Err("This channel can't hold keys yet".into()),
    }
}

// ------------------------------------------------------------ posing

/// Writes every animated channel's value at `time` into its field
/// (dropping pending edits, TK-40). Rest values are never touched.
pub fn pose(scene: &mut Scene, time: Tick) {
    if scene.animation.channels.is_empty() {
        return;
    }
    let values: Vec<(ChannelKey, f64)> = scene
        .animation
        .channels
        .iter()
        .filter(|(_, c)| !c.curve.is_empty())
        .map(|(k, c)| (k.clone(), c.curve.value_at(time)))
        .collect();
    for (k, v) in values {
        // A channel whose item went away (or can't be posed) is skipped.
        let _ = write(scene, &k, v);
    }
}

/// `pose` at the scene's own current time.
pub fn repose(scene: &mut Scene) {
    let now = scene.time.current;
    pose(scene, now);
}

/// The scene posed at `time`, a copy (AR-01): render ranges and exports
/// read other times without touching the document. Borrowed when nothing
/// is animated or `time` is the current time.
pub fn evaluate_at(scene: &Scene, time: Tick) -> std::borrow::Cow<'_, Scene> {
    if scene.animation.channels.is_empty() || time == scene.time.current {
        return std::borrow::Cow::Borrowed(scene);
    }
    let mut posed = scene.clone();
    pose(&mut posed, time);
    posed.time.current = time;
    std::borrow::Cow::Owned(posed)
}

/// Channels whose field differs from their curve now: edited, not keyed
/// (TK-40/41).
pub fn pending(scene: &Scene) -> Vec<ChannelKey> {
    let now = scene.time.current;
    scene
        .animation
        .channels
        .iter()
        .filter(|(_, c)| !c.curve.is_empty())
        .filter(|(k, c)| read(scene, k).is_some_and(|v| differs(v, c.curve.value_at(now))))
        .map(|(k, _)| k.clone())
        .collect()
}

/// Whether channel `key` holds a pending edit.
pub fn is_pending(scene: &Scene, key: &ChannelKey) -> bool {
    scene.animation.channels.get(key).is_some_and(|c| {
        !c.curve.is_empty()
            && read(scene, key).is_some_and(|v| differs(v, c.curve.value_at(scene.time.current)))
    })
}

pub(crate) fn differs(a: f64, b: f64) -> bool {
    (a - b).abs() > 1e-9 * a.abs().max(b.abs()).max(1.)
}

/// After undo or redo put back `restored`: the current time stays where it
/// was (`left` is the scene being left), and the scene is posed there
/// when the restored step was made at another time (TK-05).
pub fn keep_time(left: &Scene, restored: &mut Scene) {
    let now = left.time.current;
    if restored.time.current != now {
        restored.time.current = now;
        pose(restored, now);
    }
}

// ------------------------------------------------------------ time maths

/// `t` in frames (fractional) at `rate`.
pub fn frames(rate: FrameRate, t: Tick) -> f64 {
    t.seconds() * rate.as_f64()
}

/// The time at `frames` (fractional) at `rate`; whole frames are exact.
pub fn at_frames(rate: FrameRate, frames: f64) -> Tick {
    if frames.fract() == 0. && frames.abs() < 1e12 {
        rate.tick_of(frames as i64)
    } else {
        Tick::from_seconds(frames / rate.as_f64())
    }
}

/// `t` on the nearest whole frame.
pub fn nearest_frame(rate: FrameRate, t: Tick) -> Tick {
    rate.tick_of(frames(rate, t).round() as i64)
}

/// A time written in `display` (TK-03): `24`, `1.0s`, `00:00:01:00`,
/// `1+08` (feet + frames, 16 frames a foot).
pub fn time_text(display: TimeDisplay, rate: FrameRate, t: Tick) -> String {
    let f = frames(rate, t);
    let whole = f.round();
    match display {
        TimeDisplay::Frames => {
            if (f - whole).abs() < 1e-6 {
                format!("{}", whole as i64)
            } else {
                format!("{f:.2}")
            }
        }
        TimeDisplay::Seconds => format!("{:.2}s", t.seconds()),
        TimeDisplay::Smpte => {
            let fps = rate.as_f64().round().max(1.) as i64;
            let n = whole as i64;
            let sign = if n < 0 { "-" } else { "" };
            let n = n.abs();
            let (ff, s) = (n % fps, n / fps);
            format!(
                "{sign}{:02}:{:02}:{:02}:{:02}",
                s / 3600,
                (s / 60) % 60,
                s % 60,
                ff
            )
        }
        TimeDisplay::FilmCode => {
            let n = whole as i64;
            let sign = if n < 0 { "-" } else { "" };
            let n = n.abs();
            format!("{sign}{}+{:02}", n / 16, n % 16)
        }
    }
}

/// A typed time in `display` (or any of the forms `time_text` writes).
pub fn parse_time(display: TimeDisplay, rate: FrameRate, text: &str) -> Result<Tick> {
    let text = text.trim();
    let bad = || format!("\"{text}\" isn't a time");
    if let Some(s) = text.strip_suffix('s') {
        let seconds: f64 = s.trim().parse().map_err(|_| bad())?;
        return Ok(Tick::from_seconds(seconds));
    }
    if text.contains(':') {
        let parts: Vec<i64> = text
            .split(':')
            .map(|p| p.trim().parse::<i64>())
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| bad())?;
        let fps = rate.as_f64().round().max(1.) as i64;
        let mut n = 0i64;
        let (secs, ff) = parts.split_at(parts.len() - 1);
        for p in secs {
            n = n * 60 + p;
        }
        return Ok(rate.tick_of(n * fps + ff[0]));
    }
    if let Some((feet, ff)) = text.split_once('+') {
        let feet: i64 = feet.trim().parse().map_err(|_| bad())?;
        let ff: i64 = ff.trim().parse().map_err(|_| bad())?;
        return Ok(rate.tick_of(feet * 16 + ff));
    }
    let value: f64 = text.parse().map_err(|_| bad())?;
    if !value.is_finite() || value.abs() > 1e9 {
        return Err(bad());
    }
    Ok(match display {
        TimeDisplay::Seconds => Tick::from_seconds(value),
        _ => at_frames(rate, value),
    })
}

/// The scene range and the current range (TK-03).
pub fn scene_range(scene: &Scene) -> (Tick, Tick) {
    (scene.time.start, scene.time.end)
}
pub fn current_range(scene: &Scene) -> (Tick, Tick) {
    scene
        .animation
        .settings
        .current_range
        .unwrap_or((scene.time.start, scene.time.end))
}

/// The range a time step or playback keeps to at `t`: a Frame Limits
/// range marker holding `t` (TL-53), else `None`.
pub fn limit_range(scene: &Scene, t: Tick) -> Option<(Tick, Tick)> {
    let selected = scene.selected_items();
    scene
        .animation
        .markers
        .iter()
        .filter(|m| m.frame_limits && m.is_range())
        .filter(|m| m.owner.is_none_or(|o| selected.contains(&o)))
        .find(|m| m.time <= t && t <= m.end())
        .map(|m| (m.time, m.end()))
}

/// `t` moved `n` frames; inside a Frame Limits range it wraps round at
/// the ends (rolling a pose, TL-53).
pub fn step_frames(scene: &Scene, t: Tick, n: i64) -> Tick {
    let rate = scene.time.rate;
    let base = if scene.animation.settings.snap {
        nearest_frame(rate, t)
    } else {
        t
    };
    let moved = at_frames(rate, frames(rate, base) + n as f64);
    match limit_range(scene, t) {
        Some((a, b)) => {
            let (fa, fb) = (
                frames(rate, a).round() as i64,
                frames(rate, b).round() as i64,
            );
            let span = fb - fa + 1;
            let f = frames(rate, moved).round() as i64;
            if span > 0 && (f < fa || f > fb) {
                rate.tick_of(fa + (f - fa).rem_euclid(span))
            } else {
                moved
            }
        }
        None => moved,
    }
}

/// One playback step (TK-60..62): `t` moved `frames` (whole frames when
/// Time Snap is on; the app adds up real time and passes what's due), and
/// whether playback stops there. Plays through the current range, or a
/// Frame Limits range holding `t` (TL-53). Repeat loops, Hold stops on the
/// end, Wrap stops back at `started_at` once the end is passed.
pub fn playback_step(
    scene: &Scene,
    t: Tick,
    started_at: Tick,
    frames_due: f64,
    backwards: bool,
) -> (Tick, bool) {
    let rate = scene.time.rate;
    let s = &scene.animation.settings;
    let (a, b) = limit_range(scene, t).unwrap_or_else(|| current_range(scene));
    let (fa, fb) = (frames(rate, a), frames(rate, b));
    let dir = if backwards { -1. } else { 1. };
    let f = frames(rate, t) + dir * frames_due.max(0.);
    let past = if backwards {
        f < fa - 1e-9
    } else {
        f > fb + 1e-9
    };
    if !past {
        return (at_frames(rate, f), false);
    }
    // A whole-frame range is fb - fa + 1 frames long.
    let span = (fb - fa + 1.).max(1.);
    let wrapped = if backwards {
        fb - (fa - f - 1.).rem_euclid(span)
    } else {
        fa + (f - fb - 1.).rem_euclid(span)
    };
    match s.end {
        EndBehavior::Hold => (if backwards { a } else { b }, true),
        EndBehavior::Repeat => (at_frames(rate, wrapped), false),
        // Plays to the end, then stops back where it started.
        EndBehavior::Wrap => (started_at, true),
    }
}

// ------------------------------------------------------------ the session

/// Per-document animation state that isn't scene data: the key time, the
/// channel and key selections, whether playback runs (PLAN: everything
/// belongs to one document; switching tabs swaps it with the scene).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimSession {
    /// TL-20: where new keys land when it differs from the current time.
    pub key_time: Option<Tick>,
    /// UIC-119: channels picked by their row labels; `S` keys them.
    pub selected_channels: std::collections::BTreeSet<ChannelKey>,
    /// TL-41: selected keys in the track row.
    pub selected_keys: std::collections::BTreeSet<(ChannelKey, Tick)>,
    /// TL-52: selected markers.
    pub selected_markers: std::collections::BTreeSet<u64>,
    /// Playing (backwards when true); the app advances it each frame.
    pub playing: Option<bool>,
    /// The time playback started from (Wrap goes back there).
    pub play_start: Option<Tick>,
    /// Set while an animation command edits the scene, so its own record
    /// isn't auto-keyed again.
    pub(crate) quiet: bool,
    /// Frame renders and playblasts asked for (TL-70/71); the app runs
    /// them (they need Cycles and the viewport), a headless bridge
    /// refuses them.
    pub requests: Vec<AnimRequest>,
    /// The running frame render or playblast: frames done, frames in all,
    /// the folder it writes to. The app keeps it current.
    pub progress: Option<(usize, usize, String)>,
}

/// A side-effect job only the app can run (TL-70/71).
#[derive(Clone, Debug, PartialEq)]
pub enum AnimRequest {
    /// Render frames `start..=end` with Cycles, one image each.
    RenderRange {
        start: i64,
        end: i64,
        /// Folder; `None`: the project's `renders/` (UIC-242).
        folder: Option<String>,
        /// File name before the frame number.
        name: String,
        samples: Option<u32>,
        size: Option<[u32; 2]>,
        camera: Option<u64>,
    },
    /// Record the active 3D view, HUD hidden, at `scale` of its size.
    Playblast {
        start: i64,
        end: i64,
        scale: f32,
        folder: Option<String>,
        name: String,
        /// Open the result in the system player.
        open: bool,
        /// A movie file (frames encoded after); false keeps the PNG
        /// sequence.
        movie: bool,
    },
    /// Stop the running job.
    Cancel,
}

// ------------------------------------------------------------ auto key

/// The channels of the items `before` and the scene share whose values an
/// edit changed, with their new values. `all`: every keyable channel of
/// the edited (selected) items, else only animated ones.
fn changed_channels(before: &Scene, after: &Scene, all: bool) -> Vec<(ChannelKey, f64, f64)> {
    let mut out = Vec::new();
    for (k, c) in &after.animation.channels {
        if c.curve.is_empty() {
            continue;
        }
        if let (Some(old), Some(new)) = (read(before, k), read(after, k))
            && differs(old, new)
        {
            out.push((k.clone(), old, new));
        }
    }
    if all {
        for id in after.selected_items() {
            let (Some(a), Some(b)) = (before.item(id), after.item(id)) else {
                continue;
            };
            let names: Vec<String> = if a.transform == b.transform {
                Vec::new()
            } else {
                TRANSFORM_CHANNELS.iter().map(|s| s.to_string()).collect()
            };
            let names = names.into_iter().chain(
                keyable_channels(after, id)
                    .into_iter()
                    .filter(|n| transform_part(n).is_none()),
            );
            for name in names {
                let k = ChannelKey::new(id, &name);
                if after.animation.is_animated(&k) {
                    continue;
                }
                if let (Some(old), Some(new)) = (read(before, &k), read(after, &k))
                    && differs(old, new)
                {
                    out.push((k, old, new));
                }
            }
        }
    }
    out
}

impl crate::Editor {
    /// Where new keys land (TL-20): the key time, else the current time.
    pub fn key_time(&self) -> Tick {
        self.anim.key_time.unwrap_or(self.scene.time.current)
    }

    /// TK-30/31: an edit recorded from `before` keys the channels it
    /// changed, per Auto Key, as part of the same undo step. Called from
    /// `record` (every edit source passes there) and when a live transform
    /// tool's drag ends. Also drops the curves of items that are gone.
    pub(crate) fn auto_key_from(&mut self, before: &Scene) {
        if self.anim.quiet {
            return;
        }
        let scene = &mut self.scene;
        if !scene.animation.channels.is_empty() {
            let gone: Vec<ChannelKey> = scene
                .animation
                .channels
                .keys()
                .filter(|k| scene.item(k.item).is_none())
                .cloned()
                .collect();
            for k in gone {
                scene.animation.channels.remove(&k);
            }
        }
        let mode = scene.animation.settings.auto_key;
        if scene.animation.channels.is_empty() && mode != AutoKey::All {
            return;
        }
        let at = self.anim.key_time.unwrap_or(self.scene.time.current);
        let now = self.scene.time.current;
        let update = self.scene.animation.settings.auto_update_keys;
        for (k, old, new) in changed_channels(before, &self.scene, mode == AutoKey::All) {
            let stepped = is_stepped(&self.scene, &k);
            let anim = &mut self.scene.animation;
            match mode {
                AutoKey::Off => {
                    // TK-40: with Auto Update Existing Keys, an edit on a key
                    // updates it; otherwise the edit is pending.
                    if update
                        && let Some(c) = anim.channels.get_mut(&k)
                        && let Some(i) = c.curve.key_at(now)
                    {
                        c.curve.keys[i].value = new;
                    }
                }
                AutoKey::Animated | AutoKey::All => {
                    // Already what the curve gives there (a tool's Drop
                    // after its drags were keyed): nothing to add.
                    if anim
                        .channels
                        .get(&k)
                        .is_some_and(|c| !c.curve.is_empty() && !differs(new, c.curve.value_at(at)))
                    {
                        continue;
                    }
                    let c = anim.channels.entry(k).or_insert_with(|| AnimChannel {
                        curve: Curve {
                            keys: Vec::new(),
                            stepped,
                            ..Default::default()
                        },
                        rest: old,
                    });
                    c.curve.set(at, new);
                }
            }
        }
    }
}

/// A key with `value` at `t`, the default slopes.
pub fn new_key(t: Tick, value: f64) -> Key {
    Key::new(t, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_text_round_trips() {
        let r = FrameRate::FPS_24;
        let t = r.tick_of(30);
        for d in [
            TimeDisplay::Frames,
            TimeDisplay::Seconds,
            TimeDisplay::Smpte,
            TimeDisplay::FilmCode,
        ] {
            let text = time_text(d, r, t);
            assert_eq!(parse_time(d, r, &text).unwrap(), t, "{d:?} {text}");
        }
        assert_eq!(time_text(TimeDisplay::Smpte, r, t), "00:00:01:06");
        assert_eq!(time_text(TimeDisplay::FilmCode, r, t), "1+14");
    }

    #[test]
    fn channel_keys_parse() {
        let k = ChannelKey::parse("3:pos.Y").unwrap();
        assert_eq!(k, ChannelKey::new(3, "pos.Y"));
        assert_eq!(k.transform_part(), Some(("pos", 1)));
        assert!(ChannelKey::parse("pos.Y").is_err());
    }
}
