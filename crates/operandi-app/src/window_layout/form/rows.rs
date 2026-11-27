//! Panel mode for panels that draw their rows themselves (UIC-100..124).
//!
//! `panel::show` draws a whole panel from a list of `Row`s. Most property
//! panels in Operandi read and write their values directly (a tool's
//! parameters, an item's channels), so this module gives them the same
//! look one row at a time: the same column maths (`layout::panel_layout`),
//! the same label, status circle, section bar and fields.
//!
//! - `scope` wraps one tab of a panel. Every row inside it shares one label
//!   edge: the label column is the longest label drawn in the scope last
//!   frame (UIC-111), so a label that appears re-lays the tab once.
//! - `status` before a row marks it as a channel and gives it a status
//!   light (UIC-110/112): a plain ring at the default, "Constant" (filled)
//!   once changed. Rows without one keep the light's space empty so every
//!   field edge lines up.
//! - Right-clicking a channel row's label or light offers Reset to Default
//!   (CH-Q1): the `*_d` rows put their default back themselves; rows drawn
//!   with `status` + `frame` read `take_reset` after drawing.
//! - `section` draws a darker foldable bar (UIC-105..107). Fold state is
//!   kept per section name across restarts (in the form memory).
//! - `row`, `check`, `number`, `choice`, `button` and `value` draw rows.
//!
//! `option_row` forwards to these, so every older panel picks up the same
//! layout without each row being rewritten.
use super::layout::{self, panel::*};
use super::status::ChannelStatus;
use super::widgets::{self, Fields};
use super::{Row, Unit, Value};
use crate::theme;
use crate::unavailable::Gate;
use eframe::egui::{
    self, Align, Align2, CornerRadius, Id, InnerResponse, Layout, Pos2, Rect, Response, Sense,
    Shape, Stroke, TextStyle, Ui, UiBuilder, pos2, vec2,
};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// One open scope: the label column it measured last frame and what it
/// measures this frame.
struct Scope {
    id: Id,
    last: f32,
    measured: f32,
}

#[derive(Default)]
struct State {
    scopes: Vec<Scope>,
    /// The next row's status light (UIC-112).
    status: Option<ChannelStatus>,
    /// The next row is this much taller, for its field's min/max labels.
    extra: f32,
    /// The last channel row's Reset to Default was chosen (CH-Q1).
    reset: bool,
    /// Inside `plain`: rows are settings, not channels, so none shows a
    /// status light (UIC-112).
    plain: usize,
    /// Sections whose fold differs from their default, by name.
    flipped: BTreeSet<String>,
    /// Labels of settings an instance overrides (shader layer menu §3):
    /// drawn in italics, with Revert to Source on right-click.
    overridden: BTreeSet<String>,
    /// The overridden label whose Revert to Source was picked.
    revert: Option<String>,
    /// `flipped` changed since the app last saved it.
    dirty: bool,
    /// The next row's channels for keying (UIC-112/119, TK-20/24): its
    /// light keys them, its label selects them, right-click has the key
    /// commands. Each entry is `item:name`.
    anim: Option<AnimRow>,
    /// Command lines the rows asked for this frame (the app runs them).
    requests: Vec<String>,
}

/// A row that is a keyable channel (anim_ui.rs builds it).
#[derive(Clone, Debug, Default)]
pub(crate) struct AnimRow {
    /// `item:name` of each selected item's channel.
    pub channels: Vec<String>,
    /// Its label is selected for `S` (UIC-119).
    pub selected: bool,
}

/// Marks the next row as keyable channels `row` (after `set_status`).
pub(crate) fn anim_channel(row: AnimRow) {
    STATE.with(|s| s.borrow_mut().anim = Some(row));
}

/// The lines rows asked for since the last call.
pub(crate) fn take_anim_requests() -> Vec<String> {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().requests))
}

fn take_anim() -> Option<AnimRow> {
    STATE.with(|s| s.borrow_mut().anim.take())
}

fn request(line: String) {
    STATE.with(|s| s.borrow_mut().requests.push(line));
}

