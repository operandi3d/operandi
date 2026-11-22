//! Workspace layout model (architecture D11, feasibility step P06).
//!
//! A layout is the main window's tree of panes in splits and tab groups,
//! plus any panes torn off into their own windows. Panes are identified by
//! [`Pane`], never by the widgets that draw them, so moving, tearing off or
//! docking a pane changes only where it is drawn: the document, its history
//! and each viewport's camera live elsewhere and are untouched.
//!
//! In-window splitting, tabbing and drag-and-drop come from `egui_tiles`;
//! a torn-off pane becomes an egui native viewport (its own OS window).
//! Layouts save and load as versioned JSON, separate from scene files.
use egui_tiles::{Tile, TileId, Tiles, Tree};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod window;
pub use window::*;

/// Bumped when the saved layout format changes; older or newer files are
/// refused and the default layout is used instead.
pub const LAYOUT_VERSION: u32 = 1;

/// One panel or view. Each appears at most once in a layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Pane {
    Toolbox,
    /// A 3D view with its own camera, keyed by this number.
    Viewport(u32),
    Items,
    Properties,
    /// A UV view (spec viewport-split-tearoff section 7: one more panel
    /// kind in the same tree). Its key comes from the same counter as the
    /// 3D views' (`new_viewport_key`), so per-view maps keyed by view number
    /// (active view, rects, maximise) hold both kinds. Added at the end so
    /// saved layouts keep their meaning.
    Uv(u32),
    /// A slot whose panel kind a layout file names but this build doesn't
    /// have (LAY-49, UIC-209): an empty panel showing the name, which
    /// [`Layout::missing`] keeps by this key.
    Missing(u32),
    ToolProperties,
    /// A single registry panel removed from a sidebar tab group.
    DockTab(u32),
    ToolTabs,
}

impl Pane {
    /// The pane's own name. UV views are all "UV" here; a layout numbers
    /// them ("UV", "UV 2") with [`Layout::title_of`].
    pub fn title(self) -> String {
        match self {
            Self::Toolbox => "Toolbox".into(),
            Self::ToolTabs => "Tool Tabs".into(),
            Self::ToolProperties => "Tool Properties".into(),
            Self::Viewport(0) => "Viewport".into(),
            Self::Viewport(n) => format!("Viewport {}", n + 1),
            Self::Items => "Items".into(),
            Self::Properties => "Properties".into(),
            Self::Uv(_) => "UV".into(),
            Self::Missing(_) => "Missing".into(),
            Self::DockTab(_) => "Panel".into(),
        }
    }

    /// The view key of a 3D or UV view; `None` for other panels.
    pub fn view_key(self) -> Option<u32> {
        match self {
            Self::Viewport(n) | Self::Uv(n) => Some(n),
            _ => None,
        }
    }

    /// The key of a keyed pane: a view's, or an empty or missing panel's
    /// (layouts stage 2: those take the corner dot and the content arrow
    /// too). `None` for the fixed side panels.
    pub fn key(self) -> Option<u32> {
        match self {
            Self::Viewport(n) | Self::Uv(n) | Self::Missing(n) => Some(n),
            _ => None,
        }
    }

    /// The same kind of view under another key (a copy of a view is the
    /// same kind as its source, VST-13 / VST-17). Other panels are
    /// returned unchanged.
    pub fn with_key(self, key: u32) -> Self {
        match self {
            Self::Viewport(_) => Self::Viewport(key),
            Self::Uv(_) => Self::Uv(key),
            other => other,
        }
    }
}

/// A torn-off window holding its own pane tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Window {
    pub id: u32,
    pub tree: Tree<Pane>,
    /// Logical size in points.
    pub size: [f32; 2],
    /// The view this window's group shows alone (Maximise), if any.
    #[serde(default)]
    pub maximized: Option<u32>,
    /// Where the window's top-left corner opens on screen (points), when
    /// it was placed (a panel-dot tear-off at the release point). `None`
    /// leaves it to the system.
    #[serde(default)]
    pub position: Option<[f32; 2]>,
}

impl Window {
    /// Dock arrangements with registry or side panels are saved by the layout.
    pub fn has_dock_panels(&self) -> bool {
        panes_of(&self.tree)
            .into_iter()
            .any(|pane| pane.key().is_none())
    }
}

/// One side of a pane: where a docked pane goes beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn dir(self) -> egui_tiles::LinearDir {
        match self {
            Side::Left | Side::Right => egui_tiles::LinearDir::Horizontal,
            Side::Top | Side::Bottom => egui_tiles::LinearDir::Vertical,
        }
    }
    /// Whether the new pane comes before the target (left or above).
    fn before(self) -> bool {
        matches!(self, Side::Left | Side::Top)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub version: u32,
    pub main: Tree<Pane>,
    pub windows: Vec<Window>,
    next_window: u32,
    next_viewport: u32,
    /// The view the main window's group shows alone (Maximise), if any.
    /// Layouts saved before Maximise existed load with none.
    #[serde(default)]
    pub maximized: Option<u32>,
    /// The names of [`Pane::Missing`] slots, by key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub missing: BTreeMap<u32, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dock_tabs: BTreeMap<u32, Slot>,
}

impl Default for Layout {
    /// Toolbox | Viewport | Items above Properties: today's fixed layout.
    fn default() -> Self {
        let mut tiles = Tiles::default();
        let toolbox = tiles.insert_pane(Pane::Toolbox);
        let viewport = tiles.insert_pane(Pane::Viewport(0));
        let items = tiles.insert_pane(Pane::Items);
        let properties = tiles.insert_pane(Pane::Properties);
        let side = tiles.insert_vertical_tile(vec![items, properties]);
        let root = tiles.insert_horizontal_tile(vec![toolbox, viewport, side]);
        Self {
            version: LAYOUT_VERSION,
            main: Tree::new("operandi-main-layout", root, tiles),
            windows: Vec::new(),
            next_window: 1,
            next_viewport: 1,
            maximized: None,
            missing: BTreeMap::new(),
            dock_tabs: BTreeMap::new(),
        }
    }
}

impl Layout {
    /// Just one viewport: the app's view area while its side panels stay
    /// fixed (the first step of adopting the layout).
    pub fn single_viewport() -> Self {
        let mut tiles = Tiles::default();
        let root = tiles.insert_pane(Pane::Viewport(0));
        Self {
            version: LAYOUT_VERSION,
            main: Tree::new("operandi-views", root, tiles),
            windows: Vec::new(),
            next_window: 1,
            next_viewport: 1,
            maximized: None,
            missing: BTreeMap::new(),
            dock_tabs: BTreeMap::new(),
        }
    }
}

/// The panes of a tree, in tile order.
fn panes_of(tree: &Tree<Pane>) -> Vec<Pane> {
    let mut panes: Vec<(TileId, Pane)> = tree
        .tiles
        .iter()
        .filter_map(|(&id, tile)| match tile {
            Tile::Pane(pane) => Some((id, *pane)),
            Tile::Container(_) => None,
        })
        .collect();
    panes.sort_by_key(|(id, _)| format!("{id:?}"));
    panes.into_iter().map(|(_, pane)| pane).collect()
}

