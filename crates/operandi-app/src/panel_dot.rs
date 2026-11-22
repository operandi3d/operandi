//! UIC-71/72: view dots, hover grips and docking gestures for panel frames.
//! Requests run after the layout trees have been put back by their draw pass.
use super::*;
use crate::floating::Panel;
use crate::unavailable::Gate;
use operandi_layout::{Pane, Side};
use serde::{Deserialize, Serialize};

#[path = "panel_docking.rs"]
mod docking;
pub(crate) use docking::{DockTarget, TabSource, paint_mode_bar_guide};

/// The dot's diameter (points).
pub const DOT: f32 = 5.;
const HIT: f32 = 16.;
const VIEW_HIT: f32 = 11.;
pub(super) const VIEW_DOT_SLOT: f32 = 12.;
const FADE: f32 = 0.15;
const PRESS_ID: &str = "panel-grip-press";

#[derive(Clone, Copy)]
pub(super) struct GripPress {
    pub(super) at: Pos2,
    pub(super) button: PointerButton,
    modifiers: Modifiers,
}

pub(super) fn grip_rect(panel: Rect) -> Rect {
    Rect::from_min_size(panel.min, Vec2::splat(HIT)).intersect(panel)
}

pub(super) fn header_edge(panel: Rect, height: f32) -> Rect {
    Rect::from_min_size(panel.min, vec2(panel.width(), height)).intersect(panel)
}

pub(super) fn grip_shortcut(button: PointerButton, modifiers: Modifiers) -> bool {
    button == PointerButton::Middle && modifiers.shift && (modifiers.command || modifiers.ctrl)
}

fn grip_contains(ui: &Ui, rect: Rect, at: Pos2) -> bool {
    rect.intersect(ui.clip_rect()).contains(at) && ui.ctx().layer_id_at(at) == Some(ui.layer_id())
}

/// UIC-71: observe presses without adding a widget over the controls.
/// Claim the drag only once egui's pointer movement threshold is crossed.
pub(super) fn grip_start(ui: &Ui, id: Id, panel: Rect, handle: Option<Rect>) -> Option<GripPress> {
    let key = Id::new(PRESS_ID);
    let ctx = ui.ctx();
    let (pressed, pointer) = ui.input(|i| {
        let pressed = i.events.iter().rev().find_map(|event| match *event {
            Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers,
            } => Some(GripPress {
                at: pos,
                button,
                modifiers,
            }),
            _ => None,
        });
        (pressed, i.pointer.latest_pos())
    });
    if let Some(press) = pressed {
        if !egui::Popup::is_any_open(ctx)
            && grip_contains(ui, panel, press.at)
            && (grip_shortcut(press.button, press.modifiers)
                || (press.button == PointerButton::Primary
                    && handle.is_some_and(|rect| rect.contains(press.at))))
        {
            ctx.data_mut(|data| data.insert_temp(key, (id, press)));
        }
        return None;
    }
    let (owner, press) = ctx.data(|data| data.get_temp::<(Id, GripPress)>(key))?;
    if owner != id {
        return None;
    }
    let max_click_dist = ctx.options(|options| options.input_options.max_click_dist);
    let dragging = pointer.is_some_and(|at| at.distance(press.at) > max_click_dist);
    let down = ui.input(|i| i.focused && i.pointer.button_down(press.button));
    if dragging || !down {
        ctx.data_mut(|data| data.remove::<(Id, GripPress)>(key));
    }
    (dragging && down).then_some(press)
}

pub(super) fn grip_overlay(ui: &Ui, id: Id, panel: Rect, center: Pos2, fill: Color32) -> bool {
    let rect = grip_rect(panel);
    let hovered = ui.rect_contains_pointer(rect);
    let opacity = ui.ctx().animate_bool_with_time(id, hovered, FADE);
    if hovered && opacity > 0. {
        let overlay = grip_painter(ui).with_clip_rect(panel.intersect(ui.clip_rect()));
        paint_handle(&overlay, center, fill, opacity);
    }
    hovered
}

fn paint_handle(painter: &egui::Painter, center: Pos2, fill: Color32, opacity: f32) {
    let fill = fill.gamma_multiply(opacity);
    painter.circle_filled(
        center,
        DOT / 2. + 0.5,
        theme::border().gamma_multiply(opacity),
    );
    painter.circle_filled(center, DOT / 2., fill);
    painter.circle_filled(
        center + vec2(-0.7, -0.7),
        DOT / 5.,
        fill.gamma_multiply(0.5).additive(),
    );
}

fn grip_painter(ui: &Ui) -> egui::Painter {
    ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::new("panel-dot-overlay"),
    ))
}

/// UIC-71: a secondary click in the header opens the frame menu without
/// registering a hit box above the header's own controls.
pub(super) fn grip_menu(ui: &Ui, id: Id, header: Rect, add: impl FnOnce(&mut Ui)) {
    let at = ui.input(|i| {
        i.pointer
            .button_clicked(PointerButton::Secondary)
            .then(|| i.pointer.interact_pos())
            .flatten()
    });
    let open =
        at.is_some_and(|at| grip_contains(ui, header, at)) && !egui::Popup::is_any_open(ui.ctx());
    egui::Popup::new(
        id,
        ui.ctx().clone(),
        egui::PopupAnchor::PointerFixed,
        ui.layer_id(),
    )
    .kind(egui::PopupKind::Menu)
    .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
    .show(add);
}

/// UI build mode's cover: the whole panel except a tab row along its top
/// (tabs keep their own click and drag). `rows` are tab rows in the same
/// coordinates as `panel`.
pub(super) fn build_cover(panel: Rect, rows: impl IntoIterator<Item = Rect>) -> Rect {
    let mut cover = panel;
    for row in rows {
        // A row along the panel's top: inside it (a docked group's row sits
        // a few points in, DOCK_PANEL_INSET), or just above it.
        if row.x_range().intersects(panel.x_range())
            && row.top() <= panel.top() + 32.
            && row.bottom() > cover.top()
            && row.bottom() < panel.bottom()
        {
            cover.min.y = row.bottom();
        }
    }
    cover
}

/// UI build mode's cover in its own layer above the panel's content: a
/// press anywhere on it moves the panel, and nothing under it (controls,
/// the 3D view) gets the pointer. The accent outline marks the panel as
/// movable (Jason, 2026-11-20); a grip pill in the middle is the cue.
/// `busy`: a panel is on the move, so the grip pill stays out of the dock
/// guide's way.
pub(super) fn build_pad_area(
    ui: &Ui,
    id: Id,
    cover: Rect,
    outline: Rect,
    dragging: bool,
    busy: bool,
) -> Response {
    let response = egui::Area::new(id.with("build-pad"))
        .order(egui::Order::Foreground)
        .fixed_pos(cover.min)
        .constrain(false)
        .fade_in(false)
        .show(ui.ctx(), |ui| {
            let (rect, response) = ui.allocate_exact_size(cover.size(), Sense::click_and_drag());
            let hot = response.hovered() || dragging;
            let accent = theme::orange();
            let painter = ui.painter().with_clip_rect(outline.expand(1.));
            painter.rect_filled(
                rect,
                0.,
                accent.gamma_multiply(if hot { 0.16 } else { 0.06 }),
            );
            painter.rect_stroke(
                outline.shrink(1.),
                0.,
                Stroke::new(2_f32, accent.gamma_multiply(if hot { 1. } else { 0.8 })),
                StrokeKind::Inside,
            );
            let pill = Rect::from_center_size(
                rect.center(),
                vec2(
                    44_f32.min(rect.width() - 4.),
                    16_f32.min(rect.height() - 4.),
                ),
            );
            if pill.is_positive() && !busy {
                paint_build_pad(&painter, pill, hot);
            }
            response
        })
        .inner;
    if response.hovered() && !dragging {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    response
}

/// The cue: a raised pill with two rows of grip dots.
fn paint_build_pad(painter: &egui::Painter, rect: Rect, hot: bool) {
    let accent = theme::orange();
    painter.rect(
        rect,
        rect.height() / 2.,
        if hot { accent } else { theme::control() },
        Stroke::new(1_f32, if hot { accent } else { theme::border() }),
        StrokeKind::Inside,
    );
    let ink = if hot {
        theme::on_orange()
    } else {
        theme::text()
    };
    let c = rect.center();
    for row in [-2.5, 2.5] {
        for col in -2..=2 {
            painter.circle_filled(c + vec2(col as f32 * 5., row), 1.2, ink);
        }
    }
}

/// How far outside its window a torn-off or copied view opens from the
/// panel it came from (Detach, Copy).
const DETACH_OFFSET: f32 = 30.;

/// The panel a dot belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dot {
    View(u32),
    Side(Panel),
    ToolProperties,
    ToolTabs,
    Tab(u32),
}

