//! The rest of the shared item tabs (BEHAVIOR-ITEM-COMMON-TABS, Tier 3
//! rows): Display Pivot / Center (CP-D02/03), Draw Options (CP-D07, the
//! Enable / Wireframe / Wireframe Color pick), Dimensions (CP-D08, meshes)
//! and Notes (CP-D09); Assembly Redirect, Pick Walk, Tooltip, User Tag and
//! per-item Drop Action (CP-A03, A04, A06, A08, A09); User Channels
//! (CP-U01..04, v1 pick) and Tags (CP-T01..03, v1 pick). Each lives in
//! `ItemCommon` and is edited through `ItemCommonEdit::Extra`, one undo
//! step named after the row, on every selected item (IC-1, IC-2).
//!
//! "Add X" rows create the gadget with its defaults; "Remove X" drops it
//! and its settings (IC-4); undo brings them back.
use crate::Result;
use crate::drop_action::DropAction;
use crate::item_common::{Choice, ItemCommon};
use serde::{Deserialize, Serialize};

/// CP-D07 Wireframe: where the item's wire colour comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireColor {
    #[default]
    Default,
    User,
    BackgroundLight,
    BackgroundDark,
}

impl WireColor {
    pub const ALL: [WireColor; 4] = [
        WireColor::Default,
        WireColor::User,
        WireColor::BackgroundLight,
        WireColor::BackgroundDark,
    ];
    pub fn label(self) -> &'static str {
        match self {
            WireColor::Default => "Default",
            WireColor::User => "User",
            WireColor::BackgroundLight => "Background Light",
            WireColor::BackgroundDark => "Background Dark",
        }
    }
}

/// CP-D07 Draw Options (Enable, Wireframe, Wireframe Color).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DrawOptions {
    pub enable: bool,
    pub wireframe: WireColor,
    /// Linear-ish display RGB 0..1, used when Wireframe is User.
    pub wire_color: [f64; 3],
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            enable: true,
            wireframe: WireColor::Default,
            wire_color: [1.0, 1.0, 1.0],
        }
    }
}

/// CP-D08 Visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionsShow {
    Off,
    #[default]
    On,
    Selected,
}

impl DimensionsShow {
    pub const ALL: [DimensionsShow; 3] = [
        DimensionsShow::Off,
        DimensionsShow::On,
        DimensionsShow::Selected,
    ];
    pub fn label(self) -> &'static str {
        match self {
            DimensionsShow::Off => "Off",
            DimensionsShow::On => "On",
            DimensionsShow::Selected => "Selected",
        }
    }
}

/// CP-D08 Dimensions (v1 rows: Visibility, Opacity, per-axis show).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Dimensions {
    pub visibility: DimensionsShow,
    /// 0..1.
    pub opacity: f64,
    pub show: [bool; 3],
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            visibility: DimensionsShow::On,
            opacity: 1.0,
            show: [true; 3],
        }
    }
}

/// Most lines a Notes gadget holds (CP-D09).
pub const NOTE_LINES: usize = 9;

/// CP-D09 Notes: a title and up to nine lines, shown while selected.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Notes {
    pub title: String,
    pub lines: Vec<String>,
}

/// CP-U02 Type (v1 subset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    #[default]
    Float,
    Distance,
    Angle,
    Percent,
    Integer,
    Boolean,
    String,
    Divider,
}

impl ChannelType {
    pub const ALL: [ChannelType; 8] = [
        ChannelType::Float,
        ChannelType::Distance,
        ChannelType::Angle,
        ChannelType::Percent,
        ChannelType::Integer,
        ChannelType::Boolean,
        ChannelType::String,
        ChannelType::Divider,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ChannelType::Float => "Float",
            ChannelType::Distance => "Distance",
            ChannelType::Angle => "Angle",
            ChannelType::Percent => "Percentage",
            ChannelType::Integer => "Integer",
            ChannelType::Boolean => "Boolean",
            ChannelType::String => "String",
            ChannelType::Divider => "Divider",
        }
    }
    /// Numeric types (Float, Distance, Angle, Percent, Integer).
    pub fn numeric(self) -> bool {
        !matches!(
            self,
            ChannelType::Boolean | ChannelType::String | ChannelType::Divider
        )
    }
}

