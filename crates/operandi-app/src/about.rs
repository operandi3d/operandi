//! Help > About Operandi (the app menu's About Operandi on macOS): the
//! logo, the name, the version and build, and a line of fine print. Opened
//! from the menu only, so nothing here runs at launch (Jason, 2026-11-08:
//! no splash screen, nothing that slows startup).
use crate::Operandi;
use eframe::egui::*;

/// Shows the "not affiliated" fine print. One switch so it is easy to drop
/// (Jason, 2026-11-08: keep it small and quiet).
pub const SHOW_AFFILIATION_NOTE: bool = true;
pub const AFFILIATION_NOTE: &str = "Not affiliated with or endorsed by Foundry.";
pub const LICENCE_NOTE: &str = "Free software under the GNU GPL v3 or later.";
/// Credits for ideas other people's tools gave us, one line each (Twist
/// Bevel spec section 10; names and links wait on TB-Q10).
pub const IDEA_CREDITS: &[&str] = &[
    "Twist Bevel: idea inspired by the \"Twist Bevel\" tool for Modo by FUSION. \
     Profile Bias, Segment Spacing and Center Bias: ideas from Modest, a \
     Blender add-on by Christian.",
    // missing-tools-g2 MT6-06: the shape's own credit, as its data asks.
    "Teapot: the Utah Teapot (1987 version by Frank Crow, after Martin \
     Newell and Jim Blinn), from the University of Utah.",
];
/// The logo's own shape (1448 × 1086), at a size that fits a small window.
const LOGO_SIZE: Vec2 = vec2(240., 180.);

/// "Version 0.1.0 · build 1a2b3c4" (the commit build.rs stamps).
pub fn version_line() -> String {
    format!(
        "Version {} · build {}",
        crate::bug_report::VERSION,
        crate::bug_report::COMMIT
    )
}

impl Operandi {
    pub(super) fn about_window(&mut self, ctx: &Context) {
        if !self.show_about {
            return;
        }
        let mut open = !ctx.input(|i| i.key_pressed(Key::Escape));
        Window::new("About Operandi")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    crate::branding::logo(ui, LOGO_SIZE);
                    ui.add_space(4.);
                    ui.label(
                        RichText::new("Operandi")
                            .size(22.)
                            .strong()
                            .color(crate::theme::text()),
                    );
                    ui.label(RichText::new(version_line()).color(crate::theme::text()));
                    ui.add_space(10.);
                    // Fine print: small and muted, not a headline.
                    let fine =
                        |text: &str| RichText::new(text).small().color(crate::theme::weak_text());
                    ui.label(fine(LICENCE_NOTE));
                    if SHOW_AFFILIATION_NOTE {
                        ui.label(fine(AFFILIATION_NOTE));
                    }
                    ui.add_space(6.);
                    ui.set_max_width(LOGO_SIZE.x + 80.);
                    for credit in IDEA_CREDITS {
                        ui.label(fine(credit));
                    }
                });
            });
        self.show_about = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_menu::Action;

    fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
        let input = RawInput {
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| a.about_window(ctx));
    }

    #[test]
    fn version_line_names_the_version_and_build() {
        let line = version_line();
        assert!(line.contains(env!("CARGO_PKG_VERSION")));
        assert!(line.contains(crate::bug_report::COMMIT));
    }

    #[test]
    fn about_opens_from_the_menu_and_esc_closes_it() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        frame(&mut a, &ctx, vec![]);
        assert!(!a.show_about, "nothing shows at launch");
        a.menu_action(&ctx, Action::About);
        assert!(a.show_about);
        frame(&mut a, &ctx, vec![]);
        assert!(a.show_about, "stays open until closed");
        frame(
            &mut a,
            &ctx,
            vec![Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(!a.show_about);
    }

    /// Twist Bevel spec section 10: the credit line, as written.
    #[test]
    fn credits_name_the_twist_bevel_ideas() {
        let all = IDEA_CREDITS.join(" ");
        assert!(all.contains("\"Twist Bevel\" tool for Modo by FUSION"));
        assert!(all.contains("Modest, a Blender add-on by Christian"));
    }

    /// MT6-06: the Teapot's data is credited as a "Utah Teapot".
    #[test]
    fn credits_name_the_utah_teapot() {
        assert!(IDEA_CREDITS.join(" ").contains("Utah Teapot"));
    }

    #[test]
    fn about_changes_nothing_in_the_scene() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let before = a.editor.scene.clone();
        a.menu_action(&ctx, Action::About);
        frame(&mut a, &ctx, vec![]);
        assert!(a.editor.scene == before);
        assert!(!a.dirty());
    }
}
