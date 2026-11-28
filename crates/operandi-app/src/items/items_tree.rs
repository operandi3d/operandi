//! The Items list on the shared tree-list component (UIC-150; IL-00..08,
//! MULTI-SCENE). Every open scene is a root row (the current one bold, with
//! "*" while it has unsaved changes); the current scene's items sit under
//! it, filtered and sorted by the core's `item_list::list_rows`, and the
//! other scenes' items under theirs. This file only builds rows and turns
//! the list's events into the same commands as before.
use super::*;
use crate::tree_list::{
    self, Badge, Cell, ClickHow, Column, DropAt, Heading, NameStyle, RowIcon, Selected, TreeEvent,
    TreeHooks, TreeRow, TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::item_list::ListRow;

/// A row of the Items list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ItemKey {
    /// A scene's root row, by scene tab id.
    Scene(u64),
    /// An item, by scene tab id and item id.
    Item(u64, u64),
    /// The "(new scene)" row at the end.
    NewScene,
    /// A rig piece under its item (RG-01, IK-02), by scene and piece id.
    Rig(u64, u64),
    /// A skin under its mesh (SK-36), by scene, mesh and op id.
    Skin(u64, u64, u64),
    /// One joint influence under a skin, by scene, mesh and joint.
    Influence(u64, u64, u64),
}

/// Column indexes.
pub(crate) const EYE: usize = 0;
pub(crate) const EDIT: usize = 1;
pub(crate) const NAME: usize = 2;

/// UIC-150: visibility and edit-mode state columns, then Name; no value
/// columns. The world/local space column is left out until reference
/// spaces exist (UIC-117; TREE-Q01).
pub(crate) fn spec() -> TreeSpec {
    TreeSpec {
        id: "items-tree",
        columns: vec![
            Column::state(
                Heading::Eye,
                "Visibility · click to show or hide, drag down to set several, \
                 Alt+click to show only this one",
            ),
            Column::state(
                Heading::Icon(crate::icons::Icon::MeshEditMono),
                "Edit mode: the active mesh, the one component tools work on. \
                 Click to make an item the active mesh",
            ),
            Column::name(),
        ],
        own_search: true,
        last_fills: false,
    }
}

/// A parked scene's item row: id, parent, name, visible, depth, has
/// children, folded.
type ParkedRow = (u64, Option<u64>, String, bool, usize, bool, bool);

