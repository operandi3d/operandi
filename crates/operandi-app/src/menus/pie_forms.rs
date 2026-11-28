//! Pies as forms (UIC-308..313): reading a pie from its form, the pie
//! commands (`operandi.pie.*`), the Form Editor's slot edits
//! (`operandi.form.slot`, `operandi.form.swap`) and Export Pie / Import
//! Pie.
//!
//! - A pie is a form with style pie (UIC-308). Its first eight controls are
//!   its slots, clockwise from the top: a `command` control runs its line
//!   (any registry line, alias or `@macro`, UIC-309), a `form` control
//!   naming another pie opens that sub-pie in place (UIC-310), and a
//!   `space` control is an empty slot that keeps its place.
//! - Shipped pies are files (`assets/forms/pie.*.json`); edits are the
//!   user's copy over them and Revert drops it (UIC-216), as for every
//!   form.
//! - A pie never contains itself: setting a sub-pie that leads back to the
//!   pie is refused, and a sub-pie already open isn't opened again.
//! - Export Pie writes one file with the pie and every sub-pie it reaches
//!   (UIC-312); Import Pie reads it, asking first when an id is taken
//!   (rename keeps both, replace overwrites the user's copy).
use super::*;
use crate::form::command_form::{self as cf, Control, Form, FormRef, FormStore, IconMode, Style};
use crate::pie_menu::{self, Entry, Pie, SLOTS};
use crate::workbench::{bound, control_name, index_arg, word};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The pie file format's version (Export Pie).
const FILE_VERSION: u32 = 1;
/// A pie file's extension (JSON text).
pub(crate) const EXTENSION: &str = "oppie";

/// One Export Pie file (UIC-312): the pie's id and its form with every
/// sub-pie form it reaches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PieFile {
    pub version: u32,
    /// The pie the file was exported from (the others are its sub-pies).
    pub pie: String,
    pub forms: Vec<Form>,
    /// Custom icon files the forms wear, by name, base64 (UIC-314). A
    /// control names one as `embedded:<name>` in its iconFile.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub icons: BTreeMap<String, String>,
}

/// An Import Pie waiting for the user's answer: these ids are taken.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PendingImport {
    pub path: PathBuf,
    pub clashes: Vec<String>,
}

/// Pie editing state for this session.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    /// Import Pie asked whether to rename or replace.
    pub import: Option<PendingImport>,
    /// The pie Export Pie's save panel is for.
    pub export: Option<String>,
    /// Pick is on for this control (form, index): the next button clicked
    /// fills it (UIC-314, form_icons.rs).
    pub pick: Option<(String, usize)>,
    /// The control a Choose File… panel is for (UIC-314).
    pub icon_target: Option<(String, usize)>,
}

/// The pie that form `form` describes.
pub(crate) fn pie_from_form(store: &FormStore, form: &Form) -> Pie {
    Pie {
        name: form.id.clone(),
        title: if form.label.is_empty() {
            form.id.clone()
        } else {
            form.label.clone()
        },
        icons: form.icons,
        entries: form
            .controls
            .iter()
            .take(SLOTS)
            .map(|c| slot_entry(store, c))
            .collect(),
    }
}

/// A pie form's control as a slot: a command, a sub-pie, or empty.
fn slot_entry(store: &FormStore, c: &Control) -> Option<Entry> {
    if let Some(line) = &c.command {
        let label = if c.label.is_empty() {
            line.clone()
        } else {
            c.label.clone()
        };
        return Some(Entry {
            label,
            command: line.clone(),
            icon: c.icon.clone(),
            icon_file: c.icon_file.clone(),
            sub: None,
        });
    }
    let Some(FormRef::Id(id)) = &c.form else {
        return None;
    };
    let label = if !c.label.is_empty() {
        c.label.clone()
    } else {
        store
            .get(id)
            .map(|f| f.label.clone())
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| id.clone())
    };
    Some(Entry {
        label,
        command: pie_menu::line(id),
        icon: c.icon.clone(),
        icon_file: c.icon_file.clone(),
        sub: Some(id.clone()),
    })
}

