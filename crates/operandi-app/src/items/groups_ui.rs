//! The Groups tab next to Scenes and Shading (item_groups.rs in the core):
//! New Group from the selection, Rename (double-click), Delete Group, Add /
//! Remove Selected, click a group to select its members, expand it to see
//! them. Each change is one undo step; selecting isn't.
//!
//! Also the lights' Light Linking section (LP-30..32), which points at a
//! group.
use super::*;
use crate::tree_list::{
    self, Cell, ClickHow, Column, Heading, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow,
    TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::camera_light::{LightEdit, LightSettings, LinkMode};

/// The upper panel's Groups page index (default layout; tests).
#[cfg(test)]
pub(super) const GROUPS_PAGE: usize = 2;

fn selected_group_key() -> egui::Id {
    egui::Id::new("groups-selected")
}

fn open_key(group: u64) -> egui::Id {
    egui::Id::new(("group-open", group))
}

/// A row of the Groups list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GroupKey {
    Group(u64),
    /// A member item under its group.
    Member(u64, u64),
}

/// UIC-153: the visibility state column, then Name. The group-type column
/// is left out until Operandi has more than one kind of group (UIC-117;
/// TREE-Q03).
pub(crate) fn groups_spec() -> TreeSpec {
    TreeSpec {
        id: "groups-tree",
        columns: vec![
            Column::state(
                Heading::Eye,
                "Visibility of the group's items · drag down to set several",
            ),
            Column::name(),
        ],
        own_search: false,
        last_fills: false,
    }
}

/// Right-click menus of the Groups list.
struct GroupHooks<'a> {
    app: &'a mut Operandi,
    selection: Vec<u64>,
}

impl TreeHooks<GroupKey> for GroupHooks<'_> {
    fn has_row_menu(&self, _key: GroupKey) -> bool {
        self.app.editor.session.live.is_none()
    }
    fn row_menu(&mut self, ui: &mut Ui, key: GroupKey, rename: &mut dyn FnMut()) {
        self.app.menu_owned_input = true;
        match key {
            GroupKey::Group(g) => {
                if ui.button("Rename…").clicked() {
                    rename();
                    ui.close();
                }
                if ui
                    .button("Delete Group")
                    .on_hover_text("Delete the group; its items stay")
                    .clicked()
                {
                    self.app.run(Command::DeleteGroup(g));
                    ui.close();
                }
                ui.separator();
                let any = !self.selection.is_empty();
                if ui
                    .gated(
                        crate::unavailable::unless(any, "Select some items first"),
                        Button::new("Add Selected"),
                    )
                    .on_hover_text("Add the selected items to this group")
                    .clicked()
                {
                    self.app.run(Command::AddToGroup {
                        group: g,
                        items: self.selection.clone(),
                    });
                    ui.close();
                }
                if ui
                    .gated(
                        crate::unavailable::unless(any, "Select some items first"),
                        Button::new("Remove Selected"),
                    )
                    .on_hover_text("Take the selected items out of this group")
                    .clicked()
                {
                    self.app.run(Command::RemoveFromGroup {
                        group: g,
                        items: self.selection.clone(),
                    });
                    ui.close();
                }
            }
            GroupKey::Member(g, m) => {
                let current = self
                    .app
                    .editor
                    .scene
                    .item(m)
                    .and_then(|i| i.common.editor_color);
                if let Some(color) = crate::editor_color_ui::submenu(ui, current) {
                    let items = self.app.menu_targets(m);
                    self.app.set_item_editor_color(items, color);
                }
                if ui.button("Remove from Group").clicked() {
                    self.app.run(Command::RemoveFromGroup {
                        group: g,
                        items: vec![m],
                    });
                    ui.close();
                }
            }
        }
    }
}

impl Operandi {
    /// The group picked in the Groups tab, if it still exists.
    pub(super) fn picked_group(&self, ctx: &egui::Context) -> Option<u64> {
        ctx.data(|d| d.get_temp::<u64>(selected_group_key()))
            .filter(|g| self.editor.scene.group(*g).is_some())
    }

    /// Members in pick order.
    fn selected_items(&self) -> Vec<u64> {
        self.editor.scene.picked_items()
    }

