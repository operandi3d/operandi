//! Tab between the fields of a panel or form (UIC-51).
//!
//! While a numeric or text field is being typed in, Tab accepts the typed
//! value and moves editing to the next field of the same panel; Shift+Tab
//! moves to the previous one. The order is the order the fields are drawn
//! (top to bottom, X/Y/Z left to right) and wraps at either end. Buttons,
//! checkboxes and menus are skipped, unlike egui's own focus travel, which
//! stops on every focusable widget. Text fields joined numeric ones at
//! Jason's request (2026-11-10): "tab should also take me to the next text
//! field in the form".
//!
//! Every field in the app takes part: the Tool Properties fields
//! (`Operandi::number`) and every `TextEdit` call [`register`] after they
//! are drawn, and every other value box is this module's [`DragValue`], a
//! thin wrapper of egui's that registers the box it draws. Tests keep
//! `egui::DragValue` out of the rest of the app and find any `TextEdit`
//! that doesn't register.
//!
//! "The same panel" is the same layer (window, popover or the main window)
//! and the same clip rectangle, which is what a panel's scroll area or
//! section gives all of its rows. Disabled fields are skipped.
//!
//! The value is accepted by the field losing focus, the route a click
//! elsewhere already takes, so every field keeps its own parsing, limits and
//! undo. A Tool Properties value a tool refuses keeps focus on that field
//! with its error, as a click away would.
//!
//! Frame flow: the raw-input hook ([`take_tab`]) sees a Tab press while a
//! registered field has focus, removes it so egui's focus travel never runs,
//! and remembers the step. At the start of the next pass ([`begin_pass`])
//! the target is picked from the fields the previous pass drew and given
//! focus before anything is drawn, so the old field sees `lost_focus` and
//! accepts, and the new one sees `gained_focus` and selects its whole value
//! (typing replaces it; UIC-51).
use std::ops::RangeInclusive;

use eframe::egui::{
    self, Context, Event, Id, Key, LayerId, Modifiers, RawInput, Rect, Response, TextEdit, Ui,
    Widget, emath, text,
};

/// One numeric field drawn in a pass.
#[derive(Clone, Debug, PartialEq)]
struct Field {
    id: Id,
    layer: LayerId,
    clip: Rect,
}

impl Field {
    fn same_panel(&self, other: &Self) -> bool {
        self.layer == other.layer && self.clip == other.clip
    }
}

#[derive(Clone, Default, Debug)]
struct Registry {
    /// Fields drawn so far in this pass, in draw order.
    this: Vec<Field>,
    /// The fields the last complete pass drew.
    prev: Vec<Field>,
    /// Net Tab steps waiting for the next pass (Shift+Tab counts -1).
    pending: i32,
    /// The field Tab just moved to, until it has been drawn once.
    arrived: Option<Id>,
}

fn key() -> Id {
    Id::new("numeric-tab-registry")
}

fn with<R>(ctx: &Context, f: impl FnOnce(&mut Registry) -> R) -> R {
    ctx.data_mut(|d| f(d.get_temp_mut_or_default::<Registry>(key())))
}

/// Record a numeric field drawn this pass. For a plain `TextEdit`, `text`
/// is its text, all of which is selected when Tab arrives on it; egui's
/// DragValue selects its own.
pub(crate) fn register(ui: &Ui, response: &Response, text: Option<&str>) {
    if !response.enabled() {
        return;
    }
    let field = Field {
        id: response.id,
        layer: ui.layer_id(),
        clip: ui.clip_rect(),
    };
    let arrived = with(ui.ctx(), |r| {
        r.this.push(field);
        let arrived = r.arrived == Some(response.id);
        if arrived {
            r.arrived = None;
        }
        arrived
    });
    if arrived && response.has_focus() {
        response.scroll_to_me(None);
        if let Some(text) = text {
            select_all_text(ui.ctx(), response.id, text);
        }
    }
}

/// Is `id` a text or number field (one that takes typed keys)? Fields
/// that registered count, and so does any egui text edit, registered or
/// not (it keeps its state under its id).
pub(crate) fn is_text_field(ctx: &Context, id: Id) -> bool {
    with(ctx, |r| r.this.iter().chain(&r.prev).any(|f| f.id == id))
        || TextEdit::load_state(ctx, id).is_some()
}

/// The fields the last complete pass drew, in order (tests sweep them).
#[cfg(test)]
pub(crate) fn drawn_fields(ctx: &Context) -> Vec<Id> {
    with(ctx, |r| r.prev.iter().map(|f| f.id).collect())
}

