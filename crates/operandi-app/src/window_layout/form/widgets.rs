//! The fields of the form component, drawn into rects the layout gives
//! them. Shared by dialog and panel mode so both look the same (UIC-94,
//! UIC-125): a darker inset field with small rounded corners and no
//! shadow, the property panels' colours and font.
use super::{Kind, Row, Unit, Value};
use crate::theme;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Key, Pos2, Rect, Response,
    Sense, Shape, Stroke, StrokeKind, TextEdit, TextStyle, Ui, UiBuilder, WidgetInfo, WidgetType,
    pos2, vec2,
};
use std::collections::BTreeMap;

/// Corner radius of fields and buttons.
pub(super) const RADIUS: u8 = 2;
/// The down-triangle and mini-slider arrows' width at a field's right.
const END: f32 = 14.;
/// Text inset inside a field.
const INSET: f32 = 5.;
/// A choice list longer than this gets a filter field (UIC-42).
const FILTER_FROM: usize = 12;
/// What a mixed multi-selection shows (UIC-118).
pub(super) const MIXED: &str = "(mixed)";

/// Per-form state for fields: text being typed, a scrub in progress.
#[derive(Clone, Default)]
pub(super) struct Fields {
    /// Text typed into a number field that is still focused.
    drafts: BTreeMap<String, String>,
    /// Text fields drawn this frame (focus tracking for Enter, UIC-85).
    pub(super) text_ids: Vec<Id>,
    /// The field to focus on the next frame.
    pub(super) focus: Option<String>,
    pub(super) select_on_focus: bool,
    scrub: Option<crate::scrub::Scrub>,
    /// A long choice list's filter text.
    filter: String,
    /// The last number field's refusal (bad text), for the caller.
    pub(super) error: Option<String>,
    /// A text field's words were edited since focus last left the fields
    /// (N12: a field merely focused on opening isn't being typed in).
    pub(super) edited: bool,
}

impl Fields {
    /// Whether something was typed into the focused field (UIC-85).
    pub(super) fn typed(&self) -> bool {
        self.edited || !self.drafts.is_empty()
    }
    /// Commits the text typed into number fields (OK pressed while
    /// typing). Returns an error for text that isn't a number.
    pub(super) fn commit(
        &mut self,
        rows: &[Row],
        values: &mut super::Values,
    ) -> Result<(), String> {
        let drafts = std::mem::take(&mut self.drafts);
        for (key, text) in drafts {
            let Some(row) = rows.iter().find(|r| r.key == key) else {
                continue;
            };
            if let Kind::Number { unit, min, max } = row.kind {
                match unit.parse(&text) {
                    Ok(v) => values.set(&key, Value::Number(v.clamp(min, max))),
                    Err(e) => {
                        self.drafts.insert(key, text);
                        return Err(format!("{}: {e}", row.label));
                    }
                }
            }
        }
        Ok(())
    }
}

/// What a field did this frame.
pub(super) struct Out {
    pub(super) response: Response,
    pub(super) changed: bool,
}

fn body(ui: &Ui) -> FontId {
    TextStyle::Body.resolve(ui.style())
}

fn text_colour(ui: &Ui) -> Color32 {
    if ui.is_enabled() {
        theme::text()
    } else {
        theme::disabled_text()
    }
}

/// The dark inset every field sits on.
fn paint_field(ui: &Ui, rect: Rect, hovered: bool) {
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS), theme::field());
    let stroke = if hovered && ui.is_enabled() {
        theme::hover_outline()
    } else {
        theme::border()
    };
    painter.rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0f32, stroke),
        StrokeKind::Inside,
    );
}