    /// Item > New Group from Selection (and the Groups tab's New Group).
    pub(super) fn new_group_from_selection(&mut self, ctx: &egui::Context) {
        let members = self.selected_items();
        let before: Vec<u64> = self.editor.scene.groups.iter().map(|g| g.id).collect();
        if self.run_ok(Command::NewGroup {
            name: None,
            members,
        }) && let Some(g) = self
            .editor
            .scene
            .groups
            .iter()
            .find(|g| !before.contains(&g.id))
        {
            let (id, name, n) = (g.id, g.name.clone(), g.members.len());
            ctx.data_mut(|d| d.insert_temp(selected_group_key(), id));
            self.status = format!(
                "New group {name} with {n} item{}",
                if n == 1 { "" } else { "s" }
            );
        }
    }

    /// The Groups panel (UIC-153) on the shared tree list: New Group on the
    /// tool row, then each group with its members under it. A group's eye
    /// shows or hides all its members in one step. Delete Group and Add /
    /// Remove Selected are on a group's right-click menu.
    pub(super) fn groups_section(&mut self, ui: &mut Ui, _area: Rect) {
        let ctx = ui.ctx().clone();
        let picked = self.picked_group(&ctx);
        let mut state = std::mem::take(&mut self.groups_tree);
        let mut new_group = false;
        tree_list::tool_row(ui, &mut state, true, |ui, side| {
            if side == tree_list::Side::Left {
                new_group = ui
                    .gated(None, Button::new("New Group"))
                    .on_hover_text("A new group holding the selected items")
                    .clicked();
            }
        });
        if new_group {
            self.new_group_from_selection(&ctx);
        }
        let selection = self.selected_items();
        let spec = groups_spec();
        let mut rows = Vec::new();
        for g in &self.editor.scene.groups {
            let members = self.editor.scene.group_members(g.id);
            let open: bool = ctx.data(|d| d.get_temp(open_key(g.id)).unwrap_or(false));
            let mut row = TreeRow::new(GroupKey::Group(g.id), 0, g.name.clone(), 2);
            row.icon = RowIcon::Icon(crate::icons::Icon::TreeGroup);
            row.disclosure = (!members.is_empty()).then_some(open);
            row.selected = if picked == Some(g.id) {
                Selected::Primary
            } else {
                Selected::No
            };
            row.renamable = true;
            row.style.empty = members.is_empty();
            row.tip = Some(format!(
                "{} item{}",
                members.len(),
                if members.len() == 1 { "" } else { "s" }
            ));
            let visible = !members.is_empty()
                && members.iter().all(|m| {
                    self.editor
                        .scene
                        .items
                        .iter()
                        .any(|i| i.id == *m && i.visible)
                });
            if !members.is_empty() {
                row.cells[0] = Cell::Toggle {
                    dimmed: false,
                    on: visible,
                    tip: "Show or hide every item in the group".into(),
                };
            }
            rows.push(row);
            if open {
                for m in &members {
                    let Some(item) = self.editor.scene.item(*m) else {
                        continue;
                    };
                    let mut row = TreeRow::new(GroupKey::Member(g.id, *m), 1, item.name.clone(), 2);
                    row.icon = RowIcon::Icon(icons::Icon::Cube);
                    row.tint = item.common.editor_color.map(theme::editor_color);
                    row.style.dimmed = !item.visible;
                    row.selected = if self.editor.scene.selection.item == Some(*m) {
                        Selected::Primary
                    } else if selection.contains(m) {
                        Selected::Yes
                    } else {
                        Selected::No
                    };
                    row.cells[0] = Cell::Toggle {
                        dimmed: false,
                        on: item.visible,
                        tip: "Show or hide this item".into(),
                    };
                    rows.push(row);
                }
            }
        }
        let mut editors = std::mem::take(&mut self.text_editors);
        let mut hooks = GroupHooks {
            app: self,
            selection: selection.clone(),
        };
        let events = tree_list::show(ui, &spec, &mut state, &rows, &mut hooks, &mut editors);
        self.text_editors = editors;
        self.groups_tree = state;
        for event in events {
            match event {
                TreeEvent::Click {
                    key: GroupKey::Group(g),
                    ..
                }
                | TreeEvent::DoubleClick(GroupKey::Group(g)) => self.pick_group(&ctx, g),
                TreeEvent::Click {
                    key: GroupKey::Member(_, m),
                    how,
                } => match how {
                    ClickHow::Toggle => self.run(Command::ToggleItemSelection(m)),
                    _ => self.run(Command::SelectItem(Some(m))),
                },
                TreeEvent::DoubleClick(_) | TreeEvent::Attributes(_) | TreeEvent::Drop { .. } => {}
                TreeEvent::Disclosure {
                    key: GroupKey::Group(g),
                    ..
                } => {
                    let open: bool = ctx.data(|d| d.get_temp(open_key(g)).unwrap_or(false));
                    ctx.data_mut(|d| d.insert_temp(open_key(g), !open));
                }
                TreeEvent::Disclosure { .. } => {}
                TreeEvent::Rename {
                    key: GroupKey::Group(g),
                    name,
                } => self.run(Command::RenameGroup { group: g, name }),
                TreeEvent::Rename { .. } => {}
                TreeEvent::Toggle { key, .. } => {
                    let (items, on) = self.group_eye(key);
                    if !items.is_empty() {
                        self.run(Command::SetItemsVisible {
                            items,
                            visible: !on,
                            hides_children: false,
                        });
                    }
                }
                TreeEvent::Sweep { keys, on, .. } => {
                    let mut items = Vec::new();
                    for k in keys {
                        items.extend(self.group_eye(k).0);
                    }
                    if !items.is_empty() {
                        self.run(Command::SetItemsVisible {
                            items,
                            visible: on,
                            hides_children: false,
                        });
                    }
                }
            }
        }
    }

