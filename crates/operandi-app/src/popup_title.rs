//! The popup title bar every pop-over shares (VP-10..19, UI bible): the
//! title centred, the pin circle at the far right (hollow = not pinned,
//! filled accent = pinned; clicking it pins, or closes a pinned one), and
//! the rest of the bar drags the popup.
use crate::theme;
use eframe::egui::{self, FontId, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

/// What the title bar asked for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum TitleAction {
    #[default]
    None,
    /// The circle was clicked: pin, or (pinned) close.
    Circle,
    /// The title bar was dragged by this much.
    Drag(Vec2),
}

/// The title bar (VP-10..19): the name centred, the pin circle at the far
/// right (filled accent when pinned, hollow when not; no separate close
/// button). Dragging the rest of the bar reports the drag.
pub(crate) fn title_bar(ui: &mut Ui, title: &str, pinned: bool) -> TitleAction {
    let (bar, bar_response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 20.), Sense::drag());
    let circle = Rect::from_center_size(pos2(bar.right() - 9., bar.center().y), Vec2::splat(14.));
    // The title stays clear of the circle (and centred on the bar).
    let room = (bar.width() - 2. * 22.).max(20.);
    let mut job = egui::text::LayoutJob::single_section(
        title.to_owned(),
        egui::TextFormat {
            font_id: FontId::proportional(13.),
            color: theme::text(),
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: room,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    let at = bar.center() - galley.size() / 2.;
    ui.painter().galley(at, galley, theme::text());
    let response = ui
        .interact(circle, ui.id().with("popup-pin"), Sense::click())
        .on_hover_text(if pinned {
            "Close"
        } else {
            "Pin: keep this panel open as a floating panel"
        });
    let painter = ui.painter();
    if pinned {
        painter.circle_filled(circle.center(), 5.5, theme::orange());
    } else {
        let color = if response.hovered() {
            theme::text()
        } else {
            theme::weak_text()
        };
        painter.circle_stroke(circle.center(), 5.5, Stroke::new(1.2_f32, color));
    }
    if response.clicked() {
        TitleAction::Circle
    } else if bar_response.dragged() && bar_response.drag_delta() != Vec2::ZERO {
        TitleAction::Drag(bar_response.drag_delta())
    } else {
        TitleAction::None
    }
}
