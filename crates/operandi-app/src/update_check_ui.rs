//! The update check on screen (update_check.rs has the rest): the launch
//! check, `operandi.update.check` (Help > Check for Updates), the Update
//! Available prompt and the Updates section of Preferences > Defaults >
//! Application.
use super::*;
use crate::prefs_file::PrefsFile;
use crate::update_check::{Channel, Show, UpdatePreferences};

/// The prompt's title (UIC-81).
const TITLE: &str = "Update Available";
/// The manual check's report title: the command's name (UIC-81).
const REPORT_TITLE: &str = "Check for Updates";
/// `open_dialog`'s name for both.
pub(crate) const DIALOG_NAME: &str = "update check";

impl Operandi {
    /// Once per launch, after the first frame is up: the launch check, if
    /// Check for Updates on Launch is on.
    pub(super) fn launch_update_check(&mut self, ctx: &Context) {
        if self.update_check.launched {
            return;
        }
        self.update_check.launched = true;
        if self.update_check.prefs.check_on_launch {
            let ctx = ctx.clone();
            self.update_check
                .start(false, move || ctx.request_repaint());
        }
    }

    /// `operandi.update.check`: check now and always report back.
    pub(super) fn check_for_updates(&mut self) {
        let wake = self.frame_ctx.clone();
        self.update_check.start(true, move || {
            if let Some(ctx) = &wake {
                ctx.request_repaint();
            }
        });
        if self.update_check.busy() {
            self.status = "Checking for updates…".into();
        }
    }

    /// Whether the prompt or report is waiting or showing.
    pub(super) fn update_dialog_open(&self) -> bool {
        self.update_check.show.is_some()
    }

    /// The finished check's prompt or report, once no other dialog is
    /// open (it waits behind them).
    pub(super) fn update_check_dialogs(&mut self, ctx: &Context) {
        self.update_check.poll();
        let Some(show) = self.update_check.show.clone() else {
            return;
        };
        if !matches!(self.open_dialog(), None | Some(DIALOG_NAME)) || self.crash_prompt.is_open() {
            return;
        }
        match show {
            Show::Offer(offer) => {
                // UIC-97 / UIC-84: OK (Ret) opens the page, Cancel (Esc)
                // changes nothing, Skip This Build under Cancel.
                let message = offer.message.clone();
                match self.show_confirm(ctx, TITLE, &message, &["Skip This Build"]) {
                    form::Outcome::Ok => {
                        let url = offer.url();
                        self.status = match self.open_in_browser(&url) {
                            Ok(()) => "Opened the download page in your browser".into(),
                            Err(e) => {
                                self.copy_next = Some(url);
                                format!(
                                    "Couldn't open the browser ({e}); the link is on the clipboard"
                                )
                            }
                        };
                        self.update_check.show = None;
                    }
                    form::Outcome::Extra(_) => {
                        // Remembered through the preference setter (UIC-160).
                        self.execute(&format!("pref.value updates.skipped {}", offer.key));
                        self.status =
                            "Skipped this build; the launch check won't offer it again".into();
                        self.update_check.show = None;
                    }
                    form::Outcome::Cancel | form::Outcome::No => self.update_check.show = None,
                    form::Outcome::Open => {}
                }
            }
            Show::Report(message) => {
                if self.show_notice(ctx, REPORT_TITLE, &message) != form::Outcome::Open {
                    self.update_check.show = None;
                }
                self.status = message;
            }
        }
    }

    /// Preferences > Defaults > Application > Updates. Row edits go
    /// through `pref.value updates.*` (the page's group, UIC-164).
    pub(super) fn updates_preferences_ui(&mut self, ui: &mut egui::Ui) {
        use crate::form::rows;
        let d = UpdatePreferences::default();
        let p = &mut self.update_check.prefs;
        let channels: Vec<_> = Channel::ALL.iter().map(|&c| (c, c.label())).collect();
        rows::section(ui, "Updates", false, |ui| {
            // UIC-113: a dropdown, as every choice in a panel.
            rows::choice_d(
                ui,
                "Update Channel",
                &mut p.channel,
                &d.channel,
                &channels,
                "Release: published versions. Test: the rolling test build, rebuilt from the \
                 newest code",
            );
            // UIC-87: box then text, no left label.
            rows::check_d(
                ui,
                "Check for Updates on Launch",
                &mut p.check_on_launch,
                d.check_on_launch,
                "Ask GitHub for a newer build on the channel when Operandi starts. Nothing is \
                 downloaded or installed without asking",
            );
        });
    }

    /// The Application page's Reset: the Updates rows back to defaults
    /// (a skipped build stays skipped).
    pub(super) fn reset_update_preferences(&mut self) -> std::io::Result<()> {
        let d = UpdatePreferences::default();
        let p = &mut self.update_check.prefs;
        p.channel = d.channel;
        p.check_on_launch = d.check_on_launch;
        p.save(&self.environment.paths().update_preferences())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_check::Offer;

    fn offer() -> Offer {
        Offer {
            channel: Channel::Release,
            tag: "v0.2.0".into(),
            key: "v0.2.0".into(),
            message: "Operandi 0.2.0 is available. You have 0.1.0. OK opens the download page."
                .into(),
        }
    }

    #[test]
    fn channel_and_launch_check_are_preferences() {
        let mut a = Operandi::empty();
        a.execute("pref.value updates.channel Test");
        assert_eq!(a.update_check.prefs.channel, Channel::Test, "{}", a.status);
        a.execute("pref.value updates.check_on_launch false");
        assert!(!a.update_check.prefs.check_on_launch, "{}", a.status);
        let saved = UpdatePreferences::load(&a.environment.paths().update_preferences());
        assert_eq!(saved, a.update_check.prefs);
        a.execute("pref.value updates.channel ?");
        assert_eq!(a.status, "Test");
    }

    #[test]
    fn prompt_waits_for_other_dialogs_and_skip_is_remembered() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        a.update_check.show = Some(Show::Offer(offer()));
        assert_eq!(a.open_dialog(), Some(DIALOG_NAME));
        // Behind another modal: not drawn, still waiting.
        a.show_preferences = true;
        let _ = ctx.run(Default::default(), |ctx| a.update_check_dialogs(ctx));
        assert!(a.update_check.show.is_some());
        a.show_preferences = false;
        // Skip This Build through the setter.
        a.execute("pref.value updates.skipped v0.2.0");
        assert_eq!(a.update_check.prefs.skipped, "v0.2.0", "{}", a.status);
    }

    #[test]
    fn the_command_reports_back() {
        let mut a = Operandi::empty();
        a.execute("operandi.update.check");
        // Tests never go online: the check fails and says so.
        for _ in 0..200 {
            a.update_check.poll();
            if a.update_check.show.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        match &a.update_check.show {
            Some(Show::Report(m)) => assert!(m.starts_with("Couldn't check for updates"), "{m}"),
            other => panic!("{other:?}"),
        }
    }
}
