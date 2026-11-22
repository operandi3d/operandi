//! The Help menu (Jason 2026-11-16): Report Bug or Idea… (Report Bug until
//! 2026-11-20; its "…" per UIC-03, as the browser form asks for more before
//! anything is sent), moved off the layout bar, and Documentation, greyed
//! until the docs are published. Both run
//! registry commands (UIC-220): `operandi.help.reportBug` and
//! `operandi.help.documentation`.
use super::*;

/// Where the documentation lives. `None` until it is published: Help >
/// Documentation stays greyed ("Coming soon"). Setting it is the only
/// change needed to turn the item on.
pub const DOCS_URL: Option<&str> = None;

/// Help > Documentation's tooltip while there is no [`DOCS_URL`].
pub const DOCS_COMING: &str = "Coming soon";

impl Operandi {
    /// `operandi.help.reportBug`: a pre-filled GitHub issue in the browser.
    /// Nothing is sent; the user reads, edits and submits it there
    /// (bug_report.rs).
    pub(super) fn report_bug(&mut self) {
        let url = bug_report::bug_report_url(&self.gpu);
        self.status = match self.open_in_browser(&url) {
            Ok(()) => "Opened a new bug report in your browser".into(),
            Err(e) => {
                self.copy_next = Some(url);
                format!("Couldn't open the browser ({e}); the issue link is on the clipboard")
            }
        };
    }

    /// `operandi.help.documentation`: the docs in the browser.
    pub(super) fn open_documentation(&mut self) {
        let Some(url) = DOCS_URL else {
            self.status = format!("Documentation: {DOCS_COMING}");
            return;
        };
        self.status = match self.open_in_browser(url) {
            Ok(()) => "Opened the documentation in your browser".into(),
            Err(e) => {
                self.copy_next = Some(url.into());
                format!("Couldn't open the browser ({e}); the link is on the clipboard")
            }
        };
    }

    /// Hands `url` to the default browser. Tests record it instead.
    fn open_in_browser(&mut self, url: &str) -> std::io::Result<()> {
        self.opened_url = Some(url.to_owned());
        if cfg!(test) {
            return Ok(());
        }
        bug_report::open_url(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_menu::Entry;

    #[test]
    fn help_menu_is_last_and_runs_report_bug_and_documentation() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let model = a.menu_model(&ctx);
        let help = model.last().unwrap();
        assert_eq!(help.title, "Help");
        let leaves = help.leaves();
        let bug = leaves
            .iter()
            .find(|i| i.action == crate::app_menu::line_action("operandi.help.reportBug"))
            .unwrap();
        assert!(bug.enabled);
        let docs = leaves
            .iter()
            .find(|i| i.action == crate::app_menu::line_action("operandi.help.documentation"))
            .unwrap();
        assert_eq!(docs.enabled, DOCS_URL.is_some());
        if DOCS_URL.is_none() {
            assert_eq!(docs.why.as_deref(), Some(DOCS_COMING));
        }
        assert!(help.entries.iter().all(|e| !matches!(e, Entry::Submenu(_))));

        // The menu runs the command; the command opens the issue link.
        a.menu_action(
            &ctx,
            crate::app_menu::line_action("operandi.help.reportBug"),
        );
        let url = a.opened_url.take().expect("Report Bug opens a link");
        assert!(url.contains("issues/new"), "{url}");
        a.execute("operandi.help.reportBug");
        assert!(a.opened_url.take().is_some(), "the command alone works too");

        // Greyed Documentation: the menu does nothing, the command says why.
        a.menu_action(
            &ctx,
            crate::app_menu::line_action("operandi.help.documentation"),
        );
        assert_eq!(a.opened_url, DOCS_URL.map(str::to_owned));
        a.execute("operandi.help.documentation");
        if DOCS_URL.is_none() {
            assert!(a.opened_url.is_none());
            assert!(a.status.contains(DOCS_COMING), "{}", a.status);
        }
    }
}
