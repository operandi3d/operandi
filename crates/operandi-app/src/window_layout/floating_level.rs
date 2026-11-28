//! UIC-189: floating windows stay above the main window, above Operandi
//! only, not above other apps.
//!
//! eframe has no owned (child) windows: a viewport is either a plain window
//! or always on top of every app. So every floating window (palettes,
//! floating panels, torn-off views, Preview, Render) is always on top while
//! one of Operandi's windows has the keyboard, and a plain window while
//! another app does. Clicking the main window then never buries a floating
//! window, and switching to another app lets that app cover it. Moving the
//! keyboard between two Operandi windows passes through a moment where
//! neither has it, so the level drops only once no Operandi window has had
//! it for `GRACE` seconds.
use super::*;

/// How long after the last Operandi window lost the keyboard the floating
/// windows stay on top (seconds).
pub(crate) const GRACE: f64 = 0.3;

/// Whether floating windows are on top at `now`, given whether any
/// Operandi window has the keyboard; `last` is when one last had it.
pub(crate) fn on_top(any_focused: bool, now: f64, last: &mut Option<f64>) -> bool {
    if any_focused {
        *last = Some(now);
        return true;
    }
    last.is_some_and(|t| now - t < GRACE)
}

impl Operandi {
    /// Works out this frame's level for the floating windows from the main
    /// window's input (every window's focus is in it).
    pub(crate) fn update_floating_level(&mut self, ctx: &Context) {
        let (any, now) = ctx.input(|i| {
            (
                i.raw.viewports.values().any(|v| v.focused == Some(true)),
                i.time,
            )
        });
        let was = self.floating_on_top;
        self.floating_on_top = on_top(any, now, &mut self.floating_focused_at);
        if self.floating_on_top && !any {
            // Look again once the grace is over.
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(GRACE));
        }
        if was != self.floating_on_top {
            ctx.request_repaint();
        }
    }

    /// The level every floating window opens or stays at (UIC-189).
    pub(crate) fn floating_window_level(&self) -> egui::WindowLevel {
        if self.floating_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_top_while_operandi_has_the_keyboard() {
        let mut last = None;
        assert!(!on_top(false, 0., &mut last), "never focused yet");
        assert!(on_top(true, 1., &mut last));
        // Between two Operandi windows: still on top.
        assert!(on_top(false, 1. + GRACE / 2., &mut last));
        assert!(on_top(true, 1.2, &mut last));
        // Another app has the keyboard: a plain window.
        assert!(!on_top(false, 1.2 + GRACE, &mut last));
    }

    #[test]
    fn every_floating_window_follows_the_level() {
        let mut a = Operandi::empty();
        a.floating_on_top = true;
        assert_eq!(a.floating_window_level(), egui::WindowLevel::AlwaysOnTop);
        a.floating_on_top = false;
        assert_eq!(a.floating_window_level(), egui::WindowLevel::Normal);
    }
}
