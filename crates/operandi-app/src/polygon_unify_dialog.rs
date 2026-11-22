//! Bounded Unify options and count acknowledgment share the core transaction.
//! Both use the shared form dialog (UIC-80..99).
use super::*;
pub(super) struct UnifyDialog {
    target: UnifyTarget,
    form: form::Dialog,
}
/// The dialog's title: the command's name without "…" (UIC-81).
const TITLE: &str = "Unify";
fn rows() -> Vec<form::Row> {
    vec![
        form::Row::check("force", "Force Unify", false)
            .tip("Also merges polygons that share their points in a different order"),
    ]
}
impl Operandi {
    pub(super) fn open_unify(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Unify".into();
            return;
        }
        match self.editor.unify_target() {
            Ok(target) => {
                self.unify = Some(UnifyDialog {
                    target,
                    form: self.form_dialog(TITLE, rows()),
                })
            }
            Err(e) => self.status = e,
        }
    }
    pub(super) fn execute_unify(&mut self, force: bool, expected: Option<UnifyTarget>) {
        if self.has_modal() {
            self.status = "Close the open dialog before Unify".into();
            return;
        }
        let before = self
            .editor
            .scene
            .active()
            .map(|i| i.mesh.faces.len())
            .unwrap_or(0);
        let succeeded = self.run_ok(Command::UnifyPolygons(UnifyRequest { force, expected }));
        if succeeded {
            let after = self
                .editor
                .scene
                .active()
                .map(|i| i.mesh.faces.len())
                .unwrap_or(before);
            self.unify_count = Some(before.saturating_sub(after));
        }
    }
    pub(super) fn unify_dialog(&mut self, ctx: &Context) {
        if let Some(mut dialog) = self.unify.take() {
            if self.has_modal() {
                self.status = "Unify canceled; nothing changed".into();
                return;
            }
            match self.show_form(ctx, &mut dialog.form) {
                form::Outcome::Cancel => self.status = "Unify canceled".into(),
                form::Outcome::Ok => {
                    // UIC-96: OK runs the command once.
                    self.remember_form(&dialog.form);
                    let force = dialog.form.values.bool("force");
                    self.execute_unify(force, Some(dialog.target));
                    ctx.request_repaint();
                    return;
                }
                _ => self.unify = Some(dialog),
            }
        }
        if let Some(count) = self.unify_count {
            let message = format!("{count} polygons eliminated");
            if self.show_notice(ctx, TITLE, &message) != form::Outcome::Open {
                self.unify_count = None;
            }
        }
    }
}
