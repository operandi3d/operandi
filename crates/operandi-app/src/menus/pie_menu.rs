//! Pie menus, stage 1: eight commands around the pointer on a held key.
//!
//! A pie is data: a name (its input-map action, so it is bound, searched and
//! conflict-checked like any shortcut) and a list of entries. Each entry is
//! a registered command with a label, or an empty placeholder. Only the
//! first eight entries are used; they fill the compass slots clockwise from
//! the top (N, NE, E, SE, S, SW, W, NW), and an empty entry leaves its slot
//! empty.
//!
//! Gesture: the pie opens on key-down, centred on the pointer (nudged to stay
//! on screen). Each slot owns a 45° wedge outside a 20 px dead zone, and
//! its button: the pointer over a button highlights it whatever the wedge.
//! Releasing the key runs the highlighted slot, so a pie works hold, point,
//! release with no click. Every pie shares this (one `Open`, one event
//! step), however it is bound. Releasing it in the dead
//! zone before the pointer ever left it is a tap: the pie stays open until a
//! click on a slot (runs it), a click elsewhere or Esc (closes, no action).
//! Running a slot is exactly the command, as its toolbar button or shortcut
//! runs it, with that command's own undo step; the pie itself never is one.
//!
//! Provisional choices are logged in SPECIFICATION_QUESTIONS.md (PM-Q*).
use super::*;
use std::f32::consts::TAU;

/// The built-in pies' input-map actions.
pub const VIEW: &str = "Pie: View Direction";
pub const SELECTION: &str = "Pie: Selection & Transforms";
pub const SHADING: &str = "Pie: Shading Style";
pub const TOGGLES: &str = "Pie: Viewport Toggles";
pub const FALLOFFS: &str = "Pie: Falloffs";
pub const ACTION_CENTERS: &str = "Pie: Action Centers";
pub const MODELING: &str = "Pie: Modeling";
/// Shows or hides the window's bars and the timeline (Jason, 2026-11-27).
pub const PANELS: &str = "Pie: Panels";
/// The Layouts pie (layouts stage 2, UIC-18): its slots are the layouts,
/// so it isn't built in (`Operandi::layouts_pie`).
pub const LAYOUTS: &str = "Pie: Layouts";
pub const ALL: [&str; 8] = [
    VIEW,
    SELECTION,
    SHADING,
    TOGGLES,
    FALLOFFS,
    ACTION_CENTERS,
    MODELING,
    PANELS,
];

/// Slots around a pie.
pub const SLOTS: usize = 8;
/// Pointer distance from the centre that highlights nothing.
pub const DEAD_ZONE: f32 = 20.;
/// A slot's button. Longer labels widen it outward, away from the centre.
const BUTTON: Vec2 = vec2(120., 36.);
/// Button centres, clockwise from the top: a tight five-row cluster (rows
/// 46 px apart, so 10 px between buttons) rather than a wide circle. Top
/// and bottom sit just above and below the centre, the sides level with
/// it, the diagonals tucked in between (Jason, 2026-11-15: "a lot tighter").
const SLOT_AT: [Vec2; SLOTS] = [
    vec2(0., -92.),
    vec2(80., -46.),
    vec2(140., 0.),
    vec2(80., 46.),
    vec2(0., 92.),
    vec2(-80., 46.),
    vec2(-140., 0.),
    vec2(-80., -46.),
];
/// Room each side of a label inside its button, and for a state mark.
const PAD: f32 = 12.;
const MARK_ROOM: f32 = 18.;
/// Clicks farther than this from the centre are "elsewhere".
const REACH: f32 = 230.;
/// Room kept between the pie and the window edge.
const MARGIN: f32 = 4.;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub label: String,
    /// A command name `Operandi::execute` runs.
    pub command: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pie {
    pub name: String,
    /// `None` is an empty placeholder that keeps its slot.
    pub entries: Vec<Option<Entry>>,
}
impl Pie {
    pub fn new(name: &str, entries: &[Option<(&str, &str)>]) -> Self {
        Self {
            name: name.into(),
            entries: entries
                .iter()
                .map(|e| {
                    e.map(|(label, command)| Entry {
                        label: label.into(),
                        command: command.into(),
                    })
                })
                .collect(),
        }
    }
    /// The entry in compass slot `i` (0 = N, clockwise). Entries past the
    /// eighth are never shown.
    pub fn slot(&self, i: usize) -> Option<&Entry> {
        (i < SLOTS)
            .then(|| self.entries.get(i))
            .flatten()
            .and_then(Option::as_ref)
    }
}

pub fn is_pie(action: &str) -> bool {
    ALL.contains(&action) || action == LAYOUTS
}