/// The sub-pie ids a pie form's slots name.
fn subs_of(form: &Form) -> impl Iterator<Item = &str> {
    form.controls
        .iter()
        .take(SLOTS)
        .filter_map(|c| match &c.form {
            Some(FormRef::Id(id)) => Some(id.as_str()),
            _ => None,
        })
}

/// Every pie reachable from `id` through sub-pie slots, `id` first, each
/// once (a loop stops instead of hanging).
pub(crate) fn reachable(store: &FormStore, id: &str) -> Vec<String> {
    let mut out = vec![id.to_owned()];
    let mut i = 0;
    while i < out.len() {
        if let Some(form) = store.get(&out[i]) {
            for sub in subs_of(form) {
                if !out.iter().any(|o| o == sub) {
                    out.push(sub.to_owned());
                }
            }
        }
        i += 1;
    }
    out
}

/// Whether pie `form` opening sub-pie `sub` would make it contain itself
/// (UIC-310).
pub(crate) fn would_loop(store: &FormStore, form: &str, sub: &str) -> bool {
    reachable(store, sub).iter().any(|r| r == form)
}

/// A pie form's controls padded with empty slots to the eight places.
pub(crate) fn padded(form: &mut Form) {
    while form.controls.len() < SLOTS {
        form.controls.push(empty_slot());
    }
}

/// An empty slot that keeps its place (UIC-309).
pub(crate) fn empty_slot() -> Control {
    Control {
        space: Some(0.),
        ..Default::default()
    }
}

/// The first free id like `id` among `taken`: `id.2`, `id.3`, ...
fn free_id(id: &str, taken: &dyn Fn(&str) -> bool) -> String {
    (2..)
        .map(|n| format!("{id}.{n}"))
        .find(|c| !taken(c))
        .expect("some id is free")
}

impl Operandi {
    /// A pie by id (or a shipped pie's old action name): a form with style
    /// pie, the user's copy over the shipped one, or the Layouts pie.
    pub(super) fn pie_named(&self, id: &str) -> Option<Pie> {
        let id = pie_menu::pie_id(id).unwrap_or_else(|| id.trim().to_owned());
        if id == pie_menu::LAYOUTS {
            return Some(self.layouts_pie());
        }
        let form = self.forms.get(&id)?;
        (form.style == Style::Pie).then(|| pie_from_form(&self.forms, form))
    }

