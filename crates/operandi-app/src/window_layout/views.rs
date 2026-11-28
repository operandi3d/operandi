//! Several viewports, each with its own camera, arranged by the workspace
//! dock layout (crates/operandi-layout; docs/WORKSPACE_LAYOUT.md).
//!
//! `camera` and `viewport_rect` always belong to the active view, so tools,
//! shortcuts and tests keep working on "the" view. While another view draws,
//! its camera is swapped in and it takes no input (`view_passive`). A press
//! inside a view makes it the active one.
use super::*;
use crate::unavailable::Gate;
use operandi_layout::Pane;

/// The bottom row shared by the mode bar and the message line.
pub(crate) const MODE_BAR_HEIGHT: f32 = 26.;

/// What a view's tab asks for while the layout draws.
#[derive(Default)]
struct Requests {
    tear_off: Option<Pane>,
    close: Option<Pane>,
}

struct Views<'a> {
    app: &'a mut Operandi,
    requests: &'a mut Requests,
    count: usize,
    window: Option<u32>,
    dock_tabs: bool,
    tab_row: Option<(egui_tiles::TileId, Vec<egui_tiles::TileId>)>,
}

impl egui_tiles::Behavior<Pane> for Views<'_> {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        self.app.views.title_of(*pane).into()
    }

    /// The grab hand on every tab: a tab reorders along its own row any
    /// time; only dragging it out waits for UI build mode (Jason,
    /// 2026-11-23).
    fn tab_hover_cursor_icon(&self) -> egui::CursorIcon {
        egui::CursorIcon::Grab
    }

    fn pane_ui(
        &mut self,
        ui: &mut Ui,
        _tile: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        // 3D and UV views draw alike; a UV pane's view shows UV space.
        if let Some(view) = pane.view_key() {
            self.app.view_pane(ui, view);
        } else if let Pane::Missing(key) = *pane {
            // LAY-49: a kind this build lacks is an empty panel with its
            // name. LAY-74: an Empty slot is one to build from. Both take
            // the corner dot and the content arrow.
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0., theme::panel());
            let name = self.app.views.title_of(*pane);
            let text = if name == operandi_layout::EMPTY {
                "Empty panel\nRight-click the corner dot (top left) to split it;\nchoose what it shows with the arrow (top right)".to_owned()
            } else {
                format!("{name}\n(not in this version)")
            };
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                text,
                crate::typography::standard_font(),
                theme::disabled_text(),
            );
            if ui.ctx().viewport_id() == egui::ViewportId::ROOT {
                self.app.view_rects.insert(key, rect);
            }
            let y = rect.top() + crate::viewport_buttons::HEADER_CLEARANCE
                - crate::viewport_buttons::HEADER_HEIGHT / 2.;
            self.app.view_dot(ui, key, rect, rect);
            let arrow = Rect::from_center_size(
                pos2(rect.right() - 4. - crate::viewport_buttons::ARROW / 2., y),
                vec2(crate::viewport_buttons::ARROW, 16.),
            );
            self.app.content_arrow(ui, key, arrow);
        } else {
            self.app.draw_docked_side(ui, *pane);
        }
        egui_tiles::UiResponse::None
    }

    fn on_tab_button(
        &mut self,
        tiles: &egui_tiles::Tiles<Pane>,
        tile_id: egui_tiles::TileId,
        response: Response,
    ) -> Response {
        if let Some((group, children)) = &self.tab_row
            && let Some(index) = children.iter().position(|&id| id == tile_id)
        {
            self.app
                .dock_tree_tab(&response, self.window, *group, tile_id, index);
        }
        if let Some(&pane) = tiles.get_pane(&tile_id).filter(|p| p.key().is_some()) {
            let more_than_one = self.count > 1;
            let (in_group, maximized) = match pane {
                Pane::Viewport(view) => self
                    .app
                    .corner
                    .groups
                    .get(&view)
                    .copied()
                    .unwrap_or_default(),
                _ => (false, false),
            };
            response.context_menu(|ui| {
                if let Pane::Viewport(view) = pane
                    && in_group
                    && ui
                        .button(if maximized {
                            "Restore Viewport Layout"
                        } else {
                            "Maximise Viewport"
                        })
                        .clicked()
                {
                    self.app.request_toggle_maximized(view);
                    ui.close();
                }
                if ui
                    .gated(
                        crate::unavailable::unless(more_than_one, "Split the view first"),
                        Button::new("Tear Off"),
                    )
                    .clicked()
                {
                    self.requests.tear_off = Some(pane);
                    ui.close();
                }
                if ui
                    .gated(
                        crate::unavailable::unless(more_than_one, "Split the view first"),
                        Button::new("Close View"),
                    )
                    .clicked()
                {
                    self.requests.close = Some(pane);
                    ui.close();
                }
            });
        }
        response
    }

    fn top_bar_right_ui(
        &mut self,
        tiles: &egui_tiles::Tiles<Pane>,
        ui: &mut Ui,
        id: egui_tiles::TileId,
        tabs: &egui_tiles::Tabs,
        _scroll: &mut f32,
    ) {
        if self.dock_tabs {
            let size = tiles.rect(id).map_or(vec2(320., 400.), |r| r.size());
            self.app
                .dock_tree_row(ui, self.window, id, tabs.children.len(), size);
            self.tab_row = Some((id, tabs.children.clone()));
        }
    }

    /// The gap the layout's size checks count on (minimum sizes, VST-23).
    fn gap_width(&self, _style: &egui::Style) -> f32 {
        operandi_layout::GAP
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        // No tab bars on views: each view's floating header sits at its top
        // edge. Tear Off and Close View are in the View menu.
        let _ = self.count;
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: false,
            ..Default::default()
        }
    }
}

