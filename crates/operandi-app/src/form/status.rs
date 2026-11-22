//! The channel status light: the small circle between a channel row's
//! label and its field (UIC-112).
//!
//! It shows where a channel's value comes from: `None` (at the default),
//! `Constant` (a set, non-default value), and since animation phase 1
//! `Animated`, `Key` (a key on this frame) and `Edited` (TK-41: edited but
//! not keyed; the next time change drops it). Keyable rows (anim_ui.rs):
//! left-click sets or removes a key, right-click has the key commands.
//! Other rows: right-click the light or the label for Reset to Default
//! (CH-Q1).
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
    /// Animated, no key on this frame: a red ring.
    Animated,
    /// A key on the current frame: a filled red dot.
    Key,
    /// TK-41 [OP]: edited, not keyed (pending): a yellow dot in a red ring.
    Edited,
    /// Driven by another channel (a link or driver), or by a free node's
    /// wire in the node view (node editing, NE-04).
    Driven,
    /// Drives other channels (an arrow over the dot).
    #[allow(dead_code)]
    Driver,
    /// Selected items' channels disagree in where their values come from.
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
        ChannelStatus::Animated | ChannelStatus::Key | ChannelStatus::Edited => {
            let red = if enabled {
                key_red()
            } else {
                theme::disabled_text()
            };
            painter.circle_stroke(at, r, Stroke::new(1.5f32, red));
            match status {
                ChannelStatus::Key => {
                    painter.circle_filled(at, r - 0.5, red);
                }
                ChannelStatus::Edited => {
                    let fill = if enabled {
                        edited_yellow()
                    } else {
                        theme::disabled_text()
                    };
                    painter.circle_filled(at, r * 0.6, fill);
                }
                _ => {}
            }
        }
        // Half filled: the selected items differ.
        ChannelStatus::Mixed => {
            painter.circle_stroke(at, r, Stroke::new(1.0f32, color));
            let fill = if enabled {
                theme::channel_changed()
            } else {
                theme::disabled_text()
            };
            let points: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.5 + i as f32 / 12.);
                    at + eframe::egui::vec2(a.cos(), a.sin()) * r
                })
                .collect();
            painter.add(eframe::egui::Shape::convex_polygon(
                points,
                fill,
                Stroke::NONE,
            ));
        }
        _ => {
            painter.circle_stroke(at, r, Stroke::new(1.0f32, color));
        }
    }
}

/// The key colour (UIC-112's red).
pub(crate) fn key_red() -> eframe::egui::Color32 {
    eframe::egui::Color32::from_rgb(222, 64, 56)
}

/// TK-41's "edited, not keyed" colour.
pub(crate) fn edited_yellow() -> eframe::egui::Color32 {
    eframe::egui::Color32::from_rgb(240, 200, 60)
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
