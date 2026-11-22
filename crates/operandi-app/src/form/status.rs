//! The channel status light: the small circle between a channel row's
//! label and its field (UIC-112).
//!
//! It shows where a channel's value comes from. Operandi has no animation,
//! keys, links or drivers yet, so only `None` (at the default) and
//! `Constant` (a set, non-default value) can occur today; the other states
//! are listed so those systems can set them without changing the rows.
//! Right-click the light or the row's label: Reset to Default (CH-Q1);
//! left-click does nothing yet (later: set or remove a key).
use super::layout::panel::CIRCLE;
use crate::theme;
use eframe::egui::{Painter, Pos2, Stroke};

/// What a channel's status light shows (UIC-112).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChannelStatus {
    /// At its default (or no default is known): a plain grey ring.
    #[default]
    None,
    /// A set value that differs from the default, not animated: a filled
    /// dot. A multi-selection whose values differ shows this when any
    /// selected value is non-default (a true `Mixed` light comes later).
    Constant,
    // The states below can't occur until animation, links and drivers
    // exist; they're drawn as `None` until then.
    #[allow(dead_code)]
    Animated,
    /// A key on the current frame (red).
    #[allow(dead_code)]
    Key,
    /// Driven by another channel (a link or driver), or by a free node's
    /// wire in the node view (node editing, NE-04).
    Driven,
    /// Drives other channels (an arrow over the dot).
    #[allow(dead_code)]
    Driver,
    /// Selected items' channels disagree in where their values come from.
    #[allow(dead_code)]
    Mixed,
    #[allow(dead_code)]
    Undefined,
}

impl ChannelStatus {
    /// `Constant` when the shown value isn't the default, else `None`.
    pub(crate) fn from_non_default(non_default: bool) -> Self {
        if non_default {
            Self::Constant
        } else {
            Self::None
        }
    }
}

/// Paints `status`'s light centred at `at`. `enabled` greys it with its
/// row (UIC-117).
pub(crate) fn paint(painter: &Painter, at: Pos2, status: ChannelStatus, enabled: bool) {
    let r = CIRCLE / 2. - 0.5;
    let color = if enabled {
        theme::channel_default()
    } else {
        theme::disabled_text()
    };
    match status {
        // Changed from the default: a filled dot in the theme's changed
        // colour, so it stands out from the default ring.
        ChannelStatus::Constant => {
            let fill = if enabled {
                theme::channel_changed()
            } else {
                theme::disabled_text()
            };
            painter.circle_filled(at, r, fill);
        }
        // NE-04: a ring with an orange core, the wire's colour.
        ChannelStatus::Driven => {
            painter.circle_stroke(at, r, Stroke::new(1.0f32, color));
            let core = if enabled {
                theme::orange()
            } else {
                theme::disabled_text()
            };
            painter.circle_filled(at, r * 0.55, core);
        }
        // Only None, Constant and Driven can occur today (see the module notes).
        _ => {
            painter.circle_stroke(at, r, Stroke::new(1.0f32, color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_default_values_light_constant_and_defaults_none() {
        assert_eq!(
            ChannelStatus::from_non_default(true),
            ChannelStatus::Constant
        );
        assert_eq!(ChannelStatus::from_non_default(false), ChannelStatus::None);
        assert_eq!(ChannelStatus::default(), ChannelStatus::None);
    }
}
