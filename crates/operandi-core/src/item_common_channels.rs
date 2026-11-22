//! `item.channel` table for the shared item tabs (Display, Assembly,
//! targeting; item_common.rs, item_extras.rs): what every item has.
//!
//! visible, label, showLabel, labelOffsetX, labelOffsetY, pivot, center,
//! select, lock, target, targetEnable, targetRoll, redirect, pickWalkUp,
//! pickWalkDown, pickWalkLeft, pickWalkRight, tooltip, userTag,
//! dropAssembly, dropSource, dropDestination.
//!
//! Channel names are Operandi's own for now (provisional, like the rest of
//! channels.rs). Each set is one `Command::EditItemCommon` on that item:
//! one undo step, validated (locks, label length) by the editor.
use crate::channels::{Channel, ChannelType, ChannelValue, Result, Target};
use crate::drop_action::DropAction;
use crate::item_common::{Choice, DisplayVisible, ItemCommon, ItemCommonEdit};
use crate::item_extras::{DROP_SLOTS, ExtraEdit, Walk};
use crate::{Command, Scene};

const VISIBLE: &[&str] = &["Default", "Yes", "No", "No (Hide Children)"];
const CHOICES: &[&str] = &["Default", "Yes", "No"];
/// "(none)" (the top bar's Drop Action), then every action.
const DROPS: &[&str] = &[
    "(none)",
    "Parent",
    "Parent in Place",
    "Match",
    "Match Position",
    "Match Rotation",
    "Match Scale",
    "Insert",
    "Insert at Parent",
    "Insert in Place",
    "Place",
    "Place and Align",
];

fn common(scene: &Scene, target: Target) -> Option<&ItemCommon> {
    match target {
        Target::Item(id) => scene.item(id).map(|i| &i.common),
        _ => None,
    }
}

fn is_item(scene: &Scene, target: Target) -> bool {
    common(scene, target).is_some()
}

fn edit(target: Target, edit: ItemCommonEdit) -> Result<Command> {
    match target {
        Target::Item(id) => Ok(Command::EditItemCommon {
            items: vec![id],
            edit,
        }),
        _ => Err("Not an item".into()),
    }
}

fn index<T: PartialEq>(all: &[T], v: &T) -> usize {
    all.iter().position(|x| x == v).unwrap_or(0)
}

fn choice(v: ChannelValue) -> Result<Choice> {
    match v {
        ChannelValue::Enum(i) => Choice::ALL.get(i).copied().ok_or("Unknown choice".into()),
        _ => Err("Expected Default, Yes or No".into()),
    }
}

fn text(v: ChannelValue) -> Result<String> {
    match v {
        ChannelValue::Text(t) => Ok(t),
        _ => Err("Expected text".into()),
    }
}

fn float(v: ChannelValue) -> Result<f64> {
    match v {
        ChannelValue::Float(f) => Ok(f),
        ChannelValue::Int(n) => Ok(n as f64),
        _ => Err("Expected a number".into()),
    }
}

fn item_ref(v: ChannelValue) -> Result<Option<u64>> {
    match v {
        ChannelValue::Item(i) => Ok(i),
        _ => Err("Expected an item id or none".into()),
    }
}

fn drop_action(v: ChannelValue) -> Result<Option<DropAction>> {
    match v {
        ChannelValue::Enum(0) => Ok(None),
        ChannelValue::Enum(i) => DropAction::ALL
            .get(i)
            .copied()
            .map(Some)
            .ok_or("Unknown Drop Action".into()),
        _ => Err("Expected a Drop Action".into()),
    }
}

fn drop_get(c: &ItemCommon, slot: usize) -> ChannelValue {
    ChannelValue::Enum(
        c.extras.drop_actions[slot]
            .filter(|a| *a != DropAction::None)
            .map_or(0, |a| index(&DropAction::ALL, &a)),
    )
}

