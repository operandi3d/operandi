//! Drawing the preset browser (PB-100..112, CD-01..04): its own OS window
//! (PB-100, Jason's 11-10 rule; F6 toggles it too, PB-101), and the panel
//! body it or a panel slot shows: folder tree on the left (PB-103, the shared
//! tree list), contents on the right (PB-104), search at the top right
//! (PB-26), view buttons, size slider and gear options in the footer
//! (PB-102). Behaviour lives in `preset_browser.rs`.
//!
//! Icons are letters for now (provisional): the icon generator has no
//! recipes for the browser's kinds yet.
//!
//! Library pack (F1): pack roots show "<name> <version>"; the built-in
//! Basics root comes after the user's roots with its write actions greyed
//! and a note (UIC-117); with no pack installed the contents offer "Get the
//! free Operandi Library…" and "Locate Library Folder…" (UIC-03: both open
//! something else, so both carry "…").
use super::*;
use crate::preset_browser::{Ask, Drag, READ_ONLY_NOTE, SaveMenu, Target, ViewMode};
use crate::tree_list::{
    Cell, Column, Heading, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow, TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::preset::library::{Bucket, Entry, EntryKind, ResolutionChoice, SortBy};
use operandi_core::preset::{PresetKind, PresetSource};
use std::collections::HashMap;
use std::path::Path;

/// Tree row key for "(add path…)".
const ADD_KEY: u64 = u64::MAX;
/// The OS window's title.
const TITLE: &str = "Preset Browser";

/// A tile's name line and kind line heights, in points.
const TILE_NAME: f32 = 15.;
const TILE_KIND: f32 = 13.;

/// One line of a tile's text, cut off at `rect`'s edge; centred in the
/// Vertical tiles, left-aligned in the Horizontal ones.
fn tile_text(ui: &Ui, text: &str, size: f32, color: Color32, rect: Rect, centred: bool) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(size), color);
    let width = galley.size().x.min(rect.width());
    let x = if centred {
        rect.center().x - width / 2.
    } else {
        rect.left()
    };
    let y = rect.center().y - galley.size().y / 2.;
    ui.painter()
        .with_clip_rect(rect)
        .galley(pos2(x, y), galley, color);
}

/// The footer's view button pictures, drawn in `rect` (no icon files:
/// a few strokes read better at this size than a glyph would).
fn paint_view_icon(ui: &Ui, rect: Rect, mode: ViewMode, color: Color32) {
    let p = ui.painter();
    let c = rect.center();
    let stroke = egui::Stroke::new(1f32, color);
    match mode {
        // Two tiles side by side: a name line over a picture.
        ViewMode::Vertical => {
            for dx in [-3.5f32, 3.5] {
                let x = c.x + dx;
                p.line_segment([pos2(x - 2.5, c.y - 5.), pos2(x + 2.5, c.y - 5.)], stroke);
                p.rect_filled(
                    Rect::from_center_size(pos2(x, c.y + 1.), vec2(5., 6.)),
                    1.,
                    color,
                );
            }
        }
        // Two rows: a picture with two text lines to its right.
        ViewMode::Horizontal => {
            for dy in [-3.5f32, 3.5] {
                let y = c.y + dy;
                p.rect_filled(
                    Rect::from_center_size(pos2(c.x - 3.5, y), vec2(5., 5.)),
                    1.,
                    color,
                );
                p.line_segment([pos2(c.x, y - 1.2), pos2(c.x + 6., y - 1.2)], stroke);
                p.line_segment([pos2(c.x, y + 1.2), pos2(c.x + 4., y + 1.2)], stroke);
            }
        }
        // Rows of a dot and a line.
        ViewMode::List => {
            for dy in [-4f32, 0., 4.] {
                let y = c.y + dy;
                p.circle_filled(pos2(c.x - 5., y), 1.1, color);
                p.line_segment([pos2(c.x - 2.5, y), pos2(c.x + 6., y)], stroke);
            }
        }
    }
}

pub(super) fn key_of(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = operandi_core::fxhash::FxHasher::default();
    path.hash(&mut h);
    h.finish() & !(1 << 63)
}

/// The kind's letter on placeholders and list rows (provisional icons).
fn letter(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Preset(PresetKind::Material) => "M",
        EntryKind::Preset(PresetKind::Item) => "I",
        EntryKind::Preset(PresetKind::Assembly) => "A",
        EntryKind::Preset(PresetKind::Environment) => "E",
        EntryKind::Preset(PresetKind::RenderSettings) => "R",
        EntryKind::BrokenPreset => "!",
        EntryKind::Scene => "S",
        EntryKind::Model => "3D",
        EntryKind::Image => "P",
        EntryKind::Matcap => "●",
        EntryKind::Environment => "H",
        EntryKind::TextureSet => "T",
        EntryKind::MaterialX => "X",
        EntryKind::ModoMeshPreset => "3D",
        EntryKind::ModoMaterial => "M",
        EntryKind::ModoPreset => "P",
        EntryKind::Other => "·",
    }
}

fn size_label(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.1} GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b if b >= 1 << 10 => format!("{} KB", b >> 10),
        b => format!("{b} B"),
    }
}

/// A root drop next to this folder: before it, or after it (`true`).
type DropNear = (PathBuf, bool);

/// What a frame of the body asked for, run after drawing.
#[derive(Default)]
struct Wants {
    load: Option<Entry>,
    /// A34: an online asset's menu choice.
    online: Option<crate::online_library_ui::Want>,
    show: Option<PathBuf>,
    toggle: Option<(PathBuf, bool)>,
    add_root: bool,
    save_here: Option<(PathBuf, PresetSource)>,
    rename_root: Option<PathBuf>,
    remove_root: Option<PathBuf>,
    reveal: Option<PathBuf>,
    open_default: Option<PathBuf>,
    refresh: Option<Option<PathBuf>>,
    rename: Option<PathBuf>,
    duplicate: Option<PathBuf>,
    delete: Option<PathBuf>,
    thumb_render: Option<PathBuf>,
    thumb_file: Option<PathBuf>,
    thumb_refresh: Option<PathBuf>,
    save_texture_set: Option<Entry>,
    new_folder: Option<PathBuf>,
    get_library: bool,
    /// MC-06: this matcap in every 3D view.
    matcap_all: Option<PathBuf>,
    /// PG-07: open as its own scene / import into this one.
    open: Option<PathBuf>,
    import: Option<PathBuf>,
    locate_library: bool,
    /// Roots dragged before (false) or after (true) another root, or to
    /// the end.
    reorder: Option<(Vec<PathBuf>, Option<DropNear>)>,
}

struct FolderHooks<'a> {
    keys: &'a HashMap<u64, (PathBuf, bool)>,
    wants: &'a mut Wants,
    has_material: bool,
    has_items: bool,
    /// The read-only Basics folder.
    basics: Option<&'a Path>,
    /// The online libraries' read-only roots (A34).
    online: Vec<PathBuf>,
}

impl TreeHooks<u64> for FolderHooks<'_> {
    fn has_row_menu(&self, key: u64) -> bool {
        key != ADD_KEY
    }
    fn row_menu(&mut self, ui: &mut Ui, key: u64, _rename: &mut dyn FnMut()) {
        let Some((path, root)) = self.keys.get(&key).cloned() else {
            return;
        };
        let read_only = self.basics.is_some_and(|b| path.starts_with(b))
            || self.online.iter().any(|r| path.starts_with(r));
        folder_menu(
            ui,
            &path,
            root,
            read_only,
            self.wants,
            self.has_material,
            self.has_items,
        );
    }
}

/// A folder's right-click menu (CD-02 for roots, PB-121). In the
/// read-only Basics the write actions grey with a note (UIC-117).
fn folder_menu(
    ui: &mut Ui,
    path: &Path,
    root: bool,
    read_only: bool,
    wants: &mut Wants,
    has_material: bool,
    has_items: bool,
) {
    let gated = |ui: &mut Ui, label: &str, ok: bool, why: &str| {
        let why = if read_only {
            Some(READ_ONLY_NOTE)
        } else {
            (!ok).then_some(why)
        };
        ui.gated(why, Button::new(label)).clicked()
    };
    if gated(ui, "New Folder…", true, "") {
        wants.new_folder = Some(path.to_path_buf());
        ui.close();
    }
    if gated(
        ui,
        "Save Selection Here…",
        has_items,
        "Select one or more items first",
    ) {
        wants.save_here = Some((path.to_path_buf(), PresetSource::Items));
        ui.close();
    }
    if gated(
        ui,
        "Save Material Here…",
        has_material,
        "Select a material or material group in the Shader Tree first",
    ) {
        wants.save_here = Some((path.to_path_buf(), PresetSource::Material(0)));
        ui.close();
    }
    if gated(ui, "Save Render Settings Here…", true, "") {
        wants.save_here = Some((path.to_path_buf(), PresetSource::RenderSettings));
        ui.close();
    }
    ui.separator();
    if root && gated(ui, "Rename…", true, "") {
        wants.rename_root = Some(path.to_path_buf());
        ui.close();
    }
    if root
        && ui
            .gated(read_only.then_some(READ_ONLY_NOTE), Button::new("Remove…"))
            .on_hover_text("Takes the folder out of the browser; its files stay on disk")
            .clicked()
    {
        wants.remove_root = Some(path.to_path_buf());
        ui.close();
    }
    if ui.button(reveal_label()).clicked() {
        wants.reveal = Some(path.to_path_buf());
        ui.close();
    }
    if ui.button("Refresh").clicked() {
        wants.refresh = Some(Some(path.to_path_buf()));
        ui.close();
    }
}

