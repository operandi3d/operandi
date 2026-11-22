//! Relax Mesh... (Deform tab Commands, DEFORM §3 and DEFORM-PROBES
//! DP-20..22): a command with an options dialog built with the shared
//! dialog (UIC-80..99). OK runs `Command::RelaxMesh` once (one undo step,
//! UIC-96); the values are remembered for next time (UIC-95).
//! `sculpt.relaxMesh iter:<n> constrain:<bool>` runs it without the dialog
//! (DP-22).
use super::*;
use operandi_core::RelaxMeshOptions;

/// The open Relax Mesh dialog.
pub(super) struct RelaxMeshDialog {
    pub(super) form: form::Dialog,
}

impl RelaxMeshDialog {
    fn rows(o: &RelaxMeshOptions) -> Vec<form::Row> {
        use form::{Row, Unit};
        vec![
            Row::number("iterations", "Iterations", Unit::Count, o.iterations as f64)
                .range(1., 1000.)
                .tip("How many relax passes to run"),
            Row::check("constrain", "Constrain", o.constrain).tip(
                "Relax against the fixed border until the points settle: a stronger, flatter result (provisional, RM-Q1)",
            ),
        ]
        .into_iter()
        .map(form::Row::fresh)
        .collect()
    }

    pub(super) fn options(&self) -> RelaxMeshOptions {
        let v = &self.form.values;
        RelaxMeshOptions {
            iterations: v.number("iterations").round().max(0.) as u32,
            constrain: v.bool("constrain"),
        }
    }
}

/// `sculpt.relaxMesh` arguments over the remembered options.
pub(super) fn parse_relax_mesh(
    base: RelaxMeshOptions,
    args: &str,
) -> std::result::Result<RelaxMeshOptions, String> {
    let mut o = base;
    for arg in args.split_whitespace() {
        let (key, value) = operandi_core::command_line::named(arg);
        match key.map(str::to_ascii_lowercase).as_deref() {
            Some("iter") | Some("iterations") => {
                o.iterations = value
                    .parse::<u32>()
                    .map_err(|_| "Iterations must be a whole number from 1 to 1000".to_owned())?
            }
            Some("constrain") => {
                o.constrain = operandi_core::command_args::boolean("constrain", value)?
            }
            _ => return Err(format!("sculpt.relaxMesh: unknown argument \"{arg}\"")),
        }
    }
    o.validate()?;
    Ok(o)
}

impl Operandi {
    /// Relax Mesh... opens its dialog (the toolbox row, the command line).
    pub(super) fn open_relax_mesh(&mut self) {
        if self.relax_mesh.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Relax Mesh".into();
            return;
        }
        let rows = RelaxMeshDialog::rows(&self.editor.relax_mesh_preset);
        self.relax_mesh = Some(RelaxMeshDialog {
            form: self.form_dialog("Relax Mesh", rows),
        });
    }

    /// `sculpt.relaxMesh` with arguments runs at once; bare, the dialog.
    pub(super) fn relax_mesh_line(&mut self, rest: &str) {
        if rest.trim().is_empty() {
            self.open_relax_mesh();
            return;
        }
        match parse_relax_mesh(self.editor.relax_mesh_preset, rest) {
            Ok(options) => self.run_relax_mesh(options),
            Err(e) => self.status = e,
        }
    }

    fn run_relax_mesh(&mut self, options: RelaxMeshOptions) {
        if self.run_ok(Command::RelaxMesh(options)) {
            self.status = format!(
                "Relax Mesh: {} iteration{}{}",
                options.iterations,
                if options.iterations == 1 { "" } else { "s" },
                if options.constrain {
                    ", constrained"
                } else {
                    ""
                }
            );
        }
    }

    /// The dialog, once per frame.
    pub(super) fn relax_mesh_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.relax_mesh.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Relax Mesh closed; nothing changed".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = "Relax Mesh canceled; nothing changed".into();
            }
            form::Outcome::Ok => {
                let options = dialog.options();
                match options.validate() {
                    Ok(()) => self.run_relax_mesh(options),
                    Err(e) => {
                        self.status = e.clone();
                        dialog.form.error = Some(e);
                        self.relax_mesh = Some(dialog);
                    }
                }
            }
            _ => self.relax_mesh = Some(dialog),
        }
    }
}
