//! The one shared tree-list component (UI conventions bible section 10,
//! UIC-130..154, with the list behaviour of section 4, UIC-30..42).
//!
//! A panel declares only its tool row contents, its columns
//! ([`TreeSpec`]) and its rows ([`TreeRow`]: icon, name, style, children,
//! cell values). This file draws everything — header row, column lines,
//! tree indent, dotted connectors, selection colours, the search row — and
//! turns clicks, renames, eye sweeps and drags into [`TreeEvent`]s. The
//! panel turns events into its own commands, so undo, save and scripting
//! see the same operations as before. Panels keep their own selection; the
//! [`click_select`] helper gives every list the same click rules.
//!
//! Hand-built lists are not allowed (UIC-130): a list that needs something
//! this component can't do adds it here (UIC-154).
use super::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use theme::workspace as palette;

/// LCK-03 padlock, drawn as a glyph (body and shackle) until the icon
/// generator has a padlock icon (provisional). About three quarters of the
/// row height, between text size (easy to miss) and the full row (too
/// heavy), in the "Locked mesh elements" colour so item and geometry lock
/// share one colour; not the accent, which only means on/active/selected
/// (UIC-01).
fn paint_padlock(painter: &egui::Painter, centre: Pos2, color: Color32) {
    let body = Rect::from_center_size(centre + vec2(0., 2.5), vec2(9.5, 7.5));
    painter.rect_filled(body, 1.2, color);
    let shackle: Vec<Pos2> = (0..=10)
        .map(|i| {
            let a = std::f32::consts::PI * i as f32 / 10.;
            centre + vec2(-3.1 * a.cos(), -1.2 - 3.4 * a.sin())
        })
        .collect();
    painter.add(egui::Shape::line(shackle, Stroke::new(1.6f32, color)));
}

/// Items-list zebra stripes: even rows plain, odd rows a touch lighter.
pub(crate) fn row_fill(row: usize) -> Color32 {
    if row % 2 == 1 {
        palette::list_stripe()
    } else {
        palette::list()
    }
}

/// Row height of the tree body: the compact list height (UIC-139).
pub(crate) const ROW: f32 = 17.;
/// Height of the column header row and of the tool rows.
pub(crate) const HEADER: f32 = 18.;
/// Indent per nesting level.
pub(crate) const INDENT: f32 = 12.;
/// Narrowest the Name column gets before the list scrolls sideways.
const NAME_MIN: f32 = 90.;
/// Narrowest any column can be dragged.
const COLUMN_MIN: f32 = 14.;
/// In a [`TreeSpec::last_fills`] list: narrowest Name can be dragged, and
/// narrowest the last column gets before the list scrolls sideways.
const NAME_DRAG_MIN: f32 = 40.;
const LAST_MIN: f32 = 40.;
/// Half-width of a column divider's grab area (UIC-41).
const GRAB: f32 = 3.;
/// Width of the disclosure triangle's slot and of the type icon.
const DISCLOSURE: f32 = 12.;
const ICON: f32 = 14.;
/// Left padding of the tree inside the Name column.
const PAD: f32 = 3.;
/// Width of the editor colour bar at the Name column's left edge.
/// The editor-colour bar at the name's left (UIC-142); N15 widened it
/// from 3.
const EDITOR_BAR: f32 = 5.;
/// How much of a row's editor colour tints its background (UIC-142, N15).
const TINT: f32 = 0.32;

/// What a column is for (UIC-135): narrow icon-headed state columns first,
/// then Name taking the spare width, then value columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnKind {
    State,
    Name,
    Value,
}

/// A column header: an icon for state columns, a word or symbol otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Heading {
    /// The eye (visibility / enable), drawn by the theme.
    Eye,
    /// A one-character stand-in for a state-column icon until the icon
    /// generator has a recipe for it (provisional).
    Glyph(&'static str),
    /// A single-tone generator icon, tinted (N18: the editable mesh).
    Icon(icons::Icon),
    Word(&'static str),
}

#[derive(Clone, Debug)]
pub(crate) struct Column {
    pub kind: ColumnKind,
    pub heading: Heading,
    /// Starting width; the user can drag it (UIC-137).
    pub width: f32,
    pub tip: &'static str,
}

impl Column {
    pub(crate) const fn state(heading: Heading, tip: &'static str) -> Self {
        Self {
            kind: ColumnKind::State,
            heading,
            width: 20.,
            tip,
        }
    }
    pub(crate) const fn name() -> Self {
        Self {
            kind: ColumnKind::Name,
            heading: Heading::Word("Name"),
            width: NAME_MIN,
            tip: "",
        }
    }
    pub(crate) const fn value(word: &'static str, width: f32, tip: &'static str) -> Self {
        Self {
            kind: ColumnKind::Value,
            heading: Heading::Word(word),
            width,
            tip,
        }
    }
}

/// A panel's declaration of its list (UIC-130).
#[derive(Clone, Debug)]
pub(crate) struct TreeSpec {
    pub id: &'static str,
    pub columns: Vec<Column>,
    /// The panel filters rows from [`TreeState::search`] itself (the Items
    /// list keeps the core's IL-07 text filter); otherwise this component
    /// filters the rows it is given.
    pub own_search: bool,
    /// Text-heavy lists (Command History): the last column takes the width
    /// left over, so long text reaches the panel edge, and Name's divider
    /// drags freely (down to [`NAME_DRAG_MIN`]) instead of stopping at the
    /// spare width. Until Name is dragged it still takes the spare width
    /// beside the last column's starting width, so the list opens looking
    /// as UIC-135 has it. Cut-off cells show their full text on hover.
    pub last_fills: bool,
}

impl TreeSpec {
    /// UIC-135: state columns, then exactly one Name, then value columns.
    #[cfg(test)]
    pub(crate) fn check_shape(&self) -> std::result::Result<(), String> {
        let order = |k: ColumnKind| match k {
            ColumnKind::State => 0,
            ColumnKind::Name => 1,
            ColumnKind::Value => 2,
        };
        let kinds: Vec<_> = self.columns.iter().map(|c| order(c.kind)).collect();
        if !kinds.windows(2).all(|w| w[0] <= w[1]) {
            return Err(format!("{}: columns out of order", self.id));
        }
        if kinds.iter().filter(|&&k| k == 1).count() != 1 {
            return Err(format!("{}: needs exactly one Name column", self.id));
        }
        for c in &self.columns {
            match (c.kind, c.heading) {
                (ColumnKind::State, Heading::Word(_)) => {
                    return Err(format!("{}: a state column has a word header", self.id));
                }
                (ColumnKind::Value, Heading::Eye | Heading::Glyph(_) | Heading::Icon(_)) => {
                    return Err(format!("{}: a value column has an icon header", self.id));
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub(crate) fn name_column(&self) -> usize {
        self.columns
            .iter()
            .position(|c| c.kind == ColumnKind::Name)
            .unwrap_or(0)
    }
}

/// How a row is selected: the first-selected (primary) row is a shade
/// darker with a lighter label (UIC-34).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Selected {
    #[default]
    No,
    Yes,
    Primary,
}

/// Name styles that carry meaning (UIC-141).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NameStyle {
    /// Top-level scene/root row.
    pub bold: bool,
    /// Root row with unsaved changes: "*" after the name.
    pub unsaved: bool,
    pub italic: bool,
    /// Nothing in it (a mesh with no geometry): light grey.
    pub empty: bool,
    /// Hidden or disabled: dimmed.
    pub dimmed: bool,
}

/// The type icon before a row's name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RowIcon {
    None,
    Icon(icons::Icon),
    /// Camera and light placeholder glyphs (CL-1).
    Locator(camera_light_ui::LocatorGlyph),
    /// A one-letter type tag until the type has an icon.
    Letter(&'static str),
}

/// A small mark drawn after a row's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Badge {
    /// The scene's render camera (CL-21).
    RenderCamera,
    /// A locked item (LCK-03).
    Locked,
    /// An item whose kit isn't installed (KIT-1): a "?" in the error
    /// colour; the row's tooltip names the kit.
    MissingKit,
    /// The paint target (PNT-13): the paintbrush beside the Image Map
    /// layer painting goes into. Reuses the generator's paint icon.
    #[cfg(feature = "painting")]
    PaintTarget,
}

/// One non-Name cell of a row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Cell {
    Empty,
    /// A state-column toggle (UIC-138): the eye, or a glyph lit in accent
    /// when on.
    Toggle {
        on: bool,
        tip: String,
        /// Drawn faded: the row's own state holds, but something else
        /// (a hidden parent, GV-05) overrides it.
        dimmed: bool,
    },
    /// Drawn by the panel ([`TreeHooks::cell`]): value columns that edit in
    /// place (the Shader Tree's blend, opacity and effect).
    Custom,
}

#[derive(Clone, Debug)]
pub(crate) struct TreeRow<K> {
    pub key: K,
    pub depth: usize,
    /// `Some(open)` when the row has children (UIC-35).
    pub disclosure: Option<bool>,
    /// `Some(open)` when the row has attribute rows behind a +/- (UIC-36).
    pub attributes: Option<bool>,
    pub icon: RowIcon,
    pub name: String,
    pub style: NameStyle,
    pub selected: Selected,
    /// A user-chosen editor colour (UIC-142, note 10 IC-B1): a faint tint
    /// behind the row and a narrow bar at the Name column's left edge,
    /// which stays visible on selected rows.
    pub tint: Option<Color32>,
    /// Marks after the name.
    pub badges: Vec<Badge>,
    /// One per column; the Name column's entry is ignored.
    pub cells: Vec<Cell>,
    /// Can rows be dropped onto it as children (UIC-143)?
    pub accepts_children: bool,
    /// Can it be dragged, renamed, selected?
    pub draggable: bool,
    pub renamable: bool,
    pub tip: Option<String>,
    /// An attribute row (UIC-36) or other fixed row: no connector of its
    /// own children, never a drop target between rows.
    pub fixed: bool,
}

impl<K> TreeRow<K> {
    pub(crate) fn new(key: K, depth: usize, name: impl Into<String>, columns: usize) -> Self {
        Self {
            key,
            depth,
            disclosure: None,
            attributes: None,
            icon: RowIcon::None,
            name: name.into(),
            style: NameStyle::default(),
            selected: Selected::No,
            tint: None,
            badges: Vec::new(),
            cells: vec![Cell::Empty; columns],
            accepts_children: false,
            draggable: false,
            renamable: false,
            tip: None,
            fixed: false,
        }
    }
}

/// The search row (UIC-40).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Search {
    pub open: bool,
    pub text: String,
    /// Pattern matching with `*` and `?` instead of plain text search.
    pub wildcard: bool,
    pub match_case: bool,
    /// Matches as a flat list for easy multi-select.
    pub flat: bool,
}