/// A user channel's value. Distances metres, angles radians, percentages
/// fractions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChannelValue {
    Number(f64),
    Bool(bool),
    Text(String),
    None,
}

/// CP-U01..04: one user channel. Stored and edited only (no linking).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserChannel {
    /// Channel Name: unique on the item.
    pub name: String,
    /// Username: the label shown (Channel Name when empty).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    pub kind: ChannelType,
    pub value: ChannelValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

impl UserChannel {
    /// A new channel of `kind` set to `default` (numbers clamp to the
    /// limits; an Integer rounds).
    pub fn new(
        name: &str,
        username: &str,
        kind: ChannelType,
        default: ChannelValue,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Result<UserChannel> {
        let mut ch = UserChannel {
            name: name.trim().to_string(),
            username: username.trim().to_string(),
            kind,
            value: ChannelValue::None,
            min,
            max,
        };
        ch.set(default)?;
        ch.validate()?;
        Ok(ch)
    }

    /// The row's label.
    pub fn label(&self) -> &str {
        if self.username.is_empty() {
            &self.name
        } else {
            &self.username
        }
    }

    /// Set the value, refusing the wrong kind; numbers clamp to the limits.
    pub fn set(&mut self, value: ChannelValue) -> Result<()> {
        self.value = match (self.kind, value) {
            (ChannelType::Divider, _) => ChannelValue::None,
            (ChannelType::Boolean, ChannelValue::Bool(b)) => ChannelValue::Bool(b),
            (ChannelType::String, ChannelValue::Text(t)) => ChannelValue::Text(t),
            (k, ChannelValue::Number(n)) if k.numeric() && n.is_finite() => {
                let mut n = n;
                if let Some(lo) = self.min {
                    n = n.max(lo);
                }
                if let Some(hi) = self.max {
                    n = n.min(hi);
                }
                if k == ChannelType::Integer {
                    n = n.round();
                }
                ChannelValue::Number(n)
            }
            _ => return Err("That value doesn't suit this channel's type".into()),
        };
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let ok_name = !self.name.is_empty()
            && self.name.len() <= 256
            && self
                .name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ' ')
            && self.username.len() <= 256;
        let ok_limits = self.min.is_none_or(f64::is_finite)
            && self.max.is_none_or(f64::is_finite)
            && match (self.min, self.max) {
                (Some(lo), Some(hi)) => lo <= hi,
                _ => true,
            };
        let ok_text = match &self.value {
            ChannelValue::Text(t) => t.len() <= 64 * 1024,
            ChannelValue::Number(n) => n.is_finite(),
            _ => true,
        };
        if ok_name && ok_limits && ok_text {
            Ok(())
        } else {
            Err("Invalid user channel".into())
        }
    }
}

/// The built-in tag type (CP-T02 v1 pick).
pub const COMMENT_TAG: &str = "Comment";

/// CP-T02: a tag, one per type on an item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemTag {
    /// The tag type ("Comment" or a custom type).
    pub kind: String,
    pub value: String,
}

/// The Tier 3 extras of an item's shared tabs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemExtras {
    /// CP-D02 / CP-D03 (Default reads as No: no marker).
    pub pivot: Choice,
    pub center: Choice,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draw_options: Option<DrawOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<Dimensions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Notes>,
    /// CP-A03: a 3D pick of this item selects this item instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<u64>,
    /// CP-A04: Up, Down, Left, Right.
    pub pick_walk: [Option<u64>; 4],
    /// CP-A06.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tooltip: String,
    /// CP-A08.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user_tag: String,
    /// CP-A09: Assembly, Source, Destination; none = the top bar's.
    pub drop_actions: [Option<DropAction>; 3],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<UserChannel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<ItemTag>,
}