/// A built-in pie by its action name. Commands Operandi doesn't have yet
/// are empty slots, so the layout stays the same when they arrive.
pub fn built_in(name: &str) -> Option<Pie> {
    let s = |label, command| Some((label, command));
    let entries = match name {
        VIEW => vec![
            s("Top", "View Top"),
            s("Perspective", "View Perspective"),
            s("Right", "View Right"),
            s("Front", "View Front"),
            s("Bottom", "View Bottom"),
            s("Back", "View Back"),
            s("Left", "View Left"),
            s("Camera", "View Camera"),
        ],
        SELECTION => vec![
            s("Edges", "Edges"),
            s("Polygons", "Polygons"),
            s("Items", "Items"),
            s("Move", "Move"),
            s("Rotate", "Rotate"),
            s("Scale", "Scale"),
            s("Transform", "Transform"),
            s("Vertices", "Vertices"),
        ],
        SHADING => vec![
            s("Wire", "Shading Wireframe"),
            s("Shaded", "Shading Shaded"),
            s("Advanced", "Shading Advanced"),
            s("Texture", "Shading Texture"),
            s("Default", "Shading Default"),
            s("Gooch", "Shading Gooch"),
            s("Vertex Map", "Shading Vertex Map"),
            s("Matcap", "Shading Reflection"),
        ],
        TOGGLES => vec![
            s("Lights", "Toggle Lights"),
            s("Cameras", "Toggle Cameras"),
            s("Wireframe", "Toggle Wireframe Overlay"),
            s("Vertices", "Toggle Show Verts"),
            s("Grid", "Toggle Grid"),
            s("Background", "Toggle Background Gradient"),
            None, // Texture locators: Operandi has none.
            None, // Locators: Operandi has no locator items.
        ],
        FALLOFFS => vec![
            s("Clear", "Falloff None"),
            s("Linear", "Falloff Linear"),
            s("Cylinder", "Falloff Cylinder"),
            s("Radial", "Falloff Radial"),
            s("Airbrush", "Falloff Airbrush"),
            s("Screen", "Falloff Screen"),
            s("Element", "Falloff Element"),
            s("Vertex Map", "Falloff Vertex Map"),
        ],
        ACTION_CENTERS => vec![
            s("Automatic", "Action Center Automatic"),
            s("Selection", "Action Center Selection"),
            s("Selection Auto Axis", "Action Center Selection Auto Axis"),
            s("Element", "Action Center Element"),
            s("Screen", "Action Center Screen"),
            s("Origin", "Action Center Origin"),
            s("Local", "Action Center Local"),
            None, // Pivot (VAC-Q04)
        ],
        MODELING => vec![
            s("Extrude", "Extrude"),
            s("Bevel", "Bevel"),
            s("Edge Extend", "Extend"),
            None, // Edge knife
            s("Loop Slice", "Loop Slice"),
            s("Smooth Shift", "Smooth Shift"),
            s("Smooth", "Smooth"),
            None, // Soft drag
        ],
        // Each slot points the way its panel sits in the window; the
        // diagonals are free for more panels later.
        PANELS => vec![
            s("Top Bar", "operandi.bar.collapse bar:top"),
            None,
            s("Right Panel", "operandi.bar.collapse bar:right"),
            None,
            s("Timeline", "operandi.anim.timeline"),
            None,
            s("Left Panel", "operandi.bar.collapse bar:left"),
            None,
        ],
        _ => return None,
    };
    let title = name.strip_prefix("Pie: ").unwrap_or(name);
    let mut pie = Pie::new(title, &entries);
    pie.name = name.into();
    Some(pie)
}

/// Unit direction of slot `i` in screen space (y down): 0 points up.
#[cfg(test)]
pub fn direction(i: usize) -> Vec2 {
    let a = i as f32 * TAU / SLOTS as f32;
    vec2(a.sin(), -a.cos())
}

/// The slot whose wedge holds `offset` (pointer minus centre); `None` in
/// the dead zone.
pub fn slot_at(offset: Vec2) -> Option<usize> {
    if offset.length() <= DEAD_ZONE {
        return None;
    }
    // Clockwise angle from straight up, in slots; each wedge is centred on
    // its direction.
    let turns = offset.x.atan2(-offset.y) / TAU * SLOTS as f32;
    Some((turns.round() as i64).rem_euclid(SLOTS as i64) as usize)
}

/// Slot `i`'s button, `width` wide (at least `BUTTON.x`). A wider button
/// keeps its inner edge and grows away from the centre; the top and bottom
/// ones grow both ways.
pub(super) fn label_rect(center: Pos2, i: usize, width: f32) -> Rect {
    let at = SLOT_AT[i % SLOTS];
    let r = Rect::from_center_size(center + at, BUTTON);
    let grow = (width - BUTTON.x).max(0.);
    if at.x > 0. {
        r.with_max_x(r.max.x + grow)
    } else if at.x < 0. {
        r.with_min_x(r.min.x - grow)
    } else {
        r.expand2(vec2(grow / 2., 0.))
    }
}