impl Search {
    /// True while the search row is open with something typed.
    pub(crate) fn active(&self) -> bool {
        self.open && !self.text.trim().is_empty()
    }
    pub(crate) fn matches(&self, name: &str) -> bool {
        let text = self.text.trim();
        if text.is_empty() {
            return true;
        }
        let (pattern, name) = if self.match_case {
            (text.to_owned(), name.to_owned())
        } else {
            (text.to_lowercase(), name.to_lowercase())
        };
        if self.wildcard {
            wildcard_match(&pattern, &name)
        } else {
            name.contains(&pattern)
        }
    }
}

/// `*` matches any run of characters, `?` exactly one; the whole name must
/// match (UIC-40, provisional syntax). Shared with the core's Items filter.
pub(crate) fn wildcard_match(pattern: &str, text: &str) -> bool {
    operandi_core::item_list::wildcard_match(pattern, text)
}

/// The rows a search shows: matches, plus their ancestors (dimmed) so the
/// matches keep their place; or, Flat, just the matches at the top level.
pub(crate) fn filter_rows<K: Clone>(rows: &[TreeRow<K>], search: &Search) -> Vec<TreeRow<K>> {
    if !search.active() {
        return rows.to_vec();
    }
    if search.flat {
        return rows
            .iter()
            .filter(|r| !r.fixed && search.matches(&r.name))
            .map(|r| TreeRow {
                depth: 0,
                disclosure: None,
                attributes: None,
                ..r.clone()
            })
            .collect();
    }
    let mut keep = vec![false; rows.len()];
    let mut ancestors: Vec<usize> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        ancestors.truncate(row.depth.min(ancestors.len()));
        if !row.fixed && search.matches(&row.name) {
            keep[i] = true;
            for &a in &ancestors {
                keep[a] = true;
            }
        }
        ancestors.push(i);
    }
    rows.iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, r)| {
            let mut r = r.clone();
            if !search.matches(&r.name) {
                r.style.dimmed = true;
            }
            r
        })
        .collect()
}

/// How a click on a row's name changes the selection (UIC-30..32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClickHow {
    /// Plain click: this row alone.
    Only,
    /// Shift+click: the range from the last clicked row.
    Range,
    /// Ctrl+click: add or remove it. On a Mac both Cmd and the physical
    /// Ctrl key toggle (Jason, 2026-11-08, revising FLAGS F1: Cmd added),
    /// and Ctrl never becomes a right-click. Opt does nothing special.
    Toggle,
}

impl ClickHow {
    pub(crate) fn from_modifiers(m: Modifiers) -> Self {
        if m.shift {
            Self::Range
        } else if m.ctrl || m.mac_cmd || m.command {
            Self::Toggle
        } else {
            Self::Only
        }
    }
}

/// Original-compatible A7: on a Mac only Cmd toggles list rows; Ctrl+click
/// is a right-click.
const CMD_TOGGLES_ON_MAC: &str = "A7";

/// Whether a left click with `m` held is a right-click instead: on a Mac
/// with A7 on, a click with the physical Ctrl key and not Cmd (macOS's own
/// Ctrl+click). It then opens the row's (or empty area's) menu and doesn't
/// change the selection. Never off a Mac, never with A7 off.
fn ctrl_click_is_right_click(m: Modifiers, mac: bool, original: bool) -> bool {
    mac && original && m.ctrl && !m.mac_cmd
}

/// `Response::context_menu`, also opened by a click that
/// `ctrl_click_is_right_click` turned into a right-click.
fn context_menu_with(response: &Response, right_click: bool, add: impl FnOnce(&mut Ui)) {
    let open = response.secondary_clicked() || right_click && response.clicked();
    egui::Popup::menu(response)
        .open_memory(if open {
            Some(egui::SetOpenCommand::Bool(true))
        } else if response.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        })
        .at_pointer_fixed()
        .show(add);
}

/// The selection after a click, for panels that keep their own selection
/// list (first entry = primary, UIC-34). `order` is the rows as shown.
/// A range keeps the anchor and runs from it to the clicked row.
pub(crate) fn click_select<K: Copy + PartialEq>(
    selected: &mut Vec<K>,
    anchor: &mut Option<K>,
    order: &[K],
    key: K,
    how: ClickHow,
) {
    match how {
        ClickHow::Range => {
            let from = anchor.and_then(|a| order.iter().position(|&k| k == a));
            let to = order.iter().position(|&k| k == key);
            if let (Some(a), Some(b)) = (from, to) {
                let mut range: Vec<K> = if a <= b {
                    order[a..=b].to_vec()
                } else {
                    order[b..=a].iter().rev().copied().collect()
                };
                // The anchor stays primary when it was already selected.
                range.dedup();
                *selected = range;
                return;
            }
            *selected = vec![key];
            *anchor = Some(key);
        }
        ClickHow::Toggle => {
            if let Some(i) = selected.iter().position(|&k| k == key) {
                selected.remove(i);
            } else {
                selected.push(key);
            }
            *anchor = Some(key);
        }
        ClickHow::Only => {
            *selected = vec![key];
            *anchor = Some(key);
        }
    }
}

/// Where dragged rows land (UIC-143).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropAt<K> {
    /// Onto a row: become its children.
    On(K),
    /// Between rows: at that row's level, before or after it.
    Before(K),
    After(K),
    /// Below the last row.
    End,
}

/// The share of a row's height at its top and at its bottom that drops
/// between rows (UIC-143); the middle drops onto the row.
pub(crate) const INSERT_BAND: f32 = 0.25;

/// Where dragged rows land when the pointer is over `rows[index]`, `t` of
/// the way down it (0 top, 1 bottom), and the depth they land at (for the
/// insert line's indent). UIC-143:
///
/// - top band: before this row, at its level;
/// - bottom band: after this row, at its level; but when the row is open
///   with its children shown, as the first child (before the row below);
/// - middle: onto the row as a child, when it takes children (otherwise
///   the nearer half picks before or after);
/// - a fixed row (scene root) takes the whole row as "onto", except the
///   bottom band of an open one, which is before its first child.
///
/// `None` when the drop would put rows inside themselves: over a dragged
/// row or anywhere in a dragged row's branch.
pub(crate) fn drop_zone<K: Copy + Eq>(
    rows: &[TreeRow<K>],
    index: usize,
    t: f32,
    dragged: &[K],
) -> Option<(DropAt<K>, usize)> {
    let row = rows.get(index)?;
    // The row and its ancestors (the nearest shallower rows above it).
    let mut depth = row.depth;
    let mut lineage = std::iter::once(row).chain(rows[..index].iter().rev().filter(|r| {
        let ancestor = r.depth < depth;
        if ancestor {
            depth = r.depth;
        }
        ancestor
    }));
    if lineage.any(|r| dragged.contains(&r.key)) {
        return None;
    }
    let first_child = rows
        .get(index + 1)
        .filter(|n| n.depth > row.depth && !n.fixed);
    let bottom = t > 1. - INSERT_BAND;
    if let Some(child) = first_child.filter(|_| bottom) {
        return Some((DropAt::Before(child.key), child.depth));
    }
    let at = if row.fixed {
        if row.accepts_children {
            DropAt::On(row.key)
        } else {
            DropAt::After(row.key)
        }
    } else if t < INSERT_BAND {
        DropAt::Before(row.key)
    } else if bottom {
        DropAt::After(row.key)
    } else if row.accepts_children {
        DropAt::On(row.key)
    } else if t < 0.5 {
        DropAt::Before(row.key)
    } else {
        DropAt::After(row.key)
    };
    Some((at, row.depth))
}