/// Whether `tile` is, or holds, one of `panes`.
fn holds(tree: &egui_tiles::Tree<Pane>, tile: egui_tiles::TileId, panes: &[Pane]) -> bool {
    match tree.tiles.get(tile) {
        Some(egui_tiles::Tile::Pane(p)) => panes.contains(p),
        Some(egui_tiles::Tile::Container(c)) => c.children().any(|&t| holds(tree, t, panes)),
        None => false,
    }
}

/// Every side-by-side or stacked split's shares.
pub(crate) type LinearShares = Vec<(egui_tiles::TileId, egui_tiles::Shares)>;

pub(crate) fn linear_shares(tree: &egui_tiles::Tree<Pane>) -> LinearShares {
    tree.tiles
        .iter()
        .filter_map(|(&id, tile)| match tile {
            egui_tiles::Tile::Container(egui_tiles::Container::Linear(l)) => {
                Some((id, l.shares.clone()))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn set_linear_shares(tree: &mut egui_tiles::Tree<Pane>, shares: &LinearShares) {
    for (id, saved) in shares {
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(l))) =
            tree.tiles.get_mut(*id)
        {
            l.shares = saved.clone();
        }
    }
}

/// Put back the shares `saved` gave each tile in `hidden`, leaving its
/// visible siblings' (just resized) shares as they are.
fn keep_hidden_shares(
    tree: &mut egui_tiles::Tree<Pane>,
    saved: &LinearShares,
    hidden: &[egui_tiles::TileId],
) {
    for (id, before) in saved {
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(l))) =
            tree.tiles.get_mut(*id)
        {
            for child in l.children.clone() {
                if hidden.contains(&child) {
                    l.shares.set_share(child, before[child]);
                }
            }
        }
    }
}

/// Whether `tile` is, or holds, a view-area pane (a view or empty panel).
fn holds_view(tree: &egui_tiles::Tree<Pane>, tile: egui_tiles::TileId) -> bool {
    match tree.tiles.get(tile) {
        Some(egui_tiles::Tile::Pane(p)) => p.key().is_some(),
        Some(egui_tiles::Tile::Container(c)) => c.children().any(|&t| holds_view(tree, t)),
        None => false,
    }
}

const LEFT_COLUMN: [Pane; 2] = [Pane::Toolbox, Pane::ToolProperties];
const RIGHT_COLUMN: [Pane; 2] = [Pane::Items, Pane::Properties];

/// Where a side column starts in a side-by-side split's `children`: the
/// run of children at that end holding no view, with the column's own
/// panels among them, next to a child that holds a view. Everything in
/// that run belongs to the column, so a panel docked beside the Items
/// group (Shader Tree, say) collapses and comes back with it (Jason,
/// 2026-11-20: it stayed and filled the room instead). The range indexes
/// `children`.
fn column_run(
    tree: &egui_tiles::Tree<Pane>,
    children: &[egui_tiles::TileId],
    side: &str,
) -> Option<std::ops::Range<usize>> {
    let (panes, run) = if side == "left" {
        let end = children.iter().position(|&c| holds_view(tree, c))?;
        (&LEFT_COLUMN, 0..end)
    } else {
        let start = children.iter().rposition(|&c| holds_view(tree, c))? + 1;
        (&RIGHT_COLUMN, start..children.len())
    };
    (!run.is_empty() && children[run.clone()].iter().any(|&c| holds(tree, c, panes))).then_some(run)
}

/// The tiles of the collapsed `side` column ("left" or "right"): every
/// side-by-side split's run of children at that end (`column_run`).
fn side_column_tiles(tree: &egui_tiles::Tree<Pane>, side: &str) -> Vec<egui_tiles::TileId> {
    let mut tiles = Vec::new();
    for (_, tile) in tree.tiles.iter() {
        let egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear)) = tile else {
            continue;
        };
        if linear.dir != egui_tiles::LinearDir::Horizontal {
            continue;
        }
        if let Some(run) = column_run(tree, &linear.children, side) {
            tiles.extend_from_slice(&linear.children[run]);
        }
    }
    tiles
}

/// The side column ("left" or "right") whose divider with the rest of the
/// window is at `at`: the gap where a side-by-side split's side column
/// (`column_run`) meets its neighbour holding the views.
pub(crate) fn side_column_divider_at(
    tree: &egui_tiles::Tree<Pane>,
    at: Pos2,
) -> Option<&'static str> {
    for (_, tile) in tree.tiles.iter() {
        let egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear)) = tile else {
            continue;
        };
        if linear.dir != egui_tiles::LinearDir::Horizontal {
            continue;
        }
        let visible: Vec<_> = linear
            .children
            .iter()
            .copied()
            .filter(|&c| tree.tiles.is_visible(c))
            .collect();
        for (index, pair) in visible.windows(2).enumerate() {
            let (Some(a), Some(b)) = (tree.tiles.rect(pair[0]), tree.tiles.rect(pair[1])) else {
                continue;
            };
            let gap = Rect::from_min_max(
                pos2(a.right() - 3., a.top().max(b.top())),
                pos2(b.left() + 3., a.bottom().min(b.bottom())),
            );
            if !gap.contains(at) {
                continue;
            }
            // Only the column's outer edge, the one facing the views,
            // collapses it; an edge inside the column (the Toolbox beside a
            // docked Tool Tabs strip) is plain resizing (Jason, 2026-11-19).
            // The gap after `visible[index]`: the left column's last
            // child, or the right column's first.
            let edge = index + 1;
            for side in ["left", "right"] {
                if let Some(run) = column_run(tree, &visible, side)
                    && (if side == "left" {
                        run.end == edge
                    } else {
                        run.start == edge
                    })
                {
                    return Some(side);
                }
            }
        }
    }
    None
}

