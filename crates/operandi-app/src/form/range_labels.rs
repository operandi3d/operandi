//! Min/max labels on number fields (Preferences > Defaults > UI > Slider
//! Min/Max Labels, `pref.value ui.slider_min_max_labels`). An approved
//! change to UIC-90 (Jason, 2026-11-09, option A).
//!
//! When the preference is on, a number field whose limits are both finite
//! gets a thin bar along its bottom edge showing where the value sits
//! between them, and the two limits, with their unit, in small grey text
//! under its two ends. The row is `EXTRA` taller to make room. Fields with
//! an open limit get nothing, and with the preference off every field and
//! row is exactly as before.
//!
//! Every number field draws through here: the form's number field
//! (`widgets::number`, so dialogs and panel rows), the value boxes
//! (`numeric_tab::DragValue`) and the Tool Properties fields.
use eframe::egui::{Align2, FontId, Rect, Stroke, Ui, pos2, vec2};
use std::cell::Cell;

use crate::theme;

/// How much taller a row with labels is.
pub(crate) const EXTRA: f32 = 8.;

/// Limits this far out are guards against runaway values rather than a
/// range anyone works within (a field clamped to ±1e9, say), so they count
/// as open. Provisional (MINMAX-Q01).
const GUARD: f64 = 1e6;

/// The labels' text size.
const TEXT_SIZE: f32 = 8.;

// The UI runs on one thread per app (and one per test), like the theme's
// Popup Transparency.
thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
}

/// Mirrors `UiPreferences::slider_min_max_labels` (set by `apply`).
pub(crate) fn set_on(on: bool) {
    ON.with(|c| c.set(on));
}

/// Whether the preference is on.
pub(crate) fn on() -> bool {
    ON.with(Cell::get)
}

/// Whether `min..=max` is a real, closed range.
pub(crate) fn closed(min: f64, max: f64) -> bool {
    min.is_finite() && max.is_finite() && min < max && min.abs() < GUARD && max.abs() < GUARD
}

/// The limits to label, when the preference is on and both are finite.
pub(crate) fn shown(min: f64, max: f64) -> Option<(f64, f64)> {
    (on() && closed(min, max)).then_some((min, max))
}

/// The extra row height a field with these limits needs (0 when off).
pub(crate) fn extra(min: f64, max: f64) -> f32 {
    if shown(min, max).is_some() { EXTRA } else { 0. }
}

/// The band under `field` the labels sit in.
fn band(field: Rect) -> Rect {
    Rect::from_min_size(
        pos2(field.left(), field.bottom()),
        vec2(field.width(), EXTRA),
    )
}

/// Paints the range bar along `field`'s bottom edge (at `value`; none for a
/// mixed value) and the limits, formatted by `format`, under its ends.
/// The caller makes room for them (`EXTRA`, or `reserve`).
pub(crate) fn paint(
    ui: &Ui,
    field: Rect,
    value: Option<f64>,
    (min, max): (f64, f64),
    format: impl Fn(f64) -> String,
) {
    let enabled = ui.is_enabled();
    let painter = ui.painter();
    // The bar: a faint track across the field, filled up to the value.
    let y = field.bottom() - 1.5;
    let (left, right) = (field.left() + 2., field.right() - 2.);
    if right > left {
        let track = theme::border();
        painter.hline(left..=right, y, Stroke::new(2.0f32, track));
        if let Some(v) = value.filter(|v| v.is_finite()) {
            let t = ((v - min) / (max - min)).clamp(0., 1.) as f32;
            let fill = if enabled {
                theme::orange()
            } else {
                theme::orange().gamma_multiply(0.45)
            };
            let end = left + (right - left) * t;
            if end > left {
                painter.hline(left..=end, y, Stroke::new(2.0f32, fill));
            }
        }
    }
    let colour = if enabled {
        theme::weak_text()
    } else {
        theme::disabled_text()
    };
    let font = FontId::proportional(TEXT_SIZE);
    let band = band(field);
    painter.text(
        pos2(band.left() + 1., band.top()),
        Align2::LEFT_TOP,
        format(min),
        font.clone(),
        colour,
    );
    painter.text(
        pos2(band.right() - 1., band.top()),
        Align2::RIGHT_TOP,
        format(max),
        font,
        colour,
    );
}