/// The button width a label needs, from its text width.
pub fn button_width(text_width: f32, marked: bool) -> f32 {
    let mark = if marked { MARK_ROOM } else { 0. };
    (text_width + 2. * PAD + mark).max(BUTTON.x)
}

/// Half the size of the box every button fits in, around the centre.
fn extent(widths: &[f32; SLOTS]) -> Vec2 {
    (0..SLOTS)
        .map(|i| label_rect(Pos2::ZERO, i, widths[i]))
        .fold(Vec2::ZERO, |e, r| {
            e.max(r.max.to_vec2().abs()).max(r.min.to_vec2().abs())
        })
}

/// The pie's centre for a pointer at `pointer`: the pointer itself, nudged
/// so every slot stays inside `screen`. The pointer isn't moved.
#[cfg(test)]
pub fn place(pointer: Pos2, screen: Rect) -> Pos2 {
    place_sized(pointer, screen, &[BUTTON.x; SLOTS])
}

fn place_sized(pointer: Pos2, screen: Rect, widths: &[f32; SLOTS]) -> Pos2 {
    let half = extent(widths) + Vec2::splat(MARGIN);
    let clamp = |v: f32, lo: f32, hi: f32| {
        if lo > hi {
            (lo + hi) / 2.
        } else {
            v.clamp(lo, hi)
        }
    };
    pos2(
        clamp(pointer.x, screen.left() + half.x, screen.right() - half.x),
        clamp(pointer.y, screen.top() + half.y, screen.bottom() - half.y),
    )
}

/// What the pie does after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    StayOpen,
    Close,
    /// Run slot `i`'s command, then close.
    Run(usize),
}

/// An open pie.
#[derive(Clone, Debug)]
pub struct Open {
    pub pie: Pie,
    pub center: Pos2,
    pub pointer: Pos2,
    /// The key that opened it (both readings, as release events may carry
    /// either); `None` once tapped or when opened from the command line.
    pub key: Option<(Key, Option<Key>)>,
    /// The pointer has been outside the dead zone since opening.
    pub left_dead_zone: bool,
    /// A primary press happened while open (so its release is a click).
    pub pressed: bool,
    /// Each slot's button width (see `button_width`).
    pub widths: [f32; SLOTS],
    /// The opening chord's modifiers (Ctrl of Ctrl+Space). Letting go of
    /// them counts as letting go of the chord, whichever key goes first.
    pub chord_modifiers: Modifiers,
    /// Those modifiers have been seen held since opening.
    pub modifiers_seen: bool,
}
impl Open {
    #[cfg(test)]
    pub fn new(pie: Pie, pointer: Pos2, screen: Rect, key: Option<(Key, Option<Key>)>) -> Self {
        Self::sized(pie, pointer, screen, key, [BUTTON.x; SLOTS])
    }
    /// Open with measured button widths.
    pub fn sized(
        pie: Pie,
        pointer: Pos2,
        screen: Rect,
        key: Option<(Key, Option<Key>)>,
        widths: [f32; SLOTS],
    ) -> Self {
        let center = place_sized(pointer, screen, &widths);
        let mut open = Self {
            pie,
            center,
            pointer,
            key,
            left_dead_zone: false,
            pressed: false,
            widths,
            chord_modifiers: Modifiers::NONE,
            modifiers_seen: false,
        };
        // Nudged from the window edge: the pointer may start outside the
        // dead zone already.
        open.hover(pointer);
        open
    }
    pub fn hover(&mut self, pointer: Pos2) {
        self.pointer = pointer;
        if (pointer - self.center).length() > DEAD_ZONE {
            self.left_dead_zone = true;
        }
    }
    /// Slot `i`'s button on screen.
    pub fn rect(&self, i: usize) -> Rect {
        label_rect(self.center, i, self.widths[i % SLOTS])
    }
    /// The entry's button under `at`, if any.
    fn button_at(&self, at: Pos2) -> Option<usize> {
        (0..SLOTS).find(|&i| self.pie.slot(i).is_some() && self.rect(i).contains(at))
    }
    /// The slot to run: the button under the pointer, else the wedge the
    /// pointer is in, if it holds an entry.
    pub fn highlighted(&self) -> Option<usize> {
        self.button_at(self.pointer)
            .or_else(|| slot_at(self.pointer - self.center).filter(|&i| self.pie.slot(i).is_some()))
    }
    /// The opening key went up. `enabled` says whether a slot can run now.
    pub fn release(&mut self, enabled: impl Fn(usize) -> bool) -> Outcome {
        if self.key.take().is_none() {
            return Outcome::StayOpen;
        }
        if self.button_at(self.pointer).is_none() && slot_at(self.pointer - self.center).is_none() {
            return if self.left_dead_zone {
                Outcome::Close
            } else {
                // A tap: stay open for a click.
                Outcome::StayOpen
            };
        }
        match self.highlighted() {
            Some(i) if enabled(i) => Outcome::Run(i),
            _ => Outcome::Close,
        }
    }
    /// A primary click at `at`: a slot runs; anywhere else closes.
    pub fn click(&mut self, at: Pos2, enabled: impl Fn(usize) -> bool) -> Outcome {
        self.hover(at);
        let slot = self.button_at(at).or_else(|| {
            ((at - self.center).length() <= REACH)
                .then(|| self.highlighted())
                .flatten()
        });
        match slot {
            Some(i) if enabled(i) => Outcome::Run(i),
            _ => Outcome::Close,
        }
    }
}

