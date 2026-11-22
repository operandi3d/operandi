//! Flip It / Flip It Origin's axis dialog (tool presets spec 6b, TP-Q3:
//! one axis choice, starting on X), in the shared form dialog (UIC-98).
//! The buttons say "Flip It…" (UIC-80); the axis is a joined X / Y / Z
//! segment row (UIC-89) that opens on the axis used last (UIC-95); OK runs
//! the row once, one undo step, and Cancel changes nothing (UIC-96).
use super::*;

pub(super) struct FlipItDialog {
    /// The row's id: `FlipIt` or `FlipItOrigin`.
    pub(super) preset: &'static str,
    form: form::Dialog,
}

fn rows() -> Vec<form::Row> {
    vec![
        form::Row::segments("axis", "Axis", &["X", "Y", "Z"], 0)
            .tip("The world axis the polygons are mirrored along"),
    ]
}

impl Operandi {
    /// Open the axis dialog for the Flip It row `id`.
    pub(super) fn open_flip_it(&mut self, id: &str) {
        let Some(row) = operandi_core::tool_preset::row(id) else {
            self.status = format!("No tool preset called {id}");
            return;
        };
        if self.has_modal() {
            self.status = format!("Close the open dialog before {}", row.label);
            return;
        }
        // UIC-81: the title is the command's name without "…".
        self.flip_it = Some(FlipItDialog {
            preset: row.id.as_str(),
            form: self.form_dialog(&row.label, rows()),
        });
    }

    /// Run the Flip It row `id` on `axis` (0 X, 1 Y, 2 Z).
    pub(super) fn run_flip_it(&mut self, id: &str, axis: usize) -> bool {
        self.run_ok(Command::FlipIt {
            preset: id.to_owned(),
            axis,
        })
    }

    pub(super) fn flip_it_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.flip_it.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Flip It closed; nothing changed".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Flip It canceled".into(),
            form::Outcome::Ok => {
                let axis = dialog.form.values.choice("axis").min(2);
                if self.run_flip_it(dialog.preset, axis) {
                    self.remember_form(&dialog.form);
                    ctx.request_repaint();
                } else {
                    // A refusal keeps the dialog open with the reason, as
                    // Set Position does.
                    dialog.form.error = Some(self.status.clone());
                    self.flip_it = Some(dialog);
                }
            }
            _ => self.flip_it = Some(dialog),
        }
    }
}