/// Select the whole text of the focused text edit `id`.
fn select_all_text(ctx: &Context, id: Id, text: &str) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let end = text.chars().count();
    state.cursor.set_char_range(Some(text::CCursorRange::two(
        text::CCursor::default(),
        text::CCursor::new(end),
    )));
    state.store(ctx, id);
}

/// Focus a newly opened editor and select its initial text once.
pub(crate) fn focus_and_select_on_open(response: &Response, text: &str, opening: &mut bool) {
    if response.enabled() && std::mem::take(opening) {
        response.request_focus();
        select_all_text(&response.ctx, response.id, text);
        response.ctx.request_repaint();
    }
}

/// Raw-input hook: while a registered numeric field has focus, take its Tab
/// and Shift+Tab presses away from egui and keep them for [`begin_pass`].
/// Returns true when it took any.
pub(crate) fn take_tab(ctx: &Context, input: &mut RawInput) -> bool {
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return false;
    };
    // `this` still holds the pass that just finished.
    if !with(ctx, |r| r.this.iter().any(|f| f.id == focused)) {
        return false;
    }
    let (steps, took) = strip_tabs(&mut input.events);
    if steps != 0 {
        with(ctx, |r| r.pending += steps);
    }
    took
}

/// Remove plain Tab and Shift+Tab presses and releases from `events`.
/// Returns the net steps (Shift+Tab counts -1) and whether any were taken.
fn strip_tabs(events: &mut Vec<Event>) -> (i32, bool) {
    let plain = |m: &Modifiers| m.is_none() || m.shift_only();
    let mut steps = 0;
    let mut took = false;
    events.retain(|event| match event {
        Event::Key {
            key: Key::Tab,
            modifiers,
            pressed,
            ..
        } if plain(modifiers) => {
            if *pressed {
                steps += if modifiers.shift { -1 } else { 1 };
            }
            took = true;
            false
        }
        _ => true,
    });
    (steps, took)
}

/// Start of a pass in an OS window of its own (an egui immediate viewport:
/// the floating panels and palettes). eframe runs no raw-input hook for
/// those, so [`take_tab`] never sees their keys. Call this first thing in
/// the window's pass: while a field there has focus it takes the Tab
/// presses away from the window's input (so no shortcut sees them), stops
/// egui's own focus travel, and moves to the next field at once.
pub(crate) fn take_tab_in_window(ctx: &Context) {
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return;
    };
    if !is_text_field(ctx, focused) {
        return;
    }
    let (steps, took) = ctx.input_mut(|i| strip_tabs(&mut i.events));
    if !took {
        return;
    }
    ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
    // `prev` holds the last complete pass's fields, this window's included.
    let target = with(ctx, |r| {
        let target = step(&r.prev, focused, steps);
        if target.is_some() {
            r.arrived = target;
        }
        target
    });
    if let Some(target) = target {
        ctx.memory_mut(|m| m.request_focus(target));
        ctx.request_repaint();
    }
}

/// Start of a pass: retire the last pass's fields and carry out a waiting
/// Tab by moving focus to its target before anything is drawn.
pub(crate) fn begin_pass(ctx: &Context) {
    let focused = ctx.memory(|m| m.focused());
    let target = with(ctx, |r| {
        r.prev = std::mem::take(&mut r.this);
        let steps = std::mem::take(&mut r.pending);
        let target = focused.and_then(|id| step(&r.prev, id, steps));
        if target.is_some() {
            r.arrived = target;
        }
        target
    });
    if let Some(target) = target {
        ctx.memory_mut(|m| m.request_focus(target));
        ctx.request_repaint();
    }
}

/// The field `steps` places after `from` among the fields of its panel,
/// wrapping at either end; `None` when there is nowhere else to go.
fn step(fields: &[Field], from: Id, steps: i32) -> Option<Id> {
    if steps == 0 {
        return None;
    }
    let origin = fields.iter().find(|f| f.id == from)?;
    let panel: Vec<&Field> = fields.iter().filter(|f| f.same_panel(origin)).collect();
    let at = panel.iter().position(|f| f.id == from)? as i64;
    let len = panel.len() as i64;
    let to = (at + i64::from(steps)).rem_euclid(len);
    (to != at).then(|| panel[to as usize].id)
}