    /// Every pie id the Input Map can bind, sorted: the forms with style
    /// pie and the Layouts pie (UIC-311).
    pub(crate) fn pie_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .forms
            .ids()
            .into_iter()
            .filter(|id| self.forms.get(id).is_some_and(|f| f.style == Style::Pie))
            .map(str::to_owned)
            .chain(std::iter::once(pie_menu::LAYOUTS.to_owned()))
            .collect();
        ids.sort();
        ids
    }

    /// What the Input Map calls an action: a pie's line reads "Pie: " and
    /// its name, the rest their own names.
    pub(crate) fn action_label(&self, action: &str) -> String {
        if let Some(title) = pie_menu::shipped_title(action) {
            return title.to_owned();
        }
        match pie_menu::pie_id(action).and_then(|id| self.pie_named(&id)) {
            Some(pie) => format!("Pie: {}", pie.title),
            None => action.to_owned(),
        }
    }

    /// `operandi.pie.*` and the pie slot edits: one handler reads the line.
    pub(super) fn pie_command(&mut self, name: &str, rest: &str) {
        match self.pie_run(&name.to_ascii_lowercase(), rest) {
            Ok(status) => self.status = status,
            Err(e) => {
                self.last_failure = Some(e.clone());
                self.status = e;
            }
        }
    }

    fn pie_run(&mut self, name: &str, rest: &str) -> std::result::Result<String, String> {
        match name {
            "operandi.pie.open" => {
                let a = bound(name, rest, &["id"])?;
                let id = a[0].clone().ok_or("Give id: (the pie to open)")?;
                let id = pie_menu::pie_id(&id).unwrap_or(id);
                if self.pie_named(&id).is_none() {
                    return Err(format!("No pie called {id}"));
                }
                // Typed, or run from a slot: opens tapped, waiting for a
                // click (a key opens it held, `pie_keys`).
                self.pie_request = Some(id.clone());
                Ok(format!("Pie {id}"))
            }
            "operandi.pie.assignkey" => {
                let a = bound(name, rest, &["id", "key"])?;
                let id = a[0].clone().ok_or("Give id: (the pie to bind)")?;
                if self.pie_named(&id).is_none() {
                    return Err(format!("No pie called {id}"));
                }
                let line = pie_menu::line(&id);
                if !self.show_bindings {
                    self.open_input_map();
                }
                // Only this pie's row shows, so the key goes where it was meant.
                self.input_map_ui.search = line.clone();
                self.input_map_ui.mode = None;
                self.input_map_ui.only_assigned = false;
                match a[1].as_deref().map(str::trim) {
                    Some(key) => {
                        let chord = if key.eq_ignore_ascii_case("none") || key.is_empty() {
                            String::new()
                        } else if input::chord(key).is_some() {
                            key.to_owned()
                        } else {
                            return Err(format!("{key} isn't a key"));
                        };
                        // The Input Map's own assign: the clash banner warns
                        // (UIC-311); Save Input Map keeps it.
                        self.assign(&line, chord);
                        Ok(self.status.clone())
                    }
                    None => {
                        self.record_keys_for(line);
                        Ok(format!(
                            "Press the keys for {} (Esc cancels)",
                            self.action_label(&pie_menu::line(&id))
                        ))
                    }
                }
            }
            "operandi.pie.export" => {
                let a = bound(name, rest, &["id", "filename"])?;
                let id = a[0].clone().ok_or("Give id: (the pie to export)")?;
                self.pie_file(&id)?;
                match a[1].clone().filter(|p| !p.trim().is_empty()) {
                    Some(path) => self.export_pie_to(&id, Path::new(path.trim())),
                    None => {
                        let folder = self.saved_pies_folder();
                        let _ = std::fs::create_dir_all(&folder);
                        self.pie_edit.export = Some(id.clone());
                        if self.choose(
                            crate::file_chooser::Choose::PieFile {
                                folder: folder.clone(),
                                name: Some(format!("{id}.{EXTENSION}")),
                            },
                            crate::file_chooser::Purpose::ExportPie,
                        ) {
                            return Ok(format!("Export Pie: choose where to save {id}"));
                        }
                        // No file panel (tests, headless): the Saved Pies
                        // folder, like Save Form (LAY-Q67).
                        self.pie_edit.export = None;
                        self.export_pie_to(&id, &folder.join(format!("{id}.{EXTENSION}")))
                    }
                }
            }
            "operandi.pie.import" => {
                let a = bound(name, rest, &["filename", "onClash"])?;
                let Some(path) = a[0].clone().filter(|p| !p.trim().is_empty()) else {
                    let folder = self.saved_pies_folder();
                    let _ = std::fs::create_dir_all(&folder);
                    if self.choose(
                        crate::file_chooser::Choose::PieFile { folder, name: None },
                        crate::file_chooser::Purpose::ImportPie,
                    ) {
                        return Ok("Import Pie: choose a pie file".into());
                    }
                    return Err("Give filename: (a file written by Export Pie)".into());
                };
                let on_clash = a[1].as_deref().unwrap_or("ask").to_ascii_lowercase();
                self.import_pie_from(Path::new(path.trim()), &on_clash)
            }
            "operandi.form.slot" => {
                let a = bound(name, rest, &["index", "line", "sub", "label", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                if index >= SLOTS {
                    return Err(format!(
                        "A pie has {SLOTS} slots (index 0 to {})",
                        SLOTS - 1
                    ));
                }
                let id = self.pie_form_id(a[4].as_deref())?;
                let line = a[1]
                    .clone()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty());
                let sub = a[2]
                    .clone()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty());
                let label = a[3].clone().unwrap_or_default();
                let control = match (line, sub) {
                    (Some(_), Some(_)) => {
                        return Err("Give line: or sub:, not both".into());
                    }
                    (Some(line), None) => Control {
                        command: Some(line),
                        label,
                        ..Default::default()
                    },
                    (None, Some(sub)) => {
                        match self.forms.get(&sub) {
                            Some(f) if f.style == Style::Pie => {}
                            Some(_) => return Err(format!("{sub} isn't a pie")),
                            None => return Err(format!("No pie called {sub}")),
                        }
                        // UIC-310: a pie can never contain itself.
                        if would_loop(&self.forms, &id, &sub) {
                            return Err(format!(
                                "{sub} leads back to {id}: a pie can't contain itself"
                            ));
                        }
                        Control {
                            form: Some(FormRef::Id(sub)),
                            label,
                            ..Default::default()
                        }
                    }
                    (None, None) => empty_slot(),
                };
                let what = if control.space.is_some() {
                    "empty".to_owned()
                } else {
                    control_name(&control)
                };
                let status = self.form_edit(&id, |f| {
                    padded(f);
                    // A slot's icon stays when only its line changes.
                    let icon = f.controls[index].icon.clone();
                    f.controls[index] = Control {
                        icon: if control.space.is_some() {
                            None
                        } else {
                            control.icon.clone().or(icon)
                        },
                        ..control
                    };
                    Ok(format!("{id} slot {index}: {what}"))
                })?;
                self.form_editor.selected = Some((id, Some(index)));
                Ok(status)
            }
            "operandi.form.swap" => {
                let a = bound(name, rest, &["a", "b", "form"])?;
                let first = index_arg(a[0].as_deref(), "a")?;
                let second = index_arg(a[1].as_deref(), "b")?;
                if first >= SLOTS || second >= SLOTS {
                    return Err(format!("A pie has {SLOTS} slots (0 to {})", SLOTS - 1));
                }
                let id = self.pie_form_id(a[2].as_deref())?;
                let status = self.form_edit(&id, |f| {
                    padded(f);
                    f.controls.swap(first, second);
                    Ok(format!("{id}: slots {first} and {second} swapped"))
                })?;
                self.form_editor.selected = Some((id, Some(second)));
                Ok(status)
            }
            other => Err(format!("{other} isn't a pie command")),
        }
    }

    /// The pie form a slot edit acts on: `given`, else the one selected in
    /// the Form Editor. It must be a pie.
    fn pie_form_id(&self, given: Option<&str>) -> std::result::Result<String, String> {
        let id = given
            .map(str::to_owned)
            .or_else(|| self.form_editor.selected.as_ref().map(|(f, _)| f.clone()))
            .ok_or("Give form: (or select a pie in the Form Editor)")?;
        match self.forms.get(&id) {
            Some(f) if f.style == Style::Pie => Ok(id),
            Some(_) => Err(format!("{id} isn't a pie (its style isn't pie)")),
            None => Err(format!("No form called {id}")),
        }
    }

    /// The data folder's Saved Pies folder (Export Pie's default).
    fn saved_pies_folder(&self) -> PathBuf {
        self.environment.paths().data_root().join("Saved Pies")
    }

    /// The file Export Pie writes for pie `id`: its form and every sub-pie
    /// it reaches (UIC-312).
    pub(crate) fn pie_file(&self, id: &str) -> std::result::Result<PieFile, String> {
        if id == pie_menu::LAYOUTS {
            return Err("The Layouts pie is made from your layouts; it has no file".into());
        }
        match self.forms.get(id) {
            Some(f) if f.style == Style::Pie => {}
            Some(_) => return Err(format!("{id} isn't a pie")),
            None => return Err(format!("No pie called {id}")),
        }
        let mut forms: Vec<Form> = reachable(&self.forms, id)
            .iter()
            .filter_map(|p| self.forms.get(p).cloned())
            .filter(|f| f.style == Style::Pie)
            .collect();
        // UIC-314: custom icon files travel inside the file.
        let icons = crate::form_icons::embed_icons(&mut forms);
        Ok(PieFile {
            version: FILE_VERSION,
            pie: id.to_owned(),
            forms,
            icons,
        })
    }

    fn export_pie_to(&mut self, id: &str, path: &Path) -> std::result::Result<String, String> {
        let file = self.pie_file(id)?;
        if let Some(folder) = path.parent()
            && !folder.as_os_str().is_empty()
        {
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n";
        std::fs::write(path, text).map_err(|e| format!("Couldn't export {id}: {e}"))?;
        let subs = file.forms.len() - 1;
        Ok(format!(
            "Exported pie {id}{} to {}",
            match subs {
                0 => String::new(),
                1 => " and its sub-pie".into(),
                n => format!(" and its {n} sub-pies"),
            },
            path.display()
        ))
    }

    /// Export Pie's save panel answered.
    pub(super) fn export_pie_chosen(&mut self, path: PathBuf) {
        if let Some(id) = self.pie_edit.export.take() {
            self.execute(&format!(
                "operandi.pie.export {id} filename:{}",
                word(&path.to_string_lossy())
            ));
        }
    }

    /// Import Pie's open panel answered.
    pub(super) fn import_pie_chosen(&mut self, path: PathBuf) {
        self.execute(&format!(
            "operandi.pie.import {}",
            word(&path.to_string_lossy())
        ));
    }

    /// Read a pie file and add its pies. `on_clash`: ask (when an id is
    /// taken, ask first), rename or replace.
    fn import_pie_from(
        &mut self,
        path: &Path,
        on_clash: &str,
    ) -> std::result::Result<String, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        let file = read_pie_file(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let clashes: Vec<String> = file
            .forms
            .iter()
            .filter(|f| self.forms.get(&f.id).is_some() || f.id == pie_menu::LAYOUTS)
            .map(|f| f.id.clone())
            .collect();
        let mut forms = file.forms.clone();
        let mut root = file.pie.clone();
        match on_clash {
            "ask" if !clashes.is_empty() => {
                let names = clashes.join(", ");
                self.pie_edit.import = Some(PendingImport {
                    path: path.to_path_buf(),
                    clashes,
                });
                return Ok(format!(
                    "Import Pie: {names} already exist; rename or replace?"
                ));
            }
            "ask" | "replace" => {}
            "rename" => {
                // Every taken id gets a free one, and slots naming it follow.
                let mut renames: BTreeMap<String, String> = BTreeMap::new();
                for id in &clashes {
                    let new = free_id(id, &|c| {
                        self.forms.get(c).is_some()
                            || forms.iter().any(|f| f.id == c)
                            || renames.values().any(|v| v == c)
                    });
                    renames.insert(id.clone(), new);
                }
                for f in &mut forms {
                    if let Some(new) = renames.get(&f.id) {
                        f.id = new.clone();
                    }
                    for c in &mut f.controls {
                        if let Some(FormRef::Id(sub)) = &mut c.form
                            && let Some(new) = renames.get(sub.as_str())
                        {
                            *sub = new.clone();
                        }
                    }
                }
                if let Some(new) = renames.get(&root) {
                    root = new.clone();
                }
            }
            other => return Err(format!("onClash: {other} isn't ask, rename or replace")),
        }
        if forms.iter().any(|f| f.id == pie_menu::LAYOUTS) {
            return Err("pie.layouts is the Layouts pie; rename the pie in the file".into());
        }
        // UIC-314: the file's custom icons go into this user's icons folder.
        crate::form_icons::unpack_icons(self, &mut forms, &file.icons)?;
        for f in forms {
            self.forms.save(f)?;
        }
        // Replacing could have tied pies already here into a loop through
        // the new ones; the open-time check keeps that from hanging, and
        // the status says so.
        let looped = reachable(&self.forms, &root)
            .iter()
            .skip(1)
            .any(|s| would_loop(&self.forms, &root, s));
        self.form_editor.selected = Some((root.clone(), None));
        Ok(if looped {
            format!(
                "Imported pie {root}; one of its sub-pies leads back to it, so that slot stays greyed"
            )
        } else {
            format!("Imported pie {root}")
        })
    }

    /// Import Pie's question (UIC-97's confirmation frame): Rename keeps
    /// both, Replace overwrites, Cancel imports nothing. Each button runs
    /// the line with its answer.
    pub(super) fn pie_import_dialog(&mut self, ctx: &Context) {
        let Some(pending) = self.pie_edit.import.clone() else {
            return;
        };
        let mut answer: Option<&str> = None;
        let mut cancel = false;
        crate::dialog_place::modal(ctx, Id::new("Import Pie")).show(ctx, |ui| {
            ui.set_width(380.);
            ui.heading("Import Pie");
            ui.label(format!(
                "These pies already exist: {}.",
                pending.clashes.join(", ")
            ));
            ui.label("Rename keeps yours and gives the imported ones new ids; Replace overwrites yours (Revert brings a shipped pie back).");
            ui.add_space(6.);
            ui.horizontal(|ui| {
                if ui.button("Rename").clicked() {
                    answer = Some("rename");
                }
                if ui.button("Replace").clicked() {
                    answer = Some("replace");
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    cancel = true;
                }
            });
        });
        if cancel {
            self.pie_edit.import = None;
            self.status = "Import Pie cancelled".into();
        } else if let Some(answer) = answer {
            self.pie_edit.import = None;
            self.execute(&format!(
                "operandi.pie.import {} onClash:{answer}",
                word(&pending.path.to_string_lossy())
            ));
        }
    }
}