/// `text` fitted into `width`, cut with "…" when it doesn't fit (UIC-111).
/// Returns the galley and whether it was cut.
pub(super) fn fitted(
    ui: &Ui,
    text: &str,
    width: f32,
    color: Color32,
) -> (std::sync::Arc<egui::Galley>, bool) {
    let font = body(ui);
    let painter = ui.painter();
    // An instance's overridden setting reads in italics (shader layer
    // menu §3).
    let italic = super::rows::is_overridden(text);
    let layout = |t: String| {
        if italic {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &t,
                0.,
                egui::TextFormat {
                    font_id: font.clone(),
                    color,
                    italics: true,
                    ..Default::default()
                },
            );
            painter.layout_job(job)
        } else {
            painter.layout_no_wrap(t, font.clone(), color)
        }
    };
    let full = layout(text.to_owned());
    if full.size().x <= width {
        return (full, false);
    }
    let chars: Vec<char> = text.chars().collect();
    for n in (0..chars.len()).rev() {
        let cut: String = chars[..n].iter().collect::<String>().trim_end().to_owned() + "…";
        let galley = layout(cut);
        if galley.size().x <= width {
            return (galley, true);
        }
    }
    (layout("…".into()), true)
}

/// Width of `text` in the body font.
pub(super) fn text_width(ui: &Ui, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), body(ui), Color32::WHITE)
        .size()
        .x
}

/// A label right-aligned against `rect`'s right edge (UIC-86), dim when the
/// row is greyed (UIC-92), cut with "…" when too long (UIC-111). The
/// tooltip covers the row's help and, when cut, the full label.
pub(super) fn label(ui: &mut Ui, rect: Rect, text: &str, tooltip: &str) -> Response {
    let (galley, cut) = fitted(ui, text, rect.width(), text_colour(ui));
    let at = pos2(
        rect.right() - galley.size().x,
        rect.center().y - galley.size().y / 2.,
    );
    let shown = Rect::from_min_size(at, galley.size());
    let response = ui.interact(shown, ui.id().with(("label", text)), Sense::hover());
    ui.painter().galley(at, galley, text_colour(ui));
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, text));
    let help = match (cut, tooltip.is_empty()) {
        (true, true) => text.to_owned(),
        (true, false) => format!("{text}\n{tooltip}"),
        (false, _) => tooltip.to_owned(),
    };
    hover(response, &help)
}

/// Shows `tooltip` whether or not the widget is enabled (UIC-119),
/// without spec codes or command lines (`ui_text::hover_text`): every
/// form row, Preferences included, comes through here.
pub(super) fn hover(response: Response, tooltip: &str) -> Response {
    if tooltip.is_empty() {
        response
    } else {
        let tooltip = crate::ui_text::hover_text(tooltip);
        response
            .on_hover_text(&tooltip)
            .on_disabled_hover_text(&tooltip)
    }
}

/// Draws `row`'s field into `rect`. `mixed` shows "(mixed)" (UIC-118).
pub(super) fn field(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    row: &Row,
    value: &mut Value,
    fields: &mut Fields,
    mixed: bool,
) -> Out {
    match (&row.kind, value) {
        (Kind::Number { unit, min, max }, Value::Number(v)) => {
            number(ui, rect, id, row, *unit, (*min, *max), v, fields, mixed)
        }
        (Kind::Check, Value::Bool(on)) => check(ui, rect, id, &row.label, on, mixed),
        (Kind::Choice(options), Value::Choice(i)) => {
            choice(ui, rect, id, options, i, fields, mixed)
        }
        (Kind::Segments(options), Value::Choice(i)) => segments(ui, rect, id, options, i),
        (Kind::Text { suggestions }, Value::Text(text)) => {
            text_field(ui, rect, id, row, suggestions, text, fields)
        }
        (Kind::Colour, Value::Colour(rgb)) => colour(ui, rect, rgb),
        (Kind::Button, _) => button(ui, rect, id, &row.label),
        (Kind::Message, _) => message(ui, rect, &row.label),
        _ => {
            // A value of the wrong kind; `check_rows` refuses these, so
            // only a caller bug gets here. Draw an empty field.
            paint_field(ui, rect, false);
            Out {
                response: ui.interact(rect, id, Sense::hover()),
                changed: false,
            }
        }
    }
}

