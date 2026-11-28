//! Palettes (layouts stage 6, specifications/layouts-2026-11-16
//! BEHAVIOR-LAYOUTS-STAGE6.md; UIC-205..207).
//!
//! - LAY-50: a palette is a layout file (`assets/palettes/*.json`, a
//!   [`WindowLayout`] with a `palette` block) shown in its own floating OS
//!   window (UIC-183). One toggle opens and closes it: the menu item, the
//!   panel header's Float entry and `layout.window hash:<id>` all run
//!   [`Operandi::toggle_palette`]. Each palette exists once; opening an
//!   open one brings it forward.
//! - LAY-51 / UIC-73: each palette opens at its file's size the first
//!   time, then where it was left (palettes.json in the data folder).
//! - LAY-52: palettes, and since LAY-Q2's answer every floating (torn-off)
//!   view, belong to the app, not to a layout. A layout switch carries
//!   them to the next layout unchanged ([`Operandi::carry_floating`]).
//!   Switching back to the layout a view was torn from opens that layout's
//!   saved version from before the tear-off, so the slot gets a fresh view
//!   with the camera the view had when it was torn off.
//! - LAY-53 / UIC-206: Toggle Palettes (`) hides every palette and shows
//!   them again.
//! - LAY-54 / UIC-207: the UV Editor (Texture > Open UV Editor, or a
//!   double-click in the UV Maps list) is the `uvEditor` palette: a UV view
//!   with the UV tools down its right side (uv_editor.rs).
//! - LAY-55: Layout > Windows/Palettes lists them, with Toggle Palettes.
//!
//! Today's floating Toolbox, Items and Properties (floating.rs) are the
//! `toolbox`, `items` and `properties` palettes: they draw exactly as the
//! old floating panels did, and the side column gives up the panel while
//! it floats. A user's palette file of the same hash in the data folder's
//! `Palettes` folder sits over the shipped one.
use super::*;
use crate::floating::Panel;
use crate::panel_registry::PanelKind;
use operandi_layout::{FloatWindow, Frame as LFrame, Pane, Slot, UV_VIEW, WindowLayout};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The built-in palette files (LAY-50), listed by build.rs.
pub(super) const SHIPPED: &[(&str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/shipped_palettes.rs"));

/// The state file's version.
const STATE_VERSION: u32 = 1;

/// The UV Editor palette's hash (LAY-54).
pub(super) const UV_EDITOR: &str = "uvEditor";
/// The Preset Browser palette's hash (its window keeps its own state,
/// preset_browser.rs).
pub(super) const PRESET_BROWSER: &str = crate::preset_browser::WINDOW_HASH;

/// What a palette shows.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Content {
    /// A side panel taken out of its column (the old Window > Float).
    Side(Panel),
    /// The Preset Browser's window.
    PresetBrowser,
    /// A registered panel kind, drawn on its own.
    Panel(PanelKind),
    /// A UV view in a floating view window; `tools` adds the UV Editor's
    /// tool strip.
    UvView { tools: bool },
    /// The Form Editor (layouts stage 7, form_editor.rs).
    FormEditor,
}

/// One palette the app offers.
#[derive(Clone, Debug)]
pub(super) struct Palette {
    pub hash: String,
    pub name: String,
    pub order: i32,
    pub size: [f32; 2],
    pub content: Content,
}

/// Where a palette's window was last.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Place {
    pub size: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 2]>,
}

/// A view torn off a layout, for LAY-Q2's "switching back restores the
/// saved version".
#[derive(Clone)]
pub(super) struct Origin {
    /// The floating window it is in now.
    pub window: u32,
    /// Its view keys now (a dock back into the main window ends it).
    pub keys: Vec<u32>,
    pub layout: String,
    /// The layout's file from before the tear-off.
    pub doc: WindowLayout,
    /// Its place among that file's views, in file order.
    pub index: Option<usize>,
    pub camera: Camera,
}