/// TK-20/24, UIC-119: a keyable row's light, label and right-click menu.
fn anim_controls(ui: &mut Ui, label: &Response, circle: Pos2, row: &AnimRow) {
    if row.channels.is_empty() {
        return;
    }
    let list = row.channels.join(",");
    if row.selected {
        ui.painter().rect_filled(
            label.rect.expand2(vec2(2., 0.)),
            CornerRadius::same(2),
            theme::orange().gamma_multiply(0.25),
        );
    }
    let light = ui.interact(
        Rect::from_center_size(circle, vec2(CIRCLE + 4., ROW_HEIGHT)),
        ui.id().with(("anim-light", circle.y as i32)),
        Sense::click(),
    );
    if light.clicked() {
        request(format!("operandi.key.toggle channels:{list}"));
    }
    let light = light.on_hover_text("Click to set a key here, or remove the key on this frame");
    if label.clicked() {
        let add = ui.input(|i| i.modifiers.shift || i.modifiers.command);
        request(format!(
            "operandi.channel.select channel:{} add:{add}",
            row.channels[0]
        ));
    }
    for r in [&light, label] {
        r.context_menu(|ui| {
            // #54: Copy takes the selected channels when this row is one of
            // them, else this row's; Paste goes onto the same.
            let picked = if row.selected {
                String::new()
            } else {
                format!(" channels:{list}")
            };
            let entries: [(&str, String); 12] = [
                ("Copy", format!("operandi.channel.copy{picked}")),
                ("Paste", format!("operandi.channel.paste{picked}")),
                ("Add Key", format!("operandi.key.set channels:{list}")),
                ("Remove Key", format!("operandi.key.remove channels:{list}")),
                ("Next Key", "operandi.time.go to:nextKey".into()),
                ("Previous Key", "operandi.time.go to:prevKey".into()),
                ("First Key", "operandi.time.go to:firstKey".into()),
                ("Last Key", "operandi.time.go to:lastKey".into()),
                (
                    "Remove Animation",
                    format!("operandi.key.removeAnimation channels:{list}"),
                ),
                ("Reset", format!("operandi.key.reset channels:{list}")),
                ("Bake", format!("operandi.key.bake channels:{list}")),
                // CP-20: into the channel panel's current set.
                (
                    "Add to Channel Set",
                    format!("operandi.channelSet.add channels:{list}"),
                ),
            ];
            for (text, line) in entries {
                if ui.button(text).clicked() {
                    // Key jumps follow this channel (UIC-119 selects it).
                    if text.ends_with("Key") && !text.contains("Add") && !text.contains("Remove") {
                        request(format!(
                            "operandi.channel.select channel:{}",
                            row.channels[0]
                        ));
                    }
                    request(line);
                    ui.close();
                }
            }
        });
    }
}

// The UI runs on one thread per app (and one per test), so the row state
// lives with the thread rather than being threaded through every panel.
thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The form memory's entry for section folds.
pub(crate) const FOLDS: &str = "Panel Sections";

/// The scope used when a row is drawn outside any `scope`.
const LOOSE: &str = "form-rows-loose";

/// Runs `add` as one tab of a panel named `id`: its rows share one label
/// column (UIC-111) and are spaced `ROW_GAP` apart (UIC-120).
pub(crate) fn scope<R>(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let key = Id::new(("form-rows", id));
    let last: f32 = ui.data(|d| d.get_temp(key)).unwrap_or(0.);
    // A "(mixed)" flag never crosses into another panel.
    take_mixed();
    #[cfg(test)]
    crate::ui_inventory::push_scope(id);
    STATE.with(|s| {
        s.borrow_mut().scopes.push(Scope {
            id: key,
            last,
            measured: 0.,
        })
    });
    let r = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.y = ROW_GAP;
            add(ui)
        })
        .inner;
    take_mixed();
    #[cfg(test)]
    crate::ui_inventory::pop_scope();
    let measured = STATE.with(|s| s.borrow_mut().scopes.pop().map_or(0., |f| f.measured));
    if (measured - last).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(key, measured));
        ui.ctx().request_repaint();
    }
    r
}

/// Runs `add` with rows that are settings, not channels: no status light
/// on any row, while fields still start at the shared field column
/// (UIC-112). Preferences and popovers of settings use it.
pub(crate) fn plain<R>(add: impl FnOnce() -> R) -> R {
    STATE.with(|s| s.borrow_mut().plain += 1);
    let r = add();
    STATE.with(|s| s.borrow_mut().plain -= 1);
    r
}

/// Gives the next row a status light (UIC-110/112): "Constant" when
/// `non_default`, else "None" (a plain ring).
pub(crate) fn status(non_default: bool) {
    set_status(ChannelStatus::from_non_default(non_default));
}

