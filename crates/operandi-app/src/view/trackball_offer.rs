//! The one-time "Try Trackball Rotation?" offer (Jason, 2026-11-27).
//! Trackball Rotation stays off by default. After the first orbit drag in
//! a 3D view, a small note in the view offers to turn it on. It doesn't
//! block the view (nothing else waits on it, unlike a dialog, UIC-186),
//! goes after `SHOWN_FOR` seconds by itself, and never comes back: the
//! Input preferences remember that it was shown.
use super::*;

/// How long the offer stays before it goes by itself, in seconds.
pub(crate) const SHOWN_FOR: f64 = 17.;

/// The offer's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    TurnOn,
    NoThanks,
}

impl Operandi {
    /// An orbit drag in a 3D view just ended (at `now`, seconds): the first
    /// one shows the offer, unless orbiting is a trackball turn already.
    /// Camera and Light view orbit their item, which never trackballs.
    pub(crate) fn orbited(&mut self, now: f64) {
        if self.input_preferences.trackball_offered || self.camera.looks_through() {
            return;
        }
        self.input_preferences.trackball_offered = true;
        if !self
            .input_preferences
            .options
            .trackball_in(self.display.settings.drawing.trackball)
        {
            self.trackball_offer = Some(now);
        }
        self.save_input_preferences();
    }

    /// The offer is showing at `now`.
    pub(crate) fn trackball_offer_shown(&mut self, now: f64) -> bool {
        if self
            .trackball_offer
            .is_some_and(|since| now - since >= SHOWN_FOR)
        {
            self.trackball_offer = None;
        }
        self.trackball_offer.is_some()
    }

    /// Turn On turns the Trackball Rotation preference on; either answer
    /// closes the offer.
    pub(crate) fn answer_trackball_offer(&mut self, answer: Answer) {
        self.trackball_offer = None;
        if answer == Answer::TurnOn {
            self.input_preferences.options.trackball = true;
            self.save_input_preferences();
            self.status = "Trackball Rotation on (Preferences > Input > Remapping)".into();
        }
    }

    fn save_input_preferences(&mut self) {
        if let Err(error) = self
            .input_preferences
            .save(&self.environment.paths().input_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }

    /// Draws the offer at the bottom of view rect `r` while it shows: a
    /// small floating note, not a dialog, so the view stays usable.
    pub(crate) fn trackball_offer_ui(&mut self, ctx: &Context, r: Rect) {
        let now = ctx.input(|i| i.time);
        if !self.trackball_offer_shown(now) {
            return;
        }
        let since = self.trackball_offer.unwrap_or(now);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(
            (SHOWN_FOR - (now - since)).max(0.05),
        ));
        let mut answer = None;
        egui::Area::new(egui::Id::new("trackball offer"))
            .order(egui::Order::Foreground)
            .fixed_pos(r.center_bottom() + vec2(0., -40.))
            .pivot(Align2::CENTER_BOTTOM)
            .constrain_to(r)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.strong("Try Trackball Rotation?");
                            ui.label(
                                "Orbit turns freely about the view instead of around world up.",
                            );
                        });
                        // UIC-04: every button has a tooltip.
                        if ui
                            .button("Turn On")
                            .on_hover_text(
                                "Turn on Trackball Rotation (Preferences > Input > Remapping)",
                            )
                            .clicked()
                        {
                            answer = Some(Answer::TurnOn);
                        }
                        if ui
                            .button("No Thanks")
                            .on_hover_text("Keep the usual orbit; this won't ask again")
                            .clicked()
                        {
                            answer = Some(Answer::NoThanks);
                        }
                    });
                });
            });
        if let Some(answer) = answer {
            self.answer_trackball_offer(answer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.input_preferences.trackball_offered = false;
        a
    }

    #[test]
    fn the_first_orbit_offers_trackball_once() {
        let mut a = app();
        assert!(!a.input_preferences.options.trackball, "off by default");
        a.orbited(10.);
        assert!(a.trackball_offer_shown(10.));
        assert!(a.input_preferences.trackball_offered);
        a.answer_trackball_offer(Answer::NoThanks);
        assert!(!a.trackball_offer_shown(10.));
        assert!(!a.input_preferences.options.trackball);
        // Never again.
        a.orbited(20.);
        assert!(!a.trackball_offer_shown(20.));
    }

    #[test]
    fn turn_on_turns_the_preference_on() {
        let mut a = app();
        a.orbited(0.);
        a.answer_trackball_offer(Answer::TurnOn);
        assert!(a.input_preferences.options.trackball);
        assert!(a.trackball_offer.is_none());
    }

    #[test]
    fn the_offer_goes_by_itself_after_17_seconds() {
        let mut a = app();
        a.orbited(5.);
        assert!(a.trackball_offer_shown(5. + SHOWN_FOR - 0.1));
        assert!(!a.trackball_offer_shown(5. + SHOWN_FOR));
        assert!(!a.input_preferences.options.trackball);
    }

    #[test]
    fn no_offer_when_trackball_is_on_already() {
        let mut a = app();
        a.input_preferences.options.trackball = true;
        a.orbited(0.);
        assert!(!a.trackball_offer_shown(0.));
        assert!(a.input_preferences.trackball_offered);
    }

    #[test]
    fn shown_is_remembered_in_saved_preferences() {
        let mut a = app();
        a.orbited(0.);
        let saved = preferences::InputPreferences::load(&a.environment.paths().input_preferences());
        assert!(saved.trackball_offered);
    }
}