/// A window title naming its panes.
pub fn panes_title(tree: &Tree<Pane>) -> String {
    panes_of(tree)
        .into_iter()
        .map(Pane::title)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Before `id` leaves a split of three or more, its share goes to its
/// neighbours in proportion to their sizes (all of it to the one neighbour
/// at an end), so panels further along keep their sizes (VST-19). In a
/// split of two the other panel takes the whole place anyway, and a split
/// left with one panel dissolves into its parent (the tree's
/// simplification keeps that panel's place and share).
/// Before `gone` leaves its split: when the split also holds `kept` (or a
/// tile containing it), `gone`'s share moves to that child, so nothing
/// else in the split changes size.
fn hand_share_to(tree: &mut Tree<Pane>, gone: TileId, kept: TileId) {
    let Some(parent) = tree.tiles.parent_of(gone) else {
        return;
    };
    let mut chain = vec![kept];
    let mut at = kept;
    while let Some(up) = tree.tiles.parent_of(at) {
        chain.push(up);
        at = up;
    }
    let Some(Tile::Container(egui_tiles::Container::Linear(linear))) = tree.tiles.get_mut(parent)
    else {
        return;
    };
    let Some(&heir) = linear.children.iter().find(|c| chain.contains(c)) else {
        return;
    };
    let share = linear.shares[heir] + linear.shares[gone];
    linear.shares.set_share(heir, share);
    linear.shares.set_share(gone, 0.);
}

/// Put `new` where `old` sits among `parent`'s children, keeping its
/// position, share or active tab.
fn replace_child(tree: &mut Tree<Pane>, parent: TileId, old: TileId, new: TileId) {
    match tree.tiles.get_mut(parent) {
        Some(Tile::Container(egui_tiles::Container::Linear(linear))) => {
            if let Some(slot) = linear.children.iter_mut().find(|c| **c == old) {
                *slot = new;
            }
            let share = linear.shares[old];
            linear.shares.set_share(new, share);
        }
        Some(Tile::Container(egui_tiles::Container::Tabs(tabs))) => {
            if let Some(slot) = tabs.children.iter_mut().find(|c| **c == old) {
                *slot = new;
            }
            if tabs.active == Some(old) {
                tabs.active = Some(new);
            }
        }
        Some(Tile::Container(container)) => {
            container.remove_child(old);
            container.add_child(new);
        }
        _ => tree.root = Some(new),
    }
}

fn give_share_to_neighbours(tree: &mut Tree<Pane>, id: TileId) {
    let Some(parent) = tree.tiles.parent_of(id) else {
        return;
    };
    let Some(Tile::Container(egui_tiles::Container::Linear(linear))) = tree.tiles.get_mut(parent)
    else {
        return;
    };
    let Some(index) = linear.children.iter().position(|&c| c == id) else {
        return;
    };
    if linear.children.len() < 3 {
        return;
    }
    let share = linear.shares[id];
    let neighbours: Vec<TileId> = [index.checked_sub(1), Some(index + 1)]
        .into_iter()
        .flatten()
        .filter_map(|i| linear.children.get(i).copied())
        .collect();
    let total: f32 = neighbours.iter().map(|&n| linear.shares[n]).sum();
    for &n in &neighbours {
        let part = if total > 0. {
            linear.shares[n] / total
        } else {
            1. / neighbours.len() as f32
        };
        let grown = linear.shares[n] + share * part;
        linear.shares.set_share(n, grown);
    }
    linear.shares.set_share(id, 0.);
}

/// Take a pane out of a tree; returns whether it was there.
fn remove_pane(tree: &mut Tree<Pane>, pane: Pane) -> bool {
    let Some(id) = tree.tiles.find_pane(&pane) else {
        return false;
    };
    give_share_to_neighbours(tree, id);
    tree.remove_recursively(id);
    if tree.root == Some(id) {
        tree.root = None;
    }
    tree.simplify(&egui_tiles::SimplificationOptions {
        all_panes_must_have_tabs: false,
        ..Default::default()
    });
    true
}

/// Lock Divider Dragging: put back every split's sizes from `before` (the
/// tree as it was before it drew), undoing any divider drag or
/// double-click the drawing took. Containers that are new keep theirs.
pub fn restore_dividers(before: &Tree<Pane>, after: &mut Tree<Pane>) {
    use egui_tiles::Container;
    for (id, tile) in after.tiles.iter_mut() {
        let (Tile::Container(now), Some(Tile::Container(old))) = (tile, before.tiles.get(*id))
        else {
            continue;
        };
        match (now, old) {
            (Container::Linear(now), Container::Linear(old)) => now.shares = old.shares.clone(),
            (Container::Grid(now), Container::Grid(old)) => {
                now.col_shares = old.col_shares.clone();
                now.row_shares = old.row_shares.clone();
            }
            _ => {}
        }
    }
}

/// The gap between panels in a split (points); the app's tile behaviour
/// draws with it, so size checks here match what is on screen.
pub const GAP: f32 = 1.;

/// A floating window is never smaller than this (VST-27), whatever it holds.
pub const WINDOW_MIN: [f32; 2] = [200., 160.];

impl Pane {
    /// The smallest a docked panel may be, header included (VST-23,
    /// provisional: VST-Q04). A split that would go below it is refused and
    /// dividers stop there.
    pub fn min_size(self) -> [f32; 2] {
        match self {
            Self::Viewport(_) | Self::Uv(_) | Self::Missing(_) => [120., 90.],
            Self::ToolProperties => [100., 72.],
            Self::ToolTabs => [40., 42.],
            Self::Toolbox | Self::Items | Self::Properties | Self::DockTab(_) => [100., 80.],
        }
    }

    /// Docked toolbox and tab-group content retains the stock column
    /// minimums. Standalone panel splitting uses the VST-23 minimum.
    pub fn dock_min_size(self) -> [f32; 2] {
        match self {
            Self::Toolbox => [200., 180.],
            Self::ToolProperties => [200., 72.],
            Self::Items | Self::Properties => [240., 80.],
            other => other.min_size(),
        }
    }
}

/// Axis 0 (x) for a side-by-side split, 1 (y) for a stacked one.
fn axis(dir: egui_tiles::LinearDir) -> usize {
    match dir {
        egui_tiles::LinearDir::Horizontal => 0,
        egui_tiles::LinearDir::Vertical => 1,
    }
}

/// Whether a panel of `size` (points) splits on `side` with both halves at
/// least its minimum (VST-24). The halves share the panel less one gap.
pub fn split_fits(pane: Pane, size: [f32; 2], side: Side) -> bool {
    let a = axis(side.dir());
    (size[a] - GAP) / 2. >= pane.min_size()[a] - 1e-3
}

/// Whether a panel of `size` splits into a 2 x 2 quad (Split Quad).
pub fn quad_fits(pane: Pane, size: [f32; 2]) -> bool {
    split_fits(pane, size, Side::Right) && split_fits(pane, size, Side::Bottom)
}

/// The columns a grid lays its children out in.
fn grid_columns(grid: &egui_tiles::Grid, count: usize) -> usize {
    match grid.layout {
        egui_tiles::GridLayout::Columns(n) => n.max(1),
        egui_tiles::GridLayout::Auto => (count as f32).sqrt().ceil().max(1.) as usize,
    }
}

/// The smallest a tile may be: a panel's own minimum; a split adds its
/// children's along its axis (plus gaps) and takes the largest across it;
/// a grid does that per column and per row; tabs take their largest.
pub fn tile_min(tree: &Tree<Pane>, id: TileId) -> [f32; 2] {
    use egui_tiles::Container;
    match tree.tiles.get(id) {
        Some(Tile::Pane(pane)) => pane.dock_min_size(),
        Some(Tile::Container(Container::Linear(linear))) => {
            let a = axis(linear.dir);
            let mut out = [0., 0.];
            for (index, &child) in linear.children.iter().enumerate() {
                let min = tile_min(tree, child);
                out[a] += min[a] + if index > 0 { GAP } else { 0. };
                out[1 - a] = f32::max(out[1 - a], min[1 - a]);
            }
            out
        }
        Some(Tile::Container(Container::Grid(grid))) => {
            let children: Vec<TileId> = grid.children().copied().collect();
            let cols = grid_columns(grid, children.len());
            let rows = children.len().div_ceil(cols);
            let mut col_min = vec![0_f32; cols];
            let mut row_min = vec![0_f32; rows];
            for (index, &child) in children.iter().enumerate() {
                let min = tile_min(tree, child);
                col_min[index % cols] = col_min[index % cols].max(min[0]);
                row_min[index / cols] = row_min[index / cols].max(min[1]);
            }
            let gaps = |n: usize| GAP * n.saturating_sub(1) as f32;
            [
                col_min.iter().sum::<f32>() + gaps(cols),
                row_min.iter().sum::<f32>() + gaps(rows),
            ]
        }
        Some(Tile::Container(Container::Tabs(tabs))) => {
            tabs.children.iter().fold([0., 0.], |acc, &child| {
                let min = tile_min(tree, child);
                [acc[0].max(min[0]), acc[1].max(min[1])]
            })
        }
        None => [0., 0.],
    }
}

/// The smallest a window holding `tree` may be: what its panels need, and
/// never below [`WINDOW_MIN`] (VST-27).
pub fn window_min(tree: &Tree<Pane>) -> [f32; 2] {
    let need = tree.root.map_or([0., 0.], |root| tile_min(tree, root));
    [need[0].max(WINDOW_MIN[0]), need[1].max(WINDOW_MIN[1])]
}

/// One line of dividers (a split's children, or a grid's columns or rows):
/// move `new` back toward `old` as little as needed so that no part goes
/// below its minimum, or (when it already was below, a window too small for
/// every minimum) gets any smaller. `available` is the line's length less
/// its gaps. The furthest legal point between the two is kept, so a
/// divider stops right at a neighbour's minimum (VST-25).
fn clamp_line(old: &[f32], new: &mut [f32], mins: &[f32], available: f32) {
    if old.len() != new.len() || old.len() != mins.len() || available <= 0. {
        return;
    }
    let sizes = |shares: &[f32]| -> Vec<f32> {
        let total: f32 = shares.iter().sum();
        if total <= 0. {
            return vec![0.; shares.len()];
        }
        shares.iter().map(|s| s / total * available).collect()
    };
    let before = sizes(old);
    let lerp = |t: f32| -> Vec<f32> {
        old.iter()
            .zip(new.iter())
            .map(|(o, n)| o + (n - o) * t)
            .collect()
    };
    let legal = |shares: &[f32]| {
        sizes(shares)
            .iter()
            .zip(&before)
            .zip(mins)
            .all(|((now, was), min)| *now >= min.min(*was) - 1e-3)
    };
    if legal(new) {
        return;
    }
    let (mut lo, mut hi) = (0_f32, 1_f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if legal(&lerp(mid)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    new.copy_from_slice(&lerp(lo));
}

/// Dividers stop at minimum sizes (VST-20, VST-25): every split whose sizes
/// changed while `after` drew (a divider drag, a double-click) is pulled
/// back just far enough that no panel on either side goes below its
/// minimum; dragging a panel shut can't happen. `before` is the tree as it
/// was before it drew; `after` has laid out, so its tiles' rects are this
/// frame's.
pub fn clamp_dividers(before: &Tree<Pane>, after: &mut Tree<Pane>) {
    use egui_tiles::Container;
    let ids: Vec<TileId> = after.tiles.tile_ids().collect();
    for id in ids {
        let (Some(Tile::Container(now)), Some(Tile::Container(old)), Some(rect)) = (
            after.tiles.get(id),
            before.tiles.get(id),
            after.tiles.rect(id),
        ) else {
            continue;
        };
        match (now, old) {
            (Container::Linear(now), Container::Linear(old))
                if now.children == old.children && now.dir == old.dir =>
            {
                let a = axis(now.dir);
                let length = [rect.width(), rect.height()][a];
                let available = length - GAP * now.children.len().saturating_sub(1) as f32;
                let old_shares: Vec<f32> = old.children.iter().map(|&c| old.shares[c]).collect();
                let mut new_shares: Vec<f32> =
                    now.children.iter().map(|&c| now.shares[c]).collect();
                if old_shares == new_shares {
                    continue;
                }
                let mins: Vec<f32> = now
                    .children
                    .iter()
                    .map(|&c| tile_min(after, c)[a])
                    .collect();
                clamp_line(&old_shares, &mut new_shares, &mins, available);
                let children = now.children.clone();
                if let Some(Tile::Container(Container::Linear(now))) = after.tiles.get_mut(id) {
                    for (child, share) in children.into_iter().zip(new_shares) {
                        now.shares.set_share(child, share);
                    }
                }
            }
            (Container::Grid(now), Container::Grid(old))
                if now.children().eq(old.children())
                    && (now.col_shares != old.col_shares || now.row_shares != old.row_shares) =>
            {
                let children: Vec<TileId> = now.children().copied().collect();
                let cols = grid_columns(now, children.len());
                let rows = children.len().div_ceil(cols);
                let mut col_min = vec![0_f32; cols];
                let mut row_min = vec![0_f32; rows];
                for (index, &child) in children.iter().enumerate() {
                    let min = tile_min(after, child);
                    col_min[index % cols] = col_min[index % cols].max(min[0]);
                    row_min[index / cols] = row_min[index / cols].max(min[1]);
                }
                let (old_cols, old_rows) = (old.col_shares.clone(), old.row_shares.clone());
                let gaps = |n: usize| GAP * n.saturating_sub(1) as f32;
                if let Some(Tile::Container(Container::Grid(now))) = after.tiles.get_mut(id) {
                    clamp_line(
                        &old_cols,
                        &mut now.col_shares,
                        &col_min,
                        rect.width() - gaps(cols),
                    );
                    clamp_line(
                        &old_rows,
                        &mut now.row_shares,
                        &row_min,
                        rect.height() - gaps(rows),
                    );
                }
            }
            _ => {}
        }
    }
}

/// A 2 x 2 grid of four tiles in row order. Its column and row dividers
/// each run the grid's whole height or width, so dragging one moves both
/// rows (or columns): one linked cross.
fn quad_grid(children: Vec<TileId>) -> Tile<Pane> {
    let mut grid = egui_tiles::Grid::new(children);
    grid.layout = egui_tiles::GridLayout::Columns(2);
    Tile::Container(egui_tiles::Container::Grid(grid))
}

/// Put a pane into a tree beside the root's contents, as a split (views
/// have no tab bars to switch tabs with).
fn add_pane(tree: &mut Tree<Pane>, pane: Pane) {
    let id = tree.tiles.insert_pane(pane);
    match tree.root {
        None => tree.root = Some(id),
        Some(root) => tree.root = Some(tree.tiles.insert_horizontal_tile(vec![root, id])),
    }
}

impl Layout {
    /// Every pane in the layout: the main window's, then each window's.
    pub fn panes(&self) -> Vec<Pane> {
        let mut all = panes_of(&self.main);
        for window in &self.windows {
            all.extend(panes_of(&window.tree));
        }
        all
    }

    /// The viewports' camera keys, so each view keeps its own camera.
    pub fn viewports(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .panes()
            .into_iter()
            .filter_map(|p| match p {
                Pane::Viewport(n) => Some(n),
                _ => None,
            })
            .collect();
        ids.sort();
        ids
    }

    /// Every view's key, 3D and UV.
    pub fn views(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .panes()
            .into_iter()
            .filter_map(Pane::view_key)
            .collect();
        ids.sort();
        ids
    }

    /// The view (3D or UV) with this key, if it is in the layout.
    pub fn view_pane(&self, key: u32) -> Option<Pane> {
        self.panes().into_iter().find(|p| p.view_key() == Some(key))
    }

    /// The keyed pane (a view, or an empty or missing panel) with this key.
    pub fn pane_with_key(&self, key: u32) -> Option<Pane> {
        self.panes().into_iter().find(|p| p.key() == Some(key))
    }

    /// Put `new` (a pane not in the layout yet) where `old` is: the slot
    /// keeps its place and size and shows something else (the content
    /// arrow, LAY-73). A maximised `old` leaves Maximise.
    pub fn replace(&mut self, old: Pane, new: Pane) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        if self.panes().contains(&new) {
            return Err(format!("{} is in the layout already", new.title()));
        }
        let window = self.window_of(old);
        let tree = self
            .tree_of_mut(old)
            .ok_or_else(|| format!("{} isn't in this layout", old.title()))?;
        let id = tree.tiles.find_pane(&old).ok_or("No such panel")?;
        if let Some(Tile::Pane(slot)) = tree.tiles.get_mut(id) {
            *slot = new;
        }
        let stored = match window {
            None => &mut self.maximized,
            Some(w) => match self.windows.iter_mut().find(|x| x.id == w) {
                Some(x) => &mut x.maximized,
                None => return Ok(()),
            },
        };
        if old.view_key().is_some() && *stored == old.view_key() {
            *stored = None;
        }
        Ok(())
    }

    /// The window the view with this key is in (`None`: the main window,
    /// or no such view).
    fn view_window(&self, key: u32) -> Option<u32> {
        self.view_pane(key).and_then(|p| self.window_of(p))
    }

    /// A pane's name in this layout: UV views are numbered in key order
    /// ("UV", "UV 2"); everything else is [`Pane::title`].
    pub fn title_of(&self, pane: Pane) -> String {
        if let Pane::DockTab(key) = pane
            && let Some(slot) = self.dock_tabs.get(&key)
        {
            return slot.kind.clone();
        }
        if let Pane::Missing(key) = pane {
            return self
                .missing
                .get(&key)
                .cloned()
                .unwrap_or_else(|| pane.title());
        }
        let Pane::Uv(key) = pane else {
            return pane.title();
        };
        let mut uvs: Vec<u32> = self
            .panes()
            .into_iter()
            .filter_map(|p| match p {
                Pane::Uv(n) => Some(n),
                _ => None,
            })
            .collect();
        uvs.sort();
        match uvs.iter().position(|&n| n == key) {
            Some(0) | None => "UV".into(),
            Some(index) => format!("UV {}", index + 1),
        }
    }

    /// A window's title naming its panes ("Viewport 2, UV").
    pub fn window_title(&self, tree: &Tree<Pane>) -> String {
        panes_of(tree)
            .into_iter()
            .map(|p| self.title_of(p))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The window a pane is in: `None` for the main window.
    pub fn window_of(&self, pane: Pane) -> Option<u32> {
        self.windows
            .iter()
            .find(|w| w.tree.tiles.find_pane(&pane).is_some())
            .map(|w| w.id)
    }

    /// Move a pane from wherever it is into a new window of its own.
    pub fn tear_off(&mut self, pane: Pane, size: [f32; 2]) -> Result<u32, String> {
        if self.window_of(pane).is_none() && panes_of(&self.main).len() <= 1 {
            return Err("The main window needs at least one pane".into());
        }
        let removed = remove_pane(&mut self.main, pane)
            || self
                .windows
                .iter_mut()
                .any(|w| remove_pane(&mut w.tree, pane));
        if !removed {
            return Err(format!("{} isn't in this layout", pane.title()));
        }
        self.windows.retain(|w| w.tree.root.is_some());
        Ok(self.new_window(pane, size, None))
    }

    /// A new window holding `pane` alone (the pane must not be anywhere
    /// else in the layout). Returns its id.
    pub fn new_window(&mut self, pane: Pane, size: [f32; 2], position: Option<[f32; 2]>) -> u32 {
        let mut tiles = Tiles::default();
        let root = tiles.insert_pane(pane);
        self.push_window(tiles, root, size, position)
    }

    fn push_window(
        &mut self,
        tiles: Tiles<Pane>,
        root: TileId,
        size: [f32; 2],
        position: Option<[f32; 2]>,
    ) -> u32 {
        let id = self.next_window;
        self.next_window += 1;
        self.windows.push(Window {
            id,
            tree: Tree::new(egui::Id::new(("operandi-window-layout", id)), root, tiles),
            size,
            maximized: None,
            position,
        });
        id
    }

    /// Float exactly this tile (a pane or a group of tabs). Only the main
    /// window must keep a root; an emptied floating window goes away.
    pub fn float_dock_tile(
        &mut self,
        window: Option<u32>,
        source: TileId,
        size: [f32; 2],
        position: Option<[f32; 2]>,
    ) -> Result<u32, String> {
        let tree = match window {
            Some(id) => {
                &mut self
                    .windows
                    .iter_mut()
                    .find(|w| w.id == id)
                    .ok_or("No floating window")?
                    .tree
            }
            None => &mut self.main,
        };
        if window.is_none() && tree.root == Some(source) {
            return Err("The main window needs at least one panel".into());
        }
        let mut tiles = Tiles::default();
        let root = copy_dock_tile(&tree.tiles, source, &mut tiles)?;
        if tree.root == Some(source) {
            tree.root = None;
        }
        finish_dock(tree, source);
        if let Some(id) = window {
            self.windows.retain(|w| w.id != id || w.tree.root.is_some());
        }
        self.maximized = None;
        Ok(self.push_window(tiles, root, size, position))
    }

    /// Stage a floating window or one of its tabs for insertion into the
    /// main tree. The caller commits this layout only once docking succeeds.
    pub fn import_dock_tile(&mut self, window: u32, tab: Option<TileId>) -> Result<TileId, String> {
        let index = self
            .windows
            .iter()
            .position(|w| w.id == window)
            .ok_or("No floating window")?;
        let tree = &mut self.windows[index].tree;
        let source = tab.or(tree.root).ok_or("No floating panel")?;
        let new = copy_dock_tile(&tree.tiles, source, &mut self.main.tiles)?;
        if tree.root == Some(source) {
            self.windows.remove(index);
        } else {
            finish_dock(tree, source);
        }
        Ok(new)
    }

    pub fn reorder_window_tab(
        &mut self,
        window: u32,
        row: TileId,
        tile: TileId,
        index: usize,
    ) -> Result<(), String> {
        let tree = &mut self
            .windows
            .iter_mut()
            .find(|w| w.id == window)
            .ok_or("No floating window")?
            .tree;
        if reorder_dock_tab(tree, tile, row, index) {
            Ok(())
        } else {
            Err("The source tab moved".into())
        }
    }

    /// Move a pane into a new window of its own, opening at `position`.
    pub fn tear_off_at(
        &mut self,
        pane: Pane,
        size: [f32; 2],
        position: Option<[f32; 2]>,
    ) -> Result<u32, String> {
        if self.alone_in_window(pane) {
            return Err("The panel is alone in its window already".into());
        }
        let id = self.tear_off(pane, size)?;
        if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
            window.position = position;
        }
        Ok(id)
    }

    /// Add a single registry tab as a detached tile, ready for insertion.
    pub fn new_dock_tab(&mut self, slot: Slot) -> (Pane, TileId) {
        let key = self.new_viewport_key();
        self.dock_tabs.insert(key, slot);
        let pane = Pane::DockTab(key);
        (pane, self.main.tiles.insert_pane(pane))
    }

    /// A fresh viewport key, for a copy of a view.
    pub fn new_viewport_key(&mut self) -> u32 {
        let key = self.next_viewport;
        self.next_viewport += 1;
        key
    }

    /// Whether `pane` is the only pane in the main window: it can't be
    /// moved away, torn off or deleted.
    pub fn last_in_main(&self, pane: Pane) -> bool {
        let panes = panes_of(&self.main);
        if pane.key().is_some() {
            panes.into_iter().filter(|p| p.key().is_some()).eq([pane])
        } else {
            panes == [pane]
        }
    }

    /// Whether `pane` is the only pane of a torn-off window.
    pub fn alone_in_window(&self, pane: Pane) -> bool {
        self.windows.iter().any(|w| panes_of(&w.tree) == [pane])
    }

    fn tree_of_mut(&mut self, pane: Pane) -> Option<&mut Tree<Pane>> {
        if self.main.tiles.find_pane(&pane).is_some() {
            return Some(&mut self.main);
        }
        self.windows
            .iter_mut()
            .find(|w| w.tree.tiles.find_pane(&pane).is_some())
            .map(|w| &mut w.tree)
    }

    /// Swap two panes' places; each place keeps its size. The panes may be
    /// in different windows.
    pub fn swap(&mut self, a: Pane, b: Pane) -> Result<(), String> {
        if a == b {
            return Err("A panel can't swap with itself".into());
        }
        for pane in [a, b] {
            if self.tree_of_mut(pane).is_none() {
                return Err(format!("{} isn't in this layout", pane.title()));
            }
        }
        for (from, to) in [
            (a, Pane::Viewport(u32::MAX)),
            (b, a),
            (Pane::Viewport(u32::MAX), b),
        ] {
            let tree = self.tree_of_mut(from).ok_or("Swap lost a panel")?;
            let id = tree.tiles.find_pane(&from).ok_or("Swap lost a panel")?;
            if let Some(Tile::Pane(slot)) = tree.tiles.get_mut(id) {
                *slot = to;
            }
        }
        Ok(())
    }

    /// Put a new pane (one not in the layout yet) beside `target`: the
    /// target's place splits at half on `side` and the new pane takes that
    /// half.
    /// Splitting a maximised target leaves Maximise first (see
    /// [`Layout::leave_maximized_for_split`]).
    pub fn insert_beside(&mut self, pane: Pane, target: Pane, side: Side) -> Result<(), String> {
        if self.panes().contains(&pane) {
            return Err(format!("{} is in the layout already", pane.title()));
        }
        if !self.panes().contains(&target) {
            return Err(format!("{} isn't in this layout", target.title()));
        }
        self.leave_maximized_for_split(target);
        let tree = self
            .tree_of_mut(target)
            .ok_or_else(|| format!("{} isn't in this layout", target.title()))?;
        let target_id = tree.tiles.find_pane(&target).ok_or("No target")?;
        let new_id = tree.tiles.insert_pane(pane);
        // A parent split the same way takes the new pane directly, the
        // target's share halved between the two (no nested split that the
        // tree's simplification would merge with other shares).
        if let Some(parent) = tree.tiles.parent_of(target_id)
            && let Some(Tile::Container(egui_tiles::Container::Linear(linear))) =
                tree.tiles.get_mut(parent)
            && linear.dir == side.dir()
            && let Some(index) = linear.children.iter().position(|&c| c == target_id)
        {
            let share = linear.shares[target_id] / 2.;
            linear.shares.set_share(target_id, share);
            linear.shares.set_share(new_id, share);
            let at = if side.before() { index } else { index + 1 };
            linear.children.insert(at, new_id);
            return Ok(());
        }
        // Otherwise the target's tile becomes a two-way split holding the
        // target (moved to a new id) and the new pane, so its parent (or
        // the root) and its share are untouched.
        let moved = tree.tiles.insert_pane(target);
        let children = if side.before() {
            vec![new_id, moved]
        } else {
            vec![moved, new_id]
        };
        tree.tiles.insert(
            target_id,
            Tile::Container(egui_tiles::Container::Linear(egui_tiles::Linear::new(
                side.dir(),
                children,
            ))),
        );
        Ok(())
    }

    /// Move `pane` beside `target` (Dock Beside): the hole it leaves
    /// closes; a window it leaves empty closes too. The main window's last
    /// pane can't leave it.
    pub fn dock_beside(&mut self, pane: Pane, target: Pane, side: Side) -> Result<(), String> {
        if pane == target {
            return Err("A panel can't dock beside itself".into());
        }
        if !self.panes().contains(&target) {
            return Err(format!("{} isn't in this layout", target.title()));
        }
        if self.last_in_main(pane) && self.main.tiles.find_pane(&target).is_some() {
            remove_pane(&mut self.main, pane);
        } else {
            self.remove(pane)?;
        }
        self.insert_beside(pane, target, side)
    }

    /// The tile whose frame surrounds a dock target, including its tab row.
    pub fn dock_slot(&self, pane: Pane) -> Option<TileId> {
        let id = self.main.tiles.find_pane(&pane)?;
        Some(
            self.main
                .tiles
                .parent_of(id)
                .filter(|parent| {
                    matches!(
                        self.main.tiles.get(*parent),
                        Some(Tile::Container(egui_tiles::Container::Tabs(_)))
                    )
                })
                .unwrap_or(id),
        )
    }

    /// The smallest tile of the main window that holds every view (3D and
    /// UV): the view area an editor panel docks under. A view row's tabs
    /// count as one tile.
    pub fn view_area_tile(&self) -> Option<TileId> {
        let tree = &self.main;
        let views: Vec<TileId> = panes_of(tree)
            .into_iter()
            .filter(|p| p.view_key().is_some())
            .filter_map(|p| tree.tiles.find_pane(&p))
            .collect();
        let inside = |mut tile: TileId, area: TileId| loop {
            if tile == area {
                return true;
            }
            match tree.tiles.parent_of(tile) {
                Some(parent) => tile = parent,
                None => return false,
            }
        };
        let mut area = *views.first()?;
        while !views.iter().all(|&v| inside(v, area)) {
            area = tree.tiles.parent_of(area)?;
        }
        if let Some(parent) = tree.tiles.parent_of(area)
            && matches!(
                tree.tiles.get(parent),
                Some(Tile::Container(egui_tiles::Container::Tabs(_)))
            )
        {
            area = parent;
        }
        Some(area)
    }

    /// Dock the tab tile `source` under the view area, taking `share` (0..1)
    /// of the height the view area had (an editor panel's first place).
    pub fn dock_tile_below_views(&mut self, source: TileId, share: f32) -> Result<(), String> {
        let Some(Tile::Pane(pane)) = self.main.tiles.get(source).cloned() else {
            return Err("No dock source".into());
        };
        let area = self
            .view_area_tile()
            .ok_or("No view area in the main window")?;
        self.dock_tile_at(source, area, Side::Bottom, None)?;
        let tree = &mut self.main;
        let tile = tree.tiles.find_pane(&pane).ok_or("The panel moved")?;
        let parent = tree.tiles.parent_of(tile).ok_or("The panel moved")?;
        if let Some(Tile::Container(egui_tiles::Container::Linear(linear))) =
            tree.tiles.get_mut(parent)
            && let Some(at) = linear.children.iter().position(|&c| c == tile)
            && let Some(&above) = at.checked_sub(1).and_then(|i| linear.children.get(i))
        {
            let total = linear.shares[above] + linear.shares[tile];
            let share = share.clamp(0.1, 0.9);
            linear.shares.set_share(above, total * (1. - share));
            linear.shares.set_share(tile, total * share);
        }
        Ok(())
    }

    pub fn reveal_docked_tab(&mut self, pane: Pane) {
        let Some(tree) = self.tree_of_mut(pane) else {
            return;
        };
        let Some(id) = tree.tiles.find_pane(&pane) else {
            return;
        };
        let mut child = id;
        while let Some(parent) = tree.tiles.parent_of(child) {
            if let Some(Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                tree.tiles.get_mut(parent)
            {
                tabs.set_active(child);
            }
            child = parent;
        }
        if let Some(window) = self.window_of(pane) {
            if let Some(window) = self.windows.iter_mut().find(|w| w.id == window) {
                window.maximized = None;
            }
        } else {
            self.maximized = None;
        }
    }

    /// Insert against a subtree or between two children of a split. Keep
    /// the target tile alive until after insertion, when the source gap closes.
    pub fn dock_at(
        &mut self,
        pane: Pane,
        target: TileId,
        side: Side,
        divider: Option<usize>,
    ) -> Result<(), String> {
        let source = self.dock_slot(pane).ok_or("No dock source")?;
        self.dock_tile_at(source, target, side, divider)
    }

    /// Dock a tab tile on its own, even when its row has other children.
    pub fn dock_tile_at(
        &mut self,
        source: TileId,
        target: TileId,
        side: Side,
        divider: Option<usize>,
    ) -> Result<(), String> {
        use egui_tiles::{Container, Linear};
        let tree = &mut self.main;
        if dock_contains(tree, source, target) || tree.tiles.get(target).is_none() {
            return Err("No dock target".into());
        }
        if let Some(at) = divider {
            match tree.tiles.get(target) {
                Some(Tile::Container(Container::Grid(grid))) => {
                    let grid = grid.clone();
                    dock_between_grid_lines(tree, source, target, grid, side, at)?;
                    self.maximized = None;
                    return Ok(());
                }
                Some(Tile::Container(Container::Linear(linear)))
                    if linear.dir == side.dir() && (1..linear.children.len()).contains(&at) => {}
                _ => return Err("The divider moved".into()),
            }
        }
        let parent = tree.tiles.parent_of(target);
        let adjacent = parent.and_then(|id| match tree.tiles.get(id) {
            Some(Tile::Container(Container::Linear(linear))) if linear.dir == side.dir() => linear
                .children
                .iter()
                .position(|&child| child == target)
                .map(|i| (id, i)),
            _ => None,
        });
        let new = relocate_dock_tile(tree, source)?;
        if let Some((split, at)) = divider
            .map(|i| (target, i))
            .or_else(|| adjacent.map(|(id, i)| (id, if side.before() { i } else { i + 1 })))
        {
            let Some(Tile::Container(Container::Linear(linear))) = tree.tiles.get_mut(split) else {
                return Err("The divider moved".into());
            };
            let share = if divider.is_some() {
                let (Some(&left), Some(&right)) = (
                    at.checked_sub(1).and_then(|i| linear.children.get(i)),
                    linear.children.get(at),
                ) else {
                    return Err("The divider moved".into());
                };
                let share = linear.shares[left].min(linear.shares[right]) / 2.;
                for neighbour in [left, right] {
                    linear
                        .shares
                        .set_share(neighbour, linear.shares[neighbour] - share / 2.);
                }
                share
            } else {
                let share = linear.shares[target] / 2.;
                linear.shares.set_share(target, share);
                share
            };
            linear.shares.set_share(new, share);
            linear.children.insert(at, new);
        } else {
            let tile = tree.tiles.remove(target).ok_or("The target moved")?;
            let moved = tree.tiles.insert_new(tile);
            let children = if side.before() {
                vec![new, moved]
            } else {
                vec![moved, new]
            };
            tree.tiles.insert(
                target,
                Tile::Container(Container::Linear(Linear::new(side.dir(), children))),
            );
        }
        finish_dock(tree, source);
        self.maximized = None;
        Ok(())
    }

    /// Join the source slot to the destination row, keeping its tab order
    /// and front tab when the grip belongs to a row of tabs.
    pub fn dock_as_tab(&mut self, pane: Pane, target: TileId, at: usize) -> Result<(), String> {
        let source = self.dock_slot(pane).ok_or("No dock source")?;
        self.dock_tile_as_tab(source, target, at)
    }

    pub fn dock_tile_as_tab(
        &mut self,
        source: TileId,
        target: TileId,
        at: usize,
    ) -> Result<(), String> {
        use egui_tiles::{Container, Tabs};
        let tree = &mut self.main;
        if target == source {
            return Ok(());
        }
        if dock_contains(tree, source, target) || tree.tiles.get(target).is_none() {
            return Err("No docking target".into());
        }
        if reorder_dock_tab(tree, source, target, at) {
            return Ok(());
        }
        let (children, active) = match tree.tiles.get(source) {
            Some(Tile::Container(Container::Tabs(tabs))) => (
                tabs.children.clone(),
                tabs.active
                    .or_else(|| tabs.children.first().copied())
                    .ok_or("The source has no tabs")?,
            ),
            Some(_) => {
                let id = relocate_dock_tile(tree, source)?;
                (vec![id], id)
            }
            _ => return Err("No dock source".into()),
        };
        // The new row keeps the tab tiles. Leave a leaf at the old slot
        // until its share has been handed to the neighbours.
        tree.tiles.insert(source, empty_dock_tile());
        if let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get_mut(target) {
            let index = at.min(tabs.children.len());
            tabs.children.splice(index..index, children);
            tabs.set_active(active);
        } else {
            let tile = tree.tiles.remove(target).ok_or("The target moved")?;
            let moved = tree.tiles.insert_new(tile);
            let mut tabs = Tabs::new(vec![moved]);
            tabs.children.extend(children);
            tabs.set_active(active);
            tree.tiles
                .insert(target, Tile::Container(Container::Tabs(tabs)));
        }
        finish_dock(tree, source);
        self.maximized = None;
        Ok(())
    }

    /// Take a pane out of the layout (Delete): its neighbours fill the
    /// space; a window it leaves empty closes.
    pub fn remove(&mut self, pane: Pane) -> Result<(), String> {
        if self.last_in_main(pane) {
            return Err("The main window needs at least one panel".into());
        }
        let removed = remove_pane(&mut self.main, pane)
            || self
                .windows
                .iter_mut()
                .any(|w| remove_pane(&mut w.tree, pane));
        if !removed {
            return Err(format!("{} isn't in this layout", pane.title()));
        }
        self.windows.retain(|w| w.tree.root.is_some());
        Ok(())
    }

    /// Take a floating window out of this layout whole (layouts stage 6,
    /// LAY-52: floating windows belong to the app, so a layout switch
    /// carries them to the next layout). Its panes leave with it.
    pub fn take_window(&mut self, window: u32) -> Option<Window> {
        let index = self.windows.iter().position(|w| w.id == window)?;
        if self.maximized.is_some_and(|m| {
            panes_of(&self.windows[index].tree)
                .iter()
                .any(|p| p.view_key() == Some(m))
        }) {
            self.maximized = None;
        }
        Some(self.windows.remove(index))
    }

    /// Put a floating window taken from another layout into this one,
    /// under a new window id and new keys for its keyed panes (so they
    /// never meet this layout's). Returns the new id and each old key's
    /// new key.
    pub fn adopt_window(&mut self, mut window: Window) -> (u32, BTreeMap<u32, u32>) {
        let mut keys = BTreeMap::new();
        for (_, tile) in window.tree.tiles.iter_mut() {
            if let Tile::Pane(pane) = tile
                && let Some(old) = pane.key()
            {
                let new = *keys.entry(old).or_insert_with(|| {
                    let k = self.next_viewport;
                    self.next_viewport += 1;
                    k
                });
                *pane = match *pane {
                    Pane::Missing(_) => Pane::Missing(new),
                    p => p.with_key(new),
                };
            }
        }
        for (old, new) in &keys {
            if let Some(name) = self.missing.remove(old) {
                self.missing.insert(*new, name);
            }
        }
        window.maximized = window.maximized.and_then(|m| keys.get(&m).copied());
        let id = self.next_window;
        self.next_window += 1;
        window.id = id;
        if let Some(root) = window.tree.root {
            let tiles = std::mem::take(&mut window.tree.tiles);
            window.tree = Tree::new(egui::Id::new(("operandi-window-layout", id)), root, tiles);
        }
        self.windows.push(window);
        (id, keys)
    }

    /// Close a torn-off window and drop its panes; returns them.
    pub fn discard_window(&mut self, window: u32) -> Vec<Pane> {
        let Some(index) = self.windows.iter().position(|w| w.id == window) else {
            return Vec::new();
        };
        panes_of(&self.windows.remove(index).tree)
    }

    /// Add a new pane at the right side of a window's layout (Duplicate):
    /// the window's whole layout and the new pane split side by side.
    pub fn add_at_right(&mut self, pane: Pane, window: Option<u32>) -> Result<(), String> {
        if self.panes().contains(&pane) {
            return Err(format!("{} is in the layout already", pane.title()));
        }
        let tree = match window {
            None => &mut self.main,
            Some(id) => {
                &mut self
                    .windows
                    .iter_mut()
                    .find(|w| w.id == id)
                    .ok_or("No such window")?
                    .tree
            }
        };
        add_pane(tree, pane);
        Ok(())
    }

    /// Split `view` into a 2 x 2 quad in its place: `view` keeps the top
    /// left and `others` (new keys) take the top right, bottom left and
    /// bottom right. The quad's dividers are linked: one cross.
    pub fn split_quad(&mut self, view: u32, others: [u32; 3]) -> Result<(), String> {
        let pane = self
            .view_pane(view)
            .ok_or_else(|| format!("{} isn't in this layout", Pane::Viewport(view).title()))?;
        if others.iter().any(|&o| self.view_pane(o).is_some()) {
            return Err("A quad needs new views".into());
        }
        self.leave_maximized_for_split(pane);
        let tree = self
            .tree_of_mut(pane)
            .ok_or_else(|| format!("{} isn't in this layout", pane.title()))?;
        let at = tree.tiles.find_pane(&pane).ok_or("No view")?;
        let moved = tree.tiles.insert_pane(pane);
        let mut children = vec![moved];
        children.extend(
            others
                .iter()
                .map(|&o| tree.tiles.insert_pane(pane.with_key(o))),
        );
        tree.tiles.insert(at, quad_grid(children));
        Ok(())
    }

    /// Close a torn-off window: its panes go back to the main window, split
    /// beside what is there.
    pub fn dock(&mut self, window: u32) -> Result<(), String> {
        let index = self
            .windows
            .iter()
            .position(|w| w.id == window)
            .ok_or("No such window")?;
        let window = self.windows.remove(index);
        for pane in panes_of(&window.tree) {
            add_pane(&mut self.main, pane);
        }
        Ok(())
    }

    /// Add another viewport (its own camera) as a split beside the root.
    pub fn add_viewport(&mut self) -> Pane {
        let pane = Pane::Viewport(self.next_viewport);
        self.next_viewport += 1;
        let id = self.main.tiles.insert_pane(pane);
        let root = self.main.root;
        let split = self
            .main
            .tiles
            .insert_horizontal_tile(root.into_iter().chain([id]).collect());
        self.main.root = Some(split);
        pane
    }

    /// The viewports of one group: the main window's (`None`) or a torn-off
    /// window's.
    pub fn group_viewports(&self, window: Option<u32>) -> Vec<u32> {
        let tree = match window {
            None => Some(&self.main),
            Some(id) => self.windows.iter().find(|w| w.id == id).map(|w| &w.tree),
        };
        let mut ids: Vec<u32> = tree
            .map(panes_of)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| match p {
                Pane::Viewport(n) => Some(n),
                _ => None,
            })
            .collect();
        ids.sort();
        ids
    }

    /// Every view (3D and UV) of one group, by key.
    pub fn group_views(&self, window: Option<u32>) -> Vec<u32> {
        let tree = match window {
            None => Some(&self.main),
            Some(id) => self.windows.iter().find(|w| w.id == id).map(|w| &w.tree),
        };
        let mut ids: Vec<u32> = tree
            .map(panes_of)
            .unwrap_or_default()
            .into_iter()
            .filter_map(Pane::view_key)
            .collect();
        ids.sort();
        ids
    }

    /// The view a group shows alone, when it is maximised. A maximised
    /// view that has left the group, or a group down to one view, counts
    /// as not maximised.
    pub fn maximized_in(&self, window: Option<u32>) -> Option<u32> {
        let stored = match window {
            None => self.maximized,
            Some(id) => self.windows.iter().find(|w| w.id == id)?.maximized,
        }?;
        let views = self.group_views(window);
        (views.len() > 1 && views.contains(&stored)).then_some(stored)
    }

    /// Whether `view` is the one its group shows alone.
    pub fn is_maximized(&self, view: u32) -> bool {
        let window = self.view_window(view);
        self.maximized_in(window) == Some(view)
    }

    /// Whether `view` shares its group with another view (Maximise only
    /// applies then).
    pub fn in_group(&self, view: u32) -> bool {
        let window = self.view_window(view);
        let views = self.group_views(window);
        views.len() > 1 && views.contains(&view)
    }

    /// Maximise `view` in its group, or restore the group when it already
    /// is. Returns whether it is maximised now. A view alone in its group
    /// can't be maximised.
    pub fn toggle_maximized(&mut self, view: u32) -> Result<bool, String> {
        if !self.in_group(view) {
            return Err("This view is alone in its layout; there is nothing to maximise".into());
        }
        let window = self.view_window(view);
        let now = (self.maximized_in(window) != Some(view)).then_some(view);
        match window {
            None => self.maximized = now,
            Some(id) => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.maximized = now;
                }
            }
        }
        Ok(now.is_some())
    }

    /// Splitting a maximised view leaves Maximise: the group's other
    /// views (the layout Maximise would restore) close, so the view is
    /// alone and the split that follows becomes the group's layout, the
    /// one Maximise now toggles with. Maximise is one view versus
    /// whatever multiple-view layout the user built, not a fixed quad
    /// (Jason, 2026-11-12). Panes that aren't views stay.
    fn leave_maximized_for_split(&mut self, target: Pane) {
        let Some(view) = target.view_key() else {
            return;
        };
        if !self.is_maximized(view) {
            return;
        }
        let window = self.view_window(view);
        let others: Vec<Pane> = self
            .group_views(window)
            .into_iter()
            .filter(|&v| v != view)
            .filter_map(|v| self.view_pane(v))
            .collect();
        if let Some(tree) = self.tree_of_mut(target) {
            for other in others {
                // The closing view's room goes to the kept view, not to
                // side panels sharing its split (Jason, 2026-11-19: a
                // blank-looking wide column was left beside the view).
                if let (Some(gone), Some(kept)) =
                    (tree.tiles.find_pane(&other), tree.tiles.find_pane(&target))
                {
                    hand_share_to(tree, gone, kept);
                }
                remove_pane(tree, other);
            }
        }
        match window {
            None => self.maximized = None,
            Some(id) => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.maximized = None;
                }
            }
        }
    }

    /// Turn a view that is alone in its group into a 2 x 2 quad: `view`
    /// takes slot `slot` (0 top left, 1 top right, 2 bottom left, 3 bottom
    /// right) and three new views fill the rest. Returns the four views in
    /// slot order. The view keeps its key, so its camera and settings stay.
    pub fn make_quad(&mut self, view: u32, slot: usize) -> Result<[u32; 4], String> {
        if slot > 3 {
            return Err("A quad has four slots".into());
        }
        let window = self.view_window(view);
        if self.group_views(window) != [view] {
            return Err("Only a view alone in its layout becomes a quad".into());
        }
        let kind = self.view_pane(view).ok_or("The view isn't in its layout")?;
        let mut ids = [view; 4];
        for (index, id) in ids.iter_mut().enumerate() {
            if index != slot {
                *id = self.next_viewport;
                self.next_viewport += 1;
            }
        }
        let tree = match window {
            None => &mut self.main,
            Some(w) => {
                &mut self
                    .windows
                    .iter_mut()
                    .find(|x| x.id == w)
                    .ok_or("No such window")?
                    .tree
            }
        };
        let pane = tree
            .tiles
            .find_pane(&kind)
            .ok_or("The view isn't in its layout")?;
        // The quad takes the lone view's place: its tab group's place when
        // the view sits alone in one, else the view's own. Only that place
        // changes, so side panels and other parts of a dock tree stay
        // (replacing the whole root dropped every side panel once they
        // joined the tree).
        let place = match tree.tiles.parent_of(pane) {
            Some(parent)
                if matches!(
                    tree.tiles.get(parent),
                    Some(Tile::Container(egui_tiles::Container::Tabs(t))) if t.children == [pane]
                ) =>
            {
                parent
            }
            _ => pane,
        };
        let outer = tree.tiles.parent_of(place);
        if place != pane
            && let Some(Tile::Container(c)) = tree.tiles.get_mut(place)
        {
            c.remove_child(pane);
        }
        let tiles: Vec<TileId> = ids
            .iter()
            .enumerate()
            .map(|(index, &id)| {
                if index == slot {
                    pane
                } else {
                    tree.tiles.insert_pane(kind.with_key(id))
                }
            })
            .collect();
        let quad = tree.tiles.insert_new(quad_grid(tiles));
        match outer {
            Some(outer) => replace_child(tree, outer, place, quad),
            None => tree.root = Some(quad),
        }
        if place != pane {
            tree.tiles.remove(place);
        }
        match window {
            None => self.maximized = None,
            Some(w) => {
                if let Some(x) = self.windows.iter_mut().find(|x| x.id == w) {
                    x.maximized = None;
                }
            }
        }
        Ok(ids)
    }

    /// Check a loaded layout: known version, each pane once, at least one
    /// viewport, and window/viewport counters past the ids in use.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != LAYOUT_VERSION {
            return Err(format!("Layout version {} isn't supported", self.version));
        }
        let panes = self.panes();
        let mut sorted = panes.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != panes.len() {
            return Err("A pane appears twice in the layout".into());
        }
        // Layouts stage 2 (LAY-74): a layout may hold no view at all, only
        // empty panels to build from; the main window still needs one.
        if self.main.root.is_none() {
            return Err("The main window is empty".into());
        }
        let mut keys = self.views();
        keys.dedup();
        if keys.len() != self.views().len() {
            return Err("Two views share a key".into());
        }
        let missing: Vec<u32> = self
            .panes()
            .into_iter()
            .filter_map(|p| match p {
                Pane::Missing(k) => Some(k),
                _ => None,
            })
            .collect();
        if missing.iter().any(|k| self.views().contains(k)) {
            return Err("Two panels share a key".into());
        }
        if self.windows.iter().any(|w| w.id >= self.next_window)
            || self.views().iter().any(|&v| v >= self.next_viewport)
            || missing.iter().any(|&v| v >= self.next_viewport)
        {
            return Err("The layout's ids are inconsistent".into());
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Load a saved layout; anything wrong is an error and the caller keeps
    /// its current (or the default) layout.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let layout: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        layout.validate()?;
        Ok(layout)
    }
}