// D-07 / UIC-191: the horizontal strip is one row plus its grip and
// padding. Use its point height for drawing, keeping the saved split shares.
struct FixedToolTabs {
    split: egui_tiles::TileId,
    row: egui_tiles::TileId,
    saved_share: f32,
}

impl FixedToolTabs {
    /// `horizontal`: the strip is a row (fixed height in a top-to-bottom
    /// split); otherwise a vertical strip (fixed width in a side-by-side
    /// split). `size` is the tree's available size.
    fn size_row(
        tree: &mut egui_tiles::Tree<Pane>,
        size: egui::Vec2,
        horizontal: bool,
    ) -> Option<Self> {
        let row = tree.tiles.find_pane(&Pane::ToolTabs)?;
        if !tree.tiles.is_visible(row) {
            return None;
        }
        let split = tree.tiles.parent_of(row)?;
        let (dir, extent, row_height) = if horizontal {
            (
                egui_tiles::LinearDir::Vertical,
                size.y,
                toolbox::HORIZONTAL_TOOL_TABS_HEIGHT,
            )
        } else {
            (
                egui_tiles::LinearDir::Horizontal,
                size.x,
                toolbox::VERTICAL_TOOL_TABS_WIDTH,
            )
        };
        let height = Self::available_extent(tree, split, extent, dir)?;
        let egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear)) =
            tree.tiles.get(split)?
        else {
            return None;
        };
        if linear.dir != dir {
            return None;
        }
        let (count, flexible) = linear
            .children
            .iter()
            .filter(|&&child| child != row && tree.tiles.is_visible(child))
            .fold((0, 0.), |(count, total), &child| {
                (count + 1, total + linear.shares[child])
            });
        let remaining = height - count as f32 * operandi_layout::GAP - row_height;
        if flexible <= 0. || remaining <= 0. {
            return None;
        }
        let saved_share = linear.shares[row];
        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear))) =
            tree.tiles.get_mut(split)
        {
            linear
                .shares
                .set_share(row, flexible * row_height / remaining);
        }
        Some(Self {
            split,
            row,
            saved_share,
        })
    }

    /// The tile's extent along `dir` (height for Vertical, width for
    /// Horizontal), from the root's `extent` and the splits' shares.
    fn available_extent(
        tree: &egui_tiles::Tree<Pane>,
        tile: egui_tiles::TileId,
        extent: f32,
        dir: egui_tiles::LinearDir,
    ) -> Option<f32> {
        use egui_tiles::{Container, Tile};
        if Some(tile) == tree.root {
            return Some(extent);
        }
        let parent = tree.tiles.parent_of(tile)?;
        match tree.tiles.get(parent)? {
            Tile::Container(Container::Linear(linear)) => {
                let height = Self::available_extent(tree, parent, extent, dir)?;
                if linear.dir != dir {
                    return Some(height);
                }
                let (count, total) = linear
                    .children
                    .iter()
                    .filter(|&&child| tree.tiles.is_visible(child))
                    .fold((0, 0.), |(count, total), &child| {
                        (count + 1, total + linear.shares[child])
                    });
                let space = height - (count as f32 - 1.) * operandi_layout::GAP;
                (total > 0.).then(|| space * linear.shares[tile] / total)
            }
            _ => tree.tiles.rect(tile).map(|rect| {
                if dir == egui_tiles::LinearDir::Vertical {
                    rect.height()
                } else {
                    rect.width()
                }
            }),
        }
    }

    fn restore_shares(self, before: &egui_tiles::Tree<Pane>, after: &mut egui_tiles::Tree<Pane>) {
        use egui_tiles::{Container, Tile};
        if let (
            Some(Tile::Container(Container::Linear(used))),
            Some(Tile::Container(Container::Linear(now))),
        ) = (
            before.tiles.get(self.split),
            after.tiles.get_mut(self.split),
        ) {
            // Reject drags of a divider adjoining the fixed-height row.
            if now.shares[self.row] != used.shares[self.row] {
                now.shares = used.shares.clone();
            }
            now.shares.set_share(self.row, self.saved_share);
        }
    }
}