impl Dot {
    pub(super) fn grip_center(self, rect: Rect) -> Pos2 {
        // UIC-71: leave the Tool Properties disclosure available for a
        // click while its dimple can still be double-clicked to float.
        if self == Self::ToolProperties {
            pos2(
                rect.left() + (DOT / 2. + 0.5).min(rect.width() / 2.),
                rect.center().y,
            )
        } else {
            rect.center()
        }
    }
}

/// Where a dragged panel lands on another one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// The middle half: swap the two panels' places.
    Center,
    /// The outer quarter on one side: dock beside, splitting the target
    /// at half on that side.
    Edge(Side),
}

/// What a dot drag does, chosen by the modifiers at the press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Plain drag: move the panel.
    Move,
    /// Shift: a copy goes where the drag ends; the original stays.
    Copy,
    /// Ctrl: split the panel at half in the drag's direction.
    Split,
}

/// What the pointer is over during a dot drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Over {
    /// The dragged panel itself, or a panel that takes no drop.
    Nothing,
    Panel(Dot, Zone),
    Dock(DockTarget),
    TabReorder(usize),
    /// Outside every panel: tear off (or copy) into a new window.
    Outside,
}

/// A dot drag in progress. Positions are on screen (points), so a drag can
/// cross from one OS window to another.
pub struct DotDrag {
    pub dot: Dot,
    pub gesture: Gesture,
    start: Pos2,
    pub pointer: Pos2,
    /// The dragged panel's size: a torn-off window opens at that size.
    size: Vec2,
    button: PointerButton,
    pub over: Over,
}

/// The right-click menu's actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Duplicate,
    Copy,
    Detach,
    Delete,
    Maximize,
    SplitHorizontal,
    SplitVertical,
    SplitQuad,
}

/// Something a dot asked for while the panels drew.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Request {
    Activate(Dot),
    Drop {
        dot: Dot,
        gesture: Gesture,
        over: Over,
        start: Pos2,
        at: Pos2,
        size: Vec2,
    },
    Menu(Dot, MenuAction),
    TabDrop(TabSource, Over, Pos2, Vec2),
    WindowDrop(floating::DockWindow, DockTarget),
    Float(Dot),
    DockAtEdge(Dot, Side),
}

/// One panel drawn this frame, on screen.
#[derive(Clone, Copy, Debug)]
struct Drawn {
    dot: Dot,
    rect: Rect,
    /// In the main window (a floating side panel docks home when dropped
    /// here).
    in_main: bool,
}

/// Layout > Lock Thumb / Divider Dragging (LAY-48): each layout's own, in
/// the layouts state file. The old app-wide `layout-locks.json` is only
/// read, at launch, as the starting value for layouts without their own.
/// Both off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutLocks {
    pub version: u8,
    /// Lock Thumb Dragging: no dot drags (the dot's menu still works).
    pub thumbs: bool,
    /// Lock Divider Dragging: dividers between panels don't resize.
    pub dividers: bool,
}

impl LayoutLocks {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .filter(|b| b.len() <= 4096)
            .and_then(|b| serde_json::from_slice::<Self>(&b).ok())
            .filter(|l| l.version == 1)
            .unwrap_or_default()
    }
}

/// The dots' state across frames.
#[derive(Default)]
pub struct DotState {
    /// The active side panel, when a side panel's dot was pressed last;
    /// `None` when the active panel is the active view.
    pub active_side: Option<Dot>,
    /// Each dot drawn this frame (its press area, in its window).
    pub rects: BTreeMap<Dot, Rect>,
    drawn: Vec<Drawn>,
    /// Last frame's panels: what a drag is over (a drag can only know a
    /// window's panels once that window has drawn).
    previous: Vec<Drawn>,
    docking: docking::Targets,
    pub drag: Option<DotDrag>,
    tab_drag: Option<docking::TabDrag>,
    window_drag: Option<floating::WindowDrag>,
    dock_windows: BTreeMap<Pane, u32>,
    requests: Vec<Request>,
    /// Registry panels to close once the frame drew (panel_close.rs).
    pub close_panels: Vec<crate::panel_registry::PanelKind>,
    pub locks: LayoutLocks,
    /// UI build mode (Layout > UI Build Mode, Ctrl+Shift+L): every panel
    /// shows a drag pad, and panels drag even with Lock Thumb Dragging on.
    /// For this run only.
    pub build_mode: bool,
    /// Each view's window, taken before the views draw (`None`: main).
    view_windows: BTreeMap<u32, Option<u32>>,
    /// Where a side panel's floating window opens (a drag's release point).
    pub side_positions: BTreeMap<Panel, [f32; 2]>,
    /// The saved floating view windows have been checked against the
    /// connected screens (VST-37), once per run.
    pub windows_rescued: bool,
    /// Points per pixel of the window that drew a dot last, for listing
    /// the screens a new window is placed on.
    pixels_per_point: f32,
}

/// The top-left of the window being drawn, on screen. Embedded windows
/// (tests, backends without native windows) share the root's coordinates.
pub(super) fn window_origin(ctx: &Context) -> Vec2 {
    if ctx.embed_viewports() {
        return Vec2::ZERO;
    }
    ctx.input(|i| i.viewport().inner_rect)
        .map_or(Vec2::ZERO, |r| r.min.to_vec2())
}

/// Which drop zone of `rect` a point is in: the middle half is the centre,
/// the outer quarter of each side its edge (corners go to the nearer one).
pub fn zone(rect: Rect, at: Pos2) -> Zone {
    let u = ((at.x - rect.left()) / rect.width().max(1.)).clamp(0., 1.);
    let v = ((at.y - rect.top()) / rect.height().max(1.)).clamp(0., 1.);
    let edges = [
        (u, Side::Left),
        (1. - u, Side::Right),
        (v, Side::Top),
        (1. - v, Side::Bottom),
    ];
    let (distance, side) =
        edges.into_iter().fold(
            (f32::MAX, Side::Left),
            |best, e| if e.0 < best.0 { e } else { best },
        );
    if distance < 0.25 {
        Zone::Edge(side)
    } else {
        Zone::Center
    }
}

/// Where a panel dropped in `zone` lands: the whole target for a swap,
/// its half on that side for Dock Beside.
pub fn landing(rect: Rect, zone: Zone) -> Rect {
    let c = rect.center();
    match zone {
        Zone::Center => rect,
        Zone::Edge(Side::Left) => Rect::from_min_max(rect.min, pos2(c.x, rect.bottom())),
        Zone::Edge(Side::Right) => Rect::from_min_max(pos2(c.x, rect.top()), rect.max),
        Zone::Edge(Side::Top) => Rect::from_min_max(rect.min, pos2(rect.right(), c.y)),
        Zone::Edge(Side::Bottom) => Rect::from_min_max(pos2(rect.left(), c.y), rect.max),
    }
}

/// Ctrl+drag: the side the copy goes, from the drag's direction (mostly
/// sideways: side by side; mostly up or down: stacked).
pub fn split_side(delta: Vec2) -> Side {
    if delta.x.abs() >= delta.y.abs() {
        if delta.x >= 0. {
            Side::Right
        } else {
            Side::Left
        }
    } else if delta.y >= 0. {
        Side::Bottom
    } else {
        Side::Top
    }
}

impl Operandi {
    /// Before anything draws: last frame's panels become what drags look
    /// at, and each view's window is noted.
    pub(super) fn dot_frame_begin(&mut self) {
        self.dots.previous = std::mem::take(&mut self.dots.drawn);
        self.dots.rects.clear();
        self.dots.dock_windows = self
            .views
            .panes()
            .into_iter()
            .filter_map(|pane| Some((pane, self.views.window_of(pane)?)))
            .collect();
        self.dots.view_windows = self
            .views
            .panes()
            .into_iter()
            .filter_map(|p| Some((p.key()?, self.views.window_of(p))))
            .collect();
    }

