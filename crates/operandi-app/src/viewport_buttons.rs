//! The 3D view header's corner buttons (BEHAVIOR-VIEWPORT-BUTTONS v1), left
//! to right at the header's right end: Pan, Rotate, Zoom, Maximise/Restore
//! and the Viewport Properties gear. At the left, after the Display menu:
//! one Transparency pop-up button (HT-1, replacing XR-1's three), a small
//! separator (HT-2) and three view toggles (HT-3): Show Locators, Make
//! Inactive Invisible, Make Inactive Same As Active.
//!
//! Maximise shows one view of a view group (the main window's views, or a
//! torn-off window's) alone in the group's frame; the layout tree is left
//! untouched, so Restore brings back the splits exactly. It is the same
//! view enlarged: its camera and display settings are never copied.
use super::*;
use crate::unavailable::Gate;
use crate::view_display::TransparencyMode;

/// Size of one corner button (points; it was 18 in the header strip).
const BUTTON: f32 = 16.;
/// The floating header's height (points; the strip it replaced was 22)
/// and its gap from the view's top edge.
pub const HEADER_HEIGHT: f32 = 19.;
const HEADER_TOP: f32 = 3.;
/// How far the view's top-left information text moves down to clear the
/// floating header.
pub const HEADER_CLEARANCE: f32 = HEADER_TOP + HEADER_HEIGHT;
/// Space between corner buttons.
const GAP: f32 = 2.;
/// The content arrow's width at the header's right end (LAY-73).
pub const ARROW: f32 = 12.;
/// A Light sub-menu longer than this gets a filter field (UIC-42), as
/// form choice lists do.
const LIGHT_FILTER_FROM: usize = 12;
/// Extra space between the Transparency buttons and the navigation ones.
const GROUP_GAP: f32 = 6.;
/// The small separator after the Transparency button (HT-2): a gap, a
/// 1 pt line at 60% of the button height, a gap.
const SEPARATOR_GAP: f32 = 4.;
/// Press and hold on the Transparency button this long (seconds) opens
/// its list (HT-1; HT-Q2, 400 ms assumed).
const TRANSPARENCY_HOLD: f64 = 0.4;
/// The Transparency list's entry size.
const LIST_ENTRY: Vec2 = vec2(116., 20.);
/// A header narrower than this drops the Transparency button and the view
/// toggles (the navigation buttons and the gear always stay).
const TRANSPARENCY_MIN_HEADER: f32 = 200.;
/// The Pan button moves the view this many times the pointer's travel
/// (v1.1: "about 3x", tuned by feel).
pub const PAN_BUTTON_RATE: f32 = 3.;

/// The mouse's own movement this frame, in points (`None` when the
/// platform sent none). macOS reports it in points already; elsewhere it
/// comes in device units, taken as physical pixels (VBF-Q1, provisional).
fn raw_motion(ui: &Ui) -> Option<Vec2> {
    ui.input(|i| {
        let mut sum = None;
        for e in &i.events {
            if let Event::MouseMoved(d) = e {
                *sum.get_or_insert(Vec2::ZERO) += *d;
            }
        }
        if cfg!(target_os = "macos") {
            sum
        } else {
            sum.map(|d| d / i.pixels_per_point)
        }
    })
}

/// Whether a corner drag locks the pointer in place (macOS, VBF-2) rather
/// than putting it back on the icon after every move.
/// Tests use the put-back way unless they set `CornerState::lock_pointer`.
const LOCK_POINTER: bool = cfg!(target_os = "macos") && !cfg!(test);

/// A corner drag's end (release or Esc): the pointer is unlocked and comes
/// back where it was pressed.
fn end_corner_drag(ui: &Ui, drag: &HeaderDrag) {
    if drag.locked {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::CursorGrab(egui::CursorGrab::None));
    }
    if drag.moved || drag.locked {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::CursorPosition(drag.home));
    }
}

/// One corner button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Corner {
    Pan,
    Rotate,
    Zoom,
    Maximize,
    Properties,
    /// The Transparency pop-up button (HT-1): a click toggles the active
    /// meshes between Normal and the last mode, hold or right-click opens
    /// the list; Alt for the inactive meshes (UIC-27).
    Transparency,
    /// An on/off view toggle after the separator (HT-3).
    Toggle(ViewToggle),
}

/// The header's view toggles (HT-3), left to right. Each runs its existing
/// Viewport Properties command (UIC-165) and lights from its `?` answer
/// (UIC-223).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ViewToggle {
    /// HT-5 Show Weightmaps.
    Weights,
    /// HT-6 Show UV Distortion.
    Distortion,
    Locators,
    InactiveInvisible,
    SameAsActive,
}

impl ViewToggle {
    pub const ALL: [ViewToggle; 5] = [
        ViewToggle::Weights,
        ViewToggle::Distortion,
        ViewToggle::Locators,
        ViewToggle::InactiveInvisible,
        ViewToggle::SameAsActive,
    ];
    /// The command it runs (`<command> true|false`).
    pub fn command(self) -> &'static str {
        match self {
            ViewToggle::Weights => "view3d.showweights",
            ViewToggle::Distortion => "view3d.showDistortion",
            ViewToggle::Locators => "view3d.showLocators",
            ViewToggle::InactiveInvisible => "view3d.inactiveInvisible",
            ViewToggle::SameAsActive => "view3d.sameAsActive",
        }
    }
    fn tip(self) -> &'static str {
        match self {
            ViewToggle::Weights => "Show Weightmaps: colour meshes by the selected weight map",
            ViewToggle::Distortion => {
                "Show UV Distortion: colour polygons by UV stretch in the selected UV map"
            }
            ViewToggle::Locators => "Show Locators: draw group locators in this view",
            ViewToggle::InactiveInvisible => {
                "Make Inactive Invisible: hide meshes that aren't active"
            }
            ViewToggle::SameAsActive => {
                "Make Inactive Same As Active: draw inactive meshes like active ones"
            }
        }
    }
    /// The Viewport Properties row it shares (the same setting, so the
    /// two always agree), for the tooltip's key hint.
    fn label(self) -> &'static str {
        match self {
            ViewToggle::Weights => "Show Weightmaps",
            ViewToggle::Distortion => "Show UV Distortion",
            ViewToggle::Locators => "Show Locators",
            ViewToggle::InactiveInvisible => "Make Inactive Invisible",
            ViewToggle::SameAsActive => "Make Inactive Same As Active",
        }
    }
    fn glyph(self) -> Glyph {
        match self {
            ViewToggle::Weights => Glyph::Weights,
            ViewToggle::Distortion => Glyph::Distortion,
            ViewToggle::Locators => Glyph::Locators,
            ViewToggle::InactiveInvisible => Glyph::InactiveInvisible,
            ViewToggle::SameAsActive => Glyph::SameAsActive,
        }
    }
    /// Why the toggle is greyed, if it is: the two tints colour fills, so
    /// not with the Wireframe style (like the Transparency button).
    pub fn unavailable(self, settings: &view_display::Settings) -> Option<&'static str> {
        match self {
            ViewToggle::Weights | ViewToggle::Distortion => {
                transparency_unavailable(settings, false)
            }
            _ => None,
        }
    }
}

/// The Transparency button's open list (HT-1).
#[derive(Clone, Debug)]
pub struct TransparencyList {
    pub view: u32,
    /// Opened by holding the button: letting go over an entry picks it.
    by_hold: bool,
    /// Each entry's rect last frame (tests, hold-slide-release).
    pub entries: Vec<(TransparencyMode, Rect)>,
    /// The whole list last frame.
    pub rect: Option<Rect>,
}

/// A press on the Transparency button, while it is down.
#[derive(Clone, Copy, Debug)]
struct TransparencyPress {
    view: u32,
    start: f64,
    /// The press opened (or closed) the list, so its release isn't a
    /// quick click.
    handled: bool,
}

impl Corner {
    /// The in-view gesture a drag button stands for.
    fn navigation(self) -> Option<input::NavigationKind> {
        match self {
            Corner::Pan => Some(input::NavigationKind::Pan),
            Corner::Rotate => Some(input::NavigationKind::Orbit),
            Corner::Zoom => Some(input::NavigationKind::Zoom),
            Corner::Maximize | Corner::Properties | Corner::Transparency | Corner::Toggle(_) => {
                None
            }
        }
    }
}

/// A drag on Pan, Rotate or Zoom: it runs from press to release, anywhere
/// on screen, and Esc puts the view back as it was at the press and ends it.
pub struct HeaderDrag {
    pub view: u32,
    pub button: Corner,
    /// The view's camera at the press.
    before: Camera,
    /// Where the pointer goes back to on release.
    home: Pos2,
    /// Whether the pointer moved (so it was hidden).
    moved: bool,
    /// Whether the platform sends the mouse's own movement (raw motion,
    /// `Event::MouseMoved`). Once it has, the drag reads only that: it is
    /// not upset by the pointer being put back on the icon.
    raw: bool,
    /// Whether the pointer is locked in place for the drag (`LOCK_POINTER`).
    locked: bool,
}

/// Something a corner button asks for once every view has drawn (so no
/// view's state is swapped while another draws).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Request {
    ToggleMaximize(u32),
}

/// The corner buttons' state across frames.
#[derive(Default)]
pub struct CornerState {
    /// Each main-window view's whole pane this frame (header included), for
    /// shortcuts that act on the view under the pointer.
    pub areas: BTreeMap<u32, Rect>,
    /// The main window's inner top-left in screen points, its native and
    /// egui pixels per point, this frame: places the system mouse position
    /// in `areas` when another of our windows has the keyboard.
    pub root_window: Option<(Pos2, f32, f32)>,
    /// Each drawn corner button, by view (tests and popover anchors).
    pub rects: BTreeMap<(u32, Corner), Rect>,
    /// The open Viewport Properties popover's rect last frame.
    pub popover_rect: Option<Rect>,
    /// Each view's place in its group, taken before the views draw (the
    /// layout's trees are out of `views` while they draw): whether it shares
    /// its group, and whether it is the group's maximised view.
    pub groups: BTreeMap<u32, (bool, bool)>,
    /// The view a shortcut acts on (the one under the pointer when it was
    /// pressed).
    pub target: Option<u32>,
    /// Where the pointer was over that view when the shortcut was pressed
    /// (Viewport Properties opens near it).
    pub target_pointer: Option<Pos2>,
    requests: Vec<Request>,
    /// Each view's floating header buttons this frame (presses there
    /// don't reach the view).
    pub header_hits: BTreeMap<u32, Vec<Rect>>,
    /// The Pan, Rotate or Zoom drag in progress.
    pub drag: Option<HeaderDrag>,
    /// Lock the pointer during a drag instead of putting it back after each
    /// move; `None` takes the platform's way (`LOCK_POINTER`). Tests set it.
    pub lock_pointer: Option<bool>,
    /// Views whose size Maximise/Restore just changed, with their height
    /// before it (`keep_world_scale`).
    pub rescale: BTreeMap<u32, f32>,
    /// The Viewport Properties popover (viewport_properties.rs).
    pub properties: viewport_properties::Popover,
    /// The Transparency button's list, while open (HT-1).
    pub transparency_list: Option<TransparencyList>,
    transparency_press: Option<TransparencyPress>,
    /// Each view's last non-Normal mode, active then inactive meshes: what
    /// a quick click on the Transparency button goes back to (X-Ray until
    /// one is used).
    pub last_transparency: BTreeMap<u32, [TransparencyMode; 2]>,
}

impl Operandi {
    /// The buttons this view's header shows, left to right.
    fn corner_buttons(&self) -> Vec<Corner> {
        // Show Viewport Controls and Information off: no corner buttons.
        if !self.display.settings.drawing.controls {
            return Vec::new();
        }
        // The Transparency button and the view toggles sit at the left,
        // after the Display menu (N17, Jason 2026-11-16; HT-1..3).
        let mut buttons: Vec<Corner> = vec![Corner::Pan];
        // Rotate only where the view can orbit: not the six axis views.
        if self.camera.through || matches!(self.camera.view, View::Perspective | View::Orthographic)
        {
            buttons.push(Corner::Rotate);
        }
        buttons.push(Corner::Zoom);
        // Always shown (Jason, 2026-11-06, answering VB-Q1).
        buttons.push(Corner::Maximize);
        buttons.push(Corner::Properties);
        buttons
    }

