//! Tool Properties fields and the 3D views take turns with any tool open
//! (Jason, 2026-11-21, from the Model the Mug tutorial): a field edit never
//! leaves the views deaf. Shared by every tool; nothing here knows a tool.
//!
//! - UIC-70: keys act on the panel under the pointer. A field that holds
//!   the keyboard with nothing typed lets it go when the pointer moves over
//!   a 3D view, so A (Frame All) and the other view keys reach the view.
//!   Typed text keeps the keyboard, so a slip of the mouse never splits a
//!   number between the field and the shortcuts.
//! - UIC-51: Enter or Tab confirm a field. A press in a 3D view confirms it
//!   too and hands the keyboard back, and the press goes on to the view (a
//!   handle drag, a draw...). A value the field refuses keeps the field
//!   and its error, as before.
use super::*;

impl Operandi {
    /// The focused widget, when it is one of the Tool Properties fields.
    fn focused_tool_field(&self, ctx: &Context) -> Option<Id> {
        ctx.memory(|m| m.focused())
            .filter(|&id| self.field_focus == Some(id))
    }

    /// No field holds typing that hasn't been confirmed, and none has an
    /// error.
    fn fields_idle(&self) -> bool {
        self.field_errors.is_empty()
            && self
                .fields
                .iter()
                .all(|(key, text)| self.confirmed_fields.get(key) == Some(text))
    }

    /// Start of a frame: an idle field lets go of the keyboard once the
    /// pointer moves over a 3D view. Only a pointer move counts, so Tab
    /// from field to field with the pointer resting on a view still works.
    pub(super) fn release_idle_field_over_view(&mut self, ctx: &Context) {
        let Some(id) = self.focused_tool_field(ctx) else {
            return;
        };
        let moved = ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::PointerMoved(_))));
        if moved && self.pointer_over_a_view(ctx) && self.fields_idle() {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
    }

    /// A press in a 3D view, after its fields confirmed: the field gives
    /// the keyboard back so the rest of the press reaches the view.
    pub(super) fn release_field_for_view_press(&mut self, ctx: &Context) {
        if let Some(id) = self.focused_tool_field(ctx) {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
    }
}