impl Operandi {
    /// The Items panel: tool row, then the tree (UIC-131).
    pub(super) fn items_body(&mut self, ui: &mut Ui, _area: Rect) {
        // The core's IL-07 filter holds the search; the list's search row
        // edits it.
        // A popover copy beside the docked list (panel_popover.rs) keeps
        // its own scroll, drag and rename state; the rows are the same.
        let copy = tree_list::drawing_copy(ui.ctx());
        if copy {
            std::mem::swap(&mut self.items_tree, &mut self.items_tree_copy);
        }
        {
            let f = &self.item_list.filter;
            let s = &mut self.items_tree.search;
            s.open = f.text.is_some();
            s.text = f.text.clone().unwrap_or_default();
            s.flat = f.text_flat;
            s.wildcard = f.text_pattern;
            s.match_case = f.text_case;
        }
        self.item_list_header(ui);
        if self.items_tree.search.open != self.item_list.filter.text.is_some() {
            // The funnel was clicked: off clears the search (IL-07).
            self.item_list.filter.text = self.items_tree.search.open.then(String::new);
            self.items_tree.search.text.clear();
        }
        let spec = spec();
        let rows = self.item_rows();
        if let Some(id) = self.rename_item.take() {
            let here = self.scenes.active_tab().id;
            let name = self.name.clone();
            self.items_tree.start_rename(ItemKey::Item(here, id), &name);
        }
        let mut state = std::mem::take(&mut self.items_tree);
        let mut editors = std::mem::take(&mut self.text_editors);
        let mut hooks = ItemsHooks {
            app: self,
            action: None,
        };
        let events = tree_list::show(ui, &spec, &mut state, &rows, &mut hooks, &mut editors);
        let menu_action = hooks.action.take();
        self.text_editors = editors;
        // Write the search row back into the filter.
        if state.search.open {
            let f = &mut self.item_list.filter;
            f.text = Some(state.search.text.clone());
            f.text_flat = state.search.flat;
            f.text_pattern = state.search.wildcard;
            f.text_case = state.search.match_case;
        }
        // Rects for tests and the drag code elsewhere: the copy the
        // pointer is over (the docked one unless it is over the popover).
        let here = self.scenes.active_tab().id;
        let pointer_here = ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| ui.clip_rect().contains(p));
        if !copy || pointer_here {
            self.scene_rows.clear();
            self.scene_root_rect = None;
            self.scenes.drop_rects.clear();
        }
        for (key, _) in state.rects.iter().filter(|_| !copy || pointer_here) {
            let cell = state.cells.get(&(*key, NAME)).copied();
            match (*key, cell) {
                (ItemKey::Item(s, id), Some(cell)) if s == here => {
                    self.scene_rows.insert(id, cell);
                }
                (ItemKey::Scene(s), Some(cell)) if s == here => self.scene_root_rect = Some(cell),
                // Other scenes' rows: where a drop moves items across.
                (ItemKey::Scene(s), Some(cell)) => self.scenes.drop_rects.push((cell, s, None)),
                (ItemKey::Item(s, id), Some(cell)) => {
                    self.scenes.drop_rects.push((cell, s, Some(id)));
                }
                _ => {}
            }
        }
        self.items_tree = state;
        if copy {
            std::mem::swap(&mut self.items_tree, &mut self.items_tree_copy);
        }
        let order: Vec<u64> = rows
            .iter()
            .filter_map(|r| match r.key {
                ItemKey::Item(s, id) if s == here => Some(id),
                _ => None,
            })
            .collect();
        let mut action = menu_action;
        for event in events {
            action = action.or(self.item_event(ui, event, &order));
        }
        if let Some(action) = self.scenes.pending.take().or(action) {
            self.scene_row_action(action);
        }
    }

    /// Every row of the list, top to bottom.
    pub(super) fn item_rows(&self) -> Vec<TreeRow<ItemKey>> {
        let columns = 3;
        let mut rows = Vec::new();
        let here = self.scenes.active;
        for i in 0..self.scenes.tabs.len() {
            let id = self.scenes.tabs[i].id;
            let current = i == here;
            let (has_items, collapsed, dirty) = if current {
                (
                    !self.editor.scene.items.is_empty(),
                    self.scene_collapsed,
                    self.dirty(),
                )
            } else {
                match self.scenes.tabs[i].parked.as_ref() {
                    Some(s) => (
                        !s.editor.scene.items.is_empty(),
                        s.scene_collapsed,
                        s.dirty(),
                    ),
                    None => (false, true, false),
                }
            };
            let mut root = TreeRow::new(ItemKey::Scene(id), 0, self.scene_row_name(i), columns);
            root.icon = RowIcon::Icon(icons::Icon::Scene);
            root.disclosure = has_items.then_some(!collapsed);
            // UIC-141: the current scene's row is bold, "*" when unsaved.
            root.style.bold = current;
            root.style.unsaved = dirty;
            root.accepts_children = true;
            root.fixed = true;
            root.tip = Some(if current {
                "The current scene. Drop an item here to unparent it.".into()
            } else {
                "Click to make this scene current".into()
            });
            rows.push(root);
            if collapsed {
                continue;
            }
            if current {
                self.current_item_rows(id, &mut rows);
            } else {
                self.parked_item_rows(i, &mut rows);
            }
        }
        let mut new = TreeRow::new(ItemKey::NewScene, 0, "(new scene)", columns);
        new.style.empty = true;
        new.fixed = true;
        new.tip = Some("Click to create a new scene".into());
        rows.push(new);
        rows
    }

    fn current_item_rows(&self, scene: u64, rows: &mut Vec<TreeRow<ItemKey>>) {
        let list = operandi_core::item_list::list_rows(
            &self.editor.scene,
            &self.item_list.filter,
            &self.collapsed_items,
        );
        let items: std::collections::HashMap<u64, &operandi_core::Item> =
            self.editor.scene.items.iter().map(|i| (i.id, i)).collect();
        let s = &self.editor.scene.selection;
        // A set: searching `also` per row cost the square of the item count
        // with thousands selected (panel_speed_tests.rs).
        let also: operandi_core::fxhash::FxHashSet<u64> = s.also.iter().copied().collect();
        let render_camera = self.editor.scene.render_camera();
        for ListRow {
            id,
            depth,
            dots,
            greyed,
            has_children,
        } in list
        {
            let Some(item) = items.get(&id) else {
                continue;
            };
            // Flatten: one dot per nesting level (IL-06).
            let name = if dots > 0 {
                format!("{} {}", "·".repeat(dots), item.name)
            } else {
                item.name.clone()
            };
            let rig = operandi_core::rig_list::rows(&self.editor.scene, id);
            let mut row = TreeRow::new(ItemKey::Item(scene, id), depth, name, 3);
            row.disclosure =
                (has_children || !rig.is_empty()).then_some(!self.collapsed_items.contains(&id));
            row.icon = match camera_light_ui::LocatorGlyph::of(item) {
                Some(glyph) => RowIcon::Locator(glyph),
                None => RowIcon::Icon(item_type_icon(item)),
            };
            let plain_mesh = item.backdrop.is_none()
                && item.camera.is_none()
                && item.light.is_none()
                && item.texture.is_none()
                && item.locator.is_none()
                && item.environment.is_none()
                && item.kit.is_none();
            row.style = NameStyle {
                // CL-21: the render camera's row is bold, with its marker.
                bold: render_camera == Some(id),
                empty: plain_mesh && item.mesh.vertices.is_empty(),
                // Kept only to place a shown child (IL-06/08), or hidden.
                dimmed: greyed || !item.visible,
                ..NameStyle::default()
            };
            row.selected = if s.item == Some(id) {
                Selected::Primary
            } else if also.contains(&id) {
                Selected::Yes
            } else {
                Selected::No
            };
            if render_camera == Some(id) {
                row.badges.push(Badge::RenderCamera);
            }
            if item.is_locked() {
                row.badges.push(Badge::Locked);
            }
            // KIT-1: an item whose kit isn't installed loads, shows here
            // with its badge, and saves back unchanged.
            if item.kit.as_ref().is_some_and(|k| !k.installed()) {
                row.badges.push(Badge::MissingKit);
            }
            // GV-05: a child hidden by a hidden parent keeps its own eye,
            // drawn faded.
            let under_hidden = self.editor.scene.hidden_by_parent_eye(item);
            row.cells[EYE] = Cell::Toggle {
                on: item.visible,
                tip: match (item.visible, under_hidden) {
                    (_, true) => "Hidden by its parent · Alt+click: show only this one",
                    (true, false) => "Hide · Alt+click: show only this one",
                    (false, false) => "Show · Alt+click: show only this one",
                }
                .into(),
                dimmed: under_hidden,
            };
            // N18: only meshes can be the active (editable) mesh.
            if item.is_mesh() {
                row.cells[EDIT] = Cell::Toggle {
                    on: s.item == Some(id),
                    tip: "The active mesh (edit mode)".into(),
                    dimmed: false,
                };
            }
            row.tint = item.common.editor_color.map(theme::editor_color);
            // EP §Apply: the only place the "from preset" marker shows.
            if let Some(name) = &item.common.from_preset {
                row.tip = Some(format!("From preset: {name}"));
            }
            if let Some(kit) = item.kit.as_ref().filter(|k| !k.installed()) {
                row.tip = Some(kit.needs_text());
            }
            row.accepts_children = true;
            row.draggable = self.editor.live.is_none();
            row.renamable = self.editor.live.is_none();
            rows.push(row);
            if !self.collapsed_items.contains(&id) {
                self.rig_rows(scene, id, depth + 1, rig, rows);
            }
        }
    }

    /// The rig rows under item `item` (RG-01, IK-02, SK-36): its pieces,
    /// then its skin and, opened, the skin's joint influences.
    fn rig_rows(
        &self,
        scene: u64,
        item: u64,
        depth: usize,
        rig: Vec<operandi_core::rig_list::RigRow>,
        rows: &mut Vec<TreeRow<ItemKey>>,
    ) {
        use operandi_core::rig_list::RigRow;
        let selected = |key: ItemKey| {
            if self.rig_row == Some(key) {
                Selected::Primary
            } else {
                Selected::No
            }
        };
        let eye = |on: bool, what: &str| Cell::Toggle {
            on,
            tip: format!("{} {what}", if on { "Disable" } else { "Enable" }),
            dimmed: false,
        };
        for r in rig {
            match r {
                RigRow::Piece {
                    id,
                    kind,
                    name,
                    enabled,
                    missing,
                } => {
                    let key = ItemKey::Rig(scene, id);
                    let mut row = TreeRow::new(key, depth, name, 3);
                    row.icon = RowIcon::Icon(rig_icon(&kind));
                    row.style.dimmed = !enabled || missing;
                    row.cells[EYE] = eye(enabled, "this rig piece");
                    row.selected = selected(key);
                    row.renamable = self.editor.live.is_none();
                    row.fixed = true;
                    row.tip = Some(if missing {
                        format!("{kind}: made by another Operandi; kept as it is")
                    } else {
                        "A rig piece: right-click for its menu, double-click to rename".into()
                    });
                    rows.push(row);
                }
                RigRow::Skin {
                    op,
                    enabled,
                    influences,
                } => {
                    let key = ItemKey::Skin(scene, item, op);
                    let open = self.open_skins.contains(&(item, op));
                    let mut row = TreeRow::new(key, depth, "Skin", 3);
                    row.icon = RowIcon::Icon(icons::Icon::SkinMesh);
                    row.disclosure = (!influences.is_empty()).then_some(open);
                    row.style.dimmed = !enabled;
                    row.cells[EYE] = eye(enabled, "the skin");
                    row.selected = selected(key);
                    row.fixed = true;
                    row.tip = Some(format!("{} joint influences", influences.len()));
                    rows.push(row);
                    if !open {
                        continue;
                    }
                    for inf in influences {
                        let key = ItemKey::Influence(scene, item, inf.joint);
                        let mut row = TreeRow::new(key, depth + 1, inf.name, 3);
                        row.icon = RowIcon::Icon(icons::Icon::SkeletonTool);
                        row.style.dimmed = !inf.enabled || inf.missing;
                        row.cells[EYE] = eye(inf.enabled, "this joint's influence");
                        row.selected = selected(key);
                        row.fixed = true;
                        row.tip = Some(format!("Weight map {}", inf.map));
                        rows.push(row);
                    }
                }
            }
        }
    }

    /// Another open scene's items, in the same order and indentation as
    /// the current scene's.
    fn parked_item_rows(&self, i: usize, rows: &mut Vec<TreeRow<ItemKey>>) {
        let tab = &self.scenes.tabs[i];
        let Some(s) = tab.parked.as_ref() else {
            return;
        };
        for (item, _, name, visible, depth, children, folded) in parked_rows(s) {
            let mut row = TreeRow::new(ItemKey::Item(tab.id, item), depth, name, 3);
            row.disclosure = children.then_some(!folded);
            row.icon = RowIcon::Icon(icons::Icon::Cube);
            row.style.dimmed = !visible;
            row.accepts_children = true;
            row.draggable = true;
            rows.push(row);
        }
    }

    /// One list event; scene-level requests come back as an action.
    fn item_event(
        &mut self,
        ui: &Ui,
        event: TreeEvent<ItemKey>,
        order: &[u64],
    ) -> Option<scenes::SceneRowAction> {
        use scenes::SceneRowAction as A;
        let here = self.scenes.active_tab().id;
        let index_of = |app: &Self, scene: u64| app.scenes.tabs.iter().position(|t| t.id == scene);
        match event {
            TreeEvent::Click { key, how } => match key {
                ItemKey::NewScene => return Some(A::New),
                ItemKey::Scene(s) if s != here => return index_of(self, s).map(A::Switch),
                ItemKey::Scene(_) => {}
                ItemKey::Item(s, id) if s != here => {
                    return index_of(self, s).map(|i| A::SelectItem(i, id));
                }
                // A rig row selects the item it belongs to (an influence,
                // its joint) and stays marked.
                ItemKey::Rig(_, piece) => {
                    if let Some(p) = self.editor.scene.animation.rig.piece(piece) {
                        let owner = p.item;
                        self.run(Command::SelectItem(Some(owner)));
                        self.rig_row = Some(key);
                    }
                }
                ItemKey::Skin(_, item, _) => {
                    self.run(Command::SelectItem(Some(item)));
                    self.rig_row = Some(key);
                }
                ItemKey::Influence(_, _, joint) => {
                    if self.editor.scene.item(joint).is_some() {
                        self.run(Command::SelectItem(Some(joint)));
                    }
                    self.rig_row = Some(key);
                }
                ItemKey::Item(_, id) => {
                    if !self.confirm_fields() {
                        return None;
                    }
                    self.rig_row = None;
                    match how {
                        ClickHow::Only => self.run(Command::SelectItem(Some(id))),
                        ClickHow::Toggle => self.run(Command::ToggleItemSelection(id)),
                        ClickHow::Range => {
                            let mut selected = self.editor.scene.selected_items();
                            let mut anchor = self
                                .items_tree
                                .anchor
                                .and_then(|k| match k {
                                    ItemKey::Item(s, a) if s == here => Some(a),
                                    _ => None,
                                })
                                .or(self.editor.scene.selection.item);
                            tree_list::click_select(
                                &mut selected,
                                &mut anchor,
                                order,
                                id,
                                ClickHow::Range,
                            );
                            // Pick order: the anchor first, the range, and
                            // the clicked end last, which becomes primary
                            // (SelectItems takes the primary first).
                            if let Some(clicked) = selected.pop() {
                                selected.insert(0, clicked);
                            }
                            self.run(Command::SelectItems(selected));
                        }
                    }
                    self.focus_scene_list(ui, crate::panel_registry::PanelKind::Items);
                }
            },
            // UIC-38: a parent's double-click selects its children too.
            TreeEvent::DoubleClick(ItemKey::Item(s, id)) if s == here => {
                let mut family = vec![id];
                let mut at = 0;
                while let Some(&parent) = family.get(at) {
                    at += 1;
                    family.extend(
                        self.editor
                            .scene
                            .items
                            .iter()
                            .filter(|i| i.parent == Some(parent))
                            .map(|i| i.id),
                    );
                }
                if family.len() > 1 {
                    self.run(Command::SelectItems(family));
                }
            }
            TreeEvent::DoubleClick(_) => {}
            TreeEvent::Disclosure { key, subtree } => self.item_disclosure(key, subtree),
            TreeEvent::Attributes(_) => {}
            TreeEvent::Rename {
                key: ItemKey::Rig(_, piece),
                name,
            } => {
                self.execute(&format!(
                    "operandi.rig.rename piece:{piece} name:{}",
                    quoted(&name)
                ));
            }
            TreeEvent::Rename { key, name } => {
                if let ItemKey::Item(s, id) = key
                    && s == here
                {
                    self.run(Command::SelectItem(Some(id)));
                    self.name = name.clone();
                    self.rename_active(name);
                }
            }
            TreeEvent::Toggle {
                key: ItemKey::Item(s, id),
                column,
                solo,
            } if s == here => {
                if column == EDIT {
                    self.run(Command::SelectItem(Some(id)));
                } else if solo {
                    self.item_solo(id);
                } else {
                    // GV-01..04, 06: any item's eye; Ctrl/Cmd+click is the
                    // same as a plain click. With Auto Hide Child Items on,
                    // hiding also hides the subtree through this item (the
                    // children keep their own eyes); off, only the item.
                    let visible = self
                        .editor
                        .scene
                        .items
                        .iter()
                        .any(|i| i.id == id && i.visible);
                    self.run(Command::SetItemsVisible {
                        items: vec![id],
                        visible: !visible,
                        hides_children: self.mesh_items_preferences.auto_hide_children,
                    });
                }
            }
            TreeEvent::Toggle {
                key: ItemKey::Rig(_, piece),
                ..
            } => {
                if let Some(p) = self.editor.scene.animation.rig.piece(piece) {
                    let on = p.flag("enable");
                    self.execute(&format!(
                        "operandi.rig.set piece:{piece} channel:enable value:{}",
                        if on { 0 } else { 1 }
                    ));
                }
            }
            TreeEvent::Toggle {
                key: ItemKey::Skin(_, item, op),
                ..
            } => {
                let on = self
                    .editor
                    .scene
                    .item(item)
                    .and_then(|i| i.mesh_stack.as_ref())
                    .and_then(|st| st.ops.iter().find(|o| o.id == op))
                    .is_some_and(|o| o.enabled);
                self.execute(&format!(
                    "deformer.enable deformer:op:{item}:{op} enable:{}",
                    !on
                ));
            }
            TreeEvent::Toggle {
                key: ItemKey::Influence(_, item, joint),
                ..
            } => {
                let on = operandi_core::rig_list::rows(&self.editor.scene, item)
                    .into_iter()
                    .find_map(|r| match r {
                        operandi_core::rig_list::RigRow::Skin { influences, .. } => influences
                            .into_iter()
                            .find(|i| i.joint == joint)
                            .map(|i| i.enabled),
                        _ => None,
                    })
                    .unwrap_or(true);
                self.execute(&format!(
                    "operandi.skin.set item:{item} joint:{joint} enable:{}",
                    !on
                ));
            }
            TreeEvent::Toggle { .. } => {}
            TreeEvent::Sweep { column, keys, on } if column == EYE => {
                let items: Vec<u64> = keys
                    .into_iter()
                    .filter_map(|k| match k {
                        ItemKey::Item(s, id) if s == here => Some(id),
                        _ => None,
                    })
                    .collect();
                if !items.is_empty() {
                    self.run(Command::SetItemsVisible {
                        items,
                        visible: on,
                        hides_children: self.mesh_items_preferences.auto_hide_children,
                    });
                }
            }
            TreeEvent::Sweep { .. } => {}
            TreeEvent::Drop { keys, at, alt } => return self.item_drop(&keys, at, alt),
        }
        None
    }

    fn item_disclosure(&mut self, key: ItemKey, subtree: bool) {
        let here = self.scenes.active_tab().id;
        match key {
            ItemKey::Scene(s) if s == here => self.scene_collapsed = !self.scene_collapsed,
            ItemKey::Scene(s) => {
                if let Some(p) = self
                    .scenes
                    .tabs
                    .iter_mut()
                    .find(|t| t.id == s)
                    .and_then(|t| t.parked.as_mut())
                {
                    p.scene_collapsed = !p.scene_collapsed;
                }
            }
            ItemKey::Item(s, id) if s == here => {
                let close = !self.collapsed_items.contains(&id);
                let items = &self.editor.scene.items;
                let mut branch = vec![id];
                if subtree {
                    // UIC-37: Shift opens or closes the whole subtree.
                    let mut at = 0;
                    while let Some(&parent) = branch.get(at) {
                        at += 1;
                        branch.extend(
                            items
                                .iter()
                                .filter(|i| i.parent == Some(parent))
                                .map(|i| i.id),
                        );
                    }
                }
                for id in branch {
                    if close {
                        self.collapsed_items.insert(id);
                    } else {
                        self.collapsed_items.remove(&id);
                    }
                }
            }
            ItemKey::Item(s, id) => {
                if let Some(p) = self
                    .scenes
                    .tabs
                    .iter_mut()
                    .find(|t| t.id == s)
                    .and_then(|t| t.parked.as_mut())
                    && !p.collapsed_items.remove(&id)
                {
                    p.collapsed_items.insert(id);
                }
            }
            ItemKey::Skin(_, item, op) => {
                if !self.open_skins.remove(&(item, op)) {
                    self.open_skins.insert((item, op));
                }
            }
            ItemKey::NewScene | ItemKey::Rig(..) | ItemKey::Influence(..) => {}
        }
    }

    /// UIC-138 Alt+click: show only this item; Alt+click it again brings
    /// the others back. One undo step each way.
    fn item_solo(&mut self, id: u64) {
        if let Some((solo, hidden)) = self.item_solo.take()
            && solo == id
        {
            let hidden: Vec<u64> = hidden
                .into_iter()
                .filter(|h| self.editor.scene.item(*h).is_some())
                .collect();
            if !hidden.is_empty() {
                self.run(Command::SetItemsVisible {
                    items: hidden,
                    visible: true,
                    hides_children: false,
                });
            }
            return;
        }
        // Solo keeps the item's whole subtree showing (GV-06). Every other
        // row is hidden on its own (plain off), so hidden ancestors don't
        // hide the soloed item; a child hidden on its own stays hidden
        // (GV-02). An ancestor whose eye hides its children is turned to a
        // plain off for the same reason (Alt+click again shows it).
        let items = &self.editor.scene.items;
        let mut keep = vec![id];
        let mut at = 0;
        while let Some(&parent) = keep.get(at) {
            at += 1;
            keep.extend(
                items
                    .iter()
                    .filter(|i| i.parent == Some(parent))
                    .map(|i| i.id),
            );
        }
        let me = items.iter().find(|i| i.id == id);
        let shown = me.is_some_and(|i| i.visible);
        let mut others: Vec<u64> = items
            .iter()
            // Only rows of this list: environments keep their eye (§1).
            .filter(|i| {
                !keep.contains(&i.id)
                    && !i.is_locked()
                    && i.in_item_list()
                    && (i.visible || i.hides_children)
            })
            .map(|i| i.id)
            .collect();
        others.retain(|o| {
            // Hidden-with-children items that aren't ancestors stay as they are.
            let item = items.iter().find(|i| i.id == *o);
            item.is_some_and(|i| i.visible)
                || me.is_some_and(|m| is_ancestor(&self.editor.scene, *o, m))
        });
        if !shown {
            self.run(Command::SetItemsVisible {
                items: vec![id],
                visible: true,
                hides_children: false,
            });
        }
        if !others.is_empty() {
            self.run(Command::SetItemsVisible {
                items: others.clone(),
                visible: false,
                hides_children: false,
            });
        }
        self.item_solo = Some((id, others));
    }

    /// UIC-143: dropping on a row parents the items to it (as its last
    /// children); between rows puts them there, at that row's level; below
    /// the last row or on a scene row makes them the last top-level items.
    /// Within a scene that is one `MoveItems` (one undo step, "Reorder"
    /// or "Parent"/"Unparent"); between scenes it copies or moves
    /// (scenes.rs). Reparented items keep their world position.
    fn item_drop(
        &mut self,
        keys: &[ItemKey],
        at: DropAt<ItemKey>,
        alt: bool,
    ) -> Option<scenes::SceneRowAction> {
        let here = self.scenes.active_tab().id;
        let dragged: Vec<(u64, u64)> = keys
            .iter()
            .filter_map(|k| match *k {
                ItemKey::Item(s, id) => Some((s, id)),
                _ => None,
            })
            .collect();
        let &(source, first) = dragged.first()?;
        let parent_in = |app: &Self, scene: u64, id: u64| -> Option<u64> {
            if scene == here {
                app.editor.scene.item(id).and_then(|i| i.parent)
            } else {
                app.scenes
                    .tabs
                    .iter()
                    .find(|t| t.id == scene)
                    .and_then(|t| t.parked.as_ref())
                    .and_then(|p| p.editor.scene.item(id))
                    .and_then(|i| i.parent)
            }
        };
        let (scene, parent) = match at {
            DropAt::On(ItemKey::Item(s, id)) => (s, Some(id)),
            DropAt::On(ItemKey::Scene(s))
            | DropAt::Before(ItemKey::Scene(s))
            | DropAt::After(ItemKey::Scene(s)) => (s, None),
            DropAt::Before(ItemKey::Item(s, id)) | DropAt::After(ItemKey::Item(s, id)) => {
                (s, parent_in(self, s, id))
            }
            DropAt::On(ItemKey::NewScene)
            | DropAt::Before(ItemKey::NewScene)
            | DropAt::After(ItemKey::NewScene)
            | DropAt::End => (source, None),
            // Rig rows take no drops.
            DropAt::On(ItemKey::Rig(..) | ItemKey::Skin(..) | ItemKey::Influence(..))
            | DropAt::Before(ItemKey::Rig(..) | ItemKey::Skin(..) | ItemKey::Influence(..))
            | DropAt::After(ItemKey::Rig(..) | ItemKey::Skin(..) | ItemKey::Influence(..)) => {
                return None;
            }
        };
        if scene != source {
            // A plain drop moves, Option copies (scenes::DRAG_COPIES).
            return Some(scenes::SceneRowAction::Transfer {
                from: source,
                item: first,
                to: scene,
                parent,
                moving: alt == scenes::DRAG_COPIES,
            });
        }
        if source != here {
            return None;
        }
        let items: Vec<u64> = dragged
            .iter()
            .filter(|(s, _)| *s == here)
            .map(|&(_, id)| id)
            .collect();
        let before = match at {
            DropAt::Before(ItemKey::Item(s, id)) if s == here => Some(id),
            DropAt::After(ItemKey::Item(s, id)) if s == here => {
                item_drop_after(&self.editor.scene, id, parent)
            }
            _ => None,
        };
        self.run(Command::MoveItems {
            items,
            parent,
            before,
        });
        None
    }

    /// The right-click menu of one of the current scene's items.
    fn item_row_menu(&mut self, ui: &mut Ui, id: u64) {
        let Some(item) = self.editor.scene.item(id).cloned() else {
            return;
        };
        self.menu_owned_input = true;
        self.parent_menu(ui, item.id, item.parent);
        // Note 10: every selected item when the clicked one is selected.
        if self.editor.live.is_none()
            && let Some(color) = crate::editor_color_ui::submenu(ui, item.common.editor_color)
        {
            let items = self.menu_targets(item.id);
            self.set_item_editor_color(items, color);
        }
        ui.separator();
        // LCK-01: every selected item when the clicked one is selected,
        // else just the clicked one; one undo step.
        let locked = item.is_locked();
        if ui
            .gated(None, Button::new(if locked { "Unlock" } else { "Lock" }))
            .clicked()
        {
            let mut items = self.editor.scene.selected_items();
            if !items.contains(&item.id) {
                items = vec![item.id];
            }
            self.run(Command::EditItemCommon {
                items,
                edit: operandi_core::item_common::ItemCommonEdit::Lock(if locked {
                    operandi_core::item_common::Choice::No
                } else {
                    operandi_core::item_common::Choice::Yes
                }),
            });
            ui.close();
        }
        if ui.gated(None, Button::new("Delete Item")).clicked() {
            self.run(Command::SelectItem(Some(item.id)));
            self.request_delete();
            ui.close();
        }
        if ui.gated(None, Button::new("Rename…")).clicked() {
            self.run(Command::SelectItem(Some(item.id)));
            self.name = item.name.clone();
            self.rename_item = Some(item.id);
            ui.close();
        }
        // CL-21 / CL-G12: a camera row can become the render camera, as
        // the camera's own panel button does.
        if item.camera.is_some() {
            let is_render = self.editor.scene.render_camera() == Some(item.id);
            if ui
                .gated(
                    crate::unavailable::unless(!is_render, "This is already the render camera"),
                    Button::new("Set as Render Camera"),
                )
                .on_hover_text("Cycles renders and Camera view look through the render camera")
                .clicked()
            {
                self.run(Command::SetRenderCamera(item.id));
                ui.close();
            }
        }
        // PB-41: the selection (this row joins it if it wasn't in it) as an
        // item or assembly preset. UIC-03.
        ui.separator();
        ui.menu_button("Presets", |ui| {
            for (title, source) in [
                ("Save…", operandi_core::preset::PresetSource::Items),
                (
                    "Save Assembly…",
                    operandi_core::preset::PresetSource::Assembly,
                ),
            ] {
                if ui.button(title).clicked() {
                    if !self.editor.scene.selected_items().contains(&item.id) {
                        self.run(Command::SelectItem(Some(item.id)));
                    }
                    self.request_save_preset(source);
                    ui.close();
                }
            }
        });
    }
}