    /// The view header: its buttons float along the top edge of the view,
    /// over the 3D drawing, each the background colour with a thin edge
    /// (no strip). The corner buttons sit at the right and keep their room;
    /// the view type and Display menus sit at the left and drop out when
    /// there isn't room. Frame All, Frame Selection, Split View and the
    /// navigation help are in the View menu. Each button's rect
    /// is kept so presses on it don't reach the view (`header_hits`);
    /// presses in the gaps do. Show Viewport Controls and Information off
    /// hides them all.
    pub(super) fn view_header(&mut self, ui: &mut Ui, r: Rect) {
        let view = self.drawing_view;
        self.corner.rects.retain(|(v, _), _| *v != view);
        self.corner.header_hits.remove(&view);
        if !self.display.settings.drawing.controls {
            return;
        }
        let mut hits = Vec::new();
        let header = Rect::from_min_size(
            r.min + vec2(4., HEADER_TOP),
            vec2((r.width() - 8.).max(0.), HEADER_HEIGHT),
        );
        let buttons = self.corner_buttons();
        // The content arrow keeps the far right (layouts stage 2, LAY-73):
        // what this slot shows.
        let arrow = Rect::from_center_size(
            pos2(header.right() - ARROW / 2., header.center().y),
            vec2(ARROW, BUTTON),
        );
        hits.push(arrow);
        self.content_arrow(ui, view, arrow);
        let width = buttons.len() as f32 * (BUTTON + GAP) + ARROW + GAP;
        let mut x = header.right() - width;
        for button in buttons {
            let rect = Rect::from_center_size(
                pos2(x + (BUTTON + GAP) / 2., header.center().y),
                Vec2::splat(BUTTON),
            );
            self.corner.rects.insert((view, button), rect);
            hits.push(rect);
            self.corner_button(ui, view, button, rect);
            x += BUTTON + GAP;
        }
        let left = Rect::from_min_max(
            header.min + vec2(panel_dot::VIEW_DOT_SLOT, 0.),
            pos2(
                (header.right() - width - 6.).max(header.left()),
                header.bottom(),
            ),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.set_clip_rect(left.intersect(ui.clip_rect()));
                header_style(ui);
                let fits = |ui: &Ui, width: f32| ui.available_width() >= width;
                if fits(ui, 110.) {
                    let combo = ComboBox::from_id_salt("viewport-view")
                        .width(100.)
                        .selected_text(self.view_title())
                        .show_ui(ui, |ui| {
                            for view in [
                                View::Perspective,
                                View::Orthographic,
                                View::Top,
                                View::Bottom,
                                View::Front,
                                View::Back,
                                View::Right,
                                View::Left,
                            ] {
                                if ui
                                    .selectable_label(
                                        !self.camera.looks_through() && self.camera.view == view,
                                        format!("{view:?}"),
                                    )
                                    .clicked()
                                {
                                    self.choose_view_type(Some(view));
                                }
                            }
                            // CL-1: look through the scene's render camera.
                            let has_camera =
                                camera_light_ui::view_camera(&self.editor.scene).is_some();
                            if ui
                                .gated(
                                    crate::unavailable::unless(
                                        has_camera,
                                        "The scene has no camera",
                                    ),
                                    egui::Button::selectable(self.camera.through, "Camera"),
                                )
                                .clicked()
                            {
                                self.choose_view_type(None);
                            }
                            self.light_view_menu(ui);
                        });
                    hits.push(combo.response.rect);
                }
                if fits(ui, 90.) {
                    hits.push(self.display_menu(ui));
                }
                // N17/HT-1..3: the Transparency button, a small separator
                // and the three view toggles, right after the Display menu
                // and a little apart from it; they drop out together, before
                // the menus, when there isn't room.
                let group = (1 + ViewToggle::ALL.len()) as f32 * (BUTTON + GAP)
                    + GROUP_GAP
                    + SEPARATOR_GAP * 2.
                    + 1.;
                let mut drawn = false;
                if header.width() >= TRANSPARENCY_MIN_HEADER && fits(ui, group) {
                    drawn = true;
                    // The group's own spacing is GAP, like the corner
                    // buttons'.
                    let spacing = ui.spacing().item_spacing.x;
                    ui.spacing_mut().item_spacing.x = 0.;
                    ui.add_space(GROUP_GAP);
                    let rect = header_slot(ui);
                    self.corner.rects.insert((view, Corner::Transparency), rect);
                    hits.push(rect);
                    self.transparency_button(ui, view, rect, &mut hits);
                    // HT-2: gap, a thin line, gap.
                    ui.add_space(SEPARATOR_GAP - GAP);
                    let (line, _) = ui.allocate_exact_size(vec2(1., BUTTON), Sense::hover());
                    ui.painter().rect_filled(
                        Rect::from_center_size(line.center(), vec2(1., BUTTON * 0.6)),
                        0.,
                        theme::viewport::header_border(),
                    );
                    ui.add_space(SEPARATOR_GAP);
                    for toggle in ViewToggle::ALL {
                        let rect = header_slot(ui);
                        let button = Corner::Toggle(toggle);
                        self.corner.rects.insert((view, button), rect);
                        hits.push(rect);
                        self.toggle_button(ui, toggle, rect);
                    }
                    ui.spacing_mut().item_spacing.x = spacing;
                }
                if !drawn
                    && self
                        .corner
                        .transparency_list
                        .as_ref()
                        .is_some_and(|l| l.view == view)
                {
                    self.corner.transparency_list = None;
                }
            },
        );
        self.corner.header_hits.insert(view, hits);
    }

    /// The view menu's Light sub-menu (UIC-10): (Current Selection), then
    /// every light by name. Picking one looks through it; navigating then
    /// moves it, as Camera view moves its camera. Entries act at once, so
    /// no "…" (UIC-03); the light shown is marked (UIC-01) and named in the
    /// header; a long list gets
    /// a filter field (UIC-42).
    fn light_view_menu(&mut self, ui: &mut Ui) {
        let lights: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.light.is_some())
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let shown = self.camera.light;
        let gate = crate::unavailable::unless(!lights.is_empty(), "The scene has no lights");
        ui.gated_ui(gate, |ui| {
            let menu = ui.menu_button("Light", |ui| {
                let selected = camera_light_ui::selected_light(&self.editor.scene);
                let follows = shown.is_some_and(|v| v.follow_selection);
                let current = ui.gated(
                    crate::unavailable::unless(selected.is_some(), "Select a light"),
                    egui::Button::selectable(follows, "(Current Selection)"),
                );
                self.toolbox_rects
                    .insert("Light view: (Current Selection)".into(), current.rect);
                if current.clicked() {
                    self.enter_light_view(None);
                    ui.close();
                }
                let filter_id = ui.id().with("light-view-filter");
                let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
                if lights.len() > LIGHT_FILTER_FROM {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut filter)
                            .hint_text("Filter")
                            .desired_width(f32::INFINITY),
                    );
                    crate::numeric_tab::register(ui, &edit, Some(&filter));
                    if !edit.has_focus() && filter.is_empty() {
                        edit.request_focus();
                    }
                    ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
                }
                let needle = filter.to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(320.)
                    .show(ui, |ui| {
                        for (id, name) in &lights {
                            if !needle.is_empty() && !name.to_lowercase().contains(&needle) {
                                continue;
                            }
                            let on = shown.is_some_and(|v| !v.follow_selection && v.item == *id);
                            let entry = ui.selectable_label(on, name);
                            self.toolbox_rects
                                .insert(format!("Light view: {name}"), entry.rect);
                            if entry.clicked() {
                                self.enter_light_view(Some(*id));
                                ui.data_mut(|d| d.remove::<String>(filter_id));
                                ui.close();
                            }
                        }
                    });
            });
            self.toolbox_rects
                .insert("View menu: Light".into(), menu.response.rect);
        });
    }

    fn corner_button(&mut self, ui: &mut Ui, view: u32, button: Corner, rect: Rect) {
        let id = ui.id().with(("corner", button));
        let sense = if button.navigation().is_some() {
            Sense::drag()
        } else {
            Sense::click()
        };
        let response = ui.interact(rect, id, sense);
        if let Some(kind) = button.navigation() {
            self.corner_drag(ui, view, button, kind, rect, &response);
        }
        let lit = match button {
            // Orange whenever one view fills the frame: maximised in its
            // group, or alone in its layout.
            Corner::Maximize => self
                .corner
                .groups
                .get(&view)
                .is_none_or(|&(in_group, maximized)| maximized || !in_group),
            Corner::Properties => self.corner.properties.view == Some(view),
            _ => self
                .corner
                .drag
                .as_ref()
                .is_some_and(|d| d.view == view && d.button == button),
        };
        let (glyph, tip) = match button {
            Corner::Pan => (Glyph::Pan, "Pan view (drag)".into()),
            Corner::Rotate => (Glyph::Rotate, "Rotate view (drag)".into()),
            Corner::Zoom => (Glyph::Zoom, "Zoom view (drag)".into()),
            Corner::Maximize if lit => (
                Glyph::Restore,
                format!("Restore viewport layout{}", self.maximize_shortcut_hint()),
            ),
            Corner::Properties => (
                Glyph::Gear,
                format!(
                    "Viewport properties{}",
                    hint(&self.bindings, &[viewport_properties::ACTION])
                ),
            ),
            Corner::Maximize => (
                Glyph::Maximize,
                format!("Maximise viewport{}", self.maximize_shortcut_hint()),
            ),
            Corner::Transparency | Corner::Toggle(_) => {
                unreachable!("drawn by transparency_button and toggle_button")
            }
        };
        paint_button(ui, rect, glyph, &response, lit, true);
        let response = response.on_hover_text(tip);
        if response.clicked() {
            match button {
                Corner::Maximize => self.request_toggle_maximized(view),
                Corner::Properties => self.toggle_view_properties(view, None),
                // VB-Q2: a plain click on a drag button does nothing.
                Corner::Pan
                | Corner::Rotate
                | Corner::Zoom
                | Corner::Transparency
                | Corner::Toggle(_) => {}
            }
        }
    }

    /// The Transparency pop-up button (HT-1; XR-1's three buttons in one,
    /// Jason's call). Its glyph is the active meshes' mode, lit when that
    /// isn't Normal (UIC-01/23), with a corner triangle for its list
    /// (UIC-25). A quick click toggles Normal and the view's last mode;
    /// holding it about 400 ms or a right-click opens the list; Alt works
    /// on the inactive meshes (UIC-27). Every change runs
    /// `view3d.TransparencyOverride` (UIC-160/165). Greyed with its reason
    /// in the Wireframe style (XR-03); the list still opens, greyed.
    fn transparency_button(&mut self, ui: &mut Ui, view: u32, rect: Rect, hits: &mut Vec<Rect>) {
        let id = ui.id().with("corner-transparency");
        let alt = ui.input(|i| i.modifiers.alt);
        let now = ui.input(|i| i.time);
        let settings = &self.display.settings;
        let mode = settings.active.transparency.mode;
        let why_active = transparency_unavailable(settings, false);
        let lit = why_active.is_none() && mode != TransparencyMode::Normal;
        let response = ui.interact(rect, id, Sense::click());
        paint_button(
            ui,
            rect,
            Glyph::Transparency(mode),
            &response,
            lit,
            why_active.is_none(),
        );
        // UIC-25: a pop-up button's corner triangle.
        let corner = rect.right_bottom() + vec2(-1.5, -1.5);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![corner, corner + vec2(-4., 0.), corner + vec2(0., -4.)],
            if why_active.is_none() {
                theme::viewport::header_text()
            } else {
                theme::disabled_text()
            },
            Stroke::NONE,
        ));
        let last = self.last_transparency(view, false);
        let mut tip = format!(
            "Transparency: {}. Click: Normal \u{2194} {}. Hold or right-click: choose. Alt: inactive meshes.{}",
            mode.label(),
            last.label(),
            hint(
                &self.bindings,
                &TRANSPARENCY_ACTIONS
                    .iter()
                    .copied()
                    .chain([TOGGLE_XRAY])
                    .collect::<Vec<_>>()
            ),
        );
        if let Some(why) = why_active {
            tip.push_str(&format!("\n{why}"));
        }
        let open_here = self
            .corner
            .transparency_list
            .as_ref()
            .is_some_and(|l| l.view == view);
        // A press on the button: a hold opens the list; a press while it
        // is open closes it.
        if response.is_pointer_button_down_on() {
            match self.corner.transparency_press {
                Some(p) if p.view == view => {
                    if !p.handled && now - p.start >= TRANSPARENCY_HOLD {
                        self.open_transparency_list(view, true);
                        if let Some(p) = &mut self.corner.transparency_press {
                            p.handled = true;
                        }
                    }
                }
                _ => {
                    if open_here {
                        self.corner.transparency_list = None;
                    }
                    self.corner.transparency_press = Some(TransparencyPress {
                        view,
                        start: now,
                        handled: open_here,
                    });
                }
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        let press = self.corner.transparency_press.filter(|p| p.view == view);
        let response = response.on_hover_text(tip);
        if response.secondary_clicked() {
            self.open_transparency_list(view, false);
        } else if response.clicked() && !press.is_some_and(|p| p.handled) {
            self.quick_transparency(view, alt);
        }
        // Hold, slide, release: letting go over an entry picks it.
        if ui.input(|i| i.pointer.primary_released())
            && press.is_some_and(|p| p.handled)
            && let Some(list) = self
                .corner
                .transparency_list
                .as_ref()
                .filter(|l| l.view == view && l.by_hold)
            && let Some(at) = ui.input(|i| i.pointer.latest_pos())
            && let Some(&(picked, _)) = list.entries.iter().find(|(_, r)| r.contains(at))
        {
            self.pick_transparency(view, picked, alt);
        }
        // Kept until the button is let go, wherever the pointer slid.
        if !ui.input(|i| i.pointer.any_down()) && press.is_some() {
            self.corner.transparency_press = None;
        }
        self.transparency_list_ui(ui, view, rect, alt, hits);
    }

    fn open_transparency_list(&mut self, view: u32, by_hold: bool) {
        self.corner.transparency_list = Some(TransparencyList {
            view,
            by_hold,
            entries: Vec::new(),
            rect: None,
        });
    }

    /// This view's last non-Normal mode for the active (or inactive)
    /// meshes; X-Ray until one is used.
    fn last_transparency(&self, view: u32, inactive: bool) -> TransparencyMode {
        self.corner
            .last_transparency
            .get(&view)
            .map_or(TransparencyMode::XRay, |l| l[usize::from(inactive)])
    }

    fn remember_transparency(&mut self, view: u32, mode: TransparencyMode, inactive: bool) {
        if mode != TransparencyMode::Normal {
            self.corner
                .last_transparency
                .entry(view)
                .or_insert([TransparencyMode::XRay; 2])[usize::from(inactive)] = mode;
        }
    }

    /// A quick click (HT-1): from Ghost or X-Ray to Normal, from Normal to
    /// the view's last mode.
    fn quick_transparency(&mut self, view: u32, inactive: bool) {
        if let Some(why) = transparency_unavailable(&self.display.settings, inactive) {
            self.status = why.into();
            return;
        }
        let group = if inactive {
            &self.display.settings.inactive
        } else {
            &self.display.settings.active
        };
        let current = group.transparency.mode;
        let next = if current == TransparencyMode::Normal {
            self.last_transparency(view, inactive)
        } else {
            self.remember_transparency(view, current, inactive);
            TransparencyMode::Normal
        };
        self.set_transparency(next, inactive);
    }

    /// An entry picked in the list: sets that mode and closes it.
    fn pick_transparency(&mut self, view: u32, mode: TransparencyMode, inactive: bool) {
        self.corner.transparency_list = None;
        if let Some(why) = transparency_unavailable(&self.display.settings, inactive) {
            self.status = why.into();
            return;
        }
        self.remember_transparency(view, mode, inactive);
        self.set_transparency(mode, inactive);
    }

    /// The Transparency list under its button (HT-1): Normal, Ghost and
    /// X-Ray with their glyphs, the active meshes' mode marked (UIC-01) and
    /// the inactive meshes' with a quieter outline. Alt while picking sets
    /// the inactive meshes. A press elsewhere or Esc closes it unchanged.
    fn transparency_list_ui(
        &mut self,
        ui: &mut Ui,
        view: u32,
        button: Rect,
        alt: bool,
        hits: &mut Vec<Rect>,
    ) {
        let Some(list) = self
            .corner
            .transparency_list
            .clone()
            .filter(|l| l.view == view)
        else {
            return;
        };
        let ctx = ui.ctx().clone();
        let escape = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let pressed_away = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer.press_origin().is_some_and(|at| {
                    !button.contains(at) && !list.rect.is_some_and(|r| r.contains(at))
                })
        });
        if escape || pressed_away {
            self.corner.transparency_list = None;
            return;
        }
        let settings = &self.display.settings;
        let active = settings.active.transparency.mode;
        let inactive = settings.inactive.transparency.mode;
        let why = transparency_unavailable(settings, alt);
        let mut entries = Vec::new();
        let mut picked = None;
        let area = egui::Area::new(Id::new(("transparency-list", view)))
            .order(egui::Order::Foreground)
            .fixed_pos(button.left_bottom() + vec2(-1., 4.))
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    for mode in TransparencyMode::ALL {
                        let (r, response) = ui.allocate_exact_size(LIST_ENTRY, Sense::click());
                        entries.push((mode, r));
                        let enabled = why.is_none();
                        let hovered = enabled && response.hovered();
                        let painter = ui.painter();
                        if hovered {
                            painter.rect_filled(r, 2., theme::hover());
                        }
                        let colour = if enabled {
                            theme::text()
                        } else {
                            theme::disabled_text()
                        };
                        // The marks' column: the active meshes' mode
                        // gets a check in the accent colour, the inactive
                        // meshes' a quieter ring.
                        let mark = pos2(r.left() + 8., r.center().y);
                        if mode == active {
                            let on = if enabled {
                                theme::viewport::header_on()
                            } else {
                                theme::disabled_text()
                            };
                            let s = Stroke::new(1.6_f32, on);
                            painter.line_segment([mark + vec2(-3.5, 0.), mark + vec2(-1., 2.5)], s);
                            painter.line_segment([mark + vec2(-1., 2.5), mark + vec2(3.5, -3.)], s);
                        } else if mode == inactive {
                            painter.circle_stroke(mark, 3., Stroke::new(1_f32, theme::weak_text()));
                        }
                        paint_glyph(
                            painter,
                            pos2(r.left() + 24., r.center().y),
                            Glyph::Transparency(mode),
                            colour,
                        );
                        painter.text(
                            pos2(r.left() + 36., r.center().y),
                            Align2::LEFT_CENTER,
                            mode.label(),
                            egui::FontId::proportional(12.),
                            colour,
                        );
                        let mut tip = format!("{}: {}", mode.label(), transparency_tip(mode));
                        match (mode == active, mode == inactive) {
                            (true, true) => tip.push_str("\nNow: active and inactive meshes"),
                            (true, false) => tip.push_str("\nNow: active meshes"),
                            (false, true) => tip.push_str("\nNow: inactive meshes"),
                            (false, false) => {}
                        }
                        if let Some(why) = why {
                            tip = why.to_owned();
                        }
                        let response = response.on_hover_text(tip);
                        if response.clicked() && enabled {
                            picked = Some(mode);
                        }
                    }
                });
            });
        hits.push(area.response.rect);
        if let Some(l) = &mut self.corner.transparency_list {
            l.entries = entries;
            l.rect = Some(area.response.rect);
        }
        if let Some(mode) = picked {
            self.pick_transparency(view, mode, alt);
        }
    }

    /// A view toggle (HT-3): lit from its command's `?` answer (UIC-223,
    /// UIC-134); a click runs `<command> true|false` (UIC-165), the same
    /// setting as the Viewport Properties row.
    fn toggle_button(&mut self, ui: &mut Ui, toggle: ViewToggle, rect: Rect) {
        let id = ui.id().with(("corner-toggle", toggle));
        let command = toggle.command();
        let on = self.line_state(command).is_some_and(|s| s.on);
        let why = toggle.unavailable(&self.display.settings);
        let response = ui.interact(rect, id, Sense::click());
        paint_button(ui, rect, toggle.glyph(), &response, on, why.is_none());
        // HT-5: without a selected weight map the button still toggles.
        let no_map = toggle == ViewToggle::Weights && self.editor.selected_weight_map.is_none();
        let tip = match why {
            Some(why) => format!("{}\n{why}", toggle.tip()),
            None => format!(
                "{}{}{}",
                toggle.tip(),
                if no_map {
                    " (no weight map selected)"
                } else {
                    ""
                },
                hint(&self.bindings, &[toggle.label(), command])
            ),
        };
        if response.on_hover_text(tip).clicked() && why.is_none() {
            self.execute(&format!("{command} {}", !on));
        }
    }

    /// Set this view's active (or inactive) meshes' Transparency Mode
    /// through its command.
    pub(super) fn set_transparency(&mut self, mode: TransparencyMode, inactive: bool) {
        let line = if inactive {
            format!("view3d.TransparencyOverride {} bgmesh:1", mode.label())
        } else {
            format!("view3d.TransparencyOverride {}", mode.label())
        };
        self.execute(&line);
        self.status = format!(
            "{} meshes: {}",
            if inactive { "Inactive" } else { "Active" },
            mode.label()
        );
    }

    /// `operandi.view3d.toggleXRay` (XR-04): this view's active meshes
    /// between Normal and X-Ray. From Ghost it goes to X-Ray.
    pub(super) fn toggle_xray(&mut self) {
        if let Some(why) = transparency_unavailable(&self.display.settings, false) {
            self.status = why.into();
            return;
        }
        let next = if self.display.settings.active.transparency.mode == TransparencyMode::XRay {
            TransparencyMode::Normal
        } else {
            TransparencyMode::XRay
        };
        self.set_transparency(next, false);
    }

    /// Pan, Rotate or Zoom: press on the icon and drag (any mouse button,
    /// modifiers ignored, VB-Q3). The view changes live through the same
    /// code as the in-view gesture, so Rotate and Zoom match its rate; Pan
    /// moves the view with the drag at `PAN_BUTTON_RATE` (the scene slides
    /// the other way) unless Reverse Pan Widget Direction is on. The pointer
    /// stays hidden and held in place (locked on macOS, put back after
    /// every move elsewhere), so the drag never runs out of screen.
    /// Never an undo step.
    fn corner_drag(
        &mut self,
        ui: &mut Ui,
        view: u32,
        button: Corner,
        kind: input::NavigationKind,
        rect: Rect,
        response: &Response,
    ) {
        if self.view_passive {
            return;
        }
        if response.drag_started() {
            self.corner.drag = Some(HeaderDrag {
                view,
                button,
                before: self.camera.clone(),
                // The pointer goes back where it was pressed, not the
                // icon's centre, so the release doesn't jump it either.
                home: ui
                    .input(|i| i.pointer.press_origin())
                    .unwrap_or(rect.center()),
                moved: false,
                raw: false,
                locked: false,
            });
        }
        let ours = |d: &HeaderDrag| d.view == view && d.button == button;
        if !self.corner.drag.as_ref().is_some_and(ours) {
            return;
        }
        // Esc: the view goes back to how it was at the press, and the drag
        // ends (egui drops the drag on Esc too).
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if let Some(drag) = self.corner.drag.take() {
                self.camera = drag.before.clone();
                self.status = "View change cancelled".into();
                end_corner_drag(ui, &drag);
            }
            return;
        }
        if response.dragged() {
            let r = self
                .view_rects
                .get(&view)
                .copied()
                .unwrap_or(self.viewport_rect);
            let home = self.corner.drag.as_ref().map_or(rect.center(), |d| d.home);
            let tablet = self.input_preferences.options.tablet_navigation();
            let lock = !tablet && self.corner.lock_pointer.unwrap_or(LOCK_POINTER);
            // The pointer's latest place this frame.
            let latest = ui.input(|i| {
                i.events.iter().rev().find_map(|e| match e {
                    Event::PointerMoved(p) => Some(*p),
                    _ => None,
                })
            });
            let raw = raw_motion(ui);
            let first = response.drag_started();
            if !first
                && !tablet
                && raw.is_some()
                && let Some(drag) = self.corner.drag.as_mut()
            {
                drag.raw = true;
            }
            let use_raw =
                !first && !tablet && self.corner.drag.as_ref().is_some_and(|d| d.raw || d.locked);
            // VBF-1: a pointer put back on the icon after each move lands
            // late, so the next place read can be measured from before it
            // (part of a move counts twice or not at all, and wobble turns
            // into drift on the other axis). On macOS the mouse's own
            // movement after a put-back carries the put-back too, so the
            // view shook back to the start (VBF-2). So on macOS the pointer
            // is locked in place for the drag (no put-backs) and the drag
            // reads the mouse's own movement; elsewhere the pointer is put
            // back and the mouse's own movement, which doesn't see it there,
            // is read once the platform sends it. The first frame measures
            // from the press (no jump by the press's distance from the
            // icon's centre).
            let moved = if use_raw {
                raw.unwrap_or(Vec2::ZERO)
            } else {
                latest.map_or(Vec2::ZERO, |p| p - home)
            };
            if lock
                && let Some(drag) = self.corner.drag.as_mut()
                && !drag.locked
            {
                drag.locked = true;
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
            }
            if moved != Vec2::ZERO {
                let delta = if kind == input::NavigationKind::Pan {
                    // navigate's Pan is the in-view grab (the scene follows
                    // the pointer); the button moves the view instead.
                    let grab = self.input_preferences.options.reverse_pan_widget;
                    moved
                        * if grab {
                            PAN_BUTTON_RATE
                        } else {
                            -PAN_BUTTON_RATE
                        }
                } else {
                    moved
                };
                if let Some(drag) = self.corner.drag.as_mut() {
                    drag.moved = true;
                }
                self.navigate(kind, delta, r);
                // Input Device Mouse: the pointer goes back to the button
                // so the drag never meets the screen edge (a locked pointer
                // needs no put-back). A tablet's pen can't be moved, so its
                // drag stops at the edge instead.
                if tablet {
                    if let (Some(drag), Some(p)) = (self.corner.drag.as_mut(), latest) {
                        drag.home = p;
                    }
                } else if !lock && latest.is_some_and(|p| p != home) {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::CursorPosition(home));
                }
            }
            // The pointer hides while dragging (a tablet keeps it).
            if !tablet {
                ui.ctx().set_cursor_icon(CursorIcon::None);
            }
        } else if let Some(drag) = self.corner.drag.take() {
            // Released: the pointer comes back where it was pressed.
            end_corner_drag(ui, &drag);
        }
    }

    /// " (0)": the Maximise shortcut as the tooltip shows it.
    fn maximize_shortcut_hint(&self) -> String {
        hint(&self.bindings, &[MAXIMIZE, MAXIMIZE_ALT])
    }

    /// Note each view's group before the views draw.
    pub(super) fn corner_groups(&mut self) {
        self.corner.groups = self
            .views
            .viewports()
            .into_iter()
            .map(|v| (v, (self.views.in_group(v), self.views.is_maximized(v))))
            .collect();
    }

    /// After Maximise or Restore changes a view's size, keep its world
    /// scale (v1.1): the same units per point as before, so geometry stays
    /// the same size and the view just shows more or less around it. The
    /// projection scales with the view's height times its lens over its
    /// distance, so the distance takes the change: in perspective that
    /// keeps the size at the orbit centre and leaves the field of view
    /// alone. (Scaling the lens instead widened a Perspective view's field
    /// of view each time it was maximised, to about 95° from half the
    /// height, and the layout saved the wide lens.) Camera and Light view
    /// keep their item's placement.
    pub(super) fn keep_world_scale(&mut self, r: Rect) {
        let Some(before) = self.corner.rescale.remove(&self.drawing_view) else {
            return;
        };
        if !self.camera.looks_through() && before > 1. && r.height() > 1. {
            let factor = f64::from(r.height()) / f64::from(before);
            self.camera.distance = (self.camera.distance * factor)
                .min(crate::camera::MAX_DISTANCE.max(self.camera.distance));
        }
    }

    /// View > Tear Off View: the dot menu's Detach (the view into its own
    /// window, at its size, near where it was).
    pub(super) fn tear_off_view(&mut self, view: u32) {
        self.dot_menu_action(panel_dot::Dot::View(view), panel_dot::MenuAction::Detach);
    }

    /// Maximise or restore `view` once the views have drawn.
    pub(super) fn request_toggle_maximized(&mut self, view: u32) {
        self.corner.requests.push(Request::ToggleMaximize(view));
    }

    /// Run what the corner buttons asked for while the views drew.
    pub(super) fn corner_requests(&mut self) {
        for request in std::mem::take(&mut self.corner.requests) {
            match request {
                Request::ToggleMaximize(view) => self.toggle_maximized(view),
            }
        }
    }

    /// Maximise `view` in its group, or restore the group; a view alone in
    /// its layout becomes a quad instead. The view becomes the active one.
    /// Never an undo step: it is layout, not scene.
    pub(super) fn toggle_maximized(&mut self, view: u32) {
        if !self.views.in_group(view) {
            self.make_quad(view);
            return;
        }
        match self.views.toggle_maximized(view) {
            Ok(maximized) => {
                self.activate_view(view);
                if let Some(rect) = self.view_rects.get(&view) {
                    self.corner.rescale.insert(view, rect.height());
                }
                self.status = if maximized {
                    "Viewport maximised".into()
                } else {
                    "Viewport layout restored".into()
                };
            }
            Err(error) => self.status = error,
        }
    }

    /// A view alone in its layout becomes a 2 x 2 quad: Top, Perspective /
    /// Front, Right. The view keeps everything and takes the slot that
    /// matches its type (Perspective's for Perspective, free ortho, Camera
    /// and the other axis views); the other three are new default views.
    pub(super) fn make_quad(&mut self, view: u32) {
        self.activate_view(view);
        let slot = if self.camera.looks_through() {
            1
        } else {
            match self.camera.view {
                View::Top => 0,
                View::Front => 2,
                View::Right => 3,
                _ => 1,
            }
        };
        match self.views.make_quad(view, slot) {
            Ok(ids) => {
                let types = [View::Top, View::Perspective, View::Front, View::Right];
                for (index, (&id, kind)) in ids.iter().zip(types).enumerate() {
                    if index == slot {
                        continue;
                    }
                    self.cameras.insert(
                        id,
                        Camera {
                            view: kind,
                            ..Default::default()
                        },
                    );
                    self.displays.insert(id, Default::default());
                }
                // The view gets smaller; keep its world scale.
                if let Some(rect) = self.view_rects.get(&view) {
                    self.corner.rescale.insert(view, rect.height());
                }
                self.status = "Viewport layout: quad".into();
            }
            Err(error) => self.status = error,
        }
    }

    /// The pointer in main-window coordinates. The main window's own
    /// pointer when it has the keyboard; otherwise (a floating panel, a
    /// torn-off view or the Preview has it, and the main window hears no
    /// pointer moves) the system's mouse position.
    pub(super) fn main_window_pointer(&self, ctx: &Context) -> Option<Pos2> {
        let root_focused = ctx.viewport_id() == egui::ViewportId::ROOT
            && ctx.input(|i| i.viewport().focused != Some(false));
        if root_focused && let Some(p) = ctx.pointer_hover_pos() {
            return Some(p);
        }
        let screen = crate::native::mouse_location()?;
        let (inner, native, ppp) = self.corner.root_window?;
        Some(screen_to_window(screen, inner, native, ppp))
    }

    /// Keep the main window's place for `main_window_pointer` (call with
    /// the main window's context).
    pub(super) fn note_root_window(&mut self, ctx: &Context) {
        // Read pixels per point outside `input`: calling the context from
        // inside its own lock deadlocks the first frame.
        let ppp = ctx.pixels_per_point();
        self.corner.root_window = ctx.input(|i| {
            let inner = i.viewport().inner_rect?;
            Some((
                inner.min,
                i.viewport().native_pixels_per_point.unwrap_or(1.),
                ppp,
            ))
        });
    }

    /// The main-window view under the pointer, else the active view.
    pub(super) fn view_under_pointer(&self, ctx: &Context) -> u32 {
        self.main_window_pointer(ctx)
            .and_then(|p| {
                self.corner
                    .areas
                    .iter()
                    .find(|(_, area)| area.contains(p))
                    .map(|(&view, _)| view)
            })
            .unwrap_or(self.active_view)
    }

    /// Whether the pointer is over any main-window view (the gate for
    /// the corner buttons' shortcuts).
    pub(super) fn pointer_over_a_view(&self, ctx: &Context) -> bool {
        self.main_window_pointer(ctx)
            .is_some_and(|p| self.corner.areas.values().any(|a| a.contains(p)))
    }
}

