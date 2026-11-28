//! Pie menus: eight commands around the pointer on a held key.
//!
//! A pie is a form with style pie (UIC-308, `pie_forms.rs`): the shipped
//! ones are files in `assets/forms` (`pie.*.json`), a user's pies and
//! changed copies of shipped ones layer over them in the Forms folder and
//! can be reverted (UIC-216). Its first eight controls fill the compass
//! slots clockwise from the top (N, NE, E, SE, S, SW, W, NW): a command
//! line, a sub-pie (UIC-310), or an empty slot that keeps its place
//! (UIC-309). A pie opens through the line `operandi.pie.open <id>`, which
//! the Input Map binds like any action (UIC-311). The Layouts pie is the
//! one pie still made at run time (its slots are the layouts), opened
//! through the same engine.
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
//! A sub-pie slot opens its pie in place, at the same centre (UIC-310);
//! Esc, or a click back in the centre, goes back one level, and a pie never
//! opens a pie it is already inside. The pie's name sits in the centre
//! (UIC-313).
//!
//! Provisional choices are logged in SPECIFICATION_QUESTIONS.md (PM-Q*).
use super::*;
use crate::form::command_form::IconMode;
use std::f32::consts::TAU;

/// The shipped pies' ids (their form files are `assets/forms/<id>.json`).
pub const VIEW: &str = "pie.view";
pub const SELECTION: &str = "pie.selection";
pub const SHADING: &str = "pie.shading";
pub const TOGGLES: &str = "pie.toggles";
pub const FALLOFFS: &str = "pie.falloffs";
pub const ACTION_CENTERS: &str = "pie.actionCenters";
pub const MODELING: &str = "pie.modeling";
/// The Layouts pie (layouts stage 2, UIC-18): its slots are the layouts,
/// so it has no file (`Operandi::layouts_pie`).
pub const LAYOUTS: &str = "pie.layouts";
#[cfg(test)]
pub const ALL: [&str; 7] = [
    VIEW,
    SELECTION,
    SHADING,
    TOGGLES,
    FALLOFFS,
    ACTION_CENTERS,
    MODELING,
];

/// The command that opens a pie by id (UIC-311).
pub const OPEN: &str = "operandi.pie.open";

/// The Input Map lines that open the shipped pies (`input::ACTIONS`).
pub const VIEW_LINE: &str = "operandi.pie.open pie.view";
pub const SELECTION_LINE: &str = "operandi.pie.open pie.selection";
pub const SHADING_LINE: &str = "operandi.pie.open pie.shading";
pub const TOGGLES_LINE: &str = "operandi.pie.open pie.toggles";
pub const FALLOFFS_LINE: &str = "operandi.pie.open pie.falloffs";
pub const ACTION_CENTERS_LINE: &str = "operandi.pie.open pie.actionCenters";
pub const MODELING_LINE: &str = "operandi.pie.open pie.modeling";
pub const LAYOUTS_LINE: &str = "operandi.pie.open pie.layouts";

/// The shipped pies before they were forms: their old Input Map action
/// (still a command, and what a saved input map or layout key names),
/// their id and their Input Map line.
pub const OLD_ACTIONS: [(&str, &str, &str); 8] = [
    ("Pie: View Direction", VIEW, VIEW_LINE),
    ("Pie: Selection & Transforms", SELECTION, SELECTION_LINE),
    ("Pie: Shading Style", SHADING, SHADING_LINE),
    ("Pie: Viewport Toggles", TOGGLES, TOGGLES_LINE),
    ("Pie: Falloffs", FALLOFFS, FALLOFFS_LINE),
    ("Pie: Action Centers", ACTION_CENTERS, ACTION_CENTERS_LINE),
    ("Pie: Modeling", MODELING, MODELING_LINE),
    ("Pie: Layouts", LAYOUTS, LAYOUTS_LINE),
];

/// The line that opens pie `id`.
pub fn line(id: &str) -> String {
    format!("{OPEN} {id}")
}

