//! Q and Esc, the way out of any tool and any selection (Jason,
//! 2026-11-21, from the Model the Mug tutorial). Shared by every tool;
//! nothing here knows a tool.
//!
//! - SC-078, UIC-24, UIC-66: Q (the input map's Drop) drops the active
//!   tool, wherever the pointer is. A Tool Properties number field that
//!   holds the keyboard no longer swallows it: no number, unit or
//!   expression has a Q in it, so the field confirms what was typed (as
//!   Enter would, UIC-51) and the tool drops. A value the field refuses
//!   keeps the field and its error, and the tool stays. Words fields
//!   (names) still take the letter.
//! - SC-080: Esc cancels the current operation first, else drops the
//!   selection. The order, one step per press: a field being edited
//!   cancels its edit (UIC-51, in the field); an open menu, pie or
//!   popover closes; a drag in progress cancels; the background
//!   constraint and the falloff clear (their own rules); then, over a 3D
//!   view, Esc drops the active tool like Q (UIC-24, UIC-66; Create's
//!   scratch points are cancelled instead); with no tool on, it drops
//!   the selection of the selection mode ("Drop Selection", which the
//!   input map holds, so it can be rebound). The tool drop follows SS-U03's
//!   factory-map rule: moving or unbinding Drop Cube, or putting another
//!   action on Esc, turns it off.
//! - Esc acts over a 3D view only (UIC-70: keys act on the panel under the
//!   pointer), so panels keep their own Esc.
use super::*;

impl Operandi {
    /// Start of a frame, before anything draws: the Drop chord pressed in
    /// a Tool Properties number field confirms the field and hands the
    /// keyboard back, so `keyboard` drops the tool this same frame. The
    /// letter never reaches the field.
    pub(super) fn drop_key_in_tool_field(&mut self, ctx: &Context) {
        let Some(id) = ctx.memory(|m| m.focused()) else {
            return;
        };
        let side_tool =
            self.draw.is_some() || self.utility.is_some() || self.sculpt.brush.is_some();
        if self.field_focus != Some(id)
            || self.field_focus_words
            || !(self.tool_is_armed() || side_tool)
        {
            return;
        }
        let chord = self.bindings.get("Drop").cloned().unwrap_or_default();
        if chord.is_empty() {
            return;
        }
        let hit = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if input::matches_chord(&chord, *key, *modifiers) => Some(*key),
                _ => None,
            })
        });
        let Some(key) = hit else {
            return;
        };
        // The letter the key would type, so the field never sees it.
        let letter = key.name().to_lowercase();
        ctx.input_mut(|i| {
            i.events
                .retain(|e| !matches!(e, Event::Text(t) if t.to_lowercase() == letter))
        });
        if !self.confirm_fields() {
            // Refused: the field keeps the keyboard and shows why; the key
            // is spent so the tool stays as it was.
            ctx.input_mut(|i| {
                i.events
                    .retain(|e| !matches!(e, Event::Key { key: k, pressed: true, .. } if *k == key))
            });
            return;
        }
        ctx.memory_mut(|m| m.surrender_focus(id));
    }

    /// Esc this frame took the keyboard from a text field (egui lets go of
    /// the focus before anything draws), so the press is the field's: it
    /// cancels the edit (UIC-51) and drops neither the tool nor the
    /// selection.
    pub(super) fn escape_left_a_field(&self, ctx: &Context) -> bool {
        ctx.input(|i| i.key_pressed(Key::Escape))
            && ctx.memory(|m| {
                m.focused().is_none()
                    && self
                        .field_focus
                        .iter()
                        .chain(&self.text_editors)
                        .any(|&id| m.had_focus_last_frame(id))
            })
    }

    /// A tool is on that Q and Esc drop (the input map's `live` context).
    pub(super) fn tool_is_armed(&self) -> bool {
        self.editor_tool_active() || self.edge_click.is_some() || self.vertex_weld
    }

    /// Plain Esc over a view drops the active tool, by the rule SS-U03
    /// set for Smooth Shift: only on the factory map, that is while Drop
    /// Cube still has its factory chord (Esc) and no other action is on
    /// Esc, available or not. Unbinding or moving Drop Cube turns it off.
    pub(super) fn escape_drops_tool(&self) -> bool {
        let factory = input::ACTIONS
            .iter()
            .find(|(a, _)| *a == "Drop Cube")
            .map(|(_, c)| *c);
        self.tool_is_armed()
            && self.bindings.get("Drop Cube").map(String::as_str) == factory
            && !self.bindings.iter().any(|(action, chord)| {
                !matches!(action.as_str(), "Drop" | "Drop Cube" | "Drop Selection")
                    && input::matches_chord(chord, Key::Escape, Modifiers::NONE)
            })
    }
}
