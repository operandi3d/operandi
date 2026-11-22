//! Dicer… (EDIT.md v0, provisional): the Edit tab's row opens
//! a dialog (UIC-80..99, UIC-03), and OK runs the cut once
//! (`operandi_core::dicer`). Values are remembered for next time
//! (UIC-95).
use super::*;
use operandi_core::dicer::DicerOptions;

/// Which cut the open dialog is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CutKind {
    Dicer,
}

pub(super) struct CutDialog {
    pub(super) kind: CutKind,
    pub(super) form: form::Dialog,
}

fn dicer_rows() -> Vec<form::Row> {
    use form::{Row, Unit};
    let d = DicerOptions::default();
    vec![
        Row::number("seed", "Random Seed", Unit::Count, d.seed as f64)
            .range(0., 1e9)
            .tip("Picks the planes: the same seed always gives the same chunks"),
        Row::number(
            "planes",
            "Max. Cutting Planes",
            Unit::Count,
            d.planes as f64,
        )
        .range(1., 16.),
        Row::number("gap", "Gap", Unit::Distance, d.gap).range(0., 1e6),
        Row::text("inner", "Inner Material", "")
            .tip("Material of the new inside faces; empty uses the mesh's own"),
        Row::number("scale_x", "Fragment Scale X", Unit::Scalar, 1.)
            .tip("Larger on an axis: more cuts across it, so chunks are shorter that way"),
        Row::number("scale_y", "Fragment Scale Y", Unit::Scalar, 1.),
        Row::number("scale_z", "Fragment Scale Z", Unit::Scalar, 1.),
    ]
}

impl Operandi {
    pub(super) fn open_cut_dialog(&mut self, kind: CutKind) {
        if self.cut_dialog.is_some() {
            return;
        }
        let title = match kind {
            CutKind::Dicer => "Dicer",
        };
        if self.has_modal() {
            self.status = format!("Close the open dialog before {title}");
            return;
        }
        let rows = match kind {
            CutKind::Dicer => dicer_rows(),
        };
        self.cut_dialog = Some(CutDialog {
            kind,
            form: self.form_dialog(title, rows),
        });
    }

    pub(super) fn cut_dialog_frame(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.cut_dialog.take() else {
            return;
        };
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Canceled; nothing changed".into(),
            form::Outcome::Ok => {
                let v = &dialog.form.values;
                let command = match dialog.kind {
                    CutKind::Dicer => Command::Dicer(DicerOptions {
                        seed: v.number("seed").round() as u32,
                        planes: v.number("planes").round() as u32,
                        gap: v.number("gap"),
                        inner_material: v.text("inner").to_owned(),
                        fragment_scale: DVec3::new(
                            v.number("scale_x"),
                            v.number("scale_y"),
                            v.number("scale_z"),
                        ),
                    }),
                };
                let title = dialog.form.title.clone();
                let rows = match dialog.kind {
                    CutKind::Dicer => dicer_rows(),
                };
                self.dialog_memory
                    .remember(&title, &rows, &dialog.form.values);
                if !self.run_ok(command) {
                    // The status line says why; the dialog stays for a fix.
                    dialog.form.error = Some(self.status.clone());
                    self.cut_dialog = Some(dialog);
                }
            }
            _ => self.cut_dialog = Some(dialog),
        }
    }
}