/// What the user did in the list this frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TreeEvent<K> {
    Click {
        key: K,
        how: ClickHow,
    },
    /// UIC-38: a parent's double-click selects its children.
    DoubleClick(K),
    /// The disclosure triangle; Shift opens or closes the whole subtree
    /// (UIC-37).
    Disclosure {
        key: K,
        subtree: bool,
    },
    /// The attributes +/- (UIC-36).
    Attributes(K),
    Rename {
        key: K,
        name: String,
    },
    /// A click on a state cell; Alt+click shows only this one (UIC-138).
    Toggle {
        key: K,
        column: usize,
        solo: bool,
    },
    /// A drag down a state column: every row passed gets `on`.
    Sweep {
        column: usize,
        keys: Vec<K>,
        on: bool,
    },
    Drop {
        keys: Vec<K>,
        at: DropAt<K>,
        /// Alt held at release.
        alt: bool,
    },
}

/// Panel-specific drawing the component calls back into.
pub(crate) trait TreeHooks<K> {
    /// Draw a [`Cell::Custom`] cell.
    fn cell(&mut self, _ui: &mut Ui, _column: usize, _key: K, _rect: Rect) {}
    /// Whether a row has a right-click menu (UIC-07: never an empty one).
    fn has_row_menu(&self, _key: K) -> bool {
        false
    }
    /// `rename` opens the editor on the row's current name.
    fn row_menu(&mut self, _ui: &mut Ui, _key: K, _rename: &mut dyn FnMut()) {}
    /// The panel's general menu on empty space (UIC-144), if any.
    fn has_empty_menu(&self) -> bool {
        false
    }
    fn empty_menu(&mut self, _ui: &mut Ui) {}
}

/// View state of one list (not part of the document, not undone).
#[derive(Debug)]
pub(crate) struct TreeState<K> {
    pub search: Search,
    /// Dragged column widths, by column index (UIC-41).
    pub widths: HashMap<usize, f32>,
    /// The last clicked row, for Shift ranges.
    pub anchor: Option<K>,
    /// The row being renamed in place, and its text.
    pub rename: Option<(K, String)>,
    rename_opening: bool,
    last_click: Option<(K, f64)>,
    sweep: Option<(usize, bool, Vec<K>)>,
    drag: Option<Vec<K>>,
    drop: Option<DropAt<K>>,
    resizing: Option<usize>,
    /// Each row's rect from the last frame, in order (tests, drags).
    pub rects: Vec<(K, Rect)>,
    /// Each cell's rect from the last frame, by row and column.
    pub cells: HashMap<(K, usize), Rect>,
    /// The tool row's controls from the last frame, by name (tests).
    pub tool_rects: HashMap<String, Rect>,
    /// Column divider x positions from the last frame (tests).
    pub dividers: Vec<f32>,
    /// The body's whole rect last frame (tests: lines run its height).
    pub body: Rect,
}

impl<K> Default for TreeState<K> {
    fn default() -> Self {
        Self {
            search: Search::default(),
            widths: HashMap::new(),
            anchor: None,
            rename: None,
            rename_opening: false,
            last_click: None,
            sweep: None,
            drag: None,
            drop: None,
            resizing: None,
            rects: Vec::new(),
            cells: HashMap::new(),
            tool_rects: HashMap::new(),
            dividers: Vec::new(),
            body: Rect::NOTHING,
        }
    }
}

impl<K: Copy + Eq + Hash + Debug> TreeState<K> {
    /// Rows of this list are being dragged (another panel may take the
    /// drop, e.g. the preset browser's folders, PB-120).
    pub(crate) fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    /// Start renaming `key` in place.
    pub(crate) fn start_rename(&mut self, key: K, name: &str) {
        self.rename = Some((key, name.to_owned()));
        self.rename_opening = true;
    }
    /// Forget rows that no longer exist.
    pub(crate) fn retain(&mut self, exists: impl Fn(K) -> bool) {
        if self.anchor.is_some_and(|k| !exists(k)) {
            self.anchor = None;
        }
        if self.rename.as_ref().is_some_and(|(k, _)| !exists(*k)) {
            self.rename = None;
        }
    }
}

/// Column x-ranges for a list `width` wide: state and value columns keep
/// their widths, Name takes the rest but never less than its minimum
/// (then the list scrolls sideways, UIC-131).
pub(crate) fn column_spans(
    spec: &TreeSpec,
    widths: &HashMap<usize, f32>,
    left: f32,
    width: f32,
) -> Vec<Rangef> {
    let name = spec.name_column();
    let last = spec.columns.len() - 1;
    let fills = spec.last_fills && last != name;
    let width_of = |i: usize| widths.get(&i).copied().unwrap_or(spec.columns[i].width);
    // Columns that keep their own widths: all but Name (and the last one
    // in a filling list).
    let fixed: f32 = (0..spec.columns.len())
        .filter(|&i| i != name && !(fills && i == last))
        .map(width_of)
        .sum();
    let (name_width, last_width) = if fills {
        let name_width = match widths.get(&name) {
            Some(&w) => w.max(NAME_DRAG_MIN),
            None => (width - fixed - width_of(last)).max(NAME_MIN),
        };
        (name_width, (width - fixed - name_width).max(LAST_MIN))
    } else {
        let name_width = widths
            .get(&name)
            .copied()
            .unwrap_or(0.)
            .max(width - fixed)
            .max(NAME_MIN);
        (name_width, width_of(last))
    };
    let mut x = left;
    (0..spec.columns.len())
        .map(|i| {
            let w = if i == name {
                name_width
            } else if i == last {
                last_width
            } else {
                width_of(i)
            };
            let span = Rangef::new(x, x + w);
            x += w;
            span
        })
        .collect()
}

/// Paint a value cell's text, cut off at the cell's edge with no ellipsis
/// (UIC-137); when it is cut off, hovering the cell shows all of it.
/// Returns whether it was cut off.
pub(crate) fn value_text(
    ui: &Ui,
    id: Id,
    cell: Rect,
    text: &str,
    font: FontId,
    color: Color32,
) -> bool {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let at = pos2(cell.left() + 3., cell.center().y - galley.size().y / 2.);
    let cut = at.x + galley.size().x > cell.right();
    ui.painter()
        .with_clip_rect(cell.intersect(ui.clip_rect()))
        .galley(at, galley, color);
    if cut {
        ui.interact(cell, id, Sense::hover()).on_hover_text(text);
    }
    cut
}

/// Dotted connector lines for one row (UIC-140): which ancestor levels
/// carry a line straight through, and whether this row is its parent's
/// last child (its elbow stops at the middle).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Connector {
    pub through: Vec<bool>,
    pub last: bool,
}

/// Connectors for rows given in tree order by depth.
pub(crate) fn connectors(depths: &[usize]) -> Vec<Connector> {
    let n = depths.len();
    // Does row i have a later sibling (same depth, before any shallower row)?
    let mut later = vec![false; n];
    for i in 0..n {
        for &d in &depths[i + 1..] {
            if d < depths[i] {
                break;
            }
            if d == depths[i] {
                later[i] = true;
                break;
            }
        }
    }
    let mut stack: Vec<usize> = Vec::new();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let d = depths[i];
        stack.truncate(d);
        // Levels 1..d-1: the line of the ancestor at that depth continues
        // when that ancestor has a later sibling.
        let through = (1..d)
            .map(|level| stack.get(level).is_some_and(|&a| later[a]))
            .collect();
        out.push(Connector {
            through,
            last: !later[i],
        });
        while stack.len() < d {
            stack.push(i);
        }
        stack.push(i);
    }
    out
}

/// The x where the vertical connector for children of a row at `depth`
/// runs: under the middle of that row's disclosure slot.
fn level_x(name_left: f32, depth: usize) -> f32 {
    name_left + PAD + depth as f32 * INDENT + DISCLOSURE / 2.
}

/// The accent, and the primary row's lighter accent (UIC-34).
fn accent(primary: bool) -> Color32 {
    let a = palette::selected_text();
    if primary {
        a.lerp_to_gamma(Color32::WHITE, 0.3)
    } else {
        a
    }
}

/// Which end of the tool row a [`tool_row`] call is filling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// The panel's main add control.
    Left,
    /// Small buttons and toggles, added right to left: the first one sits
    /// next to the funnel.
    Right,
}

