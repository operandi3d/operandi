//! Whole-window layout files (layouts stage 1: LAY-70/71, UIC-190/191).
//!
//! A [`WindowLayout`] describes the whole main window as data: a frame
//! tree of splits (sizes as fractions of their parent, so a layout fits any
//! window), tab groups and grids, with a panel kind *by registry name* in
//! every slot and that panel's own settings. It also carries the layout's
//! name, switcher order and star, the command line to run when it opens,
//! a reserved key map context (LAY-20, unused until stage 5) and an
//! optional icon. Shipped layouts and a user's own are the same format, so
//! adding a built-in layout is adding a file (UIC-190).
//!
//! This crate knows only the frame tree and the view area's conversion to
//! and from [`Layout`]; which names exist and what their settings mean is
//! the app's panel registry (UIC-157).
use crate::{LAYOUT_VERSION, Layout, Pane, Window};
use egui_tiles::{Container, LinearDir, Tile, TileId, Tiles, Tree};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The layout file format's version. A file with any other version is
/// ignored and the shipped default is used (LAY-49, UIC-209).
pub const WINDOW_LAYOUT_VERSION: u32 = 1;

/// Registry names of the panel kinds the view area holds.
pub const VIEW_3D: &str = "3D View";
pub const UV_VIEW: &str = "UV View";

/// The registry name of an empty panel: a slot that shows nothing yet, to
/// fill with the content arrow (layouts stage 2, LAY-74).
pub const EMPTY: &str = "Empty";

/// Whether `kind` is one of the view area's view kinds.
pub fn is_view_kind(kind: &str) -> bool {
    kind == VIEW_3D || kind == UV_VIEW
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dir {
    /// Side by side.
    Row,
    /// Stacked.
    Column,
}

/// One slot: a panel kind by registry name and that panel's settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub kind: String,
    /// The panel's own settings; the app reads them by kind.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub settings: Value,
    /// A view shown alone in its group (Maximise).
    #[serde(default, skip_serializing_if = "is_false")]
    pub maximized: bool,
    /// The named piece this slot shows (Save Viewport As, LAY-72): saving
    /// that piece again updates this slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piece: Option<String>,
}

impl Slot {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            settings: Value::Null,
            maximized: false,
            piece: None,
        }
    }
}

/// A child of a split and its share of the split's length.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    /// Fraction of the split's length (the parts add up to 1).
    pub fraction: f32,
    pub frame: Frame,
}

/// The frame tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    Split {
        dir: Dir,
        /// The dividers don't move (reserved; stage 1 keeps View > Layout's
        /// app-wide lock).
        #[serde(default, skip_serializing_if = "is_false")]
        locked: bool,
        children: Vec<Part>,
    },
    /// A tab group: one slot shows at a time.
    Tabs {
        tabs: Vec<Slot>,
        #[serde(default)]
        active: usize,
        /// Only the header shows (reserved for later stages).
        #[serde(default, skip_serializing_if = "is_false")]
        minimized: bool,
    },
    /// A tab row whose children can carry dock-leaf identities.
    DockTabs { children: Vec<Frame>, active: usize },
    /// Views in rows and columns (the quad view).
    Grid {
        columns: usize,
        /// Each column's and row's fraction.
        #[serde(default)]
        column_fractions: Vec<f32>,
        #[serde(default)]
        row_fractions: Vec<f32>,
        children: Vec<Frame>,
    },
    /// Version 1 compatibility wrapper for the shipped three-column layout.
    ViewArea { frame: Box<Frame> },
    /// A dock leaf with a stable identity and its own panel or tab settings.
    DockPanel { pane: DockPanel, frame: Box<Frame> },
    /// A registry tab with its own dock tile.
    DockTab(Slot),
    /// One panel.
    Panel(Slot),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockPanel {
    Toolbox,
    ToolProperties,
    ToolTabs,
    Items,
    Properties,
}

impl DockPanel {
    pub fn pane(self) -> Pane {
        match self {
            Self::Toolbox => Pane::Toolbox,
            Self::ToolProperties => Pane::ToolProperties,
            Self::ToolTabs => Pane::ToolTabs,
            Self::Items => Pane::Items,
            Self::Properties => Pane::Properties,
        }
    }