impl Operandi {
    /// The view area: every view in the main window, then torn-off views in
    /// their own windows.
    pub fn viewport(&mut self, ctx: &Context) {
        self.cameras.insert(self.active_view, self.camera.clone());
        self.displays.insert(self.active_view, self.display.clone());
        let count = self.views.viewports().len();
        let mut requests = Requests::default();
        self.corner.areas.clear();
        self.note_root_window(ctx);
        self.rescue_offscreen_windows(ctx);
        self.corner_groups();
        CentralPanel::default()
            .frame(Frame::NONE.fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                // Maximised: the group's frame shows only that view; the
                // tree (and its split positions) is left as it was.
                let maximized = self.views.maximized_in(None);
                if !self.layouts.mixed_dock
                    && let Some(view) = maximized
                {
                    self.view_pane(ui, view);
                    return;
                }
                let mut tree = std::mem::replace(
                    &mut self.views.main,
                    egui_tiles::Tree::empty("operandi-views-drawing"),
                );
                if self.layouts.mixed_dock && self.layouts.mode_bar_bottom {
                    // Jason, 2026-11-19: docked at the bottom, the mode bar
                    // runs under the views, right of Tool Properties, with
                    // the message line under Tool Properties on the same
                    // row.
                    let full = ui.max_rect();
                    let band_top = full.bottom() - MODE_BAR_HEIGHT;
                    let above = Rect::from_min_max(full.min, pos2(full.right(), band_top));
                    let mut child = ui.new_child(UiBuilder::new().max_rect(above));
                    self.draw_dock_tree(&mut child, &mut tree, &mut requests, count, maximized);
                    let props = tree
                        .tiles
                        .find_pane(&Pane::ToolProperties)
                        .filter(|&id| tree.tiles.is_visible(id))
                        .and_then(|id| tree.tiles.rect(id));
                    self.views.main = tree;
                    // Under Tool Properties when it reaches the bottom;
                    // otherwise the message takes a fixed width at the left.
                    let split = props
                        .filter(|r| r.bottom() >= band_top - 2. * operandi_layout::GAP)
                        .map_or(full.left() + 240., |r| r.right())
                        .min(full.right() - 200.);
                    let band = Rect::from_min_max(pos2(full.left(), band_top), full.max);
                    let message = Rect::from_min_max(band.min, pos2(split, band.bottom()));
                    self.message_line(ui, message);
                    let bar = Rect::from_min_max(pos2(split, band.top()), band.max);
                    let mut child = ui.new_child(
                        UiBuilder::new()
                            .id_salt("mode-bar-bottom")
                            .max_rect(bar.shrink2(vec2(4., 2.)))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    child.set_clip_rect(bar);
                    self.mode_bar(&mut child);
                    return;
                }
                if self.layouts.mixed_dock {
                    self.draw_dock_tree(ui, &mut tree, &mut requests, count, maximized);
                } else {
                    self.draw_tree(ui, &mut tree, &mut requests, count, None, true);
                }
                self.views.main = tree;
            });
        self.dock_targets_frame(ctx);
        let mut closed = Vec::new();
        for index in 0..self.views.windows.len() {
            let (id, size) = (self.views.windows[index].id, self.views.windows[index].size);
            // Toggle Palettes hid it (the UV Editor palette, LAY-53).
            if self.window_hidden(id) {
                continue;
            }
            let window = &self.views.windows[index];
            let position = window.position;
            // D-10: two layouts can both have a window numbered 1.
            // Give them different native viewports so each opens in its place.
            let viewport_id = self.dock_window_viewport(id);
            let dock_tabs = self.palette_of_window(id).is_none();
            let maximized = self.views.maximized_in(Some(id));
            let mut tree = std::mem::replace(
                &mut self.views.windows[index].tree,
                egui_tiles::Tree::empty("operandi-window-drawing"),
            );
            let title = format!(
                "{} — {}",
                self.environment.title(),
                self.views.window_title(&tree)
            );
            let title_bar = dock_tabs.then(|| self.views.window_title(&tree));
            let spec = floating::OsWindow {
                id: viewport_id,
                title,
                size,
                position,
                // Never smaller than what its panels need (VST-27).
                min_size: operandi_layout::window_min(&tree),
            };
            let shown = self.os_window(ctx, spec, |app, ctx| {
                CentralPanel::default()
                    .frame(Frame::NONE.fill(ctx.style().visuals.panel_fill))
                    .show(ctx, |ui| {
                        if let Some(title) = &title_bar {
                            app.dock_window_title(ui, floating::DockWindow::Tree(id), title);
                        }
                        let mut body = ui
                            .new_child(UiBuilder::new().max_rect(ui.available_rect_before_wrap()));
                        if let Some(view) = maximized {
                            app.view_pane(&mut body, view);
                        } else {
                            app.draw_tree(
                                &mut body,
                                &mut tree,
                                &mut requests,
                                count,
                                Some(id),
                                dock_tabs,
                            );
                        }
                    });
            });
            if !shown.open {
                closed.push(id);
            }
            let new_size = shown.size.unwrap_or(size);
            // Where the user has moved it, so it comes back there (VST-37).
            if let Some(at) = shown.position {
                self.views.windows[index].position = Some(at);
            }
            self.views.windows[index].tree = tree;
            self.views.windows[index].size = new_size;
        }
        if let Some(view) = requests.tear_off.and_then(Pane::view_key) {
            self.tear_off_view(view);
        }
        if let Some(view) = requests.close.and_then(Pane::view_key) {
            self.close_view(view);
        }
        // Side and registry panels stay available when their window closes.
        // View-only windows keep their existing close behaviour (LAY-52).
        for id in closed {
            // The Node View closes with its window (panel_close.rs).
            if self.close_panels_with_window(id) {
                continue;
            }
            if self
                .views
                .windows
                .iter()
                .any(|w| w.id == id && w.has_dock_panels())
            {
                if let Some(root) = self.views.main.root
                    && let Err(error) = self.apply_window_drop(
                        floating::DockWindow::Tree(id),
                        crate::panel_dot::DockTarget::Edge(root, operandi_layout::Side::Right),
                    )
                {
                    self.status = error;
                }
            } else {
                self.discard_view_window(id);
            }
        }
        for view in std::mem::take(&mut self.uv_views_to_close) {
            self.uv_views.remove(&view);
        }
        self.corner_requests();
        self.dot_requests();
        self.layout_requests();
        self.view_properties_frame_end();
        // Whatever happened while drawing, `camera` is the active view's.
        if let Some(camera) = self.cameras.get(&self.active_view) {
            self.camera = camera.clone();
        }
        if let Some(display) = self.displays.get(&self.active_view) {
            self.display = display.clone();
        }
        if let Some(&rect) = self.view_rects.get(&self.active_view) {
            self.viewport_rect = rect;
        }
    }