/// Gives the next row the status light `status` (UIC-112).
pub(crate) fn set_status(status: ChannelStatus) {
    STATE.with(|s| s.borrow_mut().status = Some(status));
}

/// Takes the next row's status light. A "(mixed)" row's values differ, so
/// at least one is non-default: it shows "Constant" (a true "Mixed" light
/// comes with animation and links).
fn take_status() -> Option<ChannelStatus> {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.plain > 0 {
            s.status = None;
            return None;
        }
        let mixed = crate::value_display::is_mixed();
        s.status.take().map(|status| {
            if mixed && status == ChannelStatus::None {
                ChannelStatus::Constant
            } else {
                status
            }
        })
    })
}

/// Mark these row labels as an instance's overrides until
/// `clear_overridden` (shader layer menu §3).
pub(crate) fn mark_overridden(labels: impl IntoIterator<Item = String>) {
    STATE.with(|s| s.borrow_mut().overridden = labels.into_iter().collect());
}
pub(crate) fn clear_overridden() {
    STATE.with(|s| s.borrow_mut().overridden.clear());
}
/// Whether `label` is marked as overridden.
pub(crate) fn is_overridden(label: &str) -> bool {
    STATE.with(|s| s.borrow().overridden.contains(label))
}
/// The label whose Revert to Source was picked, once.
pub(crate) fn take_revert() -> Option<String> {
    STATE.with(|s| s.borrow_mut().revert.take())
}
/// An overridden row's right-click menu.
fn revert_menu(response: &Response, label: &str) {
    if !is_overridden(label) {
        return;
    }
    response.context_menu(|ui| {
        if ui
            .button("Revert to Source")
            .on_hover_text("Follow the source layer's value again")
            .clicked()
        {
            STATE.with(|s| s.borrow_mut().revert = Some(label.to_owned()));
            ui.close();
        }
    });
}
const OVERRIDDEN_TIP: &str = "Overridden on this instance · right-click: Revert to Source";

// The "(mixed)" flag (UIC-118) lives with the value fields that read it.
pub(crate) use crate::value_display::{MIXED, mixed, take_mixed};

/// The label column for a row whose label is `width` wide, recording it
/// for the scope's next frame.
fn label_column(width: f32, available: f32) -> (f32, Id) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.scopes.is_empty() {
            // A row outside any scope can't know its neighbours' labels, so
            // it uses a fixed share of the width; rows side by side still
            // line up. Panels wrap their tabs in `scope` instead.
            return ((available * 0.4).clamp(78., 150.), Id::new(LOOSE));
        }
        let top = s.scopes.last_mut().unwrap();
        top.measured = top.measured.max(width);
        (top.last.max(width), top.id)
    })
}

/// Where a row's parts go.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowRects {
    pub(crate) row: Rect,
    pub(crate) label: Rect,
    pub(crate) circle: Pos2,
    pub(crate) field: Rect,
}

