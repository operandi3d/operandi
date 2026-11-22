//! Vertical tabs on a panel's right edge (UIC-100): full names rotated,
//! every tab drawn alike, the active tab's name in the accent colour and
//! the others' grey. The same look as the form's own tab strip.
use super::*;
use theme::chrome;

/// Width of the strip.
pub(super) const WIDTH: f32 = 22.;

/// One tab's height for `label`.
fn tab_height(ui: &Ui, label: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        FontId::proportional(crate::typography::STANDARD_SIZE),
        chrome::text(),
    );
    let scale = ui.ctx().pixels_per_point();
    ((galley.size().x + 10.).max(46.) * scale).ceil() / scale
}

/// Draws `labels` down `strip` and the content edge beside it. Returns the
/// clicked tab's index. Each tab's rect is recorded in `rects` (tests).
pub(super) fn show(
    ui: &mut Ui,
    id: &str,
    strip: Rect,
    labels: &[&str],
    active: usize,
    rects: &mut Vec<Rect>,
) -> Option<usize> {
    rects.clear();
    let mut clicked = None;
    let mut y = strip.top();
    for (index, label) in labels.iter().enumerate() {
        let is_active = index == active;
        let height = tab_height(ui, label);
        let hit = Rect::from_min_size(pos2(strip.left(), y), vec2(WIDTH, height));
        y += height + 2.;
        let response = ui.interact(hit, Id::new((id, index)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, is_active, *label)
        });
        chrome::own_activation(ui, &response);
        // UIC-100: every tab draws alike; the active one is told apart by
        // its name in the accent colour, the others' names plain grey.
        let rect = hit;
        let fill = if response.hovered() && !is_active {
            chrome::hover()
        } else {
            chrome::header()
        };
        ui.painter().rect_filled(
            rect,
            crate::toolbox::category_corners(index, labels.len()),
            fill,
        );
        let stroke = Stroke::new(1.0f32, chrome::border());
        let (l, t, b, r) = (
            rect.left(),
            rect.top() + 0.5,
            rect.bottom() - 0.5,
            rect.right() - 0.5,
        );
        ui.painter().line_segment([pos2(l, t), pos2(r, t)], stroke);
        ui.painter().line_segment([pos2(r, t), pos2(r, b)], stroke);
        ui.painter().line_segment([pos2(r, b), pos2(l, b)], stroke);
        let color = if is_active {
            crate::theme::orange()
        } else {
            chrome::disabled_text()
        };
        ui.painter()
            .add(crate::toolbox::category_text(ui, label, rect, color));
        rects.push(hit);
        if response.clicked() {
            clicked = Some(index);
        }
    }
    // The content edge, unbroken (no tab opens into the content).
    let edge = strip.left() + 0.5;
    ui.painter().line_segment(
        [pos2(edge, strip.top()), pos2(edge, strip.bottom())],
        Stroke::new(1.0f32, chrome::border()),
    );
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_active_tabs_name_is_in_the_accent_and_the_rest_grey() {
        let ctx = Context::default();
        let out = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let strip = Rect::from_min_size(pos2(200., 10.), vec2(WIDTH, 300.));
                show(ui, "t", strip, &["Mesh", "Display"], 1, &mut Vec::new());
            });
        });
        let colour = |name: &str| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == name => {
                        Some(t.galley.job.sections[0].format.color)
                    }
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(colour("Display"), crate::theme::orange());
        assert_eq!(colour("Mesh"), chrome::disabled_text());
    }
}
