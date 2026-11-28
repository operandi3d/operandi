//! Layouts, stage 1 (specifications/layouts-2026-11-16).
//!
//! A layout is a named arrangement of the whole main window, kept as data
//! (LAY-70/71, UIC-190/191): one [`WindowLayout`] file per layout holds the
//! dock tree, each slot's panel kind by registry name with that panel's
//! settings, fractions for every split, the layout's name, order, star and
//! the command line it runs when it opens. Shipped layouts are the files in
//! `assets/layouts`, built into the app (LAY-75); adding one is adding a
//! file. A user's changed copy is a file of the same format in the data
//! folder's `Layouts` folder, written only once it differs from the shipped
//! one (LAY-40/41).
//!
//! - LAY-30 / UIC-198: every way of switching (Window > Layouts, Layout >
//!   Layouts, scripts) runs `layout.restore <name>`.
//! - LAY-33: switching away keeps the layout's live state (views, cameras,
//!   view settings, toolbox tab, panel sizes, open tabs); switching back puts
//!   it back exactly. The open command runs when a layout opens from its
//!   file (first time in a session, after a restart, after a reset), not when
//!   switching back (provisional, LAY-Q2).
//! - LAY-02 / UIC-195: switching never touches the scene, undo or selection.
//! - LAY-22: the app opens on the layout open at quit; a fresh install on the
//!   first shipped layout by order (Model).
//! - LAY-49 / UIC-209: a slot naming a panel kind this build lacks shows an
//!   empty panel with its name; a file of another version is ignored and the
//!   shipped default used.
//! - Migration: the first run reads the old view-layout.json,
//!   view-displays.json and layout.json once, through their old loaders, as
//!   the Model layout, so Model looks exactly as before.
//! - Stock layouts retain their three-column presentation (UIC-196). A
//!   cross-panel move materializes those columns around the existing view
//!   tiles, then saves the resulting dock tree (UIC-191).
//!
//! Stage 2 (user layouts, Revert, the full switcher bar, pieces and the
//! content arrow) is in layouts_edit.rs; the state file now also keeps the
//! bar's order, stars, starred-only, collapsed state, the MRU list and each
//! layout's locks (LAY-60).
pub(super) mod dock;