/// The palettes and the app's floating windows.
#[derive(Clone)]
pub(super) struct Palettes {
    pub list: Vec<Palette>,
    /// Open palettes (side-panel and panel palettes; view palettes and the
    /// Preset Browser answer from their own state).
    pub open: BTreeSet<String>,
    pub places: BTreeMap<String, Place>,
    /// Toggle Palettes hid them (LAY-53).
    pub hidden: bool,
    /// View palettes' view keys, by hash.
    pub views: BTreeMap<String, u32>,
    /// A palette to bring forward next frame.
    pub focus: Option<String>,
    /// Torn-off views and where they came from.
    pub origins: Vec<Origin>,
    /// The floating windows seen last frame.
    known_windows: BTreeSet<u32>,
    /// View windows from palettes.json or torn off the legacy view area.
    legacy_view_windows: BTreeSet<u32>,
    /// The main window's view keys in file order, last frame.
    pub view_order: Vec<u32>,
    /// The state file's last written text.
    saved: String,
    /// Floating views read from the state file, waiting for the layout.
    pending_views: Vec<FloatWindow>,
}

/// The state file (palettes.json).
#[derive(Default, Serialize, Deserialize)]
struct StateFile {
    version: u32,
    #[serde(default)]
    open: Vec<String>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    places: BTreeMap<String, Place>,
    /// Floating (torn-off) views: they belong to the app (LAY-52).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    views: Vec<FloatWindow>,
}

/// The old floating-panels file (panel-layout.json), read once.
#[derive(Deserialize)]
struct OldFloating {
    version: u32,
    floating: Vec<(Panel, [f32; 2])>,
}

/// A palette from its file; `None` for a file that isn't one.
pub(super) fn palette_from(doc: &WindowLayout) -> Option<Palette> {
    let info = doc.palette.as_ref()?;
    let LFrame::Panel(slot) = &doc.frame else {
        return None;
    };
    let content = match slot.kind.as_str() {
        crate::layouts::TOOLBOX => Content::Side(Panel::Toolbox),
        "Items" => Content::Side(Panel::Items),
        "Properties" => Content::Side(Panel::Properties),
        "Preset Browser" => Content::PresetBrowser,
        "Form Editor" => Content::FormEditor,
        UV_VIEW => Content::UvView {
            tools: slot.settings["tools"].as_bool().unwrap_or(false),
        },
        other => Content::Panel(crate::layouts::kind_named(other)?),
    };
    let size = info
        .size
        .map(|v| if v.is_finite() { v.max(120.) } else { 400. });
    Some(Palette {
        hash: info.hash.clone(),
        name: doc.name.clone(),
        order: doc.order,
        size,
        content,
    })
}

impl Default for Palettes {
    /// The shipped palettes, all closed.
    fn default() -> Self {
        Self {
            list: Self::read(SHIPPED, None),
            open: BTreeSet::new(),
            places: BTreeMap::new(),
            hidden: false,
            views: BTreeMap::new(),
            focus: None,
            origins: Vec::new(),
            known_windows: BTreeSet::new(),
            legacy_view_windows: BTreeSet::new(),
            view_order: Vec::new(),
            saved: String::new(),
            pending_views: Vec::new(),
        }
    }
}

