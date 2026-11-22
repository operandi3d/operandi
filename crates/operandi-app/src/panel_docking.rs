//! D-02..D-04: the corner-dot move's targets in the main workspace.
//!
//! UIC-72 dock guide (Jason, 2026-11-20, from contributor feedback): while a
//! panel is dragged, a five-way cross (left, right, top, bottom, centre =
//! tab) pops up over the panel under the pointer, and four fixed arrows sit
//! at the work area's outer edges for docking against the whole window.
//! Tab rows still take a drop between tabs. A drop anywhere else floats the
//! panel. A translucent preview shows where it will land.
use super::*;
use egui_tiles::TileId;

/// The share of the views' height a newly docked Node View takes.
const NODE_VIEW_SHARE: f32 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockTarget {
    Compass(Dot, Zone),
    PanelTabs(Dot, usize),
    TreeTabs(TileId, usize),
    Edge(TileId, Side),
}

impl DockTarget {
    fn side(self) -> Option<Side> {
        match self {
            Self::Compass(_, Zone::Edge(side)) | Self::Edge(_, side) => Some(side),
            _ => None,
        }
    }

    fn targets_tool_tabs(self) -> bool {
        matches!(
            self,
            Self::Compass(Dot::ToolTabs, _) | Self::PanelTabs(Dot::ToolTabs, _)
        )
    }
}

/// A tab carries only its own content, independently of the row's grip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabSource {
    Panel(Panel, panel_registry::PanelKind),
    Tree {
        window: Option<u32>,
        row: TileId,
        tile: TileId,
    },
}

pub(super) struct TabDrag {
    source: TabSource,
    row: Rect,
    pointer: Pos2,
    size: Vec2,
}

#[derive(Clone, Copy)]
struct Region {
    target: DockTarget,
    rect: Rect,
}

#[derive(Clone, Copy)]
struct TabHit {
    index: usize,
    rect: Rect,
}

struct TabRow<T> {
    group: T,
    rect: Rect,
    tabs: Vec<TabHit>,
    len: usize,
    body_size: Vec2,
}

impl<T: Copy> TabRow<T> {
    fn hit(&self, x: f32) -> (usize, f32) {
        for tab in &self.tabs {
            if x <= tab.rect.center().x {
                return (tab.index, tab.rect.left());
            }
            if x < tab.rect.right() {
                return (tab.index + 1, tab.rect.right());
            }
        }
        (
            self.len,
            self.tabs
                .last()
                .map_or(self.rect.left(), |tab| tab.rect.right()),
        )
    }
}

#[derive(Default)]
pub(super) struct Targets {
    pub(super) work: Option<Rect>,
    window: Option<Rect>,
    previous_work: Option<Rect>,
    /// The four fixed arrows at the work area's outer edges.
    regions: Vec<Region>,
    /// The main window's panels, on screen: the one under the pointer gets
    /// the five-way guide (worked out per hit, so it never lags a frame).
    panels: Vec<(Dot, Rect)>,
    /// Last frame's tab rows, every window, on screen: UI build mode's
    /// covers leave them free so tabs still click and drag.
    last_rows: Vec<Rect>,
    panel_rows: Vec<TabRow<Dot>>,
    tree_rows: Vec<TabRow<TileId>>,
    drawn_panel_rows: Vec<TabRow<Dot>>,
    drawn_tree_rows: Vec<TabRow<TileId>>,
    float_panel_rows: Vec<TabRow<Dot>>,
    drawn_float_panel_rows: Vec<TabRow<Dot>>,
    float_tree_rows: Vec<TabRow<(u32, TileId)>>,
    drawn_float_tree_rows: Vec<TabRow<(u32, TileId)>>,
}

impl Targets {
    /// Tab rows drawn this frame so far, and last frame's (screen).
    pub(super) fn tab_row_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.last_rows
            .iter()
            .copied()
            .chain(self.drawn_panel_rows.iter().map(|r| r.rect))
            .chain(self.drawn_tree_rows.iter().map(|r| r.rect))
            .chain(self.drawn_float_panel_rows.iter().map(|r| r.rect))
            .chain(self.drawn_float_tree_rows.iter().map(|r| r.rect))
    }

    pub(super) fn has_work_area(&self) -> bool {
        self.work
            .or(self.previous_work)
            .is_some_and(|work| work.is_positive())
    }

    pub(super) fn contains(&self, at: Pos2) -> bool {
        self.previous_work.is_some_and(|r| r.contains(at))
    }

    /// The panel the guide pops up over: the innermost one under the
    /// pointer, unless that is the panel being dragged.
    fn guide(&self, source: Option<Dot>, at: Pos2) -> Option<(Dot, Rect)> {
        self.panels
            .iter()
            .filter(|(_, rect)| rect.contains(at))
            .min_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
            .copied()
            .filter(|&(dot, _)| Some(dot) != source)
    }

    pub(super) fn hit(&self, source: Option<Dot>, at: Pos2) -> Option<DockTarget> {
        if let Some(region) = self.regions.iter().find(|r| r.rect.contains(at)) {
            return Some(region.target);
        }
        if let Some(row) = self
            .panel_rows
            .iter()
            .find(|r| Some(r.group) != source && r.rect.contains(at))
        {
            return Some(DockTarget::PanelTabs(row.group, row.hit(at.x).0));
        }
        if let Some(row) = self.tree_rows.iter().find(|r| r.rect.contains(at)) {
            return Some(DockTarget::TreeTabs(row.group, row.hit(at.x).0));
        }
        let (dot, rect) = self.guide(source, at)?;
        compass(dot, rect)
            .find(|r| r.rect.contains(at))
            .map(|r| r.target)
    }

    /// Where a drop on `target` lands, for the translucent preview.
    fn landing_of(&self, target: DockTarget) -> Option<Rect> {
        let panel = |dot| {
            self.panels
                .iter()
                .find(|&&(d, _)| d == dot)
                .map(|&(_, rect)| rect)
        };
        match target {
            DockTarget::Compass(dot, zone) => Some(landing(panel(dot)?, zone)),
            // The root splits in two (operandi_layout::Layout::dock_tile_at).
            DockTarget::Edge(_, side) => Some(landing(self.previous_work?, Zone::Edge(side))),
            DockTarget::PanelTabs(dot, _) => panel(dot),
            DockTarget::TreeTabs(id, _) => self.tree_rows.iter().find(|r| r.group == id).map(|r| {
                Rect::from_min_size(
                    r.rect.min,
                    vec2(r.rect.width(), r.rect.height() + r.body_size.y),
                )
            }),
        }
    }
}