/// egui's `DragValue`, registered for Tab travel, with the mini-slider
/// arrows at its right end (UIC-53: every number field has them, whole
/// numbers too; greyed fields keep them greyed). Use it for every value
/// box, written in full as `numeric_tab::DragValue::new(..)` (the source
/// test below looks for that); the builder methods are egui's.
/// The value's getter/setter, shared by egui's box and the arrows.
type SharedValue<'a> = std::rc::Rc<std::cell::RefCell<Box<dyn 'a + FnMut(Option<f64>) -> f64>>>;
/// A custom formatter, shared by egui's box and the min/max labels.
type Formatter<'a> = std::rc::Rc<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;

#[must_use = "add it to a Ui"]
pub(crate) struct DragValue<'a> {
    inner: egui::DragValue<'a>,
    /// Reads (None) or writes (Some) the value, shared with `inner`.
    value: SharedValue<'a>,
    range: RangeInclusive<f64>,
    /// `range` was set by the caller (not the type's own limits), so it
    /// can get min/max labels (form::range_labels).
    ranged: bool,
    speed: f64,
    integral: bool,
    /// What the labels need to show the limits as the box shows values.
    suffix: String,
    max_decimals: Option<usize>,
    formatter: Option<Formatter<'a>>,
}

impl<'a> DragValue<'a> {
    pub(crate) fn new<Num: emath::Numeric>(value: &'a mut Num) -> Self {
        let get_set: Box<dyn 'a + FnMut(Option<f64>) -> f64> = Box::new(move |v| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });
        let shared = std::rc::Rc::new(std::cell::RefCell::new(get_set));
        let for_inner = shared.clone();
        let inner = egui::DragValue::from_get_set(move |v| (for_inner.borrow_mut())(v));
        let inner = if Num::INTEGRAL {
            inner.max_decimals(0)
        } else {
            inner
        };
        Self {
            inner,
            value: shared,
            range: Num::MIN.to_f64()..=Num::MAX.to_f64(),
            ranged: false,
            speed: 1.,
            integral: Num::INTEGRAL,
            suffix: String::new(),
            max_decimals: None,
            formatter: None,
        }
    }
    fn map(self, f: impl FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>) -> Self {
        Self {
            inner: f(self.inner),
            ..self
        }
    }
    pub(crate) fn speed(self, speed: impl Into<f64>) -> Self {
        let speed = speed.into();
        let mut s = self.map(|d| d.speed(speed));
        s.speed = speed;
        s
    }
    pub(crate) fn range<Num: emath::Numeric>(self, range: RangeInclusive<Num>) -> Self {
        let r = range.start().to_f64()..=range.end().to_f64();
        let mut s = self.map(|d| d.range(range));
        s.range = r;
        s.ranged = true;
        s
    }
    pub(crate) fn suffix(self, suffix: impl ToString) -> Self {
        let suffix = suffix.to_string();
        let mut s = self.map(|d| d.suffix(suffix.clone()));
        s.suffix = suffix;
        s
    }
    pub(crate) fn max_decimals(self, max_decimals: usize) -> Self {
        let mut s = self.map(|d| d.max_decimals(max_decimals));
        s.max_decimals = Some(max_decimals);
        s
    }
    pub(crate) fn fixed_decimals(self, num_decimals: usize) -> Self {
        let mut s = self.map(|d| d.fixed_decimals(num_decimals));
        s.max_decimals = Some(num_decimals);
        s
    }
    pub(crate) fn custom_formatter(
        self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        let formatter: Formatter<'a> = std::rc::Rc::new(formatter);
        let for_inner = formatter.clone();
        let mut s = self.map(|d| d.custom_formatter(move |v, r| for_inner(v, r)));
        s.formatter = Some(formatter);
        s
    }
    /// A limit as this box would show it, for its min/max label.
    fn limit_text(&self, v: f64) -> String {
        let decimals = if self.integral {
            0
        } else {
            self.max_decimals.unwrap_or(3)
        };
        if let Some(format) = &self.formatter {
            return format(v, 0..=decimals);
        }
        let text = format!("{v:.decimals$}");
        let text = if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.')
        } else {
            &text
        };
        format!("{text}{}", self.suffix)
    }
    pub(crate) fn custom_parser(self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.map(|d| d.custom_parser(parser))
    }
    /// A distance in metres: shown in the user's Distance Units and read
    /// with units and arithmetic (UIC-52), so "25 mm" is 0.025 m and
    /// "10 mm + 5 mm" is 0.015 m. A plain number is in the Default Unit.
    pub(crate) fn distance(self) -> Self {
        let mut s = self
            .custom_formatter(|v, _| operandi_core::units::format_distance(v))
            .custom_parser(parse_distance);
        s.suffix.clear();
        s
    }
}