    pub fn of(pane: Pane) -> Option<Self> {
        match pane {
            Pane::Toolbox => Some(Self::Toolbox),
            Pane::ToolProperties => Some(Self::ToolProperties),
            Pane::ToolTabs => Some(Self::ToolTabs),
            Pane::Items => Some(Self::Items),
            Pane::Properties => Some(Self::Properties),
            _ => None,
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Frame {
    /// Every slot in the tree, in order.
    pub fn slots(&self) -> Vec<&Slot> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }
    fn collect<'a>(&'a self, out: &mut Vec<&'a Slot>) {
        match self {
            Frame::Split { children, .. } => children.iter().for_each(|p| p.frame.collect(out)),
            Frame::Tabs { tabs, .. } => out.extend(tabs),
            Frame::Grid { children, .. } | Frame::DockTabs { children, .. } => {
                children.iter().for_each(|f| f.collect(out))
            }
            Frame::ViewArea { frame } | Frame::DockPanel { frame, .. } => frame.collect(out),
            Frame::Panel(slot) | Frame::DockTab(slot) => out.push(slot),
        }
    }
    /// Fractions rounded to four places, so a layout read back from the
    /// window compares equal to the file it came from (LAY-41).
    fn rounded(&mut self) {
        let round = |f: &mut f32| *f = (*f * 10000.).round() / 10000.;
        match self {
            Frame::Split { children, .. } => children.iter_mut().for_each(|p| {
                round(&mut p.fraction);
                p.frame.rounded();
            }),
            Frame::Grid {
                column_fractions,
                row_fractions,
                children,
                ..
            } => {
                column_fractions.iter_mut().for_each(round);
                row_fractions.iter_mut().for_each(round);
                children.iter_mut().for_each(Frame::rounded);
            }
            Frame::ViewArea { frame } | Frame::DockPanel { frame, .. } => frame.rounded(),
            Frame::DockTabs { children, .. } => children.iter_mut().for_each(Frame::rounded),
            Frame::Tabs { .. } | Frame::Panel(_) | Frame::DockTab(_) => {}
        }
    }
}

/// A floating window of views (torn off) and where it opens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatWindow {
    pub frame: Frame,
    pub size: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 2]>,
}

/// One layout file (LAY-71, UIC-191).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowLayout {
    pub version: u32,
    pub name: String,
    /// Place on the switcher bar (lowest first).
    #[serde(default)]
    pub order: i32,
    /// Starred (LAY-21; the stars UI comes in stage 2).
    #[serde(default)]
    pub starred: bool,
    /// A command line run when the layout opens (UIC-221: stored as text;
    /// an unknown line reports an error and does nothing else).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_command: Option<String>,
    /// The key map context's name (LAY-20). Unused: a layout's context is
    /// its own `keys`, named after the layout (provisional, LAY-Q55).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_context: Option<String>,
    /// The layout's key map context (LAY-20, LAY-71, UIC-201): Input Map
    /// action -> chord, in the input map's text form. An empty chord
    /// unbinds the action in this layout. Actions not listed use the
    /// global map. Shipped layouts have none.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub keys: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The named group piece the view area shows (Save Viewport Group As,
    /// LAY-72): saving that piece again updates this layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_piece: Option<String>,
    pub frame: Frame,
    /// The layout's floating dock windows (D-10, UIC-73).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<FloatWindow>,
    /// The toolbox selection when it differs from the open command (UIC-195).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolbox: Option<ToolboxState>,
    /// Set when this file is a palette (layouts stage 6, LAY-50): the
    /// frame is the palette's contents, shown in its own floating window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<PaletteInfo>,
    /// Where the mode bar (Selection, Action Center, Falloff, Symmetry,
    /// snapping…) docks: the top or the bottom of the window, never a side
    /// (Jason, 2026-11-19). Files without it keep it at the top.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mode_bar_bottom: bool,
}

