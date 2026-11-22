//! Selection Sets: the small dialog behind Assign Selection Set, Use
//! Selection Set, Delete Selection Set and Create Selection Set Group, and
//! their `selset.*` command-line verbs. The dialog only edits its own draft;
//! OK dispatches the one shared core command (`Command::AssignSelectionSet`
//! and friends), which is also what the command line and the MCP bridge use.
//!
//! Buttons wire up with `open_selection_set(SetAction::…)`, or through
//! `execute` with the action names "assign selection set", "use selection
//! set", "delete selection set" and "create selection set group".
use super::*;
use operandi_core::command_line::bind_lenient;
use operandi_core::{SetKind, UseSetMode};

/// Which Selection Set command a dialog runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SetAction {
    Assign,
    Use,
    Delete,
    CreateGroup,
}
impl SetAction {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Assign => "Assign Selection Set",
            Self::Use => "Use Selection Set",
            Self::Delete => "Delete Selection Set",
            Self::CreateGroup => "Create Selection Set Group",
        }
    }
}

pub(super) struct SelectionSetDialog {
    action: SetAction,
    /// The current mode's sets on the active mesh when the dialog opened.
    names: Vec<String>,
    form: form::Dialog,
}

#[cfg(test)]
impl SelectionSetDialog {
    pub(super) fn title(&self) -> &'static str {
        self.action.title()
    }
}

/// The dialog's rows for `action` (UIC-98). Names start fresh each time
/// (the sets change); Use's mode is remembered (UIC-95).
fn rows(action: SetAction, kind: SetKind, names: &[String]) -> Vec<form::Row> {
    use form::{Row, When};
    let list: Vec<&str> = names.iter().map(String::as_str).collect();
    match action {
        SetAction::Assign => vec![
            Row::text("name", "Name", "")
                .suggestions(names.to_vec())
                .fresh()
                .tip(format!(
                    "A new {} selection set, or ▾ to add to an existing one",
                    kind.label()
                )),
        ],
        SetAction::Use => vec![
            Row::choice("name", "Set", &list, 0)
                .fresh()
                .tip(format!("{} sets of the active mesh", kind.label())),
            Row::segments("mode", "Mode", &["Replace", "Select", "Deselect"], 0)
                .tip("Replace the selection, add the set to it, or remove the set from it"),
        ],
        SetAction::Delete => vec![
            Row::check("all", &format!("All {} Sets", kind.label()), false)
                .fresh()
                .tip("Delete every set of this mode; geometry is never deleted"),
            Row::choice("name", "Set", &list, 0)
                .fresh()
                .when(When::Unchecked("all"))
                .tip(format!("{} sets of the active mesh", kind.label())),
        ],
        SetAction::CreateGroup => {
            let mut rows = vec![
                Row::text("name", "Group", "")
                    .fresh()
                    .tip("The new group's name; an existing group gains the sets"),
            ];
            rows.extend(names.iter().map(|n| {
                Row::check(&format!("set:{n}"), n, false)
                    .fresh()
                    .tip("Sets to put in the group; none ticked puts in all of them")
            }));
            rows
        }
    }
}

/// Selection set verbs (names from the name map; the old `selset.*`
/// spellings are aliases in command_names.rs) and their help text.
pub(crate) const COMMANDS: [(&str, &str); 5] = [
    (
        "select.editset",
        "Help: select.editSet <name> [add] saves the current Vertex, Edge or Polygon selection of the active mesh as a selection set of that mode; an existing set of that name gains the selection. Group spaces with quotes or braces. Bare select.editSet opens the dialog. Old spelling: selset.assign.",
    ),
    (
        "select.useset",
        "Help: select.useSet <name> [replace|add|remove] changes the selection to (replace, the default), adds or removes the members of the current mode's set; also name: and mode:. Bare select.useSet opens the dialog. Old spelling: selset.use.",
    ),
    (
        "select.deleteset",
        "Help: select.deleteSet <name> deletes one selection set of the current mode; geometry is never deleted. select.deleteSet all:true deletes every set of the current mode. Bare select.deleteSet opens the dialog. Old spellings: selset.delete, selset.deleteAll.",
    ),
    (
        "select.convertgroup",
        "Help: select.convertGroup <group> [set ...] makes a selection set group holding the named sets of the current mode, or all of them when none are named; an existing group gains them. Bare select.convertGroup opens the dialog. Old spelling: selset.group.",
    ),
    (
        "selset.ungroup",
        "Help: selset.ungroup <group> deletes a selection set group; its sets stay.",
    ),
];

