//! The Add browser (specification add-browser-2026-11-17, AB-xx): one
//! widget for every "Add …" picker, opened as a pinnable pop-over by the
//! button that asked for it (AB-01, AB-10). Add Operator, Add Selection and
//! Add Item use it, each with its own roots and remembered state (AB-24).
//!
//! The model ([`Catalog`], [`State`]) is plain data so tests drive it
//! without a window; [`show`] draws it. Layout, top to bottom (AB-11):
//! navigation bar (Back, Forward, Up, +, filter toggle), search row (Search
//! All / Search View and the field), the Directories tree and the grouped
//! Name list split at 36 %, and the bottom bar (Directories toggle, size
//! slider, view modes).
//!
//! Provisional (AB-Q1..Q3, MOQ-29): entries have no icons yet (the icon
//! generator has no recipes for ops), so rows and tiles show names only;
//! "+" and "(add path…)" are greyed because these lists are built in, not
//! folders on disk; the filter toggle shows or hides the search row; the
//! four view modes are list, tiles, small grid and large grid.
use crate::unavailable::Gate;
use eframe::egui::{self, Id, Pos2, Rect, Sense, Ui, vec2};
use std::collections::BTreeSet;

/// One thing the browser can add.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    /// What the caller gets back when it's added (an op type id, an item
    /// kind key ...).
    pub id: String,
    pub label: String,
    /// Its folder, root first: `["Mesh Operations", "Create", "Primitives"]`.
    pub folder: Vec<String>,
    /// Its icon (a generator icon; `None` = the default cube).
    pub icon: Option<crate::icons::Icon>,
}

/// The browser's contents for one entry point: entries and the roots in
/// order (AB-01). Folders come from the entries' paths; a folder with no
/// entries under it is hidden (AB-20, AB-41).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Catalog {
    pub roots: Vec<String>,
    pub entries: Vec<Entry>,
    /// A plain filtered list: no Directories panel, no navigation bar
    /// (AB-01, Add Selection Operation).
    pub plain: bool,
}

impl Catalog {
    /// Every folder that holds entries (itself or below), in tree order:
    /// roots in `roots` order, subfolders alphabetically.
    pub(crate) fn folders(&self) -> Vec<Vec<String>> {
        let mut all: BTreeSet<Vec<String>> = BTreeSet::new();
        for e in &self.entries {
            for n in 1..=e.folder.len() {
                all.insert(e.folder[..n].to_vec());
            }
        }
        let mut out: Vec<Vec<String>> = Vec::new();
        for root in &self.roots {
            out.extend(all.iter().filter(|f| &f[0] == root).cloned());
        }
        out
    }
    /// The groups the Name list shows for `folder` (AB-20): that folder's
    /// entries and every subfolder's, each folder its own group, in tree
    /// order, entries alphabetical (AB-21), empty groups left out. With a
    /// search (AB-23) only matching names, under `folder` (Search View) or
    /// everywhere (Search All).
    pub(crate) fn groups(
        &self,
        folder: &[String],
        search: &str,
        all: bool,
    ) -> Vec<(Vec<String>, Vec<&Entry>)> {
        let needle = search.trim().to_lowercase();
        let under = |f: &[String]| all && !needle.is_empty() || f.starts_with(folder);
        let mut out = Vec::new();
        for f in self.folders() {
            if !under(&f) {
                continue;
            }
            let mut entries: Vec<&Entry> = self
                .entries
                .iter()
                .filter(|e| e.folder == f)
                .filter(|e| needle.is_empty() || e.label.to_lowercase().contains(&needle))
                .collect();
            if entries.is_empty() {
                continue;
            }
            entries.sort_by_key(|e| e.label.to_lowercase());
            out.push((f, entries));
        }
        out
    }
}

/// AB-30 view modes; List is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ViewMode {
    LargeGrid,
    SmallGrid,
    #[default]
    List,
    Tiles,
}
impl ViewMode {
    const ALL: [(ViewMode, &'static str, &'static str); 4] = [
        (ViewMode::LargeGrid, "▦", "Large thumbnails"),
        (ViewMode::SmallGrid, "▤", "Small thumbnails"),
        (ViewMode::List, "☰", "List"),
        (ViewMode::Tiles, "▭", "Tiles"),
    ];
}