    pub(crate) fn dock_window_viewport(&self, id: u32) -> egui::ViewportId {
        if self
            .views
            .windows
            .iter()
            .any(|window| window.id == id && self.app_wide_window(window))
        {
            egui::ViewportId::from_hash_of(("operandi-view-window", id))
        } else {
            egui::ViewportId::from_hash_of(("operandi-view-window", &self.layouts.current, id))
        }
    }

    fn draw_dock_tree(
        &mut self,
        ui: &mut Ui,
        tree: &mut egui_tiles::Tree<Pane>,
        requests: &mut Requests,
        count: usize,
        maximized: Option<u32>,
    ) {
        let left = self.layouts.left_collapsed;
        let toolbox = left || self.is_floating(floating::Panel::Toolbox);
        let right = self.layouts.right_collapsed;
        let items = right || self.is_floating(floating::Panel::Items);
        let properties = right || self.is_floating(floating::Panel::Properties);
        // A Tool Tabs strip docked beside the Toolbox belongs to the left
        // column: it collapses and comes back with it (Jason, 2026-11-19).
        let strip_in_left = left
            && tree
                .tiles
                .find_pane(&Pane::ToolTabs)
                .and_then(|id| tree.tiles.parent_of(id))
                .is_some_and(|row| {
                    holds(tree, row, &[Pane::Toolbox])
                        && matches!(
                            tree.tiles.get(row),
                            Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(l)))
                                if l.dir == egui_tiles::LinearDir::Horizontal
                        )
                });
        let mut hidden: Vec<_> = tree
            .tiles
            .iter()
            .filter_map(|(&id, tile)| {
                let egui_tiles::Tile::Pane(pane) = tile else {
                    return None;
                };
                let hide = match pane {
                    Pane::Toolbox => toolbox,
                    Pane::ToolProperties => left,
                    Pane::ToolTabs => strip_in_left,
                    Pane::Items => items,
                    Pane::Properties => properties,
                    // Maximise hides the other views only; side panels
                    // and tab groups stay where they are.
                    // Empty view-area panels (Missing) hide too, or one
                    // stays as a blank cell beside the maximised view
                    // (Jason, 2026-11-19).
                    _ => {
                        pane.key().is_some() && maximized.is_some_and(|key| Some(key) != pane.key())
                    }
                };
                (hide && tree.tiles.is_visible(id)).then_some(id)
            })
            .collect();
        // A collapsed column hides whole, panels docked beside its own
        // ones included.
        for (collapsed, side) in [(left, "left"), (right, "right")] {
            if collapsed {
                for id in side_column_tiles(tree, side) {
                    if tree.tiles.is_visible(id) && !hidden.contains(&id) {
                        hidden.push(id);
                    }
                }
            }
        }
        // MN-08: a maximised panel's group shows only it.
        if let Some(pane) = self.panel_maximized {
            let more: Vec<_> = crate::panel_maximize::siblings(tree, pane)
                .into_iter()
                .filter(|&id| tree.tiles.is_visible(id) && !hidden.contains(&id))
                .collect();
            hidden.extend(more);
        }
        // A split or tab group whose every child is hidden hides too, or it
        // keeps its room as a blank cell (Jason, 2026-11-19).
        loop {
            let more: Vec<_> = tree
                .tiles
                .iter()
                .filter_map(|(&id, tile)| {
                    let egui_tiles::Tile::Container(c) = tile else {
                        return None;
                    };
                    let mut children = c.children().peekable();
                    (children.peek().is_some()
                        && children
                            .all(|child| hidden.contains(child) || !tree.tiles.is_visible(*child))
                        && !hidden.contains(&id)
                        && tree.tiles.is_visible(id))
                    .then_some(id)
                })
                .collect();
            if more.is_empty() {
                break;
            }
            hidden.extend(more);
        }
        for &id in &hidden {
            tree.tiles.set_visible(id, false);
        }
        // A grid keeps its column count when cells hide, so a maximised
        // view in a quad would fill only one column (Jason, 2026-11-19:
        // half the view area left empty). While maximised, grids holding
        // hidden views lay out as one column; their own layout and shares
        // come back untouched afterwards.
        let mut grids = Vec::new();
        // A split that holds a hidden view beside side panels would hand
        // the hidden view's room to those panels, leaving a wide blank
        // column (Jason, 2026-11-19). While maximised, a split's hidden
        // views give their share to the child holding the maximised view;
        // the saved shares come back afterwards.
        let mut linears = Vec::new();
        if let Some(key) = maximized {
            let mut keep = std::collections::HashSet::new();
            let shown = tree
                .tiles
                .iter()
                .find(|(_, t)| matches!(t, egui_tiles::Tile::Pane(p) if p.key() == Some(key)))
                .map(|(&id, _)| id);
            let mut at = shown;
            while let Some(id) = at {
                keep.insert(id);
                at = tree.tiles.parent_of(id);
            }
            let ids: Vec<_> = tree.tiles.tile_ids().collect();
            for id in ids {
                match tree.tiles.get_mut(id) {
                    Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(grid)))
                        if grid.children().any(|c| hidden.contains(c)) =>
                    {
                        grids.push((id, grid.clone()));
                        grid.layout = egui_tiles::GridLayout::Columns(1);
                    }
                    Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear))) => {
                        let gone: f32 = linear
                            .children
                            .iter()
                            .filter(|c| hidden.contains(c))
                            .map(|&c| linear.shares[c])
                            .sum();
                        let Some(&heir) = linear.children.iter().find(|c| keep.contains(c)) else {
                            continue;
                        };
                        if gone > 0. {
                            linears.push((id, linear.shares.clone()));
                            let share = linear.shares[heir] + gone;
                            linear.shares.set_share(heir, share);
                        }
                    }
                    _ => {}
                }
            }
        }
        // The split shares as they were before this frame's drawing, and
        // at the last press: the clicks of a double-click on a divider are
        // also tiny resize drags, which carried on against the wrong
        // neighbours once the column hid, so the column came back three
        // or four times as wide (Jason, 2026-11-19).
        let shares_before = linear_shares(tree);
        if ui.input(|i| i.pointer.primary_pressed()) {
            self.layouts.shares_at_press = Some(shares_before.clone());
        }
        let collapsed_before = left || right;
        self.draw_tree(ui, tree, requests, count, None, true);
        // BARS (Jason, 2026-11-19: lost with docking): double-clicking the
        // divider between a side column and the rest collapses that
        // column; inner dividers keep plain resizing.
        let double = ui.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        // Only on the divider itself, not on a menu or window above it.
        if let Some(at) = double
            && ui.ctx().layer_id_at(at) == Some(ui.layer_id())
            && let Some(bar) = side_column_divider_at(tree, at)
        {
            // The sizes from before the double-click's first press.
            if let Some(shares) = self.layouts.shares_at_press.take() {
                set_linear_shares(tree, &shares);
            }
            self.execute(&format!("operandi.bar.collapse bar:{bar} on:true"));
        } else if collapsed_before {
            // While a column is collapsed its hidden tiles keep their
            // shares, so it reopens exactly as wide as it was; the other
            // dividers still resize (Jason, 2026-11-20: with the left
            // column collapsed, the right one couldn't be dragged).
            keep_hidden_shares(tree, &shares_before, &hidden);
        }
        for (id, grid) in grids {
            if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(now))) =
                tree.tiles.get_mut(id)
            {
                *now = grid;
            }
        }
        for (id, shares) in linears {
            if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(now))) =
                tree.tiles.get_mut(id)
            {
                now.shares = shares;
            }
        }
        for id in hidden {
            tree.tiles.set_visible(id, true);
        }
    }

    fn draw_docked_side(&mut self, ui: &mut Ui, pane: Pane) {
        let area = ui.max_rect();
        let left = matches!(pane, Pane::Toolbox | Pane::ToolProperties | Pane::ToolTabs);
        let fill = if left {
            theme::chrome::background()
        } else {
            ui.style().visuals.panel_fill
        };
        ui.painter().rect_filled(area, 0., fill);
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(pane)
                .max_rect(area.shrink(toolbox::DOCK_PANEL_INSET)),
        );
        child.set_clip_rect(child.clip_rect().intersect(area));
        let body = child.max_rect();
        match pane {
            Pane::Toolbox => self.toolbox(&mut child),
            Pane::ToolTabs => self.tool_tabs_panel(&mut child, area),
            Pane::ToolProperties => self.tool_properties_panel(&mut child, body, true),
            Pane::DockTab(key) => self.draw_registry_dock_tab(&mut child, key, area, body),
            Pane::Items | Pane::Properties => {
                let lower = pane == Pane::Properties;
                let history = self.dock_source_in_main(Self::dot_for_pane(pane))
                    && (lower
                        || self.lower_frame.tabs.is_empty()
                        || self.is_floating(floating::Panel::Properties)
                        || self.views.window_of(Pane::Properties).is_some());
                let mut group = body;
                if history {
                    group.max.y = (group.max.y - self.command_history_reserved()).max(group.min.y);
                }
                self.group_section(&mut child, group, area, lower);
                if history {
                    let rect = Rect::from_min_max(pos2(body.min.x, group.max.y), body.max);
                    let mut history = child.new_child(UiBuilder::new().max_rect(rect));
                    self.command_history_dock(&mut history);
                }
            }
            _ => {}
        }
    }

    fn draw_registry_dock_tab(&mut self, ui: &mut Ui, key: u32, area: Rect, body: Rect) {
        let name = self.views.title_of(Pane::DockTab(key));
        let kind = layouts::kind_named(&name).unwrap_or(panel_registry::PanelKind::Missing(0));
        let header = Rect::from_min_size(body.min, vec2(body.width(), 22.));
        ui.painter()
            .rect_filled(header, 0., theme::workspace::tab_strip());
        ui.painter().with_clip_rect(header).text(
            header.left_center() + vec2(2., 0.),
            Align2::LEFT_CENTER,
            &name,
            crate::typography::standard_font(),
            theme::text(),
        );
        self.registry_tab_close(ui, kind, Pane::DockTab(key), header, area);
        self.panel_dot(
            ui,
            crate::panel_dot::Dot::Tab(key),
            area,
            Some(crate::panel_dot::header_edge(
                area,
                header.bottom() - area.top(),
            )),
        );
        let body = Rect::from_min_max(pos2(body.left(), header.bottom()), body.max);
        self.registry_panel_body(ui, area, body, kind, &name);
    }

    /// Draw one window's tree of views. Lock Divider Dragging puts every
    /// split's sizes back as they were, whatever the dividers did; else a
    /// divider stops where a panel on either side reaches its minimum
    /// (VST-25).
    fn draw_tree(
        &mut self,
        ui: &mut Ui,
        tree: &mut egui_tiles::Tree<Pane>,
        requests: &mut Requests,
        count: usize,
        window: Option<u32>,
        dock_tabs: bool,
    ) {
        // Some(true) / Some(false): a standalone strip docked as a row or
        // a column; None: the stock strip inside the toolbox.
        let fixed = self
            .layouts
            .tool_tabs_horizontal
            .filter(|_| window.is_none())
            .and_then(|horizontal| FixedToolTabs::size_row(tree, ui.available_size(), horizontal));
        let before = tree.clone();
        if dock_tabs && tree.dragged_id(ui.ctx()).is_some() {
            ui.ctx().stop_dragging();
        }
        tree.ui(
            &mut Views {
                app: self,
                requests,
                count,
                window,
                dock_tabs,
                tab_row: None,
            },
            ui,
        );
        // UIC-155 / D-05: main-window tabs share the app's docking
        // gesture. Stop the library from applying its own drop.
        if dock_tabs && tree.dragged_id(ui.ctx()).is_some() {
            ui.ctx().stop_dragging();
        }
        if self.layout_blocked(ui.ctx()) {
            operandi_layout::restore_dividers(&before, tree);
        } else {
            operandi_layout::clamp_dividers(&before, tree);
        }
        if let Some(fixed) = fixed {
            fixed.restore_shares(&before, tree);
        }
    }

    /// Once, before the saved floating view windows first open: one whose
    /// place is on no connected screen any more (a monitor was unplugged)
    /// reopens on the main screen near the main window (VST-37).
    fn rescue_offscreen_windows(&mut self, ctx: &Context) {
        if std::mem::replace(&mut self.dots.windows_rescued, true) || self.views.windows.is_empty()
        {
            return;
        }
        let screens = window_state::screens(ctx.pixels_per_point());
        let main_window = self.main_window.state.position;
        let mut moved = false;
        for window in &mut self.views.windows {
            let place = window_state::reopen_place(
                window.position,
                window.size,
                screens.as_deref(),
                main_window,
            );
            if place != window.position {
                window.position = place;
                moved = true;
            }
        }
        if moved {}
    }

    /// Close a floating view window: its views are discarded.
    pub(super) fn discard_view_window(&mut self, window: u32) {
        let panes = self.views.discard_window(window);
        if !panes.is_empty() {
            self.forget_unlisted_views();
        }
    }

    /// Draw one view: make it active on a press inside it, otherwise swap
    /// its camera in for the drawing and take no input.
    fn view_pane(&mut self, ui: &mut Ui, view: u32) {
        let area = ui.max_rect();
        if ui.ctx().viewport_id() == egui::ViewportId::ROOT {
            self.corner.areas.insert(view, area);
        }
        // A press on the Viewport Properties popover (which may hang over
        // another view) doesn't make that view active.
        let popover = self.corner.popover_rect;
        let pressed_here = ui.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| area.contains(p) && !popover.is_some_and(|r| r.contains(p)))
        });
        // The wheel zooms the view under the pointer, which becomes the
        // active one, as the view keys do: no click first (Jason,
        // 2026-11-08).
        let scrolled_here = view != self.active_view
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.input(|i| {
                i.smooth_scroll_delta.y != 0.
                    && i.pointer.hover_pos().is_some_and(|p| {
                        area.contains(p) && !popover.is_some_and(|r| r.contains(p))
                    })
            });
        if scrolled_here && self.navigation.is_none() {
            self.activate_view(view);
        }
        if pressed_here && self.navigation.is_none() {
            self.view_pressed();
            self.activate_view(view);
        }
        self.drawing_view = view;
        if view == self.active_view {
            ui.push_id(("view", view), |ui| {
                if self.is_uv_view(view) {
                    self.uv_view_ui(ui, view, true);
                } else {
                    // An untouched transform tool goes back to 3D here.
                    self.sync_uv_tool_space(false);
                    self.view_ui(ui);
                    self.view_properties_popover(ui, view);
                    // The one-time Trackball Rotation offer, in the view
                    // just orbited (trackball_offer.rs).
                    self.trackball_offer_ui(ui.ctx(), self.viewport_rect);
                }
            });
            self.cameras.insert(view, self.camera.clone());
            self.displays.insert(view, self.display.clone());
            self.view_rects.insert(view, self.viewport_rect);
        } else {
            let camera = self.cameras.get(&view).cloned().unwrap_or_default();
            let saved_camera = std::mem::replace(&mut self.camera, camera);
            let saved_rect = self.viewport_rect;
            let display = self
                .displays
                .get(&view)
                .cloned()
                .unwrap_or_else(|| self.new_view_display());
            let saved_display = std::mem::replace(&mut self.display, display);
            self.view_passive = true;
            ui.push_id(("view", view), |ui| {
                if self.is_uv_view(view) {
                    self.uv_view_ui(ui, view, false);
                } else {
                    self.view_ui(ui);
                    self.view_properties_popover(ui, view);
                }
            });
            self.view_passive = false;
            self.displays
                .insert(view, std::mem::replace(&mut self.display, saved_display));
            self.cameras
                .insert(view, std::mem::replace(&mut self.camera, saved_camera));
            self.view_rects
                .insert(view, std::mem::replace(&mut self.viewport_rect, saved_rect));
        }
        self.drawing_view = self.active_view;
    }

    /// Make `view` the active one: its camera and display become `camera`
    /// and `display`.
    pub(super) fn activate_view(&mut self, view: u32) {
        if view == self.active_view {
            return;
        }
        self.cameras.insert(self.active_view, self.camera.clone());
        self.displays.insert(self.active_view, self.display.clone());
        self.active_view = view;
        self.camera = self.cameras.get(&view).cloned().unwrap_or_default();
        self.display = self
            .displays
            .get(&view)
            .cloned()
            .unwrap_or_else(|| self.new_view_display());
        if let Some(&rect) = self.view_rects.get(&view) {
            self.viewport_rect = rect;
        }
    }

    /// View > Split View (the dot menu's Split Horizontal): the active
    /// view's place splits at half, side by side; the right half is a copy
    /// of it (camera and display settings) and becomes the active view.
    pub(super) fn split_view(&mut self) {
        self.split_panel(self.active_view, operandi_layout::Side::Right);
    }

    /// Close one view (View > Close View, the dot menu's Delete): its
    /// neighbours fill the space. Never the main window's last view.
    pub(super) fn close_view(&mut self, view: u32) {
        let pane = self.view_pane_of(view);
        match self.views.remove(pane) {
            Ok(()) => {
                self.forget_unlisted_views();
            }
            Err(error) => self.status = error,
        }
    }

    /// Whether Delete / Close View applies to `view`: not the main
    /// window's last view, not a view alone in a floating window, and not
    /// a maximised view.
    pub(super) fn can_close_view(&self, view: u32) -> bool {
        let pane = self.view_pane_of(view);
        !self.views.last_in_main(pane)
            && !self.views.alone_in_window(pane)
            && !self.views.is_maximized(view)
    }

    /// Layouts migration only (layouts.rs): the old view-layout.json and
    /// view-displays.json, read once as the Model layout's views. A file
    /// that doesn't load leaves a single view.
    pub(super) fn load_views(&mut self) {
        if let Ok(text) = std::fs::read_to_string(self.environment.paths().view_layout())
            && let Ok(views) = operandi_layout::Layout::from_json(&text)
            && views.panes().iter().all(|p| p.view_key().is_some())
        {
            self.views = views;
            for view in self.views.viewports() {
                self.cameras.insert(view, self.camera.clone());
            }
            // A view maximised at quit is the one to work in.
            self.active_view = self
                .views
                .maximized_in(None)
                .unwrap_or(self.views.viewports()[0]);
        }
        // Each view's display settings; a file that doesn't load (or a view
        // it doesn't name) leaves the defaults.
        if let Ok(text) = std::fs::read_to_string(self.environment.paths().view_displays())
            && let Ok(saved) = serde_json::from_str::<BTreeMap<u32, view_display::Settings>>(&text)
        {
            for view in self.views.viewports() {
                if let Some(&settings) = saved.get(&view) {
                    self.displays.insert(
                        view,
                        view_display::ViewDisplay {
                            settings,
                            applied: None,
                        },
                    );
                }
            }
            if let Some(display) = self.displays.get(&self.active_view) {
                self.display = display.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw, pointer};

    const SIZE: Vec2 = vec2(1440., 900.);

    #[test]
    fn one_view_looks_as_before() {
        let mut a = box_app();
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert_eq!(a.views.viewports(), [0]);
        assert_eq!(a.view_rects[&0], a.viewport_rect);
        assert!(a.viewport_rect.width() > 500.);
    }

    #[test]
    fn split_views_keep_their_own_cameras() {
        let mut a = box_app();
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        a.split_view();
        // The new (right) view is active after a split; start from the first.
        assert_eq!(a.active_view, 1);
        a.activate_view(0);
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert_eq!(a.views.viewports(), [0, 1]);
        let (first, second) = (a.view_rects[&0], a.view_rects[&1]);
        assert!(!first.intersects(second), "{first:?} {second:?}");
        // Change the active (first) view's camera; the second keeps its own.
        a.camera.view = View::Front;
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.cameras[&0].view, View::Front);
        assert_ne!(a.cameras[&1].view, View::Front);
        // A press in the second view makes it active, with its own camera.
        let at = second.center();
        draw(&mut a, &ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(at, false, Modifiers::NONE)],
        );
        assert_eq!(a.active_view, 1);
        assert_ne!(a.camera.view, View::Front);
        assert_eq!(a.viewport_rect, a.view_rects[&1]);
        // Closing it goes back to one view.
        a.close_view(1);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.views.viewports(), [0]);
        assert_eq!(a.active_view, 0);
        assert_eq!(a.camera.view, View::Front);
    }

    #[test]
    fn split_views_keep_their_own_display_settings() {
        use crate::view_display::Style;
        let mut a = box_app();
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        a.display.settings.drawing.uv_seams = true;
        a.split_view();
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        // A new view starts from the active one's settings.
        assert!(a.displays[&1].settings.drawing.uv_seams);
        // The commands and the Alt choice change only the active view, and
        // no scene history.
        let history = a.editor.history.undo.len();
        a.execute("wireframe view");
        a.display.set_style(Style::Gooch, true);
        a.display.settings.drawing.grid = false;
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.displays[&0].settings.active.style, Style::Wireframe);
        assert_eq!(a.displays[&0].settings.inactive.style, Style::Gooch);
        assert_eq!(a.displays[&1].settings.active.style, Style::Default);
        assert_eq!(a.displays[&1].settings.inactive.style, Style::Default);
        assert!(a.displays[&1].settings.drawing.grid);
        assert_eq!(a.editor.history.undo.len(), history);
        // The second view, made active, brings its own back.
        let at = a.view_rects[&1].center();
        draw(&mut a, &ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(at, false, Modifiers::NONE)],
        );
        assert_eq!(a.active_view, 1);
        assert_eq!(a.display.settings.active.style, Style::Default);
        a.execute("shaded view");
        a.close_view(1);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.display.settings.active.style, Style::Wireframe);
        assert!(!a.display.settings.drawing.grid);
    }
}