/// How an entry shows its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Plain,
    /// An on/off toggle; picking flips it.
    Toggle(bool),
    /// One of a set (view, shading, mode, action centre): whether it is the
    /// active one.
    Choice(bool),
}

/// An entry's state now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub enabled: bool,
    pub mark: Mark,
}

fn view_named(name: &str) -> Option<View> {
    Some(match name {
        "top" => View::Top,
        "bottom" => View::Bottom,
        "front" => View::Front,
        "back" => View::Back,
        "right" => View::Right,
        "left" => View::Left,
        "perspective" => View::Perspective,
        _ => return None,
    })
}

fn style_named(name: &str) -> Option<view_display::Style> {
    use view_display::Style;
    Some(match name {
        "wireframe" => Style::Wireframe,
        "shaded" => Style::Shaded,
        "advanced" => Style::Advanced,
        "texture" => Style::Texture,
        "default" => Style::Default,
        "gooch" => Style::Gooch,
        "vertex map" => Style::VertexMap,
        "reflection" => Style::Reflection,
        "solid" => Style::Solid,
        "cel" => Style::Cel,
        _ => return None,
    })
}

/// The Action Center menu's choices (workspace.rs `context_bar`) that set a
/// context directly; Element and Screen wait for a viewport click.
fn center_named(name: &str) -> Option<ActionContext> {
    let world = |center| ActionContext {
        center,
        axis: AxisOrientation::World,
    };
    Some(match name {
        "automatic" => world(CenterMode::Automatic),
        "selection" => ActionContext {
            center: CenterMode::Selection,
            axis: AxisOrientation::Selection,
        },
        "selection auto axis" => world(CenterMode::SelectionAutoAxis),
        "origin" => world(CenterMode::Origin),
        "local" => world(CenterMode::Local),
        _ => return None,
    })
}

/// The Action Center menu's name for `context`, when it is one of the
/// choices that set it directly (a workbench pipe preset puts it back).
pub(super) fn center_name(context: &ActionContext) -> Option<&'static str> {
    [
        "automatic",
        "selection",
        "selection auto axis",
        "origin",
        "local",
    ]
    .into_iter()
    .find(|n| center_named(n).as_ref() == Some(context))
}

fn center_click_named(name: &str) -> Option<action_center_click::CenterClick> {
    match name {
        "element" => Some(action_center_click::CenterClick::Element),
        "screen" => Some(action_center_click::CenterClick::Screen),
        _ => None,
    }
}

/// The selection-mode actions, for their Choice mark.
fn mode_named(name: &str) -> Option<Mode> {
    Some(match name {
        "vertices" => Mode::Vertex,
        "edges" => Mode::Edge,
        "polygons" => Mode::Polygon,
        "items" => Mode::Item,
        _ => return None,
    })
}