fn reorder_dock_tab(tree: &mut Tree<Pane>, source: TileId, target: TileId, at: usize) -> bool {
    if let Some(Tile::Container(egui_tiles::Container::Tabs(tabs))) = tree.tiles.get_mut(target)
        && let Some(from) = tabs.children.iter().position(|&id| id == source)
    {
        tabs.children.remove(from);
        let at = if at > from { at - 1 } else { at }.min(tabs.children.len());
        tabs.children.insert(at, source);
        true
    } else {
        false
    }
}

/// Window trees have their own tile ids. Copy the subtree's structure
/// with fresh ids, retaining shares and the front tab.
fn copy_dock_tile(from: &Tiles<Pane>, id: TileId, to: &mut Tiles<Pane>) -> Result<TileId, String> {
    use egui_tiles::{Container, Grid, Linear, Tabs};
    let tile = match from.get(id).ok_or("The source panel moved")? {
        Tile::Pane(pane) => Tile::Pane(*pane),
        Tile::Container(container) => {
            let children = container
                .children()
                .map(|&id| copy_dock_tile(from, id, to))
                .collect::<Result<Vec<_>, _>>()?;
            Tile::Container(match container {
                Container::Tabs(tabs) => Container::Tabs(Tabs {
                    active: tabs
                        .active
                        .and_then(|active| tabs.children.iter().position(|&id| id == active))
                        .map(|i| children[i]),
                    children,
                }),
                Container::Linear(linear) => {
                    let mut copy = Linear::new(linear.dir, children);
                    for (&old, &new) in linear.children.iter().zip(&copy.children) {
                        copy.shares.set_share(new, linear.shares[old]);
                    }
                    Container::Linear(copy)
                }
                Container::Grid(grid) => {
                    let mut copy = Grid::new(children);
                    copy.layout = grid.layout;
                    copy.col_shares = grid.col_shares.clone();
                    copy.row_shares = grid.row_shares.clone();
                    Container::Grid(copy)
                }
            })
        }
    };
    Ok(to.insert_new(tile))
}