/// Parse a pie file: its version, every form a pie, the named pie among
/// them, ids unique.
pub(crate) fn read_pie_file(text: &str) -> std::result::Result<PieFile, String> {
    let file: PieFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if file.version != FILE_VERSION {
        return Err(format!(
            "pie file version {} (this build reads {FILE_VERSION})",
            file.version
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for f in &file.forms {
        if f.version != cf::FORM_VERSION {
            return Err(format!("{}: form version {}", f.id, f.version));
        }
        if f.id.trim().is_empty() {
            return Err("a pie in the file has no id".into());
        }
        if f.style != Style::Pie {
            return Err(format!("{} isn't a pie", f.id));
        }
        f.check()?;
        if !ids.insert(f.id.clone()) {
            return Err(format!("{} is in the file twice", f.id));
        }
    }
    if !ids.contains(&file.pie) {
        return Err(format!("the file doesn't hold its pie {}", file.pie));
    }
    Ok(file)
}

/// A new pie form: eight empty slots, icons and text (UIC-309).
pub(crate) fn new_pie_controls() -> Vec<Control> {
    (0..SLOTS).map(|_| empty_slot()).collect()
}

/// The icon mode a new pie starts with.
pub(crate) const NEW_PIE_ICONS: IconMode = IconMode::Both;

/// The pie commands' handlers (UIC-160: every pie action is a command).
pub(crate) fn register(r: &mut crate::command_handlers::Commands) {
    crate::command_handlers::on_words(
        r,
        &[
            "operandi.pie.open",
            "operandi.pie.assignKey",
            "operandi.pie.export",
            "operandi.pie.import",
            "operandi.form.slot",
            "operandi.form.swap",
        ],
        |a, c| a.pie_command(c.entry.name, c.rest),
    );
}