/// A rig piece's row icon (the rig, IK and skin studio icons).
fn rig_icon(kind: &str) -> icons::Icon {
    use icons::Icon;
    match kind {
        "ik.twoBone" => Icon::IkTwoBone,
        "ik.chain" => Icon::IkChain,
        "ik.body" => Icon::IkFullBody,
        "dynamicParent" => Icon::AddParent,
        "link" => Icon::ChannelLinks,
        k if k.starts_with("mod.") => Icon::Schematic,
        _ => Icon::Constrain,
    }
}

/// The Items row icon for an item that has no locator glyph: Backdrop's
/// polygon, otherwise the mesh cube. Environments never show here
/// (ENVIRONMENT-PLACEMENT §1); their icon is on the Shader Tree's rows.
fn item_type_icon(item: &operandi_core::Item) -> icons::Icon {
    if item.backdrop.is_some() {
        icons::Icon::Polygon
    } else {
        icons::Icon::Cube
    }
}

/// "After `id`" as a `MoveItems` position: before its next sibling under
/// `parent`, or last when it is the last one.
pub(crate) fn item_drop_after(
    scene: &operandi_core::Scene,
    id: u64,
    parent: Option<u64>,
) -> Option<u64> {
    let at = scene.item_index(id)?;
    scene.items[at + 1..]
        .iter()
        .find(|i| i.parent == parent && i.in_item_list())
        .map(|i| i.id)
}

