//! Workbench edit mode (layouts stage 4: LAY-82, LAY-86, LAY-87,
//! UIC-212). A workbench is a form the user edits where it sits: any form
//! whose file says `"workbench": true` (the "Customizable Workbench"
//! setting, a checkbox in System > Form Editor since stage 7). The shipped layouts
//! have two, both empty: the toolbox's Workbench tab
//! (`toolbox.workbench`) and a row along the top (`toolbar.workbench`,
//! hidden until shown, provisional LAY-Q47).
//!
//! - Edit Workbench shows the edit tools; the items then draw one per row
//!   (one chip each along the top), dragged to reorder, right-clicked for
//!   their properties (UIC-212). Labelled dividers can't fold while
//!   editing.
//! - Pick to Add: the next click on a toolbox or menu button (a line run
//!   through `execute`), a Tool Properties row, a Preset Browser entry or
//!   a Command History line is added instead of run. What is under the
//!   pointer lights green.
//! - Add the current tool chain: action centre, falloff, symmetry and the
//!   open tool as one `cmds.batch` button, a custom tool in one click.
//! - Every edit is an `operandi.workbench.*` registry command and writes
//!   the user's copy of the form (LAY-87, `FormStore::save`); Revert
//!   (`operandi.form.revert`) drops it. Workbench edits aren't scene undo
//!   steps: `operandi.workbench.undo` takes them back (provisional,
//!   LAY-Q48).
use super::*;
use crate::form::Proficiency;
use crate::form::command_form::{
    self as cf, Control, Form, IconMode, Item, Look, PopupFace, ToggleStyle, Width,
};
use crate::toolbox::{ButtonStyle, CommandButton, row_height};
use crate::unavailable::Gate;
use operandi_core::command_runtime::CommandState;
use std::collections::HashMap;

/// The toolbox's Workbench tab.
pub(super) const TOOLBOX_WORKBENCH: &str = "toolbox.workbench";
/// The workbench row along the top of the window.
pub(super) const TOP_WORKBENCH: &str = "toolbar.workbench";
/// A workbench row's units (Normal 1, Wide 2, Full Width 3).
const UNITS: usize = 3;
/// A button row's half slots: a labelled button takes two per unit, an
/// icon-only one at Normal or Wide one per unit, so six fit a row
/// (provisional LAY-Q54).
const SLOTS: usize = 6;
/// A chip's width along the top per unit.
const TOP_UNIT: f32 = 84.;
/// The top row's height: slim (provisional LAY-Q47).
const TOP_HEIGHT: f32 = 26.;

/// What Pick to Add lights under the pointer (LAY-82: "light green").
fn pick_green() -> Color32 {
    Color32::from_rgba_unmultiplied(70, 210, 100, 80)
}

/// Workbench state for this session.
#[derive(Default)]
pub(super) struct State {
    /// The form whose edit tools are showing (Edit Workbench), by id.
    pub(super) editing: Option<String>,
    /// Pick to Add is on.
    pub(super) picking: bool,
    /// A primary click landed this frame: Pick to Add takes only clicks,
    /// never a key's line.
    pub(crate) click_now: bool,
    /// Earlier versions of edited forms, newest last (`operandi.workbench.undo`).
    undo: Vec<Form>,
    /// While a property control draws: the one Tool Properties field to
    /// draw (`tool_form.rs`).
    pub(super) only_field: Option<String>,
    /// The workbench row along the top shows.
    pub(super) top_bar: bool,
    /// The Workbench tab is in the toolbox strip: from when it is first
    /// opened, like the UV tab, so the nine Model tabs keep their size
    /// until then (provisional LAY-Q47).
    pub(super) tab_shown: bool,
    /// Where each workbench control was drawn this frame: while editing
    /// by `<form>#<index>`, in use by `<form>:<line>` (a button) or
    /// `property:<tool>/<key>` (tests and the drag).
    pub(super) rects: BTreeMap<String, Rect>,
    /// Where each Tool Properties row was drawn, by `<tool>/<key>`.
    pub(super) property_rects: BTreeMap<String, Rect>,
    /// The control being dragged, by index, in the form being edited.
    drag: Option<usize>,
    /// Custom icon pictures, by path (`None`: it didn't load).
    icons: HashMap<String, Option<TextureHandle>>,
    /// The control a Custom Icon file panel is choosing for.
    pub(super) icon_target: Option<(String, usize)>,
    /// A right-click menu text field being typed in: set when it loses
    /// focus or the menu closes, dropped on Esc.
    draft: Option<Draft>,
}

/// A right-click menu text field's typed value, not yet set.
#[derive(Clone, Debug)]
struct Draft {
    /// The right-click menu it is in.
    menu: Id,
    form: String,
    index: usize,
    attr: String,
    text: String,
    /// What the field started from: an unchanged field sets nothing.
    current: String,
}

/// The name a button shows when its label is empty: its tool's name, else
/// its line (`toolbox_form::shown_label`); a property's row key; a
/// divider's text.
pub(super) fn default_label(c: &Control) -> String {
    if let Some(line) = &c.command {
        return tool_info::by_name(&line.trim().to_lowercase())
            .map_or_else(|| line.clone(), |t| t.label().to_owned());
    }
    if let Some(p) = &c.property {
        return p
            .split_once('/')
            .map_or(p.as_str(), |(_, key)| key)
            .to_owned();
    }
    c.divider.clone().unwrap_or_default()
}

/// What a right-click menu's Label field starts from: the label shown
/// now (empty for an icon-only button).
fn shown_label(c: &Control) -> String {
    if let Some(text) = &c.divider {
        return text.clone();
    }
    if c.no_label {
        return String::new();
    }
    if c.label.is_empty() {
        default_label(c)
    } else {
        c.label.clone()
    }
}

/// The half slots of a row's [`SLOTS`] a button takes: labelled, two per
/// unit; icon-only, one at Normal or Wide, the whole row at Full Width.
fn slots(c: &Control, icon_only: bool) -> usize {
    match (icon_only, c.width) {
        (_, Width::Full) => SLOTS,
        (true, w) => w.units(),
        (false, w) => 2 * w.units(),
    }
}

/// A right-click menu's text value as a command-line word: braces group
/// spaces and keep any quotes in it (`command_line::tokens`).
pub(super) fn word(value: &str) -> String {
    if value.is_empty() {
        "\"\"".to_owned()
    } else {
        format!("{{{value}}}")
    }
}

pub(super) fn level_label(level: Proficiency) -> &'static str {
    match level {
        Proficiency::Basic => "Basic",
        Proficiency::Standard => "Standard",
        Proficiency::Advanced => "Advanced",
    }
}

pub(super) fn level_named(name: &str) -> Option<Proficiency> {
    [
        Proficiency::Basic,
        Proficiency::Standard,
        Proficiency::Advanced,
    ]
    .into_iter()
    .find(|l| level_label(*l).eq_ignore_ascii_case(name.trim()))
}

/// The words of `rest` bound to `names` (`command_args::bind`), owned.
pub(super) fn bound(
    command: &str,
    rest: &str,
    names: &[&str],
) -> std::result::Result<Vec<Option<String>>, String> {
    let words = operandi_core::command_line::tokens(rest).map_err(str::to_owned)?;
    let bound = operandi_core::command_args::bind(command, &words, names)?;
    Ok(bound.into_iter().map(|w| w.map(str::to_owned)).collect())
}

pub(super) fn index_arg(value: Option<&str>, what: &str) -> std::result::Result<usize, String> {
    value
        .ok_or_else(|| format!("Give {what}: (a control's place, from 0)"))?
        .trim()
        .parse()
        .map_err(|_| format!("{what}: must be a whole number from 0"))
}

pub(super) fn on_arg(value: Option<&str>) -> std::result::Result<Option<bool>, String> {
    value
        .map(|v| operandi_core::command_args::boolean("on", v))
        .transpose()
}

