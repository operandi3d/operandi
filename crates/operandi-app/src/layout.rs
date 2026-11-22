//! Layout owns divider pointer sequences before viewport/model input. This also
//! handles native press/move/release events coalesced into one frame.
use super::*;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Divider {
    Left,
    Right,
    Scene,
    /// The tool list / Tool Properties divider in the toolbox.
    Tools,
}
/// Side panel widths, in points. The viewport keeps at least
/// `VIEWPORT_MIN` between the docked panels; a window too narrow for that
/// leaves both panels at their minimums.
pub(super) const LEFT_MIN: f32 = 200.;
pub(super) const LEFT_MAX: f32 = 420.;
pub(super) const LEFT_DEFAULT: f32 = 244.;
pub(super) const RIGHT_MIN: f32 = 240.;
pub(super) const RIGHT_MAX: f32 = 640.;
pub(super) const RIGHT_DEFAULT: f32 = 300.;
pub(super) const VIEWPORT_MIN: f32 = 500.;
/// How far either side of a docked panel's inner edge the resize grab
/// (and its hover cursor) reaches.
pub(super) const EDGE_GRAB: f32 = 4.;
/// Two presses on a side bar's resize line this close together (seconds)
/// are a double-click: the bar collapses to its line (Jason 2026-11-18).
const EDGE_DOUBLE_CLICK: f64 = 0.4;
/// A press on a resize line that moves further than this is a drag.
const EDGE_CLICK_SLOP: f32 = 3.;
/// The left panel's widest setting in a window `window` points wide.
pub(super) fn left_max(window: f32) -> f32 {
    (window - VIEWPORT_MIN - RIGHT_MIN).clamp(LEFT_MIN, LEFT_MAX)
}
/// The right panel's widest setting once `left` points are taken by the
/// toolbox (0 when it floats).
pub(super) fn right_max(window: f32, left: f32) -> f32 {
    (window - left - VIEWPORT_MIN).clamp(RIGHT_MIN, RIGHT_MAX)
}
#[derive(Clone, Copy)]
pub(super) struct Capture {
    kind: Divider,
    start: Pos2,
    rect: Rect,
    value: f32,
    height: f32,
}
impl Operandi {
    /// Menus, popups, dialogs and View > Layout's divider lock keep the
    /// dividers still.
    pub(super) fn layout_blocked(&self, ctx: &Context) -> bool {
        self.menu_owned_input
            || self.dots.locks.dividers
            || egui::Popup::is_any_open(ctx)
            || self.pending.is_some()
            || self.file_dialog.is_some()
            || self.show_bindings
            || self.recovery.is_some()
            || self.recovery_warning.is_some()
    }
    /// The width the docked toolbox takes (0 while it floats).
    pub(super) fn docked_left_width(&self, ctx: &Context) -> f32 {
        if self.is_floating(floating::Panel::Toolbox) {
            return 0.;
        }
        if self.layouts.left_collapsed {
            return crate::collapse_line::SIDE_AREA;
        }
        egui::containers::panel::PanelState::load(ctx, Id::new("tools"))
            .map_or(LEFT_DEFAULT, |s| s.rect.width())
    }
    /// The docked side panel whose inner edge is under `pos`, with that
    /// panel's rect. A floating window over the edge keeps the pointer, and
    /// a floating panel has no edge here.
    fn side_edge_at(&self, ctx: &Context, pos: Pos2) -> Option<(Divider, Rect)> {
        if self.layouts.mixed_dock
            || ctx
                .layer_id_at(pos)
                .is_some_and(|layer| layer.order != egui::Order::Background)
        {
            return None;
        }
        let right_docked = !(self.is_floating(floating::Panel::Items)
            && self.is_floating(floating::Panel::Properties));
        let mut hit = None;
        for (kind, id, docked) in [
            (
                Divider::Left,
                "tools",
                !self.is_floating(floating::Panel::Toolbox) && !self.layouts.left_collapsed,
            ),
            (
                Divider::Right,
                "scene",
                right_docked && !self.layouts.right_collapsed,
            ),
        ] {
            if !docked {
                continue;
            }
            if let Some(state) = egui::containers::panel::PanelState::load(ctx, Id::new(id)) {
                let x = match kind {
                    Divider::Left => state.rect.right(),
                    _ => state.rect.left(),
                };
                if state.rect.y_range().contains(pos.y) && (pos.x - x).abs() <= EDGE_GRAB {
                    hit = Some((kind, state.rect));
                }
            }
        }
        hit
    }
    /// The resize cursor over a side panel edge or the Scenes/Properties
    /// divider, and throughout a divider drag. Runs after the panels and
    /// viewport have drawn, so it wins over their cursors.
    pub(super) fn layout_cursor(&self, ctx: &Context) {
        if let Some(c) = self.layout_capture {
            ctx.set_cursor_icon(if matches!(c.kind, Divider::Scene | Divider::Tools) {
                CursorIcon::ResizeVertical
            } else {
                CursorIcon::ResizeHorizontal
            });
            return;
        }
        // Another drag (a tool, a selection) passing over an edge keeps its
        // own cursor.
        if self.layout_blocked(ctx) || ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            if self.side_edge_at(ctx, pos).is_some() {
                ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
            } else if self.tool_divider_at(ctx, pos) {
                ctx.set_cursor_icon(CursorIcon::ResizeVertical);
            }
        }
    }
    /// Is `pos` on the toolbox's tool list / Tool Properties divider, with
    /// nothing (a floating window, a popup) over it?
    fn tool_divider_at(&self, ctx: &Context, pos: Pos2) -> bool {
        self.tool_divider_rect.contains(pos)
            && self.tool_divider_layer.is_some()
            && ctx.layer_id_at(pos) == self.tool_divider_layer
    }
    pub(super) fn layout_input(&mut self, ctx: &Context) -> bool {
        let scene_divider_visible = !self.layouts.mixed_dock
            && !self.layouts.right_collapsed
            && !self.is_floating(floating::Panel::Items)
            && !self.is_floating(floating::Panel::Properties);
        if !scene_divider_visible
            && self
                .layout_capture
                .is_some_and(|c| c.kind == Divider::Scene)
        {
            self.layout_capture = None;
        }
        if self.layout_blocked(ctx) {
            self.layout_capture = None;
            return false;
        }
        let mut consumed = false;
        let mut remaining = Vec::new();
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                ..
            } = event
            {
                let edge = self.side_edge_at(ctx, pos);
                let now = ctx.input(|i| i.time);
                if let Some((kind, _)) = edge {
                    let double = self.last_edge_press.is_some_and(|(k, t, at)| {
                        k == kind
                            && now - t <= EDGE_DOUBLE_CLICK
                            && at.distance(pos) <= EDGE_CLICK_SLOP
                    });
                    if double {
                        // The second press of a double-click: collapse the
                        // bar instead of starting another drag.
                        self.last_edge_press = None;
                        self.layout_capture = None;
                        let bar = if kind == Divider::Left {
                            "left"
                        } else {
                            "right"
                        };
                        self.execute(&format!("operandi.bar.collapse bar:{bar} on:true"));
                        consumed = true;
                        continue;
                    }
                    self.last_edge_press = Some((kind, now, pos));
                } else {
                    self.last_edge_press = None;
                }
                let mut hit = edge.map(|(kind, rect)| Capture {
                    kind,
                    start: pos,
                    rect,
                    value: rect.width(),
                    height: 1.,
                });
                if hit.is_none()
                    && scene_divider_visible
                    && self.scene_divider_rect.contains(pos)
                    && ctx
                        .layer_id_at(pos)
                        .is_none_or(|layer| layer.order == egui::Order::Background)
                {
                    hit = Some(Capture {
                        kind: Divider::Scene,
                        start: pos,
                        rect: self.scene_divider_rect,
                        value: self.scene_fraction,
                        height: self.scene_available_height.max(1.),
                    });
                }
                if hit.is_none() && self.tool_divider_at(ctx, pos) {
                    hit = Some(Capture {
                        kind: Divider::Tools,
                        start: pos,
                        rect: self.tool_divider_rect,
                        value: self.tool_split,
                        height: self.tool_split_total.max(1.),
                    });
                }
                if let Some(hit) = hit {
                    self.layout_capture = Some(hit);
                    consumed = true;
                    continue;
                }
            }
            if let Some(capture) = self.layout_capture {
                let pos = match event {
                    Event::PointerMoved(p)
                    | Event::PointerButton {
                        pos: p,
                        button: PointerButton::Primary,
                        pressed: false,
                        ..
                    } => Some(p),
                    _ => None,
                };
                if let Some(pos) = pos {
                    let width = ctx.content_rect().width();
                    match capture.kind {
                        Divider::Scene => {
                            self.scene_fraction = (capture.value
                                + (pos.y - capture.start.y) / capture.height)
                                .clamp(0.25, 0.65)
                        }
                        Divider::Tools => {
                            self.tool_split = tool_split::dragged(
                                capture.value,
                                pos.y - capture.start.y,
                                capture.height,
                                self.tool_list_min,
                            )
                        }
                        Divider::Left | Divider::Right => {
                            let (id, value) = match capture.kind {
                                Divider::Left => (
                                    "tools",
                                    (capture.value + pos.x - capture.start.x)
                                        .clamp(LEFT_MIN, left_max(width)),
                                ),
                                _ => (
                                    "scene",
                                    (capture.value - pos.x + capture.start.x).clamp(
                                        RIGHT_MIN,
                                        right_max(width, self.docked_left_width(ctx)),
                                    ),
                                ),
                            };
                            let mut rect = capture.rect;
                            if matches!(capture.kind, Divider::Left) {
                                rect.max.x = rect.min.x + value;
                            } else {
                                rect.min.x = rect.max.x - value;
                            }
                            ctx.data_mut(|d| {
                                d.insert_persisted(
                                    Id::new(id),
                                    egui::containers::panel::PanelState { rect },
                                )
                            });
                        }
                    }
                    consumed = true;
                    ctx.request_repaint();
                    if matches!(event, Event::PointerButton { pressed: false, .. }) {
                        self.layout_capture = None;
                        // A press that dragged isn't half a double-click.
                        if pos.distance(capture.start) > EDGE_CLICK_SLOP {
                            self.last_edge_press = None;
                        }
                    }
                    continue;
                }
                // A divider drag still held when the pointer leaves the
                // window keeps going until its release, like the viewport's
                // gestures: the OS still sends the moves and the release.
                // Losing focus (or leaving with no button down) ends it.
                let leaving = matches!(event, Event::PointerGone) && !pointer_down;
                if leaving || matches!(event, Event::WindowFocused(false)) {
                    self.layout_capture = None;
                }
            }
            remaining.push(event);
        }
        ctx.input_mut(|i| i.events = remaining);
        consumed || self.layout_capture.is_some()
    }
}