/// The parked scene's rows, depth-first, folded parents' children left out.
fn parked_rows(s: &scenes::SceneState) -> Vec<ParkedRow> {
    let items = &s.editor.scene.items;
    let mut out = Vec::new();
    // ENVIRONMENT-PLACEMENT §1: no environment rows here either.
    let mut stack: Vec<_> = items
        .iter()
        .rev()
        .filter(|it| it.parent.is_none() && it.in_item_list())
        .map(|it| (it, 1))
        .collect();
    while let Some((item, depth)) = stack.pop() {
        let folded = s.collapsed_items.contains(&item.id);
        let children = items.iter().any(|c| c.parent == Some(item.id));
        out.push((
            item.id,
            item.parent,
            item.name.clone(),
            item.visible,
            depth,
            children,
            folded,
        ));
        if !folded {
            stack.extend(
                items
                    .iter()
                    .rev()
                    .filter(|c| c.parent == Some(item.id))
                    .map(|c| (c, depth + 1)),
            );
        }
    }
    out
}

/// The list's callbacks: row menus.
struct ItemsHooks<'a> {
    app: &'a mut Operandi,
    action: Option<scenes::SceneRowAction>,
}

impl TreeHooks<ItemKey> for ItemsHooks<'_> {
    fn has_row_menu(&self, key: ItemKey) -> bool {
        !matches!(key, ItemKey::NewScene)
    }
    fn row_menu(&mut self, ui: &mut Ui, key: ItemKey, _rename: &mut dyn FnMut()) {
        let here = self.app.scenes.active_tab().id;
        let index = |s: u64| self.app.scenes.tabs.iter().position(|t| t.id == s);
        match key {
            ItemKey::Scene(s) => {
                if let Some(i) = index(s) {
                    self.action = self.action.take().or(self.app.scene_row_menu_ui(ui, i));
                }
            }
            ItemKey::Item(s, id) if s == here => self.app.item_row_menu(ui, id),
            ItemKey::Item(s, id) => {
                let Some(i) = index(s) else {
                    return;
                };
                self.app.menu_owned_input = true;
                let Some(p) = self.app.scenes.tabs[i].parked.as_ref() else {
                    return;
                };
                let names: Vec<(u64, String)> = p
                    .editor
                    .scene
                    .items
                    .iter()
                    .filter(|it| it.in_item_list())
                    .map(|it| (it.id, it.name.clone()))
                    .collect();
                let parent = p.editor.scene.item(id).and_then(|it| it.parent);
                if let Some(entry) = scenes::other_item_menu(ui, id, parent, &names) {
                    self.action = Some(scenes::SceneRowAction::Item(i, id, entry));
                }
            }
            ItemKey::Rig(_, piece) => self.app.rig_piece_menu(ui, piece, _rename),
            ItemKey::Skin(_, item, op) => self.app.skin_row_menu(ui, item, op),
            ItemKey::Influence(_, item, joint) => self.app.influence_menu(ui, item, joint),
            ItemKey::NewScene => {}
        }
    }
}