/// The workbench's own lines and Revert: they run even while picking.
fn own_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.starts_with("operandi.workbench.") || lower.starts_with("operandi.form.revert")
}

/// What a control is called in an edit row or a status line.
pub(super) fn control_name(c: &Control) -> String {
    if !c.label.is_empty() {
        return c.label.clone();
    }
    if let Some(line) = &c.command {
        return line.clone();
    }
    if let Some(p) = &c.property {
        return p.replace('/', " · ");
    }
    // Menus as forms: a live list is one item, named by its source.
    if let Some(source) = &c.list {
        return format!("{} (live)", crate::menu_forms::list_title(source));
    }
    match &c.divider {
        Some(label) if !label.is_empty() => label.clone(),
        Some(_) => "Divider".into(),
        None => "Control".into(),
    }
}

impl Operandi {
    // ------------------------------------------------------ frame hooks

    /// Start of a frame: note a click for Pick to Add; Esc ends picking
    /// (and doesn't also cancel the tool).
    pub(super) fn workbench_frame_begin(&mut self, ctx: &Context) {
        self.workbench.click_now = ctx.input(|i| i.pointer.primary_clicked());
        self.workbench.rects.clear();
        // The Form Editor's Pick (UIC-212, UIC-314) stops the same way.
        self.form_pick_frame_begin(ctx);
        if self.workbench.picking && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.workbench.picking = false;
            self.status = "Pick to Add is off".into();
        }
    }

    /// End of a frame: while picking, the clickable thing under the
    /// pointer lights green (LAY-82).
    pub(super) fn workbench_frame_end(&mut self, ctx: &Context) {
        // A click away closed the menu while a field was being typed in:
        // the typed value is set (Esc already dropped it).
        if let Some(draft) = &self.workbench.draft
            && !egui::Popup::is_id_open(ctx, draft.menu)
        {
            let draft = draft.clone();
            self.workbench.draft = None;
            self.workbench_set_draft(draft);
        }
        if !self.workbench.picking && !self.form_picking() {
            return;
        }
        // Picking shows the system's pointing hand (Jason, 2026-11-27).
        ctx.set_cursor_icon(CursorIcon::PointingHand);
        let hovered = ctx.viewport(|v| v.interact_widgets.hovered.clone());
        let under = hovered
            .iter()
            .filter_map(|id| ctx.read_response(*id))
            .filter(|r| r.sense.senses_click())
            .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
        if let Some(r) = under {
            ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("workbench-pick")))
                .rect_filled(r.rect, 2., pick_green());
        }
    }

    /// Pick to Add (LAY-82): a clicked button's line is added to the
    /// workbench being edited instead of running. True when it was taken.
    /// The workbench's own commands and Revert still run.
    /// A primary click landed this frame (Pick to Add and Find Form take
    /// only clicks).
    pub(super) fn workbench_clicked_now(&self) -> bool {
        self.workbench.click_now
    }

    pub(super) fn workbench_take_pick(&mut self, line: &str) -> bool {
        // Find Form takes a clicked button to the Form Editor (LAY-83).
        if self.form_editor_take_find(line) {
            return true;
        }
        // The Form Editor's Pick fills a control from it (UIC-314).
        if self.form_pick_take(line) {
            return true;
        }
        if !self.workbench.picking || !self.workbench.click_now {
            return false;
        }
        let line = line.trim();
        if own_line(line) {
            return false;
        }
        // One click adds one line, even if the button runs several.
        self.workbench.click_now = false;
        let result = self.workbench_add_picked(line);
        self.workbench_report(result);
        true
    }

    /// A Command History line clicked while picking: added (LAY-82).
    pub(super) fn workbench_pick_history(&mut self, line: &str) -> bool {
        if !self.workbench.picking || self.workbench.editing.is_none() || own_line(line) {
            return false;
        }
        let result = self.workbench_add_picked(line.trim());
        self.workbench_report(result);
        true
    }

    /// A Preset Browser entry clicked while picking: a button loading it
    /// (`preset.do`) is added (LAY-82).
    pub(super) fn workbench_pick_preset(&mut self, name: &str, path: &std::path::Path) -> bool {
        if !self.workbench.picking || self.workbench.editing.is_none() {
            return false;
        }
        let control = Control {
            command: Some(format!("preset.do preset:\"{}\"", path.display())),
            label: name.to_owned(),
            width: Width::Full,
            ..Default::default()
        };
        let result = self.workbench_push(None, control);
        self.workbench_report(result);
        true
    }

    /// A Tool Properties row drawn at `rect` (`tool_form.rs`): noted for
    /// Pick to Add, which takes a click on it instead of the field. True
    /// when the click was taken.
    pub(super) fn workbench_pick_property(
        &mut self,
        ui: &mut Ui,
        key: &str,
        label: &str,
        rect: Rect,
    ) -> bool {
        if self.workbench.only_field.is_some() {
            return false;
        }
        let id = format!("{}/{key}", self.tool_properties_title());
        self.workbench.property_rects.insert(id.clone(), rect);
        if !self.workbench.picking || self.workbench.editing.is_none() {
            return false;
        }
        let catch = ui.interact(
            rect,
            Id::new(("workbench-pick-property", &id)),
            Sense::click(),
        );
        if catch.hovered() {
            ui.painter().rect_filled(rect, 2., pick_green());
        }
        if !catch.clicked() {
            return false;
        }
        let result = self.workbench_push(
            None,
            Control {
                property: Some(id),
                label: label.to_owned(),
                width: Width::Full,
                ..Default::default()
            },
        );
        self.workbench_report(result);
        true
    }

    /// A picked line as a button, wearing the label, icon and tooltip of
    /// the first form button that runs it, if any (so a picked toolbox
    /// button looks as it did).
    fn workbench_add_picked(&mut self, line: &str) -> std::result::Result<String, String> {
        let mut control = self
            .forms
            .ids()
            .into_iter()
            .filter_map(|id| self.forms.get(id))
            .filter(|f| !f.workbench)
            .flat_map(|f| f.controls.iter())
            .find(|c| c.command.as_deref() == Some(line))
            .cloned()
            .unwrap_or_default();
        control.command = Some(line.to_owned());
        control.width = Width::Full;
        self.workbench_push(None, control)
    }

    fn workbench_report(&mut self, result: std::result::Result<String, String>) {
        match result {
            Ok(status) => self.status = status,
            Err(e) => {
                self.last_failure = Some(e.clone());
                self.status = e;
            }
        }
    }

    // ------------------------------------------------------ edits

    /// The workbench a command acts on: `given`, else the one being
    /// edited, else the toolbox's.
    fn workbench_id(&self, given: Option<&str>) -> String {
        given
            .map(str::to_owned)
            .or_else(|| self.workbench.editing.clone())
            .unwrap_or_else(|| TOOLBOX_WORKBENCH.to_owned())
    }

    fn workbench_form(&self, id: &str) -> std::result::Result<Form, String> {
        let form = self
            .forms
            .get(id)
            .ok_or_else(|| format!("No form called {id}"))?;
        if !form.workbench {
            return Err(format!(
                "Form {id} isn't a workbench: tick Customizable Workbench in System > Form Editor"
            ));
        }
        Ok(form.clone())
    }

    /// Changes workbench `id` with `change` and saves the user's copy
    /// (LAY-87); the old version is kept for `operandi.workbench.undo`.
    fn workbench_edit(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut Form) -> std::result::Result<String, String>,
    ) -> std::result::Result<String, String> {
        self.workbench_form(id)?;
        self.form_edit(id, change)
    }

    /// Changes any form `id` with `change` and saves the user's copy
    /// (LAY-87); the old version is kept for `operandi.workbench.undo`
    /// (the Form Editor's edits too, layouts stage 7).
    pub(super) fn form_edit(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut Form) -> std::result::Result<String, String>,
    ) -> std::result::Result<String, String> {
        let before = self
            .forms
            .get(id)
            .cloned()
            .ok_or_else(|| format!("No form called {id}"))?;
        let mut form = before.clone();
        let status = change(&mut form)?;
        if form == before {
            return Ok(status);
        }
        self.forms.save(form)?;
        self.workbench.undo.push(before);
        Ok(status)
    }

    /// Adds `control` at the end of workbench `form` (default as
    /// [`Self::workbench_id`]).
    fn workbench_push(
        &mut self,
        form: Option<&str>,
        control: Control,
    ) -> std::result::Result<String, String> {
        let id = self.workbench_id(form);
        let name = control_name(&control);
        self.workbench_edit(&id, |f| {
            f.controls.push(control);
            Ok(format!("Added {name} to {}", f.label))
        })
    }

    /// The current tool chain as command lines, in pipe order: action
    /// centre, falloff, symmetry, then the open tool (LAY-82).
    pub(super) fn workbench_pipe(&self) -> std::result::Result<(Tool, Vec<String>), String> {
        let live = self
            .editor
            .session.live
            .as_ref()
            .ok_or("Open a tool first: the preset re-runs the open tool with its action centre, falloff and symmetry")?;
        let tool = live.tool;
        let mut lines = Vec::new();
        if let Some(name) = crate::pie_menu::center_name(&self.editor.action_context) {
            lines.push(format!("action center {name}"));
        }
        match &self.editor.spatial_falloff {
            None => lines.push("Falloff none".into()),
            Some(Falloff::Linear(_)) => lines.push("Falloff linear".into()),
            Some(Falloff::Radial(_)) => lines.push("Falloff radial".into()),
            // Weight-map, Invert and Selection falloffs have no line that
            // puts them back yet (provisional, LAY-Q51).
            Some(_) => {}
        }
        lines.push(match self.editor.symmetry.axis {
            Some(0) => "Symmetry x".into(),
            Some(1) => "Symmetry y".into(),
            Some(2) => "Symmetry z".into(),
            _ => "Symmetry off".into(),
        });
        let line = tool_info::info(tool)
            .names
            .iter()
            .copied()
            .find(|n| self.line_state(n).is_some())
            .unwrap_or(tool.label());
        lines.push(line.to_owned());
        Ok((tool, lines))
    }

    fn workbench_add_pipe(
        &mut self,
        kind: &str,
        label: Option<String>,
        form: Option<&str>,
    ) -> std::result::Result<String, String> {
        match kind {
            "macro" => return Err("Recorded macros aren't in Operandi yet".into()),
            "script" => return Err("Scripts aren't in Operandi yet".into()),
            _ => {}
        }
        let (tool, lines) = self.workbench_pipe()?;
        let label = label.unwrap_or_else(|| format!("{} Preset", tool.label()));
        let mut batch = format!("cmds.batch label:{}", word(&label));
        for (i, line) in lines.iter().enumerate() {
            batch += &format!(" c{}:{}", i + 1, word(line));
        }
        let control = Control {
            command: Some(batch),
            label,
            tooltip: format!("Custom tool: {}", lines.join(" → ")),
            icon: Some(tool_info::info(tool).icon.slug().to_owned()),
            width: Width::Full,
            ..Default::default()
        };
        self.workbench_push(form, control)
    }

    /// `operandi.workbench.*` (registry, `command_handlers.rs`).
    pub(super) fn workbench_command(&mut self, name: &str, rest: &str) {
        let result = self.workbench_run(name, rest);
        self.workbench_report(result);
    }

    fn workbench_run(&mut self, name: &str, rest: &str) -> std::result::Result<String, String> {
        match name {
            "operandi.workbench.edit" => {
                let a = bound(name, rest, &["on", "form"])?;
                let id = a[1].clone().unwrap_or_else(|| {
                    self.workbench
                        .editing
                        .clone()
                        .unwrap_or_else(|| TOOLBOX_WORKBENCH.to_owned())
                });
                let editing = self.workbench.editing.as_deref() == Some(id.as_str());
                let on = on_arg(a[0].as_deref())?.unwrap_or(!editing);
                if !on {
                    self.workbench.editing = None;
                    self.workbench.picking = false;
                    self.workbench.drag = None;
                    return Ok("Workbench editing is off".into());
                }
                let form = self.workbench_form(&id)?;
                if !self.confirm_fields() {
                    return Err("Finish the field being typed in first".into());
                }
                self.workbench.editing = Some(id.clone());
                if id == TOOLBOX_WORKBENCH {
                    self.show_toolbox_tab(toolbox::Category::Workbench);
                    self.workbench.tab_shown = true;
                } else if id == TOP_WORKBENCH {
                    self.workbench.top_bar = true;
                }
                Ok(format!(
                    "Editing {}: Pick to Add, drag to reorder, right-click an item for its properties",
                    form.label
                ))
            }
            "operandi.workbench.pick" => {
                let a = bound(name, rest, &["on"])?;
                if self.workbench.editing.is_none() {
                    return Err("Edit Workbench first".into());
                }
                let on = on_arg(a[0].as_deref())?.unwrap_or(!self.workbench.picking);
                self.workbench.picking = on;
                Ok(if on {
                    "Pick to Add: click a button, a Tool Properties row, a preset or a Command History line (Esc stops)".into()
                } else {
                    "Pick to Add is off".into()
                })
            }
            "operandi.workbench.add" => {
                let a = bound(name, rest, &["line", "label", "form"])?;
                let line = a[0]
                    .clone()
                    .filter(|l| !l.trim().is_empty())
                    .ok_or("Give line: (the command line the button runs)")?;
                let control = Control {
                    command: Some(line.trim().to_owned()),
                    label: a[1].clone().unwrap_or_default(),
                    width: Width::Full,
                    ..Default::default()
                };
                self.workbench_push(a[2].as_deref(), control)
            }
            "operandi.workbench.addProperty" => {
                let a = bound(name, rest, &["property", "label", "form"])?;
                let property = a[0]
                    .clone()
                    .filter(|p| {
                        p.split_once('/')
                            .is_some_and(|(t, k)| !t.is_empty() && !k.is_empty())
                    })
                    .ok_or("Give property: as <tool>/<key>, e.g. Bevel/inset")?;
                let label = a[1].clone().unwrap_or_else(|| {
                    property
                        .split_once('/')
                        .map(|(_, k)| k.to_owned())
                        .unwrap_or_default()
                });
                let control = Control {
                    property: Some(property),
                    label,
                    width: Width::Full,
                    ..Default::default()
                };
                self.workbench_push(a[2].as_deref(), control)
            }
            "operandi.workbench.addDivider" => {
                let a = bound(name, rest, &["label", "form"])?;
                let control = Control {
                    divider: Some(a[0].clone().unwrap_or_default()),
                    ..Default::default()
                };
                self.workbench_push(a[1].as_deref(), control)
            }
            "operandi.workbench.addChain" => {
                let a = bound(name, rest, &["as", "label", "form"])?;
                let kind = a[0]
                    .clone()
                    .unwrap_or_else(|| "preset".into())
                    .to_ascii_lowercase();
                self.workbench_add_pipe(&kind, a[1].clone(), a[2].as_deref())
            }
            "operandi.workbench.move" => {
                let a = bound(name, rest, &["from", "to", "form"])?;
                let from = index_arg(a[0].as_deref(), "from")?;
                let to = index_arg(a[1].as_deref(), "to")?;
                let id = self.workbench_id(a[2].as_deref());
                self.workbench_edit(&id, |f| {
                    let n = f.controls.len();
                    if from >= n || to >= n {
                        return Err(format!(
                            "{} has {n} controls (0 to {})",
                            f.label,
                            n.saturating_sub(1)
                        ));
                    }
                    let c = f.controls.remove(from);
                    let name = control_name(&c);
                    f.controls.insert(to, c);
                    Ok(format!("Moved {name}"))
                })
            }
            "operandi.workbench.set" => {
                let a = bound(name, rest, &["index", "attr", "value", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let attr = a[1].clone().ok_or(
                    "Give attr: (label, tooltip, popup, width, level, command, toggle or iconFile)",
                )?;
                let value = a[2].clone().unwrap_or_default();
                let id = self.workbench_id(a[3].as_deref());
                self.workbench_edit(&id, |f| {
                    let c = f
                        .controls
                        .get_mut(index)
                        .ok_or_else(|| format!("{} has no control {index}", f.label))?;
                    set_attr(c, &attr, &value)
                })
            }
            "operandi.workbench.chooseIcon" => {
                let a = bound(name, rest, &["index", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let id = self.workbench_id(a[1].as_deref());
                let form = self.workbench_form(&id)?;
                if index >= form.controls.len() {
                    return Err(format!("{} has no control {index}", form.label));
                }
                self.workbench.icon_target = Some((id, index));
                let folder = crate::backdrop_ui::dirs_home().unwrap_or_default();
                if !self.choose(
                    file_chooser::Choose::Image { folder },
                    file_chooser::Purpose::WorkbenchIcon,
                ) {
                    self.workbench.icon_target = None;
                    return Err("No file panel is available to choose a picture".into());
                }
                // A panel that answered at once already set it.
                Ok(if self.workbench.icon_target.is_some() {
                    "Choose a PNG or JPEG for the icon".into()
                } else {
                    self.status.clone()
                })
            }
            "operandi.workbench.duplicate" => {
                let a = bound(name, rest, &["index", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let id = self.workbench_id(a[1].as_deref());
                self.workbench_edit(&id, |f| {
                    let c = f
                        .controls
                        .get(index)
                        .cloned()
                        .ok_or_else(|| format!("{} has no control {index}", f.label))?;
                    let name = control_name(&c);
                    f.controls.insert(index + 1, c);
                    Ok(format!("Duplicated {name}"))
                })
            }
            "operandi.workbench.delete" => {
                let a = bound(name, rest, &["index", "form"])?;
                let index = index_arg(a[0].as_deref(), "index")?;
                let id = self.workbench_id(a[1].as_deref());
                self.workbench_edit(&id, |f| {
                    if index >= f.controls.len() {
                        return Err(format!("{} has no control {index}", f.label));
                    }
                    let c = f.controls.remove(index);
                    Ok(format!("Deleted {}", control_name(&c)))
                })
            }
            "operandi.workbench.undo" => {
                let a = bound(name, rest, &["form"])?;
                let id = self.workbench_id(a[0].as_deref());
                let at = self
                    .workbench
                    .undo
                    .iter()
                    .rposition(|f| f.id == id)
                    .ok_or_else(|| format!("No workbench edit of {id} to undo"))?;
                let form = self.workbench.undo.remove(at);
                let label = form.label.clone();
                self.forms.save(form)?;
                Ok(format!("Undid the last edit of {label}"))
            }
            "operandi.workbench.topBar" => {
                let a = bound(name, rest, &["on"])?;
                let on = on_arg(a[0].as_deref())?.unwrap_or(!self.workbench.top_bar);
                self.workbench.top_bar = on;
                if !on && self.workbench.editing.as_deref() == Some(TOP_WORKBENCH) {
                    self.workbench.editing = None;
                    self.workbench.picking = false;
                }
                Ok(if on {
                    "Top workbench shown".into()
                } else {
                    "Top workbench hidden".into()
                })
            }
            other => Err(format!("{other} isn't a workbench command")),
        }
    }

    /// Revert (LAY-87) also forgets that form's workbench undo.
    pub(super) fn workbench_reverted(&mut self, id: &str) {
        self.workbench.undo.retain(|f| f.id != id);
    }

    /// The picture a Custom Icon file panel chose.
    pub(super) fn workbench_icon_chosen(&mut self, path: &std::path::Path) {
        let Some((form, index)) = self.workbench.icon_target.take() else {
            return;
        };
        // Straight to the handler: a panel without a native sheet answers
        // inside the chooseIcon line itself.
        let rest = format!(
            "index:{index} attr:iconFile value:{} form:{form}",
            word(&path.display().to_string())
        );
        self.workbench_command("operandi.workbench.set", &rest);
    }

    /// The registry's answers for the workbench commands (UIC-223).
    pub(super) fn workbench_state(&self, name: &str, rest: &str) -> CommandState {
        use operandi_core::command_runtime::CommandState as S;
        let a = bound(name, rest, &["on", "form"]).unwrap_or_default();
        match name {
            "operandi.workbench.edit" => {
                let id = a.get(1).cloned().flatten().unwrap_or_else(|| {
                    self.workbench
                        .editing
                        .clone()
                        .unwrap_or_else(|| TOOLBOX_WORKBENCH.to_owned())
                });
                match self.workbench_form(&id) {
                    Ok(_) => S::lit(self.workbench.editing.as_deref() == Some(id.as_str())),
                    Err(e) => S::unless(Some(&e)),
                }
            }
            "operandi.workbench.pick" => {
                if self.workbench.editing.is_none() {
                    S::unless(Some("Edit Workbench first"))
                } else {
                    S::lit(self.workbench.picking)
                }
            }
            "operandi.workbench.addChain" => {
                let kind = bound(name, rest, &["as", "label", "form"])
                    .ok()
                    .and_then(|a| a[0].clone())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                match kind.as_str() {
                    "macro" => S::unless(Some("Recorded macros aren't in Operandi yet")),
                    "script" => S::unless(Some("Scripts aren't in Operandi yet")),
                    _ => S::unless(self.workbench_pipe().err().as_deref()),
                }
            }
            "operandi.workbench.undo" => {
                let given = bound(name, rest, &["form"]).ok().and_then(|a| a[0].clone());
                let id = self.workbench_id(given.as_deref());
                let any = self.workbench.undo.iter().any(|f| f.id == id);
                S::unless((!any).then_some("No workbench edit to undo"))
            }
            "operandi.workbench.topBar" => S::lit(self.workbench.top_bar),
            _ => S::default(),
        }
    }

    // ------------------------------------------------------ drawing

    /// The toolbox draws workbench `id` (a form with `workbench` set):
    /// its Edit Workbench button and tools, then its items.
    pub(super) fn draw_workbench(&mut self, ui: &mut Ui, id: &str) {
        let editing = self.workbench.editing.as_deref() == Some(id);
        self.workbench_tools(ui, id, editing, false);
        if editing {
            self.workbench_edit_items(ui, id, false);
        } else {
            self.workbench_live_items(ui, id);
        }
    }

    /// A small button running `line`, lit and greyed from its answer.
    fn workbench_tool(&mut self, ui: &mut Ui, label: &str, line: &str, tip: &str) {
        let state = self.line_state(line).unwrap_or_default();
        let response = ui.gated(
            state.unavailable.as_deref(),
            Button::new(label).selected(state.on),
        );
        self.toolbox_rects.insert(label.to_owned(), response.rect);
        if response.clicked() {
            self.execute(line);
        }
        response.on_hover_text(tip);
    }

    /// The Edit Workbench button, and while editing the edit tools
    /// (UIC-212).
    fn workbench_tools(&mut self, ui: &mut Ui, id: &str, editing: bool, top: bool) {
        let edit_line = format!("operandi.workbench.edit form:{id}");
        let edit_label = if editing {
            "Done"
        } else if top {
            "Edit"
        } else {
            "Edit Workbench"
        };
        let edit_tip = if editing {
            "Stop editing this workbench"
        } else {
            "Edit Workbench: Pick to Add, drag to reorder, right-click an item for its properties (UIC-212)"
        };
        if !editing {
            self.workbench_tool(ui, edit_label, &edit_line, edit_tip);
            return;
        }
        let count = self.forms.get(id).map_or(0, |f| f.controls.len());
        let tools = |ui: &mut Ui, a: &mut Operandi| {
            a.workbench_tool(ui, edit_label, &edit_line, edit_tip);
            a.workbench_tool(
                ui,
                "Pick to Add",
                "operandi.workbench.pick",
                "Click a button, a Tool Properties row, a preset or a Command History line to add it (Esc stops)",
            );
            a.workbench_tool(
                ui,
                "Add Divider",
                &format!("operandi.workbench.addDivider form:{id}"),
                "A plain divider line",
            );
            a.workbench_tool(
                ui,
                "Add Divider with Label",
                &format!(
                    "operandi.workbench.addDivider label:{} form:{id}",
                    word(&format!("Section {}", count + 1))
                ),
                "Starts a collapsible section; right-click it to rename it",
            );
            let pipe = format!("operandi.workbench.addChain as:preset form:{id}");
            let state = a.line_state(&pipe).unwrap_or_default();
            let menu = ui.menu_button("Add Tool Chain", |ui| {
                a.menu_owned_input = true;
                for (label, kind, tip) in [
                    ("As Preset", "preset", "A custom tool in one click: the action centre, falloff, symmetry and open tool"),
                    ("As Macro", "macro", "Not in Operandi yet: there is no macro recorder"),
                    ("As Script", "script", "Not in Operandi yet"),
                ] {
                    let line = format!("operandi.workbench.addChain as:{kind} form:{id}");
                    let state = a.line_state(&line).unwrap_or_default();
                    let item = ui.gated(state.unavailable.as_deref(), Button::new(label));
                    a.toolbox_rects.insert(format!("Add Tool Chain {label}"), item.rect);
                    if item.clicked() {
                        a.execute(&line);
                        ui.close();
                    }
                    item.on_hover_text(tip);
                }
            });
            a.toolbox_rects
                .insert("Add Tool Chain".into(), menu.response.rect);
            menu.response.on_hover_text(
                state
                    .unavailable
                    .unwrap_or_else(|| "Add the current tool chain as a preset".into()),
            );
            a.workbench_tool(
                ui,
                "Undo Edit",
                &format!("operandi.workbench.undo form:{id}"),
                "Take back the last workbench edit (not a scene undo step)",
            );
            a.workbench_tool(
                ui,
                "Revert",
                &format!("operandi.form.revert {id}"),
                "Drop your changes: the shipped workbench shows again (LAY-87)",
            );
        };
        if top {
            tools(ui, self);
        } else {
            ui.horizontal_wrapped(|ui| tools(ui, self));
        }
    }

    /// The items as the user uses them: buttons in rows of three units
    /// (Normal 1, Wide 2, Full Width 3), labelled dividers as sections.
    fn workbench_live_items(&mut self, ui: &mut Ui, id: &str) {
        let items = cf::plan(&self.forms, id, self.form_level(), &|l| {
            self.line_enabled(l)
        });
        if items.is_empty() {
            ui.weak("An empty workbench: Edit Workbench, then Pick to Add.");
            return;
        }
        let mut row: Vec<(Control, usize)> = Vec::new();
        let mut skip = 0usize;
        for item in items {
            if skip > 0 {
                match item {
                    Item::Section { .. } => skip += 1,
                    Item::SectionEnd => skip -= 1,
                    _ => {}
                }
                continue;
            }
            if let Item::Button(control, _) = item {
                let icon_only = self.workbench_icon_only(ui.ctx(), &control);
                let take = slots(&control, icon_only);
                let used: usize = row.iter().map(|(_, s)| s).sum();
                if used + take > SLOTS {
                    self.workbench_row(ui, id, std::mem::take(&mut row));
                }
                row.push((*control, take));
                continue;
            }
            self.workbench_row(ui, id, std::mem::take(&mut row));
            match item {
                Item::Section { key, label } => {
                    let mut open = !self.create_folded.contains(&key);
                    let header = theme::chrome::disclosure(ui, &key, &label, open);
                    self.toolbox_rects.insert(label.clone(), header.rect);
                    if header.clicked() && self.confirm_fields() {
                        open = !open;
                    }
                    if open {
                        self.create_folded.remove(&key);
                    } else {
                        self.create_folded.insert(key);
                        skip = 1;
                    }
                }
                Item::Separator => {
                    ui.separator();
                }
                // A property row takes its alignment's share of the width,
                // on a row of its own (it draws a whole Tool Properties row).
                Item::Property(control) => {
                    let units = control.width.units() as f32;
                    let unit = (ui.available_width() - 2. * (UNITS as f32 - 1.)) / UNITS as f32;
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(unit * units + 2. * (units - 1.));
                            self.workbench_property(ui, &control);
                        });
                    });
                }
                Item::Heading { label, .. } => {
                    ui.label(label);
                }
                Item::Space(points) => {
                    ui.add_space(points);
                }
                _ => {}
            }
        }
        self.workbench_row(ui, id, row);
    }

    /// One row of buttons sharing the width by their half slots.
    fn workbench_row(&mut self, ui: &mut Ui, id: &str, row: Vec<(Control, usize)>) {
        if row.is_empty() {
            return;
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            let unit = (ui.available_width() - 2. * (SLOTS as f32 - 1.)) / SLOTS as f32;
            for (control, slots) in &row {
                let slots = *slots as f32;
                let width = unit * slots + 2. * (slots - 1.);
                let rect = self.workbench_button(ui, control, width);
                let line = control.command.as_deref().unwrap_or_default();
                self.workbench.rects.insert(format!("{id}:{line}"), rect);
            }
        });
    }

    /// A workbench button `width` wide: its toggle style and custom icon
    /// applied, otherwise the form's own button (`toolbox_form.rs`).
    fn workbench_button(&mut self, ui: &mut Ui, control: &Control, width: f32) -> Rect {
        let mut control = control.clone();
        match control.toggle {
            ToggleStyle::Default => {}
            ToggleStyle::Checkbox => control.look = Look::Check,
            ToggleStyle::ToggleButton => control.look = Look::Framed,
        }
        let line = control.command.clone().unwrap_or_default();
        if self.workbench_icon_only(ui.ctx(), &control) {
            return self.workbench_icon_button(ui, &control, &line, width);
        }
        // An icon-only button with no icon to show, or drawn as a
        // checkbox, shows its name instead: never a blank button.
        if control.no_label {
            control.no_label = false;
            control.label = default_label(&control);
        }
        if let Some(path) = control.icon_file.clone()
            && control.toggle == ToggleStyle::Default
            && let Some(texture) = self.workbench_icon(ui.ctx(), &path)
            && let Some(state) = self.line_state(&line)
        {
            let label = if control.label.is_empty() {
                line.clone()
            } else {
                control.label.clone()
            };
            let button = CommandButton {
                active: state.on,
                unavailable: state.unavailable.as_deref(),
                shortcut: (!control.no_shortcut).then_some(line.as_str()),
                size: vec2(width, row_height()),
                ..CommandButton::row(ui, &line, &label, ButtonStyle::Picture(texture))
            };
            let response = self.command_button(ui, button);
            self.form_rects.insert(line.clone(), response.rect);
            let rect = response.rect;
            if response.clicked() {
                self.execute(&line);
            }
            if let Some(tip) = self.form_button_tooltip(&control, &line) {
                response.on_hover_text(tip);
            }
            return rect;
        }
        // A tool's button with a label of its own wears it (the form's
        // tool button always shows the tool's name).
        if control.toggle == ToggleStyle::Default
            && matches!(control.look, Look::Toolbox | Look::Weak)
            && !control.label.is_empty()
            && !control.variants.shown()
            && let Some(tool) = tool_info::by_name(&line.trim().to_lowercase())
            && let Some(state) = self.line_state(&line)
        {
            let face = crate::toolbox::ToolFace {
                active: state.on,
                label: &control.label,
                tag: None,
                binding: (!control.no_shortcut).then_some(tool.label()),
            };
            let response = self.tool_cell(ui, tool, vec2(width, row_height()), false, face);
            self.form_rects.insert(line.clone(), response.rect);
            if response.clicked() {
                self.execute(&line);
            }
            let rect = response.rect;
            if let Some(tip) = self.form_button_tooltip(&control, &line) {
                response.on_hover_text(tip);
            }
            return rect;
        }
        self.form_button_sized(ui, &control, IconMode::Both, width);
        self.form_rects.get(&line).copied().unwrap_or(Rect::NOTHING)
    }

    /// Whether `c` draws as an icon alone: its label cleared, a plain
    /// button, with an icon to show (a custom picture that loads, an icon
    /// slug, or its tool's icon) and a line the app knows.
    fn workbench_icon_only(&mut self, ctx: &Context, c: &Control) -> bool {
        let Some(line) = c.command.as_deref() else {
            return false;
        };
        if !c.no_label || c.toggle != ToggleStyle::Default || self.line_state(line).is_none() {
            return false;
        }
        let picture = c
            .icon_file
            .as_deref()
            .is_some_and(|p| self.workbench_icon(ctx, p).is_some());
        picture
            || c.icon.as_deref().and_then(icons::Icon::from_slug).is_some()
            || tool_info::by_name(&line.trim().to_lowercase()).is_some()
    }

    /// An icon-only button (provisional LAY-Q54): the picture, icon slug
    /// or tool icon centred in a cell; the tooltip still names it.
    fn workbench_icon_button(&mut self, ui: &mut Ui, c: &Control, line: &str, width: f32) -> Rect {
        let Some(state) = self.line_state(line) else {
            return Rect::NOTHING;
        };
        let name = default_label(c);
        let size = vec2(width, crate::toolbox::cell_height());
        let shortcut = (!c.no_shortcut).then_some(line);
        let picture = c
            .icon_file
            .clone()
            .and_then(|p| self.workbench_icon(ui.ctx(), &p));
        let slug = c.icon.as_deref().and_then(icons::Icon::from_slug);
        let tool = tool_info::by_name(&line.trim().to_lowercase());
        let style = match (picture, slug) {
            (Some(texture), _) => Some(ButtonStyle::PictureCell(texture)),
            (None, Some(icon)) => Some(ButtonStyle::Cell(icon)),
            (None, None) => None,
        };
        let response = match (style, tool) {
            (Some(style), _) => {
                let button = CommandButton {
                    active: state.on,
                    unavailable: state.unavailable.as_deref(),
                    shortcut,
                    size,
                    ..CommandButton::row(ui, line, &name, style)
                };
                self.command_button(ui, button)
            }
            (None, Some(tool)) => {
                let face = crate::toolbox::ToolFace {
                    active: state.on,
                    label: &name,
                    tag: None,
                    binding: (!c.no_shortcut).then_some(tool.label()),
                };
                self.tool_cell(ui, tool, size, true, face)
            }
            (None, None) => return Rect::NOTHING,
        };
        self.form_rects.insert(line.to_owned(), response.rect);
        let rect = response.rect;
        if response.clicked() {
            self.execute(line);
        }
        response.on_hover_text(self.workbench_icon_tip(c, line));
        rect
    }

    /// An icon-only button's tooltip: its own or its tool's, always
    /// naming the button in full.
    pub(super) fn workbench_icon_tip(&self, c: &Control, line: &str) -> String {
        let name = default_label(c);
        match self.form_button_tooltip(c, line) {
            None => name,
            Some(tip) if tip.contains(&name) => tip,
            Some(tip) => format!("{name} · {tip}"),
        }
    }

    /// A custom icon's texture, loaded once (PNG, JPEG or SVG, at most
    /// 64 px); `None` when the file is missing or unreadable, so the caller
    /// draws its placeholder (UIC-314).
    pub(crate) fn workbench_icon(&mut self, ctx: &Context, path: &str) -> Option<TextureId> {
        self.workbench
            .icons
            .entry(path.to_owned())
            .or_insert_with(|| {
                let image = custom_icon_picture(std::path::Path::new(path))?;
                Some(ctx.load_texture(format!("workbench-icon {path}"), image, Default::default()))
            })
            .as_ref()
            .map(TextureHandle::id)
    }

    /// Tests: whether the custom icon at `path` loaded.
    #[cfg(test)]
    pub(super) fn workbench_icon_loaded(&self, path: &str) -> bool {
        self.workbench.icons.get(path).is_some_and(Option::is_some)
    }

    /// A property control: the open tool's Tool Properties row, when its
    /// tool is open; a greyed name otherwise.
    fn workbench_property(&mut self, ui: &mut Ui, control: &Control) {
        let Some((tool, key)) = control.property.as_deref().and_then(|p| p.split_once('/')) else {
            return;
        };
        let (tool, key) = (tool.to_owned(), key.to_owned());
        let label = if control.label.is_empty() {
            key.clone()
        } else {
            control.label.clone()
        };
        let rect = if self.tool_properties_title() == tool {
            self.workbench.only_field = Some(key.clone());
            let drawn = ui.push_id(("workbench-property", &tool, &key), |ui| {
                crate::form::rows::scope(ui, "workbench-properties", |ui| self.parameters(ui))
            });
            self.workbench.only_field = None;
            drawn.response.rect
        } else {
            let why = format!("Shows {label} while {tool} is open");
            ui.gated(Some(&why), Label::new(format!("{tool} · {label}")))
                .rect
        };
        self.workbench
            .rects
            .insert(format!("property:{tool}/{key}"), rect);
    }

    /// While editing: each item one row (one chip along the top), dragged
    /// to reorder, right-clicked for its properties. Labelled dividers are
    /// drawn open and don't fold (LAY-82).
    fn workbench_edit_items(&mut self, ui: &mut Ui, id: &str, top: bool) {
        let Some(form) = self.forms.get(id).cloned() else {
            return;
        };
        if form.controls.is_empty() {
            ui.weak("Empty: Pick to Add, then click a button, a Tool Properties row, a preset or a Command History line.");
            return;
        }
        let mut rects = Vec::new();
        let mut released = None;
        for (index, control) in form.controls.iter().enumerate() {
            let size = if top {
                vec2(TOP_UNIT * control.width.units() as f32, TOP_HEIGHT - 6.)
            } else {
                vec2(ui.available_width(), row_height())
            };
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            rects.push(rect);
            self.workbench.rects.insert(format!("{id}#{index}"), rect);
            self.paint_edit_item(
                ui,
                control,
                rect,
                &response,
                self.workbench.drag == Some(index),
            );
            if response.drag_started() {
                self.workbench.drag = Some(index);
            }
            if response.drag_stopped() {
                released = Some(index);
            }
            let tip = match &control.command {
                Some(line) => format!("{line}\nDrag to reorder · right-click for its properties"),
                None => "Drag to reorder · right-click for its properties".into(),
            };
            let response = response.on_hover_text(tip);
            // Clicks inside the menu leave it open (egui's default closes
            // it on any click, a click into a text field too); its entries
            // close it themselves, and a click outside or Esc does.
            let menu = egui::Popup::default_response_id(&response);
            egui::Popup::context_menu(&response)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| self.workbench_item_menu(ui, menu, id, index, control));
        }
        let Some(from) = self.workbench.drag else {
            return;
        };
        let Some(at) = ui.ctx().pointer_latest_pos() else {
            return;
        };
        // The slot the pointer is over: how many items lie before it.
        let slot = rects
            .iter()
            .filter(|r| {
                if top {
                    r.center().x < at.x
                } else {
                    r.center().y < at.y
                }
            })
            .count();
        let stroke = Stroke::new(2.0f32, theme::chrome::active());
        if top {
            let x = rects
                .get(slot)
                .map_or_else(|| rects[rects.len() - 1].right(), |r| r.left());
            let r = rects[0];
            ui.painter().vline(x, r.y_range(), stroke);
        } else {
            let y = rects
                .get(slot)
                .map_or_else(|| rects[rects.len() - 1].bottom(), |r| r.top());
            ui.painter().hline(rects[0].x_range(), y, stroke);
        }
        if released.is_some() {
            self.workbench.drag = None;
            let to = if slot > from { slot - 1 } else { slot };
            if to != from {
                self.execute(&format!(
                    "operandi.workbench.move from:{from} to:{to} form:{id}"
                ));
            }
        }
    }

    fn paint_edit_item(
        &self,
        ui: &Ui,
        c: &Control,
        rect: Rect,
        response: &Response,
        dragged: bool,
    ) {
        let painter = ui.painter();
        let font = crate::typography::standard_font();
        let text = theme::chrome::text();
        let fill = if dragged {
            theme::chrome::active()
        } else if response.hovered() {
            theme::chrome::hover()
        } else {
            theme::chrome::control()
        };
        if let Some(label) = &c.divider {
            if label.is_empty() {
                painter.hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(1.0f32, theme::chrome::border()),
                );
                return;
            }
            // A section header, always open while editing.
            painter.rect_filled(rect, 3., theme::chrome::header());
            painter.text(
                rect.left_center() + vec2(8., 0.),
                Align2::LEFT_CENTER,
                format!("▾ {label}"),
                font,
                text,
            );
            return;
        }
        painter.rect_filled(rect, 2., fill);
        painter.rect_stroke(
            rect,
            2.,
            Stroke::new(1.0f32, theme::chrome::border()),
            StrokeKind::Inside,
        );
        let mut x = rect.left() + 6.;
        let icon = c
            .icon
            .as_deref()
            .and_then(icons::Icon::from_slug)
            .or_else(|| {
                c.command
                    .as_deref()
                    .and_then(|l| tool_info::by_name(&l.trim().to_lowercase()))
                    .map(|t| tool_info::info(t).icon)
            });
        if let Some(icon) = icon {
            let side = (rect.height() - 4.).min(20.);
            icons::paint(
                ui,
                Rect::from_center_size(pos2(x + side / 2., rect.center().y), vec2(side, side)),
                icon,
            );
            x += side + 4.;
        }
        let name = control_name(c);
        let name = if c.property.is_some() {
            format!("{name} (property)")
        } else {
            name
        };
        let tag = [
            c.no_label.then_some("Icon only"),
            (c.width != Width::Normal).then(|| c.width.label()),
            (c.level != Proficiency::Basic).then(|| level_label(c.level)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let room = rect.right() - x - 6. - if tag.is_empty() { 0. } else { 70. };
        let name = crate::button_variants::fit_label(ui, &name, &font, room.max(10.));
        painter.text(
            pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            font,
            text,
        );
        if !tag.is_empty() && rect.width() > 140. {
            painter.text(
                rect.right_center() - vec2(6., 0.),
                Align2::RIGHT_CENTER,
                tag,
                FontId::proportional(9.),
                theme::chrome::disabled_text(),
            );
        }
    }

    /// A text property in the right-click menu, starting from `current`.
    /// The menu stays open while it is typed in; Enter sets it and closes
    /// the menu, a click elsewhere (in the menu or outside it) or Set sets
    /// it, Esc drops it ([`Draft`]).
    #[allow(clippy::too_many_arguments)]
    fn menu_text(
        &mut self,
        ui: &mut Ui,
        menu: Id,
        form: &str,
        index: usize,
        attr: &str,
        label: &str,
        current: &str,
    ) {
        let mine = |d: &Draft| d.form == form && d.index == index && d.attr == attr;
        let mut text = match &self.workbench.draft {
            Some(d) if mine(d) => d.text.clone(),
            _ => current.to_owned(),
        };
        let (mut set, mut enter) = (false, false);
        ui.horizontal(|ui| {
            ui.label(label);
            let edit = ui.add(
                TextEdit::singleline(&mut text)
                    .id(Id::new(("workbench-field", form, index, attr)))
                    .desired_width(160.),
            );
            crate::numeric_tab::register(ui, &edit, None);
            if edit.has_focus() || edit.changed() {
                // Focus moved here from another field: that one is set
                // first (its lost focus shows only next frame).
                if let Some(other) = self.workbench.draft.take_if(|d| !mine(d)) {
                    self.workbench_set_draft(other);
                }
                self.workbench.draft = Some(Draft {
                    menu,
                    form: form.to_owned(),
                    index,
                    attr: attr.to_owned(),
                    text: text.clone(),
                    current: current.to_owned(),
                });
            }
            enter = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            set = edit.lost_focus() || ui.button("Set").clicked();
        });
        if set && let Some(draft) = self.workbench.draft.take_if(|d| mine(d)) {
            self.workbench_set_draft(draft);
        }
        if enter {
            ui.close();
        }
    }

    /// Sets a menu field's typed value, unless it is what the field
    /// started from.
    fn workbench_set_draft(&mut self, draft: Draft) {
        if draft.text == draft.current {
            return;
        }
        self.execute(&format!(
            "operandi.workbench.set index:{} attr:{} value:{} form:{}",
            draft.index,
            draft.attr,
            word(&draft.text),
            draft.form
        ));
    }

    /// A choice in the right-click menu: one radio per value.
    fn menu_choice(
        &mut self,
        ui: &mut Ui,
        line: (&str, usize, &str),
        title: &str,
        why: Option<&str>,
        choices: &[(&str, &str, bool)],
    ) {
        let (form, index, attr) = line;
        ui.gated_ui(why, |ui| {
            ui.menu_button(title, |ui| {
                for (label, value, on) in choices {
                    if ui.radio(*on, *label).clicked() {
                        self.execute(&format!(
                            "operandi.workbench.set index:{index} attr:{attr} value:{value} form:{form}"
                        ));
                        ui.close();
                    }
                }
            })
        });
    }

    /// An item's right-click menu (LAY-82, UIC-212).
    fn workbench_item_menu(
        &mut self,
        ui: &mut Ui,
        menu: Id,
        form: &str,
        index: usize,
        c: &Control,
    ) {
        self.menu_owned_input = true;
        // Esc cancels what is being typed (egui closes the menu).
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.workbench.draft = None;
        }
        let button = c.command.is_some();
        let divider = c.divider.is_some();
        let only_buttons = (!button).then_some("Only for buttons");
        // The Label field starts from the label shown (the tool or
        // command name by default); cleared, a button shows its icon only
        // (provisional LAY-Q54).
        let label = shown_label(c);
        self.menu_text(ui, menu, form, index, "label", "Label", &label);
        let custom = c.no_label || (!divider && !c.label.is_empty() && label != default_label(c));
        if !divider && custom && ui.button("Reset Label").clicked() {
            self.workbench.draft = None;
            self.execute(&format!(
                "operandi.workbench.set index:{index} attr:label value:{} form:{form}",
                word(&default_label(c))
            ));
            ui.close();
        }
        if !divider {
            self.menu_text(ui, menu, form, index, "tooltip", "Tooltip", &c.tooltip);
        }
        if button {
            let line = c.command.clone().unwrap_or_default();
            self.menu_text(ui, menu, form, index, "command", "Command", &line);
        }
        ui.separator();
        let popup: Vec<(&str, &str, bool)> = PopupFace::ALL
            .iter()
            .map(|f| (f.label(), popup_value(*f), c.popup == *f))
            .collect();
        self.menu_choice(
            ui,
            (form, index, "popup"),
            "Popup Face",
            only_buttons,
            &popup,
        );
        let widths: Vec<(&str, &str, bool)> = Width::ALL
            .iter()
            .map(|w| (w.label(), width_value(*w), c.width == *w))
            .collect();
        let not_divider = divider.then_some("Dividers always run the full width");
        self.menu_choice(
            ui,
            (form, index, "width"),
            "Alignment",
            not_divider,
            &widths,
        );
        let levels: Vec<(&str, &str, bool)> = [
            Proficiency::Basic,
            Proficiency::Standard,
            Proficiency::Advanced,
        ]
        .iter()
        .map(|l| (level_label(*l), level_label(*l), c.level == *l))
        .collect();
        self.menu_choice(ui, (form, index, "level"), "Proficiency", None, &levels);
        let toggles: Vec<(&str, &str, bool)> = ToggleStyle::ALL
            .iter()
            .map(|t| (t.label(), toggle_value(*t), c.toggle == *t))
            .collect();
        self.menu_choice(
            ui,
            (form, index, "toggle"),
            "Toggle Style",
            only_buttons,
            &toggles,
        );
        let icon = ui.gated(only_buttons, Button::new("Custom Icon…"));
        if icon.clicked() {
            self.execute(&format!(
                "operandi.workbench.chooseIcon index:{index} form:{form}"
            ));
            ui.close();
        }
        if c.icon_file.is_some() && ui.button("Clear Custom Icon").clicked() {
            self.execute(&format!(
                "operandi.workbench.set index:{index} attr:iconFile value:\"\" form:{form}"
            ));
            ui.close();
        }
        ui.separator();
        if ui.button("Duplicate").clicked() {
            self.execute(&format!(
                "operandi.workbench.duplicate index:{index} form:{form}"
            ));
            ui.close();
        }
        if ui.button("Delete").clicked() {
            self.execute(&format!(
                "operandi.workbench.delete index:{index} form:{form}"
            ));
            ui.close();
        }
        ui.separator();
        // Layouts stage 7 (LAY-83): the item in the Form Editor.
        if ui.button("Show in Form Editor").clicked() {
            self.execute(&format!("operandi.form.select form:{form} index:{index}"));
            ui.close();
        }
    }

    /// The workbench row along the top (LAY-82): slim, empty until the
    /// user fills it, hidden until shown (provisional LAY-Q47).
    pub(super) fn workbench_top_bar(&mut self, ctx: &Context) {
        if !self.workbench.top_bar {
            return;
        }
        TopBottomPanel::top("workbench-top")
            .exact_height(TOP_HEIGHT)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    let editing = self.workbench.editing.as_deref() == Some(TOP_WORKBENCH);
                    self.workbench_tools(ui, TOP_WORKBENCH, editing, true);
                    ui.separator();
                    if editing {
                        self.workbench_edit_items(ui, TOP_WORKBENCH, true);
                        return;
                    }
                    let Some(form) = self.forms.get(TOP_WORKBENCH).cloned() else {
                        return;
                    };
                    let level = self.form_level();
                    for control in &form.controls {
                        if control.level > level {
                            continue;
                        }
                        if control.command.is_some() {
                            let icon_only = self.workbench_icon_only(ctx, control);
                            let width = TOP_UNIT / 2. * slots(control, icon_only) as f32;
                            let rect = self.workbench_button(ui, control, width);
                            let line = control.command.as_deref().unwrap_or_default();
                            self.workbench
                                .rects
                                .insert(format!("{TOP_WORKBENCH}:{line}"), rect);
                        } else if control.divider.is_some() {
                            ui.separator();
                        } else if control.property.is_some() {
                            ui.scope(|ui| {
                                ui.set_width(TOP_UNIT * control.width.units() as f32);
                                self.workbench_property(ui, control);
                            });
                        }
                    }
                });
            });
    }
}

