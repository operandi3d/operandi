//! Several open scenes, listed as root rows of the Items list.
//!
//! The scene being edited lives in the app's own fields (`editor`, `saved`,
//! `path`, `camera` …), so every tool and panel works on it unchanged. The
//! other open scenes are parked in `OpenScenes::tabs`; switching swaps the
//! per-scene fields between the app and the parked copy (`exchange`).
//!
//! Per scene: geometry, selection, undo/redo, file path, saved state, the
//! active tool and its unconfirmed panel fields (a scene is parked exactly
//! as it was, never dropped or committed), the views' cameras, the Items
//! list's collapsed rows, the recovery journal and the auto-save timer. Shared by all scenes: tool settings and presets,
//! preferences, layout, symmetry and falloff.
//!
//! From the MULTI-SCENE.md v0.1 behaviour spec: File > Current
//! lists the scenes; "Untitled", "Untitled (2)" names; New and Open add a
//! scene; Close Scene asks to save and closing the last scene leaves a new
//! blank one; Close All confirms first; undo and auto-save preferences are
//! app-wide (each scene keeps its own history under the shared limit).
//!
//! Provisional (spec items still open, see docs/SPECIFICATION_QUESTIONS.md
//! MS-Q*): the Items list shows every open scene and a click on one makes it
//! current, what is per scene, keeping a tool live in its scene,
//! Open filling an untouched new scene, switching to an already-open file,
//! one save prompt per unsaved scene on quit, per-scene backups.
use super::*;
use crate::unavailable::Gate;
use std::collections::BTreeSet;
use std::path::Path;

/// The open scenes in tab order.
pub(crate) struct OpenScenes {
    pub tabs: Vec<Tab>,
    /// Index of the scene being edited; its `parked` is `None`.
    pub active: usize,
    next_id: u64,
    /// Scenes whose changes the user chose to discard while quitting.
    pub quit_discarded: BTreeSet<u64>,
    /// Close All's "close all scenes?" confirmation is showing.
    pub confirm_close_all: bool,
    /// Close All is closing scenes one at a time.
    pub closing_all: bool,
    /// Delete's "delete the child items too?" question is showing (GL-Q4).
    pub confirm_delete_children: bool,
    /// Delete's "only one mesh item" question is showing (Jason,
    /// 2026-11-21), with the children choice already made (None: no
    /// selected item has children).
    pub confirm_last_mesh: Option<(
        crate::form::Dialog,
        Option<operandi_core::item_delete::ChildrenOnDelete>,
    )>,
    /// Where another scene's rows were drawn this frame: (row, scene id,
    /// item or None for the scene row), the drop targets between scenes.
    pub drop_rects: Vec<(Rect, u64, Option<u64>)>,
    /// A drop between scenes, carried out after the list is drawn.
    pub pending: Option<SceneRowAction>,
}

/// Dragging an item to another scene moves it; Option (Alt) copies it
/// (Jason, 2026-11-04). Set this to true to make a plain drag copy.
pub(crate) const DRAG_COPIES: bool = false;

pub(crate) struct Tab {
    pub id: u64,
    /// "Untitled (N)" number given when the scene was made (see
    /// `next_untitled`); shown while it has never been saved
    /// (1 shows as plain "Untitled").
    pub untitled: u32,
    /// Recovery journal slot: 1 is `recovery.opscene`, N is
    /// `recovery-N.opscene`.
    pub slot: u32,
    pub parked: Option<Box<SceneState>>,
}

impl Default for OpenScenes {
    fn default() -> Self {
        Self {
            tabs: vec![Tab {
                id: 0,
                untitled: 1,
                slot: 1,
                parked: None,
            }],
            active: 0,
            next_id: 1,
            quit_discarded: BTreeSet::new(),
            confirm_close_all: false,
            closing_all: false,
            confirm_delete_children: false,
            confirm_last_mesh: None,
            drop_rects: Vec::new(),
            pending: None,
        }
    }
}

impl OpenScenes {
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }
    /// A new tab with this untitled number; its journal slot is the
    /// smallest not used by another open scene.
    fn add(&mut self, at: usize, untitled: u32) -> usize {
        let free = |used: Vec<u32>| (1..).find(|n| !used.contains(n)).unwrap_or(1);
        let slot = free(self.tabs.iter().map(|t| t.slot).collect());
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.insert(
            at,
            Tab {
                id,
                untitled,
                slot,
                parked: Some(Box::default()),
            },
        );
        if at <= self.active {
            self.active += 1;
        }
        at
    }
}

