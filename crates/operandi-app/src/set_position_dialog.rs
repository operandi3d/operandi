//! Set Position dialog: a draft bound to its original scene and document,
//! in the shared form dialog (UIC-80..99).
use super::*;

pub(super) struct SetPositionDialog {
    target: PositionTarget,
    form: form::Dialog,
}
const AXES: [PositionAxis; 3] = [PositionAxis::X, PositionAxis::Y, PositionAxis::Z];
fn rows() -> Vec<form::Row> {
    use form::{Row, Unit};
    vec![
        Row::segments("axis", "Axis", &["X", "Y", "Z"], 0).tip("The axis the value sets"),
        Row::number("value", "Value", Unit::Distance, 0.)
            .tip("The position every selected vertex moves to on that axis"),
        Row::check("work_plane", "Work Plane", false)
            .tip("Value is measured from the work plane's centre"),
        Row::check("world_space", "World Space", false).tip("Value is a world coordinate"),
    ]
}
impl Operandi {
    pub(super) fn open_set_position(&mut self) {
        if self.set_position.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the current dialog before Set Position".into();
            return;
        }
        match self.editor.set_position_target() {
            Ok(target) => {
                self.set_position = Some(SetPositionDialog {
                    target,
                    form: self.form_dialog("Set Position", rows()).focus("value"),
                })
            }
            Err(error) => self.status = error,
        }
    }
    pub(super) fn assign_position(
        &mut self,
        axis: PositionAxis,
        value: f64,
        space: PositionSpace,
        target: Option<PositionTarget>,
    ) -> bool {
        self.run_ok(Command::SetPosition(PositionAssignment {
            space,
            axis,
            value,
            expected: target,
        }))
    }
    pub(super) fn set_position_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.set_position.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Set Position closed; nothing changed".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Set Position canceled".into(),
            form::Outcome::Ok => {
                let v = &dialog.form.values;
                let axis = AXES[v.choice("axis").min(2)];
                let value = v.number("value");
                match PositionSpace::from_switches(v.bool("work_plane"), v.bool("world_space")) {
                    Ok(space) => {
                        if self.assign_position(axis, value, space, Some(dialog.target.clone())) {
                            self.remember_form(&dialog.form);
                        } else {
                            dialog.form.error = Some(self.status.clone());
                            self.set_position = Some(dialog);
                        }
                    }
                    Err(error) => {
                        self.status = error.clone();
                        dialog.form.error = Some(error);
                        self.set_position = Some(dialog);
                    }
                }
            }
            _ => self.set_position = Some(dialog),
        }
    }
}