/// A screen position (system points) in a window's egui points, given the
/// window's inner top-left in egui points and its native and egui pixels
/// per point (they differ when the UI is zoomed).
pub(crate) fn screen_to_window(screen: [f32; 2], inner: Pos2, native: f32, ppp: f32) -> Pos2 {
    Pos2::new(
        (screen[0] * native - inner.x * ppp) / ppp,
        (screen[1] * native - inner.y * ppp) / ppp,
    )
}

/// The floating header's widget look: each button the view's background
/// colour (nearly opaque, so it reads over geometry) with a thin border,
/// white text, a little shorter than the panels' buttons.
fn header_style(ui: &mut Ui) {
    let backing = theme::viewport::header_button();
    let edge = Stroke::new(1_f32, theme::viewport::header_border());
    let style = ui.style_mut();
    style.spacing.interact_size.y = HEADER_HEIGHT;
    style.spacing.button_padding.y = 1.;
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.noninteractive,
    ] {
        widget.bg_fill = backing;
        widget.weak_bg_fill = backing;
        widget.bg_stroke = edge;
        widget.fg_stroke.color = theme::viewport::header_text();
    }
    for widget in [
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.bg_stroke = edge;
    }
    style.visuals.override_text_color = Some(theme::viewport::header_text());
}

/// Input-map actions (input.rs ACTIONS). Two keys toggle Maximise: numpad 0,
/// and Alt+0 for keyboards without a keypad.
pub const MAXIMIZE: &str = "Maximise Viewport";
pub const MAXIMIZE_ALT: &str = "Maximise Viewport (Alternate)";