/// For a field drawn straight into `ui` rather than into a row that already
/// made room: takes the band under `field` in `ui`, so whatever comes next
/// starts below the labels. In a vertical layout that is space after the
/// field; in a horizontal one it makes the line taller, which the layouts
/// holding it pass on.
pub(crate) fn reserve(ui: &mut Ui, field: Rect) {
    if ui.layout().main_dir().is_vertical() {
        ui.add_space(EXTRA);
    } else {
        ui.expand_to_include_rect(band(field));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_closed_ranges_are_labelled_and_only_when_on() {
        set_on(false);
        assert_eq!(shown(0., 90.), None);
        assert_eq!(extra(0., 90.), 0.);
        set_on(true);
        assert_eq!(shown(0., 90.), Some((0., 90.)));
        assert_eq!(extra(0., 90.), EXTRA);
        for (lo, hi) in [
            (f64::NEG_INFINITY, f64::INFINITY),
            (0., f64::INFINITY),
            (f64::NEG_INFINITY, 1.),
            (f64::MIN, f64::MAX),
            (-1e9, 1e9),
            (f64::NAN, 1.),
            (1., 1.),
        ] {
            assert_eq!(shown(lo, hi), None, "{lo}..{hi}");
        }
        set_on(false);
    }

    use crate::form::{Row, Unit, rows};
    use eframe::egui::{self, Context, Pos2, RawInput, Shape};

    /// Draws one frame and returns each drawn row's height step (the
    /// cursor's move) and every text painted.
    fn frame(on: bool) -> (Vec<f32>, Vec<String>) {
        set_on(on);
        let ctx = Context::default();
        let mut steps = Vec::new();
        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(320., 600.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    rows::scope(ui, "range-labels-test", |ui| {
                        let mut step = |ui: &mut Ui, draw: &mut dyn FnMut(&mut Ui)| {
                            let top = ui.cursor().top();
                            draw(ui);
                            steps.push(ui.cursor().top() - top);
                        };
                        let (mut a, mut b, mut c, mut d, mut e) = (30_u8, 0.5, 40.0, 7.0, 3.0);
                        // A form row with closed limits, then an open one.
                        step(ui, &mut |ui| {
                            rows::number_d(ui, "Closed", &mut a, 0, Unit::Percent, 0..=90, "");
                        });
                        step(ui, &mut |ui| {
                            let open = f64::NEG_INFINITY..=f64::INFINITY;
                            rows::number_d(ui, "Open", &mut b, 0., Unit::Scalar, open, "");
                        });
                        // A value box in a field row, with and without limits.
                        step(ui, &mut |ui| {
                            crate::option_row::field_row(ui, "Box", |ui| {
                                ui.add(
                                    crate::numeric_tab::DragValue::new(&mut c)
                                        .range(0.0..=100.0)
                                        .suffix(" °"),
                                )
                            });
                        });
                        step(ui, &mut |ui| {
                            crate::option_row::field_row(ui, "Free", |ui| {
                                ui.add(crate::numeric_tab::DragValue::new(&mut d))
                            });
                        });
                        // A value box in a hand-made horizontal row.
                        step(ui, &mut |ui| {
                            ui.horizontal(|ui| {
                                crate::option_row::label(ui, "Line");
                                crate::option_row::fill(ui, |ui| {
                                    ui.add(
                                        crate::numeric_tab::DragValue::new(&mut e)
                                            .range(1..=16)
                                            .max_decimals(0),
                                    )
                                })
                            });
                        });
                    });
                });
            },
        );
        set_on(false);
        let mut texts = Vec::new();
        for clipped in output.shapes {
            if let Shape::Text(text) = clipped.shape {
                texts.push(text.galley.text().to_owned());
            }
        }
        (steps, texts)
    }

    #[test]
    fn rows_grow_only_for_closed_limits_and_only_when_on() {
        let (off, off_texts) = frame(false);
        let (on, on_texts) = frame(true);
        // Off: the layout is today's, every form row one row high.
        let row = crate::form::layout::panel::ROW_HEIGHT + crate::form::layout::panel::ROW_GAP;
        assert_eq!(&off[..4], &[row; 4], "{off:?}");
        // On: closed rows are EXTRA taller, open ones unchanged.
        let grew: Vec<f32> = on.iter().zip(&off).map(|(a, b)| a - b).collect();
        for (i, want) in [EXTRA, 0., EXTRA, 0., EXTRA].into_iter().enumerate() {
            assert!((grew[i] - want).abs() < 0.01, "row {i}: {grew:?}");
        }
        // The limits, with their units, show only when on.
        for label in ["0 %", "90 %", "0 °", "100 °", "1", "16"] {
            assert!(on_texts.iter().any(|t| t == label), "{label}: {on_texts:?}");
        }
        for label in ["90 %", "100 °", "16"] {
            assert!(
                !off_texts.iter().any(|t| t == label),
                "{label}: {off_texts:?}"
            );
        }
    }

    #[test]
    fn panel_and_dialog_rows_take_the_extra_height_only_when_on() {
        let closed = Row::number("a", "A", Unit::Percent, 0.).range(0., 90.);
        let open = Row::number("b", "B", Unit::Scalar, 0.);
        assert_eq!(closed.label_extra(), 0.);
        set_on(true);
        assert_eq!(closed.label_extra(), EXTRA);
        assert_eq!(open.label_extra(), 0.);
        assert_eq!(Row::check("c", "C", false).label_extra(), 0.);
        set_on(false);
    }
}
