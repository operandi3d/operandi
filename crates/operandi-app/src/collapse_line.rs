//! The line a collapsed bar leaves behind (UIC-16; Jason 2026-11-18): the
//! top bar collapses from a click on the logo, a side bar from a
//! double-click on its resize line. Both leave the same line at their
//! window edge, which takes the theme's hover colour under the pointer.
//! A double-click on the line, or a drag outward past a few points, brings
//! the bar back; a single click does nothing. A collapsed side bar also
//! shows a small tab sticking out of its line toward the middle of the
//! window, which a click reopens (Jason 2026-11-20: the line sits at the
//! window edge, where a double-click often resizes the window instead;
//! UIC-16's "clicking the strip brings it back").
//!
//! The bars' state lives in `Layouts` (kept in layouts.json) and every
//! change runs an `operandi.bar.collapse` line (UIC-220).
use super::*;

/// Which bar a line stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bar {
    Top,
    Left,
    Right,
}

impl Bar {
    pub(super) fn name(self) -> &'static str {
        match self {
            Bar::Top => "top",
            Bar::Left => "left",
            Bar::Right => "right",
        }
    }
    pub(super) fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "top" => Some(Bar::Top),
            "left" => Some(Bar::Left),
            "right" => Some(Bar::Right),
            _ => None,
        }
    }
}

/// The collapsed top bar's click area and the line drawn in it.
pub(super) const TOP_AREA: f32 = 8.;
const TOP_LINE: f32 = 4.;
/// A collapsed side bar's click area and its (medium-thick) line.
pub(super) const SIDE_AREA: f32 = 8.;
const SIDE_LINE: f32 = 6.;
/// The reopen tab beside a collapsed side bar's line: how far it sticks out
/// and how tall it is.
const TAB_WIDTH: f32 = 14.;
const TAB_HEIGHT: f32 = 48.;
/// A drag reopens the bar once it goes this far past the line.
pub(super) const DRAG_OUT: f32 = 4.;

/// Seconds after a line appears before its clicks count (longer than a
/// double-click).
const FRESH: f64 = 0.35;

/// How a gesture on the line reopens its bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Reopen {
    /// At the size it had before it collapsed.
    OldSize,
    /// A side bar dragged out: as wide as the distance from its window
    /// edge to where the drag let go (the caller clamps it, and takes the
    /// old size when it is under the bar's minimum).
    Width(f32),
}