/// The text of a distance field, in metres ([`DragValue::distance`]).
pub(crate) fn parse_distance(text: &str) -> Option<f64> {
    operandi_core::numeric::parse_quantity(text.trim(), operandi_core::numeric::Quantity::Distance)
        .ok()
}

impl Widget for DragValue<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        use crate::scrub::HANDLE_WIDTH;
        let size = ui.spacing().interact_size;
        let width = size.x.max(HANDLE_WIDTH + 24.);
        let (cell, _) = ui.allocate_exact_size(egui::vec2(width, size.y), egui::Sense::hover());
        let (arrows, field) = crate::scrub::split(cell);
        // UIC-118: the selected items disagree; typing or dragging writes
        // one value to all of them.
        let mixed = crate::form::rows::take_mixed();
        // The min/max labels' text, read before `inner` is handed to egui.
        let (lo, hi) = (*self.range.start(), *self.range.end());
        let labels = crate::form::range_labels::shown(lo, hi)
            .filter(|_| self.ranged)
            .map(|limits| (limits, self.limit_text(lo), self.limit_text(hi)));
        let inner = if mixed {
            self.inner
                .custom_formatter(|_, _| crate::form::rows::MIXED.to_owned())
                .suffix("")
        } else {
            self.inner
        };
        // UIC-90: the value reads from the field's left, its unit after it.
        let mut response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(field)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.spacing_mut().interact_size.x = field.width();
                    ui.add(inner)
                },
            )
            .inner;
        register(ui, &response, None);
        // The arrows: drag sideways to change the value. Shift is coarser;
        // Option or the physical Ctrl key finer (F1: Ctrl is never Cmd here).
        let key = format!("{:?}", response.id);
        let drag = crate::scrub::handle(ui, arrows, &key, false);
        if drag.dragged() {
            let dx = f64::from(drag.drag_delta().x);
            let m = ui.input(|i| i.modifiers);
            let step = if m.shift {
                10.
            } else if m.alt || m.ctrl {
                0.1
            } else {
                1.
            };
            if dx != 0. {
                let mut get_set = self.value.borrow_mut();
                let old = get_set(None);
                let mut new = old + dx * self.speed * step;
                if self.integral {
                    new = new.round();
                }
                let new = new.clamp(*self.range.start(), *self.range.end());
                if new != old {
                    get_set(Some(new));
                    response.mark_changed();
                }
            }
        }
        if drag.drag_started() || drag.drag_stopped() {
            ui.ctx().request_repaint();
        }
        // Min/max labels (an approved change to UIC-90) under the box,
        // with room taken for them in whatever holds it.
        if let Some((limits, lo_text, hi_text)) = labels {
            let at = (!mixed).then(|| (self.value.borrow_mut())(None));
            let text = |v: f64| {
                if v == limits.0 {
                    lo_text.clone()
                } else {
                    hi_text.clone()
                }
            };
            crate::form::range_labels::paint(ui, cell, at, limits, text);
            crate::form::range_labels::reserve(ui, cell);
        }
        response | drag.with_new_rect(arrows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Order, pos2, vec2};

    #[test]
    fn opening_a_text_editor_selects_its_name_only_once() {
        let ctx = Context::default();
        let mut name = String::from("Café");
        let mut opening = true;
        let mut frame = |events| {
            let mut id = Id::NULL;
            let _ = ctx.run(
                RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let edit = ui.text_edit_singleline(&mut name);
                        focus_and_select_on_open(&edit, &name, &mut opening);
                        id = edit.id;
                    });
                },
            );
            (id, name.clone())
        };

        let (id, _) = frame(vec![]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(id));
        let (_, name) = frame(vec![Event::Text("New".into())]);
        assert_eq!(name, "New");
        let (_, name) = frame(vec![Event::Text("!".into())]);
        assert_eq!(name, "New!");
    }

    fn field(n: u64, clip: Rect) -> Field {
        Field {
            id: Id::new(n),
            layer: LayerId::new(Order::Background, Id::new("panel")),
            clip,
        }
    }

    #[test]
    fn distance_fields_read_units_and_arithmetic() {
        // Material editing item 1: "25 mm" was stored as 25 m.
        let close = |t: &str, v: f64| (parse_distance(t).unwrap() - v).abs() < 1e-12;
        assert!(close("25 mm", 0.025));
        assert!(close(" 10 mm + 5 mm ", 0.015));
        assert!(close("3 cm", 0.03));
        assert!(parse_distance("abc").is_none());
    }

    #[test]
    fn steps_wrap_within_one_panel_and_skip_others() {
        let a = Rect::from_min_size(pos2(0., 0.), vec2(100., 100.));
        let b = Rect::from_min_size(pos2(200., 0.), vec2(100., 100.));
        let fields = [field(1, a), field(2, a), field(9, b), field(3, a)];
        let id = |n| Some(Id::new(n));
        assert_eq!(step(&fields, Id::new(1), 1), id(2));
        // The other panel's field is skipped.
        assert_eq!(step(&fields, Id::new(2), 1), id(3));
        // Wraps forwards and backwards.
        assert_eq!(step(&fields, Id::new(3), 1), id(1));
        assert_eq!(step(&fields, Id::new(1), -1), id(3));
        // A lone field has nowhere to go.
        assert_eq!(step(&fields, Id::new(9), 1), None);
        assert_eq!(step(&fields, Id::new(42), 1), None);
    }

    /// Value boxes with a button and a checkbox between them: Tab skips the
    /// controls, accepts the typed value and opens the next box with its
    /// value selected; Shift+Tab goes back.
    #[test]
    fn drag_values_tab_past_other_controls() {
        let ctx = Context::default();
        let mut values = [1.0_f64, 2.0];
        let mut ids = [Id::NULL; 2];
        let mut frame = |values: &mut [f64; 2], events: Vec<Event>| {
            let mut raw = RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0., 0.), vec2(400., 300.))),
                events,
                focused: true,
                ..Default::default()
            };
            take_tab(&ctx, &mut raw);
            let _ = ctx.run(raw, |ctx| {
                begin_pass(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    let [a, b] = values;
                    ids[0] = ui.add(DragValue::new(a)).id;
                    let _ = ui.button("Between");
                    let mut on = false;
                    ui.checkbox(&mut on, "Also between");
                    ids[1] = ui.add(DragValue::new(b)).id;
                });
            });
            ids
        };
        let tab = |shift: bool| Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: if shift {
                Modifiers::SHIFT
            } else {
                Modifiers::NONE
            },
        };
        let typed = |s: &str| Event::Text(s.into());
        let ids = frame(&mut values, vec![]);
        // A click opens the first box with its value selected.
        let at = ctx.read_response(ids[0]).unwrap().rect.center();
        let button = |pressed| Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        frame(&mut values, vec![Event::PointerMoved(at), button(true)]);
        frame(&mut values, vec![button(false)]);
        frame(&mut values, vec![]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(ids[0]));
        frame(&mut values, vec![typed("5")]);
        frame(&mut values, vec![tab(false)]);
        assert_eq!(values[0], 5.);
        assert_eq!(ctx.memory(|m| m.focused()), Some(ids[1]));
        frame(&mut values, vec![typed("7")]);
        frame(&mut values, vec![tab(true)]);
        // "7" replaced the selected "2" rather than joining it.
        assert_eq!(values, [5., 7.]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(ids[0]));
    }

    /// Every value box in the app goes through the wrapper, so Tab reaches
    /// it. A plain `egui::DragValue` elsewhere would be skipped by Tab.
    #[test]
    fn every_drag_value_is_registered() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == "numeric_tab.rs")
                || path.extension().is_none_or(|e| e != "rs")
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for (at, _) in source.match_indices("DragValue::new") {
                if !source[..at].ends_with("numeric_tab::") {
                    let line = source[..at].lines().count();
                    offenders.push(format!("{}:{line}", path.display()));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "write numeric_tab::DragValue at {offenders:?}"
        );
    }

    /// Every text field in the app registers, so Tab reaches it: each
    /// `TextEdit` is followed, within a few lines, by a `register` call.
    #[test]
    fn every_text_edit_is_registered() {
        const WITHIN: usize = 30;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut offenders = Vec::new();
        let mut dirs = vec![std::path::PathBuf::from(root)];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if name == "numeric_tab.rs"
                    || name.ends_with("_tests.rs")
                    || name == "tests.rs"
                    || path.extension().is_none_or(|e| e != "rs")
                {
                    continue;
                }
                let source = std::fs::read_to_string(&path).unwrap();
                let lines: Vec<&str> = source.lines().collect();
                for (at, line) in lines.iter().enumerate() {
                    if !(line.contains("TextEdit::singleline(")
                        || line.contains("TextEdit::multiline("))
                    {
                        continue;
                    }
                    let end = (at + WITHIN).min(lines.len());
                    if !lines[at..end]
                        .iter()
                        .any(|l| l.contains("numeric_tab::register("))
                    {
                        offenders.push(format!("{}:{}", path.display(), at + 1));
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "call numeric_tab::register after the TextEdit at {offenders:?}"
        );
    }
}