macro_rules! walk {
    ($name:literal, $walk:expr) => {
        Channel {
            name: $name,
            ty: ChannelType::ItemRef,
            applies: is_item,
            get: |s, t| {
                common(s, t).map(|c| ChannelValue::Item(c.extras.pick_walk[$walk as usize]))
            },
            set: |_, t, v| {
                edit(
                    t,
                    ItemCommonEdit::Extra(ExtraEdit::PickWalk($walk, item_ref(v)?)),
                )
            },
        }
    };
}

macro_rules! drop_slot {
    ($name:literal, $slot:expr) => {
        Channel {
            name: $name,
            ty: ChannelType::Enum(DROPS),
            applies: is_item,
            get: |s, t| common(s, t).map(|c| drop_get(c, $slot)),
            set: |_, t, v| {
                edit(
                    t,
                    ItemCommonEdit::Extra(ExtraEdit::DropAction($slot, drop_action(v)?)),
                )
            },
        }
    };
}

/// Every item's Display, Assembly and targeting rows.
pub static ITEM: &[Channel] = &[
    Channel {
        name: "visible",
        ty: ChannelType::Enum(VISIBLE),
        applies: is_item,
        get: |s, t| {
            common(s, t)
                .map(|c| ChannelValue::Enum(index(&DisplayVisible::ALL, &c.display.visible)))
        },
        set: |_, t, v| match v {
            ChannelValue::Enum(i) => edit(
                t,
                ItemCommonEdit::Visible(*DisplayVisible::ALL.get(i).ok_or("Unknown choice")?),
            ),
            _ => Err("Expected Default, Yes, No or No (Hide Children)".into()),
        },
    },
    Channel {
        name: "label",
        ty: ChannelType::Text,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Text(c.display.label.clone())),
        set: |_, t, v| edit(t, ItemCommonEdit::Label(text(v)?)),
    },
    Channel {
        name: "showLabel",
        ty: ChannelType::Bool,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Bool(c.display.show_label)),
        set: |_, t, v| match v {
            ChannelValue::Bool(b) => edit(t, ItemCommonEdit::ShowLabel(b)),
            _ => Err("Expected true or false".into()),
        },
    },
    Channel {
        name: "labelOffsetX",
        ty: ChannelType::Float,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Float(c.display.label_offset[0])),
        set: |_, t, v| edit(t, ItemCommonEdit::LabelOffsetX(float(v)?)),
    },
    Channel {
        name: "labelOffsetY",
        ty: ChannelType::Float,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Float(c.display.label_offset[1])),
        set: |_, t, v| edit(t, ItemCommonEdit::LabelOffsetY(float(v)?)),
    },
    Channel {
        name: "pivot",
        ty: ChannelType::Enum(CHOICES),
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Enum(index(&Choice::ALL, &c.extras.pivot))),
        set: |_, t, v| edit(t, ItemCommonEdit::Extra(ExtraEdit::Pivot(choice(v)?))),
    },
    Channel {
        name: "center",
        ty: ChannelType::Enum(CHOICES),
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Enum(index(&Choice::ALL, &c.extras.center))),
        set: |_, t, v| edit(t, ItemCommonEdit::Extra(ExtraEdit::Center(choice(v)?))),
    },
    Channel {
        name: "select",
        ty: ChannelType::Enum(CHOICES),
        applies: is_item,
        get: |s, t| {
            common(s, t).map(|c| ChannelValue::Enum(index(&Choice::ALL, &c.assembly.select)))
        },
        set: |_, t, v| edit(t, ItemCommonEdit::Select(choice(v)?)),
    },
    Channel {
        name: "lock",
        ty: ChannelType::Enum(CHOICES),
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Enum(index(&Choice::ALL, &c.assembly.lock))),
        set: |_, t, v| edit(t, ItemCommonEdit::Lock(choice(v)?)),
    },
    Channel {
        // Set Target (an item id) or Remove Target (none), LP-04.
        name: "target",
        ty: ChannelType::ItemRef,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Item(c.target.as_ref().map(|x| x.item))),
        set: |_, t, v| edit(t, ItemCommonEdit::Target(item_ref(v)?)),
    },
    Channel {
        name: "redirect",
        ty: ChannelType::ItemRef,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Item(c.extras.redirect)),
        set: |_, t, v| edit(t, ItemCommonEdit::Extra(ExtraEdit::Redirect(item_ref(v)?))),
    },
    walk!("pickWalkUp", Walk::Up),
    walk!("pickWalkDown", Walk::Down),
    walk!("pickWalkLeft", Walk::Left),
    walk!("pickWalkRight", Walk::Right),
    Channel {
        name: "tooltip",
        ty: ChannelType::Text,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Text(c.extras.tooltip.clone())),
        set: |_, t, v| edit(t, ItemCommonEdit::Extra(ExtraEdit::Tooltip(text(v)?))),
    },
    Channel {
        name: "userTag",
        ty: ChannelType::Text,
        applies: is_item,
        get: |s, t| common(s, t).map(|c| ChannelValue::Text(c.extras.user_tag.clone())),
        set: |_, t, v| edit(t, ItemCommonEdit::Extra(ExtraEdit::UserTag(text(v)?))),
    },
    drop_slot!("dropAssembly", 0),
    drop_slot!("dropSource", 1),
    drop_slot!("dropDestination", 2),
];