/// Tool set and category names as shown by the app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolboxState {
    pub set: String,
    pub tab: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub uv_tab: bool,
}

/// What makes a layout file a palette (LAY-50, LAY-51).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteInfo {
    /// The palette's id, as `layout.window hash:` names it.
    pub hash: String,
    /// The window's size the first time it opens, in points (LAY-51).
    pub size: [f32; 2],
    /// Older files' "stays above the main window". Read and kept, but every
    /// floating window now stays above the main window (UIC-189).
    #[serde(default, skip_serializing_if = "is_false")]
    pub on_top: bool,
}

impl WindowLayout {
    /// Read a layout file. A file of another version, or one that isn't a
    /// layout, is an error and the caller uses the shipped default.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        match value.get("version").and_then(Value::as_u64) {
            Some(v) if v == u64::from(WINDOW_LAYOUT_VERSION) => {}
            Some(v) => return Err(format!("Layout file version {v} isn't supported")),
            None => return Err("Not a layout file (no version)".into()),
        }
        let layout: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if layout.name.trim().is_empty() {
            return Err("The layout has no name".into());
        }
        Ok(layout)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("layouts always serialise")
    }

    /// The same layout with fractions rounded, for comparing.
    pub fn canonical(&self) -> Self {
        let mut out = self.clone();
        out.frame.rounded();
        for w in &mut out.windows {
            w.frame.rounded();
        }
        out
    }

    /// Whether two layouts are the same arrangement (LAY-41: a user copy is
    /// kept only once it differs from the shipped one).
    pub fn same_as(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }
}

// ------------------------------------------------------------ view area

/// The view area's views read from frames: the [`Layout`] and each view's
/// slot (its settings), by view key.
pub struct ViewsFromFrames {
    pub layout: Layout,
    pub slots: BTreeMap<u32, Slot>,
    /// The piece each keyed slot shows, by key (views and empty panels).
    pub pieces: BTreeMap<u32, String>,
}

/// What [`Layout::graft`] put in: each new view's slot and each new
/// keyed slot's piece, by key.
pub struct Grafted {
    pub slots: BTreeMap<u32, Slot>,
    pub pieces: BTreeMap<u32, String>,
}

impl Layout {
    /// Build the view area from its frame and floating windows. Views get
    /// keys from `first_key` on, so they never meet another layout's.
    /// A slot naming a kind the view area can't hold shows as an empty
    /// panel with its name (LAY-49).
    pub fn from_frames(
        centre: &Frame,
        windows: &[FloatWindow],
        first_key: u32,
    ) -> Result<ViewsFromFrames, String> {
        Self::build_frames(centre, windows, first_key, false)
    }

    pub fn from_dock_frames(
        frame: &Frame,
        windows: &[FloatWindow],
        first_key: u32,
    ) -> Result<ViewsFromFrames, String> {
        Self::build_frames(frame, windows, first_key, true)
    }

    fn build_frames(
        centre: &Frame,
        windows: &[FloatWindow],
        first_key: u32,
        dock: bool,
    ) -> Result<ViewsFromFrames, String> {
        let mut build = Build {
            next_key: first_key,
            slots: BTreeMap::new(),
            missing: BTreeMap::new(),
            dock_tabs: BTreeMap::new(),
            pieces: BTreeMap::new(),
            maximized: None,
            dock,
        };
        let mut tiles = Tiles::default();
        let root = build.frame(&mut tiles, centre)?;
        let main_maximized = build.maximized.take();
        let main = Tree::new("operandi-views", root, tiles);
        let mut floats = Vec::new();
        for (i, w) in windows.iter().enumerate() {
            let id = i as u32 + 1;
            let mut tiles = Tiles::default();
            let root = build.frame(&mut tiles, &w.frame)?;
            floats.push(Window {
                id,
                tree: Tree::new(egui::Id::new(("operandi-window-layout", id)), root, tiles),
                size: w.size,
                maximized: build.maximized.take(),
                position: w.position,
            });
        }
        let layout = Layout {
            version: LAYOUT_VERSION,
            main,
            next_window: floats.len() as u32 + 1,
            windows: floats,
            next_viewport: build.next_key,
            maximized: main_maximized,
            missing: build.missing,
            dock_tabs: build.dock_tabs,
        };
        layout.validate()?;
        Ok(ViewsFromFrames {
            layout,
            slots: build.slots,
            pieces: build.pieces,
        })
    }