/// The tool row (UIC-132): `content` is called for the left end (the main
/// add control), then for the right end (small buttons and toggles); the
/// funnel (search toggle) is always last, at the far right.
pub(crate) fn tool_row<K: Copy + Eq + Hash + Debug>(
    ui: &mut Ui,
    state: &mut TreeState<K>,
    funnel: bool,
    mut content: impl FnMut(&mut Ui, Side),
) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEADER + 4.), Sense::hover());
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(2., 1.)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 3.;
    content(&mut row, Side::Left);
    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 3.;
        if funnel {
            let on = state.search.open;
            let response = toggle_button(ui, "F", on).on_hover_text(if on {
                "Hide the search row; the full list comes back and your selection stays"
            } else {
                "Search this list"
            });
            if response.clicked() {
                state.search.open = !on;
            }
            state.tool_rects.insert("funnel".into(), response.rect);
        }
        content(ui, Side::Right);
    });
}

/// A small text toggle for a tool row, lit in accent when on (UIC-134).
pub(crate) fn toggle_button(ui: &mut Ui, text: &str, on: bool) -> Response {
    let color = if on {
        theme::chrome::active()
    } else {
        theme::text()
    };
    ui.add(
        Button::new(RichText::new(text).color(color).size(11.))
            .min_size(vec2(16., HEADER - 2.))
            .selected(false),
    )
}

/// A labelled dropdown on the tool row (UIC-133): "Filter [(none) ▾]".
pub(crate) fn tool_dropdown<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &str,
    label: &str,
    value: &mut T,
    choices: &[(T, &str)],
    width: f32,
) -> Response {
    ui.label(RichText::new(label).size(11.));
    let current = choices
        .iter()
        .find(|(v, _)| v == value)
        .map_or("", |(_, n)| *n);
    ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(width)
        .show_ui(ui, |ui| {
            for &(v, n) in choices {
                ui.selectable_value(value, v, n);
            }
        })
        .response
}

/// The search row under the tool row (UIC-40): field, mode pop-up, Match
/// Case and Flat. The caller registers the field as a text editor.
fn search_row<K>(ui: &mut Ui, id: &str, state: &mut TreeState<K>) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEADER + 2.), Sense::hover());
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(2., 1.)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 3.;
    let s = &mut state.search;
    if toggle_button(&mut row, "Flat", s.flat)
        .on_hover_text("Show matches as a flat list for easy multi-select")
        .clicked()
    {
        s.flat ^= true;
    }
    if toggle_button(&mut row, "Aa", s.match_case)
        .on_hover_text("Match Case")
        .clicked()
    {
        s.match_case ^= true;
    }
    ComboBox::from_id_salt((id, "search-mode"))
        .selected_text(if s.wildcard { "Pattern" } else { "Text" })
        .width(58.)
        .show_ui(&mut row, |ui| {
            ui.selectable_value(&mut s.wildcard, false, "Text")
                .on_hover_text("Names containing the text");
            ui.selectable_value(&mut s.wildcard, true, "Pattern")
                .on_hover_text("* matches any run of characters, ? one character");
        });
    let search = row.add(
        TextEdit::singleline(&mut s.text)
            .hint_text("Search")
            .id_salt((id, "search"))
            .desired_width(row.available_width()),
    );
    crate::numeric_tab::register(&row, &search, Some(&s.text));
    search
}

/// A widget id that a second copy of its panel (a panel popover beside
/// the docked one, panel_popover.rs) gets salted, so both copies stay
/// live. The docked copy keeps the plain id.
pub(crate) fn copy_id(ctx: &egui::Context, source: impl std::hash::Hash) -> Id {
    let id = Id::new(source);
    match ctx.data(|d| d.get_temp::<Id>(Id::new(COPY_SALT))) {
        Some(salt) => id.with(salt),
        None => id,
    }
}

/// Where `copy_id` finds the salt of the copy drawing now.
pub(crate) const COPY_SALT: &str = "tree-list-copy-salt";

/// Whether a second copy of a panel is drawing now (`copy_id`).
pub(crate) fn drawing_copy(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Id>(Id::new(COPY_SALT))).is_some()
}

/// Draw `f` as a second copy of a panel: lists inside it take ids of
/// their own, salted with `salt`.
pub(crate) fn as_copy<R>(ctx: &egui::Context, salt: Id, f: impl FnOnce() -> R) -> R {
    ctx.data_mut(|d| d.insert_temp(Id::new(COPY_SALT), salt));
    let out = f();
    ctx.data_mut(|d| d.remove::<Id>(Id::new(COPY_SALT)));
    out
}

/// Draw the list: header row, then the tree body filling `ui`'s remaining
/// height. Returns what the user did. `editors` collects text fields the
/// app must treat as typing (search, rename).
pub(crate) fn show<K: Copy + Eq + Hash + Debug + 'static>(
    ui: &mut Ui,
    spec: &TreeSpec,
    state: &mut TreeState<K>,
    rows: &[TreeRow<K>],
    hooks: &mut dyn TreeHooks<K>,
    editors: &mut std::collections::HashSet<Id>,
) -> Vec<TreeEvent<K>> {
    let mut events = Vec::new();
    if state.search.open {
        let field = search_row(ui, spec.id, state);
        editors.insert(field.id);
    }
    let shown: Vec<TreeRow<K>>;
    let rows = if spec.own_search {
        rows
    } else {
        shown = filter_rows(rows, &state.search);
        &shown[..]
    };
    let outer = ui.available_rect_before_wrap();
    let height = (outer.height()).max(HEADER + ROW);
    // One horizontal scroll for header and body together; the body scrolls
    // up and down inside it (UIC-131).
    let content_width = column_spans(spec, &state.widths, 0., outer.width())
        .last()
        .map_or(outer.width(), |s| s.max);
    egui::ScrollArea::horizontal()
        .id_salt((spec.id, "h"))
        .auto_shrink([false, false])
        .max_height(height)
        .show(ui, |ui| {
            ui.set_min_width(content_width);
            let top = ui.cursor().top();
            let left = ui.cursor().left();
            let spans = column_spans(spec, &state.widths, left, outer.width());
            header_row(ui, spec, state, &spans, top);
            let body_height = (top + height - ui.cursor().top() - 12.).max(ROW);
            egui::ScrollArea::vertical()
                .id_salt((spec.id, "v"))
                .auto_shrink([false, false])
                .max_height(body_height)
                .show(ui, |ui| {
                    body(
                        ui,
                        spec,
                        state,
                        rows,
                        hooks,
                        editors,
                        &spans,
                        body_height,
                        &mut events,
                    );
                });
        });
    events
}

