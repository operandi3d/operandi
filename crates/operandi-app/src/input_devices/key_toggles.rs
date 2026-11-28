//! Key toggles (MN-02): hold a tool's key to use the tool briefly.
//!
//! - Pressing a key that turns a tool on starts a hold (MN-02.1).
//! - A drag in a 3D view that starts while the key is down marks the hold
//!   used (MN-02.3: what was done decides, not how long the key was down).
//! - Releasing the key after a use drops the tool, keeping the edit, as
//!   Drop does (MN-02.2). A drag still held finishes first. The tool's
//!   session is one undo step named after it, as for any drop (MN-02.4).
//! - A tap (released unused) leaves the tool on, as before (MN-02.3).
//! - A key whose tool was already on starts no hold (MN-02.5); egui's key
//!   repeats never reach the bindings (MN-02.6). Pen strokes are pointer
//!   drags, so the pen works the same (MN-02.7).
//! - Input > Keys > Key Toggles turns it off (MN-02.8, default on).
//!
//! Holding a different tool's key while one tool is on swaps to it for the
//! hold, then returns to the first tool on release (decided for MN-02's
//! open question; [`RETURN_TO_PREVIOUS_TOOL`] changes it).
use super::Operandi;
use eframe::egui::{Context, Event, Key};
use operandi_core::{ActiveTask, Command};

/// After a held use of another tool, start the tool that was on before
/// the key went down again. `false` leaves no tool on instead.
pub(crate) const RETURN_TO_PREVIOUS_TOOL: bool = true;

/// A tool key being held.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeyHold {
    pub(crate) key: Key,
    /// The task the key turned on.
    pub(crate) task: ActiveTask,
    /// What was on before the key went down.
    pub(crate) previous: Option<ActiveTask>,
    /// A viewport drag started while the key was down.
    pub(crate) used: bool,
    /// The key is up; the drop waits for the drag to end.
    pub(crate) released: bool,
}

impl Operandi {
    /// After a key ran its binding: start a hold when it turned a tool on.
    pub(super) fn note_key_hold(&mut self, key: Key, before: Option<ActiveTask>) {
        if !self.input_preferences.key_toggles {
            return;
        }
        let after = self.editor.session.active_task();
        match after {
            Some(task) if after != before => {
                self.key_hold = Some(KeyHold {
                    key,
                    task,
                    previous: before,
                    used: false,
                    released: false,
                });
            }
            _ => {}
        }
    }

    /// Each frame: mark a hold used by a viewport drag, and end it when its
    /// key comes up.
    pub(super) fn key_hold_frame(&mut self, ctx: &Context) {
        let Some(hold) = self.key_hold.as_ref() else {
            return;
        };
        let (key, released) = (hold.key, hold.released);
        if self.editor.session.active_task() != Some(hold.task) {
            // Dropped, cancelled or switched some other way meanwhile.
            self.key_hold = None;
            return;
        }
        let (dragging, button_down, key_up) = ctx.input(|i| {
            let from_view = i
                .pointer
                .press_origin()
                .is_some_and(|p| self.corner.areas.values().any(|a| a.contains(p)))
                || i.pointer
                    .press_origin()
                    .is_some_and(|p| self.viewport_rect.contains(p));
            let up = i.events.iter().any(|e| {
                matches!(e, Event::Key { key: k, pressed: false, .. } if *k == key)
                    || matches!(e, Event::WindowFocused(false))
            });
            (
                from_view && i.pointer.is_decidedly_dragging(),
                i.pointer.any_down(),
                up,
            )
        });
        let hold = self.key_hold.as_mut().unwrap();
        if dragging && !released {
            hold.used = true;
        }
        if key_up {
            hold.released = true;
        }
        if !hold.released {
            return;
        }
        if !hold.used {
            // A tap: the tool stays on (MN-02.3).
            self.key_hold = None;
            return;
        }
        if button_down || self.pointer_held {
            // Let the drag finish first; its release ends the step.
            return;
        }
        let hold = self.key_hold.take().unwrap();
        self.run(Command::Drop);
        if RETURN_TO_PREVIOUS_TOOL
            && let Some(ActiveTask::Live(tool)) = hold.previous
            && self.editor.session.active_task().is_none()
        {
            self.activate(tool);
        }
        ctx.request_repaint();
    }
}