/// UIC-90: the standard number field, value left-aligned with its unit,
/// mini-slider arrows at the right (drag them to change the value; Shift
/// coarser, Option finer, as in Tool Properties).
#[allow(clippy::too_many_arguments)]
fn number(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    row: &Row,
    unit: Unit,
    (min, max): (f64, f64),
    value: &mut f64,
    fields: &mut Fields,
    mixed: bool,
) -> Out {
    let text_id = id.with("text");
    let focused = ui.memory(|m| m.has_focus(text_id));
    let arrows = Rect::from_min_max(pos2(rect.right() - END, rect.top()), rect.max);
    let text_rect = Rect::from_min_max(rect.min, pos2(arrows.left(), rect.bottom()));
    let hovered = ui.rect_contains_pointer(rect);
    paint_field(ui, rect, hovered);
    // A draft stays while focused, and after a refused commit so the bad
    // text can be fixed (it shows outlined in the error colour).
    let draft = fields.drafts.get(&row.key).cloned();
    let refused = draft.is_some() && !focused;
    let mut text = match draft {
        Some(draft) => draft,
        None if mixed => MIXED.to_owned(),
        None => unit.format(*value),
    };
    if refused {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(RADIUS),
            Stroke::new(1.0f32, theme::error()),
            StrokeKind::Inside,
        );
    }
    let edit = ui.put(
        text_rect.shrink2(vec2(INSET, 0.)),
        TextEdit::singleline(&mut text)
            .id(text_id)
            .frame(false)
            .margin(vec2(0., 2.))
            .horizontal_align(Align::LEFT)
            .vertical_align(Align::Center)
            .text_color(text_colour(ui)),
    );
    fields.text_ids.push(text_id);
    if fields.focus.as_deref() == Some(row.key.as_str()) {
        edit.request_focus();
        fields.focus = None;
    }
    let mut changed = false;
    if edit.has_focus() && edit.gained_focus() && !refused && mixed {
        text = unit.format(*value);
    }
    // Tab and Shift+Tab move between the form's fields (UIC-51).
    crate::numeric_tab::register(ui, &edit, Some(&text));
    if edit.has_focus() {
        fields.drafts.insert(row.key.clone(), text);
    }
    if edit.lost_focus() {
        // Enter or a click elsewhere commits the typed value (UIC-85).
        // Escape cancels the dialog, so its text needn't be read.
        if let Some(draft) = fields.drafts.remove(&row.key)
            && !ui.input(|i| i.key_pressed(Key::Escape))
        {
            match unit.parse(&draft) {
                Ok(v) => {
                    let v = v.clamp(min, max);
                    changed = v != *value;
                    *value = v;
                    fields.error = None;
                }
                Err(e) => {
                    fields.error = Some(format!("{}: {e}", row.label));
                    fields.drafts.insert(row.key.clone(), draft);
                }
            }
        }
    }
    // The mini-slider arrows (UIC-90).
    let drag = ui
        .interact(arrows, id.with("scrub"), Sense::drag())
        .on_hover_cursor(CursorIcon::ResizeHorizontal);
    let colour = if drag.dragged() {
        theme::text()
    } else if drag.hovered() && ui.is_enabled() {
        theme::weak_text()
    } else if ui.is_enabled() {
        theme::disabled_text()
    } else {
        theme::disabled_text().gamma_multiply(0.6)
    };
    paint_arrows(ui, arrows, colour);
    // UIC-90/113: every number field has the arrows, whole numbers too;
    // a greyed field keeps them, greyed. Named for tests and screen readers.
    let enabled = ui.is_enabled();
    let name = format!("{} arrows", row.label);
    drag.widget_info(|| WidgetInfo::labeled(WidgetType::Slider, enabled, &name));
    if drag.drag_started() {
        fields.scrub = Some(crate::scrub::Scrub::new(
            &row.key,
            unit.scrub(),
            *value,
            unit.format(*value),
        ));
    }
    if drag.dragged()
        && let Some(scrub) = fields.scrub.as_mut().filter(|s| s.key == row.key)
    {
        // F1: the physical Ctrl key (never Cmd) makes a drag finer, like
        // Option; Shift coarser.
        let mut modifiers = ui.input(|i| i.modifiers);
        modifiers.alt |= modifiers.ctrl;
        let change = scrub.advance(drag.drag_delta().x, modifiers) * unit.scrub_scale();
        let v = (scrub.start + change).clamp(min, max);
        scrub.accept(change, (v - scrub.start) / unit.scrub_scale());
        if v != *value {
            *value = v;
            changed = true;
        }
    }
    if drag.drag_stopped() {
        fields.scrub = None;
    }
    // Min/max labels (an approved change to UIC-90); the row made room.
    if let Some(limits) = super::range_labels::shown(min, max) {
        let at = (!mixed).then_some(*value);
        super::range_labels::paint(ui, rect, at, limits, |v| unit.format(v));
    }
    let mut response = edit | drag;
    if changed {
        response.mark_changed();
    }
    Out { response, changed }
}