fn header_row<K>(
    ui: &mut Ui,
    spec: &TreeSpec,
    state: &mut TreeState<K>,
    spans: &[Rangef],
    top: f32,
) {
    let right = spans.last().map_or(ui.max_rect().right(), |s| s.max);
    let left = spans.first().map_or(ui.max_rect().left(), |s| s.min);
    let (rect, _) = ui.allocate_exact_size(vec2(right - left, HEADER), Sense::hover());
    let rect = Rect::from_x_y_ranges(rect.x_range(), top..=top + HEADER);
    ui.painter().rect_filled(rect, 0., palette::header());
    // The column lines run through the header too (UIC-136).
    for span in &spans[..spans.len().saturating_sub(1)] {
        ui.painter().vline(
            span.max,
            rect.y_range(),
            Stroke::new(1_f32, palette::separator()),
        );
    }
    state.dividers.clear();
    for (i, (column, span)) in spec.columns.iter().zip(spans).enumerate() {
        let cell = Rect::from_x_y_ranges(*span, rect.y_range());
        let response = ui.interact(
            cell,
            copy_id(ui.ctx(), (spec.id, "heading", i)),
            Sense::hover(),
        );
        match column.heading {
            Heading::Eye => theme::list_eye(ui, cell.shrink(1.), true, false),
            Heading::Icon(i) => icons::paint_in(ui, cell.shrink(2.5), i, palette::header_text()),
            Heading::Glyph(g) => {
                ui.painter().text(
                    cell.center(),
                    Align2::CENTER_CENTER,
                    g,
                    FontId::proportional(10.),
                    palette::header_text(),
                );
            }
            Heading::Word(w) => {
                ui.painter().with_clip_rect(cell).text(
                    cell.left_center() + vec2(3., 0.),
                    Align2::LEFT_CENTER,
                    w,
                    FontId::proportional(10.),
                    palette::header_text(),
                );
            }
        }
        if !column.tip.is_empty() {
            response.on_hover_text(column.tip);
        }
        // UIC-41: drag a divider to resize the column on its left.
        if i + 1 < spec.columns.len() {
            let x = span.max;
            state.dividers.push(x);
            let grab = Rect::from_x_y_ranges(x - GRAB..=x + GRAB, rect.y_range());
            let handle = ui.interact(
                grab,
                copy_id(ui.ctx(), (spec.id, "divider", i)),
                Sense::drag(),
            );
            if handle.hovered() || handle.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if handle.drag_started() {
                state.resizing = Some(i);
            }
            if handle.dragged() && state.resizing == Some(i) {
                let w = (span.max - span.min + handle.drag_delta().x).max(COLUMN_MIN);
                state.widths.insert(i, w);
            }
            if handle.drag_stopped() {
                state.resizing = None;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn body<K: Copy + Eq + Hash + Debug + 'static>(
    ui: &mut Ui,
    spec: &TreeSpec,
    state: &mut TreeState<K>,
    rows: &[TreeRow<K>],
    hooks: &mut dyn TreeHooks<K>,
    editors: &mut std::collections::HashSet<Id>,
    spans: &[Rangef],
    min_height: f32,
    events: &mut Vec<TreeEvent<K>>,
) {
    let left = spans.first().map_or(0., |s| s.min);
    let right = spans.last().map_or(0., |s| s.max);
    let top = ui.cursor().top();
    let content = (rows.len() as f32 * ROW).max(min_height);
    let (whole, background) = ui.allocate_exact_size(vec2(right - left, content), Sense::click());
    let whole = Rect::from_min_size(pos2(left, top), whole.size());
    state.body = whole;
    ui.painter().rect_filled(whole, 0., palette::list());
    let name_col = spec.name_column();
    let name_span = spans[name_col];
    let links = connectors(&rows.iter().map(|r| r.depth).collect::<Vec<_>>());
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let (mods, released, now) = ui.input(|i| (i.modifiers, i.pointer.any_released(), i.time));
    let delay = ui.ctx().options(|o| o.input_options.max_double_click_delay);
    let right_click = ctrl_click_is_right_click(
        mods,
        cfg!(target_os = "macos"),
        crate::original_compatible::is_original(CMD_TOGGLES_ON_MAC),
    );
    state.rects.clear();
    state.cells.clear();
    let selected_order: Vec<K> = rows
        .iter()
        .filter(|r| r.selected != Selected::No)
        .map(|r| r.key)
        .collect();
    let mut drop = None;
    for (index, row) in rows.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(left, top + index as f32 * ROW),
            vec2(right - left, ROW),
        );
        state.rects.push((row.key, rect));
        // Rows scrolled out of sight (or past the panel) only record where
        // they are: a list of 10,000 items lays out the few dozen on screen.
        // A row being renamed keeps its editor.
        if !rect.intersects(ui.clip_rect())
            && state.rename.as_ref().is_none_or(|(k, _)| *k != row.key)
        {
            for (c, span) in spans.iter().enumerate() {
                state
                    .cells
                    .insert((row.key, c), Rect::from_x_y_ranges(*span, rect.y_range()));
            }
            continue;
        }
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        // Background: stripe, tint, selection (UIC-33/34, UIC-142).
        let mut fill = row_fill(index);
        match row.selected {
            Selected::Primary => fill = palette::primary(),
            Selected::Yes => fill = palette::selected(),
            Selected::No => {}
        }
        // N15 (Jason, 2026-11-16: coloured items could stand out more):
        // a stronger tint (was 15%), kept on selected rows too, so the
        // colour doesn't vanish when the row is picked.
        if let Some(tint) = row.tint {
            let share = if row.selected == Selected::No {
                TINT
            } else {
                TINT * 0.6
            };
            fill = fill.lerp_to_gamma(tint, share);
        }
        painter.rect_filled(rect, 0., fill);
        if let Some(tint) = row.tint {
            let bar = Rect::from_min_size(pos2(name_span.min, rect.top()), vec2(EDITOR_BAR, ROW));
            painter.rect_filled(bar, 0., tint);
        }
        // The whole row: click selects, drag moves, right-click menus.
        let response = ui.interact(
            rect,
            copy_id(ui.ctx(), (spec.id, "row", row.key)),
            Sense::click_and_drag(),
        );
        let name_cell = Rect::from_x_y_ranges(name_span, rect.y_range());
        let indent = row.depth as f32 * INDENT;
        let disclosure = Rect::from_min_size(
            pos2(name_span.min + PAD + indent, rect.top()),
            vec2(DISCLOSURE, ROW),
        );
        let icon = Rect::from_center_size(
            pos2(disclosure.right() + ICON / 2. + 1., rect.center().y),
            vec2(ICON, ICON),
        );
        // Connectors (UIC-140).
        paint_connector(&painter, name_span.min, rect, row.depth, &links[index]);
        if let Some(open) = row.disclosure {
            let hit = ui.interact(
                disclosure,
                copy_id(ui.ctx(), (spec.id, "disclosure", row.key)),
                Sense::click(),
            );
            icons::disclosure_in(
                ui,
                Rect::from_center_size(disclosure.center(), Vec2::splat(9.)),
                if open { 1. } else { 0. },
                palette::glyphs(),
            );
            if hit.clicked() {
                events.push(TreeEvent::Disclosure {
                    key: row.key,
                    subtree: mods.shift,
                });
            }
        }
        let mut name_left = icon.right() + 3.;
        match row.icon {
            RowIcon::None => name_left = disclosure.right() + 2.,
            RowIcon::Icon(i) => icons::paint(ui, icon, i),
            RowIcon::Locator(glyph) => camera_light_ui::paint_glyph(
                ui,
                icon,
                glyph,
                if row.style.dimmed {
                    palette::inactive()
                } else {
                    palette::field_text()
                },
            ),
            RowIcon::Letter(l) => {
                let tag = icon.shrink(1.);
                painter.rect_filled(tag, 2., palette::header());
                painter.text(
                    tag.center(),
                    Align2::CENTER_CENTER,
                    l,
                    FontId::proportional(9.),
                    theme::text(),
                );
            }
        }
        if let Some(open) = row.attributes {
            let toggle =
                Rect::from_center_size(pos2(name_left + 5., rect.center().y), vec2(10., 10.));
            let hit = ui
                .interact(
                    toggle,
                    copy_id(ui.ctx(), (spec.id, "attributes", row.key)),
                    Sense::click(),
                )
                .on_hover_text(if open {
                    "Hide this row's attributes"
                } else {
                    "Show this row's attributes"
                });
            painter.rect_stroke(
                toggle,
                1.,
                Stroke::new(1_f32, palette::field_text()),
                StrokeKind::Inside,
            );
            painter.hline(
                toggle.x_range().shrink(2.5),
                toggle.center().y,
                Stroke::new(1_f32, palette::field_text()),
            );
            if !open {
                painter.vline(
                    toggle.center().x,
                    toggle.y_range().shrink(2.5),
                    Stroke::new(1_f32, palette::field_text()),
                );
            }
            if hit.clicked() {
                events.push(TreeEvent::Attributes(row.key));
            }
            name_left = toggle.right() + 3.;
        }
        let text_rect = Rect::from_min_max(
            pos2(name_left, rect.top()),
            pos2(name_cell.right() - 2., rect.bottom()),
        );
        state.cells.insert((row.key, name_col), name_cell);
        // Rename in place (UIC-39).
        let renaming = state.rename.as_ref().is_some_and(|(k, _)| *k == row.key);
        if renaming {
            let (key, text) = state.rename.as_mut().expect("renaming");
            let key = *key;
            let edit = ui.put(
                text_rect.shrink2(vec2(0., 0.5)),
                TextEdit::singleline(text)
                    .id_salt((spec.id, "rename"))
                    .font(crate::typography::standard_font())
                    .margin(Margin::symmetric(2, 0)),
            );
            editors.insert(edit.id);
            crate::numeric_tab::register(ui, &edit, Some(text.as_str()));
            crate::numeric_tab::focus_and_select_on_open(&edit, text, &mut state.rename_opening);
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                state.rename = None;
            } else if edit.lost_focus() {
                // Enter or a click elsewhere commits.
                let name = text.trim().to_owned();
                state.rename = None;
                if !name.is_empty() && name != row.name {
                    events.push(TreeEvent::Rename { key, name });
                }
            }
        } else {
            let (drawn, cut) = paint_name(&painter, text_rect, row);
            if cut && spec.last_fills && row.tip.is_none() {
                response.clone().on_hover_text(row.name.as_str());
            }
            let color = name_color(row.selected, row.style);
            let mut x = drawn.right() + 8.;
            for badge in &row.badges {
                let at = pos2(x, rect.center().y);
                match badge {
                    Badge::RenderCamera => {
                        camera_light_ui::paint_render_marker(ui, at + vec2(1., 0.), color);
                        x += 14.;
                    }
                    Badge::Locked => {
                        paint_padlock(&painter, at, theme::viewport::locked());
                        x += 12.;
                    }
                    #[cfg(feature = "painting")]
                    Badge::PaintTarget => {
                        let r = egui::Rect::from_min_size(pos2(x, at.y - 7.), vec2(14., 14.));
                        icons::paint(ui, r, icons::Icon::PaintTarget);
                        x += 16.;
                    }
                    Badge::MissingKit => {
                        painter.text(
                            at,
                            egui::Align2::LEFT_CENTER,
                            "?",
                            egui::FontId::proportional(11.),
                            theme::error(),
                        );
                        x += 12.;
                    }
                }
            }
        }
        // State and value cells.
        for (c, (column, span)) in spec.columns.iter().zip(spans).enumerate() {
            if c == name_col {
                continue;
            }
            let cell = Rect::from_x_y_ranges(*span, rect.y_range());
            state.cells.insert((row.key, c), cell);
            match row.cells.get(c).unwrap_or(&Cell::Empty) {
                Cell::Empty => {}
                Cell::Toggle { on, tip, dimmed } => {
                    let hit = ui.interact(
                        cell,
                        copy_id(ui.ctx(), (spec.id, "toggle", row.key, c)),
                        Sense::click_and_drag(),
                    );
                    let shown = match &state.sweep {
                        Some((col, value, keys)) if *col == c && keys.contains(&row.key) => *value,
                        _ => *on,
                    };
                    paint_toggle(ui, &painter, cell, column.heading, shown, *dimmed);
                    if hit.clicked() {
                        events.push(TreeEvent::Toggle {
                            key: row.key,
                            column: c,
                            solo: mods.alt,
                        });
                    } else if hit.drag_started() {
                        state.sweep = Some((c, !on, vec![row.key]));
                    }
                    if !tip.is_empty() {
                        hit.on_hover_text(tip.as_str());
                    }
                }
                Cell::Custom => hooks.cell(ui, c, row.key, cell),
            }
        }
        // Sweeping: rows the pointer passes join the sweep.
        if let (Some((_, _, keys)), Some(p)) = (&mut state.sweep, pointer)
            && rect.y_range().contains(p.y)
            && !keys.contains(&row.key)
        {
            keys.push(row.key);
        }
        // Name clicks: select, slow second click renames, double-click.
        if response.clicked() && !renaming && !right_click {
            let how = ClickHow::from_modifiers(mods);
            let slow_again = row.selected != Selected::No
                && how == ClickHow::Only
                && row.renamable
                && state
                    .last_click
                    .is_some_and(|(k, t)| k == row.key && now - t >= delay && now - t < 4.);
            if slow_again {
                state.start_rename(row.key, &row.name);
            } else {
                events.push(TreeEvent::Click { key: row.key, how });
            }
            state.last_click = Some((row.key, now));
            if how != ClickHow::Range {
                state.anchor = Some(row.key);
            }
        }
        if response.double_clicked() {
            state.rename = None;
            state.last_click = None;
            events.push(TreeEvent::DoubleClick(row.key));
        }
        if response.drag_started() && row.draggable && state.sweep.is_none() {
            let keys = if row.selected != Selected::No {
                selected_order.clone()
            } else {
                vec![row.key]
            };
            state.drag = Some(keys);
        }
        if let Some(tip) = &row.tip {
            response.clone().on_hover_text(tip.as_str());
        }
        if hooks.has_row_menu(row.key) {
            context_menu_with(&response, right_click, |ui| {
                hooks.row_menu(ui, row.key, &mut || state.start_rename(row.key, &row.name));
            });
        }
        // Where a drag would land on this row.
        if let (Some(dragged), Some(p)) = (&state.drag, pointer)
            && rect.contains(p)
        {
            let t = (p.y - rect.top()) / rect.height();
            // Loops (into a dragged row's own branch) show nothing.
            drop = drop_zone(rows, index, t, dragged).map(|(at, depth)| {
                let x = name_cell.left() + PAD + depth as f32 * INDENT;
                // The line runs along this row's top for "before it",
                // else along its bottom (after it, or before its first
                // child, the row below).
                let y = if at == DropAt::Before(row.key) {
                    rect.top()
                } else {
                    rect.bottom()
                };
                (at, rect, x, y)
            });
        }
        // Lines down every column boundary (drawn per row; the empty space
        // below gets them too, UIC-136).
        let _ = response;
    }
    // Empty space below the rows: drop at the end, the general menu.
    let used = top + rows.len() as f32 * ROW;
    let empty = Rect::from_min_max(pos2(left, used), pos2(right, whole.bottom()));
    if let (Some(_), Some(p)) = (&state.drag, pointer)
        && empty.contains(p)
    {
        // The end of the top level: indented like the shallowest movable
        // row (items sit one level under their scene's row).
        let depth = rows
            .iter()
            .filter(|r| !r.fixed)
            .map(|r| r.depth)
            .min()
            .unwrap_or(0);
        let x = name_span.min + PAD + depth as f32 * INDENT;
        drop = Some((
            DropAt::End,
            Rect::from_min_size(empty.min, vec2(right - left, 0.)),
            x,
            used,
        ));
    }
    if hooks.has_empty_menu() && pointer.is_some_and(|p| empty.contains(p)) {
        context_menu_with(&background, right_click, |ui| hooks.empty_menu(ui));
    }
    // Column lines for the whole height of the panel.
    for span in &spans[..spans.len().saturating_sub(1)] {
        ui.painter().vline(
            span.max,
            whole.y_range(),
            Stroke::new(1_f32, palette::separator()),
        );
    }
    // Drop feedback: an insert line between rows, an outline on a row.
    if let Some((at, rect, x, y)) = drop {
        let stroke = Stroke::new(2_f32, palette::drop_indicator());
        match at {
            DropAt::On(_) => {
                ui.painter()
                    .rect_stroke(rect.shrink(1.), 0., stroke, StrokeKind::Inside);
            }
            DropAt::Before(_) | DropAt::After(_) | DropAt::End => {
                ui.painter().hline(x..=rect.right(), y, stroke);
                // A small tick at the start shows the level it lands at.
                ui.painter().vline(x, y - 3.0..=y + 3., stroke);
            }
        }
        state.drop = Some(at);
    } else if state.drag.is_some() {
        state.drop = None;
    }
    if let Some(keys) = &state.drag
        && let Some(p) = ui.input(|i| i.pointer.hover_pos())
    {
        drag_ghost(ui, spec, rows, keys, p);
    }
    if released {
        if let Some((column, on, keys)) = state.sweep.take()
            && !keys.is_empty()
        {
            events.push(TreeEvent::Sweep { column, keys, on });
        }
        if let Some(keys) = state.drag.take()
            && let Some(at) = state.drop.take()
        {
            events.push(TreeEvent::Drop {
                keys,
                at,
                alt: mods.alt,
            });
        }
        state.drop = None;
    }
}

