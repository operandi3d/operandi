//! Drop Action (spec drop-action-2026-11-06): the top-bar dropdown
//! (DA-1..3) and the viewport gesture (DA-4..9). In Item mode with no tool
//! active and an action chosen, pressing on an item, dragging and releasing
//! over another item applies the action; the scene work is
//! `Command::DropAction` in operandi-core.
use eframe::egui::{self, Button, Modifiers, Pos2, Rect, Ui};
use operandi_core::Mode;
use operandi_core::editor::Command;
use operandi_core::{Component, DropAction};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

/// The chosen action, saved in `drop-action.json` (DA-Q1 pick: it survives
/// a restart; new installs start at (none)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DropActionPreferences {
    pub action: DropAction,
}
impl DropActionPreferences {
    pub fn load(path: &Path) -> Self {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 4096)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

/// A press on item `source` that may become a drop (DA-4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropDrag {
    pub source: u64,
    pub start: Pos2,
}

/// Pointer travel, in points, below which a press and release is a click.
const CLICK_SLOP: f32 = 3.;

impl super::Operandi {
    /// DA-1/2: "Drop Action: <current>" with a plain list, the current
    /// entry checked. Changing it touches neither scene nor history (DA-3).
    pub(super) fn drop_action_menu(&mut self, ui: &mut Ui) {
        let current = self.drop_action.action;
        let button = Button::new(format!("Drop Action: {} ⏷", current.label()));
        egui::containers::menu::MenuButton::from_button(button)
            .ui(ui, |ui| {
                self.menu_owned_input = true;
                ui.set_min_width(180.);
                for action in DropAction::ALL {
                    let text = format!(
                        "{} {}",
                        if action == current { "✔" } else { "   " },
                        action.label()
                    );
                    if ui.add(Button::new(text).frame(false)).clicked() {
                        self.set_drop_action(action);
                        ui.close();
                    }
                }
            })
            .0
            .on_hover_text(
                "In Item mode with no tool active, drag an item onto another item to apply this",
            );
    }
    pub(super) fn set_drop_action(&mut self, action: DropAction) {
        if self.drop_action.action == action {
            return;
        }
        self.drop_action.action = action;
        if let Err(error) = self
            .drop_action
            .save(&self.environment.paths().drop_action())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
    /// DA-4/5: whether a primary press here starts a drop drag. Only in Item
    /// mode, with no tool or task, an action chosen and no modifier keys.
    pub(super) fn drop_press(&mut self, pos: Pos2, r: Rect, modifiers: Modifiers) -> bool {
        // CP-A09: per-item Source / Destination actions also start a drop.
        let overrides = self.editor.scene.items.iter().any(|i| {
            let d = &i.common.extras.drop_actions;
            d[1].is_some() || d[2].is_some()
        });
        // MN-07.3: with no action, a selected item can still be dragged to
        // a group in the Groups list.
        let no_action = self.drop_action.action == DropAction::None && !overrides;
        if !modifiers.is_none()
            || self.editor.scene.selection.mode != Mode::Item
            || self.editor.session.has_active_task()
            || self.editor.session.pen_draft.is_some()
            || self.draw.is_some()
            || self.utility.is_some()
            || self.edge_click.is_some()
            || self.preset_center_click
            || self.falloff_hit(pos, r).is_some()
            || self.item_handle_hit(pos, r).is_some()
        {
            return false;
        }
        let Some((source, Component::Item)) = self.pick_component(pos, r) else {
            return false;
        };
        if no_action && !self.drop_selection().contains(&source) {
            return false;
        }
        // Pressing an item outside the selection selects it, as a click
        // does; pressing one inside keeps a multi-item selection (DA-7).
        if !self.drop_selection().contains(&source) {
            self.run(Command::BeginSelection);
            self.select_at(pos, r, Modifiers::NONE);
            self.run(Command::EndSelection { cancel: false });
        }
        self.drop_drag = Some(DropDrag { source, start: pos });
        true
    }
    /// The group whose row in the Groups list is under `pos`, while the
    /// list shows (MN-07.3).
    pub(super) fn group_row_at(&self, pos: Pos2) -> Option<u64> {
        if !self.front_panel(crate::panel_registry::PanelKind::Groups) {
            return None;
        }
        self.groups_tree
            .rects
            .iter()
            .find_map(|(key, rect)| match key {
                crate::groups_ui::GroupKey::Group(g) if rect.contains(pos) => Some(*g),
                _ => None,
            })
    }
    /// The selected items in selection order.
    fn drop_selection(&self) -> Vec<u64> {
        let s = &self.editor.scene.selection;
        s.item.into_iter().chain(s.also.iter().copied()).collect()
    }
    /// The sources for a drop of `pressed` (DA-7, DA-Q4): the selection in
    /// order when the pressed item is in it, else the pressed item; the
    /// Insert family only the pressed item.
    pub(super) fn drop_sources(&self, action: DropAction, pressed: u64) -> Vec<u64> {
        let selection = self.drop_selection();
        if action.single_source() || !selection.contains(&pressed) {
            vec![pressed]
        } else {
            selection
        }
    }
    /// The item a drop at `pos` would land on: an item under the pointer
    /// other than the source (DA-6).
    pub(super) fn drop_action_target(&self, pos: Pos2, r: Rect) -> Option<u64> {
        let drag = self.drop_drag?;
        match self.pick_component(pos, r) {
            Some((id, Component::Item)) if id != drag.source => Some(id),
            _ => None,
        }
    }
    /// The release: a click selects as usual; a release on another item
    /// applies the action; empty space or the source does nothing (DA-6).
    pub(super) fn drop_release(&mut self, pos: Pos2, r: Rect) {
        let Some(drag) = self.drop_drag else {
            return;
        };
        let target = self.drop_action_target(pos, r);
        self.drop_drag = None;
        self.release_pointer();
        // MN-07.3: released over a group in the Groups list, the dragged
        // items join it (one undo step).
        if pos.distance(drag.start) >= CLICK_SLOP
            && let Some(group) = self.group_row_at(pos)
        {
            let items = self.drop_sources(DropAction::None, drag.source);
            let count = items.len();
            if self.run_ok(Command::AddToGroup { group, items }) {
                self.status = format!(
                    "Added {count} item{} to the group",
                    if count == 1 { "" } else { "s" }
                );
            }
            return;
        }
        if pos.distance(drag.start) < CLICK_SLOP {
            // An ordinary click: select just this item.
            self.run(Command::BeginSelection);
            self.select_at(drag.start, r, Modifiers::NONE);
            self.run(Command::EndSelection { cancel: false });
            return;
        }
        let Some(destination) = target else {
            self.status = "Drop Action: nothing to drop onto".into();
            return;
        };
        // CP-A09: the destination's or source's own action wins.
        let action =
            self.editor
                .scene
                .drop_action_for(drag.source, destination, self.drop_action.action);
        if action == DropAction::None {
            self.status = "Drop Action: none set".into();
            return;
        }
        let ray = action.needs_ray().then(|| {
            let (origin, direction) = self.camera.ray(pos, r);
            operandi_core::drop_action::Ray { origin, direction }
        });
        let sources = self.drop_sources(action, drag.source);
        let count = sources.len();
        if self.run_ok(Command::DropAction {
            action,
            sources,
            destination,
            ray,
        }) {
            let name = self
                .editor
                .scene
                .item(destination)
                .map(|i| i.name.clone())
                .unwrap_or_default();
            self.status = format!(
                "{}: {count} item{} onto {name}",
                action.label(),
                if count == 1 { "" } else { "s" }
            );
        }
    }
    /// Esc, a lost pointer or focus: the drag ends without a change.
    pub(super) fn cancel_drop(&mut self) -> bool {
        if self.drop_drag.take().is_some() {
            self.release_pointer();
            self.status = "Drop cancelled".into();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preference_round_trips_and_defaults_to_none() {
        let dir = std::env::temp_dir().join(format!("operandi-drop-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("drop-action.json");
        assert_eq!(DropActionPreferences::load(&path).action, DropAction::None);
        let p = DropActionPreferences {
            action: DropAction::MatchRotation,
        };
        p.save(&path).unwrap();
        assert_eq!(DropActionPreferences::load(&path), p);
        fs::write(&path, b"{\"action\":\"Nope\"}").unwrap();
        assert_eq!(DropActionPreferences::load(&path).action, DropAction::None);
        fs::remove_dir_all(dir).unwrap();
    }
}