    /// Put `frame` (a saved group, LAY-72) where `target` is: the slot's
    /// place and size now hold the frame's panels, under new keys.
    pub fn graft(&mut self, target: Pane, frame: &Frame) -> Result<Grafted, String> {
        let mut build = Build {
            next_key: self.next_viewport,
            slots: BTreeMap::new(),
            missing: BTreeMap::new(),
            dock_tabs: BTreeMap::new(),
            pieces: BTreeMap::new(),
            maximized: None,
            dock: false,
        };
        let mut trial = self.clone();
        let window = trial.window_of(target);
        let tree = trial
            .tree_of_mut(target)
            .ok_or_else(|| format!("{} isn't in this layout", target.title()))?;
        let at = tree.tiles.find_pane(&target).ok_or("No such panel")?;
        let root = build.frame(&mut tree.tiles, frame)?;
        let tile = tree.tiles.remove(root).ok_or("The piece is empty")?;
        tree.tiles.insert(at, tile);
        trial.next_viewport = build.next_key;
        trial.missing.extend(build.missing);
        if let Some(key) = target.key() {
            trial.missing.remove(&key);
        }
        let stored = match window {
            None => &mut trial.maximized,
            Some(w) => match trial.windows.iter_mut().find(|x| x.id == w) {
                Some(x) => &mut x.maximized,
                None => return Err("No such window".into()),
            },
        };
        if stored.is_some() && *stored == target.view_key() {
            *stored = None;
        }
        trial.validate()?;
        *self = trial;
        Ok(Grafted {
            slots: build.slots,
            pieces: build.pieces,
        })
    }

    /// The view area as frames: the main window's, then each floating
    /// window's. `slot` names each view and gives its settings.
    pub fn to_frames(&self, mut slot: impl FnMut(Pane) -> Slot) -> (Frame, Vec<FloatWindow>) {
        let main = self.tree_frame(&self.main, self.maximized, &mut slot, &mut |_, slot| {
            Frame::Panel(slot)
        });
        let windows = self
            .windows
            .iter()
            .map(|w| FloatWindow {
                frame: self.tree_frame(&w.tree, w.maximized, &mut slot, &mut |_, slot| {
                    Frame::Panel(slot)
                }),
                size: w.size,
                position: w.position,
            })
            .collect();
        (main, windows)
    }

    pub fn to_dock_frames(
        &self,
        mut slot: impl FnMut(Pane) -> Slot,
        mut dock_frame: impl FnMut(DockPanel) -> Frame,
    ) -> (Frame, Vec<FloatWindow>) {
        let mut frame = |pane, slot| match DockPanel::of(pane) {
            Some(pane) => Frame::DockPanel {
                pane,
                frame: Box::new(dock_frame(pane)),
            },
            None => Frame::Panel(slot),
        };
        let main = self.tree_frame(&self.main, self.maximized, &mut slot, &mut frame);
        let windows = self
            .windows
            .iter()
            .map(|window| FloatWindow {
                frame: self.tree_frame(&window.tree, window.maximized, &mut slot, &mut frame),
                size: window.size,
                position: window.position,
            })
            .collect();
        (main, windows)
    }

    fn tree_frame(
        &self,
        tree: &Tree<Pane>,
        maximized: Option<u32>,
        slot: &mut impl FnMut(Pane) -> Slot,
        frame: &mut impl FnMut(Pane, Slot) -> Frame,
    ) -> Frame {
        let mut make = |pane: Pane| -> Slot {
            let mut s = slot(pane);
            if let Pane::Missing(key) = pane {
                // The name a file gave it, whatever the app called it.
                s.kind = self
                    .missing
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| "Missing".into());
            }
            s.maximized = maximized.is_some() && pane.view_key() == maximized;
            s
        };
        match tree.root {
            Some(root) => tile_frame(&tree.tiles, root, &mut make, &mut |pane, slot| {
                if let Pane::DockTab(key) = pane {
                    return Frame::DockTab(self.dock_tabs.get(&key).cloned().unwrap_or(slot));
                }
                frame(pane, slot)
            }),
            None => Frame::Panel(Slot::new(VIEW_3D)),
        }
    }
}