/// Pick Walk directions (CP-A04).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Walk {
    pub const ALL: [Walk; 4] = [Walk::Up, Walk::Down, Walk::Left, Walk::Right];
    pub fn label(self) -> &'static str {
        match self {
            Walk::Up => "Pick Walk Up",
            Walk::Down => "Pick Walk Down",
            Walk::Left => "Pick Walk Left",
            Walk::Right => "Pick Walk Right",
        }
    }
}

/// Drop Action slots (CP-A09).
pub const DROP_SLOTS: [&str; 3] = ["Assembly", "Source", "Destination"];

/// One Tier 3 row of the shared tabs.
#[derive(Clone, Debug, PartialEq)]
pub enum ExtraEdit {
    Pivot(Choice),
    Center(Choice),
    /// IC-4 Add / Remove Draw Options, Dimensions, Notes.
    DrawOptions(bool),
    DrawEnable(bool),
    Wireframe(WireColor),
    WireColor([f64; 3]),
    Dimensions(bool),
    DimensionsShow(DimensionsShow),
    DimensionsOpacity(f64),
    DimensionsAxis(usize, bool),
    Notes(bool),
    NotesTitle(String),
    NoteLine(usize, String),
    Redirect(Option<u64>),
    PickWalk(Walk, Option<u64>),
    Tooltip(String),
    UserTag(String),
    DropAction(usize, Option<DropAction>),
    AddChannel(UserChannel),
    RemoveChannel(String),
    ChannelValue(String, ChannelValue),
    /// Add or replace the tag of this type.
    SetTag(String, String),
    RemoveTag(String),
}

impl ExtraEdit {
    pub fn label(&self) -> &'static str {
        match self {
            ExtraEdit::Pivot(_) => "Pivot",
            ExtraEdit::Center(_) => "Center",
            ExtraEdit::DrawOptions(true) => "Add Draw Options",
            ExtraEdit::DrawOptions(false) => "Remove Draw Options",
            ExtraEdit::DrawEnable(_) => "Draw Options Enable",
            ExtraEdit::Wireframe(_) => "Wireframe",
            ExtraEdit::WireColor(_) => "Wireframe Color",
            ExtraEdit::Dimensions(true) => "Add Dimensions",
            ExtraEdit::Dimensions(false) => "Remove Dimensions",
            ExtraEdit::DimensionsShow(_) => "Dimensions Visibility",
            ExtraEdit::DimensionsOpacity(_) => "Dimensions Opacity",
            ExtraEdit::DimensionsAxis(..) => "Dimensions Axis",
            ExtraEdit::Notes(true) => "Add Notes",
            ExtraEdit::Notes(false) => "Remove Notes",
            ExtraEdit::NotesTitle(_) => "Notes Title",
            ExtraEdit::NoteLine(..) => "Notes Line",
            ExtraEdit::Redirect(_) => "Redirect",
            ExtraEdit::PickWalk(w, _) => w.label(),
            ExtraEdit::Tooltip(_) => "Tooltip",
            ExtraEdit::UserTag(_) => "User Tag",
            ExtraEdit::DropAction(..) => "Drop Action",
            ExtraEdit::AddChannel(_) => "Add User Channel",
            ExtraEdit::RemoveChannel(_) => "Remove User Channel",
            ExtraEdit::ChannelValue(..) => "User Channel",
            ExtraEdit::SetTag(..) => "Tag",
            ExtraEdit::RemoveTag(_) => "Remove Tag",
        }
    }

    /// Lock Yes also locks the display rows (CP-A02): Pivot and Center.
    pub fn locked_by(&self) -> bool {
        matches!(self, ExtraEdit::Pivot(_) | ExtraEdit::Center(_))
    }

    /// Item ids this edit refers to (checked against the scene).
    pub fn references(&self) -> Option<u64> {
        match self {
            ExtraEdit::Redirect(r) | ExtraEdit::PickWalk(_, r) => *r,
            _ => None,
        }
    }
}

/// Longest free text (tooltip, tag, title, line), bytes.
const MAX_TEXT: usize = 64 * 1024;

