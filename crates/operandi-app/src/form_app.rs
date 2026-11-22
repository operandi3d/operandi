//! The app's side of the shared form component (form/): dialogs open with
//! each command's remembered values (UIC-95), own the keyboard while open,
//! and report refusals in the status line.
use super::*;

impl Operandi {
    /// A dialog for command `title` with `rows`, at the values last OK'd
    /// for that command (first time: the rows' defaults; UIC-95).
    pub(super) fn form_dialog(&self, title: &str, rows: Vec<form::Row>) -> form::Dialog {
        let title = title.trim_end_matches('…');
        let values = self.dialog_memory.values(title, &rows);
        form::Dialog::new(title, rows, values)
    }

    /// One frame of an open dialog. While it is open it owns the keyboard
    /// and pointer; a new refusal goes to the status line too.
    pub(super) fn show_form(&mut self, ctx: &Context, dialog: &mut form::Dialog) -> form::Outcome {
        self.modal_owned_input = true;
        self.editor_owned_input = true;
        let before = dialog.error.clone();
        let outcome = dialog.show(ctx);
        self.text_editors.extend(dialog.text_ids().iter().copied());
        if dialog.error != before
            && let Some(error) = &dialog.error
        {
            self.status = error.clone();
        }
        ctx.request_repaint();
        outcome
    }

    /// A confirmation behind a flag (UIC-97): plain text, OK and Cancel.
    pub(super) fn show_confirm(
        &mut self,
        ctx: &Context,
        title: &str,
        message: &str,
        extras: &[&str],
    ) -> form::Outcome {
        self.modal_owned_input = true;
        self.editor_owned_input = true;
        ctx.request_repaint();
        form::Dialog::show_kept(ctx, title, || {
            form::Dialog::confirm(title, message).extras(extras)
        })
    }

    /// A Yes / No / Cancel question behind a flag (`form::Dialog::question`):
    /// Return is Yes (Ok), Escape is No, Shift+Escape is Cancel.
    pub(super) fn show_question(
        &mut self,
        ctx: &Context,
        title: &str,
        message: &str,
    ) -> form::Outcome {
        self.modal_owned_input = true;
        self.editor_owned_input = true;
        ctx.request_repaint();
        form::Dialog::show_kept(ctx, title, || form::Dialog::question(title, message))
    }

    /// An acknowledgement behind a flag: plain text and OK alone.
    pub(super) fn show_notice(
        &mut self,
        ctx: &Context,
        title: &str,
        message: &str,
    ) -> form::Outcome {
        self.modal_owned_input = true;
        self.editor_owned_input = true;
        ctx.request_repaint();
        form::Dialog::show_kept(ctx, title, || form::Dialog::notice(title, message))
    }

    /// Keeps `dialog`'s values as its command's next starting values
    /// (UIC-95), across restarts.
    pub(super) fn remember_form(&mut self, dialog: &form::Dialog) {
        self.dialog_memory
            .remember(&dialog.title, &dialog.rows, &dialog.values);
        if let Err(error) = self
            .dialog_memory
            .save(&self.environment.paths().dialog_values())
        {
            self.status = format!("{}: could not keep its settings: {error}", dialog.title);
        }
    }
}