/// One open scene's own state, while another scene is being edited. Only
/// the per-scene fields of `editor` are used (see `exchange`).
#[derive(Default)]
pub(crate) struct SceneState {
    pub(super) editor: Editor,
    saved: Scene,
    path: Option<PathBuf>,
    name: String,
    camera: Camera,
    cameras: BTreeMap<u32, Camera>,
    pub(super) collapsed_items: BTreeSet<u64>,
    pub(super) scene_collapsed: bool,
    journaled: Option<Scene>,
    autosave: undo_autosave::Autosave,
    fields: BTreeMap<String, String>,
    confirmed_fields: BTreeMap<String, String>,
    field_errors: BTreeMap<String, String>,
    bridge_ui: bridge_panel::BridgeUi,
    bevel_ui: bevel_panel::BevelUi,
    /// Image save queue and file times (painting.rs).
    #[cfg(feature = "painting")]
    images: crate::painting::DocImages,
}

impl SceneState {
    pub(super) fn dirty(&self) -> bool {
        super::needs_saving(&self.editor, &self.saved, super::same_document)
    }
}

/// A file name, or "Untitled (N)".
fn label(path: Option<&Path>, untitled: u32) -> String {
    match path.and_then(|p| p.file_name()) {
        Some(name) => name.to_string_lossy().into_owned(),
        None if untitled <= 1 => "Untitled".into(),
        None => format!("Untitled ({untitled})"),
    }
}

impl Operandi {
    /// Swap the per-scene state between the app and `s`.
    fn exchange(&mut self, s: &mut SceneState) {
        use std::mem::swap;
        // Every editor field is sorted into document or shared state in
        // core (document_state.rs, Astra P2-001).
        self.editor.swap_document_state(&mut s.editor);
        swap(&mut self.saved, &mut s.saved);
        swap(&mut self.path, &mut s.path);
        swap(&mut self.name, &mut s.name);
        swap(&mut self.camera, &mut s.camera);
        swap(&mut self.cameras, &mut s.cameras);
        swap(&mut self.collapsed_items, &mut s.collapsed_items);
        swap(&mut self.scene_collapsed, &mut s.scene_collapsed);
        swap(&mut self.journaled, &mut s.journaled);
        swap(&mut self.autosave, &mut s.autosave);
        swap(&mut self.fields, &mut s.fields);
        swap(&mut self.confirmed_fields, &mut s.confirmed_fields);
        swap(&mut self.field_errors, &mut s.field_errors);
        swap(&mut self.bridge_ui, &mut s.bridge_ui);
        swap(&mut self.bevel_ui, &mut s.bevel_ui);
        #[cfg(feature = "painting")]
        swap(&mut self.painting.doc, &mut s.images);
    }

    /// True while a tool, Pen draft or selection gesture holds the scene.
    pub(super) fn scene_busy(&self) -> bool {
        let e = &self.editor;
        e.session.live.is_some()
            || e.session.pen_draft.is_some()
            || e.session.solid_sketch.is_some()
            || e.session.bridge.is_some()
            || e.session.spike.is_some()
            || e.session.reduction.is_some()
            || e.session.ellipsoid.is_some()
            || e.session.shape.is_some()
            || e.session.cone.is_some()
            || e.selection_start.is_some()
    }