use super::*;
use crate::panel_dot::LayoutLocks;
use crate::panel_registry::{PanelGroup, PanelKind};
use operandi_core::item_list::ListFilter;
use operandi_layout::{
    DockPanel, EMPTY, Frame, Pane, Slot, ToolboxState, UV_VIEW, VIEW_3D, WindowLayout,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use toolbox::{Category, ToolSet};

/// The built-in layout files (LAY-75): every `*.json` in `assets/layouts`,
/// listed by build.rs.
pub(super) const SHIPPED: &[(&str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/shipped_layouts.rs"));

/// The shipped layouts this build offers: painting builds add the Paint
/// layout (PNT-210).
pub(super) fn shipped() -> Vec<(&'static str, &'static str)> {
    #[allow(unused_mut)]
    let mut all = SHIPPED.to_vec();
    // The animation switch (anim_switch.rs): no Animate layout while off.
    if !crate::anim_switch::enabled() {
        all.retain(|(file, _)| *file != "animate.json");
    }
    #[cfg(feature = "painting")]
    if paint_layout_shipped() {
        all.push((
            "paint.json",
            include_str!("../assets/layouts-painting/paint.json"),
        ));
    }
    all
}

// Tests that list the shipped layouts keep their list; painting's own
// tests turn the Paint layout on for their thread (as `hide_menus` does
// for menus).
#[cfg(all(test, feature = "painting"))]
thread_local! {
    static PAINT_LAYOUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(all(test, feature = "painting"))]
pub(super) fn ship_paint_layout_in_tests() {
    PAINT_LAYOUT.with(|c| c.set(true));
}
#[cfg(all(test, feature = "painting"))]
fn paint_layout_shipped() -> bool {
    PAINT_LAYOUT.with(std::cell::Cell::get)
}
#[cfg(all(not(test), feature = "painting"))]
fn paint_layout_shipped() -> bool {
    true
}

pub(super) const TOOLBOX: &str = "Toolbox";
/// The source frame for the standalone legacy layout.json and view-layout.json.
pub(super) const LEGACY_MODEL: &str = include_str!("../assets/layouts-legacy/model.json");

type ToolboxSelection = (ToolSet, Category, bool);

/// UIC-191/195: a slot keeps its free-view camera. Camera and light item
/// bindings refer to the current scene and are handled by that session.
#[derive(Deserialize, PartialEq, Serialize)]
struct ViewCamera {
    view: camera::View,
    target: glam::DVec3,
    distance: f64,
    yaw: f64,
    pitch: f64,
    roll: f64,
    lens: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    near_depth: Option<f64>,
    /// The lens is the view's own field of view (Alt+Shift+wheel). Missing
    /// in layouts from builds whose Maximise scaled the lens instead.
    #[serde(default)]
    own_lens: bool,
}

impl From<&Camera> for ViewCamera {
    fn from(camera: &Camera) -> Self {
        Self {
            view: camera.view,
            target: camera.target,
            distance: camera.distance,
            yaw: camera.yaw,
            pitch: camera.pitch,
            roll: camera.roll,
            lens: camera.lens,
            near_depth: Some(camera.near_surface_depth()).filter(|d| d.is_finite()),
            own_lens: true,
        }
    }
}

pub(super) fn camera_from_slot(slot: &Slot) -> Camera {
    let Ok(saved) = ViewCamera::deserialize(&slot.settings["camera"]) else {
        return Camera::default();
    };
    if saved.distance <= 0. || saved.lens <= 0. {
        return Camera::default();
    }
    // Older builds let Maximise scale a view's lens (a much wider field of
    // view in Perspective) and saved that; such a layout opens at the
    // Flatness of Perspective lens, the view moved so the orbit centre
    // keeps its size on screen.
    let (lens, distance) = if saved.own_lens {
        (saved.lens, saved.distance)
    } else {
        let lens = camera::default_lens();
        (lens, saved.distance * lens / saved.lens)
    };
    Camera {
        view: saved.view,
        target: saved.target,
        distance,
        yaw: saved.yaw,
        pitch: saved.pitch,
        roll: saved.roll,
        lens,
        near_depth: saved.near_depth.unwrap_or(f64::INFINITY),
        // The saved depth is the one at the saved distance.
        near_distance: if saved.near_depth.is_some() {
            distance
        } else {
            f64::INFINITY
        },
        ..Camera::default()
    }
}

#[derive(Deserialize, Serialize)]
struct UvNavigation {
    center: glam::DVec2,
    zoom: f64,
}

pub(super) fn uv_view_from_slot(slot: &Slot) -> uv_view::UvView {
    let mut view = uv_view::UvView::default();
    if let Ok(saved) = UvNavigation::deserialize(&slot.settings["camera"])
        && saved.zoom > 0.
    {
        view.center = saved.center;
        view.zoom = saved.zoom;
    }
    view
}
const MODEL_TOOLS: ToolboxSelection = (ToolSet::Model, Category::Create, false);

/// The layouts state file's version (current layout, migration done).
const STATE_VERSION: u32 = 1;

/// One layout the app offers.
#[derive(Clone, Debug)]
pub(super) struct Entry {
    /// The built-in file, for shipped layouts.
    pub shipped: Option<WindowLayout>,
    /// The user's copy (a changed shipped layout, or their own).
    pub user: Option<WindowLayout>,
}

impl Entry {
    /// What opens: the user's copy, else the shipped file.
    pub fn doc(&self) -> &WindowLayout {
        self.user
            .as_ref()
            .or(self.shipped.as_ref())
            .expect("an entry has a file")
    }
    pub fn name(&self) -> &str {
        &self.doc().name
    }
}

/// A layout's live state while another one shows (LAY-33).
#[derive(Clone)]
pub(super) struct Live {
    dock_tree: Frame,
    mixed_dock: bool,
    tool_tabs_horizontal: Option<bool>,
    views: operandi_layout::Layout,
    cameras: BTreeMap<u32, Camera>,
    displays: BTreeMap<u32, view_display::ViewDisplay>,
    uv_views: BTreeMap<u32, uv_view::UvView>,
    active_view: u32,
    category: toolbox::Category,
    tool_set: ToolSet,
    toolbox_default: ToolboxSelection,
    uv_tab: bool,
    side: (f32, f32),
    tool_split: f32,
    scene_fraction: f32,
    upper: PanelGroup,
    lower: PanelGroup,
    filter: ListFilter,
    /// The pieces its slots show, and its view area's group piece (LAY-72).
    piece_of: BTreeMap<u32, String>,
    group_piece: Option<String>,
    /// Where this layout docks the mode bar.
    mode_bar_bottom: bool,
}

/// Layout switching and dock state.
#[derive(Clone)]
pub(super) struct Layouts {
    /// Every layout, by switcher order then name.
    pub list: Vec<Entry>,
    /// The layout showing.
    pub current: String,
    /// Switched-away layouts, by name.
    stash: BTreeMap<String, Live>,
    /// The compatibility frame for layouts using the stock column chrome.
    dock_tree: Frame,
    /// Rearranged layouts use the main tree in `views` for every dock leaf.
    pub(super) mixed_dock: bool,
    /// None projects the original strip beside the toolbox (D-07, UIC-196).
    /// A detached strip has its own dock tile and remembers its orientation.
    pub(super) tool_tabs_horizontal: Option<bool>,
    /// The mode bar docks at the bottom of the window (above the command
    /// line) rather than the top (Jason, 2026-11-19).
    pub(crate) mode_bar_bottom: bool,
    /// The dock tree's split sizes at the last primary press (views.rs).
    pub(crate) shares_at_press: Option<crate::views::LinearShares>,
    /// The mode bar's grip last frame (tests) and a drag Esc cancelled.
    #[cfg(test)]
    pub(crate) mode_bar_rect: Option<Rect>,
    pub(crate) mode_bar_drag: Option<egui::PointerButton>,
    /// UIC-15: a layout reset is waiting for confirmation.
    pub confirm_reset: Option<String>,
    /// The reset question's "Also clear its keys" box (LAY-Q56).
    pub confirm_reset_keys: bool,
    /// The toolbox shows its UV tab (opened by `operandi.toolbox.tab uv`).
    pub uv_tab: bool,
    pub tool_set: ToolSet,
    toolbox_default: ToolboxSelection,
    /// The side columns' widths as fractions of the window width.
    pub side: (f32, f32),
    /// `side` waits to be applied to the panels next frame.
    side_pending: bool,
    /// The first run reads the old layout files (finish_layout_migration).
    migrating: bool,
    /// The current layout's last written text, and the state file's.
    saved: String,
    state_saved: String,
    #[cfg(test)]
    pub tool_set_rects: Vec<(toolbox::ToolSet, Rect)>,
    /// The in-window menu bar's row last frame (tests).
    #[cfg(test)]
    pub(crate) menu_bar_rect: Option<Rect>,
    /// The message line's text rect last frame, when it sat under Tool
    /// Properties (tests).
    pub message_line_rect: Option<Rect>,
    /// This frame's answer to `message_line_in_tool_properties`.
    pub(crate) message_line_here: bool,
    /// The logo's rect last frame (tests): the top bar's collapse control;
    /// `None` while the bar is collapsed.
    pub logo_rect: Option<Rect>,
    /// The collapsed top bar's line, last frame (tests); `None` while the
    /// bar is open.
    pub strip_rect: Option<Rect>,
    // ---- stage 2 (layouts_edit.rs) ----
    /// Starred layouts (UIC-14), by name.
    pub stars: BTreeSet<String>,
    /// The Layouts pie filters to starred layouts and the current layout.
    pub starred_only: bool,
    /// The top bar (logo, menus, file state, tool sets) is collapsed to a
    /// thin line (UIC-16; the logo is the control, Jason 2026-11-18).
    pub top_collapsed: bool,
    /// The docked side bars are collapsed to a line at their window edge
    /// (Jason 2026-11-18: a double-click on the resize line). Their width
    /// stays in the panel state, so they reopen at it.
    pub left_collapsed: bool,
    pub right_collapsed: bool,
    /// A side bar reopened by a drag: the width to give it next frame.
    pub side_reopen: Option<(crate::collapse_line::Bar, f32)>,
    /// Layouts by when they were last shown, newest first (UIC-18).
    pub mru: Vec<String>,
    /// Each layout's Lock Thumb / Divider Dragging (LAY-48), by name.
    pub locks: BTreeMap<String, LayoutLocks>,
    /// The locks a layout without its own starts with: the old app-wide
    /// View > Layout locks (layout-locks.json).
    pub default_locks: LayoutLocks,
    /// The open New / Save As / Rename / Delete / Save Viewport dialog.
    pub dialog: Option<crate::layouts_edit::Dialog>,
    /// The quick layout switcher, while it is open (UIC-18).
    pub switcher: Option<crate::layouts_edit::Switcher>,
    /// The saved pieces (LAY-72): shipped ones first, then the user's.
    pub pieces: Vec<crate::layouts_edit::Piece>,
    /// The piece each panel of the window shows, by view or panel key.
    pub piece_of: BTreeMap<u32, String>,
    /// The view area shows this group piece.
    pub group_piece: Option<String>,
    /// Command lines a panel header asked for while the views drew; they
    /// run once the views are back (the content arrow, LAY-73).
    pub requests: Vec<String>,
}

impl Default for Layouts {
    fn default() -> Self {
        Self::read(&shipped(), None)
    }
}

#[derive(Default, Serialize, Deserialize)]
struct StateFile {
    version: u32,
    #[serde(default)]
    current: Option<String>,
    // Stage 2 (LAY-60). A stage 1 file has none of these and reads with
    // the defaults: the files' order and stars, nothing collapsed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    order: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stars: Option<Vec<String>>,
    #[serde(default)]
    starred_only: bool,
    // The old layout-tabs-only `collapsed` key (its "‹" button is gone,
    // Jason 2026-11-18) is no longer read, so a file that has it shows the
    // tabs.
    #[serde(default, skip_serializing_if = "is_false")]
    top_bar_collapsed: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    left_collapsed: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    right_collapsed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    mru: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    locks: BTreeMap<String, LayoutLocks>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn read_layout(text: &str) -> Result<WindowLayout> {
    let mut doc = WindowLayout::from_json(text)?;
    dock::migrate(&mut doc.frame);
    Ok(doc)
}

impl Layouts {
    /// Is `bar` collapsed to its line?
    pub fn is_collapsed(&self, bar: crate::collapse_line::Bar) -> bool {
        use crate::collapse_line::Bar;
        match bar {
            Bar::Top => self.top_collapsed,
            Bar::Left => self.left_collapsed,
            Bar::Right => self.right_collapsed,
        }
    }
    fn collapsed_mut(&mut self, bar: crate::collapse_line::Bar) -> &mut bool {
        use crate::collapse_line::Bar;
        match bar {
            Bar::Top => &mut self.top_collapsed,
            Bar::Left => &mut self.left_collapsed,
            Bar::Right => &mut self.right_collapsed,
        }
    }
    pub fn set_collapsed(&mut self, bar: crate::collapse_line::Bar, on: bool) {
        *self.collapsed_mut(bar) = on;
    }
}

/// A layout file's name for a layout name.
pub(super) fn file_name(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{}.json", stem.trim())
}

impl Layouts {
    /// The shipped files (`(file name, text)`), and the user's copies in
    /// `folder`. A file that doesn't read is left out (LAY-49).
    pub fn read(shipped: &[(&str, &str)], folder: Option<&Path>) -> Self {
        let mut list: Vec<Entry> = Vec::new();
        for (_, text) in shipped {
            if let Ok(doc) = read_layout(text)
                && !list
                    .iter()
                    .any(|e| e.name().eq_ignore_ascii_case(&doc.name))
            {
                list.push(Entry {
                    shipped: Some(doc),
                    user: None,
                });
            }
        }
        if let Some(dir) = folder
            && let Ok(entries) = std::fs::read_dir(dir)
        {
            let mut paths: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect();
            paths.sort();
            for path in paths {
                let Some(doc) = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|t| read_layout(&t).ok())
                else {
                    continue;
                };
                match list
                    .iter_mut()
                    .find(|e| e.name().eq_ignore_ascii_case(&doc.name))
                {
                    Some(entry) => entry.user = Some(doc),
                    None => list.push(Entry {
                        shipped: None,
                        user: Some(doc),
                    }),
                }
            }
        }
        list.sort_by(|a, b| {
            (a.doc().order, a.name().to_lowercase()).cmp(&(b.doc().order, b.name().to_lowercase()))
        });
        let current = list
            .first()
            .map(|e| e.name().to_owned())
            .unwrap_or_default();
        let stars = list
            .iter()
            .filter(|e| e.doc().starred)
            .map(|e| e.name().to_owned())
            .collect();
        Self {
            list,
            current,
            stash: BTreeMap::new(),
            // The renderer starts in the Model arrangement before any file
            // is installed, including during the older layout.json migration.
            dock_tree: read_layout(LEGACY_MODEL)
                .expect("the built-in Model layout is valid")
                .frame,
            mixed_dock: false,
            tool_tabs_horizontal: None,
            mode_bar_bottom: false,
            shares_at_press: None,
            #[cfg(test)]
            mode_bar_rect: None,
            mode_bar_drag: None,
            confirm_reset: None,
            confirm_reset_keys: false,
            uv_tab: false,
            tool_set: ToolSet::Model,
            toolbox_default: MODEL_TOOLS,
            side: (
                layout::LEFT_DEFAULT / window_state::DEFAULT_SIZE[0],
                layout::RIGHT_DEFAULT / window_state::DEFAULT_SIZE[0],
            ),
            side_pending: false,
            migrating: false,
            saved: String::new(),
            state_saved: String::new(),
            #[cfg(test)]
            tool_set_rects: Vec::new(),
            #[cfg(test)]
            menu_bar_rect: None,
            message_line_rect: None,
            message_line_here: false,
            logo_rect: None,
            strip_rect: None,
            stars,
            starred_only: false,
            top_collapsed: false,
            left_collapsed: false,
            right_collapsed: false,
            side_reopen: None,
            mru: Vec::new(),
            locks: BTreeMap::new(),
            default_locks: LayoutLocks::default(),
            dialog: None,
            switcher: None,
            pieces: Vec::new(),
            piece_of: BTreeMap::new(),
            group_piece: None,
            requests: Vec::new(),
        }
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.list
            .iter()
            .find(|e| e.name().eq_ignore_ascii_case(name.trim()))
    }

    pub(super) fn find_mut(&mut self, name: &str) -> Option<&mut Entry> {
        self.list
            .iter_mut()
            .find(|e| e.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Forget a switched-away layout's kept state.
    pub(super) fn stash_remove(&mut self, name: &str) {
        self.stash.remove(name);
    }

    pub(super) fn stash_rename(&mut self, old: &str, new: &str) {
        if let Some(live) = self.stash.remove(old) {
            self.stash.insert(new.to_owned(), live);
        }
    }

    /// Every layout's name, in switcher order.
    pub fn names(&self) -> Vec<String> {
        self.list.iter().map(|e| e.name().to_owned()).collect()
    }
}

fn tabs_of(frame: &Frame) -> (&[Slot], usize) {
    match frame {
        Frame::Tabs { tabs, active, .. } => (tabs, *active),
        Frame::Panel(slot) => (std::slice::from_ref(slot), 0),
        _ => unreachable!("Columns::read checked the group shape"),
    }
}

fn share(fraction: f32) -> f32 {
    if fraction.is_finite() && fraction > 0. {
        fraction
    } else {
        0.
    }
}

fn split_share(first: f32, second: f32, default: f32) -> f32 {
    let (first, second) = (share(first), share(second));
    let total = first + second;
    if total > 0. { first / total } else { default }
}

/// Take `keys`' entries out of a per-view map.
fn take_keys<T>(map: &mut BTreeMap<u32, T>, keys: &[u32]) -> BTreeMap<u32, T> {
    keys.iter()
        .filter_map(|k| map.remove(k).map(|v| (*k, v)))
        .collect()
}

/// A panel kind by its registry name (UIC-157). Older layout files call the
/// Mesh Stack panel "Mesh Ops", its name before 2026-11-19; they still open.
pub(super) fn kind_named(name: &str) -> Option<PanelKind> {
    if name.eq_ignore_ascii_case("Mesh Ops") {
        return Some(PanelKind::MeshStack);
    }
    panel_registry::REGISTRY
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
        .map(|t| t.kind)
}

/// A tab group from a file's slots: unknown kinds become named empty tabs
/// (LAY-49). The saved tree has already been checked for duplicates.
fn group_from(id: &'static str, (slots, active): (&[Slot], usize)) -> PanelGroup {
    let mut group = PanelGroup::new(id, &[]);
    let mut front = None;
    for (i, slot) in slots.iter().enumerate() {
        let kind = match kind_named(&slot.kind) {
            Some(k) => k,
            None => {
                group.missing.push(slot.kind.clone());
                PanelKind::Missing((group.missing.len() - 1) as u8)
            }
        };
        if i == active {
            front = Some(group.tabs.len());
        }
        group.tabs.push(kind);
    }
    group.active = front.unwrap_or(0);
    group
}

pub(super) fn registry_group(slot: &Slot) -> PanelGroup {
    group_from("detached-tab", (std::slice::from_ref(slot), 0))
}

impl Operandi {
    /// Where the user's layout files go (LAY-75).
    pub(super) fn layouts_folder(&self) -> PathBuf {
        self.environment.paths().data_root().join("Layouts")
    }
    /// The user's file for the layout called `name`.
    pub(super) fn layout_file(&self, name: &str) -> Option<PathBuf> {
        Some(self.layouts_folder().join(file_name(name)))
    }
    fn layouts_state_path(&self) -> PathBuf {
        self.environment.paths().data_root().join("layouts.json")
    }

    /// At launch, before the view properties load: read the layouts, then
    /// open the layout that was open at quit (LAY-22). The first run turns
    /// the old layout files into the Model layout.
    pub(super) fn start_layouts(&mut self) {
        let folder = self.layouts_folder();
        self.layouts = Layouts::read(&shipped(), Some(&folder));
        // The toolbox forms' user copies sit in the same Layouts folder
        // (LAY-87).
        self.forms = crate::toolbox_form::shipped_store(Some(&folder));
        for problem in &self.forms.problems {
            log_ring::log_line!("Forms: {problem}");
        }
        self.layouts.default_locks = self.dots.locks;
        self.load_pieces();
        let state = std::fs::read_to_string(self.layouts_state_path())
            .ok()
            .and_then(|t| serde_json::from_str::<StateFile>(&t).ok())
            .filter(|s| s.version == STATE_VERSION);
        if let Some(state) = &state {
            self.layouts.apply_state_order(&state.order);
            if let Some(stars) = &state.stars {
                self.layouts.stars = stars
                    .iter()
                    .filter_map(|n| self.layouts.find(n).map(|e| e.name().to_owned()))
                    .collect();
            }
            self.layouts.starred_only = state.starred_only;
            self.layouts.top_collapsed = state.top_bar_collapsed;
            self.layouts.left_collapsed = state.left_collapsed;
            self.layouts.right_collapsed = state.right_collapsed;
            self.layouts.mru = state
                .mru
                .iter()
                .filter_map(|n| self.layouts.find(n).map(|e| e.name().to_owned()))
                .collect();
            self.layouts.locks = state
                .locks
                .iter()
                .filter_map(|(n, l)| Some((self.layouts.find(n)?.name().to_owned(), *l)))
                .collect();
        }
        let Some(first) = self.layouts.list.first().map(|e| e.name().to_owned()) else {
            self.refresh_bindings();
            return;
        };
        // Stage 2: the locks follow the layout (LAY-48); set below once the
        // layout is chosen.
        match state {
            Some(state) => {
                let name = state
                    .current
                    .filter(|n| self.layouts.find(n).is_some())
                    .unwrap_or(first);
                let status = self.status.clone();
                self.open_layout_file(&name);
                self.status = status;
                let current = self.layouts.current.clone();
                self.layouts.note_shown(&current);
            }
            None => {
                self.layouts.current = first.clone();
                self.layouts.note_shown(&first);
                let paths = self.environment.paths();
                let old = [paths.view_layout(), paths.view_displays(), paths.layout()]
                    .iter()
                    .any(|p| p.is_file());
                if old
                    && self
                        .layouts
                        .find(&first)
                        .is_some_and(|entry| entry.name() == "Model" && entry.user.is_none())
                {
                    // The old loaders, unchanged: Model looks as before.
                    self.load_views();
                    self.layouts.migrating = true;
                } else {
                    let status = self.status.clone();
                    self.open_layout_file(&first);
                    self.status = status;
                }
            }
        }
        self.apply_layout_locks();
        // The open layout's own keys over the global map (LAY-20).
        self.refresh_bindings();
    }

    /// At launch, where the old layout.json used to load: the first run
    /// reads its panel widths, splits, tabs and Items filter into Model.
    pub(super) fn finish_layout_migration(&mut self, ctx: &Context) {
        if std::mem::take(&mut self.layouts.migrating) {
            self.load_layout(ctx);
        }
    }

    /// Before the side panels draw: widths from a layout just opened.
    pub(super) fn apply_layout_side(&mut self, ctx: &Context) {
        if !std::mem::take(&mut self.layouts.side_pending) {
            return;
        }
        let window = ctx.content_rect().width().max(1.);
        let (left, right) = self.layouts.side;
        for (id, fraction, min, max) in [
            ("tools", left, layout::LEFT_MIN, layout::LEFT_MAX),
            ("scene", right, layout::RIGHT_MIN, layout::RIGHT_MAX),
        ] {
            let width = (fraction * window).clamp(min, max);
            ctx.data_mut(|d| {
                d.insert_persisted(
                    Id::new(id),
                    egui::containers::panel::PanelState {
                        rect: Rect::from_min_size(Pos2::ZERO, vec2(width, 600.)),
                    },
                )
            });
        }
    }

    /// Every frame: read the side widths back, and keep the current layout
    /// as a file once it differs from the shipped one (LAY-40/41).
    pub(super) fn persist_layout(&mut self, ctx: &Context) {
        let window = ctx.content_rect().width();
        if !self.layouts.mixed_dock && !self.layouts.side_pending && window > 1. {
            let width = |id, default| {
                egui::containers::panel::PanelState::load(ctx, Id::new(id))
                    .map_or(default, |s| s.rect.width())
            };
            self.layouts.side = (
                width("tools", layout::LEFT_DEFAULT) / window,
                width("scene", layout::RIGHT_DEFAULT) / window,
            );
        }
        if self.layouts.migrating {
            return;
        }
        // The layout holds each view's camera, so an orbit, pan or zoom
        // drag changes it every frame. Writing the file that often froze
        // Windows for 100-300 ms now and then (the write waits on the
        // virus scanner). It is written once the drag ends, and on quit.
        // A wheel zoom is smoothed over several frames, so it waits the
        // same way until the wheel goes quiet. The extra frame makes sure
        // the held-back write happens even if nothing else redraws.
        let scrolling = ctx.input(|i| {
            i.smooth_scroll_delta != egui::Vec2::ZERO || i.raw_scroll_delta != egui::Vec2::ZERO
        });
        if self.navigation.is_some() || scrolling {
            ctx.request_repaint();
            return;
        }
        self.note_torn_windows();
        self.write_current_layout();
    }

    /// The next write compares against nothing (writes the file).
    pub(super) fn layouts_saved_clear(&mut self) {
        self.layouts.saved.clear();
    }

    pub(super) fn persist_layout_file(&mut self) {
        self.write_current_layout();
    }

    /// On quit: write what a camera drag still held back (persist_layout).
    pub(super) fn persist_layout_on_exit(&mut self) {
        if !self.layouts.migrating {
            self.write_current_layout();
        }
    }

    /// Write the current layout's file when it changed, or take the user's
    /// copy away once it matches the shipped one again.
    fn write_current_layout(&mut self) {
        let Some(doc) = self.capture_layout() else {
            return;
        };
        let snapshot = doc.canonical().to_json();
        if snapshot != self.layouts.saved {
            let name = self.layouts.current.clone();
            let path = self.layout_file(&name);
            if let Some(entry) = self.layouts.find_mut(&name) {
                let same = entry.shipped.as_ref().is_some_and(|s| s.same_as(&doc));
                entry.user = (!same).then(|| doc.clone());
                if let Some(path) = path {
                    if same {
                        // A file we cannot read may belong to a newer
                        // build. Only remove a recognized copy of this layout.
                        if std::fs::read_to_string(&path)
                            .ok()
                            .and_then(|text| read_layout(&text).ok())
                            .is_some_and(|saved| saved.name.eq_ignore_ascii_case(&name))
                        {
                            let _ = std::fs::remove_file(&path);
                        }
                    } else if let Some(folder) = path.parent()
                        && std::fs::create_dir_all(folder).is_ok()
                    {
                        let temp = path.with_extension("json.tmp");
                        if std::fs::write(&temp, doc.to_json()).is_ok() {
                            let _ = std::fs::rename(&temp, &path);
                        }
                    }
                }
            }
            self.layouts.saved = snapshot;
        }
        self.write_layouts_state();
    }

    /// The layouts state file (LAY-60), when it changed.
    pub(super) fn write_layouts_state(&mut self) {
        let l = &self.layouts;
        let state = serde_json::to_string_pretty(&StateFile {
            version: STATE_VERSION,
            current: Some(l.current.clone()),
            order: l.names(),
            stars: Some(
                l.names()
                    .into_iter()
                    .filter(|n| l.stars.contains(n))
                    .collect(),
            ),
            starred_only: l.starred_only,
            top_bar_collapsed: l.top_collapsed,
            left_collapsed: l.left_collapsed,
            right_collapsed: l.right_collapsed,
            mru: l.mru.clone(),
            locks: l.locks.clone(),
        })
        .unwrap_or_default();
        if state != self.layouts.state_saved {
            let path = self.layouts_state_path();
            if let Some(folder) = path.parent() {
                let _ = std::fs::create_dir_all(folder);
            }
            let _ = std::fs::write(path, &state);
            self.layouts.state_saved = state;
        }
    }

    /// One view's slot in a layout file: its kind, its non-default
    /// display and camera settings (LAY-40/41), and its piece.
    pub(super) fn view_slot(&self, pane: Pane) -> Slot {
        let mut slot = match pane {
            Pane::Viewport(key) => {
                let display = if key == self.active_view {
                    Some(&self.display)
                } else {
                    self.displays.get(&key)
                };
                let mut slot = Slot::new(VIEW_3D);
                if let Some(d) = display.filter(|d| d.settings != self.new_view_display().settings)
                    && let Ok(value) = serde_json::to_value(d.settings)
                {
                    slot.settings = serde_json::json!({ "display": value });
                }
                let camera = if key == self.active_view {
                    Some(&self.camera)
                } else {
                    self.cameras.get(&key)
                };
                if let Some(camera) = camera.filter(|c| !c.looks_through()) {
                    let saved = ViewCamera::from(camera);
                    if saved != ViewCamera::from(&Camera::default())
                        && let Ok(value) = serde_json::to_value(saved)
                    {
                        slot.settings["camera"] = value;
                    }
                }
                slot
            }
            Pane::Uv(key) => {
                let mut slot = Slot::new(UV_VIEW);
                if let Some(view) = self.uv_views.get(&key) {
                    let default = uv_view::UvView::default();
                    if (view.center, view.zoom) != (default.center, default.zoom)
                        && let Ok(value) = serde_json::to_value(UvNavigation {
                            center: view.center,
                            zoom: view.zoom,
                        })
                    {
                        slot.settings["camera"] = value;
                    }
                }
                slot
            }
            // The name is the layout's (to_frames puts it back).
            _ => Slot::new(EMPTY),
        };
        slot.piece = pane
            .key()
            .and_then(|k| self.layouts.piece_of.get(&k).cloned());
        slot
    }

    pub(super) fn attach_dock_columns(&self, layout: &mut operandi_layout::Layout) -> Result<()> {
        if self.layouts.mixed_dock {
            return Ok(());
        }
        dock::attach_columns(
            layout,
            self.layouts.side,
            self.tool_split,
            self.scene_fraction,
        )
    }

    pub(super) fn registry_slot(&self, group: &PanelGroup, kind: PanelKind) -> Slot {
        let mut slot = Slot::new(&group.name(kind));
        if kind == PanelKind::Items && self.item_list.filter != ListFilter::default() {
            slot.settings = serde_json::json!({ "filter": self.item_list.filter });
        }
        slot
    }

    /// The current window as the current layout's file (LAY-71).
    pub(super) fn capture_layout(&self) -> Option<WindowLayout> {
        let base = self.layouts.find(&self.layouts.current)?.doc();
        let (set, tab, uv_tab) = self.toolbox_selection();
        let toolbox = ((set, tab, uv_tab) != self.layouts.toolbox_default).then(|| ToolboxState {
            set: set.label().into(),
            tab: tab.label().into(),
            uv_tab,
        });
        let mut views = self.views.clone();
        views.windows.retain(|w| !self.app_wide_window(w));
        let tabs = |group: &PanelGroup| Frame::Tabs {
            tabs: group
                .tabs
                .iter()
                .map(|&k| self.registry_slot(group, k))
                .collect(),
            active: group.active,
            minimized: false,
        };
        if self.layouts.mixed_dock {
            for slot in views.dock_tabs.values_mut() {
                if kind_named(&slot.kind) == Some(PanelKind::Items) {
                    slot.settings = serde_json::json!({ "filter": self.item_list.filter });
                }
            }
            let (frame, windows) = views.to_dock_frames(
                |pane| self.view_slot(pane),
                |pane| match pane {
                    DockPanel::Toolbox | DockPanel::ToolProperties => {
                        Frame::Panel(Slot::new(&pane.pane().title()))
                    }
                    DockPanel::ToolTabs => {
                        let mut slot = Slot::new(&Pane::ToolTabs.title());
                        slot.settings = serde_json::json!({
                            "horizontal": self.layouts.tool_tabs_horizontal.unwrap_or(false)
                        });
                        Frame::Panel(slot)
                    }
                    DockPanel::Items => tabs(&self.upper_frame),
                    DockPanel::Properties => tabs(&self.lower_frame),
                },
            );
            return Some(WindowLayout {
                frame,
                windows,
                toolbox,
                group_piece: self.layouts.group_piece.clone(),
                mode_bar_bottom: self.layouts.mode_bar_bottom,
                ..base.clone()
            });
        }
        let (centre, windows) = views.to_frames(|pane| self.view_slot(pane));
        let (left, right) = self.layouts.side;
        let mut frame = self.layouts.dock_tree.clone();
        let columns = dock::Columns::read(&mut frame).ok()?;
        *columns.left_width = left;
        *columns.right_width = right;
        columns.view.fraction = (1. - left - right).max(0.);
        let Frame::ViewArea { frame: views } = &mut columns.view.frame else {
            return None;
        };
        **views = centre;
        columns.toolbox.fraction = 1. - self.tool_split;
        columns.tool_props.fraction = self.tool_split;
        columns.upper.fraction = self.scene_fraction;
        columns.lower.fraction = 1. - self.scene_fraction;
        columns.upper.frame = tabs(&self.upper_frame);
        columns.lower.frame = tabs(&self.lower_frame);
        Some(WindowLayout {
            frame,
            windows,
            toolbox,
            group_piece: self.layouts.group_piece.clone(),
            mode_bar_bottom: self.layouts.mode_bar_bottom,
            ..base.clone()
        })
    }

    /// The highest view key a switched-away layout uses, so the layout
    /// shown now takes keys of its own (cameras and GPU state never mix).
    pub(super) fn top_view_key(&self) -> Option<u32> {
        self.layouts
            .stash
            .values()
            .flat_map(|l| l.views.panes())
            .filter_map(|p| match p {
                Pane::Viewport(k) | Pane::Uv(k) | Pane::Missing(k) => Some(k),
                _ => None,
            })
            .max()
    }

    /// Put a layout file on the window: everything is checked before
    /// anything changes, so a file that doesn't fit leaves the window as it
    /// was. Then its open command runs (UIC-221).
    pub(super) fn apply_layout_doc(
        &mut self,
        doc: &WindowLayout,
    ) -> std::result::Result<(), String> {
        self.layouts.mode_bar_bottom = doc.mode_bar_bottom;
        let mut dock_tree = doc.frame.clone();
        let first_key = self.top_view_key().map_or(0, |k| k + 1);
        let mut windows = doc.windows.clone();
        let mut placed = dock::PlacedPanels::default();
        let legacy = dock::Columns::read(&mut dock_tree).ok();
        let mixed_dock = legacy.is_none() || windows.iter().any(|w| dock::is_mixed(&w.frame));
        if mixed_dock {
            if let Some(columns) = legacy {
                dock::promote_columns(columns);
            }
            if !placed.repair_tree(&mut dock_tree) {
                dock_tree = Frame::Panel(Slot::new(EMPTY));
            }
            windows.retain_mut(|window| placed.repair_tree(&mut window.frame));
        } else if let Some(columns) = legacy {
            for part in [columns.upper, columns.lower] {
                if !placed.repair_group(&mut part.frame) {
                    part.frame = Frame::Tabs {
                        tabs: Vec::new(),
                        active: 0,
                        minimized: false,
                    };
                }
            }
        }
        let find_group = |pane| {
            dock::panel_frame(&dock_tree, pane).or_else(|| {
                windows
                    .iter()
                    .find_map(|w| dock::panel_frame(&w.frame, pane))
            })
        };
        let tool_tabs_horizontal = find_group(DockPanel::ToolTabs).map(|frame| {
            frame
                .slots()
                .first()
                .and_then(|slot| slot.settings["horizontal"].as_bool())
                .unwrap_or(false)
        });
        let (built, upper_slots, lower_slots, side, tool_split, scene_fraction) = if mixed_dock {
            let upper = find_group(DockPanel::Items);
            let lower = find_group(DockPanel::Properties);
            let built = operandi_layout::Layout::from_dock_frames(&dock_tree, &windows, first_key)?;
            (
                built,
                upper.map_or((&[] as &[Slot], 0), tabs_of),
                lower.map_or((&[] as &[Slot], 0), tabs_of),
                self.layouts.side,
                self.tool_split,
                self.scene_fraction,
            )
        } else {
            let columns = dock::Columns::read(&mut dock_tree)?;
            let (left, right) = (share(*columns.left_width), share(*columns.right_width));
            let total = left + share(columns.view.fraction) + right;
            if total <= 0. {
                return Err("The columns have no width".into());
            }
            let Frame::ViewArea { frame: centre } = &columns.view.frame else {
                return Err("The layout has no view area".into());
            };
            (
                operandi_layout::Layout::from_frames(centre, &doc.windows, first_key)?,
                tabs_of(&columns.upper.frame),
                tabs_of(&columns.lower.frame),
                (left / total, right / total),
                split_share(
                    columns.tool_props.fraction,
                    columns.toolbox.fraction,
                    tool_split::DEFAULT_FRACTION,
                ),
                split_share(columns.upper.fraction, columns.lower.fraction, 0.5),
            )
        };
        let upper = group_from(self.upper_frame.id, upper_slots);
        let lower = group_from(self.lower_frame.id, lower_slots);
        let filter = upper_slots
            .0
            .iter()
            .chain(lower_slots.0)
            .chain(built.layout.dock_tabs.values())
            .find(|s| kind_named(&s.kind) == Some(PanelKind::Items))
            .and_then(|s| serde_json::from_value::<ListFilter>(s.settings["filter"].clone()).ok())
            .unwrap_or_default();
        let mut cameras = BTreeMap::new();
        let mut displays = BTreeMap::new();
        let mut uv_views = BTreeMap::new();
        for (&key, slot) in &built.slots {
            if slot.kind == UV_VIEW {
                uv_views.insert(key, uv_view_from_slot(slot));
            } else if slot.kind == VIEW_3D {
                cameras.insert(key, camera_from_slot(slot));
                if let Ok(settings) = serde_json::from_value::<view_display::Settings>(
                    slot.settings["display"].clone(),
                ) {
                    displays.insert(
                        key,
                        view_display::ViewDisplay {
                            settings,
                            applied: None,
                        },
                    );
                }
            }
        }
        // The window's views are replaced: their state goes with them.
        for key in self.views.views() {
            self.cameras.remove(&key);
            self.displays.remove(&key);
            self.uv_views.remove(&key);
            self.view_rects.remove(&key);
            if let Some(gpu) = &mut self.viewport_gpu {
                gpu.forget(key);
            }
        }
        let views = built.layout;
        // A layout of empty panels only (LAY-74) has no view to make active:
        // its first panel's key stands in, with no camera of its own.
        let active = views
            .maximized_in(None)
            .or_else(|| views.group_views(None).into_iter().min())
            .or_else(|| views.views().first().copied())
            .or_else(|| views.panes().iter().find_map(|p| p.key()))
            .unwrap_or(0);
        self.install_live(Live {
            dock_tree,
            mixed_dock,
            tool_tabs_horizontal,
            views,
            cameras,
            displays,
            uv_views,
            active_view: active,
            category: toolbox::Category::default(),
            uv_tab: false,
            tool_set: ToolSet::Model,
            toolbox_default: MODEL_TOOLS,
            side,
            tool_split: tool_split.clamp(0., 1.),
            scene_fraction: scene_fraction.clamp(0.25, 0.65),
            upper,
            lower,
            filter,
            piece_of: built.pieces,
            group_piece: doc.group_piece.clone(),
            mode_bar_bottom: doc.mode_bar_bottom,
        });
        self.note_layout_windows();
        if let Some(line) = doc.open_command.as_deref().map(str::trim)
            && !line.is_empty()
        {
            // UIC-221: an unknown line only reports its error.
            self.execute(line);
        }
        self.layouts.toolbox_default = self.toolbox_selection();
        if let Some(saved) = &doc.toolbox
            && let Some(set) = ToolSet::named(&saved.set)
            && let Some(tab) = Category::named(&saved.tab)
        {
            self.layouts.tool_set = set;
            // A tab its set doesn't have falls back to the set's first.
            self.category = if set.tabs().contains(&tab) {
                tab
            } else {
                set.first_category().unwrap_or(tab)
            };
            self.layouts.uv_tab = saved.uv_tab;
        }
        Ok(())
    }

    fn toolbox_selection(&self) -> ToolboxSelection {
        (self.layouts.tool_set, self.category, self.layouts.uv_tab)
    }

    /// The window's layout state, to keep while another layout shows.
    fn take_live(&mut self) -> Live {
        self.cameras.insert(self.active_view, self.camera.clone());
        self.displays.insert(self.active_view, self.display.clone());
        let keys = self.views.views();
        let (cameras, displays, uv_views) = (
            take_keys(&mut self.cameras, &keys),
            take_keys(&mut self.displays, &keys),
            take_keys(&mut self.uv_views, &keys),
        );
        Live {
            dock_tree: self.layouts.dock_tree.clone(),
            mixed_dock: self.layouts.mixed_dock,
            tool_tabs_horizontal: self.layouts.tool_tabs_horizontal,
            views: self.views.clone(),
            cameras,
            displays,
            uv_views,
            active_view: self.active_view,
            category: self.category,
            uv_tab: self.layouts.uv_tab,
            tool_set: self.layouts.tool_set,
            toolbox_default: self.layouts.toolbox_default,
            side: self.layouts.side,
            tool_split: self.tool_split,
            scene_fraction: self.scene_fraction,
            upper: self.upper_frame.clone(),
            lower: self.lower_frame.clone(),
            filter: self.item_list.filter.clone(),
            piece_of: std::mem::take(&mut self.layouts.piece_of),
            group_piece: self.layouts.group_piece.take(),
            mode_bar_bottom: self.layouts.mode_bar_bottom,
        }
    }

    /// Put a layout's state on the window.
    fn install_live(&mut self, live: Live) {
        let mut views = live.views;
        // A view split off later takes a key no other layout uses.
        if let Some(top) = self.top_view_key() {
            while views.new_viewport_key() < top {}
        }
        self.layouts.dock_tree = live.dock_tree;
        self.layouts.mixed_dock = live.mixed_dock;
        self.layouts.tool_tabs_horizontal = live.tool_tabs_horizontal;
        self.views = views;
        self.cameras.extend(live.cameras);
        self.displays.extend(live.displays);
        self.uv_views.extend(live.uv_views);
        self.category = live.category;
        self.layouts.uv_tab = live.uv_tab;
        self.layouts.tool_set = live.tool_set;
        self.layouts.toolbox_default = live.toolbox_default;
        self.layouts.side = live.side;
        self.layouts.side_pending = true;
        self.tool_split = live.tool_split;
        self.scene_fraction = live.scene_fraction;
        self.upper_frame = live.upper;
        self.lower_frame = live.lower;
        self.item_list.filter = live.filter;
        self.layouts.piece_of = live.piece_of;
        self.layouts.group_piece = live.group_piece;
        self.layouts.mode_bar_bottom = live.mode_bar_bottom;
        let view = if self.views.views().contains(&live.active_view) {
            live.active_view
        } else {
            self.views
                .views()
                .first()
                .copied()
                .unwrap_or(live.active_view)
        };
        self.active_view = view;
        self.camera = self.cameras.get(&view).cloned().unwrap_or_default();
        self.display = self
            .displays
            .get(&view)
            .cloned()
            .unwrap_or_else(|| self.new_view_display());
        // Views of other layouts (their cameras are stashed) leave the
        // per-view maps.
        self.forget_unlisted_views();
        self.uv_views.retain(|k, _| self.views.views().contains(k));
    }

    /// Open `name` from its file: the user's copy, or the shipped file when
    /// that one doesn't read or fit (LAY-49).
    fn open_layout_file(&mut self, name: &str) -> bool {
        let Some(entry) = self.layouts.find(name).cloned() else {
            return false;
        };
        self.layouts.current = entry.name().to_owned();
        self.layouts.saved.clear();
        for doc in [entry.user.as_ref(), entry.shipped.as_ref()]
            .into_iter()
            .flatten()
        {
            match self.apply_layout_doc(doc) {
                Ok(()) => {
                    if entry.user.is_some() {
                        self.remember_loaded_layout();
                    }
                    return true;
                }
                Err(e) => self.status = format!("{} layout: {e}", entry.name()),
            }
        }
        false
    }

    /// Compare autosave against the projected arrangement, preserving
    /// the user's original bytes until an edit (D-10, UIC-194).
    fn remember_loaded_layout(&mut self) {
        if let Some(doc) = self.capture_layout() {
            self.layouts.saved = doc.canonical().to_json();
        }
    }

    /// Whether the screen shows layout `rest` (a `layout.restore` argument
    /// line): it is the open layout, with its own tool set. Highlights the
    /// layout's entry (Jason, 2026-11-19). Panel moves save into the
    /// layout as they happen, so they don't count as a mismatch.
    pub(crate) fn layout_matches_screen(&self, rest: &str) -> bool {
        let Ok(args) = command::tokens(rest) else {
            return false;
        };
        let Ok(bound) = operandi_core::command_args::bind("layout.restore", &args, &["name"])
        else {
            return false;
        };
        let Some(name) = bound[0] else {
            return false;
        };
        name == self.layouts.current && self.layouts.tool_set == self.layouts.toolbox_default.0
    }

    /// `layout.restore <name>` (LAY-30): the one way every switch goes.
    pub(super) fn restore_layout_line(&mut self, rest: &str) {
        let args = match command::tokens(rest) {
            Ok(args) => args,
            Err(e) => {
                self.status = format!("layout.restore: {e}");
                return;
            }
        };
        let name = match operandi_core::command_args::bind("layout.restore", &args, &["name"]) {
            Ok(b) => b[0].map(str::to_owned),
            Err(e) => {
                self.status = e.to_string();
                return;
            }
        };
        match name {
            Some(name) => {
                if let Err(e) = self.restore_layout(&name) {
                    self.status = e;
                }
            }
            None => {
                self.status = format!(
                    "layout.restore needs a layout name: {}",
                    self.layouts.names().join(", ")
                )
            }
        }
    }

    /// Show the layout called `name`, keeping the current one's state
    /// (LAY-33). The scene, undo and selection never change (LAY-02).
    pub(super) fn restore_layout(&mut self, name: &str) -> std::result::Result<(), String> {
        let Some(target) = self.layouts.find(name).map(|e| e.name().to_owned()) else {
            return Err(format!(
                "No layout named {name}; there are {}",
                self.layouts.names().join(", ")
            ));
        };
        if target == self.layouts.current {
            self.status = format!("{target} layout");
            return Ok(());
        }
        if self.editor.session.live.is_some() || self.pointer_held {
            return Err("Drop the tool before switching layouts".into());
        }
        // The layout being left keeps its file up to date, then its state.
        self.note_torn_windows();
        self.write_current_layout();
        // LAY-52 palettes and legacy app-wide windows go with the app.
        let carried = self.take_floating();
        let outgoing = self.layouts.current.clone();
        let live = self.take_live();
        self.layouts.stash.insert(outgoing.clone(), live);
        // LAY-Q2: back to a layout a view was torn from, its saved version
        // from before the tear-off opens.
        let torn_from = self.palettes.origins.iter().any(|o| o.layout == target);
        if torn_from {
            self.layouts.stash.remove(&target);
            self.layouts.current = target.clone();
            self.layouts.saved.clear();
        }
        match self.layouts.stash.remove(&target) {
            _ if torn_from && self.open_torn_from(&target) => {}
            Some(live) => {
                self.layouts.current = target.clone();
                self.install_live(live);
                self.remember_loaded_layout();
            }
            None => {
                if !self.open_layout_file(&target) {
                    // Nothing opened: back to the layout we left.
                    let error = std::mem::take(&mut self.status);
                    let live = self.layouts.stash.remove(&outgoing).expect("just stashed");
                    self.layouts.current = outgoing;
                    self.install_live(live);
                    self.put_back_floating(carried);
                    return Err(error);
                }
            }
        }
        self.put_back_floating(carried);
        self.palettes.view_order = Vec::new();
        self.status = format!("{target} layout");
        self.layouts.note_shown(&target);
        self.apply_layout_locks();
        self.write_layouts_state();
        Ok(())
    }

    /// Put `name` back to its shipped arrangement (UIC-15, asked first by
    /// the bar and the menu; `operandi.layout.revert` runs `revert_layout`).
    #[cfg(test)]
    pub(super) fn reset_layout(&mut self, name: &str) {
        if let Err(e) = self.revert_layout(name) {
            self.status = e;
        }
    }

    /// `reset_layout`, reporting why it couldn't. The layout's own keys
    /// stay unless `clear_keys` (provisional, LAY-Q56: Revert puts back the
    /// arrangement; the keys are the Input Map's to clear).
    #[cfg(test)]
    pub(super) fn revert_layout(&mut self, name: &str) -> std::result::Result<(), String> {
        self.revert_layout_keys(name, false)
    }

    /// `revert_layout`, also dropping the layout's keys when `clear_keys`.
    pub(super) fn revert_layout_keys(
        &mut self,
        name: &str,
        clear_keys: bool,
    ) -> std::result::Result<(), String> {
        let Some(entry) = self.layouts.find(name).cloned() else {
            return Err(format!("No layout named {name}"));
        };
        let name = entry.name().to_owned();
        let keys = entry.doc().keys.clone();
        let Some(shipped) = entry.shipped.clone() else {
            // Provisional (LAY-Q12): a layout you made has no default.
            return Err(format!(
                "{name} is a layout you made: it has no shipped arrangement to go back to"
            ));
        };
        if name == self.layouts.current {
            if self.editor.session.live.is_some() || self.pointer_held {
                return Err("Drop the tool before resetting the layout".into());
            }
            self.note_torn_windows();
            let carried = self.take_floating();
            let result = self.apply_layout_doc(&shipped);
            self.put_back_floating(carried);
            result.map_err(|e| format!("{name} layout: {e}"))?;
        } else {
            self.layouts.stash.remove(&name);
        }
        if let Some(path) = self.layout_file(&name)
            && path.is_file()
        {
            let _ = std::fs::remove_file(path);
        }
        if let Some(entry) = self.layouts.find_mut(&name) {
            entry.user = None;
        }
        self.layouts.saved.clear();
        let kept = !clear_keys && !keys.is_empty();
        if kept {
            self.set_layout_keys(&name, keys.clone())?;
        }
        self.refresh_bindings();
        self.status = if kept {
            format!(
                "{name} layout reset; its {} key{} kept (Input Map: Clear this layout's keys)",
                keys.len(),
                if keys.len() == 1 { " is" } else { "s are" }
            )
        } else {
            format!("{name} layout reset")
        };
        Ok(())
    }

    /// `operandi.toolbox.tab <tab>`: open a toolbox tab; the UV tab also
    /// shows in the strip from then on (a layout's open command, LAY-02).
    pub(super) fn toolbox_tab_line(&mut self, rest: &str) {
        let args = command::tokens(rest).unwrap_or_default();
        let wanted = operandi_core::command_args::bind("operandi.toolbox.tab", &args, &["tab"])
            .ok()
            .and_then(|b| b[0].map(str::to_owned));
        let Some(category) = wanted.as_deref().and_then(toolbox::Category::named) else {
            self.status = format!(
                "operandi.toolbox.tab needs one of: {}",
                toolbox::Category::ALL
                    .iter()
                    .map(|c| c.label().to_lowercase())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return;
        };
        if !self.confirm_fields() {
            return;
        }
        // The first tool set with that tab (Jason, 2026-11-23).
        if !self.show_toolbox_tab(category) {
            self.status = format!("No tool set has the {} tab", category.label());
            return;
        }
        if category == toolbox::Category::Workbench {
            self.workbench.tab_shown = true;
        }
        self.status = format!("{} tab", category.label());
    }

    /// The command line that shows `name` (quoted when it needs it).
    pub(super) fn restore_line(name: &str) -> String {
        if name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            format!("layout.restore {name}")
        } else {
            format!("layout.restore {{{name}}}")
        }
    }

    /// The reset question (UIC-15: never reset without asking).
    pub(super) fn layout_reset_dialog(&mut self, ctx: &Context) {
        let Some(name) = self.layouts.confirm_reset.clone() else {
            return;
        };
        let mut answer = None;
        egui::Window::new("Reset Layout")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0., 0.))
            .show(ctx, |ui| {
                ui.label(format!(
                    "Reset the {name} layout to its default arrangement?"
                ));
                // LAY-Q56 (provisional): the layout's own keys stay unless
                // this is ticked; Revert never drops them silently.
                let keys = self.layouts.find(&name).map_or(0, |e| e.doc().keys.len());
                if keys > 0 {
                    ui.checkbox(
                        &mut self.layouts.confirm_reset_keys,
                        format!(
                            "Also clear its {keys} layout key{} (they are kept otherwise)",
                            if keys == 1 { "" } else { "s" }
                        ),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        answer = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        answer = Some(false);
                    }
                });
            });
        if let Some(yes) = answer {
            self.answer_layout_reset(yes);
        }
    }

    /// The reset question's answer. UIC-220: Reset runs a command line, as
    /// every switch does (FLAGS.md's stage 1 exception is gone).
    pub(super) fn answer_layout_reset(&mut self, yes: bool) {
        let Some(name) = self.layouts.confirm_reset.take() else {
            return;
        };
        let clear_keys = std::mem::take(&mut self.layouts.confirm_reset_keys);
        if yes {
            let mut args = vec![("layout", name.as_str())];
            if clear_keys {
                args.push(("keys", "true"));
            }
            self.execute(&crate::layouts_edit::line("operandi.layout.revert", &args));
        }
    }
}