    /// Panels move only in UI build mode (Jason, 2026-11-20, from
    /// contributor feedback): grips, pads, tabs, floating titles and the
    /// mode bar all wait for it. Clicks, menus and Float still work.
    pub(super) fn panel_drags_locked(&self) -> bool {
        !self.dots.build_mode
    }

    /// A drag tried outside UI build mode says how to move panels.
    pub(super) fn hint_build_mode(&mut self) {
        self.status = "Panels move in UI build mode: Layout > UI Build Mode (Ctrl+Shift+L)".into();
    }

    /// Layout > UI Build Mode.
    pub(crate) fn set_build_mode(&mut self, on: bool) {
        self.dots.build_mode = on;
        self.status = if on {
            "UI build mode: drag a panel by its pad to dock or float it. Ctrl+Shift+L ends it."
                .into()
        } else {
            "UI build mode off".into()
        };
    }

    pub(super) fn panel_docking_in_progress(&self) -> bool {
        self.dots.drag.is_some() || self.dots.tab_drag.is_some() || self.dots.window_drag.is_some()
    }

    /// Escape belongs to the held grip before panels or tools read the key.
    pub(super) fn cancel_dot_drag(&mut self, ctx: &Context) {
        if (self.dots.drag.is_some()
            || self.dots.tab_drag.is_some()
            || self.dots.window_drag.is_some()
            || self.layouts.mode_bar_drag.is_some())
            && ctx.input(|i| i.key_pressed(Key::Escape))
        {
            ctx.input_mut(|i| {
                i.events.retain(|event| {
                    !matches!(
                        event,
                        Event::Key {
                            key: Key::Escape,
                            ..
                        }
                    )
                })
            });
            ctx.stop_dragging();
            self.layouts.mode_bar_drag = None;
            self.dots.drag = None;
            self.dots.tab_drag = None;
            self.dots.window_drag = None;
            self.status = "Panel move cancelled".into();
        }
    }

    /// Whether a dot's panel is the active one.
    pub(super) fn dot_is_active(&self, dot: Dot) -> bool {
        match dot {
            Dot::View(view) => self.dots.active_side.is_none() && view == self.active_view,
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => {
                self.dots.active_side == Some(dot)
            }
        }
    }

    /// A press inside a view makes it (not a side panel) the active one.
    pub(super) fn view_pressed(&mut self) {
        self.dots.active_side = None;
    }

    /// Register a panel for docking and paint its grip over its content.
    /// Tab groups supply the menu through their existing tab responses.
    pub(super) fn panel_dot(&mut self, ui: &Ui, dot: Dot, panel: Rect, header: Option<Rect>) {
        self.panel_handle(ui, dot, panel, grip_rect(panel), header);
    }

    /// UIC-71: the view's always-visible dot is also its frame menu handle.
    pub(super) fn view_dot(&mut self, ui: &Ui, view: u32, content: Rect, panel: Rect) {
        let center = content.min
            + vec2(
                4. + VIEW_DOT_SLOT / 2. - 1.,
                viewport_buttons::HEADER_CLEARANCE - viewport_buttons::HEADER_HEIGHT / 2.,
            );
        let rect = Rect::from_center_size(center, Vec2::splat(VIEW_HIT));
        let dot = Dot::View(view);
        let response = ui.interact(
            rect,
            Id::new(("view-dot-hit", view)),
            Sense::click_and_drag(),
        );
        self.panel_handle(ui, dot, panel, rect, None);
        let tip = if self.panel_drags_locked() {
            "Right-click for the panel menu. Panels move in UI build mode (Ctrl+Shift+L)."
        } else {
            "Drag to move or dock. Double-click to float."
        };
        response
            .on_hover_text(tip)
            .context_menu(|ui| self.dot_menu(ui, dot));
    }

    fn panel_handle(&mut self, ui: &Ui, dot: Dot, panel: Rect, rect: Rect, header: Option<Rect>) {
        let origin = window_origin(ui.ctx());
        self.dots.pixels_per_point = ui.ctx().pixels_per_point();
        let in_main = self.dock_source_in_main(dot);
        self.dots.drawn.push(Drawn {
            dot,
            rect: panel.translate(origin),
            in_main,
        });
        self.dots.rects.insert(dot, rect);
        let id = Id::new(("panel-dot", dot));
        let (press, double) = ui.input(|i| {
            (
                i.pointer
                    .button_pressed(PointerButton::Primary)
                    .then(|| i.pointer.press_origin())
                    .flatten(),
                i.pointer
                    .button_double_clicked(PointerButton::Primary)
                    .then(|| i.pointer.interact_pos())
                    .flatten(),
            )
        });
        if press.is_some_and(|at| grip_contains(ui, rect, at))
            && !egui::Popup::is_any_open(ui.ctx())
        {
            self.dots.requests.push(Request::Activate(dot));
        }
        // D-06: the grip's original double-click still floats the panel.
        if in_main
            && !self.panel_drags_locked()
            && double.is_some_and(|at| grip_contains(ui, rect, at))
            && !egui::Popup::is_any_open(ui.ctx())
        {
            self.dots.requests.push(Request::Float(dot));
        }
        if let Some(header) = header {
            grip_menu(ui, id.with("menu"), header, |ui| self.dot_menu(ui, dot));
        }
        if self.dots.build_mode {
            self.build_pad(ui, dot, id, origin, panel);
        }
        self.dot_drag(ui, dot, id, origin, panel, rect);
        let dragging = self.dots.drag.as_ref().is_some_and(|d| d.dot == dot);
        let fill = if self.dot_is_active(dot) {
            theme::panel_dot_active()
        } else {
            theme::panel_dot()
        };
        let hovered = if matches!(dot, Dot::View(_)) {
            let painter = grip_painter(ui).with_clip_rect(panel.intersect(ui.clip_rect()));
            paint_handle(&painter, rect.center(), fill, 1.);
            ui.rect_contains_pointer(rect)
        } else if self.panel_drags_locked() {
            // No grip to show outside UI build mode: it couldn't move.
            false
        } else {
            grip_overlay(ui, id, panel, dot.grip_center(rect), fill)
        };
        self.paint_dot(ui, dot, panel, origin, hovered || dragging);
    }

    fn dot_drag(&mut self, ui: &Ui, dot: Dot, id: Id, origin: Vec2, panel: Rect, rect: Rect) {
        let pointer = ui.input(|i| i.pointer.latest_pos()).map(|p| p + origin);
        // A screen falloff explicitly armed to draw owns the next stroke
        // in the active 3D view, including the corner (FO-43 / FO-52).
        let primary = dot != Dot::View(self.active_view)
            || !self.falloff_draw_armed
            || self.screen_falloff_press().is_none();
        if self.dots.drag.is_none()
            && self.dots.tab_drag.is_none()
            && self.dots.window_drag.is_none()
            && self.layouts.mode_bar_drag.is_none()
            && let Some(at) = pointer
            && let Some(press) = grip_start(ui, id, panel, primary.then_some(rect))
        {
            if self.panel_drags_locked() {
                self.hint_build_mode();
            } else {
                ui.ctx().set_dragged_id(id);
                self.start_dot_drag(dot, press, origin, at, panel.size());
            }
        }
        if self.dots.drag.as_ref().is_none_or(|drag| drag.dot != dot) {
            return;
        }
        self.cancel_dot_drag(ui.ctx());
        let Some(drag) = self.dots.drag.as_ref() else {
            return;
        };
        let (down, released) = ui.input(|i| {
            (
                i.focused && i.pointer.button_down(drag.button),
                i.pointer.button_released(drag.button),
            )
        });
        let over = pointer.map(|p| (p, self.dot_over(dot, p, drag.gesture == Gesture::Move)));
        let drag = self.dots.drag.as_mut().unwrap();
        if let Some((at, over)) = over {
            drag.pointer = at;
            drag.over = if drag.gesture == Gesture::Split {
                Over::Nothing
            } else {
                over
            };
        }
        if !down {
            let drag = self.dots.drag.take().unwrap();
            if released {
                self.dots.requests.push(Request::Drop {
                    dot: drag.dot,
                    gesture: drag.gesture,
                    over: drag.over,
                    start: drag.start,
                    at: drag.pointer,
                    size: drag.size,
                });
            }
            return;
        }
        ui.ctx().set_cursor_icon(match drag.gesture {
            Gesture::Copy => CursorIcon::Copy,
            _ => CursorIcon::Grabbing,
        });
        ui.ctx().request_repaint();
    }