/// Two small opposed triangles, one each side of the centre.
fn paint_arrows(ui: &Ui, rect: Rect, color: Color32) {
    let c = rect.center();
    let (w, h, gap) = (3., 3.5, 1.);
    for points in [
        vec![
            pos2(c.x - gap - w, c.y),
            pos2(c.x - gap, c.y - h),
            pos2(c.x - gap, c.y + h),
        ],
        vec![
            pos2(c.x + gap + w, c.y),
            pos2(c.x + gap, c.y + h),
            pos2(c.x + gap, c.y - h),
        ],
    ] {
        ui.painter()
            .add(Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

/// A small down-pointing triangle centred at `c`.
fn paint_down(ui: &Ui, c: Pos2, color: Color32) {
    let (w, h) = (4., 3.5);
    ui.painter().add(Shape::convex_polygon(
        vec![
            pos2(c.x - w, c.y - h / 2.),
            pos2(c.x + w, c.y - h / 2.),
            pos2(c.x, c.y + h / 2.),
        ],
        color,
        Stroke::NONE,
    ));
}

/// UIC-87 / UIC-50: the box at the start of the field column, its text to
/// the right; dark square off, accent square with a dark tick on. Clicking
/// the box or the text toggles it.
fn check(ui: &mut Ui, rect: Rect, id: Id, text: &str, on: &mut bool, mixed: bool) -> Out {
    use super::layout::dialog::{CHECK, CHECK_GAP};
    let mut response = ui.interact(rect, id, Sense::click());
    let boxed = Rect::from_min_size(
        pos2(rect.left(), rect.center().y - CHECK / 2.),
        vec2(CHECK, CHECK),
    );
    let enabled = ui.is_enabled();
    let fill = if *on && !mixed {
        if enabled {
            theme::orange()
        } else {
            theme::orange().gamma_multiply(0.45)
        }
    } else {
        theme::field()
    };
    let painter = ui.painter();
    painter.rect_filled(boxed, CornerRadius::same(RADIUS), fill);
    let edge = if response.hovered() && enabled {
        theme::hover_outline()
    } else {
        theme::border()
    };
    painter.rect_stroke(
        boxed,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0f32, edge),
        StrokeKind::Inside,
    );
    if mixed {
        let y = boxed.center().y;
        painter.line_segment(
            [pos2(boxed.left() + 3.5, y), pos2(boxed.right() - 3.5, y)],
            Stroke::new(1.6f32, text_colour(ui)),
        );
    } else if *on {
        let r = boxed.shrink(3.);
        painter.add(Shape::line(
            vec![
                pos2(r.left(), r.center().y),
                pos2(r.left() + r.width() * 0.38, r.bottom() - 0.5),
                pos2(r.right(), r.top() + 0.5),
            ],
            Stroke::new(1.8f32, theme::on_orange()),
        ));
    }
    let (galley, _) = fitted(
        ui,
        text,
        (rect.width() - CHECK - CHECK_GAP).max(0.),
        text_colour(ui),
    );
    let at = pos2(
        boxed.right() + CHECK_GAP,
        rect.center().y - galley.size().y / 2.,
    );
    ui.painter().galley(at, galley, text_colour(ui));
    let mut changed = false;
    if response.clicked() {
        *on = !*on;
        changed = true;
        response.mark_changed();
    }
    let selected = *on;
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, selected, text));
    Out { response, changed }
}