/// Allocates one row in `ui` for a label `label_width` wide.
/// The row is `take_extra()` taller (min/max labels under its field); the
/// label, light and field keep the top `ROW_HEIGHT`.
fn allocate(ui: &mut Ui, label_width: f32) -> (RowRects, Id) {
    let width = ui.available_width().max(0.);
    let (longest, scope) = label_column(label_width, width);
    let cols = layout::panel_layout(width, longest, false, 0.);
    let height = ROW_HEIGHT + take_extra();
    let (row, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let x = row.left();
    let rects = RowRects {
        row,
        label: Rect::from_min_size(
            pos2(x + cols.label_x, row.top()),
            vec2(cols.label_width, ROW_HEIGHT),
        ),
        circle: pos2(x + cols.circle_x, row.top() + ROW_HEIGHT / 2.),
        field: Rect::from_min_size(
            pos2(x + cols.field_x, row.top()),
            vec2(cols.field_width, ROW_HEIGHT),
        ),
    };
    (rects, scope)
}

/// Paints the status light, if the row is a channel (UIC-110/112).
fn paint_status(ui: &Ui, at: Pos2, status: Option<ChannelStatus>) {
    if let Some(status) = status {
        super::status::paint(ui.painter(), at, status, ui.is_enabled());
    }
}

/// Lays out a row's label, circle and field rect. `label` is `None` for
/// rows with no left label (checkboxes, buttons; UIC-115/116).
pub(crate) fn frame(ui: &mut Ui, label: Option<&str>, tooltip: &str) -> (RowRects, Response) {
    let status = take_status();
    let width = label.map_or(0., |l| widgets::text_width(ui, l).ceil());
    let (rects, _) = allocate(ui, width);
    let response = match label {
        Some(text) if is_overridden(text) => {
            let tip = if tooltip.is_empty() {
                OVERRIDDEN_TIP.to_owned()
            } else {
                format!("{tooltip}\n{OVERRIDDEN_TIP}")
            };
            let r = widgets::label(ui, rects.label, text, &tip);
            revert_menu(&r, text);
            r
        }
        Some(text) => widgets::label(ui, rects.label, text, tooltip),
        None => ui.interact(
            rects.label,
            ui.id().with(("no-label", rects.row.top() as i32)),
            Sense::hover(),
        ),
    };
    paint_status(ui, rects.circle, status);
    if let Some(row) = take_anim() {
        let response = response.interact(Sense::click());
        anim_controls(ui, &response, rects.circle, &row);
        return (rects, response);
    }
    if let Some(status) = status {
        reset_menu(ui, rects, status);
    }
    (rects, response)
}

/// CH-Q1: right-click a channel row's label or status light for Reset to
/// Default (enabled once the value differs). The row reads the choice with
/// `take_reset`; the `*_d` rows do so themselves.
fn reset_menu(ui: &mut Ui, rects: RowRects, status: ChannelStatus) {
    let area = rects.label.union(Rect::from_center_size(
        rects.circle,
        vec2(CIRCLE + 4., ROW_HEIGHT),
    ));
    let hit = ui.interact(
        area,
        ui.id().with(("channel-reset", rects.row.top() as i32)),
        Sense::click(),
    );
    hit.context_menu(|ui| {
        if ui
            .gated(
                crate::unavailable::unless(
                    status != ChannelStatus::None,
                    "Already the default value",
                ),
                egui::Button::new("Reset to Default"),
            )
            .clicked()
        {
            STATE.with(|s| s.borrow_mut().reset = true);
            ui.close();
        }
    });
}

/// Makes the next row `extra` taller (min/max labels under its field).
fn set_extra(extra: f32) {
    STATE.with(|s| s.borrow_mut().extra = extra);
}

fn take_extra() -> f32 {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().extra))
}

/// Whether the row just drawn had Reset to Default chosen (taken).
pub(crate) fn take_reset() -> bool {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().reset))
}

/// A row whose field the caller draws: `add` gets a child `Ui` exactly the
/// field column's rect (UIC-113), laid out right to left so a number or
/// dropdown fills it. The response is the label's.
pub(crate) fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    #[cfg(test)]
    crate::ui_inventory::row(label);
    let (rects, response) = frame(ui, Some(label), "");
    #[cfg(test)]
    crate::ui_inventory::row_rect(label, rects.field);
    let (inner, bottom) = in_field(ui, rects.field, add);
    // A value box with min/max labels draws them under the field
    // (range_labels::reserve); the row grows to hold them.
    let below = bottom - rects.row.bottom();
    if below > 0.5 {
        ui.add_space(below);
    }
    InnerResponse::new(inner, response)
}

/// Just the label cell and status circle, for a row the caller lays out
/// itself inside `ui.horizontal`: allocates up to the field column's left
/// edge so whatever follows starts in the field column (UIC-111/112).
pub(crate) fn label_cell(ui: &mut Ui, label: &str) -> Response {
    let status = take_status();
    let width = widgets::text_width(ui, label).ceil();
    let (longest, _) = label_column(
        if label.is_empty() { 0. } else { width },
        ui.available_width(),
    );
    let cols = layout::panel_layout(ui.available_width().max(0.), longest, false, 0.);
    let (cell, _) = ui.allocate_exact_size(
        vec2(cols.field_x - ui.spacing().item_spacing.x, ROW_HEIGHT),
        Sense::hover(),
    );
    let label_rect = Rect::from_min_size(
        pos2(cell.left() + cols.label_x, cell.top()),
        vec2(cols.label_width, ROW_HEIGHT),
    );
    let circle = pos2(cell.left() + cols.circle_x, cell.center().y);
    paint_status(ui, circle, status);
    let anim = take_anim();
    let response = widgets::label(ui, label_rect, label, "");
    if let Some(row) = anim {
        let response = response.interact(Sense::click());
        anim_controls(ui, &response, circle, &row);
        return response;
    }
    response
}