/// One entry point's remembered state (AB-24).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct State {
    pub open: bool,
    pub pinned: bool,
    /// The selected folder, root first. Empty = the first root.
    pub folder: Vec<String>,
    /// Folder history for Back and Forward (AB-25).
    pub history: Vec<Vec<String>>,
    pub at: usize,
    /// Directory rows the user folded.
    pub folded_dirs: BTreeSet<Vec<String>>,
    /// Name-list groups the user folded.
    pub folded_groups: BTreeSet<Vec<String>>,
    pub search: String,
    pub search_all: bool,
    /// The filter toggle: the search row shows.
    pub filter: bool,
    pub recent: Vec<String>,
    pub view: ViewMode,
    /// 0..100, starts at 70 (AB-31).
    pub size: f32,
    /// The Directories / Name divider, as a fraction (AB-11: 36 %).
    pub divider: f32,
    pub dirs: bool,
    pub selected: Option<String>,
    pub window: egui::Vec2,
    /// Where the user dragged it (top-left); `None` = beside its button.
    pub place: Option<Pos2>,
    /// It was drawn open last frame, so a frame drawn open without it is
    /// the one it opened on.
    shown: bool,
    /// The search field takes the keyboard on the next draw.
    focus_search: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            pinned: false,
            folder: Vec::new(),
            history: Vec::new(),
            at: 0,
            folded_dirs: BTreeSet::new(),
            folded_groups: BTreeSet::new(),
            search: String::new(),
            search_all: true,
            filter: true,
            recent: Vec::new(),
            view: ViewMode::List,
            size: 70.,
            divider: 0.36,
            dirs: true,
            selected: None,
            window: vec2(450., 600.),
            place: None,
            shown: false,
            focus_search: false,
        }
    }
}

impl State {
    /// The selected folder, defaulting to the first root (AB-24).
    pub(crate) fn current(&self, c: &Catalog) -> Vec<String> {
        if self.folder.is_empty() {
            c.roots.first().map(|r| vec![r.clone()]).unwrap_or_default()
        } else {
            self.folder.clone()
        }
    }
    /// Selects `folder`, recording it for Back (AB-25).
    pub(crate) fn go(&mut self, c: &Catalog, folder: Vec<String>) {
        let now = self.current(c);
        if now == folder {
            return;
        }
        if self.history.is_empty() {
            self.history.push(now);
        }
        self.history.truncate(self.at + 1);
        self.history.push(folder.clone());
        self.at = self.history.len() - 1;
        self.folder = folder;
    }
    pub(crate) fn can_back(&self) -> bool {
        self.at > 0
    }
    pub(crate) fn can_forward(&self) -> bool {
        self.at + 1 < self.history.len()
    }
    pub(crate) fn back(&mut self) {
        if self.can_back() {
            self.at -= 1;
            self.folder = self.history[self.at].clone();
        }
    }
    pub(crate) fn forward(&mut self) {
        if self.can_forward() {
            self.at += 1;
            self.folder = self.history[self.at].clone();
        }
    }
    pub(crate) fn up(&mut self, c: &Catalog) {
        let now = self.current(c);
        if now.len() > 1 {
            self.go(c, now[..now.len() - 1].to_vec());
        }
    }
    /// The browser opened: the search row shows with the keyboard in its
    /// field, so typing goes straight into the search (AB-11.2). The filter
    /// toggle still hides it afterwards.
    pub(crate) fn opened(&mut self) {
        self.filter = true;
        self.focus_search = true;
    }
    /// Closing clears the search (AB-23).
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.shown = false;
        self.focus_search = false;
        let s = std::mem::take(&mut self.search);
        if !s.trim().is_empty() {
            self.recent.retain(|r| r != &s);
            self.recent.insert(0, s);
            self.recent.truncate(8);
        }
    }
    /// The row height and tile size at the current slider value (AB-31:
    /// about 36 px rows at 70).
    fn row_height(&self) -> f32 {
        16. + self.size * 0.28
    }
    fn tile(&self) -> f32 {
        match self.view {
            ViewMode::LargeGrid => 64. + self.size * 1.4,
            _ => 40. + self.size * 0.6,
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Where each directory row and Name-list row was drawn last frame,
    /// by its text (tests check the layout).
    pub(crate) static RECTS: std::cell::RefCell<Vec<(String, Rect)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(test)]
fn record(text: &str, rect: Rect) {
    RECTS.with(|r| r.borrow_mut().push((text.to_owned(), rect)));
}
#[cfg(not(test))]
fn record(_: &str, _: Rect) {}

/// What happened this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outcome {
    /// An entry was added (double-click or Enter, AB-22).
    pub added: Option<String>,
}