    /// MS-Q8 (measured, v0.3): one more than the highest number among the
    /// open Untitled scenes; lower gaps are never reused, and with none open
    /// the next is plain "Untitled".
    fn next_untitled(&self) -> u32 {
        (0..self.scenes.tabs.len())
            .filter(|&i| {
                let tab = &self.scenes.tabs[i];
                match &tab.parked {
                    Some(s) => s.path.is_none(),
                    None => self.path.is_none(),
                }
            })
            .map(|i| self.scenes.tabs[i].untitled)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// The tab label of the scene being edited.
    pub(super) fn scene_label(&self) -> String {
        label(self.path.as_deref(), self.scenes.active_tab().untitled)
    }
    fn tab_label(&self, i: usize) -> String {
        let tab = &self.scenes.tabs[i];
        match &tab.parked {
            Some(s) => label(s.path.as_deref(), tab.untitled),
            None => self.scene_label(),
        }
    }
    pub(super) fn tab_dirty(&self, i: usize) -> bool {
        match &self.scenes.tabs[i].parked {
            Some(s) => s.dirty(),
            None => self.dirty(),
        }
    }
    /// Where the scene being edited keeps its recovery journal.
    pub(super) fn journal_path(&self) -> PathBuf {
        slot_path(&self.environment, self.scenes.active_tab().slot)
    }

    /// A never-saved, unchanged scene with no history: Open fills it
    /// instead of adding a tab.
    fn untouched(&self) -> bool {
        self.path.is_none()
            && !self.dirty()
            && self.editor.history.undo.is_empty()
            && self.editor.history.redo.is_empty()
            && !self.scene_busy()
    }

    /// Before a file replaces the scene being edited: keep the current
    /// scene in its tab and edit a new, empty one, unless the current one
    /// is untouched.
    pub(super) fn make_room_for_document(&mut self) {
        if !self.untouched() {
            let at = self.scenes.active + 1;
            let untitled = self.next_untitled();
            self.scenes.add(at, untitled);
            self.switch_scene_now(at);
        }
    }

    /// File > New: a new tab with a new scene.
    pub(super) fn new_scene_tab(&mut self) {
        // CA-CON-003/015: the new scene takes Cone's settings from an
        // active Cone session, which itself stays with its scene.
        let cone = self.editor.session.cone.clone();
        let at = self.scenes.active + 1;
        let untitled = self.next_untitled();
        self.scenes.add(at, untitled);
        self.switch_scene_now(at);
        self.editor.session.cone = cone;
        self.initialize_new_document();
    }

    /// Switch to the open scene at `index`. Its tool, if any, stays live in
    /// the scene being left. Not mid-gesture.
    pub(super) fn switch_scene(&mut self, index: usize) -> bool {
        if index == self.scenes.active || index >= self.scenes.tabs.len() {
            return index == self.scenes.active;
        }
        if self.has_modal() || self.navigation.is_some() || self.editor.selection_start.is_some() {
            return false;
        }
        self.switch_scene_now(index);
        self.status = format!("Scene: {}", self.scene_label());
        true
    }

    /// The swap itself (no tool checks).
    pub(super) fn switch_scene_now(&mut self, index: usize) {
        self.flush_scene();
        self.cameras.insert(self.active_view, self.camera.clone());
        let mut incoming = self.scenes.tabs[index]
            .parked
            .take()
            .expect("an inactive tab holds its scene");
        self.exchange(&mut incoming);
        let old = self.scenes.active;
        self.scenes.tabs[old].parked = Some(incoming);
        self.scenes.active = index;
        self.after_scene_change();
    }

    /// Bring the leaving scene's recovery journal and auto-save up to date,
    /// since it can't change while parked.
    fn flush_scene(&mut self) {
        self.finish_journal();
        self.journal();
        if self.undo_autosave.autosave
            && self.dirty()
            && self.autosave.changed_since_backup(&self.editor.scene)
            && self.editor.backup_ready()
            && !self.app_stroke_held()
            && !self.autosave.busy()
        {
            self.start_backup();
        }
    }

    /// Housekeeping after a different scene became the edited one.
    fn after_scene_change(&mut self) {
        // Caches are keyed by item id, and ids repeat between scenes.
        self.surface_cache = Default::default();
        self.subdivision_cache = Default::default();
        self.resident_cache = Default::default();
        self.edge_cache = Default::default();
        self.tweak_hover_cache = Default::default();
        self.edge_slide_frame_memo = Default::default();
        self.vertex_frame_memo = Default::default();
        self.edge_frame_memo = Default::default();
        self.selection_normal_memo = Default::default();
        self.twist_bevel_frame_memo = Default::default();
        self.whole_mesh_basis_memo = Default::default();
        self.editor.revision = self.editor.revision.wrapping_add(1);
        // A new scene's views start where the current ones are.
        for view in self.views.viewports() {
            if !self.cameras.contains_key(&view) {
                self.cameras.insert(view, self.camera.clone());
            }
        }
        if let Some(camera) = self.cameras.get(&self.active_view) {
            self.camera = camera.clone();
        }
        self.field_focus = None;
        self.rename_item = None;
        *self.items_tree = Default::default();
        self.item_solo = None;
        self.item_fields.clear();
        self.item_field_owner = None;
        self.edge_click = None;
        self.edge_insert_label = None;
        self.vertex_weld = false;
        self.weld_drag = None;
        self.sync_source = None;
        self.pen_plane = None;
        self.i2_gesture_ticket = None;
        self.scene_rows.clear();
    }

    /// Close the scene being edited without asking (the prompt, if any,
    /// came first). The last scene is replaced by an empty one.
    pub(super) fn close_active_scene(&mut self) {
        if let Some(job) = self.journal_job.take() {
            let _ = job.join();
        }
        // A blocked session keeps the earlier session's journals on disk
        // (Recover Scene, Cancel); closing must not delete them (P7-001).
        if !self.journal_blocked {
            let _ = std::fs::remove_file(self.journal_path());
        }
        self.journaled = None;
        if self.scenes.tabs.len() == 1 {
            self.initialize_new_document();
            self.editor.revision = self.editor.revision.wrapping_add(1);
            self.scenes.tabs[0].untitled = 1;
            self.autosave = Default::default();
            self.status = "Scene closed".into();
            return;
        }
        let closing = self.scenes.active;
        let next = if closing + 1 < self.scenes.tabs.len() {
            closing + 1
        } else {
            closing - 1
        };
        let mut incoming = self.scenes.tabs[next]
            .parked
            .take()
            .expect("an inactive tab holds its scene");
        self.exchange(&mut incoming);
        drop(incoming);
        self.scenes.tabs.remove(closing);
        self.scenes.active = if next > closing { next - 1 } else { next };
        self.after_scene_change();
        self.status = format!("Scene closed; now {}", self.scene_label());
    }

    /// Quit, one scene at a time: after the edited scene was saved or
    /// discarded, ask about the next unsaved one; quit when none is left.
    pub(super) fn continue_quit(&mut self) {
        if self.dirty() {
            let id = self.scenes.active_tab().id;
            self.scenes.quit_discarded.insert(id);
        }
        let next = (0..self.scenes.tabs.len()).find(|&i| {
            i != self.scenes.active
                && self.tab_dirty(i)
                && !self.scenes.quit_discarded.contains(&self.scenes.tabs[i].id)
        });
        if let Some(i) = next {
            self.switch_scene_now(i);
            self.pending = Some(FileAction::Close);
            self.prompt_focused = false;
            return;
        }
        // Nothing left unsaved: no journal should offer recovery next time.
        if let Some(job) = self.journal_job.take() {
            let _ = job.join();
        }
        // Unless the journals on disk belong to an earlier session the
        // user chose to keep (Recover Scene, Cancel; Astra P7-001).
        if !self.journal_blocked {
            for tab in &self.scenes.tabs {
                let _ = std::fs::remove_file(slot_path(&self.environment, tab.slot));
            }
        }
        self.journaled = None;
        self.allow_close = true;
    }

    /// Delete the selected items (the Delete key in Item mode, Delete
    /// Item in menus and the Items list). With child items it asks first
    /// whether to delete them too (Jason, 2026-11-08, GL-Q4); then, if the
    /// scene's only mesh item would go, whether to delete it
    /// (`delete_items_checked`).
    pub(super) fn request_delete(&mut self) {
        let ids = self.editor.scene.selected_items();
        if self.editor.scene.any_has_children(&ids) {
            self.scenes.confirm_delete_children = true;
        } else {
            self.delete_items_checked(None);
        }
    }
    /// The question for `request_delete` in the shared dialog's Yes / No /
    /// Cancel style (Jason, 2026-11-08, GL-Q7; UIC-81 title = the
    /// command's name). Yes (Return) deletes the children too, No (Escape)
    /// moves them to the root in place, Cancel (Shift+Escape) changes
    /// nothing (UIC-96).
    pub(super) fn delete_children_confirmation(&mut self, ctx: &Context) {
        use operandi_core::item_delete::{self, ChildrenOnDelete};
        let ids = self.editor.scene.selected_items();
        let Some(message) = item_delete::children_question(&self.editor.scene, &ids) else {
            self.scenes.confirm_delete_children = false;
            return;
        };
        let children = match self.show_question(ctx, "Delete Item", &message) {
            form::Outcome::Ok => ChildrenOnDelete::Delete,
            form::Outcome::No => ChildrenOnDelete::MoveToRoot,
            form::Outcome::Extra(_) | form::Outcome::Cancel => {
                self.scenes.confirm_delete_children = false;
                return;
            }
            form::Outcome::Open => return,
        };
        self.scenes.confirm_delete_children = false;
        self.delete_items_checked(Some(children));
    }
    /// Delete the selected items with `children` (None: none of them has
    /// children), asking first when that would take the scene's only mesh
    /// item or all of its mesh items (Jason, 2026-11-21). The question's "In the Future" choice,
    /// once not Always Ask, answers it without showing it.
    pub(super) fn delete_items_checked(
        &mut self,
        children: Option<operandi_core::item_delete::ChildrenOnDelete>,
    ) {
        use crate::defaults_preferences::InTheFuture;
        let ids = self.editor.scene.selected_items();
        if !self
            .editor
            .scene
            .removes_last_mesh(&ids, children.unwrap_or_default())
        {
            self.delete_items_now(children);
            return;
        }
        match self.defaults_preferences.last_mesh_delete {
            InTheFuture::Ask => {
                let question = self.editor.scene.last_mesh_question();
                self.scenes.confirm_last_mesh = Some((last_mesh_dialog(question), children))
            }
            InTheFuture::AlwaysYes => self.delete_items_now(children),
            InTheFuture::AlwaysNo => self.delete_all_but_the_last_mesh(children),
        }
    }
    /// The "only one mesh item" / "every mesh item" question (Jason,
    /// 2026-11-21) in the shared dialog's Yes / No / Cancel style: Yes
    /// (Return) deletes them, No (Escape) keeps the meshes and deletes any
    /// other selected items,
    /// Cancel (Shift+Escape) changes nothing (UIC-96). Its In the Future
    /// row (a dropdown, UIC-88) makes Yes or No the answer from now on;
    /// Cancel never saves it.
    pub(super) fn last_mesh_confirmation(&mut self, ctx: &Context) {
        use crate::defaults_preferences::InTheFuture;
        let Some((mut dialog, children)) = self.scenes.confirm_last_mesh.take() else {
            return;
        };
        let outcome = self.show_form(ctx, &mut dialog);
        let future = InTheFuture::from_index(dialog.values.choice("future"));
        let remember = |a: &mut Self, answer: InTheFuture| {
            if future != InTheFuture::Ask {
                let before = a.defaults_preferences.clone();
                a.defaults_preferences.last_mesh_delete = answer;
                a.save_defaults(before);
            }
        };
        match outcome {
            form::Outcome::Open => self.scenes.confirm_last_mesh = Some((dialog, children)),
            form::Outcome::Ok => {
                remember(self, InTheFuture::AlwaysYes);
                self.delete_items_now(children);
            }
            form::Outcome::No => {
                remember(self, InTheFuture::AlwaysNo);
                self.delete_all_but_the_last_mesh(children);
            }
            form::Outcome::Cancel | form::Outcome::Extra(_) => {
                self.status = "Delete canceled; nothing was deleted".into();
            }
        }
    }
    /// The last-mesh question's No: the other selected items go, every
    /// mesh item stays.
    fn delete_all_but_the_last_mesh(
        &mut self,
        children: Option<operandi_core::item_delete::ChildrenOnDelete>,
    ) {
        let scene = &self.editor.scene;
        let rest: Vec<u64> = scene
            .selected_items()
            .into_iter()
            .filter(|&id| scene.item(id).is_some_and(|i| !i.is_mesh()))
            .collect();
        if rest.is_empty() || scene.removes_last_mesh(&rest, children.unwrap_or_default()) {
            self.status = "Kept the mesh items; nothing was deleted".into();
            return;
        }
        let scene = &mut self.editor.scene;
        scene.select_item(Some(rest[0]));
        scene.selection.also = rest[1..].to_vec();
        self.delete_items_now(children.or(Some(Default::default())));
        self.status = "Kept the mesh items; deleted the other selected items".into();
    }
    /// Delete the selected items now: one undo step.
    fn delete_items_now(&mut self, children: Option<operandi_core::item_delete::ChildrenOnDelete>) {
        match children {
            Some(children) => self.run(Command::DeleteItems(children)),
            // N10: every selected item goes, not just the current one.
            None if self.editor.scene.selected_items().len() > 1 => {
                self.run(Command::DeleteItems(Default::default()))
            }
            None => self.run(Command::DeleteItem),
        }
    }
    /// File > Close All: confirm first (MS-Q5).
    pub(super) fn request_close_all(&mut self) {
        if self.pending.is_none() && self.file_dialog.is_none() {
            self.scenes.confirm_close_all = true;
        }
    }
    pub(super) fn close_all_confirmation(&mut self, ctx: &Context) {
        // UIC-97: a confirmation in the shared dialog frame.
        match self.show_confirm(
            ctx,
            "Close All",
            "Close all open scenes? Each scene with unsaved changes asks to be saved first.",
            &[],
        ) {
            form::Outcome::Ok => {
                self.scenes.confirm_close_all = false;
                self.scenes.closing_all = true;
                self.close_all_step();
            }
            form::Outcome::Cancel => self.scenes.confirm_close_all = false,
            _ => {}
        }
    }
    /// Close scenes from the current one on; stop at an unsaved scene to
    /// ask (its Save/Discard continues here). Ends with one new scene.
    pub(super) fn close_all_step(&mut self) {
        loop {
            if self.dirty() || self.scene_busy() {
                self.pending = Some(FileAction::CloseScene);
                self.prompt_focused = false;
                return;
            }
            let last = self.scenes.tabs.len() == 1;
            self.close_active_scene();
            if last {
                self.scenes.closing_all = false;
                self.status = "All scenes closed".into();
                return;
            }
        }
    }

    /// Open scene `path` if it's already open: switch to it.
    pub(super) fn switch_to_open_file(&mut self, path: &Path) -> bool {
        let same = |p: &Path| {
            p == path
                || std::fs::canonicalize(p)
                    .ok()
                    .zip(std::fs::canonicalize(path).ok())
                    .is_some_and(|(a, b)| a == b)
        };
        if self.path.as_deref().is_some_and(same) {
            self.status = format!("{} is already open", self.scene_label());
            return true;
        }
        let found = self.scenes.tabs.iter().position(|t| {
            t.parked
                .as_ref()
                .and_then(|s| s.path.as_deref())
                .is_some_and(same)
        });
        if let Some(i) = found {
            self.switch_scene(i);
            return true;
        }
        false
    }

    /// The other scene tab that has file `path` open, and whether it has
    /// unsaved changes (the MCP bridge's `scene_load`).
    // Unused on Windows, where the bridge has no transport yet (mcp.rs).
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(super) fn parked_tab_with(&self, same: impl Fn(&Path) -> bool) -> Option<(usize, bool)> {
        self.scenes.tabs.iter().enumerate().find_map(|(i, t)| {
            let s = t.parked.as_ref()?;
            s.path.as_deref().is_some_and(&same).then(|| (i, s.dirty()))
        })
    }

    /// File > Current entries: one per open scene, the edited one checked.
    pub(super) fn scene_menu_entries(&self) -> Vec<(usize, String, bool)> {
        (0..self.scenes.tabs.len())
            .map(|i| {
                let dot = if self.tab_dirty(i) { " •" } else { "" };
                (
                    i,
                    format!("{}{dot}", self.tab_label(i)),
                    i == self.scenes.active,
                )
            })
            .collect()
    }

    /// After recovering journals: put each extra recovered scene in its own
    /// tab and journal it under that tab's slot.
    pub(super) fn recover_extra_scenes(&mut self, extra: Vec<Scene>) {
        for scene in extra {
            let at = self.scenes.tabs.len();
            let untitled = self.next_untitled();
            self.scenes.add(at, untitled);
            self.switch_scene_now(at);
            self.editor.scene = scene;
            self.saved = Scene::default();
            self.path = None;
            self.name = self
                .editor
                .scene
                .active()
                .map(|i| i.name.clone())
                .unwrap_or_default();
            self.journal();
        }
    }
}

/// Journal file for `slot`.
pub(super) fn slot_path(environment: &app_environment::AppEnvironment, slot: u32) -> PathBuf {
    let first = environment.paths().recovery();
    if slot <= 1 {
        first
    } else {
        first.with_file_name(format!("recovery-{slot}.opscene"))
    }
}

/// Every journal left in the data folder, first slot first.
pub(super) fn journal_files(environment: &app_environment::AppEnvironment) -> Vec<PathBuf> {
    let first = environment.paths().recovery();
    let mut found: Vec<(u32, PathBuf)> = Vec::new();
    if first.exists() {
        found.push((1, first.clone()));
    }
    if let Some(dir) = first.parent()
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(n) = name
                .strip_prefix("recovery-")
                .and_then(|r| r.strip_suffix(".opscene"))
                .and_then(|n| n.parse::<u32>().ok())
                && n > 1
            {
                found.push((n, entry.path()));
            }
        }
    }
    found.sort();
    found.into_iter().map(|(_, p)| p).collect()
}