impl Palettes {
    /// The shipped palettes with `user` files over them, by order then name.
    pub fn read(shipped: &[(&str, &str)], user: Option<&std::path::Path>) -> Vec<Palette> {
        let mut list: Vec<Palette> = shipped
            .iter()
            .filter_map(|(_, text)| WindowLayout::from_json(text).ok())
            .filter_map(|doc| palette_from(&doc))
            .collect();
        if let Some(folder) = user
            && let Ok(entries) = std::fs::read_dir(folder)
        {
            let mut files: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            files.sort();
            for path in files {
                let Some(palette) = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|t| WindowLayout::from_json(&t).ok())
                    .and_then(|d| palette_from(&d))
                else {
                    continue;
                };
                list.retain(|p| p.hash != palette.hash);
                list.push(palette);
            }
        }
        list.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.name.cmp(&b.name)));
        list
    }

    pub fn find(&self, hash: &str) -> Option<&Palette> {
        self.list.iter().find(|p| p.hash.eq_ignore_ascii_case(hash))
    }

    /// The palette showing a side panel.
    pub fn side_hash(&self, panel: Panel) -> Option<&str> {
        self.list
            .iter()
            .find(|p| p.content == Content::Side(panel))
            .map(|p| p.hash.as_str())
    }

    /// A palette's window place: where it was, else its file's size.
    pub fn place(&self, hash: &str) -> Place {
        self.places.get(hash).copied().unwrap_or(Place {
            size: self.find(hash).map_or([400., 600.], |p| p.size),
            position: None,
        })
    }
}

impl Operandi {
    fn palettes_folder(&self) -> PathBuf {
        self.environment.paths().data_root().join("Palettes")
    }

    fn palettes_state_path(&self) -> PathBuf {
        self.environment.paths().data_root().join("palettes.json")
    }

    /// At launch: the palette files and where they were; anything
    /// unreadable leaves every palette closed. The first run reads the old
    /// floating panels (panel-layout.json) once.
    pub(super) fn load_palettes(&mut self) {
        let folder = self.palettes_folder();
        self.palettes.list = Palettes::read(SHIPPED, Some(&folder));
        let state = std::fs::read_to_string(self.palettes_state_path())
            .ok()
            .and_then(|t| serde_json::from_str::<StateFile>(&t).ok())
            .filter(|s| s.version == STATE_VERSION);
        let (open, hidden, places, views) = match state {
            Some(s) => (s.open, s.hidden, s.places, s.views),
            None => {
                let old = std::fs::read_to_string(self.environment.paths().panel_layout())
                    .ok()
                    .and_then(|t| serde_json::from_str::<OldFloating>(&t).ok())
                    .filter(|s| s.version == 1);
                let mut open = Vec::new();
                let mut places = BTreeMap::new();
                for (panel, size) in old.map(|o| o.floating).unwrap_or_default() {
                    if let Some(hash) = self.palettes.side_hash(panel) {
                        open.push(hash.to_owned());
                        places.insert(
                            hash.to_owned(),
                            Place {
                                size,
                                position: None,
                            },
                        );
                    }
                }
                (open, false, places, Vec::new())
            }
        };
        self.palettes.places = places
            .into_iter()
            .filter(|(_, p)| p.size.iter().all(|v| v.is_finite() && *v >= 100.))
            .collect();
        self.palettes.pending_views = views;
        for hash in open {
            if hash != PRESET_BROWSER {
                let _ = self.set_palette_open(&hash, true);
            }
        }
        self.palettes.hidden = hidden;
        self.palettes.focus = None;
        self.put_back_saved_views();
        self.palettes.known_windows = self.views.windows.iter().map(|w| w.id).collect();
    }

    /// The floating views the state file kept, back in the current layout.
    pub(super) fn put_back_saved_views(&mut self) {
        let windows = std::mem::take(&mut self.palettes.pending_views);
        if windows.is_empty() {
            return;
        }
        let Ok(built) = operandi_layout::Layout::from_frames(
            &LFrame::Panel(Slot::new(operandi_layout::EMPTY)),
            &windows,
            0,
        ) else {
            return;
        };
        for window in built.layout.windows {
            let (id, keys) = self.views.adopt_window(window);
            for (old, new) in keys {
                let Some(slot) = built.slots.get(&old) else {
                    continue;
                };
                if slot.kind == UV_VIEW {
                    self.uv_views.insert(new, layouts::uv_view_from_slot(slot));
                } else if slot.kind == operandi_layout::VIEW_3D {
                    self.cameras.insert(new, layouts::camera_from_slot(slot));
                    if let Ok(settings) = serde_json::from_value::<view_display::Settings>(
                        slot.settings["display"].clone(),
                    ) {
                        self.displays.insert(
                            new,
                            view_display::ViewDisplay {
                                settings,
                                applied: None,
                            },
                        );
                    }
                }
            }
            self.palettes.known_windows.insert(id);
            self.palettes.legacy_view_windows.insert(id);
        }
    }