fn tile_frame(
    tiles: &Tiles<Pane>,
    id: TileId,
    slot: &mut impl FnMut(Pane) -> Slot,
    frame: &mut impl FnMut(Pane, Slot) -> Frame,
) -> Frame {
    match tiles.get(id) {
        Some(Tile::Pane(pane)) => frame(*pane, slot(*pane)),
        Some(Tile::Container(Container::Linear(linear))) => {
            let children: Vec<TileId> = linear
                .children
                .iter()
                .copied()
                .filter(|&c| tiles.is_visible(c))
                .collect();
            let total: f32 = children.iter().map(|&c| linear.shares[c]).sum();
            let total = if total > 0. { total } else { 1. };
            Frame::Split {
                dir: match linear.dir {
                    LinearDir::Horizontal => Dir::Row,
                    LinearDir::Vertical => Dir::Column,
                },
                locked: false,
                children: children
                    .iter()
                    .map(|&c| Part {
                        fraction: linear.shares[c] / total,
                        frame: tile_frame(tiles, c, slot, frame),
                    })
                    .collect(),
            }
        }
        Some(Tile::Container(Container::Tabs(tabs))) => {
            let children = &tabs.children;
            let active = tabs
                .active
                .and_then(|a| children.iter().position(|&c| c == a))
                .unwrap_or(0);
            if children.iter().any(|&id| match tiles.get(id) {
                Some(Tile::Pane(pane)) => {
                    DockPanel::of(*pane).is_some() || matches!(pane, Pane::DockTab(_))
                }
                Some(Tile::Container(_)) => true,
                None => false,
            }) {
                return Frame::DockTabs {
                    children: children
                        .iter()
                        .map(|&id| tile_frame(tiles, id, slot, frame))
                        .collect(),
                    active,
                };
            }
            // Tabs of views hold panes; a nested container in a tab keeps
            // its first pane (egui_tiles allows it; the app never makes it).
            let slots: Vec<Slot> = children
                .iter()
                .filter_map(|&c| first_pane(tiles, c))
                .map(&mut *slot)
                .collect();
            Frame::Tabs {
                tabs: slots,
                active,
                minimized: false,
            }
        }
        Some(Tile::Container(Container::Grid(grid))) => {
            let children: Vec<TileId> = grid.children().copied().collect();
            let columns = match grid.layout {
                egui_tiles::GridLayout::Columns(n) => n.max(1),
                egui_tiles::GridLayout::Auto => {
                    (children.len() as f32).sqrt().ceil().max(1.) as usize
                }
            };
            let norm = |v: &[f32]| {
                let total: f32 = v.iter().sum();
                if total > 0. {
                    v.iter().map(|x| x / total).collect()
                } else {
                    Vec::new()
                }
            };
            Frame::Grid {
                columns,
                column_fractions: norm(&grid.col_shares),
                row_fractions: norm(&grid.row_shares),
                children: children
                    .iter()
                    .map(|&c| tile_frame(tiles, c, slot, frame))
                    .collect(),
            }
        }
        None => Frame::Panel(Slot::new("Missing")),
    }
}

fn first_pane(tiles: &Tiles<Pane>, id: TileId) -> Option<Pane> {
    match tiles.get(id)? {
        Tile::Pane(p) => Some(*p),
        Tile::Container(c) => c.children().find_map(|&c| first_pane(tiles, c)),
    }
}

struct Build {
    next_key: u32,
    slots: BTreeMap<u32, Slot>,
    missing: BTreeMap<u32, String>,
    dock_tabs: BTreeMap<u32, Slot>,
    pieces: BTreeMap<u32, String>,
    maximized: Option<u32>,
    dock: bool,
}