fn popup_value(f: PopupFace) -> &'static str {
    match f {
        PopupFace::Default => "default",
        PopupFace::Popup => "popup",
        PopupFace::Buttons => "buttons",
    }
}
fn width_value(w: Width) -> &'static str {
    match w {
        Width::Normal => "normal",
        Width::Wide => "wide",
        Width::Full => "full",
    }
}
fn toggle_value(t: ToggleStyle) -> &'static str {
    match t {
        ToggleStyle::Default => "default",
        ToggleStyle::Checkbox => "checkbox",
        ToggleStyle::ToggleButton => "toggleButton",
    }
}

/// `operandi.workbench.set`'s change to one control.
pub(super) fn set_attr(
    c: &mut Control,
    attr: &str,
    value: &str,
) -> std::result::Result<String, String> {
    let name = control_name(c);
    let bad = |what: &str| format!("{attr}: \"{value}\" isn't one of {what}");
    match attr.to_ascii_lowercase().as_str() {
        // A divider's label is its section's text. A button's cleared
        // label shows its icon only; its default name follows the tool or
        // line again (Reset Label, provisional LAY-Q54).
        "label" if c.divider.is_some() => c.divider = Some(value.trim().to_owned()),
        "label" => {
            let value = value.trim();
            c.no_label = value.is_empty() && c.command.is_some();
            c.label = if value == default_label(c) {
                String::new()
            } else {
                value.to_owned()
            };
        }
        "tooltip" => c.tooltip = value.to_owned(),
        "command" => {
            if c.command.is_none() {
                return Err(format!("{name} isn't a button; only buttons run a command"));
            }
            if value.trim().is_empty() {
                return Err("A button needs a command line".into());
            }
            c.command = Some(value.trim().to_owned());
        }
        "popup" => {
            c.popup = PopupFace::named(value).ok_or_else(|| bad("default, popup, buttons"))?
        }
        "width" => c.width = Width::named(value).ok_or_else(|| bad("normal, wide, full"))?,
        "level" => c.level = level_named(value).ok_or_else(|| bad("basic, standard, advanced"))?,
        "toggle" => {
            c.toggle =
                ToggleStyle::named(value).ok_or_else(|| bad("default, checkbox, toggleButton"))?
        }
        // The Form Editor's extra properties (layouts stage 7).
        "showwhen" => c.show_when = (!value.trim().is_empty()).then(|| value.trim().to_owned()),
        "icon" => c.icon = (!value.trim().is_empty()).then(|| value.trim().to_owned()),
        "shortcut" => c.shortcut = (!value.trim().is_empty()).then(|| value.trim().to_owned()),
        // Menus as forms: a menu entry's native macOS key equivalent.
        "keyequivalent" => {
            c.key_equivalent = (!value.trim().is_empty()).then(|| value.trim().to_owned())
        }
        "iconfile" => {
            if c.command.is_none() {
                return Err(format!("{name} isn't a button; only buttons wear an icon"));
            }
            c.icon_file = (!value.trim().is_empty()).then(|| value.trim().to_owned());
        }
        other => {
            return Err(format!(
                "No control attr \"{other}\" (label, tooltip, popup, width, level, command, toggle, iconFile, icon, shortcut, keyEquivalent, showWhen)"
            ));
        }
    }
    Ok(format!("{name}: {attr} set"))
}

/// A custom icon file as a picture of at most 64 px: PNG or JPEG through
/// `image`, SVG through resvg (UIC-314). `None` for a missing or broken
/// file.
pub(crate) fn custom_icon_picture(path: &std::path::Path) -> Option<ColorImage> {
    const MAX: u32 = 64;
    let svg = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if svg {
        let text = std::fs::read_to_string(path).ok()?;
        let tree = resvg::usvg::Tree::from_str(&text, &Default::default()).ok()?;
        let size = tree.size();
        let scale = MAX as f32 / size.width().max(size.height()).max(1.);
        let w = ((size.width() * scale).round() as u32).max(1);
        let h = ((size.height() * scale).round() as u32).max(1);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        return Some(ColorImage::from_rgba_premultiplied(
            [w as usize, h as usize],
            pixmap.data(),
        ));
    }
    let picture = image::open(path).ok()?.thumbnail(MAX, MAX).to_rgba8();
    let size = [picture.width() as usize, picture.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, picture.as_raw()))
}