impl Operandi {
    #[cfg(test)]
    pub(crate) fn compass_point(&self, dot: Dot, zone: Zone) -> Pos2 {
        compass(dot, self.dot_panel_rect(dot).unwrap())
            .find(|r| r.target == DockTarget::Compass(dot, zone))
            .unwrap()
            .rect
            .center()
    }

    #[cfg(test)]
    pub(crate) fn dock_hit(&self, source: Dot, at: Pos2) -> Option<DockTarget> {
        self.dots.docking.hit(Some(source), at)
    }

    #[cfg(test)]
    pub(crate) fn dock_slots_at_rest(&self) -> bool {
        let targets = &self.dots.docking;
        targets.regions.is_empty()
            && targets.panels.is_empty()
            && targets.panel_rows.is_empty()
            && targets.tree_rows.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn dock_tree_tab_point(&self, group: TileId, index: usize) -> Pos2 {
        let row = self
            .dots
            .docking
            .tree_rows
            .iter()
            .chain(&self.dots.docking.drawn_tree_rows)
            .find(|r| r.group == group)
            .unwrap();
        let tab = row.tabs.iter().find(|tab| tab.index == index).unwrap().rect;
        pos2(tab.left() + 2., tab.center().y)
    }

    #[cfg(test)]
    pub(crate) fn dock_region_point(&self, target: DockTarget) -> Pos2 {
        self.dots
            .docking
            .regions
            .iter()
            .find(|r| r.target == target)
            .unwrap()
            .rect
            .center()
    }

    /// The four outer arrows, in Top, Bottom, Left, Right order.
    #[cfg(test)]
    pub(crate) fn dock_test_strips(&self) -> [(DockTarget, Pos2); 4] {
        let mut layout = self.views.clone();
        self.attach_dock_columns(&mut layout).unwrap();
        let id = layout.main.root.unwrap();
        [Side::Top, Side::Bottom, Side::Left, Side::Right].map(|side| {
            let target = DockTarget::Edge(id, side);
            (target, self.dock_region_point(target))
        })
    }

    /// The panel the five-way guide is over at `at` during a drag of `source`.
    #[cfg(test)]
    pub(crate) fn dock_guide_at(&self, source: Option<Dot>, at: Pos2) -> Option<Dot> {
        self.dots.docking.guide(source, at).map(|(dot, _)| dot)
    }

    pub(super) fn outside_main(&self, at: Pos2) -> bool {
        self.dots
            .docking
            .window
            .is_some_and(|rect| !rect.contains(at))
    }

    pub(crate) fn dock_work_area(&mut self, ctx: &Context) {
        let targets = &mut self.dots.docking;
        targets.last_rows = (targets.drawn_panel_rows.iter().map(|r| r.rect))
            .chain(targets.drawn_tree_rows.iter().map(|r| r.rect))
            .chain(targets.drawn_float_panel_rows.iter().map(|r| r.rect))
            .chain(targets.drawn_float_tree_rows.iter().map(|r| r.rect))
            .chain(targets.float_panel_rows.iter().map(|r| r.rect))
            .chain(targets.float_tree_rows.iter().map(|r| r.rect))
            .collect();
        self.dots.docking.window = Some(ctx.viewport_rect().translate(window_origin(ctx)));
        self.dots.docking.work = Some(ctx.available_rect().translate(window_origin(ctx)));
        self.dots.docking.drawn_panel_rows.clear();
        self.dots.docking.drawn_tree_rows.clear();
        let targets = &mut self.dots.docking;
        targets.float_panel_rows = std::mem::take(&mut targets.drawn_float_panel_rows);
        targets.float_tree_rows = std::mem::take(&mut targets.drawn_float_tree_rows);
    }

    pub(crate) fn dock_panel_tab_row(&mut self, ui: &Ui, panel: Panel, rect: Rect) {
        let in_main = self.dock_source_in_main(Dot::Side(panel));
        let group = if panel == Panel::Items {
            &self.upper_frame
        } else {
            &self.lower_frame
        };
        let origin = window_origin(ui.ctx());
        let mut tabs: Vec<_> = group
            .rects
            .iter()
            .enumerate()
            .filter_map(|(index, &rect)| {
                rect.is_positive().then_some(TabHit {
                    index,
                    rect: rect.translate(origin),
                })
            })
            .collect();
        tabs.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
        let targets = &mut self.dots.docking;
        let rows = if in_main {
            &mut targets.drawn_panel_rows
        } else {
            &mut targets.drawn_float_panel_rows
        };
        rows.push(TabRow {
            group: Dot::Side(panel),
            rect: rect.translate(origin),
            tabs,
            len: group.tabs.len(),
            body_size: Vec2::ZERO,
        });
    }

    pub(crate) fn dock_tree_row(
        &mut self,
        ui: &Ui,
        window: Option<u32>,
        group: TileId,
        len: usize,
        body_size: Vec2,
    ) {
        let rect = ui.max_rect().translate(window_origin(ui.ctx()));
        let targets = &mut self.dots.docking;
        if let Some(window) = window {
            targets.drawn_float_tree_rows.push(TabRow {
                group: (window, group),
                rect,
                tabs: Vec::new(),
                len,
                body_size,
            });
        } else {
            targets.drawn_tree_rows.push(TabRow {
                group,
                rect,
                tabs: Vec::new(),
                len,
                body_size,
            });
        }
    }

    pub(crate) fn dock_tree_tab(
        &mut self,
        response: &Response,
        window: Option<u32>,
        group: TileId,
        tile: TileId,
        index: usize,
    ) {
        let origin = window_origin(&response.ctx);
        let hit = TabHit {
            index,
            rect: response.interact_rect.translate(origin),
        };
        fn add_hit<T: Copy>(row: &mut TabRow<T>, hit: TabHit) -> (Rect, Vec2) {
            if hit.rect.is_positive() {
                row.tabs.push(hit);
            }
            (row.rect, row.body_size)
        }
        let targets = &mut self.dots.docking;
        let measured = if let Some(window) = window {
            targets
                .drawn_float_tree_rows
                .iter_mut()
                .find(|r| r.group == (window, group))
                .map(|row| add_hit(row, hit))
        } else {
            targets
                .drawn_tree_rows
                .iter_mut()
                .find(|r| r.group == group)
                .map(|row| add_hit(row, hit))
        };
        if let Some((row, size)) = measured {
            self.track_tab_drag(
                response,
                TabSource::Tree {
                    window,
                    row: group,
                    tile,
                },
                row.translate(-origin),
                size,
            );
        }
    }

    /// UIC-155 / D-05: a shallow band around the row is for reordering.
    /// Everything stays in the source row until a valid drop commits.
    pub(crate) fn track_tab_drag(
        &mut self,
        response: &Response,
        source: TabSource,
        row: Rect,
        size: Vec2,
    ) {
        let ctx = &response.ctx;
        if self
            .dots
            .tab_drag
            .as_ref()
            .is_some_and(|drag| drag.source == source)
        {
            self.cancel_dot_drag(ctx);
        }
        let origin = window_origin(ctx);
        let Some(at) = ctx.input(|i| i.pointer.latest_pos()).map(|p| p + origin) else {
            return;
        };
        if response.drag_started_by(PointerButton::Primary) && self.panel_drags_locked() {
            self.hint_build_mode();
        }
        if response.drag_started_by(PointerButton::Primary)
            && !self.panel_drags_locked()
            && self.dots.drag.is_none()
            && self.dots.tab_drag.is_none()
            && self.dots.window_drag.is_none()
        {
            self.dots.tab_drag = Some(TabDrag {
                source,
                pointer: at,
                row: row.translate(origin),
                size,
            });
        }
        if let Some(drag) = &mut self.dots.tab_drag
            && drag.source == source
        {
            drag.pointer = at;
            if ctx.input(|i| i.pointer.primary_released()) {
                if let Some(drag) = self.dots.tab_drag.take() {
                    let over = self.tab_over(drag.source, drag.row, at);
                    self.dots
                        .requests
                        .push(Request::TabDrop(source, over, at, drag.size));
                }
            } else if !ctx.input(|i| i.pointer.primary_down()) {
                self.dots.tab_drag = None;
            } else {
                ctx.set_cursor_icon(CursorIcon::Grabbing);
                ctx.request_repaint();
                let row = drag.row;
                if row.expand(6.).contains(at)
                    && let Some((_, x)) = self.tab_insert(source, at.x)
                {
                    ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Foreground,
                        Id::new("panel-dot-overlay"),
                    ))
                    .rect_filled(
                        insert_mark(row, x).translate(-origin),
                        0.,
                        theme::workspace::drop_indicator(),
                    );
                }
            }
        }
    }

    fn tab_insert(&self, source: TabSource, x: f32) -> Option<(usize, f32)> {
        let targets = &self.dots.docking;
        match source {
            TabSource::Panel(panel, _) => targets
                .panel_rows
                .iter()
                .chain(&targets.float_panel_rows)
                .find(|r| r.group == Dot::Side(panel))
                .map(|r| r.hit(x)),
            TabSource::Tree {
                window: None, row, ..
            } => targets
                .tree_rows
                .iter()
                .find(|r| r.group == row)
                .map(|r| r.hit(x)),
            TabSource::Tree {
                window: Some(window),
                row,
                ..
            } => targets
                .float_tree_rows
                .iter()
                .find(|r| r.group == (window, row))
                .map(|r| r.hit(x)),
        }
    }

    fn tab_over(&self, source: TabSource, row: Rect, at: Pos2) -> Over {
        let targets = &self.dots.docking;
        if row.expand(6.).contains(at) {
            return self
                .tab_insert(source, at.x)
                .map_or(Over::Nothing, |(index, _)| Over::TabReorder(index));
        }
        // UIC-72 dock guide: off every target, the tab floats where it's
        // dropped, inside the main window or out of it.
        targets.hit(None, at).map_or(Over::Outside, Over::Dock)
    }

    pub(crate) fn dot_for_pane(pane: Pane) -> Dot {
        match pane {
            Pane::Viewport(key) | Pane::Uv(key) | Pane::Missing(key) => Dot::View(key),
            Pane::Toolbox => Dot::Side(Panel::Toolbox),
            Pane::Items => Dot::Side(Panel::Items),
            Pane::Properties => Dot::Side(Panel::Properties),
            Pane::ToolProperties => Dot::ToolProperties,
            Pane::ToolTabs => Dot::ToolTabs,
            Pane::DockTab(key) => Dot::Tab(key),
        }
    }

    pub(crate) fn dock_pane(&self, dot: Dot) -> Pane {
        match dot {
            Dot::View(key) => self.view_pane_of(key),
            Dot::Side(panel) => panel.pane(),
            Dot::ToolProperties => Pane::ToolProperties,
            Dot::ToolTabs => Pane::ToolTabs,
            Dot::Tab(key) => Pane::DockTab(key),
        }
    }

    pub(crate) fn dock_source_in_main(&self, dot: Dot) -> bool {
        match dot {
            Dot::View(key) => self.dots.view_windows.get(&key).is_none_or(Option::is_none),
            Dot::Side(panel) if self.is_floating(panel) => false,
            dot => !self.dots.dock_windows.contains_key(&self.dock_pane(dot)),
        }
    }

    pub(crate) fn dock_group_visible(&self, panel: Panel) -> bool {
        let dot = Dot::Side(panel);
        if self.is_floating(panel) {
            return self.dot_panel_rect(dot).is_some();
        }
        if self.layouts.right_collapsed && self.dock_source_in_main(dot) {
            return false;
        }
        self.dock_pane_visible(self.dock_pane(dot), dot)
    }

    pub(crate) fn registry_dock_pane(&self, kind: panel_registry::PanelKind) -> Option<Pane> {
        self.views.dock_tabs.iter().find_map(|(&key, slot)| {
            (super::layouts::kind_named(&slot.kind) == Some(kind)).then_some(Pane::DockTab(key))
        })
    }

    // UIC-157: the Window menu and the recovery command use the same registry
    // and look in both sidebar rows and the detached dock panes.
    pub(crate) fn registry_panel_open(&self, kind: panel_registry::PanelKind) -> bool {
        self.upper_frame.contains(kind)
            || self.lower_frame.contains(kind)
            || self.registry_dock_pane(kind).is_some()
    }

    pub(crate) fn reveal_registry_panel(&mut self, kind: panel_registry::PanelKind) -> Result<()> {
        // The Node View is an editor that wants room: its first place is
        // under the views, full width of them, not a side column's tab row.
        // From there it docks, tabs and floats like any panel.
        if kind == panel_registry::PanelKind::NodeView && !self.registry_panel_open(kind) {
            let mut next = self.views.clone();
            self.attach_dock_columns(&mut next)?;
            if next.view_area_tile().is_some() {
                let slot = operandi_layout::Slot::new(panel_registry::panel_type(kind).name);
                let (_, source) = next.new_dock_tab(slot);
                next.dock_tile_below_views(source, NODE_VIEW_SHARE)?;
                self.install_dock_tree(next);
            }
        }
        let rows = [Panel::Items, Panel::Properties];
        let docked_row = |row: Panel| {
            (!self.layouts.mixed_dock || self.views.dock_slot(row.pane()).is_some())
                && !self.is_floating(row)
        };
        let pane = if let Some(pane) = self.registry_dock_pane(kind) {
            pane
        } else if let Some(row) = rows
            .into_iter()
            .find(|&row| self.sidebar_group(row).contains(kind))
            .or_else(|| rows.into_iter().find(|&row| docked_row(row)))
        {
            let group = self.sidebar_group_mut(row);
            if let Some(index) = group.tabs.iter().position(|&tab| tab == kind) {
                group.show_page(index);
            } else {
                group.add(kind);
            }
            row.pane()
        } else {
            let mut next = self.views.clone();
            self.attach_dock_columns(&mut next)?;
            let root = next.main.root.ok_or("No dock target")?;
            let row = rows
                .into_iter()
                .find(|&row| next.window_of(row.pane()).is_none() && !self.is_floating(row));
            let (pane, source) = if let Some(row) = row {
                (row.pane(), next.main.tiles.insert_pane(row.pane()))
            } else {
                // Both sidebar rows float. Restore this panel on the right
                // of the main window and leave the rows floating.
                next.new_dock_tab(self.registry_slot(self.sidebar_group(Panel::Items), kind))
            };
            next.dock_tile_at(source, root, Side::Right, None)?;
            if let Some(row) = row {
                self.sidebar_group_mut(row).add(kind);
            }
            self.install_dock_tree(next);
            pane
        };
        self.views.reveal_docked_tab(pane);
        self.layouts.right_collapsed = false;
        self.palettes.hidden = false;
        self.activate_dot(Self::dot_for_pane(pane));
        if let Some(ctx) = &self.frame_ctx {
            let viewport = if let Some(id) = self.views.window_of(pane) {
                self.dock_window_viewport(id)
            } else if let Some(panel) = Panel::ALL
                .into_iter()
                .find(|&panel| pane == panel.pane() && self.is_floating(panel))
            {
                egui::ViewportId::from_hash_of(("operandi-panel-window", panel))
            } else {
                egui::ViewportId::ROOT
            };
            ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Focus);
        }
        Ok(())
    }

    pub(crate) fn dock_tab_visible(&self, kind: panel_registry::PanelKind) -> bool {
        self.registry_dock_pane(kind)
            .is_some_and(|pane| self.dock_pane_visible(pane, Self::dot_for_pane(pane)))
    }

    fn dock_pane_visible(&self, pane: Pane, dot: Dot) -> bool {
        let tree = if let Some(id) = self.dots.dock_windows.get(&pane) {
            let Some(window) = self.views.windows.iter().find(|w| w.id == *id) else {
                return false;
            };
            &window.tree
        } else {
            &self.views.main
        };
        if tree.root.is_none() {
            // The tree being drawn temporarily lives outside `views`.
            return self.dot_panel_rect(dot).is_some();
        }
        tree.tiles
            .find_pane(&pane)
            .is_some_and(|id| tree.active_tiles().contains(&id))
    }

    /// Build the shared hit and paint geometry after all main-window panels
    /// have drawn. Grip handling can then use it on the next input frame.
    pub(super) fn work_edge_target(&self, side: Side) -> Result<DockTarget> {
        let mut layout = self.views.clone();
        self.attach_dock_columns(&mut layout)?;
        let root = layout.main.root.ok_or("No dock target")?;
        Ok(DockTarget::Edge(root, side))
    }

    pub(crate) fn dock_targets_frame(&mut self, ctx: &Context) {
        self.build_mode_exit_button(ctx);
        let Some(work) = self.dots.docking.work else {
            return;
        };
        self.dots.docking.previous_work = Some(work);
        let grip = self
            .dots
            .drag
            .as_ref()
            .filter(|d| d.gesture == Gesture::Move)
            .map(|drag| drag.dot);
        if grip.is_none() && self.dots.tab_drag.is_none() && self.dots.window_drag.is_none() {
            self.dots.docking.regions.clear();
            self.dots.docking.panels.clear();
            self.dots.docking.panel_rows.clear();
            self.dots.docking.tree_rows.clear();
            return;
        }
        let origin = window_origin(ctx);
        let mut layout = self.views.clone();
        if self.attach_dock_columns(&mut layout).is_err() {
            return;
        }
        let Some(root) = layout.main.root else { return };
        let panels = self
            .dots
            .drawn
            .iter()
            .filter(|d| d.in_main)
            .map(|d| (d.dot, d.rect))
            .collect();
        let targets = &mut self.dots.docking;
        targets.regions = outer_regions(root, work).collect();
        targets.panels = panels;
        targets.panel_rows = std::mem::take(&mut targets.drawn_panel_rows);
        targets.tree_rows = std::mem::take(&mut targets.drawn_tree_rows);
        let (at, over) = if let Some(tab) = &self.dots.tab_drag {
            let over = self.tab_over(tab.source, tab.row, tab.pointer);
            if tab.row.expand(6.).contains(tab.pointer) {
                return;
            }
            (tab.pointer, over)
        } else if let Some(drag) = &self.dots.window_drag {
            let at = drag.pointer;
            (at, targets.hit(None, at).map_or(Over::Nothing, Over::Dock))
        } else if let Some(source) = grip {
            let at = self.dots.drag.as_ref().unwrap().pointer;
            let over = self.dot_over(source, at, true);
            self.dots.drag.as_mut().unwrap().over = over;
            (at, over)
        } else {
            return;
        };
        // Above UI build mode's panel covers, so the guide reads clearly.
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            Id::new("dock-guide"),
        ));
        let float_size = if let Some(tab) = &self.dots.tab_drag {
            Some(tab.size)
        } else {
            self.dots.drag.as_ref().map(|drag| drag.size)
        };
        self.paint_dock_targets(&painter, grip, at, over, origin, float_size);
    }

    /// UIC-72 (dock guide, Jason 2026-11-20): the outer arrows, the
    /// five-way cross over the panel under the pointer, the insert mark on
    /// a tab row, and a translucent preview of where the drop lands, a
    /// floating window included.
    fn paint_dock_targets(
        &self,
        painter: &egui::Painter,
        source: Option<Dot>,
        at: Pos2,
        over: Over,
        origin: Vec2,
        float_size: Option<Vec2>,
    ) {
        let color = theme::workspace::drop_indicator();
        let targets = &self.dots.docking;
        let local = |rect: Rect| rect.translate(-origin);
        // The preview first, so the guide's buttons stay readable over it.
        let preview = match over {
            Over::Dock(target) => targets.landing_of(target),
            Over::Outside => float_size.map(|size| {
                let size = targets
                    .previous_work
                    .map_or(size, |work| size.min(work.size() * 0.5));
                Rect::from_min_size(at, size.max(vec2(48., 32.)))
            }),
            _ => None,
        };
        if let Some(rect) = preview {
            let floating = over == Over::Outside;
            painter.rect(
                local(rect).shrink(1.),
                3.,
                color.gamma_multiply(if floating { 0.12 } else { 0.25 }),
                Stroke::new(1_f32, color.gamma_multiply(if floating { 0.6 } else { 1. })),
                StrokeKind::Inside,
            );
        }
        // UIC-155: the insert mark uses the same geometry as the drop.
        for row in &targets.panel_rows {
            if matches!(over, Over::Dock(DockTarget::PanelTabs(dot, _)) if dot == row.group) {
                painter.rect_filled(local(insert_mark(row.rect, row.hit(at.x).1)), 0., color);
            }
        }
        for row in &targets.tree_rows {
            if matches!(over, Over::Dock(DockTarget::TreeTabs(id, _)) if id == row.group) {
                painter.rect_filled(local(insert_mark(row.rect, row.hit(at.x).1)), 0., color);
            }
        }
        if let Some((dot, rect)) = targets.guide(source, at) {
            let buttons: Vec<_> = compass(dot, rect).collect();
            paint_cross_plate(painter, &buttons, origin);
            for button in &buttons {
                let DockTarget::Compass(_, zone) = button.target else {
                    continue;
                };
                let active = over == Over::Dock(button.target);
                paint_guide_button(painter, local(button.rect), zone, false, active);
            }
        }
        for region in &targets.regions {
            if let DockTarget::Edge(_, side) = region.target {
                let active = over == Over::Dock(region.target);
                let rect = local(region.rect);
                painter.rect_filled(rect.expand(3.), 6., theme::popup_fill());
                paint_guide_button(painter, rect, Zone::Edge(side), true, active);
            }
        }
    }

    pub(super) fn apply_tab_drop(
        &mut self,
        from: TabSource,
        over: Over,
        at: Pos2,
        size: Vec2,
    ) -> Result<()> {
        if let Over::TabReorder(index) = over {
            return match from {
                TabSource::Panel(panel, kind) => {
                    let group = self.sidebar_group_mut(panel);
                    if let Some(from) = group.tabs.iter().position(|&k| k == kind) {
                        group.reorder(from, index);
                    }
                    Ok(())
                }
                TabSource::Tree {
                    window: None,
                    row,
                    tile,
                } => self.views.dock_tile_as_tab(tile, row, index),
                TabSource::Tree {
                    window: Some(window),
                    row,
                    tile,
                } => self.views.reorder_window_tab(window, row, tile, index),
            };
        }
        if !matches!(over, Over::Dock(_) | Over::Outside) {
            return Ok(());
        }
        let over = match over {
            Over::Dock(target) => Over::Dock(self.flatten_group_target(target)),
            other => other,
        };
        if let (TabSource::Panel(panel, _), Over::Dock(target)) = (from, over)
            && Self::sidebar_tab_target(target).is_some_and(|(dest, _)| panel == dest)
        {
            return Ok(());
        }
        let mut next = self.views.clone();
        if let (
            TabSource::Tree {
                window: Some(window),
                tile,
                ..
            },
            Over::Outside,
        ) = (from, over)
        {
            let (position, size) =
                self.place_window(Some([at.x, at.y]), [size.x, size.y], Some(at));
            next.float_dock_tile(Some(window), tile, size, position)?;
            self.views = next;
            return Ok(());
        }
        self.attach_dock_columns(&mut next)?;
        let source = match from {
            TabSource::Panel(panel, kind) => {
                let group = self.sidebar_group(panel);
                if !group.contains(kind) {
                    return Err("The source tab moved".into());
                }
                next.new_dock_tab(self.registry_slot(group, kind)).1
            }
            TabSource::Tree {
                window: Some(window),
                tile,
                ..
            } => next.import_dock_tile(window, Some(tile))?,
            TabSource::Tree {
                window: None, tile, ..
            } => tile,
        };
        let pane = next.main.tiles.get_pane(&source).copied();
        let merge = match over {
            Over::Dock(target) => self.dock_staged_tile(&mut next, source, target)?,
            Over::Outside => {
                let (position, size) =
                    self.place_window(Some([at.x, at.y]), [size.x, size.y], Some(at));
                next.float_dock_tile(None, source, size, position)?;
                None
            }
            _ => unreachable!("Only a dock target or tear-off can change the tree"),
        };
        if let TabSource::Panel(panel, kind) = from {
            if self.sidebar_group(panel).tabs.len() == 1 {
                next.remove(self.dock_pane(Dot::Side(panel)))?;
                if self.is_floating(panel) {
                    self.set_floating(panel, false);
                }
            }
            self.sidebar_group_mut(panel).remove(kind);
        }
        let active = if let Some((panel, index, slot)) = merge {
            self.merge_dock_tab(panel, index, &slot);
            Some(Dot::Side(panel))
        } else {
            pane.map(Self::dot_for_pane)
        };
        self.install_dock_tree(next);
        if let Over::Dock(target) = over {
            self.orient_tool_tabs(pane == Some(Pane::ToolTabs), target);
        }
        if let Some(active) = active {
            self.activate_dot(active);
        }
        Ok(())
    }

    /// A drop on a tree tab row that holds a sidebar group (the Items or
    /// Properties group pane) joins that group's own tabs instead of
    /// nesting beside it (Jason, 2026-11-19: tab groups never nest; a
    /// group dropped into a group flattens into it).
    pub(crate) fn flatten_group_target(&self, target: DockTarget) -> DockTarget {
        let DockTarget::TreeTabs(id, _) = target else {
            return target;
        };
        let tiles = &self.views.main.tiles;
        // The row is either a tab container or a lone leaf drawn with
        // its own tab row.
        let children: Vec<TileId> = match tiles.get(id) {
            Some(egui_tiles::Tile::Container(container)) => container.children().copied().collect(),
            Some(egui_tiles::Tile::Pane(_)) => vec![id],
            None => return target,
        };
        for child in &children {
            match tiles.get_pane(child) {
                Some(Pane::Items) => {
                    return DockTarget::PanelTabs(Dot::Side(Panel::Items), usize::MAX);
                }
                Some(Pane::Properties) => {
                    return DockTarget::PanelTabs(Dot::Side(Panel::Properties), usize::MAX);
                }
                _ => {}
            }
        }
        target
    }

    fn sidebar_tab_target(target: DockTarget) -> Option<(Panel, usize)> {
        match target {
            DockTarget::PanelTabs(Dot::Side(panel @ (Panel::Items | Panel::Properties)), index) => {
                Some((panel, index))
            }
            DockTarget::Compass(
                Dot::Side(panel @ (Panel::Items | Panel::Properties)),
                Zone::Center,
            ) => Some((panel, usize::MAX)),
            _ => None,
        }
    }

    fn dock_staged_tile(
        &self,
        next: &mut operandi_layout::Layout,
        source: TileId,
        target: DockTarget,
    ) -> Result<Option<(Panel, usize, operandi_layout::Slot)>> {
        if target.targets_tool_tabs() {
            Self::materialize_tool_tabs(next)?;
        }
        if let (Some((panel, index)), Some(&Pane::DockTab(key))) = (
            Self::sidebar_tab_target(target),
            next.main.tiles.get_pane(&source),
        ) {
            if next.dock_slot(panel.pane()).is_none() {
                return Err("The destination tab row moved".into());
            }
            let slot = next.dock_tabs.remove(&key).ok_or("The source tab moved")?;
            next.remove(Pane::DockTab(key))?;
            next.reveal_docked_tab(panel.pane());
            return Ok(Some((panel, index, slot)));
        }
        self.dock_single_tile(next, source, target)?;
        Ok(None)
    }

    fn merge_dock_tab(&mut self, panel: Panel, index: usize, slot: &operandi_layout::Slot) {
        let mut tab = super::layouts::registry_group(slot);
        self.sidebar_group_mut(panel).merge_from(&mut tab, index);
    }

    pub(crate) fn float_dock_grip(
        &mut self,
        dot: Dot,
        size: [f32; 2],
        at: Option<[f32; 2]>,
    ) -> Result<()> {
        let pane = self.dock_pane(dot);
        let mut next = self.views.clone();
        let mut mixed = self.layouts.mixed_dock;
        if !mixed && pane.key().is_some() {
            // A stock layout can keep its columns until the only view slot
            // leaves. Once materialized, those columns keep the main open.
            if next.dock_slot(pane) == next.main.root {
                self.attach_dock_columns(&mut next)?;
                mixed = true;
            }
        } else if !mixed {
            self.attach_dock_columns(&mut next)?;
        }
        let source = Self::stage_dock_source(&mut next, pane)?;
        if let Dot::Side(panel) = dot
            && next.main.tiles.find_pane(&pane) == Some(source)
        {
            self.float_side_panel(panel, size, at);
            return Ok(());
        }
        next.float_dock_tile(None, source, size, at)?;
        if mixed || pane.key().is_none() {
            self.install_dock_tree(next);
        } else {
            self.views = next;
        }
        Ok(())
    }

    pub(crate) fn apply_window_drop(
        &mut self,
        from: floating::DockWindow,
        target: DockTarget,
    ) -> Result<()> {
        let mut next = self.views.clone();
        self.attach_dock_columns(&mut next)?;
        let source = match from {
            floating::DockWindow::Sidebar(panel) => {
                let dot = Dot::Side(panel);
                // The two registry rows already implement their ordered
                // merge (D-04). A split uses the general tile path.
                if !matches!(target, DockTarget::Compass(_, Zone::Edge(_))) {
                    self.apply_dock_target(dot, target)?;
                    self.set_floating(panel, false);
                    return Ok(());
                }
                if panel == Panel::Toolbox {
                    Self::materialize_tool_tabs(&mut next)?;
                }
                next.dock_slot(self.dock_pane(dot))
                    .ok_or("The source panel moved")?
            }
            floating::DockWindow::Tree(window) => next.import_dock_tile(window, None)?,
        };
        let pane = next.main.tiles.get_pane(&source).copied();
        let moves_tool_tabs = next.dock_slot(Pane::ToolTabs) == Some(source);
        let merge = self.dock_staged_tile(&mut next, source, target)?;
        self.install_dock_tree(next);
        self.orient_tool_tabs(moves_tool_tabs, target);
        if let floating::DockWindow::Sidebar(panel) = from {
            self.set_floating(panel, false);
        }
        if let Some((panel, index, slot)) = merge {
            self.merge_dock_tab(panel, index, &slot);
            self.activate_dot(Dot::Side(panel));
        } else if let Some(pane) = pane {
            self.activate_dot(Self::dot_for_pane(pane));
        }
        Ok(())
    }

    fn dock_single_tile(
        &self,
        next: &mut operandi_layout::Layout,
        source: TileId,
        target: DockTarget,
    ) -> Result<()> {
        match target {
            DockTarget::Compass(dot, Zone::Edge(side)) => {
                let slot = next
                    .dock_slot(self.dock_pane(dot))
                    .ok_or("No dock target")?;
                next.dock_tile_at(source, slot, side, None)
            }
            DockTarget::PanelTabs(dot, _) | DockTarget::Compass(dot, Zone::Center) => {
                let slot = next
                    .dock_slot(self.dock_pane(dot))
                    .ok_or("No dock target")?;
                next.dock_tile_as_tab(source, slot, usize::MAX)
            }
            DockTarget::TreeTabs(id, index) => next.dock_tile_as_tab(source, id, index),
            DockTarget::Edge(id, side) => next.dock_tile_at(source, id, side, None),
        }
    }

    fn stage_dock_source(next: &mut operandi_layout::Layout, pane: Pane) -> Result<TileId> {
        if let Some(slot) = next.dock_slot(pane) {
            return Ok(slot);
        }
        if pane == Pane::ToolTabs && next.window_of(pane).is_none() {
            // Stock/legacy layouts project the strip in its original position.
            // Stage its real tile directly at the drop, leaving the tree intact
            // on cancel or error (D-07, UIC-191, UIC-196).
            return Ok(next.main.tiles.insert_pane(pane));
        }
        Err("The source panel moved".into())
    }

    pub(super) fn materialize_tool_tabs(next: &mut operandi_layout::Layout) -> Result<()> {
        if next.dock_slot(Pane::ToolTabs).is_none() && next.window_of(Pane::ToolTabs).is_none() {
            next.insert_beside(Pane::ToolTabs, Pane::Toolbox, Side::Right)?;
        }
        Ok(())
    }

    fn orient_tool_tabs(&mut self, moved: bool, target: DockTarget) {
        if moved && let Some(side) = target.side() {
            self.layouts.tool_tabs_horizontal = Some(matches!(side, Side::Top | Side::Bottom));
        }
    }

    pub(crate) fn install_dock_tree(&mut self, next: operandi_layout::Layout) {
        if next.dock_slot(Pane::ToolTabs).is_some() || next.window_of(Pane::ToolTabs).is_some() {
            self.layouts.tool_tabs_horizontal.get_or_insert(false);
        }
        self.views = next;
        self.layouts.mixed_dock = true;
        self.layouts.group_piece = None;
    }

    pub(crate) fn apply_dock_target(&mut self, from: Dot, target: DockTarget) -> Result<()> {
        let target = self.flatten_group_target(target);
        // D-04 / UIC-155: return a detached registry tab to the sidebar
        // row itself, using its insertion index and preserving its other tabs.
        if let Dot::Tab(key) = from
            && Self::sidebar_tab_target(target).is_some()
        {
            let mut next = self.views.clone();
            let source = next
                .main
                .tiles
                .find_pane(&Pane::DockTab(key))
                .ok_or("The source tab moved")?;
            let (panel, index, slot) = self
                .dock_staged_tile(&mut next, source, target)?
                .ok_or("No sidebar tab target")?;
            self.merge_dock_tab(panel, index, &slot);
            self.install_dock_tree(next);
            self.activate_dot(Dot::Side(panel));
            return Ok(());
        }
        if from == Dot::ToolTabs || target.targets_tool_tabs() {
            if target == DockTarget::Compass(from, Zone::Center) {
                return Ok(());
            }
            let mut next = self.views.clone();
            self.attach_dock_columns(&mut next)?;
            if target.targets_tool_tabs() {
                Self::materialize_tool_tabs(&mut next)?;
            }
            let source = Self::stage_dock_source(&mut next, self.dock_pane(from))?;
            // Beside a stacked panel (the Toolbox over Tool Properties):
            // the column widens by the strip, so neither panel shrinks.
            let widen = match target {
                DockTarget::Compass(to, Zone::Edge(side)) if from == Dot::ToolTabs => {
                    let pane = self.dock_pane(to);
                    self.stacked_in_column(pane, side)
                        .then(|| self.dot_panel_rect(to).map(|r| (pane, r.width())))
                        .flatten()
                }
                _ => None,
            };
            self.dock_single_tile(&mut next, source, target)?;
            if let Some((pane, kept)) = widen {
                self.widen_stacked_row(
                    &mut next,
                    pane,
                    kept,
                    crate::toolbox::VERTICAL_TOOL_TABS_WIDTH,
                );
            }
            self.install_dock_tree(next);
            self.orient_tool_tabs(from == Dot::ToolTabs, target);
            self.activate_dot(from);
            return Ok(());
        }
        if let DockTarget::Compass(to, Zone::Edge(side)) = target {
            if !self.layouts.mixed_dock && matches!((from, to), (Dot::View(_), Dot::View(_))) {
                return self
                    .views
                    .dock_beside(self.dock_pane(from), self.dock_pane(to), side);
            }
            return self.move_dock_panel(from, to, Zone::Edge(side));
        }
        let mut next = self.views.clone();
        self.attach_dock_columns(&mut next)?;
        let source = self.dock_pane(from);
        if source == Pane::Toolbox {
            Self::materialize_tool_tabs(&mut next)?;
        }
        let tab_hit = match target {
            DockTarget::PanelTabs(dot, index) => Some((dot, index)),
            DockTarget::Compass(dot, Zone::Center) => Some((dot, usize::MAX)),
            _ => None,
        };
        let mut active = from;
        if let Some((to, index)) = tab_hit {
            let dest = self.dock_pane(to);
            if source == dest {
                return Ok(());
            }
            if matches!(
                (source, dest),
                (Pane::Items, Pane::Properties) | (Pane::Properties, Pane::Items)
            ) {
                next.remove(source)?;
                let (source, dest) = if source == Pane::Items {
                    (&mut self.upper_frame, &mut self.lower_frame)
                } else {
                    (&mut self.lower_frame, &mut self.upper_frame)
                };
                dest.merge_from(source, index);
                next.reveal_docked_tab(self.dock_pane(to));
                active = to;
            } else {
                let slot = next.dock_slot(dest).ok_or("No dock target")?;
                next.dock_as_tab(source, slot, usize::MAX)?;
            }
        } else {
            match target {
                DockTarget::TreeTabs(_, _) if matches!(source, Pane::Items | Pane::Properties) => {
                    // A whole group never becomes one tab of another row.
                    return Err(
                        "A tab group can't go inside this tab row; dock it beside instead".into(),
                    );
                }
                DockTarget::TreeTabs(id, index) => next.dock_as_tab(source, id, index)?,
                DockTarget::Edge(id, side) => next.dock_at(source, id, side, None)?,
                _ => return Err("No dock target".into()),
            }
        }
        // UIC-191: this is the same tree the layout writer captures.
        self.install_dock_tree(next);
        self.activate_dot(active);
        Ok(())
    }
}