/// The width left for a field after `label_cell`, keeping the right
/// margin (UIC-120).
pub(crate) fn rest_width(ui: &Ui) -> f32 {
    (ui.available_width() - RIGHT_MARGIN).max(0.)
}

/// Runs `add` in a child `Ui` filling `field`.
/// Also gives the bottom of what `add` drew.
fn in_field<R>(ui: &mut Ui, field: Rect, add: impl FnOnce(&mut Ui) -> R) -> (R, f32) {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(field)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().interact_size = vec2(field.width(), ROW_HEIGHT);
    child.spacing_mut().combo_width = field.width();
    child.spacing_mut().item_spacing.x = 2.;
    let r = add(&mut child);
    (r, child.min_rect().bottom())
}

/// Draws `row`'s field in `rect` with the shared form fields. Returns the
/// field's response, marked changed when the value changed.
pub(crate) fn field(
    ui: &mut Ui,
    rect: Rect,
    row: &Row,
    value: &mut Value,
    mixed: bool,
) -> Response {
    #[cfg(test)]
    {
        crate::ui_inventory::row(&row.label);
        crate::ui_inventory::row_rect(&row.label, rect);
        // Rows with a repeated label ("Y") are found by their key too.
        if row.key != row.label {
            crate::ui_inventory::row_rect(&row.key, rect);
        }
    }
    let scope = STATE.with(|s| s.borrow().scopes.last().map_or(Id::new(LOOSE), |f| f.id));
    let id = ui.id().with(("form-row", &row.key));
    let key = scope.with("fields");
    let mut fields: Fields = ui.data_mut(|d| d.get_temp(key)).unwrap_or_default();
    let out = widgets::field(ui, rect, id, row, value, &mut fields, mixed);
    ui.data_mut(|d| d.insert_temp(key, fields));
    let response = widgets::hover(out.response, &row.tooltip);
    if out.changed {
        let mut r = response;
        r.mark_changed();
        r
    } else {
        response
    }
}

/// A checkbox row: no left label, the box at the field column's start and
/// its text to the right (UIC-115). Clicking either toggles it.
pub(crate) fn check(ui: &mut Ui, label: &str, value: &mut bool) -> Response {
    let (rects, _) = frame(ui, None, "");
    let row = Row::check(label, label, false);
    let mut v = Value::Bool(*value);
    let mixed = take_mixed();
    let mut response = field(ui, rects.field, &row, &mut v, mixed);
    if let Value::Bool(on) = v {
        *value = on;
    }
    if is_overridden(label) {
        response = widgets::hover(response, OVERRIDDEN_TIP);
        revert_menu(&response, label);
    }
    response
}

/// A full-width button with no left label (UIC-116). A trailing "▾" in
/// `label` marks a pop-up button.
pub(crate) fn button(ui: &mut Ui, label: &str) -> Response {
    let (rects, _) = frame(ui, None, "");
    let row = Row::button(label, label);
    let mut v = Value::None;
    field(ui, rects.field, &row, &mut v, false)
}

/// Joined segments filling the field column (UIC-89; tool properties
/// only, panels use dropdowns, UIC-113). Returns true when it changed.
pub(crate) fn segments(ui: &mut Ui, label: &str, index: &mut usize, options: &[&str]) -> bool {
    let (rects, _) = frame(ui, Some(label), "");
    let row = Row::segments(label, label, options, 0);
    let mut v = Value::Choice(*index);
    field(ui, rects.field, &row, &mut v, false);
    match v {
        Value::Choice(i) if i != *index => {
            *index = i;
            true
        }
        _ => false,
    }
}

/// A read-only value: plain text in the field column (UIC-113).
pub(crate) fn value(ui: &mut Ui, label: &str, text: &str) -> Response {
    let (rects, response) = frame(ui, Some(label), "");
    let colour = if ui.is_enabled() {
        theme::text()
    } else {
        theme::disabled_text()
    };
    let (galley, cut) = widgets::fitted(ui, text, rects.field.width(), colour);
    let at = pos2(
        rects.field.left(),
        rects.field.center().y - galley.size().y / 2.,
    );
    ui.painter().galley(at, galley, colour);
    let field = ui.interact(rects.field, ui.id().with(("value", label)), Sense::hover());
    let field = if cut {
        field.on_hover_text(text)
    } else {
        field
    };
    response | field
}