fn reveal_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Show in Finder"
    } else if cfg!(target_os = "windows") {
        "Show in Explorer"
    } else {
        "Show in File Manager"
    }
}

/// The name that travels with the pointer while an entry is dragged out
/// of the browser.
fn drag_label(ctx: &Context, p: Pos2, name: &str) {
    egui::Area::new(Id::new("preset-browser-drag"))
        .order(egui::Order::Tooltip)
        .fixed_pos(p + vec2(12., 8.))
        .interactable(false)
        .show(ctx, |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(name);
            });
        });
}

/// An entry's right-click menu (PB-112's Stage 1 subset). In the
/// read-only Basics, Rename, Duplicate and Delete grey with a note
/// (UIC-117).
fn entry_menu(ui: &mut Ui, entry: &Entry, wants: &mut Wants, cycles: bool, read_only: bool) {
    // A34: an online asset's own menu (online_library_ui.rs).
    if entry.online.is_some() {
        crate::online_library_ui::entry_menu(ui, entry, &mut wants.online);
        return;
    }
    // MC-12: a Modo file Operandi can't use keeps Show in Finder only.
    if entry.kind == EntryKind::ModoPreset {
        if ui.button(reveal_label()).clicked() {
            wants.reveal = Some(entry.path.clone());
            ui.close();
        }
        return;
    }
    let why = read_only.then_some(READ_ONLY_NOTE);
    // PG-07: scenes and models open as their own scene, or join this one.
    if matches!(entry.kind, EntryKind::Scene | EntryKind::Model) {
        if ui
            .button("Open")
            .on_hover_text("Opens the file as its own scene (scene.open)")
            .clicked()
        {
            wants.open = Some(entry.path.clone());
            ui.close();
        }
        if ui
            .button("Import Into Current Scene")
            .on_hover_text(
                "Adds its items to this scene, selected, in one undo step (scene.importReference)",
            )
            .clicked()
        {
            wants.import = Some(entry.path.clone());
            ui.close();
        }
    } else if ui.button("Load").clicked() {
        wants.load = Some(entry.clone());
        ui.close();
    }
    if ui.gated(why, Button::new("Rename…")).clicked() {
        wants.rename = Some(entry.path.clone());
        ui.close();
    }
    if ui.gated(why, Button::new("Duplicate…")).clicked() {
        wants.duplicate = Some(entry.path.clone());
        ui.close();
    }
    if ui
        .gated(why, Button::new("Delete…"))
        .on_hover_text("Deletes the file from disk (asks first)")
        .clicked()
    {
        wants.delete = Some(entry.path.clone());
        ui.close();
    }
    ui.separator();
    if entry.kind == EntryKind::TextureSet && ui.button("Save as Material Preset…").clicked() {
        wants.save_texture_set = Some(entry.clone());
        ui.close();
    }
    if ui.button(reveal_label()).clicked() {
        wants.reveal = Some(entry.path.clone());
        ui.close();
    }
    // MC-06.
    if entry.kind == EntryKind::Matcap
        && ui
            .button("Use in All 3D Views")
            .on_hover_text("Every 3D view shows this matcap, in the Matcap style")
            .clicked()
    {
        wants.matcap_all = Some(entry.path.clone());
        ui.close();
    }
    if matches!(
        entry.kind,
        EntryKind::Image | EntryKind::Matcap | EntryKind::Environment | EntryKind::Other
    ) && ui.button("Open with Default App").clicked()
    {
        wants.open_default = Some(entry.path.clone());
        ui.close();
    }
    if entry.kind.has_picture() {
        ui.separator();
        if ui.button("Replace Thumbnail with Last Render").clicked() {
            wants.thumb_render = Some(entry.path.clone());
            ui.close();
        }
        if ui.button("Replace Thumbnail with Image File…").clicked() {
            wants.thumb_file = Some(entry.path.clone());
            ui.close();
        }
        let tip = if cycles {
            "Reads the picture again and renders a fresh preview"
        } else {
            "Reads the picture again (this build has no renderer)"
        };
        if ui.button("Refresh Thumbnail").on_hover_text(tip).clicked() {
            wants.thumb_refresh = Some(entry.path.clone());
            ui.close();
        }
    }
}

impl Operandi {
    /// Per frame, from `compose`: the window, drags and the browser's
    /// questions.
    pub(super) fn preset_browser_frame(&mut self, ctx: &Context) {
        self.tick_preset_browser(ctx);
        if std::mem::take(&mut self.preset_browser.popover_toggle) {
            self.toggle_preset_browser_under_pointer(ctx);
        }
        if self.preset_browser.window_open && !self.palettes.hidden {
            self.preset_browser_window(ctx);
        }
        // A drag out of the browser's own window: its label over the main
        // window, all the way to where it's dropped.
        if self.preset_browser.drag_in_window
            && let Some(drag) = &self.preset_browser.drag
        {
            if let Some(p) = self.main_window_pointer(ctx)
                && ctx.content_rect().contains(p)
            {
                drag_label(ctx, p, &drag.entry.name);
            }
            ctx.request_repaint();
        }
        self.preset_browser_drops(ctx);
        self.preset_browser_questions(ctx);
        self.save_preset_browser();
    }

    /// PB-100: the browser floats in its own movable, resizable OS window,
    /// like the other pop-up panels (Jason, 2026-11-10 and 2026-11-11: the
    /// old F6 popover was stuck inside the main window). It stays above the
    /// main window, and its place and size are remembered (UIC-73,
    /// PB-T11).
    fn preset_browser_window(&mut self, ctx: &Context) {
        let spec = crate::floating::OsWindow {
            id: egui::ViewportId::from_hash_of(("operandi-palette-window", "preset-browser")),
            title: TITLE.to_owned(),
            size: self.preset_browser.window_size,
            position: self.preset_browser.window_pos,
            min_size: [360., 240.],
        };
        let shown = self.os_window(ctx, spec, |app, ctx| {
            // Where the window is on screen, for drops between windows.
            let ppp = ctx.pixels_per_point();
            let place = ctx.input(|i| {
                let v = i.viewport();
                (v.inner_rect, v.native_pixels_per_point.unwrap_or(1.))
            });
            if let (Some(inner), native) = place {
                app.preset_browser.window_native = Some((inner.min, native, ppp));
            }
            CentralPanel::default()
                .frame(
                    Frame::side_top_panel(&ctx.style())
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(4.),
                )
                .show(ctx, |ui| app.preset_browser_body(ui, true));
        });
        if let Some(size) = shown.size {
            self.preset_browser.window_size = size;
        }
        if let Some(at) = shown.position {
            self.preset_browser.window_pos = Some(at);
        }
        if !shown.open {
            self.preset_browser.window_open = false;
        }
    }