impl ItemCommon {
    pub(crate) fn apply_extra(&mut self, edit: &ExtraEdit) -> Result<()> {
        let x = &mut self.extras;
        let text_ok = |t: &str| {
            if t.len() > MAX_TEXT {
                Err("Text is too long".to_string())
            } else {
                Ok(())
            }
        };
        let missing = |what: &str| format!("Add {what} first");
        match edit {
            ExtraEdit::Pivot(c) => x.pivot = *c,
            ExtraEdit::Center(c) => x.center = *c,
            ExtraEdit::DrawOptions(add) => {
                x.draw_options = add.then(|| x.draw_options.clone().unwrap_or_default())
            }
            ExtraEdit::DrawEnable(on) => {
                x.draw_options
                    .as_mut()
                    .ok_or_else(|| missing("Draw Options"))?
                    .enable = *on
            }
            ExtraEdit::Wireframe(w) => {
                x.draw_options
                    .as_mut()
                    .ok_or_else(|| missing("Draw Options"))?
                    .wireframe = *w
            }
            ExtraEdit::WireColor(c) => {
                if !c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) {
                    return Err("Colour out of range".into());
                }
                x.draw_options
                    .as_mut()
                    .ok_or_else(|| missing("Draw Options"))?
                    .wire_color = *c
            }
            ExtraEdit::Dimensions(add) => {
                x.dimensions = add.then(|| x.dimensions.clone().unwrap_or_default())
            }
            ExtraEdit::DimensionsShow(v) => {
                x.dimensions
                    .as_mut()
                    .ok_or_else(|| missing("Dimensions"))?
                    .visibility = *v
            }
            ExtraEdit::DimensionsOpacity(v) => {
                if !(v.is_finite() && (0.0..=1.0).contains(v)) {
                    return Err("Opacity out of range".into());
                }
                x.dimensions
                    .as_mut()
                    .ok_or_else(|| missing("Dimensions"))?
                    .opacity = *v
            }
            ExtraEdit::DimensionsAxis(axis, on) => {
                let d = x.dimensions.as_mut().ok_or_else(|| missing("Dimensions"))?;
                *d.show.get_mut(*axis).ok_or("No such axis")? = *on;
            }
            ExtraEdit::Notes(add) => x.notes = add.then(|| x.notes.clone().unwrap_or_default()),
            ExtraEdit::NotesTitle(t) => {
                text_ok(t)?;
                x.notes.as_mut().ok_or_else(|| missing("Notes"))?.title = t.clone()
            }
            ExtraEdit::NoteLine(i, t) => {
                text_ok(t)?;
                if *i >= NOTE_LINES {
                    return Err("Notes hold nine lines".into());
                }
                let n = x.notes.as_mut().ok_or_else(|| missing("Notes"))?;
                if n.lines.len() <= *i {
                    n.lines.resize(*i + 1, String::new());
                }
                n.lines[*i] = t.clone();
                while n.lines.last().is_some_and(String::is_empty) {
                    n.lines.pop();
                }
            }
            ExtraEdit::Redirect(r) => x.redirect = *r,
            ExtraEdit::PickWalk(w, r) => x.pick_walk[*w as usize] = *r,
            ExtraEdit::Tooltip(t) => {
                text_ok(t)?;
                x.tooltip = t.clone()
            }
            ExtraEdit::UserTag(t) => {
                text_ok(t)?;
                x.user_tag = t.clone()
            }
            ExtraEdit::DropAction(slot, a) => {
                *x.drop_actions
                    .get_mut(*slot)
                    .ok_or("No such Drop Action slot")? = *a
            }
            ExtraEdit::AddChannel(ch) => {
                ch.validate()?;
                if x.channels.iter().any(|c| c.name == ch.name) {
                    return Err(format!("A channel called {} already exists", ch.name));
                }
                if x.channels.len() >= 256 {
                    return Err("Too many user channels".into());
                }
                x.channels.push(ch.clone());
            }
            ExtraEdit::RemoveChannel(name) => {
                let before = x.channels.len();
                x.channels.retain(|c| &c.name != name);
                if x.channels.len() == before {
                    return Err(format!("No channel called {name}"));
                }
            }
            ExtraEdit::ChannelValue(name, v) => {
                let ch = x
                    .channels
                    .iter_mut()
                    .find(|c| &c.name == name)
                    .ok_or_else(|| format!("No channel called {name}"))?;
                if let ChannelValue::Text(t) = v {
                    text_ok(t)?;
                }
                ch.set(v.clone())?;
            }
            ExtraEdit::SetTag(kind, value) => {
                text_ok(value)?;
                let kind = kind.trim();
                if kind.is_empty() || kind.len() > 256 {
                    return Err("Give the tag a type".into());
                }
                match x.tags.iter_mut().find(|t| t.kind == kind) {
                    Some(t) => t.value = value.clone(),
                    None => x.tags.push(ItemTag {
                        kind: kind.to_string(),
                        value: value.clone(),
                    }),
                }
            }
            ExtraEdit::RemoveTag(kind) => {
                let before = x.tags.len();
                x.tags.retain(|t| &t.kind != kind);
                if x.tags.len() == before {
                    return Err(format!("No {kind} tag"));
                }
            }
        }
        Ok(())
    }
}

