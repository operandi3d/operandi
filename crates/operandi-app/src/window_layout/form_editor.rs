//! The Form Editor (layouts stage 7, BEHAVIOR-LAYOUTS-STAGE7.md: LAY-83,
//! LAY-87, LAY-Q53, UIC-10, UIC-183, UIC-213..215).
//!
//! System > Form Editor opens a floating window (the `formEditor`
//! palette, UIC-183). On the left, a tree of groups > forms > controls
//! with a search field and Find Form (the next click on a button jumps to
//! the form it is in). Groups are for tidiness only: a form's group is
//! where its id's first word puts it (toolbox, toolbar, menu) unless the
//! user moved it (`Layouts/form-groups.json`). On the right, the selected
//! form's or control's properties: a control has the workbench
//! right-click's (label, tooltip, command, width, toggle, pop-up, level,
//! icon file) plus icon, shortcut and show-only-when; a form its label,
//! style, icons, Customizable Workbench (the old file flag, LAY-Q53),
//! head/tail slot and show-only-when (UIC-214, UIC-215), columns and
//! group. New Group, New Form, Add Command (typed, or from the Command
//! History, UIC-160), Add Sub-form, Add Divider, drag to reorder, Preview
//! Form, Save Form to its own file, Undo Edit and Revert (LAY-87).
//!
//! Every edit is an `operandi.form.*` registry command (class "ui") and
//! writes the user's copy over the shipped form (`FormStore::save`), the
//! same layering and undo as workbench edits (workbench.rs).
use super::*;
use crate::form::command_form::{
    self as cf, Control, End, Form, FormRef, IconMode, IconSize, Slot, Style,
};
use crate::unavailable::Gate;
use crate::workbench::{bound, control_name, index_arg, on_arg, set_attr, word};
use operandi_core::command_runtime::CommandState;
use serde::{Deserialize, Serialize};

// UIC-308..313: the ring preview and slot editor for pies.
#[path = "form_editor_pie.rs"]
mod pie_editor;

/// The Form Editor palette's hash (`layout.window hash:formEditor`).
pub(super) const PALETTE: &str = "formEditor";

/// The groups file's name inside the data folder's Layouts folder.
const GROUPS_FILE: &str = "form-groups.json";

/// One group of forms (LAY-83: for tidiness only).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Group {
    pub name: String,
    #[serde(default)]
    pub forms: Vec<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct GroupsFile {
    version: u32,
    #[serde(default)]
    groups: Vec<Group>,
}

/// The Form Editor's state for this session.
#[derive(Default)]
pub(super) struct State {
    /// The search field's text.
    pub search: String,
    /// The selected form, and control by index (`None`: the form itself).
    pub selected: Option<(String, Option<usize>)>,
    /// Find Form is armed: the next clicked button selects its form.
    pub finding: bool,
    /// Preview Form shows the selected form drawn.
    pub preview: bool,
    /// The user's groups (`form-groups.json`).
    pub groups: Vec<Group>,
    groups_loaded: bool,
    /// The fields being typed in: (form, index, attr) -> text.
    drafts: BTreeMap<(String, Option<usize>, String), String>,
    /// The New Group / New Form / Add Command / Add Sub-form fields.
    pub new_group: String,
    pub new_form: String,
    pub add_line: String,
    pub add_sub: String,
    /// A control row being dragged: its form and index.
    drag: Option<(String, usize)>,
    /// Where each tree row drew last frame, by `<form>` or `<form>#<i>`
    /// (tests and the drag); a pie's ring slots by `<form>@<i>`.
    pub rects: BTreeMap<String, Rect>,
    /// A pie slot being dragged onto another (its pie and slot).
    pie_drag: Option<(String, usize)>,
    /// Browse Icons' search text (UIC-314).
    icon_search: String,
    /// A pie slot whose editor shows a command field (`false`) or the
    /// sub-pie pick (`true`) before it holds one.
    pie_kind: Option<(String, usize, bool)>,
}

/// The group a form's id puts it in when no user group holds it.
pub(super) fn default_group(id: &str) -> &'static str {
    match id.split('.').next().unwrap_or_default() {
        "toolbox" => "Toolbox",
        "toolbar" => "Toolbars",
        "menu" => "Menus",
        "pie" => "Pies",
        _ => "Forms",
    }
}