/// `v` as one argument of a command line.
fn quoted(v: &str) -> String {
    if v.is_empty() || v.contains(char::is_whitespace) || v.contains('"') {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        v.to_owned()
    }
}

impl Operandi {
    /// A rig piece's menu (UIC-60: never empty): rename, enable, delete,
    /// select its item.
    fn rig_piece_menu(&mut self, ui: &mut Ui, piece: u64, rename: &mut dyn FnMut()) {
        self.menu_owned_input = true;
        let Some(p) = self.editor.scene.animation.rig.piece(piece).cloned() else {
            ui.label("This rig piece is gone");
            return;
        };
        let mut line: Option<String> = None;
        if ui.button("Rename…").clicked() {
            rename();
            ui.close();
        }
        let on = p.flag("enable");
        if ui.button(if on { "Disable" } else { "Enable" }).clicked() {
            line = Some(format!(
                "operandi.rig.set piece:{piece} channel:enable value:{}",
                if on { 0 } else { 1 }
            ));
        }
        let delete = if operandi_core::ik::is_ik(&p.kind) {
            format!("operandi.ik.remove piece:{piece}")
        } else {
            format!("operandi.rig.remove piece:{piece}")
        };
        if ui.button("Delete").on_hover_text(&delete).clicked() {
            line = Some(delete);
        }
        ui.separator();
        if ui.button("Select Item").clicked() {
            self.run(Command::SelectItem(Some(p.item)));
            ui.close();
        }
        if let Some(l) = line {
            self.execute(&l);
            ui.close();
        }
    }