impl Operandi {
    /// Why `action` can't open its dialog now, or `None`. The dialog's
    /// own refusal and the Select tab rows' greying (their commands'
    /// answers, UIC-223) both come from here, so they always agree.
    pub(super) fn selection_set_unavailable(&self, action: SetAction) -> Option<String> {
        if let Some(reason) = self.editor.selection_set_refusal(action.title()) {
            return Some(reason);
        }
        let kind = SetKind::of_mode(self.editor.scene.selection.mode)?;
        match action {
            SetAction::Assign if !self.editor.has_set_selection() => {
                let what = match kind {
                    SetKind::Vertex => "vertices",
                    SetKind::Edge => "edges",
                    SetKind::Polygon => "polygons",
                };
                Some(format!("Select {what} before {}", action.title()))
            }
            SetAction::Use | SetAction::Delete | SetAction::CreateGroup
                if self.editor.selection_set_names().is_empty() =>
            {
                Some(format!(
                    "This mesh has no {} selection sets; use Assign Selection Set first",
                    kind.label()
                ))
            }
            _ => None,
        }
    }

    /// Buttons, menu rows and bare `selset.*` verbs share this. An
    /// ineligible context is refused before any dialog opens.
    pub(super) fn open_selection_set(&mut self, action: SetAction) {
        if self.selection_set.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = format!("Close the open dialog before {}", action.title());
            return;
        }
        if let Some(reason) = self.selection_set_unavailable(action) {
            self.status = reason;
            return;
        }
        let Some(kind) = SetKind::of_mode(self.editor.scene.selection.mode) else {
            return;
        };
        let names = self.editor.selection_set_names();
        let mut form = self.form_dialog(action.title(), rows(action, kind, &names));
        if matches!(action, SetAction::Assign | SetAction::CreateGroup) {
            form = form.focus("name");
        }
        self.selection_set = Some(SelectionSetDialog {
            action,
            names,
            form,
        });
    }

    /// Dispatch one Selection Set command and report it in the status line.
    /// Returns false when it was refused.
    pub(super) fn run_selection_set(&mut self, command: Command) -> bool {
        let message = match &command {
            Command::AssignSelectionSet(name) => {
                format!("Assigned selection set “{}”", name.trim())
            }
            Command::UseSelectionSet { name, .. } => format!("Selection set “{}”", name.trim()),
            Command::DeleteSelectionSet(Some(name)) => {
                format!("Deleted selection set “{}”", name.trim())
            }
            Command::DeleteSelectionSet(None) => "Deleted every selection set of this mode".into(),
            Command::CreateSelectionSetGroup { name, .. } => {
                format!("Selection set group “{}”", name.trim())
            }
            Command::DeleteSelectionSetGroup(name) => {
                format!("Deleted selection set group “{}”", name.trim())
            }
            _ => String::new(),
        };
        let ok = self.run_ok(command);
        if ok {
            self.status = message;
        }
        ok
    }

    pub(super) fn selection_set_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.selection_set.take() else {
            return;
        };
        if self.has_modal() {
            self.status = format!("{} closed; nothing changed", dialog.action.title());
            return;
        }
        let needs_name = matches!(dialog.action, SetAction::Assign | SetAction::CreateGroup);
        let blank = needs_name && dialog.form.values.text("name").trim().is_empty();
        dialog
            .form
            .block_ok(blank.then(|| "Type a name first".to_owned()));
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = format!("{} canceled", dialog.action.title());
            }
            form::Outcome::Ok => {
                self.remember_form(&dialog.form);
                let v = &dialog.form.values;
                let picked = || {
                    dialog
                        .names
                        .get(v.choice("name"))
                        .cloned()
                        .unwrap_or_default()
                };
                let command = match dialog.action {
                    SetAction::Assign => Command::AssignSelectionSet(v.text("name").to_owned()),
                    SetAction::Use => Command::UseSelectionSet {
                        name: picked(),
                        mode: [UseSetMode::Replace, UseSetMode::Add, UseSetMode::Remove]
                            [v.choice("mode").min(2)],
                    },
                    SetAction::Delete => Command::DeleteSelectionSet((!v.bool("all")).then(picked)),
                    SetAction::CreateGroup => Command::CreateSelectionSetGroup {
                        name: v.text("name").to_owned(),
                        sets: dialog
                            .names
                            .iter()
                            .filter(|n| v.bool(&format!("set:{n}")))
                            .cloned()
                            .collect(),
                    },
                };
                // A refusal closes the dialog with the reason in the status line.
                self.run_selection_set(command);
            }
            _ => self.selection_set = Some(dialog),
        }
    }

    /// `selset.*` command-line verbs and their help topics. Returns false
    /// when `text` is not one of them.
    pub(super) fn selection_set_canonical(&mut self, text: &str) -> bool {
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first().map(|a| a.to_ascii_lowercase()) else {
            return false;
        };
        let help = |topic: &str| COMMANDS.iter().find(|(c, _)| *c == topic).map(|(_, h)| *h);
        if first == "help" && args.len() == 2 {
            return match help(&args[1].to_ascii_lowercase()) {
                Some(h) => {
                    self.status = h.into();
                    true
                }
                None => false,
            };
        }
        let Some(usage) = help(&first) else {
            return false;
        };
        let rest = &args[1..];
        let bad = |app: &mut Self| app.status = usage.trim_start_matches("Help: ").into();
        match (first.as_str(), rest) {
            ("select.editset", []) => self.open_selection_set(SetAction::Assign),
            ("select.editset", _) => match bind_lenient(rest, &["name", "mode"]).as_deref() {
                // Assigning to an existing set already adds to it.
                Some([Some(name), None | Some("add")]) => {
                    self.run_selection_set(Command::AssignSelectionSet((*name).into()));
                }
                _ => bad(self),
            },
            ("select.useset", []) => self.open_selection_set(SetAction::Use),
            ("select.useset", _) => {
                let bound = bind_lenient(rest, &["name", "mode"]);
                let Some([Some(name), mode]) = bound.as_deref() else {
                    bad(self);
                    return true;
                };
                let mode = match mode.map(str::to_ascii_lowercase).as_deref() {
                    None | Some("replace") => UseSetMode::Replace,
                    Some("add" | "select") => UseSetMode::Add,
                    Some("remove" | "deselect") => UseSetMode::Remove,
                    _ => {
                        bad(self);
                        return true;
                    }
                };
                self.run_selection_set(Command::UseSelectionSet {
                    name: (*name).into(),
                    mode,
                });
            }
            ("select.deleteset", []) => self.open_selection_set(SetAction::Delete),
            ("select.deleteset", _) => match bind_lenient(rest, &["name", "all"]).as_deref() {
                Some([None, Some(all)])
                    if operandi_core::command_args::boolean("all", all) == Ok(true) =>
                {
                    self.run_selection_set(Command::DeleteSelectionSet(None));
                }
                Some([Some(name), None]) => {
                    self.run_selection_set(Command::DeleteSelectionSet(Some((*name).into())));
                }
                _ => bad(self),
            },
            ("select.convertgroup", []) => self.open_selection_set(SetAction::CreateGroup),
            ("select.convertgroup", [name, sets @ ..]) => {
                self.run_selection_set(Command::CreateSelectionSetGroup {
                    name: name.clone(),
                    sets: sets.to_vec(),
                });
            }
            ("selset.ungroup", [name]) => {
                self.run_selection_set(Command::DeleteSelectionSetGroup(name.clone()));
            }
            _ => self.status = usage.trim_start_matches("Help: ").into(),
        }
        true
    }
}