impl Operandi {
    /// Whether an entry can run now, and the state it shows.
    pub(super) fn pie_status(&self, command: &str) -> Status {
        let (why, mark) = self.pie_state(command);
        Status {
            enabled: why.is_none(),
            mark,
        }
    }
    /// Why an entry is greyed now (`None` = it can run).
    pub(super) fn pie_unavailable(&self, command: &str) -> Option<String> {
        self.pie_state(command).0
    }
    /// An entry's reason for being greyed (`None` = it can run) and mark.
    fn pie_state(&self, command: &str) -> (Option<String>, Mark) {
        use crate::unavailable::unless;
        let lowered = command.trim().to_lowercase();
        if let Some(rest) = lowered.strip_prefix("view ") {
            if rest == "camera" {
                return (
                    unless(
                        camera_light_ui::view_camera(&self.editor.scene).is_some(),
                        "The scene has no camera",
                    )
                    .map(str::to_owned),
                    Mark::Choice(self.camera.through),
                );
            }
            if let Some(view) = view_named(rest) {
                return (
                    None,
                    Mark::Choice(!self.camera.looks_through() && self.camera.view == view),
                );
            }
        }
        if let Some(style) = lowered.strip_prefix("shading ").and_then(style_named) {
            return (
                None,
                Mark::Choice(self.display.settings.active.style == style),
            );
        }
        if let Some(on) = lowered
            .strip_prefix("toggle ")
            .and_then(|t| self.toggle_state(t))
        {
            return (None, Mark::Toggle(on));
        }
        // The Panels pie: marked while the panel shows.
        if let Some(bar) = lowered
            .strip_prefix("operandi.bar.collapse bar:")
            .and_then(crate::collapse_line::Bar::parse)
        {
            return (None, Mark::Toggle(!self.layouts.is_collapsed(bar)));
        }
        if lowered == "operandi.anim.timeline" {
            return (None, Mark::Toggle(self.timeline.open));
        }
        if let Some(rest) = lowered.strip_prefix("action center ") {
            let current = self.editor.action_context;
            if let Some(context) = center_named(rest) {
                let active = match context.center {
                    // The menu marks these by centre alone.
                    CenterMode::SelectionAutoAxis | CenterMode::Local => {
                        current.center == context.center
                    }
                    _ => current == context,
                };
                let why = if self.field_errors.is_empty() {
                    self.editor
                        .validate_action_context(context)
                        .err()
                        .map(|e| e.to_string())
                } else {
                    Some(crate::unavailable::FIELD_ERRORS.to_owned())
                };
                return (why, Mark::Choice(active));
            }
            if center_click_named(rest).is_some() {
                return (
                    None,
                    Mark::Choice(current.center.label().eq_ignore_ascii_case(rest)),
                );
            }
        }
        if let Some(rest) = lowered.strip_prefix("falloff ") {
            let current = self.editor.spatial_falloff;
            let active = match rest {
                "none" => current.is_none(),
                "linear" => matches!(current, Some(Falloff::Linear(_))),
                "radial" => matches!(current, Some(Falloff::Radial(_))),
                "cylinder" => matches!(current, Some(Falloff::Cylinder(_))),
                "airbrush" => matches!(current, Some(Falloff::Airbrush(_))),
                "screen" => matches!(current, Some(Falloff::Screen(_))),
                "element" => matches!(current, Some(Falloff::Element(_))),
                "vertex map" => matches!(current, Some(Falloff::VertexMap(_))),
                _ => false,
            };
            // As the Falloff menu: a live tool without falloff support
            // must be dropped first.
            let ok = self
                .editor
                .live
                .as_ref()
                .is_none_or(|l| l.tool.card().layers.falloff);
            return (
                unless(ok, "The active tool has no falloff; drop it first").map(str::to_owned),
                Mark::Choice(active),
            );
        }
        // Input-map actions: enabled where their shortcut would run, as
        // the Cmd-click modeling menu does.
        let why = input::ACTIONS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(&lowered))
            .and_then(|(name, _)| {
                input::unavailable_reason(
                    name,
                    self.editor.scene.selection.mode,
                    self.editor_tool_active(),
                )
            });
        match mode_named(&lowered) {
            Some(mode) => (why, Mark::Choice(self.editor.scene.selection.mode == mode)),
            None => (why, Mark::Plain),
        }
    }

    /// Whether Item mode's plain wire decides what the wireframe toggle
    /// does: Item mode, and a shading style other than Wireframe (which is
    /// its own wire). `ViewDisplay::effective` has the same rule (VS-6).
    fn item_mode_wire(&self) -> bool {
        self.editor.scene.selection.mode == Mode::Item
            && self.display.settings.active.style != view_display::Style::Wireframe
    }

    /// A viewport toggle's state in the active view; `None`: not a toggle.
    fn toggle_state(&self, name: &str) -> Option<bool> {
        let s = &self.display.settings;
        let d = &s.drawing;
        Some(match name {
            "lights" => d.lights,
            "cameras" => d.cameras,
            // #22: Item mode's wire follows Show Wireframe in Item Mode
            // (VS-6), so there the toggle shows and flips that.
            "wireframe overlay" if self.item_mode_wire() => d.item_mode_wireframe,
            "wireframe overlay" => s.active.overlay != view_display::Overlay::None,
            "show verts" => d.show_verts,
            "grid" => d.grid,
            "background gradient" => d.background == view_display::Background::Gradient,
            _ => return None,
        })
    }

    /// Run the pie-only commands (view direction, shading, toggles, action
    /// centres) and open pies by name. None of these is an undo step except
    /// an action-centre change, which follows `Command::ActionContext`.
    /// Returns false for other commands.
    pub(super) fn execute_pie_command(&mut self, lowered: &str) -> bool {
        if let Some(&name) = ALL
            .iter()
            .chain(&[LAYOUTS])
            .find(|n| n.eq_ignore_ascii_case(lowered))
        {
            // From the command line: opens tapped, waiting for a click.
            self.pie_request = Some(name.into());
            return true;
        }
        if let Some(rest) = lowered.strip_prefix("view ") {
            if rest == "camera" {
                if camera_light_ui::view_camera(&self.editor.scene).is_none() {
                    self.status = "The scene has no camera".into();
                } else {
                    self.choose_view_type(None);
                }
                return true;
            }
            if let Some(view) = view_named(rest) {
                self.choose_view_type(Some(view));
                return true;
            }
        }
        if let Some(style) = lowered.strip_prefix("shading ").and_then(style_named) {
            self.display.set_style(style, false);
            self.status = format!("Active meshes: {}", style.label());
            return true;
        }
        if let Some(rest) = lowered.strip_prefix("toggle ")
            && let Some(on) = self.toggle_state(rest)
        {
            let item_wire = self.item_mode_wire();
            let s = &mut self.display.settings;
            let d = &mut s.drawing;
            match rest {
                "lights" => d.lights = !on,
                "cameras" => d.cameras = !on,
                "wireframe overlay" if item_wire => d.item_mode_wireframe = !on,
                "wireframe overlay" => {
                    s.active.overlay = if on {
                        view_display::Overlay::None
                    } else {
                        view_display::Overlay::default()
                    }
                }
                "show verts" => d.show_verts = !on,
                "grid" => d.grid = !on,
                _ => {
                    d.background = if on {
                        view_display::Background::None
                    } else {
                        view_display::Background::Gradient
                    }
                }
            }
            return true;
        }
        if let Some(rest) = lowered.strip_prefix("action center ") {
            if let Some(context) = center_named(rest) {
                if !self.field_errors.is_empty() {
                    self.status =
                        "Correct or revert invalid numeric fields before changing Action Center"
                            .into();
                } else {
                    self.run(Command::ActionContext(context));
                }
                return true;
            }
            if let Some(kind) = center_click_named(rest) {
                self.choose_center_click(kind);
                return true;
            }
        }
        false
    }

    /// A pie by its action name: built in, or the Layouts pie.
    pub(super) fn pie_named(&self, name: &str) -> Option<Pie> {
        if name == LAYOUTS {
            return Some(self.layouts_pie());
        }
        built_in(name)
    }

    fn pie_enabled(&self, open: &Open) -> [bool; SLOTS] {
        std::array::from_fn(|i| {
            open.pie
                .slot(i)
                .is_some_and(|e| self.pie_status(&e.command).enabled)
        })
    }

    /// Open `pie` at the pointer. `key`: the held key, or `None` to open
    /// tapped. Closes any other popup.
    pub(super) fn open_pie(&mut self, ctx: &Context, pie: Pie, key: Option<(Key, Option<Key>)>) {
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        self.open_pie_at(ctx, pie, key, pointer);
    }

    /// `open_pie` centred on `pointer` (the window centre when unknown).
    fn open_pie_at(
        &mut self,
        ctx: &Context,
        pie: Pie,
        key: Option<(Key, Option<Key>)>,
        pointer: Option<Pos2>,
    ) {
        egui::Popup::close_all(ctx);
        if !self.corner.properties.pinned {
            self.close_view_properties();
        }
        let screen = ctx.content_rect();
        let pointer = pointer.unwrap_or(screen.center());
        // Buttons fit their labels (and a state mark), at the font paint uses.
        let widths: [f32; SLOTS] = std::array::from_fn(|i| {
            pie.slot(i).map_or(BUTTON.x, |e| {
                let text = ctx.fonts_mut(|f| {
                    f.layout_no_wrap(e.label.clone(), label_font(), Color32::WHITE)
                        .size()
                        .x
                });
                button_width(text, self.pie_status(&e.command).mark != Mark::Plain)
            })
        });
        self.pie = Some(Open::sized(pie, pointer, screen, key, widths));
        self.menu_owned_input = true;
        ctx.request_repaint();
    }

    /// A pie's key pressed: open it. True when the key was taken (opened,
    /// or refused for a conflicting binding).
    pub(super) fn pie_keys(&mut self, ctx: &Context) -> bool {
        if self.has_modal()
            || self.text_editor_focused(ctx)
            || native_menu::tracking()
            || self.pending.is_some()
            || self.show_bindings
            || self.recovery.is_some()
            || self.recovery_warning.is_some()
            || self.show_plane
            || self.show_diagnostics
            || self.show_about
            || ctx.input(|i| i.pointer.any_down())
        {
            return false;
        }
        let mode = self.editor.scene.selection.mode;
        let live = self.editor_tool_active() || self.edge_click.is_some() || self.vertex_weld;
        let mut found = None;
        ctx.input(|i| {
            for (at, event) in i.events.iter().enumerate() {
                let Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                } = event
                else {
                    continue;
                };
                let digit = if *key == Key::Period {
                    Some(10)
                } else {
                    key.name().parse::<u8>().ok().filter(|&d| d <= 9)
                };
                let location = digit.and_then(|digit| {
                    self.native_numbers
                        .iter()
                        .find(|n| n.digit == digit && n.pressed)
                        .map(|n| n.keypad)
                });
                let matching: Vec<&String> = self
                    .bindings
                    .iter()
                    .filter(|(action, binding)| {
                        (digit.is_none() || location == Some(binding.contains("Numpad")))
                            && input::matches_chord(binding, *key, *modifiers)
                            && input::available(action, mode, live)
                    })
                    .map(|(action, _)| action)
                    .collect();
                if let Some(pie) = matching.iter().find(|a| is_pie(a)) {
                    found = Some((
                        (*pie).clone(),
                        matching.len() > 1,
                        (*key, *physical_key),
                        at,
                        *modifiers,
                    ));
                    break;
                }
            }
        });
        let Some((name, conflict, key, at, chord_modifiers)) = found else {
            return false;
        };
        if conflict {
            self.status =
                "Input map conflict: assign unique keys before invoking this shortcut".into();
            return true;
        }
        if let Some(pie) = self.pie_named(&name) {
            // The View and Shading pies act on the view they open over.
            if crate::viewport_buttons::hovered_view_pie(&name) && self.pointer_over_a_view(ctx) {
                self.activate_view(self.view_under_pointer(ctx));
            }
            // Centred where the pointer was when the key went down, not
            // where this frame's later moves left it.
            let pointer = ctx.input(|i| {
                i.events[..at]
                    .iter()
                    .rev()
                    .find_map(|e| match e {
                        Event::PointerMoved(p) => Some(*p),
                        _ => None,
                    })
                    .or_else(|| {
                        let moved_after = i.events[at..]
                            .iter()
                            .any(|e| matches!(e, Event::PointerMoved(_)));
                        let latest = i.pointer.latest_pos()?;
                        Some(if moved_after {
                            latest - i.pointer.delta()
                        } else {
                            latest
                        })
                    })
            });
            self.open_pie_at(ctx, pie, Some((key.0, key.1)), pointer);
            // A quick hold, point and release can land in one frame (a busy
            // frame batches the events): the rest of this frame goes to the
            // new pie, so the release still runs the slot.
            if let Some(mut open) = self.pie.take() {
                open.chord_modifiers = chord_modifiers;
                let enabled = self.pie_enabled(&open);
                let outcome =
                    ctx.input(|i| run_events(&mut open, &i.events[at + 1..], i, &enabled));
                self.finish_pie(ctx, open, outcome);
            }
        }
        true
    }

    /// While a pie is open it owns keys and clicks. True when one is open.
    pub(super) fn pie_input(&mut self, ctx: &Context) -> bool {
        let Some(mut open) = self.pie.take() else {
            return false;
        };
        let enabled = self.pie_enabled(&open);
        let outcome = ctx.input(|i| {
            if let Some(p) = i.pointer.latest_pos() {
                open.hover(p);
            }
            run_events(&mut open, &i.events, i, &enabled)
        });
        self.finish_pie(ctx, open, outcome);
        true
    }

    /// Apply an outcome to the pie just stepped; its keys reach nothing else.
    fn finish_pie(&mut self, ctx: &Context, open: Open, outcome: Outcome) {
        ctx.input_mut(|i| {
            i.events
                .retain(|e| !matches!(e, Event::Key { .. } | Event::Text(_)));
        });
        match outcome {
            Outcome::StayOpen => self.pie = Some(open),
            Outcome::Close => {}
            Outcome::Run(i) => {
                if let Some(entry) = open.pie.slot(i) {
                    self.execute_from(operandi_core::Source::Pie, &entry.command);
                }
            }
        }
        ctx.request_repaint();
    }

    /// Draw the open pie (and open one asked for by name).
    pub(super) fn pie_window(&mut self, ctx: &Context) {
        if let Some(name) = self.pie_request.take()
            && let Some(pie) = self.pie_named(&name)
        {
            self.open_pie(ctx, pie, None);
        }
        let Some(open) = &self.pie else {
            return;
        };
        let statuses: Vec<Option<Status>> = (0..SLOTS)
            .map(|i| open.pie.slot(i).map(|e| self.pie_status(&e.command)))
            .collect();
        // A greyed slot under the pointer says why (unavailable.rs).
        let why = open
            .highlighted()
            .and_then(|i| open.pie.slot(i))
            .and_then(|e| self.pie_unavailable(&e.command));
        let open = open.clone();
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("pie-menu"))
            .order(Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                // Covers the window so nothing under the pie takes the
                // pointer while it is open.
                ui.allocate_rect(screen, Sense::click_and_drag());
                paint(ui.painter(), &open, &statuses, why.as_deref());
            });
    }
}