    /// Whether a palette is open (hidden ones count as open).
    pub(super) fn palette_open(&self, hash: &str) -> bool {
        let Some(p) = self.palettes.find(hash) else {
            return false;
        };
        match p.content {
            Content::PresetBrowser => self.preset_browser.window_open,
            Content::UvView { .. } => self
                .palettes
                .views
                .get(&p.hash)
                .is_some_and(|k| self.views.views().contains(k)),
            Content::Side(_) | Content::Panel(_) | Content::FormEditor => {
                self.palettes.open.contains(&p.hash)
            }
        }
    }

    /// Open or close a palette. Opening one that is open brings it forward;
    /// opening any shows hidden palettes again (provisional, LAY-Q61).
    pub(super) fn set_palette_open(
        &mut self,
        hash: &str,
        open: bool,
    ) -> std::result::Result<(), String> {
        let Some(p) = self.palettes.find(hash).cloned() else {
            return Err(format!(
                "layout.window: no window {hash}; Operandi has {}",
                self.palettes
                    .list
                    .iter()
                    .map(|p| format!("hash:{}", p.hash))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        if open
            && let Content::Side(panel) = p.content
            && self.views.window_of(panel.pane()).is_some()
        {
            // A side row in a floating dock group already has a window.
            self.palettes.focus = Some(p.hash);
            return Ok(());
        }
        let was = self.palette_open(&p.hash);
        if open {
            self.palettes.hidden = false;
            self.palettes.focus = Some(p.hash.clone());
            if was {
                self.status = format!("{} is open", p.name);
                return Ok(());
            }
        } else if !was {
            return Ok(());
        }
        match p.content {
            Content::PresetBrowser => self.set_preset_browser_window(open),
            Content::Side(panel) => {
                if open {
                    self.palettes.open.insert(p.hash.clone());
                } else {
                    self.palettes.open.remove(&p.hash);
                    self.dots.side_positions.remove(&panel);
                }
            }
            Content::Panel(_) | Content::FormEditor => {
                if open {
                    self.palettes.open.insert(p.hash.clone());
                } else {
                    self.palettes.open.remove(&p.hash);
                }
            }
            Content::UvView { .. } => {
                if open {
                    self.open_view_palette(&p);
                } else if let Some(key) = self.palettes.views.remove(&p.hash)
                    && let Some(window) = self.views.window_of(Pane::Uv(key))
                {
                    self.remember_window_place(&p.hash, window);
                    self.discard_view_window(window);
                    self.uv_views.remove(&key);
                }
            }
        }
        self.status = format!("{} {}", p.name, if open { "opened" } else { "closed" });
        self.write_palettes_state();
        Ok(())
    }

    /// LAY-50: the one toggle every way of opening a palette runs.
    pub(super) fn toggle_palette(&mut self, hash: &str) {
        let open = self.palette_open(hash);
        // Hidden: the toggle shows them again rather than closing one the
        // user can't see (provisional, LAY-Q61).
        let open = !open || self.palettes.hidden;
        if let Err(e) = self.set_palette_open(hash, open) {
            self.status = e;
        }
    }

    /// A UV view palette's window: a UV view of its own, at its place.
    fn open_view_palette(&mut self, p: &Palette) {
        let key = self.views.new_viewport_key();
        self.cameras.insert(key, self.camera.clone());
        self.uv_views.entry(key).or_default();
        let place = self.palettes.place(&p.hash);
        self.views
            .new_window(Pane::Uv(key), place.size, place.position);
        if let Some(window) = self.views.window_of(Pane::Uv(key)) {
            self.palettes.known_windows.insert(window);
        }
        self.palettes.views.insert(p.hash.clone(), key);
    }

    fn remember_window_place(&mut self, hash: &str, window: u32) {
        if let Some(w) = self.views.windows.iter().find(|w| w.id == window) {
            self.palettes.places.insert(
                hash.to_owned(),
                Place {
                    size: w.size,
                    position: w.position,
                },
            );
        }
    }

    /// The palette a floating view window is, if any.
    pub(super) fn palette_of_window(&self, window: u32) -> Option<&str> {
        self.palettes
            .views
            .iter()
            .find(|(_, k)| self.views.window_of(Pane::Uv(**k)) == Some(window))
            .map(|(h, _)| h.as_str())
    }

    /// D-10: only palettes and legacy app-wide views travel across layouts.
    pub(super) fn app_wide_window(&self, window: &operandi_layout::Window) -> bool {
        self.palette_of_window(window.id).is_some()
            || (!window.has_dock_panels() && self.palettes.legacy_view_windows.contains(&window.id))
    }

    /// A floating view window is a hidden palette (views.rs skips it).
    pub(super) fn window_hidden(&self, window: u32) -> bool {
        self.palettes.hidden && self.palette_of_window(window).is_some()
    }

    /// LAY-53 / UIC-206: Toggle Palettes hides every open palette, and
    /// shows them all again.
    pub(super) fn toggle_palettes(&mut self) {
        let any = self
            .palettes
            .list
            .iter()
            .any(|p| self.palette_open(&p.hash));
        if !any && !self.palettes.hidden {
            self.status = "No palettes are open".into();
            return;
        }
        self.palettes.hidden = !self.palettes.hidden;
        self.status = if self.palettes.hidden {
            "Palettes hidden (` shows them)".into()
        } else {
            "Palettes shown".into()
        };
        self.write_palettes_state();
    }

    /// `layout.window hash:<palette> [open:]` (LAY-50): bare toggles.
    pub(super) fn palette_window_line(
        &mut self,
        hash: Option<&str>,
        open: Option<bool>,
    ) -> std::result::Result<(), String> {
        let Some(hash) = hash else {
            return Err(format!(
                "layout.window needs hash: one of {}",
                self.palettes
                    .list
                    .iter()
                    .map(|p| p.hash.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        match open {
            None => {
                if self.palettes.find(hash).is_none() {
                    return self.set_palette_open(hash, true);
                }
                self.toggle_palette(hash);
                Ok(())
            }
            Some(open) => self.set_palette_open(hash, open),
        }
    }

    /// The viewport a palette's window is drawn in.
    fn palette_viewport(&self, hash: &str) -> Option<egui::ViewportId> {
        let p = self.palettes.find(hash)?;
        Some(match p.content {
            Content::Side(panel) => self.views.window_of(panel.pane()).map_or_else(
                || egui::ViewportId::from_hash_of(("operandi-panel-window", panel)),
                |id| egui::ViewportId::from_hash_of(("operandi-view-window", id)),
            ),
            Content::PresetBrowser => {
                egui::ViewportId::from_hash_of(("operandi-palette-window", "preset-browser"))
            }
            Content::Panel(_) | Content::FormEditor => {
                egui::ViewportId::from_hash_of(("operandi-palette", &p.hash))
            }
            Content::UvView { .. } => {
                let key = self.palettes.views.get(&p.hash)?;
                let window = self.views.window_of(Pane::Uv(*key))?;
                egui::ViewportId::from_hash_of(("operandi-view-window", window))
            }
        })
    }

    /// Each frame: the panel palettes' windows, the palette brought
    /// forward, closed view palettes noticed, the state file kept.
    pub(super) fn palettes_frame(&mut self, ctx: &Context) {
        if let Some(hash) = self.palettes.focus.take()
            && let Some(id) = self.palette_viewport(&hash)
        {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
        }
        // A view palette whose window was closed (its close button).
        let gone: Vec<String> = self
            .palettes
            .views
            .iter()
            .filter(|(_, k)| !self.views.views().contains(k))
            .map(|(h, _)| h.clone())
            .collect();
        for hash in gone {
            self.palettes.views.remove(&hash);
        }
        // Where open view palettes are, so they reopen there.
        let views: Vec<(String, u32)> = self
            .palettes
            .views
            .iter()
            .map(|(h, &k)| (h.clone(), k))
            .collect();
        for (hash, key) in views {
            if let Some(window) = self.views.window_of(Pane::Uv(key)) {
                self.remember_window_place(&hash, window);
            }
        }
        if !self.palettes.hidden {
            self.panel_palettes(ctx);
        }
        self.write_palettes_state();
    }

    /// The registered-panel palettes (Shading, Groups, Vertex Maps,
    /// Command History) in their own windows.
    fn panel_palettes(&mut self, ctx: &Context) {
        let open: Vec<Palette> = self
            .palettes
            .list
            .iter()
            .filter(|p| {
                matches!(p.content, Content::Panel(_) | Content::FormEditor)
                    && self.palettes.open.contains(&p.hash)
            })
            .cloned()
            .collect();
        for p in open {
            let kind = match p.content {
                Content::Panel(kind) => Some(kind),
                Content::FormEditor => None,
                _ => continue,
            };
            let place = self.palettes.place(&p.hash);
            let spec = floating::OsWindow {
                id: egui::ViewportId::from_hash_of(("operandi-palette", &p.hash)),
                title: format!("{} — {}", p.name, self.environment.title()),
                size: place.size,
                position: place.position,
                min_size: [200., 160.],
            };
            let shown = self.os_window(ctx, spec, |app, ctx| {
                CentralPanel::default()
                    .frame(
                        Frame::side_top_panel(&ctx.style())
                            .fill(ctx.style().visuals.panel_fill)
                            .inner_margin(4.),
                    )
                    .show(ctx, |ui| {
                        let rect = ui.available_rect_before_wrap();
                        match kind {
                            Some(kind) => {
                                ui.push_id(("palette", kind), |ui| {
                                    (crate::panel_registry::panel_type(kind).draw)(app, ui, rect)
                                });
                            }
                            None => app.form_editor_ui(ui),
                        }
                    });
            });
            let mut place = place;
            if let Some(size) = shown.size.filter(|s| s.iter().all(|v| *v >= 100.)) {
                place.size = size;
            }
            if let Some(at) = shown.position {
                place.position = Some(at);
            }
            self.palettes.places.insert(p.hash.clone(), place);
            if !shown.open {
                self.palettes.open.remove(&p.hash);
            }
        }
    }

    /// The floating views as frames, for the state file.
    fn floating_view_frames(&self) -> Vec<FloatWindow> {
        let mut only = self.views.clone();
        let excluded: Vec<u32> = only
            .windows
            .iter()
            .filter(|w| !self.app_wide_window(w) || self.palette_of_window(w.id).is_some())
            .map(|w| w.id)
            .collect();
        for w in excluded {
            only.take_window(w);
        }
        only.to_frames(|pane| self.view_slot(pane)).1
    }

    /// palettes.json, when it changed.
    pub(super) fn write_palettes_state(&mut self) {
        let open = self
            .palettes
            .list
            .iter()
            .filter(|p| p.content != Content::PresetBrowser && self.palette_open(&p.hash))
            .map(|p| p.hash.clone())
            .collect();
        let state = StateFile {
            version: STATE_VERSION,
            open,
            hidden: self.palettes.hidden,
            places: self.palettes.places.clone(),
            views: self.floating_view_frames(),
        };
        let Ok(text) = serde_json::to_string_pretty(&state) else {
            return;
        };
        if text != self.palettes.saved {
            let path = self.palettes_state_path();
            if let Some(folder) = path.parent() {
                let _ = std::fs::create_dir_all(folder);
            }
            let temp = path.with_extension("json.tmp");
            if std::fs::write(&temp, &text).is_ok() {
                let _ = std::fs::rename(&temp, &path);
            }
            self.palettes.saved = text;
        }
    }

    // ------------------------------------------------ LAY-52 / LAY-Q2

    /// The main window's view keys in file order (capture_layout's order).
    fn main_view_order(&self) -> Vec<u32> {
        let mut main = self.views.clone();
        main.windows.clear();
        let mut order = Vec::new();
        main.to_frames(|pane| {
            if let Some(k) = pane.view_key() {
                order.push(k);
            }
            Slot::new(operandi_layout::EMPTY)
        });
        order
    }

    /// Before the current layout's file is written: a floating window that
    /// is new since last frame was torn off this layout (LAY-Q2), so the
    /// layout's file from before (written last frame) is kept with the
    /// view's camera. Windows a layout file opened, and palettes, were
    /// noted when they opened.
    pub(super) fn note_torn_windows(&mut self) {
        let now: BTreeSet<u32> = self.views.windows.iter().map(|w| w.id).collect();
        let fresh: Vec<u32> = now
            .difference(&self.palettes.known_windows)
            .copied()
            .filter(|&w| self.palette_of_window(w).is_none())
            .collect();
        for window in fresh {
            let Some(w) = self.views.windows.iter().find(|x| x.id == window) else {
                continue;
            };
            if self.layouts.mixed_dock || w.has_dock_panels() {
                continue;
            }
            self.palettes.legacy_view_windows.insert(window);
            let keys: Vec<u32> = w
                .tree
                .tiles
                .iter()
                .filter_map(|(_, t)| match t {
                    egui_tiles::Tile::Pane(p) => p.view_key(),
                    _ => None,
                })
                .collect();
            let Some(&first) = keys.first() else {
                continue;
            };
            let Some(doc) = self
                .layouts
                .find(&self.layouts.current)
                .map(|e| e.doc().clone())
            else {
                continue;
            };
            // A view moved from one floating window to another keeps the
            // origin it has.
            if self
                .palettes
                .origins
                .iter()
                .any(|o| o.keys.iter().any(|k| keys.contains(k)))
            {
                for o in &mut self.palettes.origins {
                    if o.keys.iter().any(|k| keys.contains(k)) {
                        o.window = window;
                    }
                }
                continue;
            }
            let camera = if first == self.active_view {
                self.camera.clone()
            } else {
                self.cameras.get(&first).cloned().unwrap_or_default()
            };
            let index = self.palettes.view_order.iter().position(|&k| k == first);
            self.palettes.origins.push(Origin {
                window,
                keys,
                layout: self.layouts.current.clone(),
                doc: WindowLayout {
                    windows: Vec::new(),
                    ..doc
                },
                index,
                camera,
            });
        }
        // A view docked back into the main window is that layout's again.
        let in_main = self.main_view_order();
        self.palettes
            .origins
            .retain(|o| !o.keys.iter().any(|k| in_main.contains(k)));
        self.palettes.legacy_view_windows.retain(|id| {
            self.views
                .windows
                .iter()
                .any(|w| w.id == *id && !w.has_dock_panels())
        });
        self.palettes.known_windows = now;
        self.palettes.view_order = in_main;
    }

    /// The floating windows a layout file just opened: not tear-offs.
    pub(super) fn note_layout_windows(&mut self) {
        self.palettes.known_windows = self.views.windows.iter().map(|w| w.id).collect();
    }

    /// LAY-52: take app-wide windows out of the layout being left, with
    /// its views' cameras, settings and UV state.
    pub(super) fn take_floating(&mut self) -> Vec<Carried> {
        self.cameras.insert(self.active_view, self.camera.clone());
        self.displays.insert(self.active_view, self.display.clone());
        let ids: Vec<u32> = self
            .views
            .windows
            .iter()
            .filter(|w| self.app_wide_window(w))
            .map(|w| w.id)
            .collect();
        let mut out = Vec::new();
        for id in ids {
            let hash = self.palette_of_window(id).map(str::to_owned);
            let palette =
                hash.and_then(|hash| self.palettes.views.remove(&hash).map(|key| (hash, key)));
            self.palettes.legacy_view_windows.remove(&id);
            let Some(window) = self.views.take_window(id) else {
                continue;
            };
            let keys: Vec<u32> = window
                .tree
                .tiles
                .iter()
                .filter_map(|(_, t)| match t {
                    egui_tiles::Tile::Pane(p) => p.key(),
                    _ => None,
                })
                .collect();
            let mut state = BTreeMap::new();
            for k in &keys {
                state.insert(
                    *k,
                    (
                        self.cameras.remove(k),
                        self.displays.remove(k),
                        self.uv_views.remove(k),
                        self.layouts.piece_of.remove(k),
                    ),
                );
                self.view_rects.remove(k);
                if let Some(gpu) = &mut self.viewport_gpu {
                    gpu.forget(*k);
                }
            }
            out.push(Carried {
                old_id: id,
                window,
                state,
                palette,
            });
        }
        out
    }

    /// LAY-52: the carried windows into the layout now showing, under keys
    /// of its own.
    pub(super) fn put_back_floating(&mut self, carried: Vec<Carried>) {
        for c in carried {
            let (id, keys) = self.views.adopt_window(c.window);
            for (old, (camera, display, uv, piece)) in c.state {
                let Some(&new) = keys.get(&old) else {
                    continue;
                };
                if let Some(v) = camera {
                    self.cameras.insert(new, v);
                }
                if let Some(v) = display {
                    self.displays.insert(new, v);
                }
                if let Some(v) = uv {
                    self.uv_views.insert(new, v);
                }
                if let Some(v) = piece {
                    self.layouts.piece_of.insert(new, v);
                }
            }
            if let Some((hash, key)) = c.palette {
                if let Some(&new) = keys.get(&key) {
                    self.palettes.views.insert(hash, new);
                }
            } else {
                self.palettes.legacy_view_windows.insert(id);
            }
            for o in &mut self.palettes.origins {
                if o.window == c.old_id {
                    o.window = id;
                    for k in &mut o.keys {
                        if let Some(&new) = keys.get(k) {
                            *k = new;
                        }
                    }
                }
            }
            self.palettes.known_windows.insert(id);
        }
        self.palettes.known_windows = self.views.windows.iter().map(|w| w.id).collect();
    }

    /// LAY-Q2: a layout a view was torn from opens its saved version from
    /// before the tear-off; the slot's fresh view takes the camera the view
    /// had then. Returns whether it opened that way.
    pub(super) fn open_torn_from(&mut self, name: &str) -> bool {
        let mine: Vec<Origin> = self
            .palettes
            .origins
            .iter()
            .filter(|o| o.layout == name)
            .cloned()
            .collect();
        let Some(first) = mine.first() else {
            return false;
        };
        if self.apply_layout_doc(&first.doc).is_err() {
            return false;
        }
        let order = self.main_view_order();
        // Only the earliest tear-off's index is in this file's order; later
        // ones keep the layout's own camera (provisional, LAY-Q60).
        if let Some(&key) = first.index.and_then(|i| order.get(i)) {
            self.cameras.insert(key, first.camera.clone());
            if key == self.active_view {
                self.camera = first.camera.clone();
            }
        }
        self.palettes.origins.retain(|o| o.layout != name);
        true
    }
}

/// A floating window on its way from one layout to the next.
pub(super) struct Carried {
    old_id: u32,
    window: operandi_layout::Window,
    #[allow(clippy::type_complexity)]
    state: BTreeMap<
        u32,
        (
            Option<Camera>,
            Option<view_display::ViewDisplay>,
            Option<uv_view::UvView>,
            Option<String>,
        ),
    >,
    palette: Option<(String, u32)>,
}