/// What a scene row in the Items list asks for; applied after the list is
/// drawn, since it may change the current scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SceneRowAction {
    /// Make this scene current (provisional, MS-Q9: a click on its row).
    Switch(usize),
    /// Make this scene current and select one of its items.
    SelectItem(usize, u64),
    /// A dropped item row: copy (or move) `item` and its children from
    /// scene `from` into scene `to` under `parent` (scene ids, not indexes).
    Transfer {
        from: u64,
        item: u64,
        to: u64,
        parent: Option<u64>,
        moving: bool,
    },
    /// An entry of an item's right-click menu, for an item of another
    /// scene: make the scene current, select the item, then do it.
    Item(usize, u64, ItemMenu),
    /// The "(new scene)" row.
    New,
    Save(usize),
    SaveAs(usize),
    Export(usize, operandi_core::interchange::ExportFormat),
    Close(usize),
}

/// The item right-click entries (the same as the current scene's rows).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ItemMenu {
    ParentTo {
        parent: u64,
        keep_position: bool,
    },
    Unparent {
        keep_position: bool,
    },
    /// GL-04, on that one item.
    ParentToGroupLocator,
    Delete,
    Rename,
}

impl Operandi {
    /// A scene's name in the Items list: its file name without extension,
    /// or "Untitled (N)".
    pub(super) fn scene_row_name(&self, i: usize) -> String {
        let tab = &self.scenes.tabs[i];
        let path = match &tab.parked {
            Some(s) => s.path.as_deref(),
            None => self.path.as_deref(),
        };
        match path.and_then(|p| p.file_stem()) {
            Some(stem) => stem.to_string_lossy().into_owned(),
            None => label(None, tab.untitled),
        }
    }

