//! Shift+double-click on empty viewport space grows the component
//! selection (Jason, 2026-11-23): Vertices and Polygons add every component
//! connected to the selection (Select Connected), Edges add each selected
//! edge's full loop (Select Loop). It only adds, and it is one undo step:
//! the first press missed everything, so it changed nothing and recorded
//! nothing; the second runs the whole-selection command once.
//! UIC-243: the gesture only turns into existing selection commands
//! (`Command::Selection`), so it shares their symmetry and multi-mesh rules.
use super::*;
use viewport::EDGE_DOUBLE_CLICK_DISTANCE;

impl Operandi {
    /// Remembers a Shift-only press that missed every component, as a
    /// possible first press of an empty-space Shift+double-click. Any
    /// other press forgets it.
    pub(super) fn note_empty_shift_press(
        &mut self,
        hit: bool,
        mods: Modifiers,
        now: f64,
        at: Pos2,
    ) {
        self.last_empty_shift_press =
            (!hit && shift_only(mods) && self.editor.scene.selection.mode != Mode::Item)
                .then_some((now, at));
    }

    /// The second press of a Shift+double-click on empty space: grows the
    /// selection and returns the gesture. Any other press (including one
    /// that lands on a component) falls through to ordinary picking.
    pub(super) fn empty_expand_double_click(
        &mut self,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
        now: f64,
        delay: f64,
    ) -> Option<input::GestureKind> {
        // Taken, not peeked: after a double-click, a third press starts over.
        let (first_time, first_pos) = self.last_empty_shift_press.take()?;
        let mode = self.editor.scene.selection.mode;
        if mode == Mode::Item
            || !shift_only(mods)
            || now - first_time > delay
            || first_pos.distance(pos) > EDGE_DOUBLE_CLICK_DISTANCE
            || self.pick_component(pos, r).is_some()
        {
            return None;
        }
        // A component double-click needs this press too; it missed, so
        // there is nothing for one to do.
        self.last_component_press = None;
        if self.editor.scene.selection.component_count() == 0 {
            self.status = "Expand Selection: select components first".into();
            return Some(input::GestureKind::Blocked);
        }
        let operation = if mode == Mode::Edge {
            SelectionOperation::Loop
        } else {
            SelectionOperation::Connected
        };
        let noun = match mode {
            Mode::Vertex => "Vertices",
            Mode::Edge => "Edges",
            _ => "Polygons",
        };
        let before = self.editor.scene.selection.component_count();
        if self.run_ok(Command::Selection(operation)) {
            let after = self.editor.scene.selection.component_count();
            self.status = if after > before {
                format!("Expand Selection: {after} selected {noun}")
            } else {
                format!("Expand Selection: nothing to add; {after} selected {noun}")
            };
        }
        // On a refusal (an Edge loop that can't go on) the status line
        // already says why.
        Some(input::GestureKind::Blocked)
    }
}

/// Shift and no other modifier: Ctrl removes, Alt and Command navigate.
fn shift_only(mods: Modifiers) -> bool {
    mods.shift && !mods.ctrl && !mods.alt && !mods.command && !mods.mac_cmd
}