/// The pie an input-map action or command line opens: `operandi.pie.open
/// <id>` (or `id:<id>`), or a shipped pie's old action name.
pub fn pie_id(action: &str) -> Option<String> {
    let action = action.trim();
    if let Some((_, id, _)) = OLD_ACTIONS
        .iter()
        .find(|(old, _, _)| old.eq_ignore_ascii_case(action))
    {
        return Some((*id).to_owned());
    }
    let (verb, rest) = action.split_once(char::is_whitespace)?;
    if !verb.eq_ignore_ascii_case(OPEN) {
        return None;
    }
    let rest = rest.trim();
    let rest = rest
        .strip_prefix("id:")
        .or_else(|| rest.strip_prefix("ID:"))
        .unwrap_or(rest)
        .trim()
        .trim_matches(|c| matches!(c, '"' | '{' | '}'));
    (!rest.is_empty() && !rest.contains(char::is_whitespace)).then(|| rest.to_owned())
}

/// Rename old pie actions in a saved key map to their lines, keeping a
/// binding the map already has under the new name.
pub fn migrate_keys(keys: &mut BTreeMap<String, String>) {
    for (old, _, line) in OLD_ACTIONS {
        if let Some(chord) = keys.remove(old) {
            keys.entry(line.to_owned()).or_insert(chord);
        }
    }
}

/// What the Input Map and the guide call a shipped pie's line: its old
/// action name ("Pie: View Direction"); `None` for other actions.
pub fn shipped_title(action: &str) -> Option<&'static str> {
    OLD_ACTIONS
        .iter()
        .find(|(_, _, line)| *line == action)
        .map(|(old, _, _)| *old)
}

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
/// A slot's icon, and the gap after it (UIC-309).
const ICON: f32 = 20.;
const ICON_GAP: f32 = 4.;
/// Room at a sub-pie slot's outer end for its marker (UIC-310).
const SUB_ROOM: f32 = 12.;
/// The widest the pie's name in the centre gets (UIC-313); longer names
/// end in "…". The sides' buttons start 80 px out, so the name and the
/// pointer arrow beside it stay clear of them.
const NAME_WIDTH: f32 = 130.;
/// Clicks farther than this from the centre are "elsewhere".
const REACH: f32 = 230.;
/// Room kept between the pie and the window edge.
const MARGIN: f32 = 4.;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub label: String,
    /// A command line `Operandi::execute` runs; for a sub-pie, the line
    /// that opens it.
    pub command: String,
    /// An icon slug (`icons::Icon::from_slug`) drawn per the pie's icon
    /// mode.
    pub icon: Option<String>,
    /// A custom icon file, worn instead of `icon` (UIC-314).
    pub icon_file: Option<String>,
    /// The sub-pie this slot opens in place (UIC-310), by id.
    pub sub: Option<String>,
}