/// Whether an action is one of the Maximise keys.
pub fn is_maximize(action: &str) -> bool {
    matches!(action, MAXIMIZE | MAXIMIZE_ALT)
}

/// Whether an action only runs with the pointer over a view (it has no
/// sensible target otherwise).
pub fn pointer_view_action(action: &str) -> bool {
    matches!(
        action,
        MAXIMIZE | MAXIMIZE_ALT | viewport_properties::ACTION
    )
}

/// Whether an action changes one view (its camera, display or layout).
/// Pressed with the pointer over a view, it acts on that view, which
/// becomes the active one: no click first (UIC-70, Jason 2026-11-08). With
/// the pointer elsewhere the ones not gated by `pointer_view_action` or
/// the " View" rule act on the active view, as before.
/// Viewport Properties is the exception: its popover edits the hovered
/// view's own settings and leaves the active view alone.
pub fn hovered_view_action(action: &str) -> bool {
    matches!(action, MAXIMIZE | MAXIMIZE_ALT)
        || action.ends_with(" View")
        || matches!(
            action,
            "Frame All"
                | "Frame Selection"
                | "Zoom In"
                | "Zoom Out"
                | crate::input::RECENTRE
                | "Turn"
                | "Turn Upright"
                | "Align Selected"
                | "Fit and Align Selected"
                | "Wireframe Overlay None"
                | "Wireframe Overlay Uniform"
                | "Wireframe Overlay Colored"
        )
        || transparency_for_action(action).is_some()
        || action.eq_ignore_ascii_case(TOGGLE_XRAY)
}

/// Pies whose entries change one view: opened by key over a view, they
/// act on that view.
pub fn hovered_view_pie(pie: &str) -> bool {
    matches!(pie, crate::pie_menu::VIEW | crate::pie_menu::SHADING)
}

/// The Input Map actions that set a Transparency Mode (XR-2): the active
/// meshes', or with `inactive` the inactive meshes'.
pub const TRANSPARENCY_ACTIONS: [&str; 6] = [
    "Transparency Normal",
    "Transparency Ghost",
    "Transparency X-Ray",
    "Inactive Transparency Normal",
    "Inactive Transparency Ghost",
    "Inactive Transparency X-Ray",
];
/// XR-2: the one-key toggle between Normal and X-Ray (no default key, Q1).
pub const TOGGLE_XRAY: &str = "Toggle X-Ray";

/// The mode and group an action in `TRANSPARENCY_ACTIONS` sets.
pub fn transparency_for_action(action: &str) -> Option<(TransparencyMode, bool)> {
    let i = TRANSPARENCY_ACTIONS
        .iter()
        .position(|a| a.eq_ignore_ascii_case(action.trim()))?;
    Some((TransparencyMode::ALL[i % 3], i >= 3))
}

/// What a mode does, for its button's tooltip (UIC-04).
fn transparency_tip(mode: TransparencyMode) -> &'static str {
    match mode {
        TransparencyMode::Normal => "active meshes drawn solid, as their style sets",
        TransparencyMode::Ghost => {
            "active meshes' polygons facing you see-through and tinted, those facing away solid"
        }
        TransparencyMode::XRay => {
            "active meshes' polygons facing you clear, only those facing away show"
        }
    }
}

/// Why a group can't be see-through in this view, if it can't (XR-1,
/// VP-80): the Wireframe style; for the inactive group also Make Inactive
/// Same as Active (it follows the active group) or Make Inactive Invisible.
pub fn transparency_unavailable(
    settings: &view_display::Settings,
    inactive: bool,
) -> Option<&'static str> {
    if !inactive {
        return (settings.active.style == view_display::Style::Wireframe)
            .then_some("Not with the Wireframe style");
    }
    if settings.drawing.inactive_same_as_active {
        Some("Inactive meshes draw like the active ones (Make Inactive Same as Active)")
    } else if settings.drawing.inactive_invisible {
        Some("Inactive meshes are hidden in this view (Make Inactive Invisible)")
    } else if settings.inactive.style == view_display::Style::Wireframe {
        Some("Not with the inactive meshes' Wireframe style")
    } else {
        None
    }
}

/// " (Numpad0, Alt+0)": an action's keys from the input map, for a tooltip.
fn hint(bindings: &BTreeMap<String, String>, actions: &[&str]) -> String {
    let keys: Vec<String> = actions
        .iter()
        .filter_map(|a| bindings.get(*a).filter(|b| !b.is_empty()).cloned())
        .collect();
    if keys.is_empty() {
        String::new()
    } else {
        format!(" ({})", keys.join(", "))
    }
}

#[derive(Clone, Copy)]
enum Glyph {
    Pan,
    Rotate,
    Zoom,
    Maximize,
    Restore,
    Gear,
    Transparency(TransparencyMode),
    Weights,
    Distortion,
    Locators,
    InactiveInvisible,
    SameAsActive,
}

/// One header button's slot at the cursor: the button centred in it.
fn header_slot(ui: &mut Ui) -> Rect {
    let (slot, _) = ui.allocate_exact_size(vec2(BUTTON + GAP, BUTTON), Sense::hover());
    Rect::from_center_size(slot.center(), Vec2::splat(BUTTON))
}

/// A corner button: monochrome glyph in the header's icon colour,
/// brighter on hover, the accent colour when lit (maximised, dragging,
/// popover open). The glyphs are drawn shapes, provisional until icons
/// come from the generator (tools/operandi-icons).
/// `enabled` false: greyed (UIC-117), no hover brightening.
fn paint_button(ui: &Ui, rect: Rect, glyph: Glyph, response: &Response, lit: bool, enabled: bool) {
    let painter = ui.painter();
    let backing = rect.expand(1.);
    let fill = if enabled && response.is_pointer_button_down_on() {
        theme::pressed()
    } else if enabled && response.hovered() {
        theme::hover()
    } else {
        theme::viewport::header_button()
    };
    painter.rect(
        backing,
        3.,
        fill,
        Stroke::new(1_f32, theme::viewport::header_border()),
        StrokeKind::Inside,
    );
    let color = if !enabled {
        theme::disabled_text()
    } else if lit {
        theme::viewport::header_on()
    } else if response.hovered() || response.dragged() {
        theme::text()
    } else {
        theme::viewport::header_text()
    };
    paint_glyph(painter, rect.center(), glyph, color);
}