/// The item.channel line that makes `edit` on item `item` (UIC-164), or
/// `None` for rows no channel covers (Draw Options, Notes, User Channels…).
pub fn common_line(item: u64, edit: &ItemCommonEdit) -> Option<String> {
    let brace = |t: &str| format!("{{{t}}}");
    let id = |i: &Option<u64>| i.map_or_else(|| "none".to_owned(), |i| i.to_string());
    let (name, value) = match edit {
        ItemCommonEdit::Visible(v) => ("visible", brace(v.label())),
        // Text values: a label or tooltip with braces in it can't be
        // written as a line, so the panel edits it directly.
        ItemCommonEdit::Label(t) if !t.contains(['{', '}']) => ("label", brace(t)),
        ItemCommonEdit::ShowLabel(b) => ("showLabel", b.to_string()),
        ItemCommonEdit::LabelOffsetX(v) => ("labelOffsetX", v.to_string()),
        ItemCommonEdit::LabelOffsetY(v) => ("labelOffsetY", v.to_string()),
        ItemCommonEdit::Select(c) => ("select", c.label().to_owned()),
        ItemCommonEdit::Lock(c) => ("lock", c.label().to_owned()),
        ItemCommonEdit::Target(t) => ("target", id(t)),
        ItemCommonEdit::Extra(x) => match x {
            ExtraEdit::Pivot(c) => ("pivot", c.label().to_owned()),
            ExtraEdit::Center(c) => ("center", c.label().to_owned()),
            ExtraEdit::Redirect(t) => ("redirect", id(t)),
            ExtraEdit::PickWalk(w, t) => (
                [
                    "pickWalkUp",
                    "pickWalkDown",
                    "pickWalkLeft",
                    "pickWalkRight",
                ][*w as usize],
                id(t),
            ),
            ExtraEdit::Tooltip(t) if !t.contains(['{', '}']) => ("tooltip", brace(t)),
            ExtraEdit::UserTag(t) if !t.contains(['{', '}']) => ("userTag", brace(t)),
            ExtraEdit::DropAction(slot, a) => (
                ["dropAssembly", "dropSource", "dropDestination"][*slot.min(&2)],
                brace(
                    DROPS[a
                        .filter(|a| *a != DropAction::None)
                        .map_or(0, |a| index(&DropAction::ALL, &a))],
                ),
            ),
            _ => return None,
        },
        _ => return None,
    };
    debug_assert_eq!(DROP_SLOTS.len(), 3);
    Some(format!("item.channel {name} {value} item:{item}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_names_follow_drop_action_labels() {
        for (i, a) in DropAction::ALL.iter().enumerate() {
            assert_eq!(DROPS[i], a.label());
        }
        for (i, v) in DisplayVisible::ALL.iter().enumerate() {
            assert_eq!(VISIBLE[i], v.label());
        }
        for (i, c) in Choice::ALL.iter().enumerate() {
            assert_eq!(CHOICES[i], c.label());
        }
    }
}