fn style_named(name: &str) -> Option<Style> {
    serde_json::from_value(serde_json::Value::String(lower_first(name.trim()))).ok()
}
fn icons_named(name: &str) -> Option<IconMode> {
    serde_json::from_value(serde_json::Value::String(lower_first(name.trim()))).ok()
}
fn icon_size_named(name: &str) -> Option<IconSize> {
    serde_json::from_value(serde_json::Value::String(lower_first(name.trim()))).ok()
}
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
fn json_word<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// A form attribute set by `operandi.form.set` without `index:`.
fn set_form_attr(f: &mut Form, attr: &str, value: &str) -> std::result::Result<String, String> {
    let bad = |what: &str| format!("{attr}: \"{value}\" isn't one of {what}");
    let text = value.trim();
    let opt = || (!text.is_empty()).then(|| text.to_owned());
    match attr.to_ascii_lowercase().as_str() {
        "label" => f.label = text.to_owned(),
        "style" => {
            f.style = style_named(text)
                .ok_or_else(|| bad("vertical, horizontal, sheet, menu, flyout, hold, pie"))?
        }
        "icons" => f.icons = icons_named(text).ok_or_else(|| bad("icon, text, both"))?,
        "iconsize" => {
            f.icon_size = icon_size_named(text).ok_or_else(|| bad("small, normal, large"))?
        }
        // LAY-Q53: the "Customizable Workbench" setting.
        "workbench" => f.workbench = operandi_core::command_args::boolean("workbench", text)?,
        "exported" => f.exported = operandi_core::command_args::boolean("exported", text)?,
        "showwhen" => f.show_when = opt(),
        // UIC-214: the head/tail slot.
        "slot" => {
            f.slot = opt().map(|form| Slot {
                form,
                at: f.slot.as_ref().map_or(End::Tail, |s| s.at),
                sort: f.slot.as_ref().map_or(0, |s| s.sort),
            })
        }
        "slotat" => {
            let at = match text.to_ascii_lowercase().as_str() {
                "head" => End::Head,
                "tail" => End::Tail,
                _ => return Err(bad("head, tail")),
            };
            match &mut f.slot {
                Some(s) => s.at = at,
                None => return Err("Give the form a slot: first (the form it goes in)".into()),
            }
        }
        "slotsort" => {
            let sort: i32 = text
                .parse()
                .map_err(|_| format!("slotSort: \"{value}\" isn't a whole number"))?;
            match &mut f.slot {
                Some(s) => s.sort = sort,
                None => return Err("Give the form a slot: first (the form it goes in)".into()),
            }
        }
        "columns" => {
            f.columns = if text.is_empty() || text == "0" {
                None
            } else {
                Some(
                    text.parse()
                        .map_err(|_| format!("columns: \"{value}\" isn't a whole number"))?,
                )
            }
        }
        other => {
            return Err(format!(
                "No form attr \"{other}\" (label, style, icons, iconSize, workbench, exported, showWhen, slot, slotAt, slotSort, columns, group)"
            ));
        }
    }
    Ok(format!("{}: {attr} set", f.id))
}

/// A form's controls (inline sub-forms included) name or run `needle`
/// (lower case).
fn mentions(f: &Form, needle: &str) -> bool {
    f.controls.iter().any(|c| {
        control_name(c).to_lowercase().contains(needle)
            || c.command
                .as_deref()
                .is_some_and(|l| l.to_lowercase().contains(needle))
            || c.variants.all().any(|v| match v {
                cf::Variant::Shown { label, line, .. } => {
                    label.to_lowercase().contains(needle) || line.to_lowercase().contains(needle)
                }
                cf::Variant::Line(line) => line.to_lowercase().contains(needle),
            })
            || matches!(&c.form, Some(FormRef::Inline(inner)) if mentions(inner, needle))
    })
}

/// Every command line in a form, inline sub-forms included.
fn has_line(f: &Form, line: &str) -> Option<usize> {
    f.controls.iter().position(|c| {
        c.command.as_deref().is_some_and(|l| l.trim() == line)
            || matches!(&c.form, Some(FormRef::Inline(inner)) if has_line(inner, line).is_some())
    })
}

impl Operandi {
    fn groups_path(&self) -> PathBuf {
        self.environment
            .paths()
            .data_root()
            .join("Layouts")
            .join(GROUPS_FILE)
    }

    /// The user's groups, read once (at launch, or by the first command).
    pub(super) fn load_form_groups(&mut self) {
        if std::mem::replace(&mut self.form_editor.groups_loaded, true) {
            return;
        }
        self.form_editor.groups = std::fs::read_to_string(self.groups_path())
            .ok()
            .and_then(|t| serde_json::from_str::<GroupsFile>(&t).ok())
            .map(|g| g.groups)
            .unwrap_or_default();
    }

    fn save_form_groups(&self) -> std::result::Result<(), String> {
        let path = self.groups_path();
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&GroupsFile {
            version: 1,
            groups: self.form_editor.groups.clone(),
        })
        .map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    /// The group form `id` shows under.
    pub(super) fn form_group(&self, id: &str) -> String {
        self.form_editor
            .groups
            .iter()
            .find(|g| g.forms.iter().any(|f| f == id))
            .map_or_else(
                // Pies sit together whatever their ids (UIC-308).
                || match self.forms.get(id) {
                    Some(f) if f.style == Style::Pie => "Pies".to_owned(),
                    _ => default_group(id).to_owned(),
                },
                |g| g.name.clone(),
            )
    }

    /// The tree: each group and its form ids, filtered by the search.
    pub(super) fn form_tree(&self) -> Vec<(String, Vec<String>)> {
        let needle = self.form_editor.search.trim().to_lowercase();
        let matches = |id: &str| {
            if needle.is_empty() {
                return true;
            }
            let Some(f) = self.forms.get(id) else {
                return false;
            };
            id.to_lowercase().contains(&needle)
                || f.label.to_lowercase().contains(&needle)
                || mentions(f, &needle)
        };
        let mut names: Vec<String> = ["Toolbox", "Toolbars", "Menus", "Pies", "Forms"]
            .map(str::to_owned)
            .into_iter()
            .collect();
        for g in &self.form_editor.groups {
            if !names.contains(&g.name) {
                names.push(g.name.clone());
            }
        }
        names
            .into_iter()
            .filter_map(|name| {
                let ids: Vec<String> = self
                    .forms
                    .ids()
                    .into_iter()
                    .filter(|id| self.form_group(id) == name && matches(id))
                    .map(str::to_owned)
                    .collect();
                let user_group = self.form_editor.groups.iter().any(|g| g.name == name);
                (!ids.is_empty() || (user_group && needle.is_empty())).then_some((name, ids))
            })
            .collect()
    }