    /// Right-click menu of a scene row (MS-Q9).
    pub(super) fn scene_row_menu_ui(&mut self, ui: &mut Ui, i: usize) -> Option<SceneRowAction> {
        let mut chosen = None;
        {
            self.menu_owned_input = true;
            if ui.button("Save").clicked() {
                chosen = Some(SceneRowAction::Save(i));
                ui.close();
            }
            if ui.button("Save As…").clicked() {
                chosen = Some(SceneRowAction::SaveAs(i));
                ui.close();
            }
            ui.menu_button("Export As", |ui| {
                for format in operandi_core::interchange::ExportFormat::ALL {
                    if ui.button(format!("{}…", format.label())).clicked() {
                        chosen = Some(SceneRowAction::Export(i, format));
                        ui.close();
                    }
                }
            });
            ui.separator();
            if ui.button("Close Scene").clicked() {
                chosen = Some(SceneRowAction::Close(i));
                ui.close();
            }
        }
        chosen
    }

    /// Carry out a scene row's request; one about another scene makes that
    /// scene current first.
    pub(super) fn scene_row_action(&mut self, action: SceneRowAction) {
        if let SceneRowAction::Transfer {
            from,
            item,
            to,
            parent,
            moving,
        } = action
        {
            self.transfer_items(from, item, to, parent, moving);
            return;
        }
        let target = match action {
            SceneRowAction::New => {
                self.request_file(FileAction::New);
                return;
            }
            SceneRowAction::Switch(i)
            | SceneRowAction::SelectItem(i, _)
            | SceneRowAction::Item(i, _, _)
            | SceneRowAction::Save(i)
            | SceneRowAction::SaveAs(i)
            | SceneRowAction::Export(i, _)
            | SceneRowAction::Close(i) => i,
            SceneRowAction::Transfer { .. } => return,
        };
        if !self.switch_scene(target) {
            return;
        }
        match action {
            SceneRowAction::Save(_) => self.save_current(),
            SceneRowAction::SaveAs(_) => self.request_file(FileAction::SaveAs),
            SceneRowAction::Export(_, format) => self.request_file(FileAction::Export(format)),
            SceneRowAction::Close(_) => self.request_file(FileAction::CloseScene),
            SceneRowAction::SelectItem(_, item) => self.run(Command::SelectItem(Some(item))),
            SceneRowAction::Item(_, item, entry) => {
                self.run(Command::SelectItem(Some(item)));
                match entry {
                    ItemMenu::ParentTo {
                        parent,
                        keep_position,
                    } => self.run(Command::SetParent {
                        item,
                        parent: Some(parent),
                        keep_position,
                    }),
                    ItemMenu::Unparent { keep_position } => self.run(Command::SetParent {
                        item,
                        parent: None,
                        keep_position,
                    }),
                    ItemMenu::ParentToGroupLocator => self.run(Command::ParentToGroupLocator),
                    ItemMenu::Delete => self.request_delete(),
                    ItemMenu::Rename => {
                        self.rename_item = Some(item);
                        if let Some(it) = self.editor.scene.item(item) {
                            self.name = it.name.clone();
                        }
                    }
                }
            }
            SceneRowAction::Switch(_) | SceneRowAction::New | SceneRowAction::Transfer { .. } => {}
        }
    }
}