impl Build {
    fn pane(&mut self, slot: &Slot) -> Pane {
        let key = self.next_key;
        self.next_key += 1;
        let pane = match slot.kind.as_str() {
            VIEW_3D => Pane::Viewport(key),
            UV_VIEW => Pane::Uv(key),
            other => {
                self.missing.insert(key, other.to_owned());
                Pane::Missing(key)
            }
        };
        if let Some(piece) = &slot.piece {
            self.pieces.insert(key, piece.clone());
        }
        if pane.view_key().is_some() {
            if slot.maximized {
                self.maximized = Some(key);
            }
            self.slots.insert(key, slot.clone());
        }
        pane
    }

    fn frame(&mut self, tiles: &mut Tiles<Pane>, frame: &Frame) -> Result<TileId, String> {
        Ok(match frame {
            Frame::Panel(slot) => tiles.insert_pane(self.pane(slot)),
            Frame::DockTab(slot) if self.dock => {
                let key = self.next_key;
                self.next_key += 1;
                self.dock_tabs.insert(key, slot.clone());
                tiles.insert_pane(Pane::DockTab(key))
            }
            Frame::DockTab(_) => return Err("A view area cannot contain a dock tab".into()),
            Frame::DockPanel { pane, .. } if self.dock => tiles.insert_pane(pane.pane()),
            Frame::DockPanel { .. } | Frame::ViewArea { .. } => {
                return Err("A view area cannot contain a dock leaf".into());
            }

            Frame::Split { dir, children, .. } => {
                if children.is_empty() {
                    return Err("A split has no parts".into());
                }
                let mut ids = Vec::new();
                for part in children {
                    ids.push(self.frame(tiles, &part.frame)?);
                }
                let mut linear = egui_tiles::Linear::new(
                    match dir {
                        Dir::Row => LinearDir::Horizontal,
                        Dir::Column => LinearDir::Vertical,
                    },
                    ids.clone(),
                );
                for (id, part) in ids.iter().zip(children) {
                    let f = if part.fraction.is_finite() && part.fraction > 0. {
                        part.fraction
                    } else {
                        1. / children.len() as f32
                    };
                    linear.shares.set_share(*id, f);
                }
                tiles.insert_container(linear)
            }
            Frame::Tabs { tabs, active, .. } => {
                if tabs.is_empty() {
                    return Err("A tab group has no tabs".into());
                }
                let ids: Vec<TileId> = tabs
                    .iter()
                    .map(|s| tiles.insert_pane(self.pane(s)))
                    .collect();
                let mut group = egui_tiles::Tabs::new(ids.clone());
                group.set_active(ids[(*active).min(ids.len() - 1)]);
                tiles.insert_container(group)
            }
            Frame::DockTabs { children, active } => {
                if children.is_empty() {
                    return Err("A tab group has no tabs".into());
                }
                let ids: Vec<_> = children
                    .iter()
                    .map(|f| self.frame(tiles, f))
                    .collect::<Result<_, _>>()?;
                let mut group = egui_tiles::Tabs::new(ids.clone());
                group.set_active(ids[(*active).min(ids.len() - 1)]);
                tiles.insert_container(group)
            }
            Frame::Grid {
                columns,
                column_fractions,
                row_fractions,
                children,
            } => {
                if children.is_empty() {
                    return Err("A grid has no cells".into());
                }
                let mut ids = Vec::new();
                for f in children {
                    ids.push(self.frame(tiles, f)?);
                }
                let mut grid = egui_tiles::Grid::new(ids);
                grid.layout = egui_tiles::GridLayout::Columns((*columns).max(1));
                grid.col_shares = column_fractions.clone();
                grid.row_shares = row_fractions.clone();
                tiles.insert_container(grid)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(kind: &str) -> Frame {
        Frame::Panel(Slot::new(kind))
    }

    #[test]
    fn views_round_trip_through_frames() {
        let centre = Frame::Split {
            dir: Dir::Row,
            locked: false,
            children: vec![
                Part {
                    fraction: 0.25,
                    frame: view(UV_VIEW),
                },
                Part {
                    fraction: 0.75,
                    frame: view(VIEW_3D),
                },
            ],
        };
        let built = Layout::from_frames(&centre, &[], 10).unwrap();
        assert_eq!(built.layout.panes(), [Pane::Uv(10), Pane::Viewport(11)]);
        assert_eq!(built.slots.len(), 2);
        let (back, windows) = built.layout.to_frames(|p| {
            Slot::new(if matches!(p, Pane::Uv(_)) {
                UV_VIEW
            } else {
                VIEW_3D
            })
        });
        assert!(windows.is_empty());
        assert_eq!(back, centre);
    }

    #[test]
    fn an_unknown_kind_becomes_a_named_empty_panel() {
        let centre = Frame::Split {
            dir: Dir::Column,
            locked: false,
            children: vec![
                Part {
                    fraction: 0.5,
                    frame: view(VIEW_3D),
                },
                Part {
                    fraction: 0.5,
                    frame: view("Schematic"),
                },
            ],
        };
        let built = Layout::from_frames(&centre, &[], 0).unwrap();
        assert_eq!(built.layout.views(), [0]);
        assert_eq!(
            built.layout.missing.get(&1).map(String::as_str),
            Some("Schematic")
        );
        assert_eq!(built.layout.title_of(Pane::Missing(1)), "Schematic");
        let (back, _) = built.layout.to_frames(|_| Slot::new(VIEW_3D));
        assert_eq!(back, centre, "the name is kept for the next save");
    }

    #[test]
    fn a_group_grafts_into_a_slot_under_new_keys() {
        let mut built = Layout::from_frames(&view(EMPTY), &[], 0).unwrap().layout;
        let mut piece = Slot::new(VIEW_3D);
        piece.piece = Some("Front".into());
        let group = Frame::Split {
            dir: Dir::Row,
            locked: false,
            children: vec![
                Part {
                    fraction: 0.5,
                    frame: Frame::Panel(piece),
                },
                Part {
                    fraction: 0.5,
                    frame: view(UV_VIEW),
                },
            ],
        };
        let grafted = built.graft(Pane::Missing(0), &group).unwrap();
        assert_eq!(built.panes(), [Pane::Viewport(1), Pane::Uv(2)]);
        assert!(built.missing.is_empty());
        assert_eq!(grafted.slots.len(), 2);
        assert_eq!(grafted.pieces.get(&1).map(String::as_str), Some("Front"));
        assert_eq!(built.new_viewport_key(), 3);
        // A target that isn't there changes nothing.
        let before = built.panes();
        assert!(built.graft(Pane::Missing(9), &group).is_err());
        assert_eq!(built.panes(), before);
    }

    #[test]
    fn a_layout_of_one_empty_panel_builds() {
        // LAY-74: a new blank layout is one empty panel.
        let built = Layout::from_frames(&view(EMPTY), &[], 0).unwrap();
        assert!(built.layout.views().is_empty());
        assert_eq!(built.layout.panes(), [Pane::Missing(0)]);
        assert_eq!(built.layout.title_of(Pane::Missing(0)), EMPTY);
        let (back, _) = built.layout.to_frames(|_| Slot::new(VIEW_3D));
        assert_eq!(back, view(EMPTY));
    }

    #[test]
    fn other_file_versions_are_refused() {
        let file = WindowLayout {
            version: WINDOW_LAYOUT_VERSION,
            name: "Model".into(),
            order: 0,
            starred: true,
            open_command: Some("operandi.toolbox.tab create".into()),
            key_context: None,
            keys: BTreeMap::new(),
            icon: None,
            group_piece: None,
            frame: view(VIEW_3D),
            windows: Vec::new(),
            palette: None,
            toolbox: None,
            mode_bar_bottom: false,
        };
        let text = file.to_json();
        assert_eq!(WindowLayout::from_json(&text).unwrap(), file);
        let newer = text.replace("\"version\": 1", "\"version\": 2");
        assert!(WindowLayout::from_json(&newer).is_err());
        assert!(WindowLayout::from_json("{}").is_err());
        assert!(WindowLayout::from_json("not json").is_err());
    }

    #[test]
    fn grids_tabs_and_floating_windows_round_trip() {
        let centre = Frame::Grid {
            columns: 2,
            column_fractions: vec![0.25, 0.75],
            row_fractions: vec![1.],
            children: vec![
                Frame::Tabs {
                    tabs: vec![Slot::new(VIEW_3D), Slot::new(UV_VIEW)],
                    active: 1,
                    minimized: false,
                },
                view(VIEW_3D),
            ],
        };
        let floats = vec![FloatWindow {
            frame: view(UV_VIEW),
            size: [320., 240.],
            position: Some([10., 20.]),
        }];
        let built = Layout::from_frames(&centre, &floats, 5).unwrap();
        assert_eq!(centre.slots().len(), 3);
        assert_eq!(built.layout.views(), [5, 6, 7, 8]);
        let (back, windows) = built.layout.to_frames(|p| {
            Slot::new(if matches!(p, Pane::Uv(_)) {
                UV_VIEW
            } else {
                VIEW_3D
            })
        });
        assert_eq!(back, centre);
        assert_eq!(windows, floats);
    }

    #[test]
    fn empty_frames_are_rejected_and_bad_fractions_are_replaced() {
        let empty = [
            Frame::Split {
                dir: Dir::Row,
                locked: false,
                children: vec![],
            },
            Frame::Tabs {
                tabs: vec![],
                active: 0,
                minimized: false,
            },
            Frame::Grid {
                columns: 0,
                column_fractions: vec![],
                row_fractions: vec![],
                children: vec![],
            },
        ];
        for frame in empty {
            assert!(Layout::from_frames(&frame, &[], 0).is_err());
        }
        let frame = Frame::Split {
            dir: Dir::Column,
            locked: false,
            children: vec![
                Part {
                    fraction: -1.,
                    frame: view(VIEW_3D),
                },
                Part {
                    fraction: f32::NAN,
                    frame: view(UV_VIEW),
                },
            ],
        };
        let built = Layout::from_frames(&frame, &[], 0).unwrap();
        let (back, _) = built.layout.to_frames(|p| {
            Slot::new(if matches!(p, Pane::Uv(_)) {
                UV_VIEW
            } else {
                VIEW_3D
            })
        });
        let Frame::Split { children, .. } = back else {
            panic!("expected split")
        };
        assert_eq!(
            children.iter().map(|p| p.fraction).collect::<Vec<_>>(),
            [0.5, 0.5]
        );
    }

    #[test]
    fn old_window_layout_defaults_and_invalid_names() {
        let old = r#"{"version":1,"name":"Old","frame":{"type":"panel","kind":"3D View"}}"#;
        let layout = WindowLayout::from_json(old).unwrap();
        assert_eq!(layout.order, 0);
        assert!(!layout.starred);
        assert!(layout.windows.is_empty());
        assert_eq!(WindowLayout::from_json(&layout.to_json()).unwrap(), layout);
        assert!(WindowLayout::from_json(&old.replace("Old", "  ")).is_err());
        assert!(WindowLayout::from_json(&old.replace("1,", "0,")).is_err());
    }

    #[test]
    fn rounding_makes_read_back_layouts_compare_equal() {
        let mk = |f: f32| WindowLayout {
            mode_bar_bottom: false,
            version: 1,
            name: "A".into(),
            order: 0,
            starred: false,
            open_command: None,
            key_context: None,
            keys: BTreeMap::new(),
            icon: None,
            group_piece: None,
            frame: Frame::Split {
                dir: Dir::Row,
                locked: false,
                children: vec![
                    Part {
                        fraction: f,
                        frame: view(VIEW_3D),
                    },
                    Part {
                        fraction: 1. - f,
                        frame: view(UV_VIEW),
                    },
                ],
            },
            windows: Vec::new(),
            palette: None,
            toolbox: None,
        };
        assert!(mk(0.169_44).same_as(&mk(0.169_444_4)));
        assert!(!mk(0.2).same_as(&mk(0.3)));
    }
}