/// A header glyph centred on `c` in `color` (also the Transparency list's
/// entries).
fn paint_glyph(painter: &Painter, c: Pos2, glyph: Glyph, color: Color32) {
    let stroke = Stroke::new(1.3_f32, color);
    match glyph {
        Glyph::Pan => {
            // Four arrows out from the centre.
            for dir in [vec2(1., 0.), vec2(-1., 0.), vec2(0., 1.), vec2(0., -1.)] {
                let tip = c + dir * 6.;
                let side = vec2(-dir.y, dir.x) * 2.;
                painter.line_segment([c, tip], stroke);
                painter.line_segment([tip, tip - dir * 2. + side], stroke);
                painter.line_segment([tip, tip - dir * 2. - side], stroke);
            }
        }
        Glyph::Rotate => {
            // A clockwise arrow round a ball: orbit the view. Open at the
            // top with the head pointing along the arc, so it can't read
            // as a letter G (the old drawing did). Open head like Pan's.
            let r = 5.6;
            let (t0, t1) = ((-55_f32).to_radians(), 230_f32.to_radians());
            let points: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let t = t0 + (t1 - t0) * i as f32 / 24.;
                    c + vec2(t.cos(), t.sin()) * r
                })
                .collect();
            painter.circle_filled(c, 2., color);
            let end = points[24];
            let dir = (end - points[22]).normalized();
            painter.add(egui::Shape::line(points, stroke));
            let tip = end + dir * 0.4;
            let side = vec2(-dir.y, dir.x) * 2.4;
            painter.line_segment([tip, tip - dir * 2.6 + side], stroke);
            painter.line_segment([tip, tip - dir * 2.6 - side], stroke);
        }
        Glyph::Zoom => {
            // A magnifier.
            let lens = c + vec2(-1.5, -1.5);
            painter.circle_stroke(lens, 4., stroke);
            painter.line_segment([lens + vec2(2.9, 2.9), c + vec2(5.5, 5.5)], stroke);
        }
        Glyph::Gear => {
            // Eight teeth round a ring.
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let dir = vec2(a.cos(), a.sin());
                painter.line_segment([c + dir * 4., c + dir * 6.5], Stroke::new(2.2_f32, color));
            }
            painter.circle_stroke(c, 4., stroke);
            painter.circle_stroke(c, 1.5, stroke);
        }
        Glyph::Maximize => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(10.)),
                1.,
                stroke,
                StrokeKind::Middle,
            );
            painter.line_segment([c + vec2(-5., -3.5), c + vec2(5., -3.5)], stroke);
        }
        // XR-1, drawn shapes (provisional like the others): a ball seen
        // three ways. Normal: solid. Ghost: a faint fill you can half see
        // through. X-Ray: an empty outline with a small cube inside.
        Glyph::Transparency(TransparencyMode::Normal) => {
            painter.circle_filled(c, 5.5, color);
        }
        // Ghost and X-Ray's rings sit inside Normal's 5.5 disc, so all
        // three are the same size (Jason, 2026-11-16).
        Glyph::Transparency(TransparencyMode::Ghost) => {
            let ring = 5.5 - stroke.width / 2.;
            painter.circle_filled(c, ring, color.gamma_multiply(0.35));
            painter.circle_stroke(c, ring, stroke);
            painter.circle_filled(c + vec2(-2., -2.), 1.2, color);
        }
        Glyph::Transparency(TransparencyMode::XRay) => {
            painter.circle_stroke(c, 5.5 - stroke.width / 2., stroke);
            painter.rect_filled(Rect::from_center_size(c, Vec2::splat(4.)), 0.5, color);
        }
        // HT-5 Weightmaps: a small square with a soft gradient from its
        // top left to its bottom right.
        Glyph::Weights => {
            let r = Rect::from_center_size(c, Vec2::splat(10.));
            let mut mesh = egui::Mesh::default();
            let faint = color.gamma_multiply(0.15);
            for (p, col) in [
                (r.left_top(), color),
                (r.right_top(), color.gamma_multiply(0.55)),
                (r.right_bottom(), faint),
                (r.left_bottom(), color.gamma_multiply(0.55)),
            ] {
                mesh.colored_vertex(p, col);
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(mesh);
            painter.rect_stroke(r, 1., stroke, StrokeKind::Middle);
        }
        // HT-6 UV Distortion: three short wavy vertical strokes.
        Glyph::Distortion => {
            for dx in [-3.5, 0., 3.5] {
                let points: Vec<Pos2> = (0..=8)
                    .map(|i| {
                        let t = i as f32 / 8.;
                        c + vec2(dx + 1.2 * (t * std::f32::consts::TAU).sin(), -5. + 10. * t)
                    })
                    .collect();
                painter.add(egui::Shape::line(points, stroke));
            }
        }
        // HT-3, drawn shapes like the others. Locators: a small axis
        // tripod, three short strokes from one point.
        Glyph::Locators => {
            let o = c + vec2(0., 1.);
            painter.line_segment([o, o + vec2(0., -6.5)], stroke);
            painter.line_segment([o, o + vec2(-5.5, 3.5)], stroke);
            painter.line_segment([o, o + vec2(5.5, 3.5)], stroke);
        }
        // Make Inactive Invisible: one dashed-outline box.
        Glyph::InactiveInvisible => {
            let r = Rect::from_center_size(c, Vec2::splat(10.));
            let corners = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            painter.extend(egui::Shape::dashed_line(&corners, stroke, 2., 1.6));
        }
        // Make Inactive Same As Active: two solid boxes side by side.
        Glyph::SameAsActive => {
            for dx in [-3.2, 3.2] {
                painter.rect_filled(
                    Rect::from_center_size(c + vec2(dx, 0.), vec2(5., 8.)),
                    0.8,
                    color,
                );
            }
        }
        Glyph::Restore => {
            let back = Rect::from_min_size(c + vec2(-3., -6.), Vec2::splat(8.));
            let front = Rect::from_min_size(c + vec2(-6., -2.), Vec2::splat(8.));
            painter.line_segment([back.left_top(), back.right_top()], stroke);
            painter.line_segment([back.right_top(), back.right_bottom()], stroke);
            painter.rect_stroke(front, 1., stroke, StrokeKind::Middle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw, key, pointer};
    use operandi_layout::Pane;

    const SIZE: Vec2 = vec2(1440., 900.);

    fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
        draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
        draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
        draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
        draw(a, ctx, SIZE, vec![]);
    }

    fn click_held(a: &mut Operandi, ctx: &Context, at: Pos2, held: Modifiers) {
        crate::tests::draw_held(a, ctx, SIZE, vec![Event::PointerMoved(at)], held);
        crate::tests::draw_held(a, ctx, SIZE, vec![pointer(at, true, held)], held);
        crate::tests::draw_held(a, ctx, SIZE, vec![pointer(at, false, held)], held);
        draw(a, ctx, SIZE, vec![]);
    }

    fn transparency_rect(a: &Operandi, view: u32) -> Rect {
        a.corner.rects[&(view, Corner::Transparency)]
    }

    fn history_lines(a: &Operandi) -> Vec<String> {
        a.history
            .log
            .records
            .iter()
            .map(|r| r.line.clone())
            .collect()
    }

    fn toggle_rect(a: &Operandi, view: u32, toggle: ViewToggle) -> Rect {
        a.corner.rects[&(view, Corner::Toggle(toggle))]
    }

    fn active_mode(a: &Operandi) -> TransparencyMode {
        a.display.settings.active.transparency.mode
    }

    fn inactive_mode(a: &Operandi) -> TransparencyMode {
        a.display.settings.inactive.transparency.mode
    }

    /// Press at `at` and hold it (no movement) past the hold time.
    fn hold(a: &mut Operandi, ctx: &Context, at: Pos2, held: Modifiers) {
        crate::tests::draw_held(a, ctx, SIZE, vec![Event::PointerMoved(at)], held);
        crate::tests::draw_held(a, ctx, SIZE, vec![pointer(at, true, held)], held);
        for _ in 0..30 {
            crate::tests::draw_held(a, ctx, SIZE, vec![], held);
        }
    }

    fn entry(a: &Operandi, mode: TransparencyMode) -> Rect {
        let list = a
            .corner
            .transparency_list
            .as_ref()
            .expect("the list is open");
        list.entries.iter().find(|(m, _)| *m == mode).unwrap().1
    }

    #[test]
    fn ht01_one_transparency_button_a_separator_then_three_toggles() {
        let (a, _ctx) = one_view();
        let rects: Vec<Rect> = [transparency_rect(&a, 0)]
            .into_iter()
            .chain(ViewToggle::ALL.iter().map(|&t| toggle_rect(&a, 0, t)))
            .chain([a.corner.rects[&(0, Corner::Pan)]])
            .collect();
        assert!(rects.windows(2).all(|w| w[0].right() <= w[1].left()));
        // HT-2: the separator's gap is wider than the toggles' own.
        let separator = rects[1].left() - rects[0].right();
        let between = rects[2].left() - rects[1].right();
        assert!(separator >= 2. * SEPARATOR_GAP, "{separator}");
        assert!((between - GAP).abs() < 0.5, "{between}");
        // N17: at the left, beside the view and Display menus.
        assert!(rects[3].right() < SIZE.x / 2., "{rects:?}");
        // Fresh view: Normal, unlit; Locators on, the other two off.
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
        assert!(a.line_state("view3d.showLocators").unwrap().on);
        assert!(!a.line_state("view3d.inactiveInvisible").unwrap().on);
        assert!(!a.line_state("view3d.sameAsActive").unwrap().on);
    }

    #[test]
    fn ht02_quick_click_toggles_normal_and_x_ray() {
        // HT-02, and XR-01's X-Ray on the active meshes only.
        let (mut a, ctx) = one_view();
        a.display.settings.active.style = view_display::Style::Advanced;
        draw(&mut a, &ctx, SIZE, vec![]);
        let at = transparency_rect(&a, 0).center();
        click(&mut a, &ctx, at);
        assert_eq!(active_mode(&a), TransparencyMode::XRay);
        assert_eq!(inactive_mode(&a), TransparencyMode::Normal);
        assert!(
            a.corner.transparency_list.is_none(),
            "a click opens no list"
        );
        // Past a double-click's time, so the second is its own click.
        for _ in 0..30 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        click(&mut a, &ctx, at);
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
    }

    #[test]
    fn ht03_hold_opens_the_list_and_the_click_remembers_the_pick() {
        let (mut a, ctx) = one_view();
        let at = transparency_rect(&a, 0).center();
        hold(&mut a, &ctx, at, Modifiers::NONE);
        assert!(
            a.corner.transparency_list.is_some(),
            "a hold opens the list"
        );
        // Letting go on the button keeps it open, changes nothing.
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(at, false, Modifiers::NONE)],
        );
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.corner.transparency_list.is_some());
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
        let ghost = entry(&a, TransparencyMode::Ghost).center();
        click(&mut a, &ctx, ghost);
        assert_eq!(active_mode(&a), TransparencyMode::Ghost);
        assert!(a.corner.transparency_list.is_none(), "a pick closes it");
        for _ in 0..30 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        click(&mut a, &ctx, at);
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
        for _ in 0..30 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        click(&mut a, &ctx, at);
        assert_eq!(active_mode(&a), TransparencyMode::Ghost, "it remembers");
    }

    #[test]
    fn ht03_hold_slide_release_picks() {
        let (mut a, ctx) = one_view();
        let at = transparency_rect(&a, 0).center();
        hold(&mut a, &ctx, at, Modifiers::NONE);
        let xray = entry(&a, TransparencyMode::XRay).center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(xray)]);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(xray, false, Modifiers::NONE)],
        );
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(active_mode(&a), TransparencyMode::XRay);
        assert!(a.corner.transparency_list.is_none());
    }

    #[test]
    fn ht04_right_click_opens_the_list_and_away_or_esc_closes_it() {
        let (mut a, ctx) = one_view();
        let at = transparency_rect(&a, 0).center();
        let right = |at: Pos2, pressed: bool| Event::PointerButton {
            pos: at,
            pressed,
            modifiers: Modifiers::NONE,
            button: PointerButton::Secondary,
        };
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
        draw(&mut a, &ctx, SIZE, vec![right(at, true)]);
        draw(&mut a, &ctx, SIZE, vec![right(at, false)]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.corner.transparency_list.is_some());
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
        draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
        assert!(a.corner.transparency_list.is_none(), "Esc closes it");
        draw(&mut a, &ctx, SIZE, vec![right(at, true)]);
        draw(&mut a, &ctx, SIZE, vec![right(at, false)]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.corner.transparency_list.is_some());
        let away = pos2(SIZE.x / 2., SIZE.y / 2.);
        click(&mut a, &ctx, away);
        assert!(
            a.corner.transparency_list.is_none(),
            "a press away closes it"
        );
        assert_eq!(active_mode(&a), TransparencyMode::Normal, "no change");
    }

    #[test]
    fn ht05_alt_works_on_the_inactive_meshes() {
        // XR-02 / HT-05.
        let (mut a, ctx) = one_view();
        let at = transparency_rect(&a, 0).center();
        hold(&mut a, &ctx, at, Modifiers::ALT);
        crate::tests::draw_held(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(at, false, Modifiers::ALT)],
            Modifiers::ALT,
        );
        let xray = entry(&a, TransparencyMode::XRay).center();
        click_held(&mut a, &ctx, xray, Modifiers::ALT);
        assert_eq!(inactive_mode(&a), TransparencyMode::XRay);
        assert_eq!(
            active_mode(&a),
            TransparencyMode::Normal,
            "the glyph's mode"
        );
        assert!(a.status.contains("Inactive"), "{}", a.status);
        // Alt+click toggles the inactive meshes back to Normal.
        for _ in 0..30 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        click_held(&mut a, &ctx, at, Modifiers::ALT);
        assert_eq!(inactive_mode(&a), TransparencyMode::Normal);
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
    }

    #[test]
    fn ht06_wireframe_greys_transparency_and_toggles_still_work() {
        // XR-03: a click changes nothing and says why.
        let (mut a, ctx) = one_view();
        a.display.settings.active.style = view_display::Style::Wireframe;
        draw(&mut a, &ctx, SIZE, vec![]);
        let at = transparency_rect(&a, 0).center();
        click(&mut a, &ctx, at);
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
        assert!(a.status.contains("Wireframe"), "{}", a.status);
        // The list still opens; a greyed entry picks nothing.
        hold(&mut a, &ctx, at, Modifiers::NONE);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(at, false, Modifiers::NONE)],
        );
        draw(&mut a, &ctx, SIZE, vec![]);
        let ghost = entry(&a, TransparencyMode::Ghost).center();
        click(&mut a, &ctx, ghost);
        assert_eq!(active_mode(&a), TransparencyMode::Normal);
        let locators = toggle_rect(&a, 0, ViewToggle::Locators).center();
        click(&mut a, &ctx, locators);
        assert!(!a.display.settings.drawing.locators, "toggles still work");
        // Alt+click still works when the inactive meshes aren't wireframe,
        // and not with Make Inactive Same as Active.
        for _ in 0..30 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        click_held(&mut a, &ctx, at, Modifiers::ALT);
        assert_eq!(inactive_mode(&a), TransparencyMode::XRay);
        a.display.settings.drawing.inactive_same_as_active = true;
        for _ in 0..30 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        click_held(&mut a, &ctx, at, Modifiers::ALT);
        assert_eq!(inactive_mode(&a), TransparencyMode::XRay);
        assert!(a.status.contains("Same as Active"), "{}", a.status);
    }

    #[test]
    fn ht07_toggles_run_their_commands_and_follow_viewport_properties() {
        // HT-07, HT-11.
        let (mut a, ctx) = one_view();
        let before = history_lines(&a).len();
        for (toggle, initially) in [
            (ViewToggle::Locators, true),
            (ViewToggle::InactiveInvisible, false),
            (ViewToggle::SameAsActive, false),
        ] {
            let at = toggle_rect(&a, 0, toggle).center();
            assert_eq!(a.line_state(toggle.command()).unwrap().on, initially);
            click(&mut a, &ctx, at);
            assert_eq!(a.line_state(toggle.command()).unwrap().on, !initially);
            for _ in 0..30 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
        }
        let d = &a.display.settings.drawing;
        assert!(!d.locators && d.inactive_invisible && d.inactive_same_as_active);
        let lines = history_lines(&a);
        assert!(
            lines[before..]
                .iter()
                .any(|l| l.starts_with("view3d.showLocators") && l.ends_with("false")),
            "{lines:?}"
        );
        // The Viewport Properties row's command sets the same field: the
        // button follows it.
        a.execute("view3d.showLocators true");
        assert!(a.line_state("view3d.showLocators").unwrap().on);
        assert!(a.display.settings.drawing.locators);
    }

    #[test]
    fn ht05_ht06_weightmaps_and_distortion_lead_the_toggles() {
        // Spec v1.1: Weightmaps, UV Distortion, Locators, Inactive
        // Invisible, Same As Active after the separator; both off.
        let (a, _ctx) = one_view();
        let order: Vec<f32> = ViewToggle::ALL
            .iter()
            .map(|&t| toggle_rect(&a, 0, t).left())
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        assert_eq!(ViewToggle::ALL[0], ViewToggle::Weights);
        assert_eq!(ViewToggle::ALL[1], ViewToggle::Distortion);
        assert!(!a.line_state("view3d.showweights").unwrap().on);
        assert!(!a.line_state("view3d.showDistortion").unwrap().on);
        let d = view_display::Drawing::default();
        assert!(!d.weights && !d.distortion, "off by default");
    }

    #[test]
    fn ht20_weight_and_distortion_rows_follow_the_buttons_and_back() {
        let (mut a, ctx) = one_view();
        for (toggle, k) in [
            (
                ViewToggle::Weights,
                crate::viewport_properties::key::WEIGHTS,
            ),
            (
                ViewToggle::Distortion,
                crate::viewport_properties::key::DISTORTION,
            ),
        ] {
            let at = toggle_rect(&a, 0, toggle).center();
            click(&mut a, &ctx, at);
            assert!(a.line_state(toggle.command()).unwrap().on, "{toggle:?}");
            // The Viewport Properties row reads the same field.
            a.execute(&format!("{} ?", toggle.command()));
            assert_eq!(a.status, "true", "{toggle:?}");
            assert!(crate::viewport_properties::row_value_for_tests(&a, k));
            // The row's command (named argument) turns it back off.
            let arg = if toggle == ViewToggle::Weights {
                "state"
            } else {
                "enabled"
            };
            a.execute(&format!("{} {arg}:false", toggle.command()));
            assert!(!a.line_state(toggle.command()).unwrap().on, "{toggle:?}");
            assert!(!crate::viewport_properties::row_value_for_tests(&a, k));
            for _ in 0..30 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
        }
        // Saved with the view's settings; older settings load them off.
        a.display.settings.drawing.weights = true;
        let json = serde_json::to_string(&a.display.settings.drawing).unwrap();
        let back: view_display::Drawing = serde_json::from_str(&json).unwrap();
        assert!(back.weights && !back.distortion);
        let old = json
            .replace(",\"weights\":true", "")
            .replace(",\"distortion\":false", "");
        assert!(!old.contains("weights"), "{old}");
        let back: view_display::Drawing = serde_json::from_str(&old).unwrap();
        assert!(!back.weights && !back.distortion);
    }

    #[test]
    fn ht18_weightmaps_and_distortion_grey_out_in_wireframe() {
        let (mut a, ctx) = one_view();
        a.display.settings.active.style = view_display::Style::Wireframe;
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        for toggle in [ViewToggle::Weights, ViewToggle::Distortion] {
            assert_eq!(
                toggle.unavailable(&a.display.settings),
                Some("Not with the Wireframe style")
            );
            let at = toggle_rect(&a, 0, toggle).center();
            click(&mut a, &ctx, at);
            assert!(!a.line_state(toggle.command()).unwrap().on, "{toggle:?}");
        }
        // The other toggles stay usable.
        assert_eq!(ViewToggle::Locators.unavailable(&a.display.settings), None);
    }

    #[test]
    fn each_view_keeps_its_own_transparency_and_toggles() {
        // XR-3 / HT-10: per view, like the other viewport properties.
        let (mut a, ctx) = quad();
        let other = *a
            .corner
            .areas
            .keys()
            .find(|v| {
                **v != a.active_view && a.corner.rects.contains_key(&(**v, Corner::Transparency))
            })
            .expect("a second view with the button");
        let at = transparency_rect(&a, other).center();
        click(&mut a, &ctx, at);
        let mode = |a: &Operandi, v: u32| {
            if v == a.active_view {
                a.display.settings.active.transparency.mode
            } else {
                a.displays[&v].settings.active.transparency.mode
            }
        };
        assert_eq!(mode(&a, other), TransparencyMode::XRay);
        let rest: Vec<u32> = a
            .corner
            .areas
            .keys()
            .copied()
            .filter(|v| *v != other)
            .collect();
        for v in rest {
            assert_eq!(mode(&a, v), TransparencyMode::Normal, "view {v}");
        }
    }

    /// Four views: the main window split into a quad.
    fn quad() -> (Operandi, Context) {
        let mut a = box_app();
        a.camera.view = View::Perspective;
        let ctx = Context::default();
        for _ in 0..3 {
            a.split_view();
        }
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        (a, ctx)
    }

    #[test]
    fn a_single_view_shows_an_orange_maximise_that_builds_a_quad() {
        // Jason, 2026-11-06 (VB-Q1): always visible; orange when one view
        // fills the frame; in a lone view it builds Top, Perspective /
        // Front, Right with the view in the slot matching its type.
        for (kind, slot) in [
            (View::Top, 0),
            (View::Perspective, 1),
            (View::Front, 2),
            (View::Right, 3),
            (View::Left, 1),
        ] {
            let mut a = box_app();
            a.camera.view = kind;
            a.camera.distance = 3.25;
            let ctx = Context::default();
            for _ in 0..2 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
            let button = a.corner.rects[&(0, Corner::Maximize)];
            assert_eq!(a.corner.groups[&0], (false, false), "alone: drawn orange");
            let height = a.viewport_rect.height();
            click(&mut a, &ctx, button.center());
            assert_eq!(a.views.viewports().len(), 4, "{kind:?}");
            assert!(a.views.in_group(0));
            assert_eq!(a.active_view, 0);
            assert_eq!(a.camera.view, kind, "the view keeps its type");
            // And its camera: the lens stays, the distance keeps the world
            // scale in the smaller view.
            assert_eq!(a.camera.lens, camera::DEFAULT_LENS, "{kind:?}");
            let scale = f64::from(a.viewport_rect.height() / height);
            assert!(
                (a.camera.distance - 3.25 * scale).abs() < 1e-3,
                "{kind:?}: {} {scale}",
                a.camera.distance
            );
            // Slots run top left, top right, bottom left, bottom right.
            let mut views: Vec<(u32, Rect)> =
                a.corner.areas.iter().map(|(&v, &r)| (v, r)).collect();
            views.sort_by(|x, y| {
                (x.1.top() as i32, x.1.left() as i32).cmp(&(y.1.top() as i32, y.1.left() as i32))
            });
            assert_eq!(views[slot].0, 0, "{kind:?} in slot {slot}");
            let types = [View::Top, View::Perspective, View::Front, View::Right];
            for (index, (view, _)) in views.iter().enumerate() {
                if index != slot {
                    assert_eq!(a.cameras[view].view, types[index]);
                }
            }
            // Now a group: every button white, and Maximise works as usual.
            for view in a.views.viewports() {
                assert_eq!(a.corner.groups[&view], (true, false));
            }
            a.toggle_maximized(0);
            draw(&mut a, &ctx, SIZE, vec![]);
            assert!(a.views.is_maximized(0));
        }
    }

    #[test]
    fn maximise_shows_one_view_and_restore_brings_the_group_back() {
        // VB-T1.
        let (mut a, ctx) = quad();
        let before: BTreeMap<u32, Rect> = a.corner.areas.clone();
        assert_eq!(before.len(), 4);
        for view in 0..4 {
            assert!(a.corner.rects.contains_key(&(view, Corner::Maximize)));
            assert!(!a.views.is_maximized(view));
        }
        a.cameras.get_mut(&2).unwrap().view = View::Front;
        let history = a.editor.undo.len();
        let button = a.corner.rects[&(2, Corner::Maximize)];
        click(&mut a, &ctx, button.center());
        assert!(a.views.is_maximized(2));
        assert_eq!(a.active_view, 2);
        assert_eq!(a.camera.view, View::Front, "the same view, enlarged");
        // Only it draws, filling the group's frame.
        assert_eq!(a.corner.areas.len(), 1);
        let frame = before
            .values()
            .fold(Rect::NOTHING, |r: Rect, a| r.union(*a));
        let area = a.corner.areas[&2];
        // (The group's tab bar goes too, so the view reaches its top.)
        assert!(area.contains_rect(frame), "{area:?} {frame:?}");
        assert!(area.height() - frame.height() < 30., "{area:?} {frame:?}");
        // The button now restores; clicking it brings every view back where
        // it was.
        let button = a.corner.rects[&(2, Corner::Maximize)];
        click(&mut a, &ctx, button.center());
        assert!(!a.views.is_maximized(2));
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.areas, before);
        assert_eq!(a.editor.undo.len(), history, "never an undo step");
    }

    #[test]
    fn splitting_a_maximised_view_leaves_two_views_to_toggle_with() {
        // Maximise is one view versus the multiple-view layout: splitting
        // the maximised view drops out of Maximise into the two halves,
        // and they replace the quad Maximise would have restored.
        let (mut a, ctx) = quad();
        a.cameras.get_mut(&2).unwrap().view = View::Front;
        a.toggle_maximized(2);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.views.is_maximized(2));
        a.split_view();
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.views.maximized_in(None), None);
        let views = a.views.viewports();
        assert_eq!(views.len(), 2, "{views:?}");
        assert!(views.contains(&2));
        let copy = a.active_view;
        assert_ne!(copy, 2, "the new half is active");
        assert_eq!(a.camera.view, View::Front, "a copy of the split view");
        assert_eq!(a.corner.areas.len(), 2);
        // The hidden quad views are gone, cameras and all.
        for gone in [0, 1, 3] {
            assert!(!a.cameras.contains_key(&gone));
            assert!(!a.displays.contains_key(&gone));
        }
        // Maximise now toggles between one view and the two halves.
        let halves = a.corner.areas.clone();
        a.toggle_maximized(copy);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.views.is_maximized(copy));
        assert_eq!(a.corner.areas.len(), 1);
        a.toggle_maximized(copy);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.areas, halves);
    }

    #[test]
    fn changes_made_while_maximised_stay_after_restore() {
        // VB-T2.
        let (mut a, ctx) = quad();
        a.cameras.get_mut(&1).unwrap().view = View::Perspective;
        a.toggle_maximized(1);
        draw(&mut a, &ctx, SIZE, vec![]);
        a.camera.yaw += 0.7;
        let yaw = a.camera.yaw;
        a.execute("wireframe view");
        draw(&mut a, &ctx, SIZE, vec![]);
        a.toggle_maximized(1);
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert_eq!(a.cameras[&1].yaw, yaw);
        assert_eq!(
            a.displays[&1].settings.active.style,
            view_display::Style::Wireframe
        );
        // The hidden views kept theirs.
        assert_eq!(
            a.displays[&0].settings.active.style,
            view_display::Style::Default
        );
    }

    #[test]
    fn numpad_zero_and_alt_zero_toggle_the_view_under_the_pointer() {
        // VB-T3.
        let (mut a, ctx) = quad();
        let over = a.corner.areas[&3].center();
        let press = |a: &mut Operandi, keypad: bool, modifiers: Modifiers| {
            a.native_numbers = vec![native::NumberKey {
                digit: 0,
                keypad,
                pressed: true,
            }];
            draw(
                a,
                &ctx,
                SIZE,
                vec![
                    Event::PointerMoved(over),
                    Event::Key {
                        key: Key::Num0,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                    Event::Key {
                        key: Key::Num0,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers,
                    },
                ],
            );
            draw(a, &ctx, SIZE, vec![]);
        };
        press(&mut a, true, Modifiers::NONE);
        assert!(a.views.is_maximized(3));
        press(&mut a, true, Modifiers::NONE);
        assert!(!a.views.is_maximized(3));
        press(&mut a, false, Modifiers::ALT);
        assert!(a.views.is_maximized(3));
        press(&mut a, false, Modifiers::ALT);
        assert!(!a.views.is_maximized(3));
        // Plain main-keyboard 0 isn't Maximise.
        press(&mut a, false, Modifiers::NONE);
        assert!(!a.views.is_maximized(3));
        // Rebindable like any shortcut.
        a.bindings.insert(MAXIMIZE.into(), "Shift+M".into());
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![
                Event::PointerMoved(over),
                Event::Key {
                    key: Key::M,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::SHIFT,
                },
            ],
        );
        assert!(a.views.is_maximized(3));
        let _ = key;
    }

    /// Press `key` (a keypad digit when `keypad`) with the pointer at
    /// `over`, then let a frame settle.
    fn press_over(a: &mut Operandi, ctx: &Context, over: Pos2, key: Key, keypad: bool) {
        let digit = if key == Key::Period {
            Some(10)
        } else {
            key.name().parse::<u8>().ok()
        };
        a.native_numbers = digit
            .map(|digit| native::NumberKey {
                digit,
                keypad,
                pressed: true,
            })
            .into_iter()
            .collect();
        draw(
            a,
            ctx,
            SIZE,
            vec![
                Event::PointerMoved(over),
                Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                },
                Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        a.native_numbers.clear();
        draw(a, ctx, SIZE, vec![]);
    }

    #[test]
    fn the_wheel_zooms_the_hovered_view_not_the_last_clicked() {
        // Jason, 2026-11-08: "now do it for the scroll wheel to zoom".
        let (mut a, ctx) = quad();
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        let before0 = a.cameras[&0].distance;
        let before2 = a.cameras[&2].distance;
        let over = a.corner.areas[&2].center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(over)]);
        for _ in 0..3 {
            draw(
                &mut a,
                &ctx,
                SIZE,
                vec![Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0., 40.),
                    modifiers: Modifiers::NONE,
                }],
            );
        }
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.active_view, 2);
        assert_ne!(a.cameras[&2].distance, before2);
        assert_eq!(a.cameras[&0].distance, before0);
    }

    #[test]
    fn the_space_mouse_moves_the_hovered_view_and_stops_at_rest() {
        use crate::space_mouse::decode::{DeviceState, RawAxes};
        let (mut a, ctx) = quad();
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        let before0 = a.cameras[&0].clone();
        let over = a.corner.areas[&2].center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(over)]);
        let before2 = a.cameras[&2].clone();
        // Twist and pull: orbit and zoom in.
        a.space_mouse.state = DeviceState {
            axes: RawAxes {
                t: [0, 300, 0],
                r: [0, 0, 300],
            },
            buttons: 0,
        };
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        assert_eq!(a.active_view, 2, "the hovered view becomes active (UIC-70)");
        a.space_mouse.state = DeviceState::default();
        draw(&mut a, &ctx, SIZE, vec![]);
        let moved = a.cameras[&2].clone();
        assert_ne!(moved.yaw, before2.yaw);
        assert!(moved.distance < before2.distance);
        // Orbit keeps the centre; the pull takes it toward the surface at
        // the view centre (fix-stuck-zoom), never further than the eye
        // closes in: no jump.
        assert!(
            (moved.target - before2.target).length() <= before2.distance - moved.distance + 1e-9,
            "orbit and zoom keep the centre"
        );
        assert_eq!(a.cameras[&0].yaw, before0.yaw);
        assert_eq!(a.cameras[&0].distance, before0.distance);
        // At rest nothing moves.
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.cameras[&2].yaw, moved.yaw);
        assert_eq!(a.cameras[&2].distance, moved.distance);
    }

    #[test]
    fn a_space_mouse_button_presses_its_shortcut_on_the_hovered_view() {
        use crate::space_mouse::decode::DeviceState;
        use crate::space_mouse::source::Event as Puck;
        let (mut a, ctx) = quad();
        // A view's camera: the live one for the active view.
        let distance = |a: &Operandi, view: u32| {
            if a.active_view == view {
                a.camera.distance
            } else {
                a.cameras[&view].distance
            }
        };
        // View 2 far away, view 0 active.
        a.activate_view(2);
        a.camera.distance = 500.;
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        let before0 = distance(&a, 0);
        // Button 2 presses A (Frame All).
        a.space_mouse.prefs.buttons.0[1] = "A".into();
        let over = a.corner.areas[&2].center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(over)]);
        let press = |buttons| {
            Puck::State(DeviceState {
                buttons,
                ..Default::default()
            })
        };
        // Button 1 has no shortcut: nothing happens.
        a.space_mouse.handle(press(0b01));
        a.space_mouse.handle(press(0));
        draw(&mut a, &ctx, SIZE, vec![]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(distance(&a, 2), 500.);
        a.space_mouse.handle(press(0b10));
        draw(&mut a, &ctx, SIZE, vec![]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.active_view, 2, "the hovered view, as the key does");
        assert_ne!(distance(&a, 2), 500., "framed");
        assert_eq!(distance(&a, 0), before0);
        // Held: no repeat.
        a.camera.distance = 500.;
        a.space_mouse.handle(press(0b10));
        draw(&mut a, &ctx, SIZE, vec![]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(distance(&a, 2), 500., "still held");
        assert_eq!(a.space_mouse.seen_buttons, 2);
    }

    #[test]
    fn numpad_zero_maximises_the_hovered_view_not_the_last_clicked() {
        // UIC-70: no click first (Jason, 2026-11-08).
        let (mut a, ctx) = quad();
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        let over = a.corner.areas[&2].center();
        press_over(&mut a, &ctx, over, Key::Num0, true);
        assert!(a.views.is_maximized(2));
        assert_eq!(a.active_view, 2);
        press_over(&mut a, &ctx, over, Key::Num0, true);
        assert!(!a.views.is_maximized(2));
    }

    #[test]
    fn view_keys_act_on_the_hovered_view_not_the_last_clicked() {
        // Jason, 2026-11-08: "pressing 'a' to frame all should also work
        // on whatever viewport the mouse is hovering over".
        let (mut a, ctx) = quad();
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        let before0 = a.cameras[&0].clone();

        // Top View (keypad 1) over view 1.
        let over1 = a.corner.areas[&1].center();
        press_over(&mut a, &ctx, over1, Key::Num1, true);
        assert_eq!(a.active_view, 1);
        assert_eq!(a.cameras[&1].view, View::Top);
        assert_eq!(a.cameras[&0].view, before0.view);

        // Wireframe View (keypad 7) over view 3.
        let over3 = a.corner.areas[&3].center();
        press_over(&mut a, &ctx, over3, Key::Num7, true);
        assert_eq!(a.active_view, 3);
        assert_eq!(
            a.displays[&3].settings.active.style,
            view_display::Style::Wireframe
        );
        assert_ne!(
            a.displays[&1].settings.active.style,
            view_display::Style::Wireframe
        );

        // Frame All (A) and Zoom In (.) over view 2: its camera moves,
        // view 0's stays.
        let over2 = a.corner.areas[&2].center();
        let before2 = a.cameras[&2].clone();
        a.cameras.get_mut(&2).unwrap().distance *= 7.;
        press_over(&mut a, &ctx, over2, Key::A, false);
        assert_eq!(a.active_view, 2);
        assert_ne!(a.cameras[&2].distance, before2.distance * 7.);
        let framed = a.cameras[&2].distance;
        press_over(&mut a, &ctx, over2, Key::Period, false);
        assert!(a.cameras[&2].distance < framed);
        assert_eq!(a.cameras[&0].distance, before0.distance);
        assert_eq!(a.cameras[&0].view, before0.view);
    }

    #[test]
    fn the_system_mouse_lands_in_main_window_points() {
        // Retina, no UI zoom: window at (100, 50) points.
        let p = screen_to_window([300., 250.], Pos2::new(100., 50.), 2., 2.);
        assert_eq!(p, Pos2::new(200., 200.));
        // UI zoomed 1.5x: egui points are bigger than screen points.
        let inner = Pos2::new(100. * 2. / 3., 50. * 2. / 3.);
        let p = screen_to_window([400., 350.], inner, 2., 3.);
        assert!((p - Pos2::new(200., 200.)).length() < 1e-3, "{p:?}");
    }

    #[test]
    fn a_window_with_a_known_place_draws_and_notes_it() {
        // Regression: with the window's place known (always, in the app),
        // noting it locked the context and the app never showed.
        let mut a = box_app();
        let ctx = Context::default();
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
            ..Default::default()
        };
        let info = raw.viewports.entry(egui::ViewportId::ROOT).or_default();
        info.inner_rect = Some(Rect::from_min_size(Pos2::new(100., 50.), SIZE));
        info.native_pixels_per_point = Some(2.);
        let _ = ctx.run(raw, |ctx| a.viewport(ctx));
        assert_eq!(
            a.corner.root_window.map(|(min, native, _)| (min, native)),
            Some((Pos2::new(100., 50.), 2.))
        );
    }

    #[test]
    fn the_view_pie_acts_on_the_view_it_opens_over() {
        let (mut a, ctx) = quad();
        a.activate_view(0);
        draw(&mut a, &ctx, SIZE, vec![]);
        let over = a.corner.areas[&1].center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(over)]);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::Key {
                key: Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            }],
        );
        assert_eq!(
            a.pie.as_ref().map(|p| p.pie.name.as_str()),
            Some(crate::pie_menu::VIEW)
        );
        assert_eq!(a.active_view, 1);
    }

    #[test]
    fn frame_all_with_the_pointer_off_the_views_still_frames_the_active_one() {
        let (mut a, ctx) = quad();
        a.activate_view(1);
        draw(&mut a, &ctx, SIZE, vec![]);
        a.camera.distance *= 7.;
        let far = a.camera.distance;
        // Over the left toolbox, not a view.
        let outside = Pos2::new(2., SIZE.y / 2.);
        assert!(!a.corner.areas.values().any(|r| r.contains(outside)));
        press_over(&mut a, &ctx, outside, Key::A, false);
        assert_eq!(a.active_view, 1);
        assert_ne!(a.camera.distance, far);
    }

    #[test]
    fn maximised_state_survives_a_relaunch() {
        // VB-T10.
        let (mut a, ctx) = quad();
        a.toggle_maximized(1);
        let saved = a.views.to_json().unwrap();
        let reopened = operandi_layout::Layout::from_json(&saved).unwrap();
        assert_eq!(reopened.maximized_in(None), Some(1));
        let mut b = box_app();
        b.views = reopened;
        for _ in 0..2 {
            draw(&mut b, &ctx, SIZE, vec![]);
        }
        assert_eq!(b.corner.areas.keys().copied().collect::<Vec<_>>(), [1]);
        // Closing the maximised view restores the rest.
        b.close_view(1);
        draw(&mut b, &ctx, SIZE, vec![]);
        assert_eq!(b.corner.areas.len(), 3);
        assert!(b.views.panes().contains(&Pane::Viewport(0)));
    }

    /// Press `button` (mouse button `with`) on a corner button and drag by
    /// `steps`, then release, with `escape` pressed after the drag if asked.
    /// The app puts the pointer back on the icon after each move, so every
    /// move is reported from the icon, as the window would.
    fn drag_with(
        a: &mut Operandi,
        ctx: &Context,
        view: u32,
        button: Corner,
        with: PointerButton,
        steps: &[Vec2],
        escape: bool,
    ) {
        let start = a.corner.rects[&(view, button)].center();
        let press = |pos, pressed| Event::PointerButton {
            pos,
            button: with,
            pressed,
            modifiers: Modifiers::NONE,
        };
        draw(a, ctx, SIZE, vec![Event::PointerMoved(start)]);
        draw(a, ctx, SIZE, vec![press(start, true)]);
        for step in steps {
            let out = draw(a, ctx, SIZE, vec![Event::PointerMoved(start + *step)]);
            assert!(a.corner.drag.is_some(), "dragging");
            assert_eq!(out.platform_output.cursor_icon, CursorIcon::None, "hidden");
            draw(a, ctx, SIZE, vec![Event::PointerMoved(start)]);
        }
        if escape {
            draw(a, ctx, SIZE, vec![key(Key::Escape)]);
            draw(
                a,
                ctx,
                SIZE,
                vec![Event::PointerMoved(start + vec2(40., 40.))],
            );
        }
        draw(a, ctx, SIZE, vec![press(start, false)]);
        draw(a, ctx, SIZE, vec![]);
        assert!(a.corner.drag.is_none());
    }

    fn drag_button(
        a: &mut Operandi,
        ctx: &Context,
        view: u32,
        button: Corner,
        steps: &[Vec2],
        escape: bool,
    ) {
        drag_with(a, ctx, view, button, PointerButton::Primary, steps, escape);
    }

    /// One Perspective view (the box app starts in Top).
    fn one_view() -> (Operandi, Context) {
        let mut a = box_app();
        a.camera.view = View::Perspective;
        let ctx = Context::default();
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        (a, ctx)
    }

    #[test]
    fn the_buttons_run_pan_rotate_zoom_maximise_left_to_right() {
        let (mut a, ctx) = quad();
        let rects: Vec<Rect> = [
            Corner::Pan,
            Corner::Rotate,
            Corner::Zoom,
            Corner::Maximize,
            Corner::Properties,
        ]
        .iter()
        .map(|&b| a.corner.rects[&(a.active_view, b)])
        .collect();
        assert!(rects.windows(2).all(|w| w[0].right() <= w[1].left()));
        // At the header's right end, above the view.
        let area = a.corner.areas[&a.active_view];
        // (Layouts stage 2: the content arrow keeps the far right.)
        assert!(area.right() - rects[4].right() < 12. + ARROW + GAP);
        // Floating inside the view along its top edge.
        let view = a.view_rects[&a.active_view];
        assert!(rects[0].top() >= view.top() && rects[0].bottom() <= view.top() + 26.);
        let _ = &mut a;
        let _ = ctx;
    }

    #[test]
    fn ortho_views_have_no_rotate_and_perspective_adds_it() {
        // VB-T4.
        let (mut a, ctx) = one_view();
        for view in [
            View::Top,
            View::Bottom,
            View::Front,
            View::Back,
            View::Left,
            View::Right,
        ] {
            a.corner.rects.clear();
            a.camera.view = view;
            draw(&mut a, &ctx, SIZE, vec![]);
            assert!(
                !a.corner.rects.contains_key(&(0, Corner::Rotate)),
                "{view:?}"
            );
            assert!(a.corner.rects.contains_key(&(0, Corner::Pan)));
            assert!(a.corner.rects.contains_key(&(0, Corner::Zoom)));
        }
        a.execute("perspective view");
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.corner.rects.contains_key(&(0, Corner::Rotate)));
    }

    #[test]
    fn pan_moves_the_view_and_the_preference_gives_grab_feel() {
        // VB-T5 (v1.1): drag right, the scene slides left at the button's
        // rate; drag down, it slides up.
        let (mut a, ctx) = one_view();
        a.camera.view = View::Front;
        draw(&mut a, &ctx, SIZE, vec![]);
        let history = a.editor.undo.len();
        let origin = |a: &Operandi| a.camera.project(DVec3::ZERO, a.viewport_rect).unwrap().0;
        let before = origin(&a);
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Pan,
            &[vec2(10., 0.), vec2(10., 0.)],
            false,
        );
        let after = origin(&a);
        let travel = 20. * PAN_BUTTON_RATE;
        assert!(
            (before.x - after.x - travel).abs() < 1.,
            "{before:?} {after:?}"
        );
        assert!((after.y - before.y).abs() < 0.5);
        drag_button(&mut a, &ctx, 0, Corner::Pan, &[vec2(0., 10.)], false);
        assert!(
            (after.y - origin(&a).y - 10. * PAN_BUTTON_RATE).abs() < 1.,
            "down: up"
        );
        // Reverse Pan Widget Direction: grab feel, same rate.
        a.input_preferences.options.reverse_pan_widget = true;
        let before = origin(&a);
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Pan,
            &[vec2(10., 0.), vec2(10., 0.)],
            false,
        );
        assert!(
            (origin(&a).x - before.x - travel).abs() < 1.,
            "reversed: scene moves right"
        );
        assert_eq!(a.editor.undo.len(), history, "never an undo step");
    }

    #[test]
    fn right_and_middle_drags_act_like_left_and_clicks_do_nothing() {
        let (mut a, ctx) = quad();
        let view = a.active_view;
        let yaw = a.camera.yaw;
        drag_with(
            &mut a,
            &ctx,
            view,
            Corner::Rotate,
            PointerButton::Secondary,
            &[vec2(50., 0.)],
            false,
        );
        let by_right = a.camera.yaw;
        assert_ne!(by_right, yaw);
        drag_with(
            &mut a,
            &ctx,
            view,
            Corner::Rotate,
            PointerButton::Middle,
            &[vec2(50., 0.)],
            false,
        );
        assert!((a.camera.yaw - by_right - (by_right - yaw)).abs() < 1e-9);
        // A right-click (no drag) on any button does nothing and opens nothing.
        let camera = a.camera.clone();
        for button in [
            Corner::Pan,
            Corner::Rotate,
            Corner::Zoom,
            Corner::Maximize,
            Corner::Properties,
        ] {
            let at = a.corner.rects[&(view, button)].center();
            let right = |pressed| Event::PointerButton {
                pos: at,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
            draw(&mut a, &ctx, SIZE, vec![right(true)]);
            draw(&mut a, &ctx, SIZE, vec![right(false)]);
            draw(&mut a, &ctx, SIZE, vec![]);
            assert!(!egui::Popup::is_any_open(&ctx), "{button:?}");
            assert_eq!(a.corner.properties.view, None);
            assert!(!a.views.is_maximized(view));
        }
        assert_eq!(
            (a.camera.yaw, a.camera.distance),
            (camera.yaw, camera.distance)
        );
        assert_eq!(a.camera.target, camera.target);
    }

    #[test]
    fn maximise_and_restore_keep_world_scale() {
        // v1.1: the same units per point before and after, in perspective
        // and ortho; the view just shows more around the geometry.
        for view_type in [View::Perspective, View::Front] {
            // Two views one above the other, so Maximise changes the height
            // (the projection's scale follows the height).
            let mut a = box_app();
            let ctx = Context::default();
            a.split_view();
            if let Some(root) = a.views.main.root
                && let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(split))) =
                    a.views.main.tiles.get_mut(root)
            {
                split.dir = egui_tiles::LinearDir::Vertical;
            }
            let view = a.active_view;
            a.camera.view = view_type;
            for _ in 0..3 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
            // A unit across the view at the orbit centre.
            let size = |a: &Operandi| {
                let (_, right, _, _) = a.camera.basis();
                let p = |x: f64| {
                    a.camera
                        .project(a.camera.target + right * x, a.viewport_rect)
                        .unwrap()
                        .0
                };
                p(1.).x - p(0.).x
            };
            let before = size(&a);
            let lens = a.camera.lens;
            let small = a.viewport_rect;
            a.toggle_maximized(view);
            for _ in 0..2 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
            assert!(a.viewport_rect.height() > small.height() * 1.5);
            assert!(
                (size(&a) - before).abs() < 0.01,
                "{view_type:?}: {} {before}",
                size(&a)
            );
            // The field of view stays: the lens never takes the change.
            assert_eq!(a.camera.lens, lens, "{view_type:?}");
            a.toggle_maximized(view);
            for _ in 0..2 {
                draw(&mut a, &ctx, SIZE, vec![]);
            }
            assert_eq!(a.viewport_rect, small);
            assert!((size(&a) - before).abs() < 0.01, "{view_type:?} restored");
        }
        // Choosing another view type afterwards keeps the scale, and a
        // Perspective view chosen after a maximised Front view keeps the
        // normal lens (a maximised ortho view used to hand it a wide one).
        let mut a = box_app();
        let ctx = Context::default();
        a.split_view();
        if let Some(root) = a.views.main.root
            && let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(split))) =
                a.views.main.tiles.get_mut(root)
        {
            split.dir = egui_tiles::LinearDir::Vertical;
        }
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        a.camera.view = View::Front;
        a.toggle_maximized(a.active_view);
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        let distance = a.camera.distance;
        assert_eq!(a.camera.lens, camera::DEFAULT_LENS);
        a.choose_view_type(Some(View::Top));
        assert_eq!(a.camera.distance, distance);
        a.choose_view_type(Some(View::Perspective));
        assert_eq!(a.camera.lens, camera::DEFAULT_LENS);
    }

    /// Sylvain's stretched Perspective view: Maximise and Restore, again
    /// and again, with the window resized in between, never change the
    /// field of view (the lens), only how far back the view sits.
    #[test]
    fn maximising_never_widens_the_perspective_lens() {
        let mut a = box_app();
        let ctx = Context::default();
        a.split_view();
        if let Some(root) = a.views.main.root
            && let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(split))) =
                a.views.main.tiles.get_mut(root)
        {
            split.dir = egui_tiles::LinearDir::Vertical;
        }
        let view = a.active_view;
        a.camera.view = View::Perspective;
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        let sizes = [
            SIZE,
            vec2(SIZE.x, SIZE.y * 0.6),
            vec2(SIZE.x * 1.3, SIZE.y * 1.4),
        ];
        for size in sizes.into_iter().cycle().take(12) {
            a.toggle_maximized(view);
            for _ in 0..2 {
                draw(&mut a, &ctx, size, vec![]);
            }
        }
        assert_eq!(a.camera.view, View::Perspective);
        assert_eq!(a.camera.lens, camera::DEFAULT_LENS);
    }

    #[test]
    fn zoom_follows_the_zoom_drag_axis() {
        // VB-T6.
        use crate::navigation_preferences::ZoomDragAxis;
        let (mut a, ctx) = one_view();
        let start = a.camera.distance;
        let target = a.camera.target;
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Zoom,
            &[vec2(40., 0.), vec2(40., 0.)],
            false,
        );
        assert!(a.camera.distance < start, "left to right zooms in");
        // About the view centre: the orbit centre only moves along the
        // view's axis (toward the surface there, fix-stuck-zoom).
        let (_, _, _, forward) = a.camera.basis();
        assert!(
            (a.camera.target - target).cross(forward).length() < 1e-9,
            "about the view centre"
        );
        // Only the chosen axis counts.
        let d = a.camera.distance;
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Zoom,
            &[vec2(0., 40.), vec2(0., 40.)],
            false,
        );
        assert_eq!(a.camera.distance, d);
        a.input_preferences.options.zoom_drag_axis = ZoomDragAxis::BottomToTop;
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Zoom,
            &[vec2(0., -40.), vec2(0., -40.)],
            false,
        );
        assert!(a.camera.distance < d, "bottom to top zooms in");
    }

    #[test]
    fn rotate_turns_the_view_like_the_orbit_gesture() {
        // VB-T7: 200 points on the button = 200 points of in-view orbit.
        let (mut a, ctx) = one_view();
        let (yaw, pitch) = (a.camera.yaw, a.camera.pitch);
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Rotate,
            &[vec2(100., 0.), vec2(100., 0.)],
            false,
        );
        let by_button = (a.camera.yaw, a.camera.pitch);
        assert_ne!(by_button.0, yaw);
        let (mut b, ctx_b) = one_view();
        assert_eq!((b.camera.yaw, b.camera.pitch), (yaw, pitch));
        let at = b.viewport_rect.center();
        draw(&mut b, &ctx_b, SIZE, vec![Event::PointerMoved(at)]);
        draw(
            &mut b,
            &ctx_b,
            SIZE,
            vec![pointer(at, true, Modifiers::ALT)],
        );
        for x in [100., 200.] {
            draw(
                &mut b,
                &ctx_b,
                SIZE,
                vec![Event::PointerMoved(at + vec2(x, 0.))],
            );
        }
        draw(
            &mut b,
            &ctx_b,
            SIZE,
            vec![pointer(at + vec2(200., 0.), false, Modifiers::ALT)],
        );
        assert!(
            (b.camera.yaw - by_button.0).abs() < 1e-9,
            "{} {}",
            b.camera.yaw,
            by_button.0
        );
        assert!((b.camera.pitch - by_button.1).abs() < 1e-9);
    }

    #[test]
    fn escape_puts_the_view_back_and_a_click_does_nothing() {
        let (mut a, ctx) = one_view();
        let before = a.camera.clone();
        drag_button(
            &mut a,
            &ctx,
            0,
            Corner::Rotate,
            &[vec2(50., 20.), vec2(50., 0.)],
            true,
        );
        assert_eq!(a.camera.yaw, before.yaw);
        assert_eq!(a.camera.pitch, before.pitch);
        let pan = a.corner.rects[&(0, Corner::Pan)].center();
        click(&mut a, &ctx, pan);
        assert_eq!(a.camera.target, before.target);
        assert_eq!(a.camera.distance, before.distance);
    }

    /// The view menu's Light sub-menu: greyed with no lights; with one,
    /// picking it by name looks through it.
    #[test]
    fn the_view_menu_light_entry_looks_through_a_light() {
        let (mut a, ctx) = one_view();
        let open = |a: &mut Operandi, ctx: &Context| {
            a.toolbox_rects
                .retain(|k, _| !k.starts_with("Light view") && !k.starts_with("View menu"));
            // The view type menu: the header's leftmost button.
            let combo = a.corner.header_hits[&0]
                .iter()
                .copied()
                .min_by(|x, y| x.left().total_cmp(&y.left()))
                .unwrap();
            click(a, ctx, combo.center());
            let light = a.toolbox_rects["View menu: Light"];
            for _ in 0..3 {
                draw(a, ctx, SIZE, vec![Event::PointerMoved(light.center())]);
            }
        };
        // box_app's scene has no lights: the entry is greyed.
        a.editor.scene.items.retain(|i| i.light.is_none());
        open(&mut a, &ctx);
        assert!(
            !a.toolbox_rects
                .contains_key("Light view: (Current Selection)")
        );
        egui::Popup::close_all(&ctx);
        a.add_item(operandi_core::item_list::AddItemKind::Light(
            operandi_core::camera_light::LightKind::Spot,
        ));
        let id = a.editor.scene.selection.item.unwrap();
        let name = a.editor.scene.item(id).unwrap().name.clone();
        draw(&mut a, &ctx, SIZE, vec![]);
        open(&mut a, &ctx);
        let entry = a.toolbox_rects[&format!("Light view: {name}")];
        let current = a.toolbox_rects["Light view: (Current Selection)"];
        assert!(
            current.bottom() <= entry.top() + 0.5,
            "(Current Selection) first"
        );
        click(&mut a, &ctx, entry.center());
        assert_eq!(a.through_light(), Some(id));
        assert!(!a.camera.light.unwrap().follow_selection);
        assert_eq!(a.view_title(), name);
    }

    #[test]
    fn the_header_floats_over_the_view_and_keeps_its_presses() {
        let (mut a, ctx) = one_view();
        let view = a.view_rects[&0];
        let pane = a.corner.areas[&0];
        assert!(
            view.top() - pane.top() < 1.,
            "no strip: the view starts at the top"
        );
        let hits = a.corner.header_hits[&0].clone();
        assert!(
            hits.len() >= 7,
            "view type, display, frame and corner buttons: {hits:?}"
        );
        for hit in &hits {
            assert!(view.contains_rect(*hit), "{hit:?} inside {view:?}");
            assert!(hit.height() <= HEADER_HEIGHT + 0.5, "{hit:?}");
        }
        // Buttons never overlap one another.
        for (i, x) in hits.iter().enumerate() {
            for y in &hits[i + 1..] {
                assert!(x.intersect(*y).area() < 1., "{x:?} {y:?}");
            }
        }
        // A press on a header button stays the button's: the view's
        // selection doesn't change, the Display menu opens.
        a.editor.scene.selection.mode = Mode::Polygon;
        let selection = a.editor.scene.selection.clone();
        let menu = a.corner.header_hits[&0]
            .iter()
            .copied()
            .find(|r| {
                r.left() > a.view_rects[&0].left() + 100.
                    && r.left() < a.view_rects[&0].left() + 250.
            })
            .expect("the Display button");
        click(&mut a, &ctx, menu.center());
        assert_eq!(a.editor.scene.selection, selection);
        assert!(egui::Popup::is_any_open(&ctx), "the Display menu opened");
        // Show Viewport Controls and Information off hides the header.
        egui::Popup::close_all(&ctx);
        a.display.settings.drawing.controls = false;
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(!a.corner.header_hits.contains_key(&0));
        assert!(!a.corner.rects.keys().any(|(v, _)| *v == 0));
    }

    #[test]
    fn a_narrow_view_drops_left_buttons_before_they_overlap() {
        for width in [1440., 1000., 860., 760.] {
            let mut a = box_app();
            a.camera.view = View::Perspective;
            let ctx = Context::default();
            for _ in 0..3 {
                a.split_view();
            }
            let size = vec2(width, 900.);
            for _ in 0..3 {
                draw(&mut a, &ctx, size, vec![]);
            }
            for (view, hits) in &a.corner.header_hits {
                let area = a.corner.areas[view];
                for (i, x) in hits.iter().enumerate() {
                    assert!(x.right() <= area.right() + 0.5, "{width}: {x:?}");
                    for y in &hits[i + 1..] {
                        assert!(x.intersect(*y).area() < 1., "{width}: {x:?} {y:?}");
                    }
                }
                // The corner buttons always stay.
                assert!(
                    a.corner.rects.contains_key(&(*view, Corner::Properties)),
                    "{width}"
                );
            }
        }
    }

    #[test]
    fn views_have_no_tab_bars_and_the_view_menu_has_their_commands() {
        use crate::app_menu::line_action;
        let (mut a, ctx) = quad();
        // No tab bar: each view's area starts where its view starts.
        for (view, area) in &a.corner.areas {
            assert!(
                (a.view_rects[view].top() - area.top()).abs() < 0.5,
                "{view}"
            );
        }
        // Only the view type and Display menus besides the corner buttons.
        // (Narrow views here may drop even those.)
        assert!(a.corner.header_hits[&a.active_view].len() <= 2 + 5);
        // The View menu: framing, split, tear off, close, navigation help.
        let view = a.active_view;
        a.camera.distance = 40.;
        a.menu_action(&ctx, line_action("Frame All"));
        assert!(a.camera.distance < 40.);
        a.menu_action(&ctx, line_action("operandi.help.navigation"));
        assert!(a.status.contains("Orbit Alt + Left drag"), "{}", a.status);
        // Split View refuses a view too narrow for two halves (VST-24):
        // the quad's last views are; the first is wide enough.
        a.menu_action(&ctx, line_action("viewport.split"));
        assert_eq!(a.views.viewports().len(), 4);
        a.activate_view(0);
        a.menu_action(&ctx, line_action("viewport.split"));
        assert_eq!(a.views.viewports().len(), 5);
        a.menu_action(&ctx, line_action("viewport.detach"));
        assert_eq!(a.views.windows.len(), 1);
        // Alone in its window the torn-off view can't be closed (the dot's
        // Delete rule); a main-window view can.
        a.menu_action(&ctx, line_action("operandi.viewport.close"));
        assert_eq!(a.views.viewports().len(), 5);
        a.activate_view(view);
        a.menu_action(&ctx, line_action("operandi.viewport.close"));
        assert_eq!(a.views.viewports().len(), 4);
        assert!(!a.views.viewports().contains(&view));
        draw(&mut a, &ctx, SIZE, vec![]);
    }

    #[test]
    fn view_information_is_off_by_default_and_per_view() {
        let d = view_display::Drawing::default();
        assert!(!d.info);
        let (mut a, ctx) = one_view();
        let text = |out: &egui::FullOutput| {
            out.shapes.iter().any(|s| {
                matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.contains("selected Polygon")
                    || t.galley.job.text.contains("selected Item")
                    || t.galley.job.text.contains("selected Vertex"))
            })
        };
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        assert!(!text(&out), "hidden by default");
        a.display.settings.drawing.info = true;
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        assert!(text(&out), "shown when on");
    }

    /// Front view, the origin's place on screen, and a Pan press at `at`.
    fn pan_press(at: Vec2) -> (Operandi, Context, Pos2, Pos2) {
        let (mut a, ctx) = one_view();
        a.camera.view = View::Front;
        draw(&mut a, &ctx, SIZE, vec![]);
        let start = a.corner.rects[&(0, Corner::Pan)].center() + at;
        let origin = a.camera.project(DVec3::ZERO, a.viewport_rect).unwrap().0;
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(start)]);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(start, true, Modifiers::NONE)],
        );
        (a, ctx, start, origin)
    }

    fn origin_now(a: &Operandi) -> Pos2 {
        a.camera.project(DVec3::ZERO, a.viewport_rect).unwrap().0
    }

    #[test]
    fn the_mouse_movement_drives_the_drag_not_late_pointer_places() {
        // VBF-1: after the pointer is put back, a pointer place measured
        // from before the put-back (here 15 right and 4 down of home, for a
        // real move of 5 right) must not count; the mouse's own movement
        // does. No drift down, and no move counted twice.
        let (mut a, ctx, start, before) = pan_press(Vec2::ZERO);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(start + vec2(10., 0.))],
        );
        for _ in 0..3 {
            draw(
                &mut a,
                &ctx,
                SIZE,
                vec![
                    Event::MouseMoved(vec2(5., 0.)),
                    Event::PointerMoved(start + vec2(15., 4.)),
                ],
            );
        }
        let after = origin_now(&a);
        let travel = 25. * PAN_BUTTON_RATE;
        assert!(
            (before.x - after.x - travel).abs() < 1.,
            "{before:?} {after:?}"
        );
        assert!((after.y - before.y).abs() < 0.5, "no sideways drift");
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(start, false, Modifiers::NONE)],
        );
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(a.corner.drag.is_none());
    }

    #[test]
    fn the_first_frame_counts_from_the_press_not_the_icon_centre() {
        // A press 6 right and 6 down of the icon's centre, then 10 right:
        // the view moves 10 right only, and the release puts the pointer
        // back where it was pressed.
        let (mut a, ctx, start, before) = pan_press(vec2(6., 6.));
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(start + vec2(10., 0.))],
        );
        let after = origin_now(&a);
        assert!(
            (before.x - after.x - 10. * PAN_BUTTON_RATE).abs() < 1.,
            "{before:?} {after:?}"
        );
        assert!((after.y - before.y).abs() < 0.5, "no jump down");
        let out = draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(start, false, Modifiers::NONE)],
        );
        let back = out
            .viewport_output
            .values()
            .flat_map(|v| v.commands.iter())
            .any(|c| matches!(c, ViewportCommand::CursorPosition(p) if *p == start));
        assert!(back, "pointer back at the press");
    }

    #[test]
    fn a_pointer_already_home_asks_for_no_put_back() {
        // Raw motion with no pointer move (the pointer is where it was put
        // back): the view moves, and no new put-back is sent.
        let (mut a, ctx, start, before) = pan_press(Vec2::ZERO);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(start + vec2(10., 0.))],
        );
        draw(&mut a, &ctx, SIZE, vec![Event::MouseMoved(vec2(1., 0.))]);
        let out = draw(&mut a, &ctx, SIZE, vec![Event::MouseMoved(vec2(0., 2.))]);
        let warped = out
            .viewport_output
            .values()
            .flat_map(|v| v.commands.iter())
            .any(|c| matches!(c, ViewportCommand::CursorPosition(_)));
        assert!(!warped);
        let after = origin_now(&a);
        assert!((before.x - after.x - 11. * PAN_BUTTON_RATE).abs() < 1.);
        assert!((before.y - after.y - 2. * PAN_BUTTON_RATE).abs() < 1.);
    }

    fn commands(out: &egui::FullOutput) -> Vec<ViewportCommand> {
        out.viewport_output
            .values()
            .flat_map(|v| v.commands.iter().cloned())
            .collect()
    }

    #[test]
    fn on_macos_the_pointer_is_locked_and_never_put_back_mid_drag() {
        // VBF-2: a put-back on macOS shows up in the mouse's own movement
        // as a move back, so the view shook in place. Locked: the first move
        // counts from the press, then only the mouse's own movement counts,
        // nothing is put back until the release, which unlocks and returns
        // the pointer to the press.
        let (mut a, ctx, start, before) = pan_press(Vec2::ZERO);
        a.corner.lock_pointer = Some(true);
        let out = draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(start + vec2(10., 0.))],
        );
        let cmds = commands(&out);
        assert!(cmds.contains(&ViewportCommand::CursorGrab(egui::CursorGrab::Locked)));
        assert!(
            !cmds
                .iter()
                .any(|c| matches!(c, ViewportCommand::CursorPosition(_)))
        );
        for _ in 0..4 {
            let out = draw(&mut a, &ctx, SIZE, vec![Event::MouseMoved(vec2(5., 0.))]);
            assert!(
                !commands(&out)
                    .iter()
                    .any(|c| matches!(c, ViewportCommand::CursorPosition(_))),
                "no put-back while locked"
            );
        }
        let after = origin_now(&a);
        assert!(
            (before.x - after.x - 30. * PAN_BUTTON_RATE).abs() < 1.,
            "{before:?} {after:?}"
        );
        assert!((after.y - before.y).abs() < 0.5);
        let out = draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(start + vec2(10., 0.), false, Modifiers::NONE)],
        );
        let cmds = commands(&out);
        let unlock = cmds
            .iter()
            .position(|c| *c == ViewportCommand::CursorGrab(egui::CursorGrab::None));
        let back = cmds
            .iter()
            .position(|c| *c == ViewportCommand::CursorPosition(start));
        assert!(
            unlock.is_some() && back.is_some() && unlock < back,
            "{cmds:?}"
        );
        assert!(a.corner.drag.is_none());
    }

    #[test]
    fn esc_while_locked_unlocks_and_restores_the_view() {
        let (mut a, ctx, start, _) = pan_press(Vec2::ZERO);
        a.corner.lock_pointer = Some(true);
        let camera = a.camera.clone();
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(start + vec2(10., 0.))],
        );
        draw(&mut a, &ctx, SIZE, vec![Event::MouseMoved(vec2(5., 3.))]);
        let out = draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
        assert!(commands(&out).contains(&ViewportCommand::CursorGrab(egui::CursorGrab::None)));
        assert_eq!(a.camera.target, camera.target);
        assert!(a.corner.drag.is_none());
    }
}
