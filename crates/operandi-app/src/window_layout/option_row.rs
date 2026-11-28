//! One layout for every option row in Operandi's property panels: panel
//! mode of the shared form (UIC-100..124), one row at a time. See
//! `form::rows`, which does the drawing; these helpers keep the older
//! call signatures so every panel shares it.
//!
//! Each row is label | status circle | field (UIC-110..112): labels sit
//! right-aligned on one edge shared across the tab, fields one column and
//! one width (UIC-113). Checkboxes and buttons have no left label
//! (UIC-115/116).
use crate::form::rows;
use eframe::egui::{Align, ComboBox, InnerResponse, Layout, Response, Ui, vec2};

/// Height of one option row (UIC-120).
pub(crate) const ROW_HEIGHT: f32 = crate::form::layout::panel::ROW_HEIGHT;

/// Just the label-column cell (and status circle space), for a row the
/// caller lays out itself inside `ui.horizontal` (Properties panels).
/// What follows starts at the field column.
pub(crate) fn label(ui: &mut Ui, label: &str) -> Response {
    rows::label_cell(ui, label)
}

/// Lays out `add` across the rest of the row, right to left: a number
/// field, colour swatch or dropdown stretches to the field column's right
/// edge (UIC-113).
pub(crate) fn fill<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width = rows::rest_width(ui).max(40.);
    ui.allocate_ui_with_layout(
        vec2(width, ROW_HEIGHT),
        // Main align Max: egui buttons and number fields place their text
        // by the layout's horizontal align, so this right-aligns the text.
        Layout::right_to_left(Align::Center).with_main_align(Align::Max),
        |ui| {
            ui.spacing_mut().interact_size = vec2(width, ROW_HEIGHT);
            ui.spacing_mut().combo_width = width;
            add(ui)
        },
    )
    .inner
}

/// A vector as one row per component (UIC-114): "Position X", then just
/// the axis ("Y", "Z"), each with its own field filling the field column.
/// `add` draws component `k`'s field. Every row shows `tip` on hover.
pub(crate) fn vector(
    ui: &mut Ui,
    label: &str,
    axes: &[&str],
    tip: &str,
    mut add: impl FnMut(&mut Ui, usize),
) {
    for (k, axis) in axes.iter().enumerate() {
        let shown = if k == 0 {
            format!("{label} {axis}")
        } else {
            (*axis).to_owned()
        };
        let response = field_row(ui, &shown, |ui| add(ui, k)).response;
        if !tip.is_empty() {
            response.on_hover_text(tip);
        }
    }
}

/// `row` with its control filling the field column (see `fill`).
pub(crate) fn field_row<R>(
    ui: &mut Ui,
    label: &str,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    row(ui, label, |ui| fill(ui, add))
}

/// A row: `label` in the label column, then whatever `add` draws in the
/// field column. The response is the label's.
pub(crate) fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    rows::row(ui, label, add)
}

/// A checkbox row (UIC-115): no left label, the box at the field column's
/// start and its name to the right. Clicking either toggles it.
pub(crate) fn check(ui: &mut Ui, label: &str, value: &mut bool) -> Response {
    rows::check(ui, label, value)
}

/// A dropdown row picking one of `options` (value, shown name). Returns true
/// when the choice changed.
pub(crate) fn choice<T: PartialEq + Clone>(
    ui: &mut Ui,
    id: &str,
    label: &str,
    value: &mut T,
    options: &[(T, &str)],
) -> InnerResponse<bool> {
    // UIC-118: "(mixed)" until picked; any pick then writes every item.
    let mixed = rows::take_mixed();
    let current = if mixed {
        rows::MIXED.to_owned()
    } else {
        options
            .iter()
            .find(|(v, _)| v == value)
            .map_or("", |(_, name)| *name)
            .to_owned()
    };
    dropdown(ui, id, label, &current, |ui| {
        let mut changed = false;
        for (option, name) in options {
            let on = !mixed && option == value;
            if ui.selectable_label(on, *name).clicked() && (mixed || option != value) {
                *value = option.clone();
                changed = true;
            }
        }
        changed
    })
}

/// A dropdown row with caller-drawn entries. `shown` is the closed text. The
/// inner value is `contents`' result, or `R::default()` while closed.
pub(crate) fn dropdown<R: Default>(
    ui: &mut Ui,
    id: &str,
    label: &str,
    shown: &str,
    contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    rows::row(ui, label, |ui| {
        let combo = ComboBox::from_id_salt(id)
            .width(ui.available_width().max(40.))
            .height(320.)
            // Long names truncate instead of widening the panel.
            .truncate()
            .selected_text(shown)
            .show_ui(ui, contents);
        InnerResponse::new(combo.inner.unwrap_or_default(), combo.response)
    })
    .inner
}

/// A read-only value row (text the user can't change here).
pub(crate) fn value(ui: &mut Ui, label: &str, value: &str) -> Response {
    rows::value(ui, label, value)
}

/// A full-width button with no left label (UIC-116).
pub(crate) fn button(ui: &mut Ui, label: &str) -> Response {
    rows::button(ui, label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Context, Pos2, RawInput, Rect};

    fn frame(draw: impl FnMut(&mut Ui)) -> egui::FullOutput {
        let ctx = Context::default();
        let mut draw = draw;
        ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300., 400.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| draw(ui));
            },
        )
    }

    /// Every label ends at the same x, and every control starts after it.
    #[test]
    fn labels_share_one_right_edge_and_controls_one_left_edge() {
        let mut labels = Vec::new();
        let mut controls = Vec::new();
        frame(|ui| {
            labels.clear();
            controls.clear();
            let r = row(ui, "Shift", |ui| ui.button("1 m"));
            labels.push(r.response.rect.right());
            controls.push(r.inner.rect.left());
            let r = row(ui, "Max. Smoothing Angle", |ui| ui.button("45°"));
            assert!(
                r.response.rect.width() > 40.,
                "label drawn: {:?}",
                r.response.rect
            );
            labels.push(r.response.rect.right());
            controls.push(r.inner.rect.left());
            let mut on = false;
            check(ui, "Group Polygons", &mut on);
            let mut v = 1;
            let d = choice(ui, "t", "Mode", &mut v, &[(1, "One"), (2, "Two")]);
            controls.push(d.response.rect.left());
            assert!(
                d.response.rect.right() <= ui.max_rect().right() + 0.5,
                "dropdown overflows: {:?} in {:?}",
                d.response.rect,
                ui.max_rect()
            );
        });
        let first = labels[0];
        assert!(labels.iter().all(|x| (x - first).abs() < 0.5), "{labels:?}");
        // Controls start in the control column, right of every label.
        assert!(
            controls.iter().all(|x| *x > first),
            "{controls:?} vs {first}"
        );
    }
}