fn paint_connector(painter: &Painter, name_left: f32, rect: Rect, depth: usize, c: &Connector) {
    if depth == 0 {
        return;
    }
    let stroke = Stroke::new(1_f32, palette::tree_lines());
    let dotted = |a: Pos2, b: Pos2| egui::Shape::dotted_line(&[a, b], stroke.color, 2., 0.5);
    for (i, &on) in c.through.iter().enumerate() {
        if on {
            let x = level_x(name_left, i);
            painter.extend(dotted(pos2(x, rect.top()), pos2(x, rect.bottom())));
        }
    }
    let x = level_x(name_left, depth - 1);
    let mid = rect.center().y;
    let bottom = if c.last { mid } else { rect.bottom() };
    painter.extend(dotted(pos2(x, rect.top()), pos2(x, bottom)));
    let end = name_left + PAD + depth as f32 * INDENT;
    painter.extend(dotted(pos2(x, mid), pos2(end, mid)));
    // A small mark where the line meets the row.
    painter.circle_filled(pos2(x, mid), 1.2, stroke.color);
}

fn paint_toggle(ui: &Ui, painter: &Painter, cell: Rect, heading: Heading, on: bool, dimmed: bool) {
    match heading {
        Heading::Eye => theme::list_eye(ui, cell.shrink(1.), on, dimmed),
        // N18: orange when on, a dim outline of it when off.
        Heading::Icon(i) => {
            let tint = if on {
                palette::active_mesh()
            } else {
                palette::glyphs().gamma_multiply(0.45)
            };
            let tint = if dimmed {
                tint.gamma_multiply(0.5)
            } else {
                tint
            };
            icons::paint_in(ui, cell.shrink(2.5), i, tint);
        }
        Heading::Glyph(g) | Heading::Word(g) => {
            painter.text(
                cell.center(),
                Align2::CENTER_CENTER,
                if on { "●" } else { g },
                FontId::proportional(if on { 9. } else { 10. }),
                if on {
                    theme::chrome::active()
                } else {
                    palette::inactive()
                },
            );
        }
    }
}

/// The colour a row's name is drawn in (UIC-141, UIC-33/34).
pub(crate) fn name_color(row_selected: Selected, style: NameStyle) -> Color32 {
    match row_selected {
        Selected::Primary => accent(true),
        Selected::Yes => accent(false),
        Selected::No if style.dimmed => palette::inactive(),
        Selected::No if style.empty => theme::weak_text(),
        Selected::No => palette::field_text(),
    }
}