    /// A skin row's menu (SK-36).
    fn skin_row_menu(&mut self, ui: &mut Ui, item: u64, op: u64) {
        self.menu_owned_input = true;
        let enabled = self
            .editor
            .scene
            .item(item)
            .and_then(|i| i.mesh_stack.as_ref())
            .and_then(|st| st.ops.iter().find(|o| o.id == op))
            .map(|o| o.enabled);
        let Some(enabled) = enabled else {
            ui.label("This skin is gone");
            return;
        };
        let setup = self.editor.scene.animation.setup;
        let mut line: Option<String> = None;
        if ui
            .button(if enabled { "Disable" } else { "Enable" })
            .clicked()
        {
            line = Some(format!(
                "deformer.enable deformer:op:{item}:{op} enable:{}",
                !enabled
            ));
        }
        if ui.button("Bake Weights").clicked() {
            line = Some(format!("operandi.skin.bakeWeights item:{item}"));
        }
        if ui
            .button("Duplicate and Freeze")
            .on_hover_text("A new mesh with the deformed shape; this one stays bound")
            .clicked()
        {
            line = Some(format!(
                "deformer.freeze deformer:op:{item}:{op} duplicate:true"
            ));
        }
        if ui
            .gated(
                crate::unavailable::unless(setup, operandi_core::skin_bind::NEEDS_SETUP),
                Button::new("Delete (Unbind)"),
            )
            .clicked()
        {
            line = Some(format!("operandi.skin.unbind items:{item}"));
        }
        if let Some(l) = line {
            self.execute(&l);
            ui.close();
        }
    }