/// UIC-88: a full-width dark field, the current choice on the left and a
/// small down-triangle on the right; a click opens the list. Long lists
/// get a filter field at the top (UIC-42).
fn choice(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    options: &[String],
    index: &mut usize,
    fields: &mut Fields,
    mixed: bool,
) -> Out {
    let mut response = ui.interact(rect, id, Sense::click());
    paint_field(ui, rect, response.hovered());
    let shown = if mixed {
        MIXED
    } else {
        options.get(*index).map_or("", |o| leaf(o))
    };
    let (galley, _) = fitted(ui, shown, rect.width() - INSET - END, text_colour(ui));
    let at = pos2(rect.left() + INSET, rect.center().y - galley.size().y / 2.);
    ui.painter().galley(at, galley, text_colour(ui));
    paint_down(
        ui,
        pos2(rect.right() - END / 2. - 1., rect.center().y),
        text_colour(ui),
    );
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, enabled, shown));
    let mut changed = false;
    let picked = egui::Popup::menu(&response)
        .width(rect.width())
        .show(|ui| {
            ui.set_min_width(rect.width() - 8.);
            let mut picked = None;
            let filter = if options.len() > FILTER_FROM {
                let edit = ui.add(
                    TextEdit::singleline(&mut fields.filter)
                        .hint_text("Filter")
                        .desired_width(f32::INFINITY),
                );
                crate::numeric_tab::register(ui, &edit, Some(&fields.filter));
                if edit.gained_focus() || !edit.has_focus() && fields.filter.is_empty() {
                    edit.request_focus();
                }
                fields.filter.to_lowercase()
            } else {
                String::new()
            };
            let current = (!mixed).then_some(*index);
            egui::ScrollArea::vertical()
                .max_height(320.)
                .show(ui, |ui| {
                    if filter.is_empty() {
                        let all: Vec<(usize, &str)> =
                            options.iter().map(String::as_str).enumerate().collect();
                        choice_entries(ui, &all, current, &mut picked);
                        return;
                    }
                    for (i, option) in options.iter().enumerate() {
                        if !option.to_lowercase().contains(&filter) {
                            continue;
                        }
                        let label = option.replace(super::SUBMENU, " / ");
                        if ui.selectable_label(Some(i) == current, label).clicked() {
                            picked = Some(i);
                        }
                    }
                });
            if picked.is_some() {
                ui.close();
            }
            picked
        })
        .and_then(|r| r.inner);
    if let Some(i) = picked {
        fields.filter.clear();
        if i != *index || mixed {
            *index = i;
            changed = true;
            response.mark_changed();
        }
    }
    Out { response, changed }
}

/// An option's last part (after any `SUBMENU` splits): what the field
/// shows.
fn leaf(option: &str) -> &str {
    option.rsplit(super::SUBMENU).next().unwrap_or(option)
}

/// A choice list's entries: plain options in order, and options that
/// start with `group ▸` gathered into one submenu per group where the
/// group's first option sits (MC-05).
fn choice_entries(
    ui: &mut Ui,
    options: &[(usize, &str)],
    current: Option<usize>,
    picked: &mut Option<usize>,
) {
    let mut done = std::collections::HashSet::new();
    for &(i, option) in options {
        match option.split_once(super::SUBMENU) {
            None => {
                if ui.selectable_label(Some(i) == current, option).clicked() {
                    *picked = Some(i);
                }
            }
            Some((group, _)) => {
                if !done.insert(group) {
                    continue;
                }
                let inner: Vec<(usize, &str)> = options
                    .iter()
                    .filter_map(|&(j, o)| {
                        o.split_once(super::SUBMENU)
                            .filter(|(g, _)| *g == group)
                            .map(|(_, rest)| (j, rest))
                    })
                    .collect();
                let holds_current = inner.iter().any(|(j, _)| Some(*j) == current);
                let text = if holds_current {
                    egui::RichText::new(group).strong()
                } else {
                    egui::RichText::new(group)
                };
                ui.menu_button(text, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(420.)
                        .show(ui, |ui| choice_entries(ui, &inner, current, picked));
                    if picked.is_some() {
                        ui.close();
                    }
                });
            }
        }
    }
}