/// A checkbox row with its status circle against `default` (UIC-110,
/// UIC-115) and tooltip (UIC-119).
pub(crate) fn check_d(
    ui: &mut Ui,
    label: &str,
    value: &mut bool,
    default: bool,
    tip: &str,
) -> Response {
    status(*value != default);
    let (rects, _) = frame(ui, None, "");
    let row = Row::check(label, label, false).tip(tip);
    let mut v = Value::Bool(*value);
    let mut response = field(ui, rects.field, &row, &mut v, false);
    if let Value::Bool(on) = v {
        *value = on;
    }
    if take_reset() {
        *value = default;
        response.mark_changed();
    }
    response
}

/// A number row with its unit, mini-slider arrows (UIC-53), status circle
/// against `default` and tooltip. Whole-number types step whole numbers.
pub(crate) fn number_d<T: egui::emath::Numeric>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    default: T,
    unit: Unit,
    range: std::ops::RangeInclusive<T>,
    tip: &str,
) -> Response {
    number_keyed_d(ui, label, label, value, default, unit, range, tip)
}

/// `number_d` with its own `key`, for rows whose short label repeats in
/// one form ("Y" under both "Position X" and "Normal X"): the key keeps
/// their typed text and focus apart (GitHub #21).
#[allow(clippy::too_many_arguments)]
pub(crate) fn number_keyed_d<T: egui::emath::Numeric>(
    ui: &mut Ui,
    key: &str,
    label: &str,
    value: &mut T,
    default: T,
    unit: Unit,
    range: std::ops::RangeInclusive<T>,
    tip: &str,
) -> Response {
    status(value.to_f64() != default.to_f64());
    set_extra(super::range_labels::extra(
        range.start().to_f64(),
        range.end().to_f64(),
    ));
    let (rects, label_response) = frame(ui, Some(label), tip);
    let unit = if T::INTEGRAL && unit == Unit::Scalar {
        Unit::Count
    } else {
        unit
    };
    let row = Row::number(key, label, unit, default.to_f64())
        .range(range.start().to_f64(), range.end().to_f64())
        .tip(tip);
    let mut v = Value::Number(value.to_f64());
    let mut response = field(ui, rects.field, &row, &mut v, false);
    // The field's text box is placed with `ui.put`, which leaves the
    // cursor under the field; step back below the whole (taller) row.
    let short = rects.row.bottom() + ui.spacing().item_spacing.y - ui.cursor().top();
    if short > 0.5 {
        ui.add_space(short);
    }
    if let Value::Number(n) = v {
        let n = if T::INTEGRAL { n.round() } else { n };
        if n != value.to_f64() {
            *value = T::from_f64(n);
        }
    }
    if take_reset() {
        *value = default;
        response.mark_changed();
    }
    let _ = label_response;
    response
}

/// A dropdown row (UIC-113) with its status circle against `default` and
/// tooltip.
pub(crate) fn choice_d<T: PartialEq + Clone>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    default: &T,
    options: &[(T, &str)],
    tip: &str,
) -> Response {
    status(value != default);
    let (rects, _) = frame(ui, Some(label), tip);
    let names: Vec<&str> = options.iter().map(|(_, n)| *n).collect();
    let index = options.iter().position(|(o, _)| o == value).unwrap_or(0);
    let row = Row::choice(label, label, &names, 0).tip(tip);
    let mut v = Value::Choice(index);
    let mut response = field(ui, rects.field, &row, &mut v, false);
    if let Value::Choice(i) = v
        && i != index
        && let Some((o, _)) = options.get(i)
    {
        *value = o.clone();
    }
    if take_reset() {
        *value = default.clone();
        response.mark_changed();
    }
    response
}

/// A plain text row (UIC-113) with its status circle against `default`.
pub(crate) fn text_d(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    default: &str,
    tip: &str,
) -> Response {
    status(value != default);
    let (rects, _) = frame(ui, Some(label), tip);
    let row = Row::text(label, label, default).tip(tip);
    let mut v = Value::Text(value.clone());
    let mut response = field(ui, rects.field, &row, &mut v, false);
    if let Value::Text(t) = v
        && t != *value
    {
        *value = t;
    }
    if take_reset() {
        *value = default.to_owned();
        response.mark_changed();
    }
    response
}