    /// The panel body: shared by the window and a panel slot.
    pub(super) fn preset_browser_body(&mut self, ui: &mut Ui, in_window: bool) {
        let mut wants = Wants::default();
        let full = ui.available_rect_before_wrap();
        self.preset_browser.body_rect = Some((full, in_window));
        // MC-02: in its own window, folders dropped from the Finder land
        // here (the main window sees its own drops).
        if in_window {
            let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .filter(|p| p.is_dir())
                    .collect()
            });
            let at = ui.ctx().input(|i| i.pointer.latest_pos());
            if let Some(folder) = dropped.into_iter().next() {
                self.browser_folder_dropped(folder, at.or(Some(full.center())), true);
            }
        }
        // Top row: where we are, the busy mark, the search toggle (PB-26).
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            let label = self
                .preset_browser
                .folder
                .as_ref()
                .map(|f| crate::preset_browser::folder_name(f))
                .unwrap_or_else(|| "No folder".into());
            ui.label(RichText::new(label).strong());
            if self.preset_browser.listing_busy() || self.preset_browser.thumbs.busy() {
                ui.add(egui::Spinner::new().size(11.))
                    .on_hover_text("Reading the folder…");
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let on = self.preset_browser.search_open;
                // The funnel stays at the far right (UIC-132); the kind
                // filter is a label and a dropdown beside it (UIC-133).
                if tree_list::toggle_button(ui, "F", on)
                    .on_hover_text(if on {
                        "Hide the search"
                    } else {
                        "Search this folder and everything below it"
                    })
                    .clicked()
                {
                    self.preset_browser.search_open = !on;
                    self.preset_browser.search_focus = !on;
                }
                let mut bucket = self.preset_browser.kind_filter;
                egui::ComboBox::from_id_salt("preset-browser-kind")
                    .selected_text(bucket.label())
                    .width(96.)
                    .show_ui(ui, |ui| {
                        for b in Bucket::ALL {
                            ui.selectable_value(&mut bucket, b, b.label());
                        }
                    })
                    .response
                    .on_hover_text("Show only one kind of file (operandi.pbview.kindFilter)");
                self.preset_browser.set_kind_filter(bucket);
                ui.weak("Show");
            });
        });
        if self.preset_browser.search_open {
            let field = ui.add(
                TextEdit::singleline(&mut self.preset_browser.search)
                    .hint_text("Search")
                    .id_salt("preset-browser-search")
                    .desired_width(f32::INFINITY),
            );
            self.text_editors.insert(field.id);
            crate::numeric_tab::register(ui, &field, Some(&self.preset_browser.search));
            if std::mem::take(&mut self.preset_browser.search_focus) {
                field.request_focus();
            }
            if field.changed() {
                self.preset_browser.search_changed();
                self.preset_browser.highlight =
                    self.preset_browser.shown().first().map(|e| e.path.clone());
            }
            if field.has_focus() {
                // PB-26: Up/Down move the highlight, Enter loads it.
                let (down, up, enter) = ui.input(|i| {
                    (
                        i.key_pressed(Key::ArrowDown),
                        i.key_pressed(Key::ArrowUp),
                        i.key_pressed(Key::Enter),
                    )
                });
                if down || up {
                    self.move_highlight(if down { 1 } else { -1 });
                }
                if enter && let Some(e) = self.highlighted_entry() {
                    wants.load = Some(e);
                }
            }
        }
        let footer = 24.;
        let body = ui.available_rect_before_wrap();
        let body = Rect::from_min_max(
            body.min,
            pos2(body.max.x, (body.max.y - footer).max(body.min.y + 40.)),
        );
        let tree_width = (body.width() * 0.3).clamp(130., 260.);
        let tree_rect = Rect::from_min_max(body.min, pos2(body.min.x + tree_width, body.max.y));
        let contents_rect = Rect::from_min_max(pos2(tree_rect.max.x + 4., body.min.y), body.max);
        let mut tree_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(tree_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        self.preset_browser_tree(&mut tree_ui, &mut wants, in_window);
        ui.painter().vline(
            tree_rect.max.x + 2.,
            body.y_range(),
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
        let mut contents_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(contents_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        self.preset_browser_contents(&mut contents_ui, &mut wants);
        let footer_rect = Rect::from_min_max(
            pos2(full.min.x, body.max.y + 2.),
            pos2(full.max.x, body.max.y + footer),
        );
        let mut footer_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(footer_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        self.preset_browser_footer(&mut footer_ui);
        ui.allocate_rect(full, Sense::hover());
        self.preset_browser_keys(ui, &mut wants);
        self.preset_browser_drag_frame(ui.ctx(), in_window);
        self.preset_browser_save_menu(&ui.ctx().clone(), in_window);
        self.preset_browser_folder_drop_menu(&ui.ctx().clone(), in_window);
        self.run_browser_wants(wants);
    }

    /// The folder tree (PB-103, PB-11..13, CD-01).
    fn preset_browser_tree(&mut self, ui: &mut Ui, wants: &mut Wants, in_window: bool) {
        let spec = TreeSpec {
            id: "preset-browser-folders",
            columns: vec![
                Column::state(
                    Heading::Glyph("×"),
                    "Remove a root from the browser (its files stay)",
                ),
                Column::name(),
            ],
            own_search: true,
            last_fills: false,
        };
        let mut rows = Vec::new();
        let mut keys: HashMap<u64, (PathBuf, bool)> = HashMap::new();
        let roots = self.preset_browser.roots.clone();
        for root in &roots {
            let missing = self.preset_browser.missing.contains(&root.path);
            let key = key_of(&root.path);
            keys.insert(key, (root.path.clone(), true));
            let open = self.preset_browser.expanded.contains(&root.path);
            let children = if missing {
                Vec::new()
            } else {
                self.preset_browser.subfolders(&root.path)
            };
            let title = self.preset_browser.root_title(root);
            let mut row = TreeRow::new(key, 0, title.clone(), 2);
            row.icon = RowIcon::Icon(crate::icons::Icon::TreeFolder);
            row.disclosure = (!children.is_empty()).then_some(open);
            row.style.bold = true;
            row.style.dimmed = missing;
            // BQ-06: roots can be dragged into a new order.
            row.draggable = true;
            if missing {
                // CD-01, UIC-117: greyed with a note, never dropped.
                row.name = format!("{title} (not found)");
                row.tip = Some(format!(
                    "{} isn't there now; it comes back when the drive does",
                    root.path.display()
                ));
            } else if let Some(m) = self.preset_browser.manifest(&root.path) {
                // F1: a pack shows its name and version; with two versions
                // installed the older one says which is newer.
                let mut tip = format!(
                    "{}\n{} items ({})\n{}",
                    m.title(),
                    m.items.len(),
                    m.licence_summary(),
                    root.path.display()
                );
                if let Some((newer, at)) = self.preset_browser.newer_version(&root.path) {
                    row.name = format!("{title} (newer: {newer})");
                    tip += &format!(
                        "\nA newer version, {newer}, is installed at {}",
                        at.display()
                    );
                }
                row.tip = Some(tip);
            } else {
                row.tip = Some(root.path.display().to_string());
            }
            row.selected = if self.preset_browser.folder.as_ref() == Some(&root.path) {
                Selected::Primary
            } else {
                Selected::No
            };
            row.cells[0] = Cell::Toggle {
                on: false,
                tip: "Remove from the browser (asks first; files stay on disk)".into(),
                dimmed: false,
            };
            rows.push(row);
            if open {
                self.folder_rows(&root.path, 1, &mut rows, &mut keys);
            }
        }
        // F1: the built-in Basics, after the user's roots; read-only, so
        // no remove toggle and no drag.
        if let Some(basics) = self.preset_browser.basics.clone() {
            let key = key_of(&basics);
            keys.insert(key, (basics.clone(), true));
            let mut row = TreeRow::new(key, 0, operandi_core::preset::basics::ROOT_NAME, 2);
            row.icon = RowIcon::Icon(crate::icons::Icon::TreeFolder);
            row.style.bold = true;
            row.tip = Some(format!(
                "Simple materials built into Operandi (GPL-3.0-or-later); read-only\n{}",
                basics.display()
            ));
            row.selected = if self.preset_browser.folder.as_ref() == Some(&basics) {
                Selected::Primary
            } else {
                Selected::No
            };
            row.cells[0] = Cell::Empty;
            rows.push(row);
        }
        // A34: the online libraries that are on, after the Basics.
        self.online_root_rows(&mut rows, &mut keys);
        let mut add = TreeRow::new(ADD_KEY, 0, "(add path…)", 2);
        add.fixed = true;
        add.style.dimmed = true;
        add.tip = Some("Add any folder as a new top-level root".into());
        rows.push(add);
        let mut state = std::mem::take(&mut self.preset_browser.tree);
        let mut editors = std::mem::take(&mut self.text_editors);
        let has_material = self.selected_material_row().is_some();
        let has_items = PresetSource::Items.build(&self.editor.scene).is_ok();
        let events = {
            let basics = self.preset_browser.basics.clone();
            let mut hooks = FolderHooks {
                keys: &keys,
                wants,
                has_material,
                has_items,
                basics: basics.as_deref(),
                online: self
                    .preset_browser
                    .online_roots
                    .iter()
                    .map(|(r, _)| r.clone())
                    .collect(),
            };
            tree_list::show(ui, &spec, &mut state, &rows, &mut hooks, &mut editors)
        };
        self.text_editors = editors;
        self.preset_browser.folder_rects = state
            .rects
            .iter()
            .filter_map(|(k, r)| keys.get(k).map(|(p, _)| (p.clone(), *r)))
            .collect();
        self.preset_browser.folder_rects_in_window = in_window;
        self.preset_browser.tree = state;
        for event in events {
            match event {
                TreeEvent::Click { key: ADD_KEY, .. } | TreeEvent::DoubleClick(ADD_KEY) => {
                    wants.add_root = true
                }
                TreeEvent::Click { key, .. } => {
                    if let Some((path, _)) = keys.get(&key) {
                        wants.show = Some(path.clone());
                    }
                }
                TreeEvent::DoubleClick(key) | TreeEvent::Disclosure { key, .. } => {
                    if let Some((path, _)) = keys.get(&key) {
                        let open = !self.preset_browser.expanded.contains(path);
                        wants.toggle = Some((path.clone(), open));
                    }
                }
                TreeEvent::Drop {
                    keys: moved, at, ..
                } => {
                    let paths: Vec<PathBuf> = moved
                        .iter()
                        .filter_map(|k| {
                            keys.get(k)
                                .filter(|(_, root)| *root)
                                .map(|(p, _)| p.clone())
                        })
                        .filter(|p| !self.preset_browser.read_only(p))
                        .collect();
                    let target = match at {
                        tree_list::DropAt::Before(k) => {
                            keys.get(&k).map(|(p, _)| (p.clone(), false))
                        }
                        tree_list::DropAt::After(k) | tree_list::DropAt::On(k) => {
                            keys.get(&k).map(|(p, _)| (p.clone(), true))
                        }
                        tree_list::DropAt::End => None,
                    };
                    wants.reorder = Some((paths, target));
                }
                TreeEvent::Toggle { key, column: 0, .. } => {
                    if let Some((path, true)) = keys.get(&key) {
                        wants.remove_root = Some(path.clone());
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn folder_rows(
        &mut self,
        folder: &Path,
        depth: usize,
        rows: &mut Vec<TreeRow<u64>>,
        keys: &mut HashMap<u64, (PathBuf, bool)>,
    ) {
        if depth > 24 {
            return;
        }
        for child in self.preset_browser.subfolders(folder) {
            let key = key_of(&child);
            keys.insert(key, (child.clone(), false));
            let open = self.preset_browser.expanded.contains(&child);
            let grand = if open {
                self.preset_browser.subfolders(&child)
            } else {
                library_has_subfolders(&child)
            };
            let mut row = TreeRow::new(key, depth, crate::preset_browser::folder_name(&child), 2);
            row.icon = RowIcon::Icon(crate::icons::Icon::TreeFolder);
            row.disclosure = (!grand.is_empty()).then_some(open);
            row.selected = if self.preset_browser.folder.as_ref() == Some(&child) {
                Selected::Primary
            } else {
                Selected::No
            };
            row.tip = Some(child.display().to_string());
            rows.push(row);
            if open {
                self.folder_rows(&child, depth + 1, rows, keys);
            }
        }
    }

    /// The contents (PB-104): thumbnails or list.
    fn preset_browser_contents(&mut self, ui: &mut Ui, wants: &mut Wants) {
        let entries = self.preset_browser.shown();
        let area = ui.available_rect_before_wrap();
        // Clicking empty contents gives them the keyboard (PB-111).
        let back = ui.interact(area, Id::new("preset-browser-contents-bg"), Sense::click());
        if back.clicked() {
            self.preset_browser.contents_focus = true;
            self.preset_browser.highlight = None;
        }
        // F1: with no library pack installed, offer it (no download in
        // the app: the page opens in the web browser).
        if !self.preset_browser.has_pack() {
            library_offer(ui, wants);
        }
        if self.preset_browser.folder.is_none() {
            ui.weak("Add a folder with (add path…)");
            return;
        }
        // A34: an online library's credit, refresh and state.
        if self.online_contents_header(ui) {
            return;
        }
        // UIC-40, PB-26: show when search only covered part of the folder tree.
        if self.preset_browser.search_truncated() {
            ui.weak("Search limited to the first 20,000 entries");
        }
        if entries.is_empty() {
            if !self.preset_browser.listing_busy() {
                let searching = self.preset_browser.search_open
                    && !self.preset_browser.search.trim().is_empty();
                let meshes_folder = self
                    .preset_browser
                    .folder
                    .as_deref()
                    .is_some_and(crate::preset_browser::is_meshes_folder);
                ui.weak(match self.preset_browser.kind_filter.empty_hint() {
                    _ if searching => "Nothing matches",
                    // PG-01.
                    _ if meshes_folder => crate::preset_browser::MESHES_HINT,
                    // MC-21: Meshes is there even before any meshes are.
                    Some(hint) => hint,
                    None => "This folder holds nothing the browser can use",
                });
            }
            return;
        }
        match self.preset_browser.view {
            ViewMode::Vertical | ViewMode::Horizontal => self.thumbnail_grid(ui, &entries, wants),
            ViewMode::List => self.entry_list(ui, &entries, wants),
        }
    }

    /// One tile's size in the current view: Vertical stacks name, picture
    /// and kind; Horizontal puts the picture left of name and kind, so its
    /// tiles are about 2.6 pictures wide.
    fn tile_cell(&self) -> Vec2 {
        let size = self.preset_browser.thumb_size;
        match self.preset_browser.view {
            ViewMode::Horizontal => vec2((size * 2.6).max(size + 90.), size + 8.),
            _ => vec2(size + 10., size + TILE_NAME + TILE_KIND + 8.),
        }
    }

    /// How many tiles fit across.
    fn grid_columns(&self, width: f32) -> usize {
        ((width / self.tile_cell().x).floor() as usize).max(1)
    }

    /// PB-104 tiles, wrapping across and scrolling down, in the Vertical
    /// or Horizontal layout. Only visible rows are drawn and asked for
    /// (PB-106). The size is read each frame, so the footer slider resizes
    /// the tiles live while it's dragged.
    fn thumbnail_grid(&mut self, ui: &mut Ui, entries: &[Entry], wants: &mut Wants) {
        let size = self.preset_browser.thumb_size;
        let cell = self.tile_cell();
        let horizontal = self.preset_browser.view == ViewMode::Horizontal;
        let columns = self.grid_columns(ui.available_width());
        self.preset_browser.grid_columns = columns;
        let rows = entries.len().div_ceil(columns);
        let cycles = operandi_cycles::available();
        let accent = theme::chrome::active();
        egui::ScrollArea::vertical()
            .id_salt("preset-browser-grid")
            .auto_shrink([false, false])
            .show_rows(ui, cell.y, rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        for col in 0..columns {
                            let Some(entry) = entries.get(row * columns + col) else {
                                break;
                            };
                            let (rect, response) =
                                ui.allocate_exact_size(cell, Sense::click_and_drag());
                            let highlighted =
                                self.preset_browser.highlight.as_ref() == Some(&entry.path);
                            // Where the picture, the name and the kind go.
                            let (picture, name_rect, kind_rect, align) = if horizontal {
                                let picture = Rect::from_min_size(
                                    pos2(rect.left() + 4., rect.top() + 4.),
                                    vec2(size, size),
                                );
                                let text_left = picture.right() + 6.;
                                let mid = picture.center().y;
                                (
                                    picture,
                                    Rect::from_min_max(
                                        pos2(text_left, mid - TILE_NAME),
                                        pos2(rect.right() - 4., mid),
                                    ),
                                    Rect::from_min_max(
                                        pos2(text_left, mid + 1.),
                                        pos2(rect.right() - 4., mid + 1. + TILE_KIND),
                                    ),
                                    false,
                                )
                            } else {
                                let name_rect = Rect::from_min_max(
                                    pos2(rect.left() + 2., rect.top() + 3.),
                                    pos2(rect.right() - 2., rect.top() + 3. + TILE_NAME),
                                );
                                let picture = Rect::from_center_size(
                                    pos2(rect.center().x, name_rect.bottom() + 1. + size / 2.),
                                    vec2(size, size),
                                );
                                let kind_rect = Rect::from_min_max(
                                    pos2(rect.left() + 2., picture.bottom() + 1.),
                                    pos2(rect.right() - 2., picture.bottom() + 1. + TILE_KIND),
                                );
                                (picture, name_rect, kind_rect, true)
                            };
                            if highlighted {
                                ui.painter().rect_filled(
                                    rect.shrink(1.),
                                    3.,
                                    ui.visuals().selection.bg_fill.gamma_multiply(0.5),
                                );
                            }
                            let job = crate::preset_browser::PresetBrowser::thumb_job(entry);
                            // MC-12: greyed tiles keep Modo's own picture
                            // (MC-31), dimmed (UIC-117).
                            let greyed = !entry.kind.usable();
                            match self.preset_browser.thumbs.get(&job) {
                                Some(texture) => {
                                    let t = texture.size_vec2();
                                    let fit = (size / t.x.max(t.y)).min(1e6);
                                    let shown = Rect::from_center_size(picture.center(), t * fit);
                                    ui.painter().image(
                                        texture.id(),
                                        shown,
                                        Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                                        if greyed {
                                            Color32::from_gray(255).gamma_multiply(0.35)
                                        } else {
                                            Color32::WHITE
                                        },
                                    );
                                }
                                None => {
                                    ui.painter().rect_filled(
                                        picture.shrink(size * 0.12),
                                        6.,
                                        ui.visuals().faint_bg_color,
                                    );
                                    ui.painter().text(
                                        picture.center(),
                                        Align2::CENTER_CENTER,
                                        letter(entry.kind),
                                        FontId::proportional((size * 0.3).max(12.)),
                                        ui.visuals().weak_text_color(),
                                    );
                                }
                            }
                            // A34: thumbnail request and download state.
                            self.online_tile_badge(ui, entry, picture);
                            // MC-10: Modo scenes come in as geometry only.
                            if entry.is_modo_scene() {
                                ui.painter().text(
                                    picture.left_bottom() + vec2(3., -3.),
                                    Align2::LEFT_BOTTOM,
                                    "geometry only",
                                    FontId::proportional(9.),
                                    ui.visuals().weak_text_color(),
                                );
                            }
                            // LXP-05: a "partial" badge where the look
                            // depends mainly on dropped layers; LXP-02: an
                            // info dot when anything was dropped or
                            // approximated, which shows the note.
                            if let Some(info) = self.lxp_info(entry) {
                                if info.partial {
                                    ui.painter().text(
                                        picture.left_bottom() + vec2(3., -3.),
                                        Align2::LEFT_BOTTOM,
                                        "partial",
                                        FontId::proportional(9.),
                                        ui.visuals().warn_fg_color,
                                    );
                                }
                                if info.changed_look {
                                    let dot = Rect::from_center_size(
                                        picture.right_top() + vec2(-7., 7.),
                                        vec2(12., 12.),
                                    );
                                    let hit = ui
                                        .interact(
                                            dot,
                                            ui.id().with(("lxp-note", &entry.path)),
                                            Sense::click(),
                                        )
                                        .on_hover_text(info.notes.join("\n"));
                                    ui.painter().circle_filled(
                                        dot.center(),
                                        if hit.hovered() { 4.5 } else { 3.5 },
                                        accent,
                                    );
                                    if hit.clicked() {
                                        self.status =
                                            format!("{}: {}", entry.name, info.notes.join(" · "));
                                    }
                                }
                            }
                            let name_color = if highlighted {
                                accent
                            } else if greyed {
                                ui.visuals().weak_text_color()
                            } else {
                                ui.visuals().text_color()
                            };
                            tile_text(ui, &entry.name, 11., name_color, name_rect, align);
                            tile_text(
                                ui,
                                &entry.kind_label(),
                                10.,
                                ui.visuals().weak_text_color(),
                                kind_rect,
                                align,
                            );
                            // The tip is made only while hovered (a matcap's
                            // reads its sidecar file).
                            let response = if response.hovered() {
                                response.on_hover_text(self.entry_tip(entry))
                            } else {
                                response
                            };
                            self.entry_response(&response, entry, wants, cycles);
                        }
                    });
                }
            });
    }

    /// Clicks, double-clicks, drags and the menu on one entry.
    fn entry_response(
        &mut self,
        response: &Response,
        entry: &Entry,
        wants: &mut Wants,
        cycles: bool,
    ) {
        // Pick to Add takes the entry as a button loading it (LAY-82).
        if response.clicked() && self.workbench_pick_preset(&entry.name, &entry.path) {
            return;
        }
        if response.clicked() || response.secondary_clicked() {
            self.preset_browser.highlight = Some(entry.path.clone());
            self.preset_browser.contents_focus = true;
        }
        if response.double_clicked() {
            wants.load = Some(entry.clone());
        }
        if response.drag_started() {
            self.preset_browser.highlight = Some(entry.path.clone());
            self.preset_browser.drag = Some(Drag {
                entry: entry.clone(),
            });
        }
        let read_only = self.preset_browser.read_only(&entry.path);
        response.context_menu(|ui| entry_menu(ui, entry, wants, cycles, read_only));
    }

    /// An entry's tooltip: its name and kind, then its pack's licence,
    /// credit and source when it's in a library pack (F1), or the Basics
    /// note.
    pub(super) fn entry_tip(&self, entry: &Entry) -> String {
        let mut tip = format!("{} · {}", entry.name, entry.kind_label());
        if let Some(note) = entry.unsupported_note() {
            tip += &format!("\n{note}");
        } else if let Some(Some(info)) = self.preset_browser.lxp_info.get(&entry.path) {
            // LXP-02: what the conversion dropped or approximated.
            if info.partial {
                tip += "\nPartial: some layers not supported";
            }
            for n in &info.notes {
                tip += &format!("\n{n}");
            }
        } else if entry.is_modo_scene() {
            tip += "\nGeometry only: Modo materials arrive as named, empty material groups";
        }
        // MC-11: a matcap's sidecar note.
        if entry.kind == EntryKind::Matcap
            && let Some(note) = self.preset_browser.matcap_note(&entry.path)
        {
            tip += &format!("\n{note}");
        }
        if let Some(online) = self.online_tip(entry) {
            tip += &online;
        } else if let Some((pack, item)) = self.preset_browser.pack_item(&entry.path) {
            tip += &format!("\n{}\n{}", item.info(), pack.title());
        } else if self.preset_browser.read_only(&entry.path) {
            tip += &format!(
                "\nLicence: {} · Operandi Basics (read-only)",
                operandi_core::preset::basics::LICENCE
            );
        }
        tip
    }

    /// PB-104 list: icon, name, kind and size on the shared tree list.
    fn entry_list(&mut self, ui: &mut Ui, entries: &[Entry], wants: &mut Wants) {
        let spec = TreeSpec {
            id: "preset-browser-list",
            columns: vec![
                Column::name(),
                Column::value("Type", 110., "What the file is"),
                Column::value("Size", 64., "Size on disk"),
            ],
            own_search: true,
            last_fills: false,
        };
        let mut keys: HashMap<u64, Entry> = HashMap::new();
        let rows: Vec<TreeRow<u64>> = entries
            .iter()
            .map(|e| {
                let key = key_of(&e.path);
                keys.insert(key, e.clone());
                let mut row = TreeRow::new(key, 0, e.name.clone(), 3);
                row.icon = RowIcon::Letter(letter(e.kind));
                row.cells[1] = Cell::Custom;
                row.cells[2] = Cell::Custom;
                row.draggable = false;
                row.style.dimmed = !e.kind.usable();
                row.selected = if self.preset_browser.highlight.as_ref() == Some(&e.path) {
                    Selected::Primary
                } else {
                    Selected::No
                };
                row.tip = Some(format!("{}\n{}", self.entry_tip(e), e.path.display()));
                row
            })
            .collect();
        struct ListHooks<'a> {
            keys: &'a HashMap<u64, Entry>,
            wants: &'a mut Wants,
            cycles: bool,
            basics: Option<&'a Path>,
        }
        impl TreeHooks<u64> for ListHooks<'_> {
            fn cell(&mut self, ui: &mut Ui, column: usize, key: u64, rect: Rect) {
                let Some(e) = self.keys.get(&key) else { return };
                let text = if column == 1 {
                    e.kind_label()
                } else {
                    size_label(e.size)
                };
                ui.painter().with_clip_rect(rect).text(
                    rect.left_center() + vec2(3., 0.),
                    Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(11.),
                    ui.visuals().weak_text_color(),
                );
            }
            fn has_row_menu(&self, _: u64) -> bool {
                true
            }
            fn row_menu(&mut self, ui: &mut Ui, key: u64, _rename: &mut dyn FnMut()) {
                if let Some(e) = self.keys.get(&key) {
                    let read_only = self.basics.is_some_and(|b| e.path.starts_with(b));
                    entry_menu(ui, e, self.wants, self.cycles, read_only);
                }
            }
        }
        let mut state = std::mem::take(&mut self.preset_browser.list);
        let mut editors = std::mem::take(&mut self.text_editors);
        let basics = self.preset_browser.basics.clone();
        let events = {
            let mut hooks = ListHooks {
                keys: &keys,
                wants,
                cycles: operandi_cycles::available(),
                basics: basics.as_deref(),
            };
            tree_list::show(ui, &spec, &mut state, &rows, &mut hooks, &mut editors)
        };
        // Drags out of the list (PB-110): a press that moves on a row.
        let pointer = ui
            .ctx()
            .input(|i| (i.pointer.is_decidedly_dragging(), i.pointer.press_origin()));
        if pointer.0
            && self.preset_browser.drag.is_none()
            && let Some(origin) = pointer.1
            && let Some((key, _)) = state.rects.iter().find(|(_, r)| r.contains(origin))
            && let Some(e) = keys.get(key)
        {
            self.preset_browser.drag = Some(Drag { entry: e.clone() });
        }
        self.text_editors = editors;
        self.preset_browser.list = state;
        for event in events {
            match event {
                TreeEvent::Click { key, .. } => {
                    if let Some(e) = keys.get(&key) {
                        self.preset_browser.highlight = Some(e.path.clone());
                        self.preset_browser.contents_focus = true;
                    }
                }
                TreeEvent::DoubleClick(key) => {
                    if let Some(e) = keys.get(&key) {
                        wants.load = Some(e.clone());
                    }
                }
                _ => {}
            }
        }
    }

    /// The footer (PB-102), all at the bottom right: the thumbnail size
    /// slider (tile views only), the Vertical, Horizontal and List buttons
    /// (the active one lit, UIC-22 icon buttons with tooltips, UIC-04), and
    /// the gear's options at the far right.
    fn preset_browser_footer(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.x = 4.;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.menu_button("⚙", |ui| {
                ui.label(RichText::new("Sort by").weak());
                for s in SortBy::ALL {
                    if ui.radio(self.preset_browser.sort == s, s.label()).clicked() {
                        self.preset_browser.sort = s;
                    }
                }
                ui.separator();
                let mut only = self.preset_browser.only_usable;
                if ui
                    .checkbox(&mut only, "Show only usable files")
                    .on_hover_text("Off: list every file, greyed for ones the browser can't use (operandi.pbview.onlyUsable)")
                    .changed()
                {
                    self.preset_browser.set_only_usable(only);
                }
                ui.label(RichText::new("Texture set resolution").weak());
                let choices = [
                    (ResolutionChoice::Highest, "Highest found"),
                    (ResolutionChoice::Lowest, "Lowest found"),
                    (ResolutionChoice::Set(1), "1K"),
                    (ResolutionChoice::Set(2), "2K"),
                    (ResolutionChoice::Set(4), "4K"),
                    (ResolutionChoice::Set(8), "8K"),
                ];
                for (c, label) in choices {
                    if ui.radio(self.preset_browser.resolution == c, label).clicked() {
                        self.preset_browser.resolution = c;
                    }
                }
                ui.separator();
                ui.checkbox(
                    &mut self.preset_browser.write_materialx,
                    "Also write MaterialX for material presets",
                )
                .on_hover_text("Saving a material preset writes a .mtlx file of the material beside it");
                ui.separator();
                if ui.button("Refresh").clicked() {
                    self.preset_browser.refresh(None);
                    ui.close();
                }
            })
            .response
            .on_hover_text("Options");
            // Right to left: List, Horizontal, Vertical, then the slider.
            for mode in ViewMode::ALL.into_iter().rev() {
                let on = self.preset_browser.view == mode;
                let response = tree_list::toggle_button(ui, "", on);
                let color = if on {
                    theme::chrome::active()
                } else {
                    theme::text()
                };
                paint_view_icon(ui, response.rect, mode, color);
                if response.on_hover_text(mode.label()).clicked() {
                    self.preset_browser.view = mode;
                }
            }
            if self.preset_browser.view.tiles() {
                ui.spacing_mut().slider_width = 90.;
                ui.add(
                    egui::Slider::new(
                        &mut self.preset_browser.thumb_size,
                        crate::preset_browser::THUMB_MIN..=crate::preset_browser::THUMB_MAX,
                    )
                    .show_value(false),
                )
                .on_hover_text("Thumbnail size");
            }
        });
    }

    /// PB-111: arrows move the highlight, Enter loads, typing searches.
    /// Only while the contents have the keyboard and no field is typing.
    fn preset_browser_keys(&mut self, ui: &mut Ui, wants: &mut Wants) {
        if ui.ctx().input(|i| i.pointer.any_pressed())
            && !ui
                .ctx()
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| ui.max_rect().contains(p))
        {
            self.preset_browser.contents_focus = false;
        }
        if !self.preset_browser.contents_focus || self.text_editor_focused(ui.ctx()) {
            return;
        }
        // The grid's columns as drawn this frame (it's drawn before the
        // keys are read), so Up/Down move one row in either tile view.
        let columns = if self.preset_browser.view.tiles() {
            self.preset_browser.grid_columns.max(1) as isize
        } else {
            1
        };
        let mut step = 0isize;
        let mut enter = false;
        let mut typed = String::new();
        ui.ctx().input_mut(|i| {
            for (key, d) in [
                (Key::ArrowRight, 1),
                (Key::ArrowLeft, -1),
                (Key::ArrowDown, columns),
                (Key::ArrowUp, -columns),
            ] {
                if i.consume_key(Modifiers::NONE, key) {
                    step += d;
                }
            }
            enter = i.consume_key(Modifiers::NONE, Key::Enter);
            i.events.retain(|e| match e {
                Event::Text(t) if !t.trim().is_empty() => {
                    typed.push_str(t);
                    false
                }
                _ => true,
            });
        });
        if step != 0 {
            self.move_highlight(step);
        }
        if enter && let Some(e) = self.highlighted_entry() {
            wants.load = Some(e);
        }
        if !typed.is_empty() {
            self.preset_browser.search_open = true;
            self.preset_browser.search.push_str(&typed);
            self.preset_browser.search_focus = true;
            self.preset_browser.search_changed();
        }
    }

    pub(super) fn highlighted_entry(&self) -> Option<Entry> {
        let h = self.preset_browser.highlight.as_ref()?;
        self.preset_browser
            .shown()
            .into_iter()
            .find(|e| &e.path == h)
    }

    pub(super) fn move_highlight(&mut self, step: isize) {
        let shown = self.preset_browser.shown();
        if shown.is_empty() {
            return;
        }
        let at = self
            .preset_browser
            .highlight
            .as_ref()
            .and_then(|h| shown.iter().position(|e| &e.path == h));
        let next = match at {
            None => 0,
            Some(i) => (i as isize + step).clamp(0, shown.len() as isize - 1) as usize,
        };
        self.preset_browser.highlight = Some(shown[next].path.clone());
    }

    /// While an entry is dragged: its name follows the pointer, and a
    /// material highlights the material group it would land on (PB-110);
    /// the scene doesn't change. On release over a 3D view it loads there;
    /// Esc cancels. From the OS window the drop point is found through the
    /// screen.
    fn preset_browser_drag_frame(&mut self, ctx: &Context, in_window: bool) {
        let Some(drag) = self.preset_browser.drag.clone() else {
            return;
        };
        self.preset_browser.drag_in_window = in_window;
        if in_window {
            // Keep the main window drawing the label as the pointer moves.
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        // The label follows the pointer. Out of the browser's own window it
        // is drawn by the main window instead (preset_browser_frame), so it
        // doesn't stop at the window's edge.
        if let Some(p) = ctx.pointer_interact_pos()
            && (!in_window || ctx.content_rect().contains(p))
        {
            drag_label(ctx, p, &drag.entry.name);
        }
        let pointer = if in_window {
            self.main_window_pointer(ctx)
        } else {
            ctx.pointer_interact_pos()
        };
        // Only a point over a 3D view targets anything (not one over the
        // browser itself, docked in the main window)...
        let at = pointer.filter(|p| self.corner.areas.values().any(|a| a.contains(*p)));
        // ...or a material over a shader tree group row (item 7).
        let tree_drop = pointer
            .filter(|_| crate::preset_browser::drops_material(&drag.entry))
            .and_then(|p| self.tree_drop_target(p));
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.cancel_browser_drag();
            return;
        }
        if !ctx.input(|i| i.pointer.any_released() || !i.pointer.any_down()) {
            self.update_drop_hover(at);
            self.preset_browser.tree_drop = tree_drop;
            return;
        }
        self.cancel_browser_drag();
        if let Some(group) = tree_drop {
            self.preset_browser.deferred_tree_drop = Some((drag.entry, group));
        } else if let Some(at) = at {
            self.preset_browser.drop_modifiers = ctx.input(|i| i.modifiers);
            self.preset_browser.deferred_drop = Some((drag.entry, at));
        }
    }

    /// Ends a drag out of the browser without dropping: the highlight goes
    /// and the scene is as it was.
    pub(super) fn cancel_browser_drag(&mut self) {
        self.preset_browser.drag = None;
        self.preset_browser.drop_hover = None;
        self.preset_browser.tree_drop = None;
    }

    /// Drops of Item List / Shader Tree rows onto a browser folder (PB-120)
    /// and loads deferred to the main window's frame.
    fn preset_browser_drops(&mut self, ctx: &Context) {
        if let Some((entry, at)) = self.preset_browser.deferred_drop.take() {
            self.browser_load(&entry, Target::Drop(at));
        }
        if let Some((entry, group)) = self.preset_browser.deferred_tree_drop.take() {
            self.browser_load_into_group(&entry, group);
        }
        let items = self.items_tree.dragging();
        let shading = self.shading.tree.dragging();
        if (items || shading) && ctx.input(|i| i.pointer.any_released()) {
            let at = if self.preset_browser.folder_rects_in_window {
                // The pointer in the browser window's coordinates.
                crate::os_mouse::mouse_location().and_then(|screen| {
                    let (inner, native, ppp) = self.preset_browser.window_native?;
                    Some(crate::viewport_buttons::screen_to_window(
                        screen, inner, native, ppp,
                    ))
                })
            } else {
                ctx.pointer_interact_pos()
            };
            if let Some(at) = at
                && let Some((folder, _)) = self
                    .preset_browser
                    .folder_rects
                    .iter()
                    .find(|(_, r)| r.contains(at))
                    .cloned()
            {
                let choices = self.save_choices(shading);
                if choices.is_empty() {
                    self.status = if shading {
                        "Drag a material or material group to save it".into()
                    } else {
                        "Select the items to save, then drag them".into()
                    };
                } else {
                    self.preset_browser.save_menu = Some(SaveMenu {
                        folder,
                        at,
                        choices,
                        in_window: self.preset_browser.folder_rects_in_window,
                    });
                }
            }
        }
    }

    /// PB-120's small menu: the kinds that fit, Cancel last.
    fn preset_browser_save_menu(&mut self, ctx: &Context, in_window: bool) {
        let Some(menu) = self.preset_browser.save_menu.clone() else {
            return;
        };
        // Drawn in the window that holds the folder it was dropped on.
        if menu.in_window != in_window {
            return;
        }
        let mut picked = None;
        let mut cancel = false;
        let meshes = self.selection_is_meshes();
        let show = |ctx: &Context, picked: &mut Option<PresetSource>, cancel: &mut bool| {
            let r = egui::Area::new(Id::new("preset-browser-save-menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(menu.at)
                .show(ctx, |ui| {
                    Frame::menu(ui.style()).show(ui, |ui| {
                        for &source in &menu.choices {
                            let label = match source {
                                // PG-02.
                                PresetSource::Items if meshes => "Save Mesh Preset…",
                                PresetSource::Items => "Save Item Preset…",
                                PresetSource::Assembly => "Save Assembly Preset…",
                                PresetSource::Material(_) => "Save Material Preset…",
                                PresetSource::Environment { .. } => "Save Environment Preset…",
                                PresetSource::RenderSettings => "Save Render Settings Preset…",
                            };
                            let button = ui.button(label);
                            // PG-03.
                            let button =
                                if matches!(source, PresetSource::Items | PresetSource::Assembly) {
                                    button.on_hover_text(
                                        "Saved as if the item sat at the origin with no rotation \
                                     or scale, so it lands upright (use Freeze first to keep \
                                     an off-centre build, or Centre on Ground when naming it)",
                                    )
                                } else {
                                    button
                                };
                            if button.clicked() {
                                *picked = Some(source);
                            }
                        }
                        ui.separator();
                        if ui.button("Cancel").clicked() {
                            *cancel = true;
                        }
                    });
                });
            let outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !r.response.rect.contains(p))
            });
            if outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
                *cancel = true;
            }
        };
        show(ctx, &mut picked, &mut cancel);
        if let Some(source) = picked {
            self.preset_browser.save_menu = None;
            self.ask_save_here(menu.folder, source);
        } else if cancel {
            self.preset_browser.save_menu = None;
        }
    }

    /// MC-02's small menu for a folder dropped from the Finder: Add as
    /// Location (default, first) / Copy Into Library / Cancel (last).
    fn preset_browser_folder_drop_menu(&mut self, ctx: &Context, in_window: bool) {
        let Some(drop) = self.preset_browser.folder_drop.clone() else {
            return;
        };
        if drop.in_window != in_window {
            return;
        }
        let mut pick = None;
        let r = egui::Area::new(Id::new("preset-browser-folder-drop"))
            .order(egui::Order::Foreground)
            .fixed_pos(drop.at)
            .show(ctx, |ui| {
                Frame::menu(ui.style()).show(ui, |ui| {
                    if ui
                        .button("Add as Location")
                        .on_hover_text("Browse the folder where it is; nothing is copied (pbview.basePathAdd)")
                        .clicked()
                    {
                        pick = Some(1);
                    }
                    if ui
                        .button("Copy Into Library")
                        .on_hover_text("Copy the whole folder into your library folder in the background (dircache.copy)")
                        .clicked()
                    {
                        pick = Some(2);
                    }
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        pick = Some(0);
                    }
                });
            });
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !r.response.rect.contains(p))
        });
        if outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
            pick = Some(0);
        }
        // Enter takes the default, Add as Location.
        if pick.is_none() && ctx.input(|i| i.key_pressed(Key::Enter)) {
            pick = Some(1);
        }
        match pick {
            Some(1) => {
                self.preset_browser.folder_drop = None;
                self.status = self.preset_browser.add_root(drop.folder, None);
            }
            Some(2) => {
                self.preset_browser.folder_drop = None;
                self.browser_copy_to_library(&drop.folder, false);
            }
            Some(_) => self.preset_browser.folder_drop = None,
            None => {}
        }
    }

    /// Run what the body's clicks asked for.
    fn run_browser_wants(&mut self, mut w: Wants) {
        if let Some(want) = w.online.take() {
            self.run_online_want(want);
        }
        if let Some((moved, target)) = w.reorder {
            self.preset_browser.reorder_roots(&moved, target);
        }
        if let Some((path, open)) = w.toggle {
            if open {
                self.preset_browser.expanded.insert(path);
            } else {
                self.preset_browser.expanded.remove(&path);
            }
        }
        if let Some(path) = w.show {
            self.preset_browser.show_folder(path);
        }
        if w.add_root {
            let folder = self
                .preset_browser
                .folder
                .clone()
                .unwrap_or_else(|| self.environment.paths().data_root().to_path_buf());
            if !self.choose(
                file_chooser::Choose::Folder {
                    folder,
                    title: "Add Folder to the Preset Browser",
                },
                file_chooser::Purpose::PresetBrowserRoot,
            ) {
                self.status =
                    "No file panel is available; use pbview.basePathAdd path:<folder>".into();
            }
        }
        if let Some(path) = w.remove_root {
            self.preset_browser.ask = Some(Ask::RemoveRoot(path));
        }
        if let Some(path) = w.rename_root {
            let name = self
                .preset_browser
                .roots
                .iter()
                .find(|r| r.path == path)
                .map(|r| r.name())
                .unwrap_or_default();
            let form = form::Dialog::with_defaults(
                "Rename Root",
                vec![form::Row::text("name", "Name", &name)],
            )
            .focus_name("name");
            self.preset_browser.ask = Some(Ask::RenameRoot { path, form });
        }
        if let Some(parent) = w.new_folder {
            let form = form::Dialog::with_defaults(
                "New Folder",
                vec![form::Row::text("name", "Name", "New Folder")],
            )
            .focus("name");
            self.preset_browser.ask = Some(Ask::NewFolder { parent, form });
        }
        if let Some((folder, source)) = w.save_here {
            let source = match source {
                PresetSource::Material(_) => match self.selected_material_row() {
                    Some(id) => PresetSource::Material(id),
                    None => {
                        self.status =
                            "Select a material or material group in the Shader Tree".into();
                        return;
                    }
                },
                s => s,
            };
            self.ask_save_here(folder, source);
        }
        if let Some(path) = w.reveal {
            self.browser_reveal(&path);
        }
        if let Some(path) = w.open_default {
            self.browser_open_default(&path);
        }
        if let Some(folder) = w.refresh {
            self.preset_browser.refresh(folder.as_deref());
        }
        if let Some(path) = w.rename {
            let name = self.entry_for_path(&path).name;
            let form =
                form::Dialog::with_defaults("Rename", vec![form::Row::text("name", "Name", &name)])
                    .focus_name("name");
            self.preset_browser.ask = Some(Ask::Rename { path, form });
        }
        if let Some(path) = w.duplicate {
            let name = format!("{} copy", self.entry_for_path(&path).name);
            let form = form::Dialog::with_defaults(
                "Duplicate",
                vec![form::Row::text("name", "New Name", &name)],
            )
            .focus("name");
            self.preset_browser.ask = Some(Ask::Duplicate { path, form });
        }
        if let Some(path) = w.delete {
            self.preset_browser.ask = Some(Ask::Delete(path));
        }
        if let Some(path) = w.thumb_render {
            self.thumbnail_from_last_render(&path);
        }
        if let Some(path) = w.thumb_file {
            let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
            self.preset_browser.thumb_target = Some(path);
            if !self.choose(
                file_chooser::Choose::Image { folder },
                file_chooser::Purpose::PresetBrowserThumbnail,
            ) {
                self.status = "No file panel is available".into();
            }
        }
        if let Some(path) = w.thumb_refresh {
            self.preset_browser.thumbs.forget(&path, true);
        }
        if let Some(entry) = w.save_texture_set {
            self.save_texture_set_as_preset(&entry);
        }
        if w.get_library
            && let Err(e) = self.browser_get_library(crate::preset_browser::LIBRARY_URL)
        {
            self.status = e;
        }
        if w.locate_library {
            self.browser_locate_library();
        }
        if let Some(path) = w.open {
            self.execute(&format!("scene.open filename:\"{}\"", path.display()));
        }
        if let Some(path) = w.import {
            self.execute(&format!(
                "scene.importReference filename:\"{}\"",
                path.display()
            ));
        }
        if let Some(path) = w.matcap_all {
            self.use_matcap(&path, None, true);
        }
        if let Some(entry) = w.load {
            self.browser_load(&entry, Target::Default);
        }
    }

    /// CD-26: a texture set saved as a material preset in the shown folder
    /// (or the Presets folder when the shown one is a content folder the
    /// user didn't save into before; provisional PB1-Q06: the shown folder).
    fn save_texture_set_as_preset(&mut self, entry: &Entry) {
        let Some(set) = &entry.texture_set else {
            return;
        };
        let (preset, notes) = match set.material_preset(self.preset_browser.resolution, &|p| {
            image::image_dimensions(p).ok()
        }) {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let folder = entry
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let path = operandi_core::preset::library::free_path(
            &folder,
            &set.name,
            operandi_core::preset::PRESET_EXTENSION,
        );
        match operandi_core::persistence::save_preset(&path, &preset) {
            Ok(()) => {
                self.status = std::iter::once(format!("Saved {} as a material preset", set.name))
                    .chain(notes)
                    .collect::<Vec<_>>()
                    .join(" · ");
                self.preset_browser.refresh(Some(&folder));
                self.preset_browser.highlight = Some(path.clone());
                self.preset_browser
                    .thumbs
                    .render_into_file(&path, PresetKind::Material);
            }
            Err(e) => self.status = format!("Save {}: {e}", set.name),
        }
    }

    /// Replace thumbnail with last render (PB-112): the newest render in
    /// the render history, cropped square. A preset keeps it in its file;
    /// anything else in the thumbnail cache.
    fn thumbnail_from_last_render(&mut self, path: &Path) {
        let history = &self.render.history;
        let Some(slot) = history
            .slots
            .iter()
            .find(|s| s.kind == crate::image_viewer::SlotKind::Render)
        else {
            self.status = "Render something first: the render history is empty".into();
            return;
        };
        let file = slot.thumb_file(&history.folder);
        let picture = crate::image_decode::open(&file).ok().or_else(|| {
            let pixels = crate::image_viewer::read_output(&history.folder, slot, 0).ok()?;
            let bytes: Vec<u8> = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| {
                    let c = |v: f32| {
                        (crate::image_viewer::linear_to_srgb(v).clamp(0., 1.) * 255. + 0.5) as u8
                    };
                    [c(p[0]), c(p[1]), c(p[2]), 255]
                })
                .collect();
            image::RgbaImage::from_raw(slot.width, slot.height, bytes)
                .map(image::DynamicImage::ImageRgba8)
        });
        let Some(picture) = picture else {
            self.status = "Can't read the last render".into();
            return;
        };
        self.set_entry_thumbnail(path, picture);
    }

    /// Store `picture` (cropped square, small) as `path`'s thumbnail.
    pub(super) fn set_entry_thumbnail(&mut self, path: &Path, picture: image::DynamicImage) {
        let side = picture.width().min(picture.height());
        let square = picture.crop_imm(
            (picture.width() - side) / 2,
            (picture.height() - side) / 2,
            side,
            side,
        );
        let small = square.thumbnail(crate::preset_thumbs::PIXELS, crate::preset_thumbs::PIXELS);
        let mut png = std::io::Cursor::new(Vec::new());
        if small.write_to(&mut png, image::ImageFormat::Png).is_err() {
            self.status = "Can't make the thumbnail".into();
            return;
        }
        let png = png.into_inner();
        // A read-only Basics preset keeps its picture in the cache instead.
        let is_preset = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(operandi_core::preset::PRESET_EXTENSION))
            && !self.preset_browser.read_only(path);
        let result = if is_preset {
            operandi_core::persistence::set_preset_thumbnail(path, Some(&png))
                .map_err(|e| e.to_string())
        } else {
            self.preset_browser
                .thumbs
                .store_rendered(path, &png)
                .map_err(|e| e.to_string())
        };
        match result {
            Ok(()) => {
                self.preset_browser.thumbs.forget(path, false);
                self.status = "Thumbnail replaced".into();
            }
            Err(e) => self.status = format!("Can't store the thumbnail: {e}"),
        }
    }

    /// The browser's questions (PB-13, PB-112, PB-121, PB-122, CD-02).
    fn preset_browser_questions(&mut self, ctx: &Context) {
        let Some(ask) = self.preset_browser.ask.take() else {
            return;
        };
        use crate::form::Outcome;
        let keep = match ask {
            Ask::RemoveRoot(path) => {
                let message = format!(
                    "Take {} out of the Preset Browser? Its files stay on disk.",
                    path.display()
                );
                match self.show_confirm(ctx, "Remove Root", &message, &[]) {
                    Outcome::Ok => {
                        self.preset_browser.remove_root(&path);
                        None
                    }
                    Outcome::Open => Some(Ask::RemoveRoot(path)),
                    _ => None,
                }
            }
            Ask::CopyOverwrite(folder) => {
                let name = crate::preset_browser::folder_name(&folder);
                let message = format!(
                    "{name} is already in the library. Copy over it? Files of the same name are replaced."
                );
                match self.show_confirm(ctx, "Copy Into Library", &message, &[]) {
                    Outcome::Ok => {
                        self.browser_copy_to_library(&folder, true);
                        None
                    }
                    Outcome::Open => Some(Ask::CopyOverwrite(folder)),
                    _ => {
                        self.status = "Nothing copied".into();
                        None
                    }
                }
            }
            Ask::Delete(path) => {
                let name = self.entry_for_path(&path).name;
                let message = format!("Delete {name} from disk? This can't be undone.");
                match self.show_confirm(ctx, "Delete", &message, &[]) {
                    Outcome::Ok => {
                        self.browser_delete(&path);
                        None
                    }
                    Outcome::Open => Some(Ask::Delete(path)),
                    _ => {
                        self.status = format!("{name} kept");
                        None
                    }
                }
            }
            Ask::Clash { path, source } => {
                let name = crate::preset_browser::folder_name(&path);
                let message = format!(
                    "{name} already exists. OK replaces it; Keep Both saves the new one with a number."
                );
                match self.show_confirm(ctx, "Replace Preset", &message, &["Keep Both"]) {
                    Outcome::Ok => {
                        self.browser_save(path, source, true);
                        None
                    }
                    Outcome::Extra(0) => {
                        let stem = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
                        let free = operandi_core::preset::library::free_path(
                            &folder,
                            &stem,
                            operandi_core::preset::PRESET_EXTENSION,
                        );
                        self.browser_save(free, source, false);
                        None
                    }
                    Outcome::Open => Some(Ask::Clash { path, source }),
                    _ => {
                        self.status = "Nothing saved".into();
                        None
                    }
                }
            }
            Ask::SaveName {
                folder,
                source,
                mut form,
            } => {
                let name = form.values.text("name").trim().to_owned();
                form.block_ok(name.is_empty().then(|| "Type a name first".to_owned()));
                match self.show_form(ctx, &mut form) {
                    Outcome::Ok => {
                        let clean: String = name
                            .chars()
                            .map(|c| {
                                if matches!(c, '/' | '\\' | ':') {
                                    '-'
                                } else {
                                    c
                                }
                            })
                            .collect();
                        let path = folder.join(format!(
                            "{clean}.{}",
                            operandi_core::preset::extension_for(source.kind())
                        ));
                        self.preset_browser.save_centered = form
                            .values
                            .get(crate::preset_browser::CENTER_ON_GROUND)
                            .is_some_and(|v| *v == form::Value::Bool(true));
                        self.browser_save(path, source, false);
                        // A clash question replaces this one.
                        self.preset_browser.ask.take()
                    }
                    Outcome::Open => Some(Ask::SaveName {
                        folder,
                        source,
                        form,
                    }),
                    _ => None,
                }
            }
            Ask::Rename { path, mut form } => match self.show_form(ctx, &mut form) {
                Outcome::Ok => {
                    let name = form.values.text("name").to_owned();
                    if self.browser_rename(&path, &name) {
                        None
                    } else {
                        Some(Ask::Rename { path, form })
                    }
                }
                Outcome::Open => Some(Ask::Rename { path, form }),
                _ => None,
            },
            Ask::Duplicate { path, mut form } => match self.show_form(ctx, &mut form) {
                Outcome::Ok => {
                    let name = form.values.text("name").to_owned();
                    if self.browser_duplicate(&path, &name) {
                        None
                    } else {
                        Some(Ask::Duplicate { path, form })
                    }
                }
                Outcome::Open => Some(Ask::Duplicate { path, form }),
                _ => None,
            },
            Ask::RenameRoot { path, mut form } => match self.show_form(ctx, &mut form) {
                Outcome::Ok => {
                    let name = form.values.text("name").trim().to_owned();
                    if let Some(root) = self
                        .preset_browser
                        .roots
                        .iter_mut()
                        .find(|r| r.path == path)
                    {
                        root.label = (!name.is_empty()
                            && name != crate::preset_browser::folder_name(&path))
                        .then_some(name);
                    }
                    None
                }
                Outcome::Open => Some(Ask::RenameRoot { path, form }),
                _ => None,
            },
            Ask::NewFolder { parent, mut form } => match self.show_form(ctx, &mut form) {
                Outcome::Ok => {
                    let name = form.values.text("name").trim().to_owned();
                    let path = parent.join(&name);
                    if self.preset_browser.read_only(&parent) {
                        self.status = READ_ONLY_NOTE.into();
                        None
                    } else if name.is_empty() || name.contains(['/', '\\', ':']) {
                        self.status = "Type a folder name without / \\ or :".into();
                        Some(Ask::NewFolder { parent, form })
                    } else if let Err(e) = std::fs::create_dir(&path) {
                        self.status = format!("Can't make {name}: {e}");
                        Some(Ask::NewFolder { parent, form })
                    } else {
                        self.preset_browser.expanded.insert(parent.clone());
                        self.preset_browser.refresh(Some(&parent));
                        self.preset_browser.show_folder(path);
                        None
                    }
                }
                Outcome::Open => Some(Ask::NewFolder { parent, form }),
                _ => None,
            },
        };
        if self.preset_browser.ask.is_none() {
            self.preset_browser.ask = keep;
        }
    }
}

/// F1's offer while no library pack is installed: a short note and two
/// buttons, above the folder's contents.
fn library_offer(ui: &mut Ui, wants: &mut Wants) {
    ui.horizontal_wrapped(|ui| {
        ui.weak("More materials:");
        if ui
            .button("Get the free Operandi Library…")
            .on_hover_text(format!(
                "Opens {} in your web browser; unzip the download into your library folder",
                crate::preset_browser::LIBRARY_URL
            ))
            .clicked()
        {
            wants.get_library = true;
        }
        if ui
            .button("Locate Library Folder…")
            .on_hover_text("Choose an unzipped Operandi Library folder to add it here")
            .clicked()
        {
            wants.locate_library = true;
        }
    });
    ui.add_space(4.);
}

/// Whether `folder` has any subfolder (for its disclosure arrow), without
/// keeping the list.
fn library_has_subfolders(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| !operandi_core::preset::library::is_hidden(&e.file_name().to_string_lossy()))
        .find(|e| e.path().is_dir())
        .map(|e| e.path())
        .into_iter()
        .collect()
}