/// The item right-click menu for an item of another scene.
pub(super) fn other_item_menu(
    ui: &mut Ui,
    item: u64,
    parent: Option<u64>,
    names: &[(u64, String)],
) -> Option<ItemMenu> {
    let mut chosen = None;
    let others: Vec<&(u64, String)> = names.iter().filter(|(id, _)| *id != item).collect();
    for (title, keep_position) in [("Parent To", false), ("Parent To, Keep Position", true)] {
        ui.gated_ui(
            crate::unavailable::unless(
                !others.is_empty(),
                "The scene has no other item to parent to",
            ),
            |ui| {
                ui.menu_button(title, |ui| {
                    for (id, name) in &others {
                        if ui.button(name).clicked() {
                            chosen = Some(ItemMenu::ParentTo {
                                parent: *id,
                                keep_position,
                            });
                            ui.close();
                        }
                    }
                });
            },
        );
    }
    for (title, keep_position) in [("Unparent", false), ("Unparent, Keep Position", true)] {
        if ui
            .gated(
                crate::unavailable::unless(parent.is_some(), "The item has no parent"),
                Button::new(title),
            )
            .clicked()
        {
            chosen = Some(ItemMenu::Unparent { keep_position });
            ui.close();
        }
    }
    if ui
        .button(operandi_core::locator::PARENT_TO_GROUP_LOCATOR)
        .clicked()
    {
        chosen = Some(ItemMenu::ParentToGroupLocator);
        ui.close();
    }
    ui.separator();
    if ui.button("Delete Item").clicked() {
        chosen = Some(ItemMenu::Delete);
        ui.close();
    }
    if ui.button("Rename…").clicked() {
        chosen = Some(ItemMenu::Rename);
        ui.close();
    }
    chosen
}