/// A note under a page's rows: weak, wrapped text across the panel (not a
/// row; used for what a page leaves out and why).
pub(crate) fn note(ui: &mut Ui, text: &str) {
    ui.add_space(ROW_GAP);
    let text = crate::ui_text::without_spec_codes(text);
    ui.add(egui::Label::new(egui::RichText::new(text).weak()).wrap());
}

/// Is the section `name` folded? `folded_by_default` sections (rarely used
/// ones, like Notes) start folded (UIC-106).
pub(crate) fn is_folded(name: &str, folded_by_default: bool) -> bool {
    STATE.with(|s| s.borrow().flipped.contains(name)) != folded_by_default
}

fn flip(name: &str) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.flipped.remove(name) {
            s.flipped.insert(name.to_owned());
        }
        s.dirty = true;
    });
}

/// A section: a darker bar with a fold triangle, the whole bar clickable
/// (UIC-105/106), then `add`'s rows while open. Fold state is kept per
/// section name across restarts (UIC-107). No gap or box around it.
pub(crate) fn section<R>(
    ui: &mut Ui,
    name: &str,
    folded_by_default: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let folded = is_folded(name, folded_by_default);
    let width = ui.available_width();
    if section_bar(ui, width, name, !folded).clicked() {
        flip(name);
    }
    (!folded).then(|| add(ui))
}

/// A section's bar with its fold triangle (UIC-105/106, UIC-120: 18 high).
/// Returns true when clicked.
pub(crate) fn section_bar(ui: &mut Ui, width: f32, name: &str, open: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, SECTION_HEIGHT), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, theme::pressed());
    let c = pos2(rect.left() + LEFT_MARGIN + 4., rect.center().y);
    let color = theme::weak_text();
    let points = if open {
        vec![
            pos2(c.x - 4., c.y - 2.),
            pos2(c.x + 4., c.y - 2.),
            pos2(c.x, c.y + 3.),
        ]
    } else {
        vec![
            pos2(c.x - 2., c.y - 4.),
            pos2(c.x + 3., c.y),
            pos2(c.x - 2., c.y + 4.),
        ]
    };
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    painter.text(
        pos2(c.x + 9., rect.center().y),
        Align2::LEFT_CENTER,
        name,
        TextStyle::Body.resolve(ui.style()),
        theme::text(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::CollapsingHeader, true, open, name)
    });
    response
}

/// The sections whose fold differs from their default, for saving.
pub(crate) fn folds() -> BTreeSet<String> {
    STATE.with(|s| s.borrow().flipped.clone())
}

/// Restores saved folds (at startup).
pub(crate) fn set_folds(flipped: BTreeSet<String>) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.flipped = flipped;
        s.dirty = false;
    });
}