/// Step an open pie through one frame's `events`. The frame's key state
/// backs up the opening key's key-up event, so letting go of the chord
/// always counts: the key no longer down (its key-up taken elsewhere or
/// never sent), or the chord's modifiers let go first (Ctrl before Space).
fn run_events(
    open: &mut Open,
    events: &[Event],
    input: &egui::InputState,
    enabled: &[bool; SLOTS],
) -> Outcome {
    for event in events {
        let next = step(open, event, enabled);
        if next != Outcome::StayOpen {
            return next;
        }
    }
    let Some((k, _)) = open.key else {
        return Outcome::StayOpen;
    };
    if !input.keys_down.contains(&k) {
        return open.release(|s| enabled[s]);
    }
    if !open.chord_modifiers.is_none() {
        if input.modifiers.contains(open.chord_modifiers) {
            open.modifiers_seen = true;
        } else if open.modifiers_seen {
            return open.release(|s| enabled[s]);
        }
    }
    Outcome::StayOpen
}

/// One event for an open pie: pointer moves highlight, the opening key's
/// release runs the highlighted slot, clicks run or close, Esc closes.
fn step(open: &mut Open, event: &Event, enabled: &[bool; SLOTS]) -> Outcome {
    match event {
        Event::PointerMoved(p) => {
            open.hover(*p);
            Outcome::StayOpen
        }
        Event::Key {
            key: Key::Escape,
            pressed: true,
            ..
        } => Outcome::Close,
        Event::Key {
            key,
            physical_key,
            pressed: false,
            ..
        } if open.key.is_some_and(|(k, physical)| {
            *key == k || physical.is_some_and(|p| Some(p) == *physical_key)
        }) =>
        {
            open.release(|s| enabled[s])
        }
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            ..
        } => {
            if *pressed {
                open.pressed = true;
                open.hover(*pos);
                Outcome::StayOpen
            } else if open.pressed {
                open.click(*pos, |s| enabled[s])
            } else {
                Outcome::StayOpen
            }
        }
        Event::PointerButton { pressed: true, .. } | Event::WindowFocused(false) => Outcome::Close,
        _ => Outcome::StayOpen,
    }
}