impl crate::scene::Scene {
    /// CP-A03: the item a 3D pick of `id` selects (its Redirect when that
    /// item exists; one step, no chains).
    pub fn redirected(&self, id: u64) -> u64 {
        self.item(id)
            .and_then(|i| i.common.extras.redirect)
            .filter(|r| self.item(*r).is_some())
            .unwrap_or(id)
    }

    /// CP-A04: the item the arrow key `walk` selects from `id`. A Pick Walk
    /// item wins; otherwise Up = parent, Down = first child, Left / Right =
    /// the previous / next item with the same parent (Items-list order).
    pub fn pick_walk(&self, id: u64, walk: Walk) -> Option<u64> {
        let item = self.item(id)?;
        if let Some(to) =
            item.common.extras.pick_walk[walk as usize].filter(|to| self.item(*to).is_some())
        {
            return Some(to);
        }
        match walk {
            Walk::Up => item.parent,
            Walk::Down => self
                .items
                .iter()
                .find(|i| i.parent == Some(id))
                .map(|i| i.id),
            Walk::Left | Walk::Right => {
                let siblings: Vec<u64> = self
                    .items
                    .iter()
                    .filter(|i| i.parent == item.parent)
                    .map(|i| i.id)
                    .collect();
                let at = siblings.iter().position(|s| *s == id)?;
                match walk {
                    Walk::Left => at.checked_sub(1).map(|k| siblings[k]),
                    _ => siblings.get(at + 1).copied(),
                }
            }
        }
    }

    /// CP-A09: the Drop Action for dropping `source` onto `destination`:
    /// the destination's own Destination action, else the source's Source
    /// action, else `top_bar`.
    pub fn drop_action_for(
        &self,
        source: u64,
        destination: u64,
        top_bar: DropAction,
    ) -> DropAction {
        let own = |id: u64, slot: usize| {
            self.item(id)
                .and_then(|i| i.common.extras.drop_actions[slot])
        };
        own(destination, 2)
            .or_else(|| own(source, 1))
            .unwrap_or(top_bar)
    }
}

impl ItemExtras {
    pub(crate) fn validate(&self) -> Result<()> {
        for ch in &self.channels {
            ch.validate()?;
        }
        let names: std::collections::BTreeSet<&str> =
            self.channels.iter().map(|c| c.name.as_str()).collect();
        let kinds: std::collections::BTreeSet<&str> =
            self.tags.iter().map(|t| t.kind.as_str()).collect();
        if names.len() != self.channels.len()
            || kinds.len() != self.tags.len()
            || self
                .notes
                .as_ref()
                .is_some_and(|n| n.lines.len() > NOTE_LINES)
            || self.draw_options.as_ref().is_some_and(|d| {
                !d.wire_color
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            })
            || self
                .dimensions
                .as_ref()
                .is_some_and(|d| !(0.0..=1.0).contains(&d.opacity))
        {
            return Err("Invalid item extras".into());
        }
        Ok(())
    }
}