fn path_text(f: &[String]) -> String {
    f.join("/")
}

/// Draws the browser for `catalog` next to `anchor` while `state.open`.
/// `title` is the window title (AB-01).
pub(crate) fn show(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    anchor: Rect,
    catalog: &Catalog,
    state: &mut State,
) -> Outcome {
    let mut out = Outcome::default();
    if !state.open {
        state.shown = false;
        return out;
    }
    if !state.shown {
        state.shown = true;
        state.opened();
    }
    // AB-10: to the side of the button, kept on screen.
    let screen = ctx.content_rect();
    let size = state.window.max(vec2(250., 150.)).min(screen.size());
    let mut pos = Pos2::new(anchor.right() + 4., anchor.top());
    if pos.x + size.x > screen.right() {
        pos.x = (anchor.left() - size.x - 4.).max(screen.left());
    }
    pos.y = pos.y.min(screen.bottom() - size.y).max(screen.top());
    let pos = state.place.unwrap_or(pos);
    let mut title_action = crate::popup_title::TitleAction::None;
    let area = egui::Area::new(Id::new(("add-browser", id)))
        .order(egui::Order::Foreground)
        .constrain(true)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            let frame = egui::Frame::popup(ui.style()).fill(crate::theme::popup_fill());
            let margin = frame.total_margin().sum();
            frame.show(ui, |ui| {
                // A fixed 450×600 (AB-10), whatever the content asks for.
                let inner = size - margin;
                ui.set_min_size(inner);
                ui.set_max_size(inner);
                ui.set_width(inner.x);
                ui.set_height(inner.y);
                // Nothing draws outside the window.
                ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                // The shared popup title bar: pin circle, drag to move.
                title_action = crate::popup_title::title_bar(ui, title, state.pinned);
                body(ui, catalog, state, &mut out);
            });
        });
    let rect = area.response.rect;
    record("window", rect);
    match title_action {
        // The circle pins it where it is; pinned, it closes it.
        crate::popup_title::TitleAction::Circle if state.pinned => {
            state.close();
            return out;
        }
        crate::popup_title::TitleAction::Circle => {
            state.pinned = true;
            state.place = Some(rect.min);
        }
        crate::popup_title::TitleAction::Drag(delta) => {
            // #60: a drag pins it too, so it stays where it is dropped
            // (UIC-185: only a pinned popover moves).
            state.pinned = true;
            state.place = Some(state.place.unwrap_or(rect.min) + delta);
        }
        crate::popup_title::TitleAction::None => {}
    }
    let (click, escape) = ctx.input(|i| {
        (
            i.pointer
                .any_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten(),
            i.key_pressed(egui::Key::Escape),
        )
    });
    // AB-10: unpinned, it closes when focus leaves it.
    let outside = click.is_some_and(|p| !rect.contains(p) && !anchor.contains(p));
    if escape || (outside && !state.pinned) {
        state.close();
    }
    if out.added.is_some() && !state.pinned {
        state.close();
    }
    out
}