/// UIC-89: 2–4 short options as equal joined segments; the chosen one is
/// filled with the accent, the rest with the field colour.
fn segments(ui: &mut Ui, rect: Rect, id: Id, options: &[String], index: &mut usize) -> Out {
    let n = options.len().max(1);
    let width = rect.width() / n as f32;
    let mut response = ui.interact(rect, id, Sense::hover());
    let mut changed = false;
    let enabled = ui.is_enabled();
    for (i, option) in options.iter().enumerate() {
        let cell = Rect::from_min_size(
            pos2(rect.left() + width * i as f32, rect.top()),
            vec2(width, rect.height()),
        );
        let r = ui.interact(cell, id.with(i), Sense::click());
        let chosen = i == *index;
        let fill = match (chosen, enabled) {
            (true, true) => theme::orange(),
            (true, false) => theme::orange().gamma_multiply(0.45),
            (false, _) if r.hovered() && enabled => theme::control(),
            _ => theme::field(),
        };
        let radius = CornerRadius {
            nw: if i == 0 { RADIUS } else { 0 },
            sw: if i == 0 { RADIUS } else { 0 },
            ne: if i + 1 == n { RADIUS } else { 0 },
            se: if i + 1 == n { RADIUS } else { 0 },
        };
        ui.painter().rect_filled(cell, radius, fill);
        let color = if chosen && enabled {
            theme::on_orange()
        } else {
            text_colour(ui)
        };
        let (galley, _) = fitted(ui, option, cell.width() - 4., color);
        ui.painter()
            .galley(cell.center() - galley.size() / 2., galley, color);
        r.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, enabled, chosen, option));
        if r.clicked() && !chosen {
            *index = i;
            changed = true;
        }
        response |= r;
    }
    // One outline around the joined row, with thin separators.
    let painter = ui.painter();
    painter.rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0f32, theme::border()),
        StrokeKind::Inside,
    );
    for i in 1..n {
        let x = rect.left() + width * i as f32;
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0f32, theme::border()),
        );
    }
    if changed {
        response.mark_changed();
    }
    Out { response, changed }
}

/// UIC-91: free text in the same dark field, left-aligned. With
/// suggestions, a down-triangle at the right lists existing names.
fn text_field(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    row: &Row,
    suggestions: &[String],
    text: &mut String,
    fields: &mut Fields,
) -> Out {
    let has_list = !suggestions.is_empty();
    let list_rect = Rect::from_min_max(pos2(rect.right() - END, rect.top()), rect.max);
    let text_rect = if has_list {
        Rect::from_min_max(rect.min, pos2(list_rect.left(), rect.bottom()))
    } else {
        rect
    };
    paint_field(ui, rect, ui.rect_contains_pointer(rect));
    let text_id = id.with("text");
    let before = text.clone();
    let mut response = ui.put(
        text_rect.shrink2(vec2(INSET, 0.)),
        TextEdit::singleline(text)
            .id(text_id)
            .frame(false)
            .margin(vec2(0., 2.))
            .horizontal_align(Align::LEFT)
            .vertical_align(Align::Center)
            .text_color(text_colour(ui)),
    );
    fields.text_ids.push(text_id);
    crate::numeric_tab::register(ui, &response, Some(text));
    if fields.focus.as_deref() == Some(row.key.as_str()) {
        if fields.select_on_focus {
            crate::numeric_tab::focus_and_select_on_open(
                &response,
                text,
                &mut fields.select_on_focus,
            );
        } else {
            response.request_focus();
        }
        fields.focus = None;
    }
    if has_list {
        let arrow = ui
            .interact(list_rect, id.with("list"), Sense::click())
            .on_hover_text("Existing names");
        let enabled = ui.is_enabled();
        arrow.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, enabled, "Existing names"));
        paint_down(ui, list_rect.center(), text_colour(ui));
        let picked = egui::Popup::menu(&arrow)
            .width(rect.width())
            .align(egui::RectAlign::BOTTOM_END)
            .show(|ui| {
                ui.set_min_width(rect.width() - 8.);
                let mut picked = None;
                egui::ScrollArea::vertical()
                    .max_height(320.)
                    .show(ui, |ui| {
                        for name in suggestions {
                            if ui.selectable_label(name == text, name).clicked() {
                                picked = Some(name.clone());
                            }
                        }
                    });
                if picked.is_some() {
                    ui.close();
                }
                picked
            })
            .and_then(|r| r.inner);
        if let Some(name) = picked {
            *text = name;
        }
        response |= arrow;
    }
    let changed = *text != before;
    if changed {
        response.mark_changed();
        fields.edited = true;
    }
    Out { response, changed }
}

