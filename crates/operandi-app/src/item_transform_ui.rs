//! Item Properties > Item Transform: the Reset and Freeze action menus
//! (spec freeze-transforms-2026-11-14 section 1). Each is a dropdown whose
//! closed text never changes; picking an entry runs `transform.reset` or
//! `transform.freeze` on the item selection at once and closes the menu,
//! through the same command the Item > Transform menu and scripts use
//! (buttons, menus and commands share one operation). Shear is hidden until
//! there is a shear channel (section 10 Q6).
use super::*;
use crate::unavailable::Gate;
use operandi_core::camera_props::TransformReset;

/// The new Freeze's tooltip (section 10 Q1): unlike Geometry > Subdivision
/// > Freeze, it bakes the item's transform.
pub(crate) const FREEZE_TIP: &str = "Bake the transform into the geometry";
/// The closed text of both action menus: what picking runs, never a value
/// (section 1: the label never changes).
pub(crate) const MENU_TEXT: &str = "Position, Rotation, Scale, All";
pub(crate) const RESET_TIP: &str =
    "Put the transform back to identity; the item moves, its geometry doesn't";

/// The typed line an entry runs.
pub(crate) fn line(action: &str, part: TransformReset) -> String {
    format!("transform.{action} {}", part.script_word())
}

impl Operandi {
    /// The Reset dropdown (every item type) and, on a mesh item, the
    /// Freeze dropdown, at the foot of the Item Transform section.
    pub(super) fn transform_action_rows(&mut self, ui: &mut Ui, item: &operandi_core::Item) {
        let targets = self.transform_targets();
        let all_locked = !targets.is_empty() && targets.iter().all(|i| i.is_locked());
        // Greyed with a reason (unavailable.rs): a live tool, or every
        // selected item locked; a mixed selection skips the locked ones.
        let gate = crate::unavailable::first([
            crate::unavailable::unless(self.editor.live.is_none(), crate::unavailable::TOOL_ACTIVE),
            crate::unavailable::unless(!all_locked, crate::unavailable::LOCKED),
        ]);
        let mut picked: Option<(&str, TransformReset)> = None;
        ui.gated_ui(gate, |ui| {
            if let Some(part) =
                action_menu(ui, &format!("item-reset-{}", item.id), "Reset", RESET_TIP)
            {
                picked = Some(("reset", part));
            }
            // Freeze only on items that own geometry (section 1).
            if item.is_mesh()
                && let Some(part) = action_menu(
                    ui,
                    &format!("item-freeze-{}", item.id),
                    "Freeze",
                    FREEZE_TIP,
                )
            {
                picked = Some(("freeze", part));
            }
        });
        if let Some((action, part)) = picked {
            self.run_command_line(&line(action, part));
        }
    }
}

/// One action dropdown row, `title` in the label column and the menu full
/// width of the value column, its closed text always [`MENU_TEXT`]; the
/// picked part, if any.
fn action_menu(ui: &mut Ui, id: &str, title: &str, tip: &str) -> Option<TransformReset> {
    let r = crate::option_row::dropdown(ui, id, title, MENU_TEXT, |ui| {
        let mut picked = None;
        for part in TransformReset::ALL {
            if ui.selectable_label(false, part.label()).clicked() {
                picked = Some(part);
            }
        }
        picked
    });
    r.response.on_hover_text(tip);
    r.inner
}