fn body(ui: &mut Ui, c: &Catalog, s: &mut State, out: &mut Outcome) {
    if !c.plain {
        // AB-11.1 navigation bar.
        ui.horizontal(|ui| {
            if ui
                .gated(
                    (!s.can_back()).then_some("No earlier folder"),
                    egui::Button::new("◀"),
                )
                .on_hover_text("Back")
                .clicked()
            {
                s.back();
            }
            if ui
                .gated(
                    (!s.can_forward()).then_some("No later folder"),
                    egui::Button::new("▶"),
                )
                .on_hover_text("Forward")
                .clicked()
            {
                s.forward();
            }
            let up = s.current(c).len() > 1;
            if ui
                .gated(
                    (!up).then_some("Already at the top"),
                    egui::Button::new("⏶"),
                )
                .on_hover_text("Up to the parent folder")
                .clicked()
            {
                s.up(c);
            }
            ui.gated(
                Some("Adding a folder path waits for folders on disk: these lists are built in"),
                egui::Button::new("+"),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let f = ui
                    .selectable_label(s.filter, "⏷")
                    .on_hover_text("Show the search row");
                if f.clicked() {
                    s.filter ^= true;
                    s.focus_search = s.filter;
                }
            });
        });
    }
    // AB-11.2 search row.
    if s.filter || c.plain {
        ui.horizontal(|ui| {
            if !c.plain {
                let label = if s.search_all {
                    "Search All"
                } else {
                    "Search View"
                };
                if ui
                    .button(label)
                    .on_hover_text("Search every folder, or only the selected one")
                    .clicked()
                {
                    s.search_all ^= true;
                }
            }
            let field = ui.add(
                egui::TextEdit::singleline(&mut s.search)
                    .hint_text("Search")
                    .desired_width(ui.available_width() - 28.),
            );
            if !s.recent.is_empty() {
                ui.menu_button("⏷", |ui| {
                    for r in s.recent.clone() {
                        if ui.button(&r).clicked() {
                            s.search = r;
                            ui.close();
                        }
                    }
                });
            }
            if std::mem::take(&mut s.focus_search) {
                field.request_focus();
            }
            record("search", field.rect);
            crate::numeric_tab::register(ui, &field, Some(&s.search));
        });
    }
    ui.separator();
    // AB-11.3 body.
    // The bottom bar and its separator.
    let bottom = ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 3. + 6.;
    let body_h = (ui.available_height() - bottom).max(60.);
    let width = ui.available_width();
    let folder = s.current(c);
    ui.horizontal_top(|ui| {
        let dirs_w = if s.dirs && !c.plain {
            (width * s.divider).clamp(0., width - 80.)
        } else {
            0.
        };
        if dirs_w > 8. {
            // Each column lays out top to bottom (the row around them is
            // horizontal; a child would otherwise inherit that).
            ui.allocate_ui_with_layout(
                vec2(dirs_w, body_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_size(vec2(dirs_w, body_h));
                    ui.set_max_width(dirs_w);
                    pane(ui, "Directories", |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("dirs")
                            .auto_shrink([false, false])
                            .show(ui, |ui| directories(ui, c, s));
                    });
                },
            );
            // Draggable divider (AB-11); fully left hides the tree.
            let (r, resp) = ui.allocate_exact_size(vec2(4., body_h), Sense::drag());
            ui.painter().vline(
                r.center().x,
                r.y_range(),
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if resp.dragged() {
                let x = resp
                    .interact_pointer_pos()
                    .map_or(0., |p| p.x - ui.min_rect().left());
                s.divider = (x / width).clamp(0., 0.8);
                if s.divider < 0.05 {
                    s.dirs = false;
                    s.divider = 0.36;
                }
            }
            resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        }
        let names_w = ui.available_width();
        ui.allocate_ui_with_layout(
            vec2(names_w, body_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_size(vec2(names_w, body_h));
                ui.set_max_width(names_w);
                pane(ui, "Name", |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("names")
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                        )
                        .show(ui, |ui| names(ui, c, s, &folder, out));
                });
            },
        );
    });
    // AB-11.4 bottom bar.
    ui.separator();
    ui.horizontal(|ui| {
        if !c.plain
            && ui
                .selectable_label(s.dirs, "▥")
                .on_hover_text("Show or hide Directories")
                .clicked()
        {
            s.dirs ^= true;
        }
        ui.add(egui::Slider::new(&mut s.size, 0.0..=100.0).show_value(false))
            .on_hover_text("Icon and row size");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (mode, glyph, tip) in ViewMode::ALL.iter().rev() {
                if ui
                    .selectable_label(s.view == *mode, *glyph)
                    .on_hover_text(*tip)
                    .clicked()
                {
                    s.view = *mode;
                }
            }
        });
    });
    // Enter on a selected row adds it (AB-22).
    if ui.input(|i| i.key_pressed(egui::Key::Enter))
        && let Some(sel) = s.selected.clone()
    {
        out.added = Some(sel);
    }
}

/// A sunken pane with a border and a column header bar (Directories,
/// Name), filling the space it's given.
fn pane(ui: &mut Ui, header: &str, add: impl FnOnce(&mut Ui)) {
    egui::Frame::canvas(ui.style())
        .inner_margin(egui::Margin::same(0))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.), Sense::hover());
            let v = ui.visuals();
            ui.painter()
                .rect_filled(bar, 0., v.widgets.inactive.bg_fill);
            ui.painter().hline(
                bar.x_range(),
                bar.bottom(),
                v.widgets.noninteractive.bg_stroke,
            );
            ui.painter().text(
                bar.left_center() + vec2(6., 0.),
                egui::Align2::LEFT_CENTER,
                header,
                egui::FontId::proportional(12.),
                v.weak_text_color(),
            );
            add(ui);
        });
}