fn dock_contains(tree: &Tree<Pane>, source: TileId, mut target: TileId) -> bool {
    loop {
        if source == target {
            return true;
        }
        let Some(parent) = tree.tiles.parent_of(target) else {
            return false;
        };
        target = parent;
    }
}

fn relocate_dock_tile(tree: &mut Tree<Pane>, source: TileId) -> Result<TileId, String> {
    let tile = tree.tiles.remove(source).ok_or("No dock source")?;
    tree.tiles.insert(source, empty_dock_tile());
    Ok(tree.tiles.insert_new(tile))
}

fn empty_dock_tile() -> Tile<Pane> {
    Tile::Container(egui_tiles::Container::Tabs(egui_tiles::Tabs::new(
        Vec::new(),
    )))
}

fn finish_dock(tree: &mut Tree<Pane>, source: TileId) {
    give_share_to_neighbours(tree, source);
    tree.remove_recursively(source);
    tree.simplify(&egui_tiles::SimplificationOptions {
        all_panes_must_have_tabs: false,
        ..Default::default()
    });
}

/// Cut a linked grid along a divider, retaining the grid on each side of
/// the inserted strip and its original shares along both axes.
fn dock_between_grid_lines(
    tree: &mut Tree<Pane>,
    source: TileId,
    target: TileId,
    grid: egui_tiles::Grid,
    side: Side,
    at: usize,
) -> Result<(), String> {
    use egui_tiles::{Container, Grid, GridLayout, Linear};
    let cells: Vec<_> = grid.children().copied().collect();
    let columns = grid_columns(&grid, cells.len());
    let rows = cells.len().div_ceil(columns);
    let across = side.dir() == egui_tiles::LinearDir::Horizontal;
    let lines = if across { columns } else { rows };
    if !(1..lines).contains(&at) {
        return Err("The grid divider moved".into());
    }
    let weights = |shares: &[f32], range: std::ops::Range<usize>| {
        range
            .map(|i| shares.get(i).copied().unwrap_or(1.))
            .collect::<Vec<_>>()
    };
    let mut halves = Vec::new();
    for range in [0..at, at..lines] {
        let children = cells
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                range
                    .contains(&(if across { i % columns } else { i / columns }))
                    .then_some(*id)
            })
            .collect();
        let mut half = Grid::new(children);
        half.layout = GridLayout::Columns(if across { range.len() } else { columns });
        half.col_shares = weights(
            &grid.col_shares,
            if across { range.clone() } else { 0..columns },
        );
        half.row_shares = weights(&grid.row_shares, if across { 0..rows } else { range });
        let share = if across {
            half.col_shares.iter().sum::<f32>()
        } else {
            half.row_shares.iter().sum()
        };
        halves.push((tree.tiles.insert_container(half), share));
    }
    let new = relocate_dock_tile(tree, source)?;
    let mut split = Linear::new(side.dir(), vec![halves[0].0, new, halves[1].0]);
    let share = halves[0].1.min(halves[1].1) / 2.;
    split.shares.set_share(new, share);
    for (id, total) in halves {
        split.shares.set_share(id, total - share / 2.);
    }
    tree.tiles
        .insert(target, Tile::Container(Container::Linear(split)));
    finish_dock(tree, source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_swaps_what_a_slot_shows_and_keeps_its_place() {
        let mut layout = Layout::single_viewport();
        layout.add_viewport();
        assert!(layout.toggle_maximized(1).unwrap());
        layout.replace(Pane::Viewport(1), Pane::Missing(5)).unwrap();
        assert_eq!(layout.panes(), [Pane::Viewport(0), Pane::Missing(5)]);
        assert_eq!(layout.maximized, None, "the maximised view left");
        assert_eq!(layout.pane_with_key(5), Some(Pane::Missing(5)));
        assert!(layout.replace(Pane::Missing(5), Pane::Viewport(0)).is_err());
        assert!(layout.replace(Pane::Uv(9), Pane::Uv(10)).is_err());
    }

    /// Layouts stage 6 (LAY-52): a floating window moves to another layout
    /// whole, under keys and an id of that layout's own.
    #[test]
    fn a_window_taken_from_one_layout_is_adopted_by_another() {
        let mut from = Layout::single_viewport();
        from.add_viewport();
        let id = from.tear_off(Pane::Viewport(1), [300., 200.]).unwrap();
        from.windows[0].position = Some([10., 20.]);
        let window = from.take_window(id).unwrap();
        assert!(from.windows.is_empty());
        from.validate().unwrap();
        let mut to = Layout::single_viewport();
        to.add_viewport();
        let (new_id, keys) = to.adopt_window(window);
        to.validate().unwrap();
        let new_key = keys[&1];
        assert_ne!(new_key, 0);
        assert_ne!(new_key, 1, "a key the layout's own view doesn't use");
        assert_eq!(to.window_of(Pane::Viewport(new_key)), Some(new_id));
        let w = &to.windows[0];
        assert_eq!((w.size, w.position), ([300., 200.], Some([10., 20.])));
        assert!(to.take_window(99).is_none());
    }

    #[test]
    fn single_viewport_layout() {
        let mut layout = Layout::single_viewport();
        assert_eq!(layout.panes(), [Pane::Viewport(0)]);
        layout.validate().unwrap();
        assert!(layout.tear_off(Pane::Viewport(0), [100., 100.]).is_err());
        layout.add_viewport();
        layout.tear_off(Pane::Viewport(1), [100., 100.]).unwrap();
        layout.validate().unwrap();
    }

    #[test]
    fn default_layout_matches_the_fixed_one() {
        let layout = Layout::default();
        assert_eq!(
            layout.panes().len(),
            4,
            "toolbox, viewport, items, properties"
        );
        assert_eq!(layout.viewports(), [0]);
        assert!(layout.windows.is_empty());
        layout.validate().unwrap();
    }

    #[test]
    fn tear_off_and_dock_keep_every_pane_once() {
        let mut layout = Layout::default();
        let window = layout.tear_off(Pane::Properties, [300., 400.]).unwrap();
        assert_eq!(layout.window_of(Pane::Properties), Some(window));
        assert_eq!(layout.window_of(Pane::Items), None);
        layout.validate().unwrap();
        // Tearing it off again moves it to a fresh window; the old one goes.
        let again = layout.tear_off(Pane::Properties, [300., 400.]).unwrap();
        assert_ne!(again, window);
        assert_eq!(layout.windows.len(), 1);
        layout.dock(again).unwrap();
        assert!(layout.windows.is_empty());
        assert_eq!(layout.window_of(Pane::Properties), None);
        let mut panes = layout.panes();
        panes.sort();
        assert_eq!(
            panes,
            [
                Pane::Toolbox,
                Pane::Viewport(0),
                Pane::Items,
                Pane::Properties
            ]
        );
        layout.validate().unwrap();
    }

    #[test]
    fn the_last_main_pane_stays() {
        let mut layout = Layout::default();
        for pane in [Pane::Toolbox, Pane::Items, Pane::Properties] {
            layout.tear_off(pane, [200., 200.]).unwrap();
        }
        assert!(layout.tear_off(Pane::Viewport(0), [200., 200.]).is_err());
        assert_eq!(layout.windows.len(), 3);
        layout.validate().unwrap();
    }

    #[test]
    fn a_second_viewport_has_its_own_key() {
        let mut layout = Layout::default();
        let pane = layout.add_viewport();
        assert_eq!(pane, Pane::Viewport(1));
        assert_eq!(layout.viewports(), [0, 1]);
        layout.validate().unwrap();
    }

    #[test]
    fn save_and_reopen_restore_the_layout() {
        let mut layout = Layout::default();
        layout.add_viewport();
        let window = layout.tear_off(Pane::Items, [250., 500.]).unwrap();
        let text = layout.to_json().unwrap();
        let reopened = Layout::from_json(&text).unwrap();
        assert_eq!(reopened.window_of(Pane::Items), Some(window));
        assert_eq!(reopened.viewports(), [0, 1]);
        assert_eq!(reopened.windows[0].size, [250., 500.]);
        assert_eq!(reopened.main, layout.main);
        // Tearing off after a reopen still gets a new window id.
        let mut reopened = reopened;
        let next = reopened.tear_off(Pane::Toolbox, [100., 100.]).unwrap();
        assert!(next > window);
    }

    #[test]
    fn bad_saved_layouts_are_refused() {
        assert!(Layout::from_json("{").is_err());
        let layout = Layout {
            version: LAYOUT_VERSION + 1,
            ..Default::default()
        };
        assert!(Layout::from_json(&layout.to_json().unwrap()).is_err());
        let mut layout = Layout::default();
        let id = layout.main.tiles.insert_pane(Pane::Items);
        if let Some(Tile::Container(c)) = layout.main.tiles.get_mut(layout.main.root.unwrap()) {
            c.add_child(id);
        }
        assert!(
            Layout::from_json(&layout.to_json().unwrap()).is_err(),
            "Items twice"
        );
    }

    #[test]
    fn maximise_shows_one_view_and_restore_keeps_the_splits() {
        let mut layout = Layout::single_viewport();
        // A view alone can't be maximised (VB-Q1: no button there).
        assert!(!layout.in_group(0));
        assert!(layout.toggle_maximized(0).is_err());
        layout.add_viewport();
        layout.add_viewport();
        let before = layout.main.clone();
        assert!(layout.toggle_maximized(1).unwrap());
        assert!(layout.is_maximized(1));
        assert!(!layout.is_maximized(0));
        assert_eq!(layout.maximized_in(None), Some(1));
        // The tree itself never changes, so the splits come back exactly.
        assert_eq!(layout.main, before);
        // Another view's button maximises that one instead.
        assert!(layout.toggle_maximized(2).unwrap());
        assert_eq!(layout.maximized_in(None), Some(2));
        assert!(!layout.toggle_maximized(2).unwrap());
        assert_eq!(layout.maximized_in(None), None);
        assert_eq!(layout.main, before);
    }

    #[test]
    fn splitting_a_maximised_view_replaces_the_layout_it_restores() {
        // A quad, with the bottom-right view maximised.
        let mut layout = Layout::single_viewport();
        let ids = layout.make_quad(0, 1).unwrap();
        assert!(layout.toggle_maximized(ids[3]).unwrap());
        // Split it: Maximise ends and the two halves are the whole layout.
        let copy = Pane::Viewport(layout.new_viewport_key());
        layout
            .insert_beside(copy, Pane::Viewport(ids[3]), Side::Right)
            .unwrap();
        assert_eq!(layout.maximized_in(None), None);
        let pair = vec![ids[3], copy.view_key().unwrap()];
        let mut sorted = pair.clone();
        sorted.sort();
        assert_eq!(layout.group_views(None), sorted);
        assert_eq!(
            parent_split(&layout.main, Pane::Viewport(ids[3])).1,
            vec![Some(Pane::Viewport(ids[3])), Some(copy)]
        );
        // Maximise now toggles between one view and that two-way split.
        let split = layout.main.clone();
        assert!(layout.toggle_maximized(ids[3]).unwrap());
        assert!(!layout.toggle_maximized(ids[3]).unwrap());
        assert_eq!(layout.main, split);
        // Split Quad on a maximised view does the same: four views, no
        // Maximise, the hidden views gone.
        assert!(layout.toggle_maximized(ids[3]).unwrap());
        let others = [
            layout.new_viewport_key(),
            layout.new_viewport_key(),
            layout.new_viewport_key(),
        ];
        layout.split_quad(ids[3], others).unwrap();
        assert_eq!(layout.maximized_in(None), None);
        let mut four = vec![ids[3], others[0], others[1], others[2]];
        four.sort();
        assert_eq!(layout.group_views(None), four);
        // A split of a view that isn't maximised keeps every view.
        let copy = Pane::Viewport(layout.new_viewport_key());
        layout
            .insert_beside(copy, Pane::Viewport(others[0]), Side::Bottom)
            .unwrap();
        assert_eq!(layout.group_views(None).len(), 5);
    }

    #[test]
    fn maximised_state_saves_and_groups_are_independent() {
        let mut layout = Layout::single_viewport();
        layout.add_viewport();
        layout.add_viewport();
        layout.add_viewport();
        let window = layout.tear_off(Pane::Viewport(2), [300., 300.]).unwrap();
        layout.dock(window).unwrap();
        let window = layout.tear_off(Pane::Viewport(3), [300., 300.]).unwrap();
        // One view alone in its window: nothing to maximise there.
        assert!(layout.toggle_maximized(3).is_err());
        layout.toggle_maximized(0).unwrap();
        let text = layout.to_json().unwrap();
        let reopened = Layout::from_json(&text).unwrap();
        assert_eq!(reopened.maximized_in(None), Some(0));
        assert_eq!(reopened.maximized_in(Some(window)), None);
        // A layout saved before Maximise existed loads unmaximised.
        let old = text.replace("\"maximized\": 0", "\"unused\": 0");
        let old: serde_json::Value = serde_json::from_str(&old).unwrap();
        let mut old = old;
        old.as_object_mut().unwrap().remove("unused");
        let reopened = Layout::from_json(&old.to_string()).unwrap();
        assert_eq!(reopened.maximized_in(None), None);
        // Tearing the maximised view off leaves its old group restored.
        let mut layout = reopened;
        layout.toggle_maximized(1).unwrap();
        layout.tear_off(Pane::Viewport(1), [200., 200.]).unwrap();
        assert_eq!(layout.maximized_in(None), None);
    }

    /// The split holding `pane` directly: its direction, its children's
    /// panes (None for a nested container) in order, and their shares.
    fn parent_split(
        tree: &Tree<Pane>,
        pane: Pane,
    ) -> (egui_tiles::LinearDir, Vec<Option<Pane>>, Vec<f32>) {
        let id = tree.tiles.find_pane(&pane).unwrap();
        let parent = tree.tiles.parent_of(id).unwrap();
        let Some(Tile::Container(egui_tiles::Container::Linear(linear))) = tree.tiles.get(parent)
        else {
            panic!("{pane:?} isn't in a split");
        };
        (
            linear.dir,
            linear
                .children
                .iter()
                .map(|c| tree.tiles.get_pane(c).copied())
                .collect(),
            linear.children.iter().map(|&c| linear.shares[c]).collect(),
        )
    }

    /// Three views side by side in the main window.
    fn three_views() -> Layout {
        let mut layout = Layout::single_viewport();
        layout.main = Tree::new_horizontal("three", (0..3).map(Pane::Viewport).collect());
        layout.next_viewport = 3;
        layout
    }

    /// Node View (dock below the views): a new editor tab docks under the
    /// tile holding every view, not under the side column, and takes the
    /// asked share of the views' old height.
    #[test]
    fn an_editor_tab_docks_below_the_whole_view_area() {
        let mut layout = Layout::single_viewport();
        let mut tiles = Tiles::default();
        let v0 = tiles.insert_pane(Pane::Viewport(0));
        let v1 = tiles.insert_pane(Pane::Viewport(1));
        let views = tiles.insert_horizontal_tile(vec![v0, v1]);
        let items = tiles.insert_pane(Pane::Items);
        let root = tiles.insert_horizontal_tile(vec![views, items]);
        layout.main = Tree::new("views-and-side", root, tiles);
        layout.next_viewport = 2;
        assert_eq!(layout.view_area_tile(), Some(views));
        let (pane, source) = layout.new_dock_tab(Slot::new("Node View"));
        layout.dock_tile_below_views(source, 0.4).unwrap();
        layout.validate().unwrap();
        let (dir, children, shares) = parent_split(&layout.main, pane);
        assert_eq!(dir, egui_tiles::LinearDir::Vertical);
        assert_eq!(children.last(), Some(&Some(pane)), "{children:?}");
        assert!((shares[1] / (shares[0] + shares[1]) - 0.4).abs() < 1e-4);
        // The side column stays full height beside both.
        let (dir, children, _) = parent_split(&layout.main, Pane::Items);
        assert_eq!(dir, egui_tiles::LinearDir::Horizontal);
        assert_eq!(children.len(), 2);
        // No views in the main window: nowhere to dock below.
        let mut lone = Layout::single_viewport();
        let mut tiles = Tiles::default();
        let only = tiles.insert_pane(Pane::Items);
        lone.main = Tree::new("side", only, tiles);
        assert_eq!(lone.view_area_tile(), None);
    }

    #[test]
    fn swap_trades_places_and_each_place_keeps_its_size() {
        let mut layout = three_views();
        let (_, before, _) = parent_split(&layout.main, Pane::Viewport(0));
        layout.swap(Pane::Viewport(0), Pane::Viewport(2)).unwrap();
        let (_, after, _) = parent_split(&layout.main, Pane::Viewport(0));
        assert_eq!(
            before,
            [
                Some(Pane::Viewport(0)),
                Some(Pane::Viewport(1)),
                Some(Pane::Viewport(2))
            ]
        );
        assert_eq!(
            after,
            [
                Some(Pane::Viewport(2)),
                Some(Pane::Viewport(1)),
                Some(Pane::Viewport(0))
            ]
        );
        // Across windows: the torn-off view and a main view trade places.
        let window = layout.tear_off(Pane::Viewport(1), [300., 200.]).unwrap();
        layout.swap(Pane::Viewport(1), Pane::Viewport(2)).unwrap();
        assert_eq!(layout.window_of(Pane::Viewport(2)), Some(window));
        assert_eq!(layout.window_of(Pane::Viewport(1)), None);
        layout.validate().unwrap();
        assert!(layout.swap(Pane::Viewport(1), Pane::Viewport(1)).is_err());
        assert!(layout.swap(Pane::Viewport(1), Pane::Viewport(9)).is_err());
    }

    #[test]
    fn dock_beside_splits_the_target_at_half_on_each_side() {
        use egui_tiles::LinearDir::{Horizontal, Vertical};
        for (side, dir, before) in [
            (Side::Left, Horizontal, true),
            (Side::Right, Horizontal, false),
            (Side::Top, Vertical, true),
            (Side::Bottom, Vertical, false),
        ] {
            let mut layout = three_views();
            layout
                .dock_beside(Pane::Viewport(0), Pane::Viewport(2), side)
                .unwrap();
            layout.validate().unwrap();
            let (d, children, shares) = parent_split(&layout.main, Pane::Viewport(0));
            assert_eq!(d, dir, "{side:?}");
            let (moved, target) = (Some(Pane::Viewport(0)), Some(Pane::Viewport(2)));
            let i = children.iter().position(|c| *c == moved).unwrap();
            let j = children.iter().position(|c| *c == target).unwrap();
            assert_eq!(i + 1 == j, before, "{side:?}: {children:?}");
            assert_eq!(j + 1 == i, !before, "{side:?}: {children:?}");
            // At half: the moved view and the target share equally, and
            // (side by side) the untouched view, which took the moved view's
            // old place as its neighbour (VST-19), is twice the target's
            // whole place.
            assert_eq!(shares[i], shares[j], "{side:?}");
            if dir == Horizontal {
                let k = children
                    .iter()
                    .position(|c| *c == Some(Pane::Viewport(1)))
                    .unwrap();
                assert_eq!(shares[k], 2. * (shares[i] + shares[j]), "{side:?}");
            }
            // The hole closed: View 1 and the target's place are all that's
            // left at the top.
            assert_eq!(layout.panes().len(), 3);
        }
    }

    #[test]
    fn dock_back_from_a_window_closes_the_empty_window() {
        let mut layout = three_views();
        let window = layout.tear_off(Pane::Viewport(1), [300., 200.]).unwrap();
        assert_eq!(layout.windows.len(), 1);
        layout
            .dock_beside(Pane::Viewport(1), Pane::Viewport(0), Side::Bottom)
            .unwrap();
        assert!(layout.windows.is_empty(), "window {window} closed");
        assert_eq!(layout.window_of(Pane::Viewport(1)), None);
        layout.validate().unwrap();
        // A main view docks into a window beside its view; Delete of a
        // window's last view closes that window too.
        let window = layout.tear_off(Pane::Viewport(2), [300., 200.]).unwrap();
        layout
            .dock_beside(Pane::Viewport(1), Pane::Viewport(2), Side::Right)
            .unwrap();
        assert_eq!(layout.window_of(Pane::Viewport(1)), Some(window));
        layout.remove(Pane::Viewport(1)).unwrap();
        layout.remove(Pane::Viewport(2)).unwrap();
        assert!(layout.windows.is_empty());
        layout.validate().unwrap();
    }

    #[test]
    fn tear_off_at_opens_where_asked_and_never_empties_a_window() {
        let mut layout = three_views();
        let window = layout
            .tear_off_at(Pane::Viewport(2), [320., 240.], Some([500., 300.]))
            .unwrap();
        let w = layout.windows.iter().find(|w| w.id == window).unwrap();
        assert_eq!(w.position, Some([500., 300.]));
        assert_eq!(w.size, [320., 240.]);
        // Alone in its window: it can't tear off again (the app makes a copy
        // instead).
        assert!(layout.alone_in_window(Pane::Viewport(2)));
        assert!(
            layout
                .tear_off_at(Pane::Viewport(2), [1., 1.], None)
                .is_err()
        );
        // Saved and reopened, the position stays.
        let reopened = Layout::from_json(&layout.to_json().unwrap()).unwrap();
        assert_eq!(reopened.windows[0].position, Some([500., 300.]));
        // A copy into its own window.
        let copy = Pane::Viewport(layout.new_viewport_key());
        let second = layout.new_window(copy, [100., 100.], Some([10., 10.]));
        assert_eq!(layout.window_of(copy), Some(second));
        layout.validate().unwrap();
        // Closing a window discards what it held.
        assert_eq!(layout.discard_window(second), [copy]);
        assert_eq!(layout.window_of(copy), None);
        assert!(!layout.panes().contains(&copy));
    }

    #[test]
    fn the_last_main_panel_never_leaves() {
        let mut layout = Layout::single_viewport();
        let view = Pane::Viewport(0);
        assert!(layout.last_in_main(view));
        assert!(layout.remove(view).is_err());
        assert!(layout.tear_off_at(view, [1., 1.], None).is_err());
        let other = Pane::Viewport(layout.new_viewport_key());
        let window = layout.new_window(other, [100., 100.], None);
        assert!(layout.dock_beside(view, other, Side::Left).is_err());
        assert_eq!(layout.window_of(view), None);
        // The window's view can come in beside it, though.
        layout.dock_beside(other, view, Side::Left).unwrap();
        assert!(layout.windows.iter().all(|w| w.id != window));
        assert!(!layout.last_in_main(view));
        layout.validate().unwrap();
    }

    #[test]
    fn split_and_duplicate_add_new_views_beside() {
        let mut layout = Layout::single_viewport();
        let copy = Pane::Viewport(layout.new_viewport_key());
        layout
            .insert_beside(copy, Pane::Viewport(0), Side::Right)
            .unwrap();
        let (_, children, shares) = parent_split(&layout.main, copy);
        assert_eq!(children, [Some(Pane::Viewport(0)), Some(copy)]);
        assert_eq!(shares[0], shares[1]);
        assert!(
            layout
                .insert_beside(copy, Pane::Viewport(0), Side::Right)
                .is_err()
        );
        // Duplicate: at the right of the whole layout.
        let dup = Pane::Viewport(layout.new_viewport_key());
        layout.add_at_right(dup, None).unwrap();
        let root = layout.main.root.unwrap();
        let Some(Tile::Container(c)) = layout.main.tiles.get(root) else {
            panic!()
        };
        let last = *c.children().last().unwrap();
        assert_eq!(layout.main.tiles.get_pane(&last), Some(&dup));
        layout.validate().unwrap();
    }

    /// The grid a quad is: two columns, four views in row order.
    fn quad_views(tree: &Tree<Pane>, any: Pane) -> Vec<Pane> {
        let id = tree.tiles.find_pane(&any).unwrap();
        let parent = tree.tiles.parent_of(id).unwrap();
        let Some(Tile::Container(egui_tiles::Container::Grid(grid))) = tree.tiles.get(parent)
        else {
            panic!("not a grid");
        };
        assert_eq!(grid.layout, egui_tiles::GridLayout::Columns(2));
        grid.children()
            .map(|c| *tree.tiles.get_pane(c).unwrap())
            .collect()
    }

    #[test]
    fn quads_are_one_linked_grid() {
        // Split Quad in place.
        let mut layout = three_views();
        let others = [
            layout.new_viewport_key(),
            layout.new_viewport_key(),
            layout.new_viewport_key(),
        ];
        layout.split_quad(1, others).unwrap();
        layout.validate().unwrap();
        assert_eq!(
            quad_views(&layout.main, Pane::Viewport(1)),
            [1, others[0], others[1], others[2]].map(Pane::Viewport)
        );
        // Its neighbours keep their places.
        let (_, children, _) = parent_split(&layout.main, Pane::Viewport(0));
        assert_eq!(children.len(), 3);
        assert_eq!(children[1], None, "the quad sits in the middle");
        // Maximise's quad (a lone view) is the same kind of grid.
        let mut layout = Layout::single_viewport();
        let ids = layout.make_quad(0, 3).unwrap();
        assert_eq!(
            quad_views(&layout.main, Pane::Viewport(0)),
            ids.map(Pane::Viewport)
        );
    }

    #[test]
    fn locked_dividers_come_back_after_a_drag() {
        let mut layout = three_views();
        let before = layout.main.clone();
        let root = layout.main.root.unwrap();
        if let Some(Tile::Container(egui_tiles::Container::Linear(l))) =
            layout.main.tiles.get_mut(root)
        {
            let first = l.children[0];
            l.shares.set_share(first, 2.5);
        }
        assert_ne!(layout.main, before);
        restore_dividers(&before, &mut layout.main);
        assert_eq!(layout.main, before);
    }

    #[test]
    fn a_lone_view_becomes_a_quad() {
        let mut layout = Layout::single_viewport();
        let ids = layout.make_quad(0, 1).unwrap();
        assert_eq!(ids, [1, 0, 2, 3]);
        assert_eq!(layout.viewports(), [0, 1, 2, 3]);
        assert!(layout.in_group(0));
        layout.validate().unwrap();
        // Saved and reopened, it is still the same quad.
        let reopened = Layout::from_json(&layout.to_json().unwrap()).unwrap();
        assert_eq!(reopened.main, layout.main);
        // A view that already shares its group doesn't.
        assert!(layout.make_quad(0, 0).is_err());
        let next = layout.add_viewport();
        assert_eq!(next, Pane::Viewport(4));
    }

    /// Each view's width in a horizontal tree laid out `width` wide.
    fn widths(tree: &mut Tree<Pane>, width: f32) -> Vec<(Pane, f32)> {
        let ctx = egui::Context::default();
        let mut out = Vec::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 400.));
                tree.ui(
                    &mut Plain,
                    &mut ui.new_child(egui::UiBuilder::new().max_rect(rect)),
                );
            });
        });
        for (id, tile) in tree.tiles.iter() {
            if let Tile::Pane(p) = tile {
                out.push((*p, tree.tiles.rect(*id).unwrap().width()));
            }
        }
        out.sort_by_key(|(p, _)| *p);
        out
    }

    /// The least a tile behaviour needs, with the app's gap.
    struct Plain;
    impl egui_tiles::Behavior<Pane> for Plain {
        fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
            pane.title().into()
        }
        fn pane_ui(
            &mut self,
            _ui: &mut egui::Ui,
            _tile: TileId,
            _pane: &mut Pane,
        ) -> egui_tiles::UiResponse {
            egui_tiles::UiResponse::None
        }
        fn gap_width(&self, _style: &egui::Style) -> f32 {
            GAP
        }
        fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
            egui_tiles::SimplificationOptions {
                all_panes_must_have_tabs: false,
                ..Default::default()
            }
        }
    }

    /// Three views side by side with these shares.
    fn row(shares: [f32; 3]) -> Layout {
        let mut layout = three_views();
        let root = layout.main.root.unwrap();
        if let Some(Tile::Container(egui_tiles::Container::Linear(l))) =
            layout.main.tiles.get_mut(root)
        {
            for (child, share) in l.children.clone().into_iter().zip(shares) {
                l.shares.set_share(child, share);
            }
        }
        layout
    }

    #[test]
    fn a_deleted_middle_panel_is_shared_by_its_neighbours_in_proportion() {
        // T08: 200 / 300 / 500, delete the middle: about 286 and 714.
        let mut layout = row([200., 300., 500.]);
        layout.remove(Pane::Viewport(1)).unwrap();
        let w = widths(&mut layout.main, 1000. + GAP);
        assert!((w[0].1 - 285.7).abs() < 1., "{w:?}");
        assert!((w[1].1 - 714.3).abs() < 1., "{w:?}");
        // In a row of four, only the two neighbours grow.
        let mut layout = Layout::single_viewport();
        layout.main = Tree::new_horizontal("four", (0..4).map(Pane::Viewport).collect());
        layout.next_viewport = 4;
        layout.remove(Pane::Viewport(1)).unwrap();
        let w = widths(&mut layout.main, 400. + 2. * GAP);
        assert!(
            (w[2].1 - 100.).abs() < 0.5,
            "the far view keeps its size: {w:?}"
        );
        assert!(
            (w[0].1 - 150.).abs() < 0.5 && (w[1].1 - 150.).abs() < 0.5,
            "{w:?}"
        );
        // At an end, the one neighbour takes it all.
        let mut layout = row([200., 300., 500.]);
        layout.remove(Pane::Viewport(2)).unwrap();
        let w = widths(&mut layout.main, 1000. + GAP);
        assert!((w[0].1 - 200.).abs() < 1., "{w:?}");
        assert!((w[1].1 - 800.).abs() < 1., "{w:?}");
    }

    #[test]
    fn split_then_delete_leaves_no_nesting() {
        // T09: one view split any way and the copy deleted is one view again.
        for side in [Side::Bottom, Side::Top, Side::Left, Side::Right] {
            let mut layout = Layout::single_viewport();
            let copy = Pane::Viewport(layout.new_viewport_key());
            layout.insert_beside(copy, Pane::Viewport(0), side).unwrap();
            layout.remove(copy).unwrap();
            let root = layout.main.root.unwrap();
            assert_eq!(layout.main.tiles.get_pane(&root), Some(&Pane::Viewport(0)));
        }
        // A stacked split inside a row dissolves and the remaining view
        // takes the whole old place.
        for side in [Side::Bottom, Side::Top] {
            let mut layout = three_views();
            let before = layout.main.clone();
            let copy = Pane::Viewport(layout.new_viewport_key());
            layout.insert_beside(copy, Pane::Viewport(1), side).unwrap();
            layout.remove(copy).unwrap();
            let (dir, children, shares) = parent_split(&layout.main, Pane::Viewport(1));
            assert_eq!(dir, egui_tiles::LinearDir::Horizontal, "{side:?}");
            assert_eq!(children.len(), 3, "{side:?}");
            assert!(children.iter().all(Option::is_some), "{side:?}: no nesting");
            let (_, _, old) = parent_split(&before, Pane::Viewport(1));
            let ratio = |s: &[f32]| s[1] / s.iter().sum::<f32>();
            assert!((ratio(&shares) - ratio(&old)).abs() < 1e-4, "{side:?}");
        }
    }

    #[test]
    fn splits_too_small_for_both_halves_are_refused() {
        let view = Pane::Viewport(0);
        // T07: 150 wide halves to under 120.
        assert!(!split_fits(view, [150., 400.], Side::Right));
        assert!(!split_fits(view, [150., 400.], Side::Left));
        assert!(split_fits(view, [150., 400.], Side::Bottom));
        assert!(split_fits(view, [241., 181.], Side::Right));
        assert!(split_fits(view, [241., 181.], Side::Top));
        assert!(!split_fits(view, [241., 179.], Side::Top));
        assert!(quad_fits(view, [241., 181.]));
        assert!(!quad_fits(view, [241., 170.]));
        assert!(!split_fits(Pane::Uv(3), [200., 400.], Side::Right));
        assert!(split_fits(Pane::Items, [201., 100.], Side::Right));
    }

    #[test]
    fn minimums_add_along_a_split_and_windows_never_go_below_their_floor() {
        let layout = three_views();
        let root = layout.main.root.unwrap();
        assert_eq!(tile_min(&layout.main, root), [3. * 120. + 2. * GAP, 90.]);
        let mut layout = Layout::single_viewport();
        assert_eq!(window_min(&layout.main), WINDOW_MIN);
        layout.make_quad(0, 0).unwrap();
        assert_eq!(window_min(&layout.main), [241., 181.]);
    }

    #[test]
    fn dividers_stop_at_a_neighbours_minimum() {
        // T10: drag the first divider far right: the middle view stops at
        // 120 and the last keeps its size.
        let mut layout = row([1., 1., 1.]);
        let width = 600. + 2. * GAP;
        widths(&mut layout.main, width);
        let before = layout.main.clone();
        let root = layout.main.root.unwrap();
        if let Some(Tile::Container(egui_tiles::Container::Linear(l))) =
            layout.main.tiles.get_mut(root)
        {
            let (a, b) = (l.children[0], l.children[1]);
            l.shares.set_share(a, 1.9);
            l.shares.set_share(b, 0.1);
        }
        clamp_dividers(&before, &mut layout.main);
        let w = widths(&mut layout.main, width);
        assert!((w[1].1 - 120.).abs() < 0.5, "{w:?}");
        assert!((w[0].1 - 280.).abs() < 0.5, "{w:?}");
        assert!((w[2].1 - 200.).abs() < 0.5, "{w:?}");
        // A legal drag is left alone.
        let before = layout.main.clone();
        if let Some(Tile::Container(egui_tiles::Container::Linear(l))) =
            layout.main.tiles.get_mut(root)
        {
            let (a, b) = (l.children[0], l.children[1]);
            let (sa, sb) = (l.shares[a], l.shares[b]);
            l.shares.set_share(a, sa - 0.2);
            l.shares.set_share(b, sb + 0.2);
        }
        let dragged = layout.main.clone();
        clamp_dividers(&before, &mut layout.main);
        assert_eq!(layout.main, dragged);
    }

    #[test]
    fn panels_already_below_their_minimum_may_grow_but_not_shrink() {
        let old = [1., 1.];
        // 100 points for two views of minimum 120: both are under it.
        let mut grow = [1.2, 0.8];
        clamp_line(&old, &mut grow, &[120., 120.], 100.);
        assert!(
            (grow[1] - 1.).abs() < 1e-3,
            "the second would shrink further: {grow:?}"
        );
        let mut same = old;
        clamp_line(&old, &mut same, &[120., 120.], 100.);
        assert_eq!(same, old);
    }

    #[test]
    fn resize_boundaries_and_invalid_lines() {
        for side in [Side::Left, Side::Right] {
            assert!(split_fits(Pane::Missing(2), [241., 1.], side));
            assert!(!split_fits(Pane::Missing(2), [240.9, 500.], side));
        }
        for side in [Side::Top, Side::Bottom] {
            assert!(split_fits(Pane::Properties, [1., 161.], side));
            assert!(!split_fits(Pane::Properties, [500., 160.9], side));
        }
        let mut new = [0.1, 1.9];
        clamp_line(&[1., 1.], &mut new, &[120.], 300.);
        assert_eq!(new, [0.1, 1.9]);
        clamp_line(&[1., 1.], &mut new, &[120., 120.], 0.);
        assert_eq!(new, [0.1, 1.9]);
    }

    #[test]
    fn closing_floating_last_panel_and_rejecting_missing_panels() {
        let mut layout = Layout::single_viewport();
        assert!(layout.remove(Pane::Viewport(0)).is_err());
        let window = layout.new_window(Pane::Uv(1), [300., 200.], Some([4., 5.]));
        assert!(layout.remove(Pane::Uv(9)).is_err());
        layout.remove(Pane::Uv(1)).unwrap();
        assert!(layout.windows.is_empty());
        assert!(layout.discard_window(window).is_empty());
        assert_eq!(layout.panes(), [Pane::Viewport(0)]);
    }

    #[test]
    fn invalid_layout_ids_and_old_optional_fields() {
        let layout = Layout::single_viewport();
        let mut value = serde_json::to_value(&layout).unwrap();
        value.as_object_mut().unwrap().remove("maximized");
        value.as_object_mut().unwrap().remove("missing");
        let loaded = Layout::from_json(&value.to_string()).unwrap();
        assert_eq!(loaded.maximized, None);
        assert!(loaded.missing.is_empty());
        value["version"] = serde_json::json!(0);
        assert!(Layout::from_json(&value.to_string()).is_err());
        value["version"] = serde_json::json!(LAYOUT_VERSION);
        value["next_viewport"] = serde_json::json!(0);
        assert!(Layout::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn split_errors_leave_the_layout_unchanged() {
        let mut layout = Layout::single_viewport();
        let before = layout.to_json().unwrap();
        assert!(
            layout
                .insert_beside(Pane::Viewport(0), Pane::Viewport(0), Side::Left)
                .is_err()
        );
        assert!(
            layout
                .insert_beside(Pane::Uv(1), Pane::Viewport(99), Side::Top)
                .is_err()
        );
        assert!(
            layout
                .dock_beside(Pane::Viewport(0), Pane::Viewport(0), Side::Right)
                .is_err()
        );
        assert!(layout.split_quad(99, [1, 2, 3]).is_err());
        assert!(layout.split_quad(0, [0, 2, 3]).is_err());
        assert!(layout.make_quad(0, 4).is_err());
        assert_eq!(layout.to_json().unwrap(), before);
    }

    #[test]
    fn uv_views_are_views_numbered_on_their_own() {
        let mut layout = three_views();
        let uv = Pane::Uv(layout.new_viewport_key());
        layout
            .insert_beside(uv, Pane::Viewport(0), Side::Left)
            .unwrap();
        let uv2 = Pane::Uv(layout.new_viewport_key());
        layout.new_window(uv2, [300., 200.], None);
        layout.validate().unwrap();
        assert_eq!(layout.views(), [0, 1, 2, 3, 4]);
        assert_eq!(layout.viewports(), [0, 1, 2], "3D views only");
        assert_eq!(layout.title_of(uv), "UV");
        assert_eq!(layout.title_of(uv2), "UV 2");
        assert_eq!(layout.view_pane(3), Some(uv));
        // A UV view maximises, and its quad is four UV views.
        assert!(layout.toggle_maximized(3).unwrap());
        assert!(layout.is_maximized(3));
        let mut layout = Layout::single_viewport();
        let uv = layout.new_viewport_key();
        layout.add_at_right(Pane::Uv(uv), None).unwrap();
        layout.remove(Pane::Viewport(0)).unwrap();
        let ids = layout.make_quad(uv, 0).unwrap();
        assert!(
            ids.iter()
                .all(|&k| matches!(layout.view_pane(k), Some(Pane::Uv(_))))
        );
        let reopened = Layout::from_json(&layout.to_json().unwrap()).unwrap();
        assert_eq!(reopened.views(), layout.views());
    }
}