    /// Start a grip or pad drag, its gesture chosen by the press's
    /// modifiers. `press.at` is in the window, `at` on screen.
    fn start_dot_drag(&mut self, dot: Dot, press: GripPress, origin: Vec2, at: Pos2, size: Vec2) {
        let gesture = if press.button == PointerButton::Middle {
            Gesture::Move
        } else if press.modifiers.ctrl {
            Gesture::Split
        } else if press.modifiers.shift {
            Gesture::Copy
        } else {
            Gesture::Move
        };
        if self.dot_can_drag(dot, gesture) {
            self.dots.requests.push(Request::Activate(dot));
            self.dots.drag = Some(DotDrag {
                dot,
                gesture,
                start: press.at + origin,
                pointer: at,
                size,
                button: press.button,
                over: Over::Nothing,
            });
        }
    }

    /// UI build mode: a pad at the top middle of every panel, drawn over
    /// its content in its own layer so a press there moves the panel and
    /// never reaches the controls below (the corner grip, UIC-71, keeps
    /// working beside it). A faint frame shows each panel's extent.
    fn build_pad(&mut self, ui: &Ui, dot: Dot, id: Id, origin: Vec2, panel: Rect) {
        let rows = self
            .dots
            .docking
            .tab_row_rects()
            .map(|r| r.translate(-origin));
        let cover = build_cover(panel, rows);
        if !cover.is_positive() || !ui.clip_rect().intersects(cover) {
            return;
        }
        let dragging = self.dots.drag.as_ref().is_some_and(|d| d.dot == dot);
        let busy = self.panel_docking_in_progress();
        let response = build_pad_area(ui, id, cover, panel, dragging, busy);
        if self.dots.drag.is_none()
            && self.dots.tab_drag.is_none()
            && self.dots.window_drag.is_none()
            && self.layouts.mode_bar_drag.is_none()
            && let Some(button) = [PointerButton::Primary, PointerButton::Middle]
                .into_iter()
                .find(|&b| response.drag_started_by(b))
            && let Some(at) = ui.input(|i| i.pointer.latest_pos())
        {
            let press = GripPress {
                at: ui.input(|i| i.pointer.press_origin()).unwrap_or(at),
                button,
                modifiers: ui.input(|i| i.modifiers),
            };
            self.start_dot_drag(dot, press, origin, at + origin, panel.size());
        }
        response.on_hover_text("Drag to dock or float this panel");
    }

    /// Where UI build mode's cover sits over `dot`'s panel (screen).
    #[cfg(test)]
    pub(crate) fn build_cover_of(&self, dot: Dot) -> Rect {
        build_cover(
            self.dot_panel_rect(dot).unwrap(),
            self.dots.docking.tab_row_rects(),
        )
    }