/// One painted row: zebra background, selection, icon and a left-aligned
/// label after `indent`. Clickable.
#[allow(clippy::too_many_arguments)]
fn row(
    ui: &mut Ui,
    n: usize,
    height: f32,
    indent: f32,
    selected: bool,
    icon: crate::icons::Icon,
    text: &str,
    strong: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let v = ui.visuals();
    if selected {
        ui.painter().rect_filled(rect, 0., v.selection.bg_fill);
    } else if n % 2 == 1 {
        ui.painter().rect_filled(rect, 0., v.faint_bg_color);
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 0., v.widgets.hovered.weak_bg_fill);
    }
    let icon_size = (height - 4.).min(32.);
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + indent, rect.center().y - icon_size / 2.),
        vec2(icon_size, icon_size),
    );
    crate::icons::paint(ui, icon_rect, icon);
    let color = if selected {
        v.selection.stroke.color
    } else {
        v.text_color()
    };
    let font = if strong {
        egui::FontId::proportional(13.)
    } else {
        egui::FontId::proportional(12.5)
    };
    let left = icon_rect.right() + 6.;
    let (galley, cut) = one_line(ui, text, font, color, rect.right() - left - 4.);
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.galley(
        Pos2::new(left, rect.center().y - galley.size().y / 2.),
        galley,
        color,
    );
    // The full name when it didn't fit.
    if cut { resp.on_hover_text(text) } else { resp }
}

/// `text` on one line, cut with "…" to `width`; whether it was cut.
fn one_line(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    width: f32,
) -> (std::sync::Arc<egui::Galley>, bool) {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(8.),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    let cut = galley.text() != text;
    (galley, cut)
}
fn entry_icon(e: &Entry) -> crate::icons::Icon {
    // Jason 2026-11-17: the default cube, as most ops show elsewhere.
    e.icon.unwrap_or(crate::icons::Icon::Cube)
}

fn directories(ui: &mut Ui, c: &Catalog, s: &mut State) {
    ui.spacing_mut().item_spacing.y = 0.;
    let current = s.current(c);
    let folders = c.folders();
    let height = 24.;
    let mut n = 0;
    for f in &folders {
        // Hidden under a folded parent.
        if (1..f.len()).any(|n| s.folded_dirs.contains(&f[..n])) {
            continue;
        }
        let has_children = folders
            .iter()
            .any(|g| g.len() > f.len() && g.starts_with(f));
        let indent = 16. + 14. * (f.len() - 1) as f32;
        let name = f.last().cloned().unwrap_or_default();
        let resp = row(
            ui,
            n,
            height,
            indent,
            *f == current,
            crate::icons::Icon::TreeFolder,
            &name,
            false,
        );
        n += 1;
        record(&format!("dir:{}", path_text(f)), resp.rect);
        // The disclosure triangle, left of the folder icon.
        let tri = Rect::from_center_size(
            Pos2::new(resp.rect.left() + indent - 8., resp.rect.center().y),
            vec2(10., 10.),
        );
        if has_children {
            let folded = s.folded_dirs.contains(f);
            crate::icons::disclosure(ui, tri, if folded { 0. } else { 1. });
        }
        if resp.clicked() {
            let on_triangle = resp
                .interact_pointer_pos()
                .is_some_and(|p| tri.expand(3.).contains(p));
            if has_children && on_triangle {
                if !s.folded_dirs.remove(f) {
                    s.folded_dirs.insert(f.clone());
                }
            } else {
                s.go(c, f.clone());
            }
        }
    }
    ui.add_space(2.);
    ui.gated(
        Some("These lists are built in; folders from disk come later"),
        egui::Label::new(egui::RichText::new("   (add path…)").weak()),
    );
}