/// The font pie labels are drawn (and measured) in.
fn label_font() -> FontId {
    FontId::proportional(13.)
}

/// Paint a pie: dead-zone ring, arrow toward the highlighted slot, labels.
fn paint(painter: &Painter, open: &Open, statuses: &[Option<Status>], why: Option<&str>) {
    let c = open.center;
    let fill = theme::popup_fill();
    // A small solid dot marks the centre (the dead zone around it is
    // invisible); the arrow beside it points at the highlighted button.
    painter.circle(c, 5., theme::text(), Stroke::new(1.0f32, theme::border()));
    let highlighted = open.highlighted();
    if let Some(i) = highlighted {
        let d = (open.rect(i).center() - c).normalized();
        let n = vec2(-d.y, d.x);
        let tip = c + d * 17.;
        let base = c + d * 9.;
        painter.add(Shape::convex_polygon(
            vec![tip, base + n * 5., base - n * 5.],
            theme::orange(),
            Stroke::NONE,
        ));
    }
    let font = label_font();
    for (i, status) in statuses.iter().enumerate() {
        let (Some(entry), Some(status)) = (open.pie.slot(i), *status) else {
            continue;
        };
        let r = open.rect(i);
        let lit = highlighted == Some(i);
        let (bg, text) = match (lit, status.enabled) {
            (true, true) => (theme::orange(), theme::on_orange()),
            (true, false) => (theme::hover(), theme::disabled_text()),
            (false, true) => (fill, theme::text()),
            (false, false) => (fill, theme::disabled_text()),
        };
        painter.rect(
            r,
            6.,
            bg,
            Stroke::new(1.0f32, theme::border()),
            StrokeKind::Inside,
        );
        // State marker at the left: a box for toggles, a dot for choices.
        let m = pos2(r.left() + PAD + 4., r.center().y);
        match status.mark {
            Mark::Plain => {}
            Mark::Toggle(on) => {
                let b = Rect::from_center_size(m, Vec2::splat(9.));
                painter.rect_stroke(b, 1., Stroke::new(1.0f32, text), StrokeKind::Inside);
                if on {
                    painter.rect_filled(b.shrink(2.), 0., text);
                }
            }
            Mark::Choice(on) => {
                painter.circle_stroke(m, 4.5, Stroke::new(1.0f32, text));
                if on {
                    painter.circle_filled(m, 2.5, text);
                }
            }
        }
        // Centred, in the room right of the mark when there is one.
        let room = match status.mark {
            Mark::Plain => r,
            _ => r.with_min_x(r.left() + MARK_ROOM),
        };
        painter.text(
            room.center(),
            Align2::CENTER_CENTER,
            &entry.label,
            font.clone(),
            text,
        );
    }
    // Under the greyed slot the pointer is on: why it can't run.
    // Under the whole cluster, so it never covers another button.
    if let (Some(why), Some(_)) = (why, open.highlighted()) {
        let r = open.rect(4);
        let galley = painter.layout(
            why.to_owned(),
            FontId::proportional(11.),
            theme::text(),
            220.,
        );
        let at = pos2(r.center().x - galley.size().x / 2., r.bottom() + 4.);
        let back = Rect::from_min_size(at, galley.size()).expand(4.);
        painter.rect(
            back,
            4.,
            theme::popup_fill(),
            Stroke::new(1.0f32, theme::border()),
            StrokeKind::Inside,
        );
        painter.galley(at, galley, theme::text());
    }
}