    /// Pick a group and select its members (not an undo step).
    fn pick_group(&mut self, ctx: &egui::Context, g: u64) {
        ctx.data_mut(|d| d.insert_temp(selected_group_key(), g));
        let n = self.editor.scene.group_members(g).len();
        let name = self
            .editor
            .scene
            .group(g)
            .map(|x| x.name.clone())
            .unwrap_or_default();
        self.run(Command::SelectGroup(g));
        self.status = format!(
            "Selected the {n} item{} of {name}",
            if n == 1 { "" } else { "s" }
        );
    }

    /// The items an eye click changes, and whether they show now.
    fn group_eye(&self, key: GroupKey) -> (Vec<u64>, bool) {
        let items = match key {
            GroupKey::Group(g) => self.editor.scene.group_members(g),
            GroupKey::Member(_, m) => vec![m],
        };
        let on = items.iter().all(|m| {
            self.editor
                .scene
                .items
                .iter()
                .any(|i| i.id == *m && i.visible)
        });
        (items, on)
    }

    /// LP-30..32 rows: Control Light Linking, then Item Group and Mode,
    /// greyed while Control is off (LPQ-03).
    pub(super) fn light_linking_rows(&mut self, ui: &mut Ui, id: u64, l: &LightSettings) {
        let link = l.linking.clone().unwrap_or_default();
        let edit = |e: LightEdit| Command::EditLight { item: id, edit: e };
        let mut control = link.control;
        if crate::option_row::check(ui, "Control Light Linking", &mut control)
            .on_hover_text("Light only a group's items (Include) or everything but them (Exclude)")
            .changed()
        {
            self.run(edit(LightEdit::LinkControl(control)));
        }
        let groups: Vec<(u64, String)> = self
            .editor
            .scene
            .groups
            .iter()
            .map(|g| (g.id, g.name.clone()))
            .collect();
        ui.gated_ui(
            crate::unavailable::unless(link.control, "Turn on Control Light Linking first"),
            |ui| {
                let shown = link.group.map_or("(none)".to_string(), |g| {
                    groups
                        .iter()
                        .find(|x| x.0 == g)
                        .map_or("(missing)".to_string(), |x| x.1.clone())
                });
                let picked = crate::option_row::dropdown(
                    ui,
                    &format!("light-link-group-{id}"),
                    "Item Group",
                    &shown,
                    |ui| {
                        let mut picked = None;
                        if ui
                            .selectable_label(link.group.is_none(), "(none)")
                            .clicked()
                        {
                            picked = Some(None);
                        }
                        for (g, name) in &groups {
                            if ui.selectable_label(link.group == Some(*g), name).clicked() {
                                picked = Some(Some(*g));
                            }
                        }
                        picked
                    },
                )
                .inner;
                if let Some(g) = picked
                    && g != link.group
                {
                    self.run(edit(LightEdit::LinkGroup(g)));
                }
                let modes = LinkMode::ALL.map(|m| (m, m.label()));
                let mut mode = link.mode;
                crate::option_row::choice(
                    ui,
                    &format!("light-link-mode-{id}"),
                    "Mode",
                    &mut mode,
                    &modes,
                )
                .response
                .on_hover_text(
                    "Include: lights only the group. Exclude: lights everything but the group",
                );
                if mode != link.mode {
                    self.run(edit(LightEdit::LinkMode(mode)));
                }
            },
        );
    }
}