    /// UI build mode's way out, always on screen in the main window (the
    /// menus and Ctrl+Shift+L work too). Hidden during a drag so it never
    /// covers the bottom arrow.
    pub(super) fn build_mode_exit_button(&mut self, ctx: &Context) {
        if !self.dots.build_mode || self.panel_docking_in_progress() {
            return;
        }
        let Some(work) = self.dots.docking.work else {
            return;
        };
        let work = work.translate(-window_origin(ctx));
        let clicked = egui::Area::new(Id::new("build-mode-exit"))
            .order(egui::Order::Foreground)
            // Bottom middle: clear of the panels' top-left dots and the
            // tab rows people grab most.
            .pivot(Align2::CENTER_BOTTOM)
            .fixed_pos(pos2(work.center().x, work.bottom() - 10.))
            .constrain(false)
            .fade_in(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme::orange())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("UI build mode: drag any outlined panel")
                                    .color(theme::on_orange()),
                            );
                            ui.button("Done").clicked()
                        })
                        .inner
                    })
                    .inner
            })
            .inner;
        if clicked {
            self.set_build_mode(false);
        }
    }

    pub(super) fn track_dock_title(&mut self, response: &Response, source: floating::DockWindow) {
        let ctx = &response.ctx;
        if self
            .dots
            .window_drag
            .as_ref()
            .is_some_and(|drag| drag.source == source)
        {
            self.cancel_dot_drag(ctx);
        }
        let at = ctx
            .input(|i| i.pointer.latest_pos())
            .map(|p| p + window_origin(ctx));
        if response.drag_started_by(PointerButton::Primary)
            && !self.panel_drags_locked()
            && self.dots.drag.is_none()
            && self.dots.tab_drag.is_none()
            && self.dots.window_drag.is_none()
            && let Some(pointer) = at
        {
            self.dots.window_drag = Some(floating::WindowDrag { source, pointer });
        }
        if let Some(drag) = &mut self.dots.window_drag
            && drag.source == source
        {
            if let Some(at) = at {
                drag.pointer = at;
            }
            let pointer = drag.pointer;
            if ctx.input(|i| i.pointer.primary_released()) {
                self.dots.window_drag = None;
                if let Some(target) = self.dots.docking.hit(None, pointer) {
                    self.dots.requests.push(Request::WindowDrop(source, target));
                }
            } else if !ctx.input(|i| i.pointer.primary_down()) {
                self.dots.window_drag = None;
            } else {
                ctx.set_cursor_icon(CursorIcon::Grabbing);
                ctx.request_repaint();
            }
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
    }

    /// Whether a dot drag may start. The main window's last panel, and
    /// side panels, only take some gestures; Lock Thumb Dragging stops all.
    fn dot_can_drag(&self, dot: Dot, gesture: Gesture) -> bool {
        if self.panel_drags_locked() {
            return false;
        }
        match dot {
            Dot::View(view) => {
                gesture == Gesture::Move || gesture == Gesture::Split || !self.view_alone(view).1
            }
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => {
                gesture == Gesture::Move
            }
        }
    }

    /// What a drag of `dragged` is over at screen point `at`, from last
    /// frame's panels (the later-drawn, floating windows first).
    fn dot_over(&self, dragged: Dot, at: Pos2, moving: bool) -> Over {
        let hit = self
            .dots
            .previous
            .iter()
            .rev()
            .find(|d| d.rect.contains(at));
        if moving && (hit.is_none_or(|d| d.in_main) || !self.dock_source_in_main(dragged)) {
            if let Some(target) = self.dots.docking.hit(Some(dragged), at) {
                return Over::Dock(target);
            }
            if self.dots.docking.contains(at) {
                // UIC-72 dock guide: off every target, a main-window panel
                // floats where it's dropped. The main window keeps its last
                // view, and a panel that already floats stays as it is.
                let floats = self.dock_source_in_main(dragged)
                    && !matches!(dragged, Dot::View(view) if self.view_alone(view).1);
                return if floats { Over::Outside } else { Over::Nothing };
            }
        }
        let Some(hit) = hit else {
            return Over::Outside;
        };
        match (dragged, hit.dot) {
            (Dot::View(a), Dot::View(b)) if a != b => {
                let zone = zone(hit.rect, at);
                if self.view_alone(a).1 && !hit.in_main && zone != Zone::Center {
                    Over::Nothing
                } else {
                    Over::Panel(hit.dot, zone)
                }
            }
            // A floating side panel over the main window goes home.
            (Dot::Side(p), _) if self.is_floating(p) && hit.in_main => {
                Over::Panel(hit.dot, Zone::Center)
            }
            // UIC-72: use the existing zones on docked leaves and address
            // the exact viewport under the grip (owner's brief D-02).
            (a, b)
                if a != b && hit.in_main && !matches!(a, Dot::Side(p) if self.is_floating(p)) =>
            {
                Over::Panel(hit.dot, zone(hit.rect, at))
            }
            _ => Over::Nothing,
        }
    }

    /// The dot, its panel's hover outline, and (when a drag is over this
    /// panel) its drop zones, on top of everything in this window.
    fn paint_dot(&self, ui: &Ui, dot: Dot, panel: Rect, origin: Vec2, outlined: bool) {
        let overlay = grip_painter(ui);
        let accent = theme::orange();
        if outlined {
            overlay.rect_stroke(
                panel.shrink(0.5),
                0.,
                Stroke::new(1_f32, accent),
                StrokeKind::Inside,
            );
        }
        let Some(drag) = &self.dots.drag else {
            return;
        };
        // Ctrl+drag: the panel being split shows the result (VST-04): an
        // insert line where it divides and a translucent fill over the half
        // the new panel takes; in the warning colour when the halves would
        // be too small and release would do nothing (VST-24).
        if drag.gesture == Gesture::Split && drag.dot == dot {
            let side = split_side(drag.pointer - drag.start);
            let fits = self.split_fits(dot, panel, side);
            let colour = if fits {
                theme::workspace::drop_indicator()
            } else {
                theme::error()
            };
            let land = landing(panel, Zone::Edge(side));
            overlay.rect_filled(land.shrink(1.), 2., colour.gamma_multiply(0.22));
            let c = panel.center();
            let line = match side {
                Side::Left | Side::Right => [pos2(c.x, panel.top()), pos2(c.x, panel.bottom())],
                Side::Top | Side::Bottom => [pos2(panel.left(), c.y), pos2(panel.right(), c.y)],
            };
            overlay.line_segment(line, Stroke::new(2_f32, colour));
        }
        if let Over::Panel(target, zone_now) = drag.over
            && target == dot
        {
            if !matches!(drag.dot, Dot::Side(p) if self.is_floating(p)) {
                // The five zones, as a frame round the middle half.
                let middle = Rect::from_center_size(panel.center(), panel.size() * 0.5);
                let faint = Stroke::new(1_f32, accent.gamma_multiply(0.5));
                overlay.rect_stroke(middle, 0., faint, StrokeKind::Middle);
                for (a, b) in [
                    (panel.left_top(), middle.left_top()),
                    (panel.right_top(), middle.right_top()),
                    (panel.left_bottom(), middle.left_bottom()),
                    (panel.right_bottom(), middle.right_bottom()),
                ] {
                    overlay.line_segment([a, b], faint);
                }
            }
            let land = landing(panel, zone_now);
            overlay.rect(
                land.shrink(1.),
                2.,
                accent.gamma_multiply(0.25),
                Stroke::new(1_f32, accent),
                StrokeKind::Inside,
            );
        }
        // The pointer carries a small hollow rectangle: a panel on the move.
        if drag.dot == dot {
            let at = drag.pointer - origin;
            let mark = Rect::from_min_size(at + vec2(10., 12.), vec2(14., 10.));
            overlay.rect_stroke(
                mark,
                1.,
                Stroke::new(1.2_f32, theme::text()),
                StrokeKind::Middle,
            );
        }
    }

    /// UIC-71/72: these are the same actions used by the work-area drop strips.
    pub(super) fn panel_dock_menu(&mut self, ui: &mut Ui, dot: Dot) {
        self.menu_owned_input = true;
        let rules = self.dot_rules(dot);
        if ui
            .gated(self.dot_reasons(dot, rules).detach, Button::new("Float"))
            .clicked()
        {
            self.dots
                .requests
                .push(Request::Menu(dot, MenuAction::Detach));
            ui.close();
        }
        if self.dots.docking.has_work_area() {
            for (side, label) in [
                (Side::Left, "Dock at Left"),
                (Side::Right, "Dock at Right"),
                (Side::Top, "Dock at Top"),
                (Side::Bottom, "Dock at Bottom"),
            ] {
                if ui.button(label).clicked() {
                    self.dots.requests.push(Request::DockAtEdge(dot, side));
                    ui.close();
                }
            }
        }
    }

    /// The dot's right-click menu. Each entry is the same action the View
    /// menu runs; greyed where it can't apply.
    fn dot_menu(&mut self, ui: &mut Ui, dot: Dot) {
        let view = matches!(dot, Dot::View(_));
        if view {
            self.menu_owned_input = true;
        } else {
            self.panel_dock_menu(ui, dot);
            ui.separator();
        }
        let rules = self.dot_rules(dot);
        let why = self.dot_reasons(dot, rules);
        let mut entry = |ui: &mut Ui, why: Option<&str>, label: &str, action: MenuAction| {
            if ui.gated(why, Button::new(label)).clicked() {
                self.dots.requests.push(Request::Menu(dot, action));
                ui.close();
            }
        };
        entry(ui, why.copy, "Duplicate", MenuAction::Duplicate);
        entry(ui, why.copy, "Copy", MenuAction::Copy);
        if view {
            entry(ui, why.detach, "Detach", MenuAction::Detach);
        }
        entry(ui, why.delete, "Delete", MenuAction::Delete);
        ui.separator();
        entry(
            ui,
            why.maximize,
            if rules.maximized {
                "Restore"
            } else {
                "Maximize"
            },
            MenuAction::Maximize,
        );
        ui.separator();
        entry(
            ui,
            why.copy.or(why.split_horizontal),
            "Split Horizontal",
            MenuAction::SplitHorizontal,
        );
        entry(
            ui,
            why.copy.or(why.split_vertical),
            "Split Vertical",
            MenuAction::SplitVertical,
        );
        entry(
            ui,
            why.copy.or(why.split_quad),
            "Split Quad",
            MenuAction::SplitQuad,
        );
    }

    /// Why each dot-menu entry `rules` greys is unavailable.
    fn dot_reasons(&self, dot: Dot, rules: DotRules) -> DotReasons {
        let gate = crate::unavailable::unless;
        let too_small = "The panel is too small to split that way";
        let size = self.dot_panel_rect(dot);
        let fits = |side| size.is_none_or(|r| self.split_fits(dot, r, side));
        let split_horizontal = gate(fits(Side::Right), too_small);
        let split_vertical = gate(fits(Side::Bottom), too_small);
        let split_quad = gate(fits(Side::Right) && fits(Side::Bottom), too_small);
        match dot {
            Dot::View(view) => {
                let (alone, last_in_main) = self.view_alone(view);
                DotReasons {
                    copy: gate(rules.copy, "Not available for this panel"),
                    detach: gate(
                        rules.detach,
                        if last_in_main {
                            "The main window keeps its last panel"
                        } else {
                            "Already alone in its own window"
                        },
                    ),
                    delete: gate(
                        rules.delete,
                        if alone || last_in_main {
                            "A window's last panel can't be deleted"
                        } else {
                            "Restore the maximised view first"
                        },
                    ),
                    maximize: gate(
                        rules.maximize,
                        "Split the view first: a lone view is already full size",
                    ),
                    split_horizontal,
                    split_vertical,
                    split_quad,
                }
            }
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => DotReasons {
                copy: gate(rules.copy, "Side panels can't be copied or split"),
                detach: gate(rules.detach, "Already in its own window"),
                delete: gate(rules.delete, "Side panels can't be deleted"),
                maximize: gate(rules.maximize, "Side panels can't be maximised"),
                split_horizontal,
                split_vertical,
                split_quad,
            },
        }
    }

    /// Which dot-menu entries apply to a panel. The views' trees may be
    /// out of `views` while this runs, so group facts come from
    /// `corner.groups` and `dots.view_windows`, noted before drawing.
    fn dot_rules(&self, dot: Dot) -> DotRules {
        match dot {
            Dot::View(view) => {
                let (in_group, maximized) =
                    self.corner.groups.get(&view).copied().unwrap_or_default();
                let (alone, last_in_main) = self.view_alone(view);
                DotRules {
                    copy: true,
                    detach: !alone,
                    delete: !alone && !maximized,
                    maximize: in_group,
                    maximized,
                }
                .with_last(last_in_main)
            }
            Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => DotRules {
                copy: false,
                detach: self.dock_source_in_main(dot),
                delete: false,
                maximize: false,
                maximized: false,
            },
            Dot::Side(panel) => DotRules {
                copy: false,
                detach: self.dock_source_in_main(Dot::Side(panel)),
                delete: false,
                maximize: false,
                maximized: false,
            },
        }
    }

    /// Whether `view` is alone in its window, and whether that window is
    /// the main one (the main window's last panel). From the note taken
    /// before drawing: while views draw, their trees are out of `views`.
    fn view_alone(&self, view: u32) -> (bool, bool) {
        let window = self.dots.view_windows.get(&view).copied().flatten();
        let mut others = self
            .dots
            .view_windows
            .iter()
            .filter(|(v, w)| **w == window && **v != view);
        let alone = others.next().is_none();
        (alone, alone && window.is_none())
    }

    /// Run what the dots asked for once every window has drawn.
    pub(super) fn dot_requests(&mut self) {
        for request in std::mem::take(&mut self.dots.requests) {
            match request {
                Request::Activate(dot) => self.activate_dot(dot),
                Request::Menu(dot, action) => self.dot_menu_action(dot, action),
                Request::Float(dot) => {
                    let rect = self
                        .dot_panel_rect(dot)
                        .unwrap_or(Rect::from_min_size(Pos2::ZERO, vec2(320., 420.)));
                    let (at, size) = self.place_window(
                        Some([rect.left(), rect.top()]),
                        [rect.width(), rect.height()],
                        Some(rect.center()),
                    );
                    if let Err(error) = self.float_dock_grip(dot, size, at) {
                        self.status = error;
                    }
                }
                Request::DockAtEdge(dot, side) => match self.work_edge_target(side) {
                    Ok(target) => {
                        let rect = self.dot_panel_rect(dot).unwrap_or(Rect::ZERO);
                        self.dot_drop(
                            dot,
                            Gesture::Move,
                            Over::Dock(target),
                            Vec2::ZERO,
                            rect.center(),
                            rect.size(),
                        );
                    }
                    Err(error) => self.status = error,
                },
                Request::WindowDrop(source, target) => {
                    if let Err(error) = self.apply_window_drop(source, target) {
                        self.status = error;
                    }
                }
                Request::TabDrop(source, over, at, size) => {
                    if let Err(error) = self.apply_tab_drop(source, over, at, size) {
                        self.status = error;
                    }
                }

                Request::Drop {
                    dot,
                    gesture,
                    over,
                    start,
                    at,
                    size,
                } => self.dot_drop(dot, gesture, over, at - start, at, size),
            }
        }
    }

    pub(super) fn activate_dot(&mut self, dot: Dot) {
        match dot {
            Dot::View(view) => {
                self.dots.active_side = None;
                if self.views.view_pane(view).is_some() {
                    self.activate_view(view);
                } else if self.views.pane_with_key(view).is_some() {
                    // An empty panel (LAY-74) is the active slot, with no
                    // camera of its own: the content arrow and the dot's
                    // menu act on it.
                    self.cameras.insert(self.active_view, self.camera.clone());
                    self.displays.insert(self.active_view, self.display.clone());
                    self.active_view = view;
                }
            }
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => {
                self.dots.active_side = Some(dot)
            }
        }
    }

    /// The panel's last rect on screen.
    pub(crate) fn dot_panel_rect(&self, dot: Dot) -> Option<Rect> {
        self.dots
            .drawn
            .iter()
            .chain(&self.dots.previous)
            .find(|d| d.dot == dot)
            .map(|d| d.rect)
    }

    /// A dot-menu entry (the View menu's entries call the same actions).
    pub(super) fn dot_menu_action(&mut self, dot: Dot, action: MenuAction) {
        let rect = self.dot_panel_rect(dot);
        let size = rect.map_or([640., 480.], |r| [r.width(), r.height()]);
        let near = rect.map(|r| [r.left() + DETACH_OFFSET, r.top() + DETACH_OFFSET]);
        // Detach opens the window over the panel's old place (VST-31).
        let over = rect.map(|r| [r.left(), r.top()]);
        let pointer = rect.map(|r| r.center());
        match dot {
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => {
                if action == MenuAction::Detach && self.dock_source_in_main(dot) {
                    let (at, size) = self.place_window(near, size, pointer);
                    if let Err(error) = self.float_dock_grip(dot, size, at) {
                        self.status = error;
                    }
                }
            }
            Dot::View(view) => match action {
                MenuAction::Duplicate => self.duplicate_view(view),
                MenuAction::Copy => {
                    let copy = self.copy_pane(view);
                    let (at, size) = self.place_window(near, size, pointer);
                    self.views.new_window(copy, size, at);
                }
                MenuAction::Detach => {
                    let (at, size) = self.place_window(over, size, pointer);
                    self.detach_view(view, size, at);
                }
                MenuAction::Delete => {
                    if self.dot_rules(dot).delete {
                        self.close_view(view);
                    }
                }
                MenuAction::Maximize => {
                    if self.views.in_group(view) {
                        self.toggle_maximized(view);
                    }
                }
                MenuAction::SplitHorizontal => self.split_panel(view, Side::Right),
                MenuAction::SplitVertical => self.split_panel(view, Side::Bottom),
                MenuAction::SplitQuad => self.split_quad(view),
            },
        }
    }

    /// A drag's end, as if released over `over` (tests: windows drawn
    /// inside the main one cover it, so a drop back onto the main window
    /// can't be aimed by pointer there).
    #[cfg(test)]
    pub(super) fn drop_for_test(&mut self, dot: Dot, over: Over) {
        self.dot_drop(
            dot,
            Gesture::Move,
            over,
            Vec2::ZERO,
            Pos2::ZERO,
            vec2(400., 300.),
        );
    }

    /// The end of a dot drag.
    fn dot_drop(
        &mut self,
        dot: Dot,
        gesture: Gesture,
        over: Over,
        moved: Vec2,
        at: Pos2,
        size: Vec2,
    ) {
        if gesture == Gesture::Move
            && let Over::Dock(target) = over
        {
            let result = if let Dot::Side(panel) = dot
                && self.is_floating(panel)
            {
                self.apply_window_drop(floating::DockWindow::Sidebar(panel), target)
            } else if let Some(window) = self.views.window_of(self.dock_pane(dot)) {
                self.apply_window_drop(floating::DockWindow::Tree(window), target)
            } else {
                self.apply_dock_target(dot, target)
            };
            if let Err(error) = result {
                self.status = error;
            }
            return;
        }
        if gesture == Gesture::Move
            && let Over::Panel(target, zone) = over
            && (!matches!(dot, Dot::View(_)) || !matches!(target, Dot::View(_)))
            && !matches!(dot, Dot::Side(panel) if self.is_floating(panel))
        {
            let result = if dot == Dot::ToolTabs || target == Dot::ToolTabs {
                self.apply_dock_target(dot, DockTarget::Compass(target, zone))
            } else {
                self.move_dock_panel(dot, target, zone)
            };
            if let Err(error) = result {
                self.status = error;
            }
            return;
        }
        let pointer = at;
        let size = [size.x, size.y];
        let (at, size) = self.place_window(Some([at.x, at.y]), size, Some(pointer));
        match dot {
            Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => {
                if over == Over::Outside
                    && self.dock_source_in_main(dot)
                    && let Err(error) = self.float_dock_grip(dot, size, at)
                {
                    self.status = error;
                }
            }
            Dot::Side(panel) => match over {
                Over::Outside if self.dock_source_in_main(dot) => {
                    if let Err(error) = self.float_dock_grip(dot, size, at) {
                        self.status = error;
                    }
                }
                Over::Panel(..) if self.is_floating(panel) => {
                    self.set_floating(panel, false);
                    self.status = format!("{} docked", panel.label());
                }
                _ => {}
            },
            Dot::View(view) => {
                let pane = self.view_pane_of(view);
                let result = match (gesture, over) {
                    (Gesture::Split, _) => {
                        self.split_panel(view, split_side(moved));
                        Ok(())
                    }
                    (Gesture::Move, Over::Panel(Dot::View(target), Zone::Center)) => {
                        self.views.swap(pane, self.view_pane_of(target))
                    }
                    (Gesture::Move, Over::Panel(Dot::View(target), Zone::Edge(side))) => self
                        .views
                        .dock_beside(pane, self.view_pane_of(target), side),
                    // A view alone in its window can't leave it empty: the
                    // window keeps it and a copy opens at the release point.
                    (Gesture::Move, Over::Outside) if self.views.alone_in_window(pane) => {
                        self.copy_to_window(view, size, at);
                        Ok(())
                    }
                    (Gesture::Move, Over::Outside)
                        if !self.view_alone(view).1 || self.outside_main(pointer) =>
                    {
                        self.float_dock_grip(dot, size, at)
                    }
                    (Gesture::Copy, Over::Panel(Dot::View(target), Zone::Edge(side))) => {
                        let copy = self.copy_pane(view);
                        let target = self.view_pane_of(target);
                        let result = self.views.insert_beside(copy, target, side);
                        if result.is_ok()
                            && let Some(key) = copy.view_key()
                        {
                            self.activate_dot(Dot::View(key));
                        }
                        result
                    }
                    (Gesture::Copy, Over::Panel(_, Zone::Center)) => {
                        Err("Drop a copy on a panel's edge to dock it there".into())
                    }
                    (Gesture::Copy, Over::Outside) => {
                        self.copy_to_window(view, size, at);
                        Ok(())
                    }
                    _ => return,
                };
                match result {
                    Ok(()) => {
                        self.forget_unlisted_views();
                    }
                    Err(error) => self.status = error,
                }
            }
        }
    }

    fn move_dock_panel(&mut self, from: Dot, to: Dot, zone: Zone) -> Result<()> {
        if from == to {
            return Ok(());
        }
        let (source, target) = (self.dock_pane(from), self.dock_pane(to));
        let is_main = |dot, pane| match dot {
            Dot::View(_) => self.views.main.tiles.find_pane(&pane).is_some(),
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => {
                self.dock_source_in_main(dot)
            }
        };
        if !is_main(from, source) || !is_main(to, target) {
            return Err("Drag between two docked panels to rearrange the workspace".into());
        }
        let fits = |pane: Pane, size: Vec2| {
            let [width, height] = pane.dock_min_size();
            size.x + 0.01 >= width && size.y + 0.01 >= height
        };
        match zone {
            Zone::Center => {
                if self
                    .dot_panel_rect(to)
                    .is_some_and(|r| !fits(source, r.size()))
                    || self
                        .dot_panel_rect(from)
                        .is_some_and(|r| !fits(target, r.size()))
                {
                    return Err("These panels need more room to swap".into());
                }
            }
            Zone::Edge(side) => {
                // A panel stacked in a column (the Toolbox over Tool
                // Properties) gets the newcomer beside it by widening the
                // column, so neither stacked panel loses width (Jason,
                // 2026-11-19); other targets split their own room.
                if !self.stacked_in_column(target, side)
                    && let Some(rect) = self.dot_panel_rect(to)
                {
                    let mut size = rect.size();
                    match side {
                        Side::Left | Side::Right => size.x = (size.x - operandi_layout::GAP) / 2.,
                        Side::Top | Side::Bottom => size.y = (size.y - operandi_layout::GAP) / 2.,
                    }
                    if !fits(target, size) || !fits(source, size) {
                        return Err("The target panel is too small to split that way".into());
                    }
                }
            }
        }
        let widen = match zone {
            Zone::Edge(side) if self.stacked_in_column(target, side) => {
                let [min_width, _] = source.dock_min_size();
                // Tool Tabs beside a panel is a vertical strip of fixed
                // width; other panels keep their current width.
                let width = if source == Pane::ToolTabs {
                    crate::toolbox::VERTICAL_TOOL_TABS_WIDTH
                } else {
                    self.dot_panel_rect(from)
                        .map_or(min_width, |r| r.width().max(min_width))
                };
                self.dot_panel_rect(to).map(|r| (r.width(), width))
            }
            _ => None,
        };
        let mut next = self.views.clone();
        self.attach_dock_columns(&mut next)?;
        if source == Pane::Toolbox {
            Self::materialize_tool_tabs(&mut next)?;
        }
        match zone {
            Zone::Center => next.swap(source, target)?,
            Zone::Edge(side) => {
                // Docking against a maximised panel restores the layout
                // first, preserving the views hidden by Maximise.
                next.maximized = None;
                let slot = next.dock_slot(target).ok_or("No dock target")?;
                if next.main.tiles.find_pane(&target) == Some(slot)
                    && next.main.tiles.find_pane(&source) == next.dock_slot(source)
                {
                    next.dock_beside(source, target, side)?;
                } else {
                    next.dock_at(source, slot, side, None)?;
                }
                if let Some((kept, added)) = widen {
                    self.widen_stacked_row(&mut next, target, kept, added);
                }
            }
        }
        self.install_dock_tree(next);
        self.activate_dot(from);
        Ok(())
    }

    /// Whether `target` sits in a column of stacked panels (its slot's
    /// parent splits top to bottom) and the drop is beside it.
    pub(crate) fn stacked_in_column(&self, target: Pane, side: Side) -> bool {
        if !matches!(side, Side::Left | Side::Right) {
            return false;
        }
        let tiles = &self.views.main.tiles;
        let Some(slot) = self.views.dock_slot(target) else {
            return false;
        };
        tiles.parent_of(slot).is_some_and(|parent| {
            matches!(
                tiles.get(parent),
                Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(l)))
                    if l.dir == egui_tiles::LinearDir::Vertical
            )
        })
    }

    /// After docking a panel beside a stacked one: the new row keeps the
    /// target at `kept` px and gives the newcomer `added` px, and the
    /// column grows by `added` (plus the gap), taken from its widest
    /// neighbour in the row of columns, so the other stacked panels keep
    /// at least their width.
    pub(crate) fn widen_stacked_row(
        &self,
        next: &mut operandi_layout::Layout,
        target: Pane,
        kept: f32,
        added: f32,
    ) {
        use egui_tiles::{Container, Tile};
        let old = &self.views.main.tiles;
        let tiles = &mut next.main.tiles;
        let Some(target_id) = tiles.find_pane(&target) else {
            return;
        };
        let Some(row) = tiles.parent_of(target_id) else {
            return;
        };
        if let Some(Tile::Container(Container::Linear(linear))) = tiles.get_mut(row) {
            let children = linear.children.clone();
            for child in children {
                let px = if child == target_id { kept } else { added };
                linear.shares.set_share(child, px.max(1.));
            }
        }
        let Some(column) = tiles.parent_of(row) else {
            return;
        };
        let Some(outer) = tiles.parent_of(column) else {
            return;
        };
        let Some(outer_rect) = old.rect(outer) else {
            return;
        };
        let Some(Tile::Container(Container::Linear(linear))) = tiles.get_mut(outer) else {
            return;
        };
        if linear.dir != egui_tiles::LinearDir::Horizontal {
            return;
        }
        let total: f32 = linear.children.iter().map(|&c| linear.shares[c]).sum();
        if total <= 0. || outer_rect.width() <= 0. {
            return;
        }
        let per_px = total / outer_rect.width();
        let grow = (added + operandi_layout::GAP) * per_px;
        let Some(&donor) = linear
            .children
            .iter()
            .filter(|&&c| c != column)
            .max_by(|&&a, &&b| linear.shares[a].total_cmp(&linear.shares[b]))
        else {
            return;
        };
        let give = grow.min(linear.shares[donor] * 0.5);
        let column_share = linear.shares[column] + give;
        let donor_share = linear.shares[donor] - give;
        linear.shares.set_share(column, column_share);
        linear.shares.set_share(donor, donor_share);
    }

    /// The layout pane of the view with this key (3D or UV).
    pub(super) fn view_pane_of(&self, view: u32) -> Pane {
        self.views
            .pane_with_key(view)
            .unwrap_or(Pane::Viewport(view))
    }

    /// A copy of `view` of the same kind (a UV view's copy is a UV view,
    /// VST-17), with its settings; not placed in the layout yet. An empty
    /// panel's copy is another empty panel (LAY-74).
    fn copy_pane(&mut self, view: u32) -> Pane {
        let kind = self.view_pane_of(view);
        if let Pane::Missing(key) = kind {
            let copy = self.views.new_viewport_key();
            let name = self
                .views
                .missing
                .get(&key)
                .cloned()
                .unwrap_or_else(|| operandi_layout::EMPTY.into());
            self.views.missing.insert(copy, name);
            return Pane::Missing(copy);
        }
        kind.with_key(self.copy_view(view))
    }

    /// Whether `dot`'s panel, drawn at `rect`, splits on `side` with both
    /// halves at least the minimum size (VST-24).
    fn split_fits(&self, dot: Dot, rect: Rect, side: Side) -> bool {
        let pane = match dot {
            Dot::View(view) => self.view_pane_of(view),
            Dot::Side(_) | Dot::ToolProperties | Dot::ToolTabs | Dot::Tab(_) => return false,
        };
        operandi_layout::split_fits(pane, [rect.width(), rect.height()], side)
    }

    /// Whether `view` splits on `side` at its last drawn size (true while
    /// it hasn't drawn yet).
    pub(super) fn view_splits(&self, view: u32, side: Side) -> bool {
        let dot = Dot::View(view);
        self.dot_panel_rect(dot)
            .is_none_or(|rect| self.split_fits(dot, rect, side))
    }

    /// A new floating window's place and size, pulled fully onto the
    /// screen the pointer is on and no smaller than a floating window may
    /// be (VST-31, VST-27). With no place, the system places it.
    pub(super) fn place_window(
        &self,
        at: Option<[f32; 2]>,
        size: [f32; 2],
        pointer: Option<Pos2>,
    ) -> (Option<[f32; 2]>, [f32; 2]) {
        let min = operandi_layout::WINDOW_MIN;
        let size = [size[0].max(min[0]), size[1].max(min[1])];
        let Some(at) = at else {
            return (None, size);
        };
        let ppp = if self.dots.pixels_per_point > 0. {
            self.dots.pixels_per_point
        } else {
            1.
        };
        let screens = window_state::screens(ppp).unwrap_or_default();
        let (at, size) = window_state::place_on_screen(at, size, min, &screens, pointer);
        (Some(at), size)
    }

    /// A new view with `view`'s camera (type, angle, zoom) and display
    /// settings; not placed in the layout yet. Returns its key. The
    /// maximise state and any panel name aren't copied (VST-15). This is
    /// the one place a view's own state is copied: a UV view's state is
    /// copied here too.
    pub(super) fn copy_view(&mut self, view: u32) -> u32 {
        let copy = self.views.new_viewport_key();
        let (camera, display) = if view == self.active_view {
            (self.camera.clone(), self.display.clone())
        } else {
            (
                self.cameras.get(&view).cloned().unwrap_or_default(),
                self.displays
                    .get(&view)
                    .cloned()
                    .unwrap_or_else(|| self.new_view_display()),
            )
        };
        self.cameras.insert(copy, camera);
        self.displays.insert(copy, display);
        // A UV view's map, framing and overlays come along (uv_view.rs).
        if let Some(uv) = self.uv_views.get(&view).cloned() {
            self.uv_views.insert(copy, uv);
        }
        copy
    }

    /// A copy of `view` alone in a new window.
    fn copy_to_window(&mut self, view: u32, size: [f32; 2], at: Option<[f32; 2]>) {
        let copy = self.copy_pane(view);
        self.views.new_window(copy, size, at);
        self.status = "Panel copied to a new window".into();
    }

    /// Split Horizontal / Vertical (and View > Split View, and Ctrl+drag):
    /// `view`'s place splits at half and a copy takes the `side` half;
    /// the copy becomes the active view.
    /// A panel too small for both halves to keep the minimum size isn't
    /// split (VST-24).
    pub(super) fn split_panel(&mut self, view: u32, side: Side) {
        if let Some(rect) = self.dot_panel_rect(Dot::View(view))
            && !self.split_fits(Dot::View(view), rect, side)
        {
            self.status = "The view is too small to split that way".into();
            return;
        }
        let source = self.view_pane_of(view);
        let copy = self.copy_pane(view);
        match self.views.insert_beside(copy, source, side) {
            Ok(()) => {
                // Splitting a maximised view closes the views it hid.
                self.forget_unlisted_views();
                self.activate_dot(Dot::View(copy.view_key().unwrap_or(view)));
            }
            Err(error) => {
                self.forget_unlisted_views();
                self.status = error;
            }
        }
    }

    /// Split Quad: four equal views in `view`'s place, the three new ones
    /// copies of it; the dividers form one linked cross; the bottom-right
    /// view becomes active.
    pub(super) fn split_quad(&mut self, view: u32) {
        if let Some(rect) = self.dot_panel_rect(Dot::View(view))
            && !(self.split_fits(Dot::View(view), rect, Side::Right)
                && self.split_fits(Dot::View(view), rect, Side::Bottom))
        {
            self.status = "The view is too small to split into four".into();
            return;
        }
        let others = [
            self.copy_view(view),
            self.copy_view(view),
            self.copy_view(view),
        ];
        match self.views.split_quad(view, others) {
            Ok(()) => {
                self.forget_unlisted_views();
                self.activate_dot(Dot::View(others[2]));
            }
            Err(error) => {
                self.forget_unlisted_views();
                self.status = error;
            }
        }
    }

    /// Duplicate: a copy docked at the right of the view's window layout.
    fn duplicate_view(&mut self, view: u32) {
        let copy = self.copy_pane(view);
        let window = self.views.window_of(self.view_pane_of(view));
        match self.views.add_at_right(copy, window) {
            Ok(()) => {}
            Err(error) => {
                self.forget_unlisted_views();
                self.status = error;
            }
        }
    }

    /// Detach (and View > Tear Off View): the view into its own window,
    /// near where it was, at its size.
    pub(super) fn detach_view(&mut self, view: u32, size: [f32; 2], near: Option<[f32; 2]>) {
        match self.views.tear_off_at(self.view_pane_of(view), size, near) {
            Ok(_) => {}
            Err(error) => self.status = error,
        }
    }

    /// Float a side panel at a point (its window's top-left) and size.
    fn float_side_panel(&mut self, panel: Panel, size: [f32; 2], at: Option<[f32; 2]>) {
        if let Some(at) = at {
            self.dots.side_positions.insert(panel, at);
        }
        if let Some(hash) = self.palettes.side_hash(panel).map(str::to_owned) {
            let position = at.or(self.palettes.place(&hash).position);
            self.palettes
                .places
                .insert(hash, crate::palettes::Place { size, position });
        }
        self.set_floating(panel, true);
    }

    /// Drop the cameras and settings of views no longer in the layout (a
    /// copy that found no place, a discarded window's views).
    pub(super) fn forget_unlisted_views(&mut self) {
        let listed = self.views.views();
        let gone: Vec<u32> = self
            .cameras
            .keys()
            .chain(self.displays.keys())
            .copied()
            .filter(|v| !listed.contains(v))
            .collect();
        for view in gone {
            self.cameras.remove(&view);
            self.displays.remove(&view);
            self.view_rects.remove(&view);
            if let Some(gpu) = &mut self.viewport_gpu {
                gpu.forget(view);
            }
        }
        if !listed.contains(&self.active_view)
            && let Some(&first) = self.views.group_viewports(None).first().or(listed.first())
        {
            self.active_view = first;
            self.camera = self.cameras.get(&first).cloned().unwrap_or_default();
            self.display = self
                .displays
                .get(&first)
                .cloned()
                .unwrap_or_else(|| self.new_view_display());
            if let Some(&rect) = self.view_rects.get(&first) {
                self.viewport_rect = rect;
            }
        }
    }
}

/// Why each dot-menu entry is greyed (`None` = available).
struct DotReasons {
    copy: Option<&'static str>,
    detach: Option<&'static str>,
    delete: Option<&'static str>,
    maximize: Option<&'static str>,
    /// Too small to split that way (VST-24).
    split_horizontal: Option<&'static str>,
    split_vertical: Option<&'static str>,
    split_quad: Option<&'static str>,
}

/// Which dot-menu entries are on.
#[derive(Clone, Copy)]
struct DotRules {
    /// Duplicate, Copy and the splits.
    copy: bool,
    detach: bool,
    delete: bool,
    maximize: bool,
    maximized: bool,
}

impl DotRules {
    /// The main window's last panel can't be deleted or detached.
    fn with_last(self, last: bool) -> Self {
        if last {
            Self {
                detach: false,
                delete: false,
                ..self
            }
        } else {
            self
        }
    }
}