impl Operandi {
    fn scene_of(&self, id: u64) -> Option<&Scene> {
        let index = self.scenes.tabs.iter().position(|t| t.id == id)?;
        match &self.scenes.tabs[index].parked {
            Some(s) => Some(&s.editor.scene),
            None => Some(&self.editor.scene),
        }
    }

    /// Copy (or move) `item` and its descendants from scene `from` into
    /// scene `to`, under `parent` there, keeping their world position. The
    /// receiving scene becomes current and gets one undo step; a move also
    /// leaves a "Move Items Out" step in the scene it left. Provisional
    /// (MS-Q10): copy versus move, where the result lands, which scene
    /// ends up current.
    pub(super) fn transfer_items(
        &mut self,
        from: u64,
        item: u64,
        to: u64,
        parent: Option<u64>,
        moving: bool,
    ) {
        let index = |a: &Self, id: u64| a.scenes.tabs.iter().position(|t| t.id == id);
        let branch = match self.scene_of(from).map(|s| s.branch(item)) {
            Some(Ok(branch)) => branch,
            Some(Err(e)) => {
                self.status = e;
                return;
            }
            None => return,
        };
        let images = self
            .scene_of(from)
            .map(|s| s.images.clone())
            .unwrap_or_default();
        let (Some(source), Some(target)) = (index(self, from), index(self, to)) else {
            return;
        };
        if moving {
            if !self.switch_scene(source) {
                return;
            }
            if self.scene_busy() {
                self.status = "Drop or Cancel the active tool before moving its items".into();
                return;
            }
        }
        if !self.switch_scene(target) {
            return;
        }
        let count = branch.len();
        // Recorded or refused, by mark: at Max Undo Level the length stays
        // put, and a Move would leave its copy behind (Astra P4-004).
        let mark = self.editor.history_mark();
        self.run(Command::InsertItems {
            items: branch,
            parent,
            images,
        });
        if !self.editor.recorded_since(mark) {
            return; // refused; the status line says why
        }
        let mut status = format!(
            "{} {count} item{} into {}",
            if moving { "Moved" } else { "Copied" },
            if count == 1 { "" } else { "s" },
            self.scene_label()
        );
        if moving && self.switch_scene(source) {
            self.run(Command::RemoveBranch(item));
            self.switch_scene(target);
            status.push_str("; undo in each scene separately");
        }
        self.status = status;
    }
}