/// True once after a fold changed: the app saves the folds then.
pub(crate) fn take_folds_dirty() -> bool {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().dirty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Context, RawInput};

    /// CH-Q1: right-click a changed number row's label, choose Reset to
    /// Default: the value goes back and the row reports one change.
    #[test]
    fn right_click_resets_a_changed_row_to_its_default() {
        use eframe::egui::{Event, Modifiers, PointerButton};
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut value = 5.0_f64;
        let mut changes = 0;
        let step = |events: Vec<Event>, value: &mut f64, changes: &mut i32| {
            let out = ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300., 400.))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        scope(ui, "reset", |ui| {
                            let r = number_d(ui, "Size", value, 1.0, Unit::Scalar, 0.0..=10.0, "");
                            if r.changed() {
                                *changes += 1;
                            }
                        });
                    });
                },
            );
            out.platform_output
                .accesskit_update
                .expect("AccessKit on")
                .nodes
                .into_iter()
                .map(|(_, n)| n)
                .collect::<Vec<_>>()
        };
        let click = |at: Pos2, button: PointerButton| {
            [true, false].map(|pressed| Event::PointerButton {
                pos: at,
                button,
                pressed,
                modifiers: Modifiers::NONE,
            })
        };
        step(vec![], &mut value, &mut changes);
        let nodes = step(vec![], &mut value, &mut changes);
        let named = |n: &egui::accesskit::Node, text: &str| n.label().or(n.value()) == Some(text);
        let label = nodes
            .iter()
            .find(|n| named(n, "Size"))
            .and_then(|n| n.bounds())
            .expect("label");
        let at = pos2(
            ((label.x0 + label.x1) / 2.) as f32,
            ((label.y0 + label.y1) / 2.) as f32,
        );
        let mut events = vec![Event::PointerMoved(at)];
        events.extend(click(at, PointerButton::Secondary));
        step(events, &mut value, &mut changes);
        let nodes = step(vec![], &mut value, &mut changes);
        let reset = nodes
            .iter()
            .find(|n| named(n, "Reset to Default"))
            .and_then(|n| n.bounds())
            .expect("the row's menu offers Reset to Default");
        let at = pos2(
            ((reset.x0 + reset.x1) / 2.) as f32,
            ((reset.y0 + reset.y1) / 2.) as f32,
        );
        let mut events = vec![Event::PointerMoved(at)];
        events.extend(click(at, PointerButton::Primary));
        step(events, &mut value, &mut changes);
        assert_eq!(value, 1.0);
        assert_eq!(changes, 1, "one change, so one undo step for the caller");
        assert!(!take_reset(), "the choice is used once");
    }

    #[test]
    fn every_default_row_helper_takes_a_reset() {
        let ctx = Context::default();
        let (mut n, mut b, mut c, mut t) = (5.0_f64, true, 2_usize, "x".to_owned());
        let mut changed = Vec::new();
        run(&ctx, |ui| {
            STATE.with(|s| s.borrow_mut().reset = true);
            changed.push(number_d(ui, "N", &mut n, 1.0, Unit::Scalar, 0.0..=9.0, "").changed());
            STATE.with(|s| s.borrow_mut().reset = true);
            changed.push(check_d(ui, "B", &mut b, false, "").changed());
            STATE.with(|s| s.borrow_mut().reset = true);
            changed.push(choice_d(ui, "C", &mut c, &0, &[(0, "a"), (2, "b")], "").changed());
            STATE.with(|s| s.borrow_mut().reset = true);
            changed.push(text_d(ui, "T", &mut t, "", "").changed());
        });
        assert_eq!((n, b, c, t.as_str()), (1.0, false, 0, ""));
        assert!(changed.iter().all(|&c| c), "{changed:?}");
    }

    fn run(ctx: &Context, mut draw: impl FnMut(&mut Ui)) {
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300., 400.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| draw(ui));
            },
        );
    }

    #[test]
    fn labels_in_a_scope_share_the_longest_labels_edge() {
        // UIC-111: one right edge for every label in the tab, even across
        // sections, sized to the longest label.
        // The second frame lays every row out with the longest label.
        let ctx2 = Context::default();
        let mut lefts = Vec::new();
        for _ in 0..2 {
            run(&ctx2, |ui| {
                lefts.clear();
                scope(ui, "t", |ui| {
                    let (a, _) = frame(ui, Some("Size"), "");
                    let (b, _) = frame(ui, Some("A Much Longer Label"), "");
                    let (c, _) = frame(ui, None, "");
                    lefts.extend([a.field.left(), b.field.left(), c.field.left()]);
                });
            });
        }
        assert!(
            lefts.iter().all(|x| (x - lefts[0]).abs() < 0.5),
            "{lefts:?}"
        );
    }

    #[test]
    fn sections_fold_and_remember_their_state() {
        set_folds(BTreeSet::new());
        assert!(!is_folded("Transform", false));
        assert!(is_folded("Notes", true));
        flip("Transform");
        flip("Notes");
        assert!(is_folded("Transform", false));
        assert!(!is_folded("Notes", true));
        assert!(take_folds_dirty());
        assert!(!take_folds_dirty());
        let saved = folds();
        set_folds(BTreeSet::new());
        set_folds(saved);
        assert!(is_folded("Transform", false));
    }

    #[test]
    fn rows_are_twenty_high_with_two_between() {
        // UIC-120.
        let ctx = Context::default();
        let mut tops = Vec::new();
        run(&ctx, |ui| {
            tops.clear();
            scope(ui, "h", |ui| {
                for _ in 0..3 {
                    let (r, _) = frame(ui, Some("Row"), "");
                    tops.push((r.row.top(), r.row.height()));
                }
            });
        });
        assert_eq!(tops[0].1, ROW_HEIGHT);
        assert!(
            (tops[1].0 - tops[0].0 - ROW_HEIGHT - ROW_GAP).abs() < 0.01,
            "{tops:?}"
        );
    }
}