    /// A joint influence's menu (SK-35).
    fn influence_menu(&mut self, ui: &mut Ui, item: u64, joint: u64) {
        self.menu_owned_input = true;
        let on = operandi_core::rig_list::rows(&self.editor.scene, item)
            .into_iter()
            .find_map(|r| match r {
                operandi_core::rig_list::RigRow::Skin { influences, .. } => influences
                    .into_iter()
                    .find(|i| i.joint == joint)
                    .map(|i| i.enabled),
                _ => None,
            })
            .unwrap_or(true);
        if ui.button(if on { "Disable" } else { "Enable" }).clicked() {
            self.execute(&format!(
                "operandi.skin.set item:{item} joint:{joint} enable:{}",
                !on
            ));
            ui.close();
        }
        if ui
            .gated(
                crate::unavailable::unless(
                    self.editor.scene.item(joint).is_some(),
                    "The joint is gone",
                ),
                Button::new("Select Joint"),
            )
            .clicked()
        {
            self.run(Command::SelectItem(Some(joint)));
            ui.close();
        }
    }
}

/// Whether `ancestor` is a parent, grandparent... of `item`.
fn is_ancestor(scene: &operandi_core::Scene, ancestor: u64, item: &operandi_core::Item) -> bool {
    let mut parent = item.parent;
    for _ in 0..scene.items.len() {
        match parent {
            Some(p) if p == ancestor => return true,
            Some(p) => parent = scene.item(p).and_then(|i| i.parent),
            None => return false,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_columns_follow_uic_150() {
        let s = spec();
        s.check_shape().unwrap();
        assert_eq!(s.columns[EYE].heading, Heading::Eye);
        assert_eq!(s.columns[NAME].kind, tree_list::ColumnKind::Name);
        assert!(
            s.columns
                .iter()
                .all(|c| c.kind != tree_list::ColumnKind::Value),
            "no value columns"
        );
    }

    /// Item List cost per frame with 100,000 items (scene-budget). Opt-in:
    /// `cargo test --release -p operandi-app item_list_frame_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn item_list_frame_cost_with_a_hundred_thousand_items() {
        let mut a = crate::Operandi::empty();
        let mut scene = operandi_core::Scene::default();
        for k in 0..100_000 {
            scene.new_mesh_named(Some(format!("Item {k}"))).unwrap();
        }
        a.editor.scene = scene;
        let t = std::time::Instant::now();
        let rows = a.item_rows();
        eprintln!(
            "item_rows: {} rows in {:.1} ms",
            rows.len(),
            t.elapsed().as_secs_f64() * 1e3
        );
        let ctx = egui::Context::default();
        for k in 0..std::env::var("FRAMES")
            .ok()
            .and_then(|f| f.parse().ok())
            .unwrap_or(4)
        {
            let t = std::time::Instant::now();
            crate::tests::draw(&mut a, &ctx, egui::vec2(1440., 900.), Vec::new());
            eprintln!("frame {k}: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
        }
    }

    #[test]
    fn plain_item_rows_use_the_cube_icon() {
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        let mesh = scene
            .items
            .iter()
            .find(|i| camera_light_ui::LocatorGlyph::of(i).is_none())
            .expect("a mesh item");
        assert_eq!(item_type_icon(mesh), icons::Icon::Cube);
    }
}

#[cfg(test)]
#[path = "panel_speed_tests.rs"]
mod panel_speed_tests;

#[cfg(test)]
#[path = "../rigging/items_rig_tests.rs"]
mod rig_tests;