fn compass(dot: Dot, panel: Rect) -> impl Iterator<Item = Region> {
    let size = GUIDE_BUTTON
        .min((panel.width().min(panel.height()) - 6.) / 3.)
        .max(8.);
    let step = size + GUIDE_GAP;
    [
        (Zone::Center, Vec2::ZERO),
        (Zone::Edge(Side::Left), vec2(-step, 0.)),
        (Zone::Edge(Side::Right), vec2(step, 0.)),
        (Zone::Edge(Side::Top), vec2(0., -step)),
        (Zone::Edge(Side::Bottom), vec2(0., step)),
    ]
    .into_iter()
    .map(move |(zone, offset)| Region {
        target: DockTarget::Compass(dot, zone),
        rect: Rect::from_center_size(panel.center() + offset, Vec2::splat(size)).intersect(panel),
    })
}

fn insert_mark(row: Rect, x: f32) -> Rect {
    Rect::from_center_size(pos2(x, row.center().y), vec2(3., row.height()))
}

/// The four fixed arrows, one at the middle of each outer edge of the work
/// area, docking against the whole window (UIC-72 dock guide).
fn outer_regions(root: TileId, work: Rect) -> impl Iterator<Item = Region> {
    let size = GUIDE_BUTTON.min((work.width().min(work.height()) - 6.) / 3.);
    let inset = GUIDE_INSET + size / 2.;
    let c = work.center();
    [
        (Side::Top, pos2(c.x, work.top() + inset)),
        (Side::Bottom, pos2(c.x, work.bottom() - inset)),
        (Side::Left, pos2(work.left() + inset, c.y)),
        (Side::Right, pos2(work.right() - inset, c.y)),
    ]
    .into_iter()
    .filter(move |_| size >= 8.)
    .map(move |(side, center)| Region {
        target: DockTarget::Edge(root, side),
        rect: Rect::from_center_size(center, Vec2::splat(size)),
    })
}