    /// The form a command acts on: `given`, else the selected one.
    fn editor_form(&self, given: Option<&str>) -> std::result::Result<String, String> {
        given
            .map(str::to_owned)
            .or_else(|| self.form_editor.selected.as_ref().map(|(f, _)| f.clone()))
            .ok_or_else(|| "Give form: (or select a form in the Form Editor)".to_owned())
    }

    /// `operandi.form.*` (layouts stage 7): one handler reads the command.
    pub(super) fn form_command(&mut self, name: &str, rest: &str) {
        let result = self.form_run(name, rest);
        match result {
            Ok(status) => self.status = status,
            Err(e) => {
                self.last_failure = Some(e.clone());
                self.status = e;
            }
        }
    }

    fn form_run(&mut self, name: &str, rest: &str) -> std::result::Result<String, String> {
        self.load_form_groups();
        match name {
            "operandi.form.newgroup" => {
                let a = bound(name, rest, &["name"])?;
                let group = a[0]
                    .clone()
                    .map(|n| n.trim().to_owned())
                    .filter(|n| !n.is_empty())
                    .ok_or("Give name: (the new group's name)")?;
                if self.form_tree().iter().any(|(g, _)| *g == group) {
                    return Err(format!("There is a group called {group} already"));
                }
                self.form_editor.groups.push(Group {
                    name: group.clone(),
                    forms: Vec::new(),
                });
                self.save_form_groups()?;
                Ok(format!("New group {group}"))
            }
            "operandi.form.new" => {
                let a = bound(name, rest, &["id", "label", "style", "group"])?;
                let id = a[0]
                    .clone()
                    .map(|n| n.trim().to_owned())
                    .filter(|n| !n.is_empty())
                    .ok_or("Give id: (the new form's id, e.g. my.tools)")?;
                if self.forms.get(&id).is_some() {
                    return Err(format!("There is a form called {id} already"));
                }
                let style = match a[2].as_deref() {
                    Some(s) => {
                        style_named(s).ok_or_else(|| format!("style: {s} isn't a form style"))?
                    }
                    None => Style::Vertical,
                };
                // UIC-308/309: a new pie starts with eight empty slots.
                let pie = style == Style::Pie;
                let form = Form {
                    version: cf::FORM_VERSION,
                    id: id.clone(),
                    label: a[1].clone().unwrap_or_else(|| id.clone()),
                    style,
                    icons: if pie {
                        crate::pie_forms::NEW_PIE_ICONS
                    } else {
                        IconMode::default()
                    },
                    icon_size: IconSize::default(),
                    exported: false,
                    show_when: None,
                    slot: None,
                    columns: None,
                    workbench: false,
                    controls: if pie {
                        crate::pie_forms::new_pie_controls()
                    } else {
                        Vec::new()
                    },
                };
                self.forms.save(form)?;
                if let Some(group) = a[3].clone() {
                    self.set_form_group(&id, &group)?;
                }
                self.form_editor.selected = Some((id.clone(), None));
                Ok(format!("New form {id}"))
            }
            "operandi.form.add" => {
                let a = bound(name, rest, &["line", "label", "form", "index"])?;
                let line = a[0]
                    .clone()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty())
                    .ok_or("Give line: (the command line the button runs)")?;
                let id = self.editor_form(a[2].as_deref())?;
                let at = a[3]
                    .as_deref()
                    .map(|i| index_arg(Some(i), "index"))
                    .transpose()?;
                let control = Control {
                    command: Some(line),
                    label: a[1].clone().unwrap_or_default(),
                    ..Default::default()
                };
                self.form_insert(&id, at, control)
            }
            "operandi.form.addform" => {
                let a = bound(name, rest, &["sub", "form", "index"])?;
                let sub = a[0]
                    .clone()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty())
                    .ok_or("Give sub: (the id of the form to place)")?;
                let id = self.editor_form(a[1].as_deref())?;
                if sub == id {
                    return Err("A form can't hold itself".into());
                }
                if self.forms.get(&sub).is_none() {
                    return Err(format!("No form called {sub}"));
                }
                let at = a[2]
                    .as_deref()
                    .map(|i| index_arg(Some(i), "index"))
                    .transpose()?;
                let control = Control {
                    form: Some(FormRef::Id(sub)),
                    ..Default::default()
                };
                self.form_insert(&id, at, control)
            }
            "operandi.form.adddivider" => {
                let a = bound(name, rest, &["label", "form"])?;
                let id = self.editor_form(a[1].as_deref())?;
                let control = Control {
                    divider: Some(a[0].clone().unwrap_or_default()),
                    ..Default::default()
                };
                self.form_insert(&id, None, control)
            }
            "operandi.form.move" => {
                let a = bound(name, rest, &["from", "to", "form"])?;
                let from = index_arg(a[0].as_deref(), "from")?;
                let to = index_arg(a[1].as_deref(), "to")?;
                let id = self.editor_form(a[2].as_deref())?;
                let status = self.form_edit(&id, |f| {
                    let n = f.controls.len();
                    if from >= n || to >= n {
                        return Err(format!(
                            "{id} has {n} controls (0 to {})",
                            n.saturating_sub(1)
                        ));
                    }
                    let c = f.controls.remove(from);
                    let name = control_name(&c);
                    f.controls.insert(to, c);
                    Ok(format!("Moved {name}"))
                })?;
                if let Some((f, Some(i))) = &mut self.form_editor.selected
                    && *f == id
                    && *i == from
                {
                    *i = to;
                }
                Ok(status)
            }
            "operandi.form.set" => {
                let a = bound(name, rest, &["attr", "value", "index", "form"])?;
                let attr = a[0].clone().ok_or("Give attr: (a property's name)")?;
                let value = a[1].clone().unwrap_or_default();
                let id = self.editor_form(a[3].as_deref())?;
                let index = a[2]
                    .as_deref()
                    .map(|i| index_arg(Some(i), "index"))
                    .transpose()?;
                match index {
                    None if attr.eq_ignore_ascii_case("group") => {
                        self.set_form_group(&id, &value)?;
                        Ok(format!("{id} is in {}", self.form_group(&id)))
                    }
                    None => self.form_edit(&id, |f| set_form_attr(f, &attr, &value)),
                    Some(index) => self.form_edit(&id, |f| {
                        let c = f
                            .controls
                            .get_mut(index)
                            .ok_or_else(|| format!("{id} has no control {index}"))?;
                        set_attr(c, &attr, &value)
                    }),
                }
            }
            "operandi.form.delete" => {
                let a = bound(name, rest, &["index", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let id = self.editor_form(a[1].as_deref())?;
                let status = self.form_edit(&id, |f| {
                    if index >= f.controls.len() {
                        return Err(format!("{id} has no control {index}"));
                    }
                    let c = f.controls.remove(index);
                    Ok(format!("Deleted {}", control_name(&c)))
                })?;
                if let Some((f, sel)) = &mut self.form_editor.selected
                    && *f == id
                {
                    *sel = None;
                }
                Ok(status)
            }
            "operandi.form.select" => {
                let a = bound(name, rest, &["form", "index"])?;
                let id = a[0]
                    .clone()
                    .ok_or("Give form: (the form to show in the Form Editor)")?;
                if self.forms.get(&id).is_none() {
                    return Err(format!("No form called {id}"));
                }
                let index = a[1]
                    .as_deref()
                    .map(|i| index_arg(Some(i), "index"))
                    .transpose()?;
                self.form_editor.selected = Some((id.clone(), index));
                self.form_editor.search.clear();
                self.set_palette_open(PALETTE, true)?;
                Ok(format!("Form Editor: {id}"))
            }
            "operandi.form.find" => {
                let a = bound(name, rest, &["on"])?;
                let on = on_arg(a[0].as_deref())?.unwrap_or(!self.form_editor.finding);
                self.form_editor.finding = on;
                Ok(if on {
                    "Find Form: click a button to show its form (Esc stops)".into()
                } else {
                    "Find Form is off".into()
                })
            }
            "operandi.form.preview" => {
                let a = bound(name, rest, &["on", "form"])?;
                if let Some(id) = a[1].clone() {
                    if self.forms.get(&id).is_none() {
                        return Err(format!("No form called {id}"));
                    }
                    self.form_editor.selected = Some((id, None));
                }
                if self.form_editor.selected.is_none() {
                    return Err("Select a form to preview".into());
                }
                let on = on_arg(a[0].as_deref())?.unwrap_or(!self.form_editor.preview);
                self.form_editor.preview = on;
                Ok(if on {
                    "Preview Form on"
                } else {
                    "Preview Form off"
                }
                .into())
            }
            "operandi.form.export" => {
                let a = bound(name, rest, &["form", "filename"])?;
                let id = self.editor_form(a[0].as_deref())?;
                let form = self
                    .forms
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| format!("No form called {id}"))?;
                let path = match a[1].clone().filter(|p| !p.trim().is_empty()) {
                    Some(p) => PathBuf::from(p.trim()),
                    // Provisional (LAY-Q67): the data folder's Saved Forms.
                    None => self
                        .environment
                        .paths()
                        .data_root()
                        .join("Saved Forms")
                        .join(cf::file_name(&id)),
                };
                if let Some(folder) = path.parent()
                    && !folder.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
                }
                // UIC-314: custom icon files go along, beside the form.
                let mut form = form;
                crate::form_icons::copy_icons_beside(&mut form, &path)?;
                std::fs::write(&path, form.to_json())
                    .map_err(|e| format!("Couldn't save {id}: {e}"))?;
                Ok(format!("Saved form {id} to {}", path.display()))
            }
            other => Err(format!("{other} isn't a form command")),
        }
    }

    /// Insert `control` into form `id` at `at` (the end without it).
    fn form_insert(
        &mut self,
        id: &str,
        at: Option<usize>,
        control: Control,
    ) -> std::result::Result<String, String> {
        let name = control_name(&control);
        let mut placed = 0;
        let status = self.form_edit(id, |f| {
            let at = at.unwrap_or(f.controls.len()).min(f.controls.len());
            f.controls.insert(at, control);
            placed = at;
            Ok(format!("Added {name} to {}", f.id))
        })?;
        self.form_editor.selected = Some((id.to_owned(), Some(placed)));
        Ok(status)
    }

    /// Put form `id` in group `name` (an empty name: its default group).
    fn set_form_group(&mut self, id: &str, name: &str) -> std::result::Result<(), String> {
        for g in &mut self.form_editor.groups {
            g.forms.retain(|f| f != id);
        }
        let name = name.trim();
        if !name.is_empty() && name != default_group(id) {
            match self.form_editor.groups.iter_mut().find(|g| g.name == name) {
                Some(g) => g.forms.push(id.to_owned()),
                None => self.form_editor.groups.push(Group {
                    name: name.to_owned(),
                    forms: vec![id.to_owned()],
                }),
            }
        }
        self.save_form_groups()
    }

    /// The registry's answers for the form commands (UIC-223).
    pub(super) fn form_state(&self, name: &str, _rest: &str) -> CommandState {
        match name {
            "operandi.form.find" => CommandState::lit(self.form_editor.finding),
            "operandi.form.preview" => {
                if self.form_editor.selected.is_none() {
                    CommandState::unless(Some("Select a form to preview"))
                } else {
                    CommandState::lit(self.form_editor.preview)
                }
            }
            _ => CommandState::default(),
        }
    }

    /// Find Form (LAY-83): a clicked button's line selects the form it is
    /// in instead of running. True when it was taken.
    pub(super) fn form_editor_take_find(&mut self, line: &str) -> bool {
        if !self.form_editor.finding || !self.workbench_clicked_now() {
            return false;
        }
        let line = line.trim();
        if line.to_ascii_lowercase().starts_with("operandi.form.") {
            return false;
        }
        // The toolbox tab showing first, then the other forms by id.
        let showing = format!("toolbox.{}", self.category.label().to_lowercase());
        let found = std::iter::once(showing.as_str())
            .chain(self.forms.ids())
            .find_map(|id| Some((id.to_owned(), has_line(self.forms.get(id)?, line)?)));
        self.form_editor.finding = false;
        match found {
            Some((id, index)) => {
                self.form_editor.selected = Some((id.clone(), Some(index)));
                self.form_editor.search.clear();
                let _ = self.set_palette_open(PALETTE, true);
                self.status = format!("Find Form: {id}");
            }
            None => self.status = format!("Find Form: no form runs {line}"),
        }
        true
    }

    /// Esc ends Find Form.
    pub(super) fn form_editor_frame_begin(&mut self, ctx: &Context) {
        if self.form_editor.finding
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.form_editor.finding = false;
            self.status = "Find Form is off".into();
        }
    }

    // ------------------------------------------------------ drawing

    /// The Form Editor's window contents.
    pub(super) fn form_editor_ui(&mut self, ui: &mut Ui) {
        self.load_form_groups();
        self.form_editor.rects.clear();
        let full = ui.available_rect_before_wrap();
        let left_width = (full.width() * 0.42).clamp(220., 420.);
        let left = Rect::from_min_size(full.min, vec2(left_width, full.height()));
        let right = Rect::from_min_max(pos2(left.right() + 8., full.top()), full.max);
        let mut l = ui.new_child(egui::UiBuilder::new().max_rect(left));
        self.form_editor_tree(&mut l);
        ui.painter().vline(
            left.right() + 4.,
            full.y_range(),
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
        let mut r = ui.new_child(egui::UiBuilder::new().max_rect(right));
        egui::ScrollArea::vertical()
            .id_salt("form-editor-properties")
            .show(&mut r, |ui| self.form_editor_properties(ui));
    }

    fn form_editor_tree(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.form_editor.search)
                    .hint_text("Search forms")
                    .desired_width(ui.available_width() - 80.),
            );
            crate::numeric_tab::register(ui, &edit, None);
            let finding = self.form_editor.finding;
            if ui
                .add(Button::new("Find Form").selected(finding))
                .on_hover_text("Click a button anywhere to show its form")
                .clicked()
            {
                self.execute("operandi.form.find");
            }
        });
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.form_editor.new_group)
                    .hint_text("Group name")
                    .desired_width(110.),
            );
            crate::numeric_tab::register(ui, &edit, None);
            if ui.button("New Group").clicked() {
                let name = std::mem::take(&mut self.form_editor.new_group);
                self.execute(&format!("operandi.form.newGroup name:{}", word(&name)));
            }
        });
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.form_editor.new_form)
                    .hint_text("Form id")
                    .desired_width(110.),
            );
            crate::numeric_tab::register(ui, &edit, None);
            if ui.button("New Form").clicked() {
                let id = std::mem::take(&mut self.form_editor.new_form);
                self.execute(&format!("operandi.form.new id:{}", word(&id)));
            }
            // UIC-308: a pie is a form with style pie, eight empty slots.
            if ui
                .button("New Pie")
                .on_hover_text("Makes a pie with this id: eight empty slots around its name")
                .clicked()
            {
                let id = std::mem::take(&mut self.form_editor.new_form);
                self.execute(&format!("operandi.form.new id:{} style:pie", word(&id)));
            }
            // UIC-312: a pie file with its sub-pies.
            if ui
                .button("Import Pie…")
                .on_hover_text("Reads a pie file written by Export Pie")
                .clicked()
            {
                self.execute("operandi.pie.import");
            }
        });
        ui.separator();
        let tree = self.form_tree();
        let searching = !self.form_editor.search.trim().is_empty();
        let mut drop: Option<(String, usize, usize)> = None;
        egui::ScrollArea::vertical()
            .id_salt("form-editor-tree")
            .show(ui, |ui| {
                for (group, ids) in tree {
                    egui::CollapsingHeader::new(RichText::new(&group).strong())
                        .id_salt(("form-group", &group))
                        .default_open(group != "Toolbox" || searching)
                        .open(searching.then_some(true))
                        .show(ui, |ui| {
                            for id in ids {
                                self.form_editor_tree_form(ui, &id, &mut drop);
                            }
                        });
                }
            });
        if let Some((form, from, to)) = drop
            && from != to
        {
            self.execute(&format!(
                "operandi.form.move from:{from} to:{to} form:{form}"
            ));
        }
        if ui.input(|i| !i.pointer.any_down()) {
            self.form_editor.drag = None;
        }
    }

    fn form_editor_tree_form(
        &mut self,
        ui: &mut Ui,
        id: &str,
        drop: &mut Option<(String, usize, usize)>,
    ) {
        let Some(form) = self.forms.get(id).cloned() else {
            return;
        };
        let selected = self.form_editor.selected.clone();
        let changed = if self.forms.is_changed(id) {
            " •"
        } else {
            ""
        };
        let open_id = ui.make_persistent_id(("form-node", id));
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            open_id,
            false,
        );
        // The selected form shows its controls.
        if selected.as_ref().is_some_and(|(f, _)| f == id) {
            state.set_open(true);
        }
        let header = state.show_header(ui, |ui| {
            let r = ui.selectable_label(
                selected
                    .as_ref()
                    .is_some_and(|(f, i)| f == id && i.is_none()),
                format!("{id}{changed}"),
            );
            if r.clicked() {
                self.form_editor.selected = Some((id.to_owned(), None));
            }
            r.rect
        });
        let (_, head_rect, _) = header.body(|ui| {
            for (i, c) in form.controls.iter().enumerate() {
                let kind = if c.form.is_some() {
                    "▸ "
                } else if c.divider.is_some() {
                    "— "
                } else {
                    ""
                };
                let on = selected.as_ref() == Some(&(id.to_owned(), Some(i)));
                let r = ui.add(
                    egui::Button::selectable(on, format!("{kind}{}", control_name(c)))
                        .sense(Sense::click_and_drag()),
                );
                self.form_editor.rects.insert(format!("{id}#{i}"), r.rect);
                if r.clicked() {
                    self.form_editor.selected = Some((id.to_owned(), Some(i)));
                }
                if r.drag_started() {
                    self.form_editor.drag = Some((id.to_owned(), i));
                }
                if let Some((f, from)) = &self.form_editor.drag
                    && f == id
                    && *from != i
                    && ui.input(|inp| {
                        inp.pointer
                            .interact_pos()
                            .is_some_and(|p| r.rect.contains(p))
                    })
                {
                    ui.painter().hline(
                        r.rect.x_range(),
                        if *from < i {
                            r.rect.bottom()
                        } else {
                            r.rect.top()
                        },
                        ui.visuals().selection.stroke,
                    );
                    if ui.input(|inp| inp.pointer.any_released()) {
                        *drop = Some((id.to_owned(), *from, i));
                    }
                }
            }
        });
        self.form_editor
            .rects
            .insert(id.to_owned(), head_rect.inner);
    }

    /// A text property: typed, set when Enter is pressed or it loses focus.
    fn form_text_field(
        &mut self,
        ui: &mut Ui,
        label: &str,
        form: &str,
        index: Option<usize>,
        attr: &str,
        current: &str,
    ) {
        let key = (form.to_owned(), index, attr.to_owned());
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(label));
            let mut text = self
                .form_editor
                .drafts
                .get(&key)
                .cloned()
                .unwrap_or_else(|| current.to_owned());
            let r =
                ui.add(egui::TextEdit::singleline(&mut text).desired_width(ui.available_width()));
            crate::numeric_tab::register(ui, &r, None);
            if r.changed() {
                self.form_editor.drafts.insert(key.clone(), text.clone());
            }
            if r.lost_focus() {
                self.form_editor.drafts.remove(&key);
                if text != current {
                    self.form_set(form, index, attr, &text);
                }
            }
        });
    }

    fn form_set(&mut self, form: &str, index: Option<usize>, attr: &str, value: &str) {
        let index = index.map(|i| format!(" index:{i}")).unwrap_or_default();
        self.execute(&format!(
            "operandi.form.set attr:{attr} value:{}{index} form:{form}",
            word(value)
        ));
    }

    // A property row's form, control and attr name plus its own look.
    #[allow(clippy::too_many_arguments)]
    fn form_choice(
        &mut self,
        ui: &mut Ui,
        label: &str,
        form: &str,
        index: Option<usize>,
        attr: &str,
        current: &str,
        choices: &[&str],
    ) {
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(label));
            let mut picked = None;
            egui::ComboBox::from_id_salt(("form-choice", form, index, attr))
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for c in choices {
                        if ui.selectable_label(*c == current, *c).clicked() {
                            picked = Some(*c);
                        }
                    }
                });
            if let Some(c) = picked
                && c != current
            {
                self.form_set(form, index, attr, c);
            }
        });
    }

    fn form_editor_properties(&mut self, ui: &mut Ui) {
        let Some((id, index)) = self.form_editor.selected.clone() else {
            ui.weak("Select a form or one of its controls.");
            return;
        };
        let Some(form) = self.forms.get(&id).cloned() else {
            self.form_editor.selected = None;
            return;
        };
        // A pie's slots are edited in its ring (form_editor_pie.rs).
        if form.style == Style::Pie {
            self.form_properties(ui, &form);
            return;
        }
        match index.and_then(|i| form.controls.get(i).cloned().map(|c| (i, c))) {
            Some((i, c)) => self.form_control_properties(ui, &id, i, &c),
            None => self.form_properties(ui, &form),
        }
    }

    fn form_control_properties(&mut self, ui: &mut Ui, id: &str, i: usize, c: &Control) {
        ui.label(RichText::new(format!("{id} › {}", control_name(c))).strong());
        ui.add_space(4.);
        let is_button = c.command.is_some();
        self.form_text_field(ui, "Label", id, Some(i), "label", &c.label);
        if is_button {
            self.form_text_field(
                ui,
                "Command",
                id,
                Some(i),
                "command",
                c.command.as_deref().unwrap_or_default(),
            );
            // Pick and History beside the line (UIC-212, UIC-160).
            self.command_sources(ui, id, i, |line| {
                format!(
                    "operandi.form.set attr:command value:{} index:{i} form:{id}",
                    word(line)
                )
            });
            self.form_text_field(ui, "Tooltip", id, Some(i), "tooltip", &c.tooltip);
            // UIC-314: built-in icon, custom file, or none.
            self.icon_picker(ui, id, i, c);
            self.form_text_field(
                ui,
                "Shortcut",
                id,
                Some(i),
                "shortcut",
                c.shortcut.as_deref().unwrap_or_default(),
            );
            let width = c.width.label();
            self.form_choice(
                ui,
                "Width",
                id,
                Some(i),
                "width",
                width,
                &["Normal", "Wide", "Full Width"],
            );
            let toggle = c.toggle.label();
            self.form_choice(
                ui,
                "Toggle",
                id,
                Some(i),
                "toggle",
                toggle,
                &["Default", "Checkbox", "Toggle Button"],
            );
            let popup = c.popup.label();
            self.form_choice(
                ui,
                "Pop-up",
                id,
                Some(i),
                "popup",
                popup,
                &["Default", "Pop-up", "Buttons"],
            );
            // Menus as forms: the macOS menu bar's own key ("s" is Cmd+S).
            self.form_text_field(
                ui,
                "Key Equivalent",
                id,
                Some(i),
                "keyEquivalent",
                c.key_equivalent.as_deref().unwrap_or_default(),
            );
        }
        if let Some(source) = &c.list {
            ui.label(format!(
                "Filled while the app runs from {}; move or delete it as one item.",
                crate::menu_forms::list_title(source)
            ));
        }
        if c.divider.is_some() {
            self.form_text_field(
                ui,
                "Show Only When",
                id,
                Some(i),
                "showWhen",
                c.show_when.as_deref().unwrap_or_default(),
            );
        }
        let level = crate::workbench::level_label(c.level);
        self.form_choice(
            ui,
            "Level",
            id,
            Some(i),
            "level",
            level,
            &["Basic", "Standard", "Advanced"],
        );
        if let Some(FormRef::Id(sub)) = &c.form
            && ui.button(format!("Open {sub}")).clicked()
        {
            self.form_editor.selected = Some((sub.clone(), None));
        }
        ui.add_space(6.);
        ui.horizontal(|ui| {
            if ui.button("Move Up").clicked() && i > 0 {
                self.execute(&format!(
                    "operandi.form.move from:{i} to:{} form:{id}",
                    i - 1
                ));
            }
            if ui.button("Move Down").clicked() {
                self.execute(&format!(
                    "operandi.form.move from:{i} to:{} form:{id}",
                    i + 1
                ));
            }
            if ui.button("Delete").clicked() {
                self.execute(&format!("operandi.form.delete index:{i} form:{id}"));
            }
        });
        if ui.button("‹ Form").clicked() {
            self.form_editor.selected = Some((id.to_owned(), None));
        }
    }

    fn form_properties(&mut self, ui: &mut Ui, form: &Form) {
        let id = form.id.clone();
        let shipped = self.forms.shipped(&id).is_some();
        let changed = self.forms.is_changed(&id);
        ui.label(RichText::new(&id).strong());
        ui.weak(match (shipped, changed) {
            (true, true) => "Shipped form with your changes",
            (true, false) => "Shipped form",
            (false, _) => "Your form",
        });
        ui.add_space(4.);
        self.form_text_field(ui, "Label", &id, None, "label", &form.label);
        let style = json_word(&form.style);
        self.form_choice(
            ui,
            "Style",
            &id,
            None,
            "style",
            &style,
            &[
                "vertical",
                "horizontal",
                "sheet",
                "menu",
                "flyout",
                "hold",
                "pie",
            ],
        );
        let icons = json_word(&form.icons);
        self.form_choice(
            ui,
            "Icons",
            &id,
            None,
            "icons",
            &icons,
            &["icon", "text", "both"],
        );
        let size = json_word(&form.icon_size);
        self.form_choice(
            ui,
            "Icon Size",
            &id,
            None,
            "iconSize",
            &size,
            &["small", "normal", "large"],
        );
        // LAY-Q53: the Customizable Workbench setting.
        let mut workbench = form.workbench;
        ui.horizontal(|ui| {
            ui.add_space(100.);
            if ui
                .checkbox(&mut workbench, "Customizable Workbench")
                .changed()
            {
                self.form_set(
                    &id,
                    None,
                    "workbench",
                    if workbench { "true" } else { "false" },
                );
            }
        });
        self.form_text_field(
            ui,
            "Show Only When",
            &id,
            None,
            "showWhen",
            form.show_when.as_deref().unwrap_or_default(),
        );
        // UIC-214: the head/tail slot.
        let slot = form.slot.clone();
        self.form_text_field(
            ui,
            "Slot In Form",
            &id,
            None,
            "slot",
            slot.as_ref().map(|s| s.form.as_str()).unwrap_or_default(),
        );
        if let Some(s) = &slot {
            let at = if s.at == End::Head { "head" } else { "tail" };
            self.form_choice(ui, "Slot At", &id, None, "slotAt", at, &["head", "tail"]);
            self.form_text_field(ui, "Slot Sort", &id, None, "slotSort", &s.sort.to_string());
        }
        self.form_text_field(
            ui,
            "Columns",
            &id,
            None,
            "columns",
            &form.columns.map(|c| c.to_string()).unwrap_or_default(),
        );
        let group = self.form_group(&id);
        let mut groups: Vec<String> = self.form_tree().into_iter().map(|(g, _)| g).collect();
        if !groups.contains(&group) {
            groups.push(group.clone());
        }
        let choices: Vec<&str> = groups.iter().map(String::as_str).collect();
        self.form_choice(ui, "Group", &id, None, "group", &group, &choices);
        ui.separator();
        if form.style == Style::Pie {
            self.pie_form_editor(ui, form);
        } else {
            self.form_add_rows(ui, &id);
        }
        ui.horizontal(|ui| {
            let undo = self.workbench_state("operandi.workbench.undo", &format!("form:{id}"));
            if ui
                .gated(undo.unavailable.as_deref(), Button::new("Undo Edit"))
                .clicked()
            {
                self.execute(&format!("operandi.workbench.undo form:{id}"));
            }
            let revert = if shipped { "Revert" } else { "Delete Form" };
            if ui
                .gated(
                    crate::unavailable::unless(changed, "This form has no changes"),
                    Button::new(revert),
                )
                .clicked()
            {
                self.execute(&format!("operandi.form.revert {id}"));
                if !shipped {
                    self.form_editor.selected = None;
                }
            }
        });
        if self.form_editor.preview && form.style != Style::Pie {
            ui.separator();
            ui.label(RichText::new("Preview").strong());
            let width = 260.;
            ui.allocate_ui(vec2(width, 10.), |ui| {
                ui.set_width(width);
                egui::Frame::new()
                    .fill(theme::chrome::background())
                    .inner_margin(4.)
                    .show(ui, |ui| {
                        ui.push_id(("form-preview", &id), |ui| self.draw_form(ui, &id));
                    });
            });
        }
    }

    /// The rows that add controls to a form (not a pie: its eight slots
    /// are edited in its ring).
    fn form_add_rows(&mut self, ui: &mut Ui, id: &str) {
        let id = id.to_owned();
        // Add a command: typed, or from the Command History (UIC-160).
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.form_editor.add_line)
                    .hint_text("Command line")
                    .desired_width(ui.available_width() - 150.),
            );
            crate::numeric_tab::register(ui, &edit, None);
            if ui.button("Add Command").clicked() {
                let line = std::mem::take(&mut self.form_editor.add_line);
                self.execute(&format!("operandi.form.add line:{} form:{id}", word(&line)));
            }
            let recent: Vec<String> = self
                .history
                .log
                .records
                .iter()
                .rev()
                .filter(|r| r.ok && !r.line.to_ascii_lowercase().starts_with("operandi.form."))
                .map(|r| r.line.clone())
                .fold(Vec::new(), |mut v, l| {
                    if !v.contains(&l) && v.len() < 20 {
                        v.push(l);
                    }
                    v
                });
            ui.menu_button("History ▾", |ui| {
                if recent.is_empty() {
                    ui.weak("No commands run yet");
                }
                for line in recent {
                    if ui.button(&line).clicked() {
                        self.form_editor.add_line = line;
                        ui.close();
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            let ids: Vec<String> = self
                .forms
                .ids()
                .into_iter()
                .filter(|f| *f != id)
                .map(str::to_owned)
                .collect();
            egui::ComboBox::from_id_salt(("form-add-sub", &id))
                .selected_text(if self.form_editor.add_sub.is_empty() {
                    "Sub-form…"
                } else {
                    self.form_editor.add_sub.as_str()
                })
                .width(ui.available_width() - 150.)
                .show_ui(ui, |ui| {
                    for f in ids {
                        if ui
                            .selectable_label(self.form_editor.add_sub == f, &f)
                            .clicked()
                        {
                            self.form_editor.add_sub = f;
                        }
                    }
                });
            if ui.button("Add Sub-form").clicked() && !self.form_editor.add_sub.is_empty() {
                let sub = std::mem::take(&mut self.form_editor.add_sub);
                self.execute(&format!("operandi.form.addForm sub:{sub} form:{id}"));
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Add Divider").clicked() {
                self.execute(&format!("operandi.form.addDivider form:{id}"));
            }
            let preview = self.form_editor.preview;
            if ui
                .add(Button::new("Preview Form").selected(preview))
                .clicked()
            {
                self.execute(&format!("operandi.form.preview form:{id}"));
            }
            if ui
                .button("Save Form")
                .on_hover_text("Saves the form to its own file")
                .clicked()
            {
                self.execute(&format!("operandi.form.export form:{id}"));
            }
        });
    }
}