/// Draws the name; returns where it went.
/// Returns the drawn rect and whether the name was cut off.
fn paint_name<K>(painter: &Painter, rect: Rect, row: &TreeRow<K>) -> (Rect, bool) {
    let color = name_color(row.selected, row.style);
    let mut text = row.name.clone();
    if row.style.unsaved {
        text.push_str(" *");
    }
    // Names cut off with "…" (UIC-137).
    let mut job = egui::text::LayoutJob::single_section(
        text,
        TextFormat {
            font_id: crate::typography::standard_font(),
            color,
            italics: row.style.italic,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: rect.width().max(1.),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);
    let cut = galley.elided;
    let at = pos2(rect.left(), rect.center().y - galley.size().y / 2.);
    let drawn = Rect::from_min_size(at, galley.size());
    if row.style.bold {
        // The UI font has no bold face: draw it twice, a hair apart.
        painter.galley(at + vec2(0.6, 0.), galley.clone(), color);
    }
    painter.galley(at, galley, color);
    (drawn, cut)
}

/// A faded copy of the dragged rows' first name follows the pointer.
fn drag_ghost<K: Copy + Eq + Hash + Debug + 'static>(
    ui: &Ui,
    spec: &TreeSpec,
    rows: &[TreeRow<K>],
    keys: &[K],
    at: Pos2,
) {
    let Some(first) = rows.iter().find(|r| keys.contains(&r.key)) else {
        return;
    };
    let name = if keys.len() > 1 {
        format!("{} (+{})", first.name, keys.len() - 1)
    } else {
        first.name.clone()
    };
    let icon = first.icon;
    egui::Area::new(copy_id(ui.ctx(), (spec.id, "drag-ghost")))
        .order(egui::Order::Tooltip)
        .fixed_pos(at + vec2(10., 4.))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            ui.set_opacity(0.55);
            Frame::NONE
                .fill(palette::selected())
                .inner_margin(Margin::symmetric(4, 1))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let RowIcon::Icon(i) = icon {
                            let (r, _) = ui.allocate_exact_size(vec2(14., 14.), Sense::hover());
                            icons::paint(ui, r, i);
                        }
                        ui.label(RichText::new(name).color(theme::text()));
                    });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: u32, depth: usize, name: &str) -> TreeRow<u32> {
        TreeRow::new(key, depth, name, 2)
    }

    /// Scene root 1; items 2 (open, children 3 and 4, 4 has child 5), 6.
    fn tree() -> Vec<TreeRow<u32>> {
        let mut root = row(1, 0, "Scene");
        root.fixed = true;
        root.accepts_children = true;
        let mut rows = vec![
            root,
            row(2, 1, "A"),
            row(3, 2, "B"),
            row(4, 2, "C"),
            row(5, 3, "D"),
            row(6, 1, "E"),
        ];
        for r in &mut rows[1..] {
            r.accepts_children = true;
            r.draggable = true;
        }
        rows
    }

    #[test]
    fn drop_zones_top_band_before_middle_on_bottom_band_after() {
        let rows = tree();
        let at = |i: usize, t: f32| drop_zone(&rows, i, t, &[99]);
        // Row 6 (E, last, no children).
        assert_eq!(at(5, 0.1), Some((DropAt::Before(6), 1)));
        assert_eq!(at(5, 0.5), Some((DropAt::On(6), 1)));
        assert_eq!(at(5, 0.9), Some((DropAt::After(6), 1)));
        // Band edges.
        assert_eq!(at(5, 0.24), Some((DropAt::Before(6), 1)));
        assert_eq!(at(5, 0.26), Some((DropAt::On(6), 1)));
        assert_eq!(at(5, 0.76), Some((DropAt::After(6), 1)));
        // A row that takes no children: the nearer half.
        let mut flat = rows.clone();
        flat[5].accepts_children = false;
        assert_eq!(drop_zone(&flat, 5, 0.4, &[]), Some((DropAt::Before(6), 1)));
        assert_eq!(drop_zone(&flat, 5, 0.6, &[]), Some((DropAt::After(6), 1)));
    }

    #[test]
    fn an_open_parent_s_bottom_band_inserts_as_its_first_child() {
        let rows = tree();
        // A is open (B shown under it): bottom band = before B, depth 2.
        assert_eq!(drop_zone(&rows, 1, 0.9, &[6]), Some((DropAt::Before(3), 2)));
        // C is open over D likewise.
        assert_eq!(drop_zone(&rows, 3, 0.9, &[6]), Some((DropAt::Before(5), 3)));
        // The last child D: after it, at its own depth.
        assert_eq!(drop_zone(&rows, 4, 0.9, &[6]), Some((DropAt::After(5), 3)));
    }

    #[test]
    fn a_scene_root_row_is_on_except_its_open_bottom_band() {
        let rows = tree();
        assert_eq!(drop_zone(&rows, 0, 0.1, &[6]), Some((DropAt::On(1), 0)));
        assert_eq!(drop_zone(&rows, 0, 0.5, &[6]), Some((DropAt::On(1), 0)));
        assert_eq!(
            drop_zone(&rows, 0, 0.9, &[6]),
            Some((DropAt::Before(2), 1)),
            "first item of the scene"
        );
    }

    #[test]
    fn drops_into_a_dragged_rows_own_branch_show_nothing() {
        let rows = tree();
        for t in [0.1, 0.5, 0.9] {
            assert_eq!(drop_zone(&rows, 1, t, &[2]), None, "onto itself");
            assert_eq!(drop_zone(&rows, 2, t, &[2]), None, "a child");
            assert_eq!(drop_zone(&rows, 4, t, &[2]), None, "a grandchild");
        }
        // A sibling's branch is fine; so is the row after the branch.
        assert_eq!(drop_zone(&rows, 3, 0.5, &[3]), Some((DropAt::On(4), 2)));
        assert_eq!(drop_zone(&rows, 4, 0.5, &[3]), Some((DropAt::On(5), 3)));
        assert_eq!(drop_zone(&rows, 5, 0.5, &[2]), Some((DropAt::On(6), 1)));
        // Multi-drag: blocked under any dragged row.
        assert_eq!(drop_zone(&rows, 4, 0.5, &[6, 4]), None);
        // The open-parent band of a row just above a dragged branch still
        // works when the parent itself isn't dragged.
        assert_eq!(drop_zone(&rows, 1, 0.9, &[3]), Some((DropAt::Before(3), 2)));
    }

    #[test]
    fn plain_click_selects_only_ctrl_toggles_shift_selects_a_range() {
        let order = [1, 2, 3, 4, 5];
        let (mut sel, mut anchor) = (vec![], None);
        click_select(&mut sel, &mut anchor, &order, 2, ClickHow::Only);
        assert_eq!(sel, [2]);
        click_select(&mut sel, &mut anchor, &order, 4, ClickHow::Toggle);
        assert_eq!(sel, [2, 4], "the rest stays; order is selection order");
        click_select(&mut sel, &mut anchor, &order, 2, ClickHow::Toggle);
        assert_eq!(sel, [4], "Ctrl+click on a selected row removes it");
        // Shift: from the last clicked row (the anchor) to this one.
        click_select(&mut sel, &mut anchor, &order, 1, ClickHow::Range);
        assert_eq!(sel, [2, 1], "anchor 2 (last clicked) stays primary");
        click_select(&mut sel, &mut anchor, &order, 5, ClickHow::Range);
        assert_eq!(sel, [2, 3, 4, 5]);
        click_select(&mut sel, &mut anchor, &order, 3, ClickHow::Only);
        assert_eq!(sel, [3]);
        assert_eq!(anchor, Some(3));
    }

    #[test]
    fn modifiers_map_to_click_kinds_and_cmd_or_ctrl_toggles() {
        assert_eq!(ClickHow::from_modifiers(Modifiers::NONE), ClickHow::Only);
        assert_eq!(ClickHow::from_modifiers(Modifiers::SHIFT), ClickHow::Range);
        assert_eq!(ClickHow::from_modifiers(Modifiers::CTRL), ClickHow::Toggle);
        // Jason, 2026-11-08: Cmd toggles too on a Mac (egui sets `mac_cmd`
        // and `command` for Cmd, `ctrl` only for the physical Ctrl key).
        assert_eq!(
            ClickHow::from_modifiers(Modifiers::MAC_CMD),
            ClickHow::Toggle
        );
        assert_eq!(
            ClickHow::from_modifiers(Modifiers::COMMAND),
            ClickHow::Toggle
        );
        assert_eq!(
            ClickHow::from_modifiers(Modifiers::MAC_CMD | Modifiers::COMMAND),
            ClickHow::Toggle
        );
        // Opt (Alt) is a plain click; Shift wins over Cmd (a range).
        assert_eq!(ClickHow::from_modifiers(Modifiers::ALT), ClickHow::Only);
        assert_eq!(
            ClickHow::from_modifiers(Modifiers::SHIFT | Modifiers::MAC_CMD),
            ClickHow::Range
        );
    }

    /// Original-compatible A7: on a Mac with the switch on, Ctrl+click is a
    /// right-click and only Cmd toggles; off a Mac, or with it off, Ctrl
    /// toggles as before.
    #[test]
    fn a7_ctrl_click_is_a_right_click_only_on_a_mac_when_original_compatible() {
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        assert!(ctrl_click_is_right_click(Modifiers::CTRL, true, true));
        assert!(ctrl_click_is_right_click(
            Modifiers::CTRL | Modifiers::SHIFT,
            true,
            true
        ));
        assert!(!ctrl_click_is_right_click(cmd, true, true));
        assert!(!ctrl_click_is_right_click(
            Modifiers::CTRL | cmd,
            true,
            true
        ));
        assert!(!ctrl_click_is_right_click(Modifiers::NONE, true, true));
        assert!(!ctrl_click_is_right_click(Modifiers::CTRL, true, false));
        assert!(!ctrl_click_is_right_click(Modifiers::CTRL, false, true));
        // Cmd still toggles.
        assert_eq!(ClickHow::from_modifiers(cmd), ClickHow::Toggle);
    }

    #[test]
    fn wildcards_match_whole_names() {
        let m = |p: &str, t: &str| wildcard_match(p, t);
        assert!(m("mesh*", "mesh 2"));
        assert!(m("*2", "mesh 2"));
        assert!(m("m?sh", "mesh"));
        assert!(!m("m?sh", "meesh"));
        assert!(!m("mesh", "mesh 2"), "no star: the whole name");
        assert!(m("*", ""));
        assert!(m("a*b*c", "aXXbYYc"));
    }

    #[test]
    fn search_is_case_blind_by_default_and_match_case_respects_it() {
        let mut s = Search {
            open: true,
            text: "Cube".into(),
            ..Default::default()
        };
        assert!(s.matches("my cube"));
        s.match_case = true;
        assert!(!s.matches("my cube"));
        assert!(s.matches("my Cube"));
        s.wildcard = true;
        assert!(!s.matches("my Cube"), "pattern matches the whole name");
        s.text = "*Cube".into();
        assert!(s.matches("my Cube"));
        // Closed: everything shows.
        s.open = false;
        assert!(!s.active());
    }

    #[test]
    fn search_keeps_ancestors_dimmed_or_shows_a_flat_list() {
        let rows = vec![
            row(1, 0, "Scene"),
            row(2, 1, "Group"),
            row(3, 2, "Cube"),
            row(4, 1, "Sphere"),
            row(5, 2, "Cube 2"),
            row(6, 0, "Other"),
        ];
        let search = Search {
            open: true,
            text: "cube".into(),
            ..Default::default()
        };
        let shown = filter_rows(&rows, &search);
        let keys: Vec<_> = shown.iter().map(|r| (r.key, r.style.dimmed)).collect();
        assert_eq!(
            keys,
            [(1, true), (2, true), (3, false), (4, true), (5, false)]
        );
        let flat = filter_rows(
            &rows,
            &Search {
                flat: true,
                ..search.clone()
            },
        );
        assert_eq!(
            flat.iter().map(|r| (r.key, r.depth)).collect::<Vec<_>>(),
            [(3, 0), (5, 0)]
        );
        // Turning the search off restores the full list.
        let off = Search {
            open: false,
            ..search
        };
        assert_eq!(filter_rows(&rows, &off).len(), rows.len());
    }

    #[test]
    fn connectors_run_through_while_an_ancestor_has_later_siblings() {
        // 0 A
        // 1   B
        // 2     C
        // 1   D
        // 2     E
        // 0 F
        let c = connectors(&[0, 1, 2, 1, 2, 0]);
        assert_eq!(
            c[0],
            Connector {
                through: vec![],
                last: false
            }
        );
        assert_eq!(
            c[1],
            Connector {
                through: vec![],
                last: false
            },
            "D follows B"
        );
        assert_eq!(
            c[2],
            Connector {
                through: vec![true],
                last: true
            },
            "B's line goes on to D"
        );
        assert_eq!(
            c[3],
            Connector {
                through: vec![],
                last: true
            }
        );
        assert_eq!(
            c[4],
            Connector {
                through: vec![false],
                last: true
            },
            "D is last"
        );
        assert!(c[5].last);
    }

    #[test]
    fn name_takes_the_spare_width_and_never_less_than_its_minimum() {
        let spec = TreeSpec {
            id: "t",
            columns: vec![
                Column::state(Heading::Eye, ""),
                Column::name(),
                Column::value("%", 40., ""),
            ],
            own_search: false,
            last_fills: false,
        };
        spec.check_shape().unwrap();
        let spans = column_spans(&spec, &HashMap::new(), 0., 300.);
        assert_eq!(spans[0], Rangef::new(0., 20.));
        assert_eq!(spans[1], Rangef::new(20., 260.));
        assert_eq!(spans[2], Rangef::new(260., 300.));
        // Too narrow: Name keeps its minimum and the list scrolls sideways.
        let narrow = column_spans(&spec, &HashMap::new(), 0., 100.);
        assert_eq!(narrow[1].span(), NAME_MIN);
        assert!(narrow[2].max > 100.);
        // A dragged width sticks.
        let mut widths = HashMap::new();
        widths.insert(2, 70.);
        assert_eq!(column_spans(&spec, &widths, 0., 300.)[2].span(), 70.);
    }

    /// Command History's lists (UIC-137, UIC-41): the last column fills
    /// the panel, Name's divider drags well left of the spare width, and
    /// a narrow panel shrinks the last column instead of hiding its text
    /// past the edge.
    #[test]
    fn a_filling_lists_last_column_takes_the_rest_and_name_drags_freely() {
        let spec = TreeSpec {
            id: "t",
            columns: vec![
                Column::name(),
                Column::value("Source", 60., ""),
                Column::value("Result", 140., ""),
            ],
            own_search: false,
            last_fills: true,
        };
        spec.check_shape().unwrap();
        // Opens as before: Name takes the spare width beside the starting widths.
        let wide = column_spans(&spec, &HashMap::new(), 0., 400.);
        assert_eq!(wide[0], Rangef::new(0., 200.));
        assert_eq!(wide[2], Rangef::new(260., 400.));
        // Name dragged far left: the last column takes what it gave up.
        let mut widths = HashMap::new();
        widths.insert(0, 50.);
        let dragged = column_spans(&spec, &widths, 0., 400.);
        assert_eq!(dragged[0].span(), 50.);
        assert_eq!(dragged[2], Rangef::new(110., 400.));
        // ...but never below its small minimum.
        widths.insert(0, 5.);
        assert_eq!(
            column_spans(&spec, &widths, 0., 400.)[0].span(),
            NAME_DRAG_MIN
        );
        // Narrow panel: the last column ends at the panel edge.
        let narrow = column_spans(&spec, &HashMap::new(), 0., 250.);
        assert_eq!(narrow[0].span(), NAME_MIN);
        assert_eq!(narrow[2].max, 250.);
        // Only when even that is too tight does the list scroll sideways.
        let tight = column_spans(&spec, &HashMap::new(), 0., 150.);
        assert_eq!(tight[2].span(), LAST_MIN);
        // Once Name has been dragged, a middle column dragged wider takes
        // from the last one (before that, from Name's spare width).
        let mut widths = HashMap::new();
        widths.insert(1, 100.);
        assert_eq!(column_spans(&spec, &widths, 0., 400.)[0].span(), 160.);
        widths.insert(0, 50.);
        assert_eq!(column_spans(&spec, &widths, 0., 400.)[2].span(), 250.);
        // Other lists keep their look: the last column keeps its width.
        let plain = TreeSpec {
            last_fills: false,
            ..spec
        };
        assert_eq!(column_spans(&plain, &widths, 0., 400.)[2].span(), 140.);
    }

    #[test]
    fn cut_off_value_text_is_reported_so_it_gets_a_tooltip() {
        let ctx = egui::Context::default();
        let mut cut = (false, false);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let font = FontId::proportional(11.);
                let text = "Cannot run: nothing is selected in this mode";
                let narrow = Rect::from_min_size(pos2(0., 0.), vec2(40., ROW));
                let wide = Rect::from_min_size(pos2(0., 20.), vec2(600., ROW));
                cut.0 = value_text(ui, Id::new("n"), narrow, text, font.clone(), Color32::WHITE);
                cut.1 = value_text(ui, Id::new("w"), wide, text, font, Color32::WHITE);
            });
        });
        assert_eq!(cut, (true, false));
    }

    #[test]
    fn spec_shape_rejects_value_before_name_and_word_state_headers() {
        let bad = TreeSpec {
            id: "bad",
            columns: vec![Column::value("#", 20., ""), Column::name()],
            own_search: false,
            last_fills: false,
        };
        assert!(bad.check_shape().is_err());
        let worded = TreeSpec {
            id: "worded",
            columns: vec![
                Column {
                    heading: Heading::Word("Vis"),
                    ..Column::state(Heading::Eye, "")
                },
                Column::name(),
            ],
            own_search: false,
            last_fills: false,
        };
        assert!(worded.check_shape().is_err());
    }

    #[test]
    fn name_colours_carry_selection_and_style() {
        let plain = NameStyle::default();
        assert_eq!(name_color(Selected::Yes, plain), palette::selected_text());
        assert_ne!(
            name_color(Selected::Primary, plain),
            name_color(Selected::Yes, plain),
            "primary is a shade lighter"
        );
        assert_eq!(
            name_color(
                Selected::No,
                NameStyle {
                    dimmed: true,
                    ..plain
                }
            ),
            palette::inactive()
        );
        assert_eq!(
            name_color(
                Selected::No,
                NameStyle {
                    empty: true,
                    ..plain
                }
            ),
            theme::weak_text()
        );
    }
}