/// One guide button's size, the gap between the cross's buttons, and how far
/// the outer arrows sit from the work area's edge (points).
const GUIDE_BUTTON: f32 = 32.;
const GUIDE_GAP: f32 = 4.;
const GUIDE_INSET: f32 = 8.;

/// The cross's backing: one plate behind the five buttons, outlined as a
/// single plus shape.
fn paint_cross_plate(painter: &egui::Painter, buttons: &[Region], origin: Vec2) {
    let Some(center) = buttons
        .iter()
        .find(|b| matches!(b.target, DockTarget::Compass(_, Zone::Center)))
    else {
        return;
    };
    let all = buttons
        .iter()
        .map(|b| b.rect)
        .reduce(Rect::union)
        .unwrap_or(center.rect);
    let bars = [
        Rect::from_x_y_ranges(all.x_range(), center.rect.y_range()),
        Rect::from_x_y_ranges(center.rect.x_range(), all.y_range()),
    ]
    .map(|r| r.translate(-origin).expand(4.));
    // Both outlines first, then both fills: the bars join without a seam.
    for bar in bars {
        painter.rect_stroke(
            bar,
            6.,
            Stroke::new(1_f32, theme::border()),
            StrokeKind::Outside,
        );
    }
    for bar in bars {
        painter.rect_filled(bar, 6., theme::popup_fill());
    }
}