/// Draws the line for `bar` filling the space it is given (a full-width
/// strip for the top bar, a full-height one for a side bar), and reads a
/// reopen gesture from it.
pub(super) fn show(ui: &mut Ui, bar: Bar) -> (Response, Option<Reopen>) {
    let size = match bar {
        Bar::Top => vec2(ui.available_width(), TOP_AREA),
        Bar::Left | Bar::Right => vec2(SIDE_AREA, ui.available_height()),
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let line = match bar {
        Bar::Top => Rect::from_center_size(rect.center(), vec2(rect.width(), TOP_LINE)),
        Bar::Left => Rect::from_min_size(rect.min, vec2(SIDE_LINE, rect.height())),
        Bar::Right => Rect::from_min_max(pos2(rect.max.x - SIDE_LINE, rect.min.y), rect.max),
    };
    let hot = response.hovered() || response.dragged();
    let colour = if hot {
        theme::hover_outline()
    } else {
        theme::border()
    };
    ui.painter().rect_filled(line, 2., colour);
    let response = response
        .on_hover_cursor(match bar {
            Bar::Top => CursorIcon::ResizeVertical,
            Bar::Left | Bar::Right => CursorIcon::ResizeHorizontal,
        })
        .on_hover_text(match bar {
            Bar::Top => "Double-click to show bar",
            Bar::Left | Bar::Right => "Double-click to show panel",
        });
    // A line that just appeared ignores clicks for a moment, so the
    // second click of a double-click on the logo (or the resize line)
    // doesn't reopen the bar it just collapsed.
    let (now, pass) = (ui.input(|i| i.time), ui.ctx().cumulative_pass_nr());
    let key = Id::new(("collapse-line", bar.name()));
    let (since, seen) = ui
        .ctx()
        .data(|d| d.get_temp::<(f64, u64)>(key))
        .unwrap_or((now, pass));
    let since = if seen + 1 >= pass { since } else { now };
    ui.ctx().data_mut(|d| d.insert_temp(key, (since, pass)));
    let settled = now - since > FRESH;
    let tab_clicked = match bar {
        Bar::Left | Bar::Right => reopen_tab(ui, bar, rect, hot),
        Bar::Top => false,
    };
    let reopen = if !settled {
        None
    } else if tab_clicked || response.double_clicked() || response.triple_clicked() {
        Some(Reopen::OldSize)
    } else if response.drag_stopped() {
        ui.input(|i| i.pointer.latest_pos())
            .and_then(|at| dragged_out(bar, rect, ui.ctx().content_rect(), at))
    } else {
        None
    };
    (response, reopen)
}

/// Where a collapsed side bar's reopen tab sits: against the inner side of
/// its line area `area`, halfway down.
pub(super) fn tab_rect(bar: Bar, area: Rect) -> Rect {
    let top = area.center().y - TAB_HEIGHT / 2.;
    match bar {
        Bar::Right => Rect::from_min_size(
            pos2(area.left() - TAB_WIDTH, top),
            vec2(TAB_WIDTH, TAB_HEIGHT),
        ),
        _ => Rect::from_min_size(pos2(area.right(), top), vec2(TAB_WIDTH, TAB_HEIGHT)),
    }
}

/// The reopen tab, over whatever is beside the line (the 3D view), in its
/// own layer so its click never reaches that. True when it was clicked.
/// `line_hot`: the line is under the pointer, so the tab lights with it.
fn reopen_tab(ui: &Ui, bar: Bar, area: Rect, line_hot: bool) -> bool {
    let tab = tab_rect(bar, area);
    egui::Area::new(Id::new(("collapse-tab", bar.name())))
        .order(egui::Order::Foreground)
        .fixed_pos(tab.min)
        .constrain(false)
        .fade_in(false)
        .show(ui.ctx(), |ui| {
            let (rect, response) = ui.allocate_exact_size(tab.size(), Sense::click());
            let hot = line_hot || response.hovered();
            // Rounded on the side away from the line only.
            let r = 5;
            let corners = match bar {
                Bar::Right => egui::CornerRadius {
                    nw: r,
                    sw: r,
                    ne: 0,
                    se: 0,
                },
                _ => egui::CornerRadius {
                    nw: 0,
                    sw: 0,
                    ne: r,
                    se: r,
                },
            };
            let painter = ui.painter();
            painter.rect(
                rect,
                corners,
                if hot {
                    theme::hover()
                } else {
                    theme::control()
                },
                Stroke::new(
                    1_f32,
                    if hot {
                        theme::hover_outline()
                    } else {
                        theme::border()
                    },
                ),
                egui::StrokeKind::Inside,
            );
            // A chevron pointing the way the panel comes back from.
            let c = rect.center();
            let (tip, back) = match bar {
                Bar::Right => (c.x - 2.5, c.x + 2.5),
                _ => (c.x + 2.5, c.x - 2.5),
            };
            let ink = Stroke::new(1.5_f32, theme::text());
            painter.line_segment([pos2(back, c.y - 5.), pos2(tip, c.y)], ink);
            painter.line_segment([pos2(tip, c.y), pos2(back, c.y + 5.)], ink);
            response
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Show panel")
                .clicked()
        })
        .inner
}

/// The reopen a drag on `bar`'s line (`line`, in a window `window`) asks
/// for when it lets go at `at`; `None` when it didn't get past the line.
pub(super) fn dragged_out(bar: Bar, line: Rect, window: Rect, at: Pos2) -> Option<Reopen> {
    match bar {
        Bar::Top => (at.y > line.bottom() + DRAG_OUT).then_some(Reopen::OldSize),
        Bar::Left => {
            (at.x > line.right() + DRAG_OUT).then_some(Reopen::Width(at.x - window.left()))
        }
        Bar::Right => {
            (at.x < line.left() - DRAG_OUT).then_some(Reopen::Width(window.right() - at.x))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_reopens_only_once_it_gets_past_the_line() {
        let window = Rect::from_min_size(Pos2::ZERO, vec2(1000., 600.));
        let top = Rect::from_min_size(pos2(0., 20.), vec2(1000., TOP_AREA));
        assert_eq!(dragged_out(Bar::Top, top, window, pos2(50., 30.)), None);
        assert_eq!(
            dragged_out(Bar::Top, top, window, pos2(50., 40.)),
            Some(Reopen::OldSize)
        );
        let left = Rect::from_min_size(pos2(0., 50.), vec2(SIDE_AREA, 500.));
        assert_eq!(dragged_out(Bar::Left, left, window, pos2(10., 90.)), None);
        assert_eq!(
            dragged_out(Bar::Left, left, window, pos2(260., 90.)),
            Some(Reopen::Width(260.))
        );
        let right = Rect::from_min_size(pos2(1000. - SIDE_AREA, 50.), vec2(SIDE_AREA, 500.));
        assert_eq!(
            dragged_out(Bar::Right, right, window, pos2(990., 90.)),
            None
        );
        assert_eq!(
            dragged_out(Bar::Right, right, window, pos2(700., 90.)),
            Some(Reopen::Width(300.))
        );
        assert_eq!(Bar::parse("Left"), Some(Bar::Left));
        assert_eq!(Bar::parse("middle"), None);
    }
}
