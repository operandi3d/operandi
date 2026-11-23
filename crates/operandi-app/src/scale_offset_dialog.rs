//! The Enter Inset Amount dialog for PolyScale Offset and its Detach
//! variant (spec polyscale-offset-2026-11-06; earlier SO-U01/U02).
//!
//! The Polygon row, the `@ppi_PolyScale_Offset.py` command and the remappable
//! "Scale Offset" input action all open this one dialog. Opening runs the
//! core's read-only offer check: context, selection, ownership and every
//! distance-independent geometry/data/transform refusal (concave, nonplanar,
//! shared points, ...), so an active tool is refused rather than dropped and
//! an unsupported polygon is refused before the dialog. Editing the field never
//! changes geometry; a valid OK or Return dispatches the reviewed core edit
//! `Command::ScaleOffsetPolygon` once. The draft is bound to the document and
//! scene it was opened on, so a replacement document or changed target is
//! refused instead of receiving a stale confirmation. Distance-dependent
//! refusals (for example an inset that collapses the polygon) are reported
//! by the core on OK and keep the dialog open for a retry.
use super::*;

/// Exact command-entry spelling of the action; matched case-insensitively
/// like every other command-entry name. No script host is involved.
pub(super) const SCALE_OFFSET_COMMAND: &str = "@ppi_PolyScale_Offset.py";

pub(super) struct ScaleOffsetDialog {
    /// PolyScale Offset Detach: the inset polygons get their own points.
    detach: bool,
    document: DocumentToken,
    scene: Scene,
    form: form::Dialog,
}
#[cfg(test)]
impl ScaleOffsetDialog {
    pub(super) fn has_error(&self) -> bool {
        self.form.error.is_some()
    }
    pub(super) fn error_text(&self) -> String {
        self.form.error.clone().unwrap_or_default()
    }
}
/// The dialog's one row. SO-U02: it starts at 0 m every time (no
/// retained value), so it isn't remembered (a spec exception to UIC-95).
fn rows() -> Vec<form::Row> {
    vec![
        form::Row::number("inset", "Inset", form::Unit::Distance, 0.)
            .fresh()
            .tip("How far each selected polygon's edges move inward, in its plane"),
    ]
}
impl Operandi {
    pub(super) fn open_scale_offset(&mut self) {
        self.open_scale_offset_as(false);
    }
    /// Scale Offset (Detach)...: the same dialog; OK detaches.
    pub(super) fn open_scale_offset_detach(&mut self) {
        self.open_scale_offset_as(true);
    }
    fn open_scale_offset_as(&mut self, detach: bool) {
        if self.scale_offset.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the current dialog before Scale Offset".into();
            return;
        }
        let eligible = if detach {
            self.editor.scale_offset_polygon_target()
        } else {
            self.editor.scale_offset_polygon_eligibility()
        };
        match eligible {
            Ok(_) => {
                // UIC-81: the title is the command's name.
                let title = if detach {
                    "Scale Offset (Detach)"
                } else {
                    "Scale Offset"
                };
                self.scale_offset = Some(ScaleOffsetDialog {
                    detach,
                    document: self.editor.identities.document_token(),
                    scene: self.editor.scene.clone(),
                    form: self.form_dialog(title, rows()).focus("inset"),
                });
            }
            Err(error) => self.status = error,
        }
    }

    /// Dispatch one confirmed distance against the dialog's original target.
    /// Returns the refusal to show in the still-open dialog.
    fn confirm_scale_offset(
        &mut self,
        dialog: &ScaleOffsetDialog,
        distance: f64,
    ) -> Option<String> {
        if self.editor.identities.document_token() != dialog.document
            || self.editor.scene != dialog.scene
        {
            self.status =
                "The Scale Offset target changed since the dialog opened; nothing changed".into();
            return Some(self.status.clone());
        }
        let mark = self.editor.history_mark();
        let succeeded = self.run_ok(if dialog.detach {
            Command::ScaleOffsetDetach(distance)
        } else {
            Command::ScaleOffsetPolygon(distance)
        });
        if !succeeded {
            return Some(self.status.clone());
        }
        self.status = if self.editor.recorded_since(mark) {
            "Scale Offset applied".into()
        } else {
            "Scale Offset: nothing changed".into()
        };
        None
    }

    pub(super) fn scale_offset_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.scale_offset.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Scale Offset closed; nothing changed".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = "Scale Offset canceled; nothing changed".into();
            }
            form::Outcome::Ok => {
                let distance = dialog.form.values.number("inset");
                if let Some(error) = self.confirm_scale_offset(&dialog, distance) {
                    dialog.form.error = Some(error);
                    self.scale_offset = Some(dialog);
                }
            }
            _ => self.scale_offset = Some(dialog),
        }
    }
}