fn names(ui: &mut Ui, c: &Catalog, s: &mut State, folder: &[String], out: &mut Outcome) {
    ui.spacing_mut().item_spacing.y = 0.;
    let groups = c.groups(folder, &s.search, s.search_all);
    if groups.is_empty() {
        ui.weak(if s.search.trim().is_empty() {
            "Nothing here yet"
        } else {
            "No matches"
        });
        return;
    }
    let row_h = s.row_height();
    let line = ui.visuals().widgets.noninteractive.bg_stroke;
    let mut n = 0;
    for (f, entries) in groups {
        let folded = s.folded_groups.contains(&f);
        // Group header: folder icon + path relative to the root (AB-13).
        let header = row(
            ui,
            n,
            row_h,
            18.,
            false,
            crate::icons::Icon::TreeFolder,
            &path_text(&f),
            true,
        );
        n += 1;
        record(&format!("group:{}", path_text(&f)), header.rect);
        let tri = Rect::from_center_size(
            Pos2::new(header.rect.left() + 9., header.rect.center().y),
            vec2(10., 10.),
        );
        crate::icons::disclosure(ui, tri, if folded { 0. } else { 1. });
        if header.clicked() && !s.folded_groups.remove(&f) {
            s.folded_groups.insert(f.clone());
        }
        if folded {
            continue;
        }
        // (id, double-clicked) of the row pressed this frame.
        let mut pressed: Option<(String, bool)> = None;
        let selected_id = s.selected.clone();
        // Tree lines run from the header's folder icon down to each entry.
        let stem_x = header.rect.left() + 18. + (row_h - 4.).min(32.) / 2.;
        let mut last_y = header.rect.bottom();
        match s.view {
            ViewMode::List => {
                for e in entries {
                    let selected = selected_id.as_deref() == Some(e.id.as_str());
                    let resp = row(
                        ui,
                        n,
                        row_h,
                        40. + 18.,
                        selected,
                        entry_icon(e),
                        &e.label,
                        false,
                    );
                    n += 1;
                    record(&format!("entry:{}", e.label), resp.rect);
                    let y = resp.rect.center().y;
                    ui.painter().hline(stem_x..=resp.rect.left() + 54., y, line);
                    last_y = y;
                    if resp.clicked() || resp.double_clicked() {
                        pressed = Some((e.id.clone(), resp.double_clicked()));
                    }
                }
                ui.painter()
                    .vline(stem_x, header.rect.bottom()..=last_y, line);
            }
            ViewMode::Tiles | ViewMode::SmallGrid | ViewMode::LargeGrid => {
                let tile = s.tile();
                let wide = s.view == ViewMode::Tiles;
                ui.horizontal_wrapped(|ui| {
                    for e in entries {
                        let selected = selected_id.as_deref() == Some(e.id.as_str());
                        let size = if wide {
                            vec2(tile * 2.5, row_h.max(28.))
                        } else {
                            vec2(tile, tile + 18.)
                        };
                        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                        let v = ui.visuals();
                        if selected {
                            ui.painter().rect_filled(rect, 3., v.selection.bg_fill);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(rect, 3., v.faint_bg_color);
                        }
                        let (icon_rect, text_at, align) = if wide {
                            let s = (size.y - 4.).min(32.);
                            (
                                Rect::from_min_size(
                                    rect.left_top() + vec2(4., (size.y - s) / 2.),
                                    vec2(s, s),
                                ),
                                Pos2::new(rect.left() + s + 10., rect.center().y),
                                egui::Align2::LEFT_CENTER,
                            )
                        } else {
                            let s = tile - 12.;
                            (
                                Rect::from_min_size(rect.left_top() + vec2(6., 4.), vec2(s, s)),
                                Pos2::new(rect.center().x, rect.bottom() - 9.),
                                egui::Align2::CENTER_CENTER,
                            )
                        };
                        crate::icons::paint(ui, icon_rect, entry_icon(e));
                        let room = if wide {
                            rect.right() - text_at.x - 4.
                        } else {
                            rect.width() - 6.
                        };
                        let color = v.text_color();
                        let (galley, _) =
                            one_line(ui, &e.label, egui::FontId::proportional(11.), color, room);
                        let at = align.anchor_size(text_at, galley.size()).min;
                        ui.painter()
                            .with_clip_rect(rect.intersect(ui.clip_rect()))
                            .galley(at, galley, color);
                        let resp = resp.on_hover_text(&e.label);
                        if resp.clicked() || resp.double_clicked() {
                            pressed = Some((e.id.clone(), resp.double_clicked()));
                        }
                    }
                });
            }
        }
        if let Some((id, double)) = pressed {
            if double {
                out.added = Some(id.clone());
            }
            s.selected = Some(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, folder: &[&str]) -> Entry {
        Entry {
            id: id.into(),
            label: id.into(),
            folder: folder.iter().map(|s| s.to_string()).collect(),
            icon: None,
        }
    }
    fn catalog() -> Catalog {
        Catalog {
            roots: vec!["Mesh Operations".into(), "Deformers".into()],
            entries: vec![
                entry("Sphere", &["Mesh Operations", "Create", "Primitives"]),
                entry("Cube", &["Mesh Operations", "Create", "Primitives"]),
                entry("Create Vertex", &["Mesh Operations", "Create"]),
                entry("Polygon Bevel", &["Mesh Operations", "Polygon"]),
                entry("Edge Bevel", &["Mesh Operations", "Edge"]),
            ],
            plain: false,
        }
    }
    fn names(g: &[(Vec<String>, Vec<&Entry>)]) -> Vec<(String, Vec<String>)> {
        g.iter()
            .map(|(f, e)| (path_text(f), e.iter().map(|e| e.label.clone()).collect()))
            .collect()
    }

    #[test]
    fn ab_20_21_a_folder_lists_itself_and_its_subfolders_grouped_and_sorted() {
        let c = catalog();
        let root = vec!["Mesh Operations".to_string()];
        assert_eq!(
            names(&c.groups(&root, "", true)),
            [
                (
                    "Mesh Operations/Create".into(),
                    vec!["Create Vertex".into()]
                ),
                (
                    "Mesh Operations/Create/Primitives".into(),
                    vec!["Cube".into(), "Sphere".into()]
                ),
                ("Mesh Operations/Edge".into(), vec!["Edge Bevel".into()]),
                (
                    "Mesh Operations/Polygon".into(),
                    vec!["Polygon Bevel".into()]
                ),
            ]
        );
    }

    #[test]
    fn ab_41_empty_roots_and_folders_are_hidden() {
        let c = catalog();
        assert!(c.folders().iter().all(|f| f[0] != "Deformers"));
    }

    #[test]
    fn ab_23_search_all_and_search_view() {
        let c = catalog();
        let edge = vec!["Mesh Operations".to_string(), "Edge".to_string()];
        let all = c.groups(&edge, "BEVEL", true);
        assert_eq!(
            all.iter().map(|(_, e)| e.len()).sum::<usize>(),
            2,
            "case-insensitive, everywhere"
        );
        let view = c.groups(&edge, "bevel", false);
        assert_eq!(
            names(&view),
            [("Mesh Operations/Edge".into(), vec!["Edge Bevel".into()])]
        );
    }

    #[test]
    fn ab_24_25_first_open_and_history() {
        let c = catalog();
        let mut s = State::default();
        assert_eq!(s.current(&c), ["Mesh Operations"]);
        assert_eq!((s.view, s.size, s.divider), (ViewMode::List, 70., 0.36));
        assert!(!s.can_back());
        let create = vec!["Mesh Operations".to_string(), "Create".to_string()];
        s.go(&c, create.clone());
        let prims = vec![create[0].clone(), create[1].clone(), "Primitives".into()];
        s.go(&c, prims.clone());
        s.back();
        assert_eq!(s.current(&c), create);
        s.forward();
        assert_eq!(s.current(&c), prims);
        s.up(&c);
        assert_eq!(s.current(&c), create);
        // Closing clears the search and remembers it.
        s.search = "cube".into();
        s.open = true;
        s.close();
        assert!(s.search.is_empty() && s.recent == ["cube"]);
    }

    #[test]
    fn ab_11_2_opening_shows_the_search_row_with_the_keyboard_in_it() {
        let c = catalog();
        let ctx = egui::Context::default();
        let mut s = State {
            filter: false,
            ..Default::default()
        };
        let frame = |s: &mut State, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 900.))),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                show(
                    ctx,
                    "test",
                    "Add",
                    Rect::from_min_size(Pos2::ZERO, vec2(10., 10.)),
                    &c,
                    s,
                );
            });
        };
        s.open = true;
        frame(&mut s, vec![]);
        assert!(s.filter, "the search row shows");
        // Typing goes straight into the search field.
        frame(&mut s, vec![egui::Event::Text("cu".into())]);
        assert_eq!(s.search, "cu");
        // The filter toggle still hides the row, and closing clears it.
        s.filter = false;
        frame(&mut s, vec![egui::Event::Text("x".into())]);
        assert_eq!(s.search, "cu");
        s.close();
        assert!(s.search.is_empty());
        // Opened again: the row is back, with the keyboard.
        s.open = true;
        frame(&mut s, vec![]);
        frame(&mut s, vec![egui::Event::Text("b".into())]);
        assert!(s.filter);
        assert_eq!(s.search, "b");
    }
}