/// The "only one mesh item" / "every mesh item" question: `question`
/// (`Scene::last_mesh_question`), then In the Future (UIC-86 label, UIC-88
/// dropdown), as a Yes / No / Cancel question. Its title is the command's
/// name (UIC-81).
pub(crate) fn last_mesh_dialog(question: &str) -> form::Dialog {
    use crate::defaults_preferences::InTheFuture;
    form::Dialog::with_defaults(
        "Delete Item",
        vec![
            form::Row::message(question),
            form::Row::choice(
                "future",
                "In the Future",
                &InTheFuture::LABELS,
                InTheFuture::Ask.index(),
            )
            .fresh()
            .tip("Ask each time, or give the same answer from now on without asking"),
        ],
    )
    .asking()
}

#[cfg(test)]
#[test]
fn hierarchy_delete_status_describes_only_the_committed_reparenting() {
    use operandi_core::item_delete::ChildrenOnDelete;

    let mut a = crate::tests::box_app();
    a.run(Command::Rename("Parent".into()));
    for name in ["Rotated", "Ordinary"] {
        a.run(Command::NewMesh);
        a.run(Command::Rename(name.into()));
    }
    let parent = a.editor.scene.items[0].id;
    let rotated = a.editor.scene.items[1].id;
    let ordinary = a.editor.scene.items[2].id;
    a.editor.scene.items[0].transform.scale = glam::DVec3::new(3., 1., 1.);
    a.editor.scene.items[1].transform.rotation = glam::DVec3::new(0., 0., 0.7);
    a.editor.scene.items[2].transform.translation = glam::DVec3::X;
    for child in [rotated, ordinary] {
        a.run(Command::SetParent {
            item: child,
            parent: Some(parent),
            keep_position: false,
        });
    }
    a.editor.scene.select_item(Some(parent));
    a.editor.scene.selection.mode = Mode::Item;
    let before = a.editor.scene.clone();
    let rotated_local = before.item(rotated).unwrap().transform.clone();
    let ordinary_world = before.world_matrix(ordinary).unwrap();
    a.activate(Tool::Move);
    assert!(a.editor_tool_active(), "{}", a.status);
    // UIC-307: Move settles; the delete runs with the tool on.
    a.delete_items_checked(Some(ChildrenOnDelete::MoveToRoot));
    assert!(a.status.starts_with("Rotated kept its own transform"));
    assert!(!a.status.contains("Ordinary"));
    assert!(a.editor.scene.item(parent).is_none());
    assert_eq!(
        a.editor.scene.item(rotated).unwrap().transform,
        rotated_local
    );
    for child in [rotated, ordinary] {
        assert_eq!(a.editor.scene.item(child).unwrap().parent, None);
    }
    assert_eq!(
        a.editor.scene.world_matrix(ordinary).unwrap(),
        ordinary_world
    );
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}