impl Entry {
    pub fn command(label: &str, command: &str) -> Self {
        Self {
            label: label.into(),
            command: command.into(),
            icon: None,
            icon_file: None,
            sub: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pie {
    /// The pie's id (its form's).
    pub name: String,
    /// What the centre shows (UIC-313): the form's label.
    pub title: String,
    /// Text, icon or both (UIC-309).
    pub icons: IconMode,
    /// `None` is an empty placeholder that keeps its slot.
    pub entries: Vec<Option<Entry>>,
}
impl Pie {
    pub fn new(id: &str, title: &str, entries: &[Option<(&str, &str)>]) -> Self {
        Self {
            name: id.into(),
            title: title.into(),
            icons: IconMode::Text,
            entries: entries
                .iter()
                .map(|e| e.map(|(label, command)| Entry::command(label, command)))
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
    /// Whether slot `i` shows its icon, and its label (UIC-309). An icon
    /// mode asking for an icon the slot hasn't got shows the label. A
    /// custom file wins over a built-in icon (UIC-314).
    pub fn shows(&self, i: usize) -> (Option<SlotIcon>, bool) {
        let Some(e) = self.slot(i) else {
            return (None, false);
        };
        let icon = e.icon_file.clone().map(SlotIcon::File).or_else(|| {
            e.icon
                .as_deref()
                .and_then(icons::Icon::from_slug)
                .map(SlotIcon::Builtin)
        });
        match (self.icons, icon) {
            (IconMode::Text, _) | (_, None) => (None, true),
            (IconMode::Icon, Some(i)) => (Some(i), false),
            (IconMode::Both, Some(i)) => (Some(i), true),
        }
    }
}

/// The icon a slot wears (UIC-314).
#[derive(Clone, Debug, PartialEq)]
pub enum SlotIcon {
    Builtin(icons::Icon),
    /// A custom picture's path; a missing file draws a placeholder.
    File(String),
}

pub fn is_pie(action: &str) -> bool {
    pie_id(action).is_some()
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

/// A slot's button width: its label (when shown), its icon, a state mark
/// and a sub-pie marker each take room (UIC-309, UIC-310).
pub fn slot_width(text_width: Option<f32>, icon: bool, marked: bool, sub: bool) -> f32 {
    let icon = match (icon, text_width.is_some()) {
        (true, true) => ICON + ICON_GAP,
        (true, false) => ICON,
        (false, _) => 0.,
    };
    let sub = if sub { SUB_ROOM } else { 0. };
    button_width(text_width.unwrap_or(0.) + icon + sub, marked)
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
    /// Run slot `i`'s command, then close; a sub-pie slot opens its pie
    /// in place instead (UIC-310).
    Run(usize),
    /// Back from a sub-pie to the pie that opened it (UIC-310).
    Back,
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
    /// The pies this sub-pie was opened from, outermost first (UIC-310);
    /// empty for the pie the key or line opened.
    pub stack: Vec<Pie>,
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
            stack: Vec::new(),
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
    /// Open `sub` in place of the pie showing, at the same centre
    /// (UIC-310), with its own button `widths`. It waits for a click: the
    /// key that opened the outer pie has gone up.
    pub fn enter(&mut self, sub: Pie, widths: [f32; SLOTS]) {
        let outer = std::mem::replace(&mut self.pie, sub);
        self.stack.push(outer);
        self.widths = widths;
        self.key = None;
        self.pressed = false;
    }
    /// Back one level (UIC-310): the pie this one was opened from, with
    /// its `widths`. False when this is the outermost pie.
    pub fn back(&mut self, widths: impl FnOnce(&Pie) -> [f32; SLOTS]) -> bool {
        let Some(outer) = self.stack.pop() else {
            return false;
        };
        self.widths = widths(&outer);
        self.pie = outer;
        self.pressed = false;
        true
    }
    /// The pie ids open now, outermost first: a sub-pie may not open one
    /// of these (UIC-310: a pie never contains itself).
    pub fn open_ids(&self) -> impl Iterator<Item = &str> {
        self.stack
            .iter()
            .chain(std::iter::once(&self.pie))
            .map(|p| p.name.as_str())
    }
    /// A primary click at `at`: a slot runs; in a sub-pie a click in the
    /// centre goes back one level (UIC-310); anywhere else closes.
    pub fn click(&mut self, at: Pos2, enabled: impl Fn(usize) -> bool) -> Outcome {
        self.hover(at);
        if !self.stack.is_empty() && (at - self.center).length() <= DEAD_ZONE {
            return Outcome::Back;
        }
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
    /// No command has the slot's line: drawn red, does nothing (UIC-221).
    pub unknown: bool,
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
        "local" => ActionContext::LOCAL,
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

/// The Action Centers that aren't one fixed pair: Element takes its pick
/// and Screen its place from the view (`action_center_click.rs`).
fn center_from_view(name: &str) -> bool {
    // Match arms, so `pie_phrase_choices_match_the_registry` reads the
    // names.
    #[allow(clippy::match_like_matches_macro)]
    match name {
        "element" | "screen" => true,
        _ => false,
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
        let unknown = self.line_state(command).is_none();
        let (why, mark) = self.pie_state(command);
        Status {
            enabled: why.is_none() && !unknown,
            mark,
            unknown,
        }
    }
    /// A slot's state in the pies open now (`open`: their ids). A sub-pie
    /// slot can open its pie unless that pie is missing or already open
    /// (UIC-310).
    pub(super) fn slot_status<'a>(
        &self,
        entry: &Entry,
        mut open: impl Iterator<Item = &'a str>,
    ) -> Status {
        match &entry.sub {
            Some(sub) => Status {
                enabled: self.pie_named(sub).is_some() && !open.any(|id| id == sub),
                mark: Mark::Plain,
                unknown: self.pie_named(sub).is_none(),
            },
            None => self.pie_status(&entry.command),
        }
    }
    /// Why an entry is greyed now (`None` = it can run).
    pub(super) fn pie_unavailable(&self, command: &str) -> Option<String> {
        if self.line_state(command).is_none() {
            return Some(format!(
                "Unknown command: {}. This slot does nothing until a command has that name.",
                command.trim()
            ));
        }
        self.pie_state(command).0
    }
    /// Why a slot is greyed in the pies open now (`open`: their ids).
    fn slot_unavailable<'a>(
        &self,
        entry: &Entry,
        mut open: impl Iterator<Item = &'a str>,
    ) -> Option<String> {
        match &entry.sub {
            Some(sub) if self.pie_named(sub).is_none() => Some(format!("No pie called {sub}")),
            Some(sub) if open.any(|id| id == sub) => {
                Some(format!("{sub} is already open: a pie can't contain itself"))
            }
            Some(_) => None,
            None => self.pie_unavailable(&entry.command),
        }
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
        if let Some(rest) = lowered.strip_prefix("action center ") {
            let current = self.editor.action_context;
            if let Some(context) = center_named(rest) {
                let active = match context.center {
                    // The menu marks these by centre alone.
                    CenterMode::SelectionAutoAxis => current.center == context.center,
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
            if center_from_view(rest) {
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
                .session
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
        let action = input::ACTIONS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(&lowered));
        if action.is_none() && mode_named(&lowered).is_none() {
            // Any other registry line (UIC-222): greyed and lit from its
            // own answer (UIC-223); a lit one shows a filled dot.
            return match self.line_state(command) {
                Some(state) => (
                    state.unavailable,
                    if state.on {
                        Mark::Choice(true)
                    } else {
                        Mark::Plain
                    },
                ),
                None => (None, Mark::Plain),
            };
        }
        let why = action.and_then(|(name, _)| {
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
        // The shipped pies' old action names ("Pie: View Direction") still
        // open them; from the command line a pie opens tapped, waiting for
        // a click.
        if let Some((_, id, _)) = OLD_ACTIONS
            .iter()
            .find(|(old, _, _)| old.eq_ignore_ascii_case(lowered))
        {
            self.pie_request = Some((*id).into());
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
                    self.choose_action_context(context);
                }
                return true;
            }
            if center_from_view(rest) {
                if !self.field_errors.is_empty() {
                    self.status =
                        "Correct or revert invalid numeric fields before changing Action Center"
                            .into();
                } else if rest == "element" {
                    self.choose_element_center();
                } else {
                    self.choose_screen_center();
                }
                return true;
            }
        }
        false
    }

    fn pie_enabled(&self, open: &Open) -> [bool; SLOTS] {
        std::array::from_fn(|i| {
            open.pie
                .slot(i)
                .is_some_and(|e| self.slot_status(e, open.open_ids()).enabled)
        })
    }

    /// Each slot's button width for `pie`, at the font paint uses: its
    /// label, icon, state mark and sub-pie marker (UIC-309, UIC-310).
    fn pie_widths(&self, ctx: &Context, pie: &Pie) -> [f32; SLOTS] {
        std::array::from_fn(|i| {
            pie.slot(i).map_or(BUTTON.x, |e| {
                let (icon, label) = pie.shows(i);
                let text = label.then(|| {
                    ctx.fonts_mut(|f| {
                        f.layout_no_wrap(e.label.clone(), label_font(), Color32::WHITE)
                            .size()
                            .x
                    })
                });
                let marked = e.sub.is_none() && self.pie_status(&e.command).mark != Mark::Plain;
                slot_width(text, icon.is_some(), marked, e.sub.is_some())
            })
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
        let widths = self.pie_widths(ctx, &pie);
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
        let Some((action, conflict, key, at, chord_modifiers)) = found else {
            return false;
        };
        if conflict {
            self.status =
                "Input map conflict: assign unique keys before invoking this shortcut".into();
            return true;
        }
        let id = pie_id(&action).unwrap_or_default();
        let Some(pie) = self.pie_named(&id) else {
            self.status = format!("No pie called {id}");
            return true;
        };
        // The View and Shading pies act on the view they open over.
        if crate::viewport_buttons::hovered_view_pie(&id) && self.pointer_over_a_view(ctx) {
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
            let outcome = ctx.input(|i| run_events(&mut open, &i.events[at + 1..], i, &enabled));
            self.finish_pie(ctx, open, outcome);
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
    fn finish_pie(&mut self, ctx: &Context, mut open: Open, outcome: Outcome) {
        ctx.input_mut(|i| {
            i.events
                .retain(|e| !matches!(e, Event::Key { .. } | Event::Text(_)));
        });
        match outcome {
            Outcome::StayOpen => self.pie = Some(open),
            Outcome::Close => {}
            Outcome::Back => {
                open.back(|outer| self.pie_widths(ctx, outer));
                self.pie = Some(open);
            }
            Outcome::Run(i) => match open.pie.slot(i).cloned() {
                // UIC-310: a sub-pie opens in place, at the same centre.
                Some(Entry { sub: Some(sub), .. }) => {
                    if let Some(why) = open
                        .pie
                        .slot(i)
                        .and_then(|e| self.slot_unavailable(e, open.open_ids()))
                    {
                        self.status = why;
                    } else if let Some(pie) = self.pie_named(&sub) {
                        let widths = self.pie_widths(ctx, &pie);
                        open.enter(pie, widths);
                    }
                    self.pie = Some(open);
                }
                Some(entry) => self.execute_from(operandi_core::Source::Pie, &entry.command),
                None => {}
            },
        }
        ctx.request_repaint();
    }

    /// Draw the open pie (and open one asked for by id).
    pub(super) fn pie_window(&mut self, ctx: &Context) {
        self.pie_import_dialog(ctx);
        if let Some(id) = self.pie_request.take() {
            match self.pie_named(&id) {
                Some(pie) => self.open_pie(ctx, pie, None),
                None => self.status = format!("No pie called {id}"),
            }
        }
        let Some(open) = &self.pie else {
            return;
        };
        let statuses: Vec<Option<Status>> = (0..SLOTS)
            .map(|i| {
                open.pie
                    .slot(i)
                    .map(|e| self.slot_status(e, open.open_ids()))
            })
            .collect();
        // A greyed slot under the pointer says why (unavailable.rs).
        let why = open
            .highlighted()
            .and_then(|i| open.pie.slot(i))
            .and_then(|e| self.slot_unavailable(e, open.open_ids()));
        let open = open.clone();
        let pictures = self.slot_pictures(ctx, &open.pie);
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("pie-menu"))
            .order(Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                // Covers the window so nothing under the pie takes the
                // pointer while it is open.
                ui.allocate_rect(screen, Sense::click_and_drag());
                paint(ui, &open, &statuses, &pictures, why.as_deref());
            });
    }

    /// Each slot's custom icon picture, loaded once (UIC-314); `None` for
    /// a slot without one or whose file is missing (a placeholder shows).
    pub(crate) fn slot_pictures(&mut self, ctx: &Context, pie: &Pie) -> [Option<TextureId>; SLOTS] {
        std::array::from_fn(|i| match pie.shows(i).0 {
            Some(SlotIcon::File(path)) => self.workbench_icon(ctx, &path),
            _ => None,
        })
    }
}

/// Paint a slot's icon in `at`: a built-in icon, a custom picture, or for
/// a custom file that won't load a dashed placeholder box (UIC-314).
pub(crate) fn paint_slot_icon(
    ui: &Ui,
    at: Rect,
    icon: &SlotIcon,
    picture: Option<TextureId>,
    enabled: bool,
) {
    match (icon, picture) {
        (SlotIcon::Builtin(icon), _) if enabled => icons::paint(ui, at, *icon),
        (SlotIcon::Builtin(icon), _) => icons::paint_faded(ui, at, *icon),
        (SlotIcon::File(_), Some(texture)) => {
            let tint = if enabled {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(90)
            };
            ui.painter().image(
                texture,
                at,
                Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                tint,
            );
        }
        (SlotIcon::File(_), None) => {
            let stroke = Stroke::new(1.0f32, theme::disabled_text());
            ui.painter()
                .rect_stroke(at.shrink(1.), 2., stroke, StrokeKind::Inside);
            ui.painter().text(
                at.center(),
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(at.height() * 0.6),
                theme::disabled_text(),
            );
        }
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
        // UIC-310: Esc in a sub-pie goes back one level.
        Event::Key {
            key: Key::Escape,
            pressed: true,
            ..
        } if !open.stack.is_empty() => Outcome::Back,
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

/// The pie's name in the centre (UIC-313): one line, at most
/// `NAME_WIDTH` wide, ending in "…" when longer.
fn name_galley(painter: &Painter, title: &str) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(
        title.to_owned(),
        FontId::proportional(12.),
        theme::text(),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: NAME_WIDTH,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

/// How far from the centre, along unit `d`, the ray leaves `half` (a box's
/// half size around the centre).
fn box_exit(half: Vec2, d: Vec2) -> f32 {
    let tx = if d.x.abs() > 1e-4 {
        half.x / d.x.abs()
    } else {
        f32::INFINITY
    };
    let ty = if d.y.abs() > 1e-4 {
        half.y / d.y.abs()
    } else {
        f32::INFINITY
    };
    tx.min(ty)
}

/// Paint a pie: its name in the centre with an arrow toward the
/// highlighted slot, then the slots' buttons.
fn paint(
    ui: &Ui,
    open: &Open,
    statuses: &[Option<Status>],
    pictures: &[Option<TextureId>; SLOTS],
    why: Option<&str>,
) {
    let painter = ui.painter();
    let c = open.center;
    let fill = theme::popup_fill();
    // UIC-313: the pie's name sits in the centre (it replaced the dot; the
    // dead zone around it is invisible).
    let name = name_galley(painter, &open.pie.title);
    let plate = Rect::from_center_size(c, name.size() + vec2(12., 6.));
    painter.rect(
        plate,
        plate.height() / 2.,
        fill,
        Stroke::new(1.0f32, theme::border()),
        StrokeKind::Inside,
    );
    painter.galley(c - name.size() / 2., name, theme::text());
    let highlighted = open.highlighted();
    if let Some(i) = highlighted {
        // The arrow sits just outside the name plate, pointing at the
        // highlighted button.
        let d = (open.rect(i).center() - c).normalized();
        let n = vec2(-d.y, d.x);
        let edge = box_exit(plate.size() / 2., d);
        let tip = c + d * (edge + 9.);
        let base = c + d * (edge + 2.);
        painter.add(Shape::convex_polygon(
            vec![tip, base + n * 4.5, base - n * 4.5],
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
        let (bg, mut text) = match (lit, status.enabled) {
            (true, true) => (theme::orange(), theme::on_orange()),
            (true, false) => (theme::hover(), theme::disabled_text()),
            (false, true) => (fill, theme::text()),
            (false, false) => (fill, theme::disabled_text()),
        };
        // UIC-221: a line no command has shows red.
        let border = if status.unknown {
            text = theme::error();
            theme::error()
        } else {
            theme::border()
        };
        painter.rect(r, 6., bg, Stroke::new(1.0f32, border), StrokeKind::Inside);
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
        // UIC-310: a sub-pie slot carries a small outward-pointing
        // triangle at its far end (a tiny glyph, drawn flat).
        let mut room = match status.mark {
            Mark::Plain => r,
            _ => r.with_min_x(r.left() + MARK_ROOM),
        };
        if entry.sub.is_some() {
            let left = SLOT_AT[i].x < 0.;
            let x = if left {
                r.left() + PAD / 2. + 3.
            } else {
                r.right() - PAD / 2. - 3.
            };
            let dir = if left { -1. } else { 1. };
            let y = r.center().y;
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(x + dir * 3.5, y),
                    pos2(x - dir * 2.5, y - 4.5),
                    pos2(x - dir * 2.5, y + 4.5),
                ],
                text,
                Stroke::NONE,
            ));
            room = if left {
                room.with_min_x(room.left() + SUB_ROOM)
            } else {
                room.with_max_x(room.right() - SUB_ROOM)
            };
        }
        // Centred, in the room left by the marks: the icon, the label or
        // both, per the pie's icon mode (UIC-309).
        let (icon, show_label) = open.pie.shows(i);
        let label =
            show_label.then(|| painter.layout_no_wrap(entry.label.clone(), font.clone(), text));
        let width = label.as_ref().map_or(0., |g| g.size().x)
            + match (icon.is_some(), label.is_some()) {
                (true, true) => ICON + ICON_GAP,
                (true, false) => ICON,
                (false, _) => 0.,
            };
        let mut x = room.center().x - width / 2.;
        if let Some(icon) = icon {
            let at = Rect::from_min_size(pos2(x, room.center().y - ICON / 2.), Vec2::splat(ICON));
            paint_slot_icon(ui, at, &icon, pictures[i], status.enabled);
            x += ICON + ICON_GAP;
        }
        if let Some(galley) = label {
            let y = room.center().y - galley.size().y / 2.;
            painter.galley(pos2(x, y), galley, text);
        }
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
