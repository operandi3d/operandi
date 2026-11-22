//! PART-002/003 Polygon Set Part dialog and command route. Typing and list
//! choice change only this modal's draft; OK, Return and `poly.setPart <name>`
//! dispatch the one shared core operation (`Command::SetPart`).
use super::*;

pub(super) struct SetPartDialog {
    /// Eligible target captured at open; commit refuses if it went stale.
    target: PartTarget,
    form: form::Dialog,
}

#[cfg(test)]
impl SetPartDialog {
    /// The Name field's text.
    pub(super) fn name(&self) -> &str {
        self.form.values.text("name")
    }
}

/// The dialog's rows (UIC-98). The name starts fresh each time from the
/// targets' shared part or the Default Part preference (SP-10/SP-Q03),
/// so it isn't remembered (A6 on: the last name applied this run).
fn rows(name: &str, names: Vec<String>) -> Vec<form::Row> {
    vec![
        form::Row::text("name", "Name", name)
            .suggestions(names)
            .fresh()
            .tip("The part name for the selected polygons; ▾ lists the existing parts"),
    ]
}

/// Original-compatible A6: Set Part starts with the last name it applied.
const DEFAULT_PART_START: &str = "A6";

/// The Name field's starting text. SP-10/SP-Q03 (Jason): the part the
/// targets already share, otherwise the Default Part preference. With A6
/// on, the selection's part is ignored: the last name Set Part applied
/// this run, or the Default Part preference before any use.
fn starting_name(
    shared: Option<String>,
    last_applied: Option<&str>,
    default_part: &str,
    original: bool,
) -> String {
    if original {
        return last_applied.unwrap_or(default_part).to_owned();
    }
    shared.unwrap_or_else(|| default_part.to_owned())
}

impl Operandi {
    /// Panel row, bare `poly.setPart` and the input-map action share this.
    /// An ineligible context is refused before any modal or draft change.
    pub(super) fn open_set_part(&mut self) {
        if self.set_part.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Set Part".into();
            return;
        }
        match self.editor.set_part_target() {
            Ok(target) => {
                let name = starting_name(
                    Editor::common_part(&target),
                    self.last_set_part.as_deref(),
                    &self.defaults_preferences.default_part,
                    crate::original_compatible::is_original(DEFAULT_PART_START),
                );
                let rows = rows(&name, self.editor.part_names());
                self.set_part = Some(SetPartDialog {
                    target,
                    form: self.form_dialog("Set Part", rows).focus("name"),
                });
            }
            Err(e) => self.status = e,
        }
    }
    /// One Set Part dispatch. Field drafts of other panels are neither
    /// confirmed nor discarded (see `run`). Returns false on refusal.
    pub(super) fn assign_part(&mut self, name: String, expected: Option<PartTarget>) -> bool {
        if let Err(e) = validate_part_name(&name) {
            self.status = e;
            return false;
        }
        let before = self.editor.scene.clone();
        let succeeded = self.run_ok(Command::SetPart(PartAssignment {
            name: name.clone(),
            expected,
        }));
        if !succeeded {
            return false;
        }
        self.last_set_part = Some(name.clone());
        self.status = if self.editor.scene == before {
            format!("Set Part: those polygons already use “{name}”")
        } else {
            format!("Set Part: “{name}”")
        };
        true
    }
    pub(super) fn set_part_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.set_part.take() else {
            return;
        };
        // A file/close prompt or import that opens meanwhile owns the screen;
        // abandon only this draft.
        if self.has_modal() {
            self.status = "Set Part closed; nothing assigned".into();
            return;
        }
        // A blank or malformed name greys OK, with the reason as its tooltip.
        dialog
            .form
            .block_ok(validate_part_name(dialog.form.values.text("name")).err());
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Set Part canceled".into(),
            form::Outcome::Ok => {
                let name = dialog.form.values.text("name").to_owned();
                match validate_part_name(&name) {
                    // Stale or refused targets close with the reason; nothing applies.
                    Ok(()) => {
                        self.assign_part(name, Some(dialog.target.clone()));
                    }
                    Err(e) => {
                        self.status = e.clone();
                        dialog.form.error = Some(e);
                        self.set_part = Some(dialog);
                    }
                }
            }
            _ => {
                // Shown live while the name can't be used.
                if let Err(e) = validate_part_name(dialog.form.values.text("name")) {
                    self.status = e;
                }
                self.set_part = Some(dialog);
            }
        }
    }
}

#[cfg(test)]
mod starting_name_tests {
    use super::starting_name;

    /// Original-compatible A6: on, the selection's part is ignored and the
    /// name is the last one applied, or the Default Part before any use;
    /// off, the selection's shared part first, as today.
    #[test]
    fn a6_set_part_starts_with_the_last_name_applied_when_original_compatible() {
        let wheel = || Some("Wheel".to_owned());
        assert_eq!(starting_name(wheel(), None, "Default", false), "Wheel");
        assert_eq!(
            starting_name(wheel(), Some("Hub"), "Default", false),
            "Wheel"
        );
        assert_eq!(
            starting_name(None, Some("Hub"), "Default", false),
            "Default"
        );
        assert_eq!(starting_name(wheel(), None, "Default", true), "Default");
        assert_eq!(starting_name(wheel(), Some("Hub"), "Default", true), "Hub");
        assert_eq!(starting_name(None, Some("Hub"), "Default", true), "Hub");
    }
}