/// The shared colour field (swatch and numbers), at the row's height.
fn colour(ui: &mut Ui, rect: Rect, rgb: &mut [f64; 3]) -> Out {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
    child.spacing_mut().interact_size.y = rect.height();
    let out = crate::colour_field::show(&mut child, rgb, crate::colour_field::Options::ui());
    Out {
        response: out.response,
        changed: out.changed,
    }
}

/// UIC-116: a button the full width of the field column.
fn button(ui: &mut Ui, rect: Rect, id: Id, text: &str) -> Out {
    let response = ui.interact(rect, id, Sense::click());
    paint_button(ui, rect, &response, text, None, false);
    let changed = response.clicked();
    Out { response, changed }
}

/// A form button: label left-aligned, the key hint right-aligned and dim
/// (UIC-84).
/// The width a `paint_button` needs to show `text` (and `hint`) whole.
pub(super) fn button_width_for(ui: &Ui, text: &str, hint: Option<&str>) -> f32 {
    let font = body(ui);
    let width = |t: &str, f: FontId| {
        ui.painter()
            .layout_no_wrap(t.to_owned(), f, Color32::WHITE)
            .size()
            .x
    };
    let mut needed = (INSET + 1.) + width(text, font.clone()) + INSET;
    if let Some(hint) = hint {
        // A small gap between the label and its dim key.
        needed += 2. + width(hint, FontId::new(font.size - 1.5, font.family));
    }
    needed.ceil()
}

pub(super) fn paint_button(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    text: &str,
    hint: Option<&str>,
    default: bool,
) {
    let enabled = ui.is_enabled() && response.enabled();
    let fill = if !enabled {
        theme::control().gamma_multiply(0.7)
    } else if response.is_pointer_button_down_on() {
        theme::pressed()
    } else if response.hovered() {
        theme::hover()
    } else {
        theme::control()
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS), fill);
    let edge = if default && enabled {
        theme::hover_outline()
    } else {
        theme::border()
    };
    painter.rect_stroke(
        rect,
        CornerRadius::same(RADIUS),
        Stroke::new(1.0f32, edge),
        StrokeKind::Inside,
    );
    let color = if enabled {
        theme::text()
    } else {
        theme::disabled_text()
    };
    let font = body(ui);
    painter.text(
        pos2(rect.left() + INSET + 1., rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font.clone(),
        color,
    );
    if let Some(hint) = hint {
        painter.text(
            pos2(rect.right() - INSET, rect.center().y),
            Align2::RIGHT_CENTER,
            hint,
            FontId::new(font.size - 1.5, font.family),
            theme::disabled_text(),
        );
    }
    let label = text.to_owned();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &label));
}

/// UIC-97: plain text in the form area.
fn message(ui: &mut Ui, rect: Rect, text: &str) -> Out {
    let galley = ui
        .painter()
        .layout(text.to_owned(), body(ui), theme::text(), rect.width());
    let response = ui.interact(rect, ui.id().with(("message", text)), Sense::hover());
    let at = pos2(rect.left(), rect.center().y - galley.size().y / 2.);
    ui.painter().galley(at, galley, theme::text());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
    Out {
        response,
        changed: false,
    }
}

/// A message's height when wrapped at `width`.
pub(super) fn message_height(ui: &Ui, text: &str, width: f32) -> f32 {
    ui.painter()
        .layout(text.to_owned(), body(ui), Color32::WHITE, width)
        .size()
        .y
}

/// The width a row's field needs to show itself whole.
pub(super) fn natural_width(ui: &Ui, row: &Row) -> f32 {
    use super::layout::dialog::{CHECK, CHECK_GAP};
    match &row.kind {
        Kind::Check => CHECK + CHECK_GAP + text_width(ui, &row.label) + 2.,
        Kind::Choice(options) => {
            options
                .iter()
                .map(|o| text_width(ui, leaf(o)))
                .fold(0., f32::max)
                + INSET
                + END
                + 4.
        }
        Kind::Segments(options) => {
            options
                .iter()
                .map(|o| text_width(ui, o) + 12.)
                .fold(0., f32::max)
                * options.len() as f32
        }
        Kind::Message => text_width(ui, &row.label)
            .min(super::layout::dialog::MAX_MESSAGE)
            .ceil(),
        _ => 0.,
    }
}
