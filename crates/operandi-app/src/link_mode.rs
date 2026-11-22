//! Linked X/Y/Z fields (UIC-56, UIC-57; MN-06.2, MN-06.3).
//!
//! A vector group (Item Transform's Position, Rotation and Scale Factor,
//! and every Tool Properties X/Y/Z group made by `tool_form::vector`) has
//! one link button before the first row's mini-slider. Each click cycles
//! off → same value → scale together → add together → off:
//!
//! - Independent: only the edited axis changes;
//! - Copy: every axis takes the new value;
//! - Proportional: every axis is scaled by the edited axis's ratio;
//! - Relative: every axis gets the edited axis's change added.
//!
//! The Enter that confirms a typed value can pick the mode for that one
//! entry: Ctrl+Alt+Enter Copy, Alt+Enter Relative, Ctrl+Enter
//! Proportional, plain Enter the button's mode. On a Mac Cmd stands for
//! Ctrl, as everywhere in the app.
//!
//! The button sits on the group's first row, just left of its mini-slider
//! arrows and as tall as them, and shows the mode as a rendered chain icon
//! (icon studio, MN-06b): a broken chain when off, two equal links for the
//! same value, a big and a small link for scale together, links and a plus
//! for add together. The mode is per group and per session.
use eframe::egui::{self, Modifiers, Rect, Response, Sense, Ui};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LinkMode {
    #[default]
    Independent,
    Copy,
    Proportional,
    Relative,
}

impl LinkMode {
    /// The mode the next click picks (UIC-56).
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Independent => Self::Copy,
            Self::Copy => Self::Proportional,
            Self::Proportional => Self::Relative,
            Self::Relative => Self::Independent,
        }
    }

    /// The mode in plain words, as the tooltip names it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Independent => "off",
            Self::Copy => "same value",
            Self::Proportional => "scale together",
            Self::Relative => "add together",
        }
    }

    /// The button's face (icon studio).
    pub(crate) fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Self::Independent => Icon::LinkIndependent,
            Self::Copy => Icon::LinkCopy,
            Self::Proportional => Icon::LinkProportional,
            Self::Relative => Icon::LinkRelative,
        }
    }

    /// MN-06b.5: the state, what a click switches to, and the Enter keys.
    pub(crate) fn tip(self) -> String {
        let what = match self {
            Self::Independent => "only the axis you edit changes",
            Self::Copy => "every axis takes the value you type",
            Self::Proportional => "every axis scales by the same ratio",
            Self::Relative => "every axis gets the same change added",
        };
        let chord = |c: &str| crate::input::display_chord(c);
        format!(
            "Link: {} ({what}).\nClick: link {}.\nEnter keeps this link for one entry; {}: same value, {}: scale together, {}: add together.",
            self.label(),
            self.next().label(),
            chord("Cmd+Alt+Enter"),
            chord("Cmd+Enter"),
            chord("Alt+Enter"),
        )
    }

    /// The mode an Enter with `modifiers` confirms with (UIC-57); `button`
    /// is the group's own mode. `command` is Ctrl, or Cmd on a Mac.
    pub(crate) fn for_enter(modifiers: Modifiers, button: Self) -> Self {
        match (modifiers.command, modifiers.alt) {
            (true, true) => Self::Copy,
            (false, true) => Self::Relative,
            (true, false) => Self::Proportional,
            (false, false) => button,
        }
    }

    /// The button's place at the right end of the first row's `field`
    /// (left of the mini-slider) and the rest of the field.
    pub(crate) fn split(field: Rect) -> (Rect, Rect) {
        let link = Rect::from_min_max(egui::pos2(field.max.x - WIDTH, field.min.y), field.max);
        let mut rest = field;
        rest.max.x = link.min.x - 2.;
        (link, rest)
    }

    /// The group's values once `axis` goes from `old[axis]` to `value`.
    /// Proportional from a zero has no ratio: only that axis changes.
    pub(crate) fn apply(self, old: [f64; 3], axis: usize, value: f64) -> [f64; 3] {
        let mut out = old;
        match self {
            Self::Independent => out[axis] = value,
            Self::Copy => out = [value; 3],
            Self::Relative => {
                let change = value - old[axis];
                out = old.map(|v| v + change);
            }
            Self::Proportional => {
                if old[axis].abs() < 1e-12 {
                    out[axis] = value;
                } else {
                    let ratio = value / old[axis];
                    out = old.map(|v| v * ratio);
                    out[axis] = value;
                }
            }
        }
        out
    }
}

/// The button's width: square to the row, as tall as the mini-slider.
pub(crate) const WIDTH: f32 = crate::option_row::ROW_HEIGHT;

/// Draw a group's link button in `rect` (from [`LinkMode::split`]): the
/// mode's chain icon, greyed with the row, the tooltip from
/// [`LinkMode::tip`]. The caller cycles the mode on a click.
pub(crate) fn button(ui: &mut Ui, rect: Rect, mode: LinkMode) -> Response {
    let response = ui
        .allocate_rect(rect, Sense::click())
        .on_hover_text(mode.tip());
    if response.hovered() && ui.is_enabled() {
        ui.painter()
            .rect_filled(rect, 3., ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let size = (rect.height() - 4.).min(rect.width()).max(8.);
    let icon = Rect::from_center_size(rect.center(), egui::vec2(size, size));
    if ui.is_enabled() {
        crate::icons::paint(ui, icon, mode.icon());
    } else {
        crate::icons::paint_faded(ui, icon, mode.icon());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_cycle_the_four_modes() {
        let mut m = LinkMode::Independent;
        let mut seen = vec![m];
        for _ in 0..4 {
            m = m.next();
            seen.push(m);
        }
        assert_eq!(
            seen,
            [
                LinkMode::Independent,
                LinkMode::Copy,
                LinkMode::Proportional,
                LinkMode::Relative,
                LinkMode::Independent
            ]
        );
    }

    #[test]
    fn each_mode_spreads_the_edit() {
        let old = [1., 2., 4.];
        assert_eq!(LinkMode::Independent.apply(old, 1, 3.), [1., 3., 4.]);
        assert_eq!(LinkMode::Copy.apply(old, 1, 3.), [3., 3., 3.]);
        assert_eq!(LinkMode::Proportional.apply(old, 1, 3.), [1.5, 3., 6.]);
        assert_eq!(LinkMode::Relative.apply(old, 1, 3.), [2., 3., 5.]);
        // No ratio from zero.
        assert_eq!(
            LinkMode::Proportional.apply([0., 2., 4.], 0, 5.),
            [5., 2., 4.]
        );
    }

    #[test]
    fn enter_modifiers_pick_the_mode_for_one_entry() {
        let button = LinkMode::Relative;
        let cmd = Modifiers::COMMAND;
        assert_eq!(
            LinkMode::for_enter(cmd | Modifiers::ALT, button),
            LinkMode::Copy
        );
        assert_eq!(
            LinkMode::for_enter(Modifiers::ALT, button),
            LinkMode::Relative
        );
        assert_eq!(LinkMode::for_enter(cmd, button), LinkMode::Proportional);
        assert_eq!(LinkMode::for_enter(Modifiers::NONE, button), button);
        assert_eq!(
            LinkMode::for_enter(Modifiers::NONE, LinkMode::Independent),
            LinkMode::Independent
        );
    }
}