/// A guide button: a small frame with the part the panel would take filled
/// in, and an arrow toward that side. `whole` (the outer arrows) fills a
/// thin strip along the edge instead of half the frame.
fn paint_guide_button(painter: &egui::Painter, rect: Rect, zone: Zone, whole: bool, active: bool) {
    let accent = theme::workspace::drop_indicator();
    painter.rect(
        rect,
        4.,
        if active {
            accent.gamma_multiply(0.45)
        } else {
            theme::control()
        },
        Stroke::new(
            if active { 2. } else { 1_f32 },
            if active { accent } else { theme::border() },
        ),
        StrokeKind::Inside,
    );
    let ink = theme::text();
    let glyph = rect.shrink(rect.width() * 0.25);
    let filled = match zone {
        Zone::Center => {
            // A frame with a tab on its top edge.
            let tab = Rect::from_min_size(
                glyph.min - vec2(0., glyph.height() * 0.2),
                vec2(glyph.width() * 0.45, glyph.height() * 0.2),
            );
            painter.rect_filled(tab, 1., ink);
            glyph
        }
        Zone::Edge(side) if whole => {
            let strip = glyph.width() * 0.3;
            match side {
                Side::Left => glyph.with_max_x(glyph.left() + strip),
                Side::Right => glyph.with_min_x(glyph.right() - strip),
                Side::Top => glyph.with_max_y(glyph.top() + strip),
                Side::Bottom => glyph.with_min_y(glyph.bottom() - strip),
            }
        }
        Zone::Edge(_) => landing(glyph, zone),
    };
    painter.rect_filled(filled, 1., accent.gamma_multiply(0.85));
    painter.rect_stroke(glyph, 1., Stroke::new(1_f32, ink), StrokeKind::Middle);
    if let Zone::Edge(side) = zone {
        // A small arrow between the frame and the button's edge.
        let (dir, tip) = match side {
            Side::Left => (vec2(-1., 0.), pos2(rect.left() + 2., rect.center().y)),
            Side::Right => (vec2(1., 0.), pos2(rect.right() - 2., rect.center().y)),
            Side::Top => (vec2(0., -1.), pos2(rect.center().x, rect.top() + 2.)),
            Side::Bottom => (vec2(0., 1.), pos2(rect.center().x, rect.bottom() - 2.)),
        };
        let depth = (rect.width() * 0.25 - 3.).max(2.);
        let base = tip - dir * depth;
        let across = vec2(dir.y, dir.x) * depth;
        painter.add(egui::Shape::convex_polygon(
            vec![tip, base + across, base - across],
            ink,
            Stroke::NONE,
        ));
    }
}

/// The mode bar's drag (UIC-71: it docks at the top or bottom only, never
/// floats): the dock guide's top and bottom arrows over the work area, the
/// nearer one lit, and a translucent strip where the bar lands. `top` is
/// where the work area starts (below the top bar).
pub(crate) fn paint_mode_bar_guide(
    painter: &egui::Painter,
    screen: Rect,
    top: f32,
    bar_height: f32,
    bottom: bool,
) {
    let accent = theme::workspace::drop_indicator();
    let landing = if bottom {
        Rect::from_min_max(
            pos2(screen.left(), screen.bottom() - bar_height),
            screen.max,
        )
    } else {
        Rect::from_min_max(
            pos2(screen.left(), top),
            pos2(screen.right(), top + bar_height),
        )
    };
    painter.rect(
        landing.shrink(1.),
        3.,
        accent.gamma_multiply(0.25),
        Stroke::new(1_f32, accent),
        StrokeKind::Inside,
    );
    let inset = GUIDE_INSET + GUIDE_BUTTON / 2.;
    for (side, y) in [
        (Side::Top, top + bar_height + inset),
        (Side::Bottom, screen.bottom() - bar_height - inset),
    ] {
        let rect = Rect::from_center_size(pos2(screen.center().x, y), Vec2::splat(GUIDE_BUTTON));
        painter.rect_filled(rect.expand(3.), 6., theme::popup_fill());
        paint_guide_button(
            painter,
            rect,
            Zone::Edge(side),
            true,
            (side == Side::Bottom) == bottom,
        );
    }
}
