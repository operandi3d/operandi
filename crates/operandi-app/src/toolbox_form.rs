//! Draws command forms in the toolbox (layouts stage 3, LAY-80..87,
//! UIC-210..223; the data is form/command_form.rs).
//!
//! The toolbox moves onto forms one tab at a time; tabs not converted yet
//! stay drawn by their own code until their turn. Converted so far: UV
//! (`assets/forms/toolbox.uv.json`), Select (`toolbox.select.json`),
//! Deform (`toolbox.deform.json`), Duplicate (`toolbox.duplicate.json`),
//! Vertex (`toolbox.vertex.json`), Edge (`toolbox.edge.json`), Polygon
//! (`toolbox.polygon.json`), Edit (`toolbox.edit.json`, with the UV
//! operations shared with UV as `toolbox.uv.ops.json`) and Sculpt
//! (`toolbox.sculpt.json`) and Create (`toolbox.create.json`, batch 7).
//!
//! Every button reads its state from its command (UIC-223): lit from the
//! command's `?` answer, greyed (with the reason as its tooltip) from its
//! enable answer, both through the registry's state answers
//! (`command_handlers.rs`, `answers`). A line the registry doesn't know is
//! drawn red and does nothing (UIC-221). A click runs the line through
//! `execute`, like a menu or a typed line; Ctrl/Cmd, Shift and Alt run the
//! control's variant lines when it has them.
use super::*;
use crate::button_variants::Mods;
use crate::form::Proficiency;
use crate::form::command_form::{self as cf, Control, FormStore, IconMode, Item, Look, Variant};
use crate::toolbox::{ButtonStyle, CommandButton, ToolFace, cell_height, row_height};
use crate::unavailable::Gate;
use operandi_core::command_runtime::CommandState;
use std::path::Path;

/// The built-in form files: every `*.json` in `assets/forms`, listed by
/// build.rs.
pub(super) const SHIPPED_FORMS: &[(&str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/shipped_forms.rs"));

/// The UV tab's form.
pub(super) const UV_FORM: &str = "toolbox.uv";
/// The Select tab's form.
pub(super) const SELECT_FORM: &str = "toolbox.select";
/// The Deform tab's form.
pub(super) const DEFORM_FORM: &str = "toolbox.deform";
/// The Duplicate tab's form.
pub(super) const DUPLICATE_FORM: &str = "toolbox.duplicate";
/// The Vertex tab's form.
pub(super) const VERTEX_FORM: &str = "toolbox.vertex";
/// The Edge tab's form.
pub(super) const EDGE_FORM: &str = "toolbox.edge";
/// The Polygon tab's form.
pub(super) const POLYGON_FORM: &str = "toolbox.polygon";
/// The Edit tab's form.
pub(super) const EDIT_FORM: &str = "toolbox.edit";
/// The Sculpt tab's form.
pub(super) const SCULPT_FORM: &str = "toolbox.sculpt";
/// The Create tab's form.
pub(super) const CREATE_FORM: &str = "toolbox.create";
/// The Rig tab's form.
pub(super) const RIG_FORM: &str = "toolbox.rig";
/// The Curve tab's form.
pub(super) const CURVE_FORM: &str = "toolbox.curve";
/// The Paint tab's form (painting builds only, PNT-85).
#[cfg(feature = "painting")]
pub(super) const PAINT_FORM: &str = "toolbox.paint";
/// The Sculpt/Paint set's Vertex Map Tools and Utilities tabs (PNT-211, 212).
#[cfg(feature = "painting")]
pub(super) const VERTEX_MAPS_FORM: &str = "toolbox.vertexmaps";
#[cfg(feature = "painting")]
pub(super) const PAINT_UTILITIES_FORM: &str = "toolbox.paintutil";
/// Painting builds' own forms: not in the shipped folder, so default
/// builds never list them.
#[cfg(feature = "painting")]
const PAINTING_FORMS: &[(&str, &str)] = &[
    (
        "toolbox.paint.json",
        include_str!("../assets/forms-painting/toolbox.paint.json"),
    ),
    // PNT-151: the Sculpt tab's tip, preset and nozzle buttons, in its
    // tail slot (sculpting with them comes later, PNT-Q38).
    (
        "toolbox.sculpt.painting.json",
        include_str!("../assets/forms-painting/toolbox.sculpt.painting.json"),
    ),
    (
        "toolbox.vertexmaps.json",
        include_str!("../assets/forms-painting/toolbox.vertexmaps.json"),
    ),
    (
        "toolbox.paintutil.json",
        include_str!("../assets/forms-painting/toolbox.paintutil.json"),
    ),
];
/// The Topology tab's form.
pub(super) const TOPOLOGY_FORM: &str = "toolbox.topology";
/// The Setup set's tabs (Jason, 2026-11-23).
pub(super) const SETUP_FORM: &str = "toolbox.setup";
pub(super) const CONSTRAINTS_FORM: &str = "toolbox.setup.constraints";
pub(super) const IK_FORM: &str = "toolbox.setup.ik";
pub(super) const WEIGHTING_FORM: &str = "toolbox.setup.weighting";
/// The Game set's tabs (Jason, 2026-11-23).
pub(super) const GAME_VERTEX_MAP_FORM: &str = "toolbox.game.vertexmap";
pub(super) const BAKING_FORM: &str = "toolbox.game.baking";

/// How long a press on a tool flyout is held before its list opens.
const HOLD_SECONDS: f64 = 0.35;
/// How long a press on a hold cell is held before its menu opens: the
/// same short hold as a flyout (N24, UIC-25; it was 0.8 s, long enough to
/// seem not to work).
const HOLD_CELL_SECONDS: f64 = HOLD_SECONDS;

/// The shipped forms, with the user's copies from `layouts_folder`'s
/// Forms folder on top (LAY-87).
pub(super) fn shipped_store(layouts_folder: Option<&Path>) -> FormStore {
    let user = layouts_folder.map(|f| f.join(cf::USER_FOLDER));
    #[cfg(feature = "painting")]
    {
        let all: Vec<(&str, &str)> = SHIPPED_FORMS
            .iter()
            .chain(PAINTING_FORMS)
            .copied()
            .collect();
        FormStore::read(&all, user.as_deref())
    }
    #[cfg(not(feature = "painting"))]
    FormStore::read(SHIPPED_FORMS, user.as_deref())
}

/// A flat row's look: an icon row with `icon`, or the tool's icon for a
/// tool's line, else a text row.
fn flat_style(control: &Control, line: &str) -> ButtonStyle {
    match control_icon(control, line) {
        Some(icon) => ButtonStyle::IconRow(icon),
        None => ButtonStyle::TextRow,
    }
}

/// A control's icon: its own, or its tool's for a tool's line.
fn control_icon(control: &Control, line: &str) -> Option<icons::Icon> {
    control
        .icon
        .as_deref()
        .and_then(icons::Icon::from_slug)
        .or_else(|| {
            tool_info::by_name(&line.trim().to_lowercase()).map(|t| tool_info::info(t).icon)
        })
        .filter(|&i| i != icons::Icon::ScriptAction)
}

/// One row of a pop-up button's list: its icon (N26) and name, lit when on.
fn list_item(
    ui: &Ui,
    control: &Control,
    line: &str,
    label: &str,
    on: bool,
) -> egui::Button<'static> {
    match control_icon(control, line) {
        Some(icon) => icons::button(ui, icon, label).selected(on),
        None => egui::Button::selectable(on, label.to_owned()),
    }
}

/// Whose binding a button shows: a shown variant's own line's, else the
/// control's `shortcut` or its line's.
fn shown_keys<'c>(control: &'c Control, line: &'c str, shown: Option<&'c Variant>) -> &'c str {
    match shown {
        Some(v) => v.line(),
        None => control.shortcut.as_deref().unwrap_or(line),
    }
}

/// A button's label: its shown variant's, else its own, else its line.
fn shown_label<'c>(control: &'c Control, line: &'c str, shown: Option<&'c Variant>) -> &'c str {
    match shown {
        Some(Variant::Shown { label, .. }) => label,
        _ if control.label.is_empty() => line,
        _ => &control.label,
    }
}

/// The icon a button wears: its shown variant's own, else its control's
/// (`control_icon`).
fn shown_icon(control: &Control, line: &str, shown: Option<&Variant>) -> Option<icons::Icon> {
    match shown {
        Some(Variant::Shown { icon, .. }) if !icon.is_empty() => icons::Icon::from_slug(icon),
        _ => None,
    }
    .or_else(|| control_icon(control, line))
}

/// A shown variant's corner tag, if it has one.
fn shown_tag(shown: Option<&Variant>) -> Option<&str> {
    match shown {
        Some(Variant::Shown { tag, .. }) if !tag.is_empty() => Some(tag),
        _ => None,
    }
}

/// The variant of `control` for exactly the keys `held`, if it has one.
fn variant_for(control: &Control, held: Mods) -> Option<&Variant> {
    let v = &control.variants;
    match held {
        h if h == Mods::SHIFT => v.shift.as_ref(),
        h if h == Mods::CTRL => v.ctrl.as_ref(),
        h if h == Mods::ALT => v.alt.as_ref(),
        _ => None,
    }
}

/// A tooltip's `{variants}`: "Shift: Label · Option: Label", each variant
/// by its key and the label it shows (MB-Q7's hidden-variant line).
fn variant_list(control: &Control, line: &str) -> String {
    let own = if control.label.is_empty() {
        line
    } else {
        control.label.as_str()
    };
    let v = &control.variants;
    [
        (Mods::SHIFT, &v.shift),
        (Mods::CTRL, &v.ctrl),
        (Mods::ALT, &v.alt),
    ]
    .into_iter()
    .filter_map(|(mods, variant)| {
        let label = match variant.as_ref()? {
            Variant::Shown { label, .. } => label.as_str(),
            Variant::Line(_) => own,
        };
        Some(format!("{}: {label}", mods.label()))
    })
    .collect::<Vec<_>>()
    .join(" · ")
}

/// Where a button sits: a full-width row of a vertical form, or a share
/// of a horizontal one's width.
#[derive(Clone, Copy)]
enum Place {
    Column,
    Row(f32),
}

impl Operandi {
    /// What the command `line` answers about itself now (UIC-223), or
    /// `None` when no command has that name (UIC-221). `@macro` lines are
    /// scripts the registry doesn't list; they count as known (UIC-222).
    pub(super) fn line_state(&self, line: &str) -> Option<CommandState> {
        let line = line.trim();
        if line.starts_with('@') {
            return Some(CommandState::default());
        }
        self.commands.state(self, line)
    }

    /// The line is known and can run now (LAY-85's show-only-when).
    pub(super) fn line_enabled(&self, line: &str) -> bool {
        self.line_state(line)
            .is_some_and(|s| s.unavailable.is_none())
    }

    /// The toolbox's proficiency level (LAY-86). Everything shows until the
    /// Base Form Proficiency preference exists (provisional, LAY-Q11).
    pub(super) fn form_level(&self) -> Proficiency {
        Proficiency::Advanced
    }

    /// Draws form `id`.
    pub(super) fn draw_form(&mut self, ui: &mut Ui, id: &str) {
        // A workbench draws its edit tools too (layouts stage 4).
        if self.forms.get(id).is_some_and(|f| f.workbench) {
            return self.draw_workbench(ui, id);
        }
        let items = cf::plan(&self.forms, id, self.form_level(), &|l| {
            self.line_enabled(l)
        });
        // Depth of folded sections being skipped.
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
                Item::SectionEnd => {}
                Item::Separator => {
                    ui.separator();
                }
                Item::Widget(widget) => self.form_widget(ui, &widget),
                Item::Heading { key, label, weak } => {
                    let text = if weak {
                        RichText::new(label).weak()
                    } else {
                        RichText::new(label)
                            .size(crate::typography::STANDARD_SIZE)
                            .color(theme::chrome::text())
                    };
                    let heading = ui.label(text);
                    self.toolbox_rects.insert(key, heading.rect);
                }
                Item::Flyout {
                    key,
                    face,
                    controls,
                } => self.form_flyout(ui, &key, face, &controls),
                Item::Space(points) => {
                    ui.add_space(points);
                }
                Item::Menu {
                    label,
                    tooltip,
                    controls,
                } => self.form_menu(ui, &label, &tooltip, &controls),
                Item::Button(control, icons) => {
                    self.form_button(ui, &control, icons, Place::Column)
                }
                Item::Row(controls, icons, columns) => {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.;
                        let n = controls.len().max(columns) as f32;
                        let width = (ui.available_width() - 2. * (n - 1.)) / n;
                        for control in &controls {
                            if let Some(widget) = &control.widget {
                                self.form_row_widget(ui, widget, width);
                            } else if let Some(cf::FormRef::Inline(hold)) = &control.form {
                                let height = if icons == IconMode::Icon {
                                    cell_height()
                                } else {
                                    row_height()
                                };
                                self.form_hold(
                                    ui,
                                    &control.tooltip,
                                    &hold.controls,
                                    icons,
                                    vec2(width, height),
                                );
                            } else {
                                self.form_button(ui, control, icons, Place::Row(width));
                            }
                        }
                    });
                }
                Item::Hold {
                    tooltip,
                    controls,
                    icons,
                } => {
                    let height = if icons == IconMode::Icon {
                        cell_height()
                    } else {
                        row_height()
                    };
                    let size = vec2(ui.available_width(), height);
                    self.form_hold(ui, &tooltip, &controls, icons, size);
                }
                // Only a workbench holds properties (workbench.rs).
                Item::Property(_) => {}
                Item::Missing(id) => {
                    ui.label(RichText::new(format!("Missing form {id}")).color(theme::error()))
                        .on_hover_text("No form has this id; it shows nothing (UIC-221)");
                }
            }
        }
    }

    /// The escape hatch's custom widgets (`command_form::WIDGETS`).
    fn form_widget(&mut self, ui: &mut Ui, widget: &str) {
        match widget {
            "uv.mapField" => self.uv_map_field(ui),
            "select.lassoStyle" => self.lasso_style_field(ui),
            "topology.foregroundOpacity" => self.topology_opacity_field(ui),
            "topology.hitSize" => self.topology_hit_size_field(ui),
            "select.falloffPlaceholder" => crate::select_panel::falloff_placeholder(ui),
            "select.polygonTagType" => self.polygon_tag_type_field(ui),
            "sculpt.weightMap" => self.weight_map_row(ui),
            "sculpt.newWeightMap" => self.new_weight_map_button(ui),
            other => {
                ui.label(RichText::new(format!("Unknown widget {other}")).color(theme::error()));
            }
        }
    }

    /// An escape-hatch widget sharing a row, `width` wide.
    fn form_row_widget(&mut self, ui: &mut Ui, widget: &str, width: f32) {
        match widget {
            "sculpt.pinchSlot" => self.sculpt_pinch_slot(ui, width),
            other => {
                ui.label(RichText::new(format!("Unknown widget {other}")).color(theme::error()));
            }
        }
    }

    /// A form's button `width` wide (a workbench's, workbench.rs).
    pub(super) fn form_button_sized(
        &mut self,
        ui: &mut Ui,
        control: &Control,
        icons: IconMode,
        width: f32,
    ) {
        self.form_button(ui, control, icons, Place::Row(width));
    }

    /// The tooltip a form button running `line` shows.
    pub(super) fn form_button_tooltip(&self, control: &Control, line: &str) -> Option<String> {
        self.button_tooltip(control, line)
    }

    fn form_button(&mut self, ui: &mut Ui, control: &Control, icons: IconMode, place: Place) {
        let Some(line) = control.command.as_deref() else {
            return;
        };
        // A labelled variant shows while its key is held: its label, its
        // line's state and its tooltip (MB). `line` stays the button's key.
        let shown = self.held_variant(control);
        let run = shown.map_or(line, Variant::line);
        let keys = shown_keys(control, line, shown);
        let label = shown_label(control, line, shown);
        let icon_only = icons == IconMode::Icon;
        let width = match place {
            Place::Column => ui.available_width(),
            Place::Row(w) => w,
        };
        let Some(state) = self.line_state(run) else {
            let height = if icon_only {
                cell_height()
            } else {
                row_height()
            };
            let rect = self.unknown_button(ui, vec2(width, height), label, line);
            self.form_rects.insert(line.to_owned(), rect);
            return;
        };
        let response = match control.look {
            Look::Framed => {
                let why = state.unavailable.as_deref();
                let response = match place {
                    // A full-width row shows its shortcut after the label.
                    Place::Column => {
                        let text = format!("{label}    {}", self.shortcut(keys));
                        let button = Button::new(text).selected(state.on);
                        ui.add_sized(vec2(width, row_height()), |ui: &mut Ui| {
                            ui.gated(why, button)
                        })
                    }
                    Place::Row(w) => ui.gated(
                        why,
                        Button::new(label)
                            .selected(state.on)
                            .min_size(vec2(w, row_height())),
                    ),
                };
                self.toolbox_rects.insert(label.to_owned(), response.rect);
                response
            }
            // Weak is for headings; on a button it is the toolbox look.
            Look::Toolbox | Look::Weak => {
                let height = if icon_only {
                    cell_height()
                } else {
                    row_height()
                };
                if let Some(tool) = tool_info::by_name(&line.to_lowercase())
                    && control.variants.shown()
                {
                    // A tool's button with labelled variants (batch 7: Cube,
                    // Sphere, Cylinder): the tool's cell wearing what is
                    // shown, with its corner tag; the form runs the click.
                    let response = self.tool_cell(
                        ui,
                        tool,
                        vec2(width, height),
                        icon_only,
                        ToolFace {
                            active: state.on,
                            label,
                            tag: shown_tag(shown),
                            binding: (!control.no_shortcut).then_some(keys),
                        },
                    );
                    self.form_rects.insert(line.to_owned(), response.rect);
                    self.run_shown_variant(ui, control, line, &response);
                    if let Some(tip) = self.button_tooltip(control, line) {
                        response.on_hover_text(tip);
                    }
                    return;
                }
                if let Some(tool) = tool_info::by_name(&line.to_lowercase()) {
                    // A tool's button keeps its own look, tooltip and
                    // shortcut; the lit state is its command's answer.
                    let clicked =
                        self.tool_button(ui, tool, vec2(width, height), icon_only, Some(state.on));
                    self.form_rects
                        .insert(line.to_owned(), self.toolbox_rects[tool.label()]);
                    if clicked {
                        self.run_form_line(ui, control, line);
                    }
                    return;
                }
                let icon = match shown {
                    Some(Variant::Shown { icon, .. }) if !icon.is_empty() => {
                        icons::Icon::from_slug(icon)
                    }
                    _ => None,
                }
                .or_else(|| control.icon.as_deref().and_then(icons::Icon::from_slug));
                let style = match (icons, icon) {
                    (IconMode::Icon, Some(icon)) => ButtonStyle::Cell(icon),
                    (IconMode::Both, Some(icon)) => ButtonStyle::IconRow(icon),
                    _ => ButtonStyle::TextRow,
                };
                let mut button = CommandButton {
                    active: state.on,
                    unavailable: state.unavailable.as_deref(),
                    shortcut: (!control.no_shortcut).then_some(keys),
                    // Keyed by its line: tests and the UIC-164 inventory
                    // find it by the command it runs.
                    ..CommandButton::row(ui, line, label, style)
                };
                if let Place::Row(w) = place {
                    button.size = vec2(w, height);
                }
                let response = self.command_button(ui, button);
                if let Some(tag) = shown_tag(shown) {
                    crate::toolbox::paint_corner_tag(ui, response.rect, tag, state.on);
                }
                response
            }
            Look::Tall => {
                let button = CommandButton {
                    active: state.on,
                    unavailable: state.unavailable.as_deref(),
                    shortcut: (!control.no_shortcut).then_some(keys),
                    size: vec2(width, row_height()),
                    ..CommandButton::row(ui, line, label, ButtonStyle::TextRow)
                };
                self.command_button(ui, button)
            }
            Look::Check => {
                let mut on = state.on;
                let response = ui.gated(
                    state.unavailable.as_deref(),
                    egui::Checkbox::new(&mut on, label),
                );
                self.toolbox_rects.insert(label.to_owned(), response.rect);
                response
            }
            Look::Button => {
                let text = format!("{label}{}", self.shortcut(keys));
                let response = ui.gated(state.unavailable.as_deref(), Button::new(text));
                self.toolbox_rects.insert(label.to_owned(), response.rect);
                response
            }
            Look::Natural => {
                let text = format!("{label}    {}", self.shortcut(keys));
                let response = ui.gated(state.unavailable.as_deref(), Button::new(text));
                self.toolbox_rects.insert(label.to_owned(), response.rect);
                response
            }
            Look::Centered => {
                let mut button = CommandButton {
                    active: state.on,
                    unavailable: state.unavailable.as_deref(),
                    shortcut: None,
                    ..CommandButton::row(ui, line, label, ButtonStyle::Centered)
                };
                if let Place::Row(w) = place {
                    button.size = vec2(w, button.size.y);
                }
                self.command_button(ui, button)
            }
            Look::Flat => {
                let mut button = CommandButton {
                    active: state.on,
                    unavailable: state.unavailable.as_deref(),
                    shortcut: (!control.no_shortcut).then_some(keys),
                    ..CommandButton::row(
                        ui,
                        line,
                        label,
                        match shown_icon(control, line, shown) {
                            Some(icon) => ButtonStyle::IconRow(icon),
                            None => flat_style(control, line),
                        },
                    )
                };
                if let Place::Row(w) = place {
                    button.size = vec2(w, button.size.y);
                }
                self.command_button(ui, button)
            }
        };
        self.form_rects.insert(line.to_owned(), response.rect);
        if control.variants.shown() {
            self.run_shown_variant(ui, control, line, &response);
        } else if response.clicked() {
            self.run_form_line(ui, control, line);
        }
        if let Some(tip) = self.button_tooltip(control, line) {
            response.on_hover_text(tip);
        }
    }

    /// A form button's tooltip for the keys held now: its shown variant's,
    /// else its own, else its tool's or tool preset's.
    fn button_tooltip(&self, control: &Control, line: &str) -> Option<String> {
        let shown = self.held_variant(control);
        let keys = shown_keys(control, line, shown);
        let tooltip = match shown {
            Some(Variant::Shown { tooltip, .. }) if !tooltip.is_empty() => tooltip,
            _ => &control.tooltip,
        };
        if tooltip.is_empty() {
            return self.line_tooltip(line);
        }
        Some(if tooltip.contains("{variants}") {
            // The MB tooltip ends at the variant list.
            let tip = tooltip.replace("{variants}", &variant_list(control, line));
            self.form_tooltip(&tip, keys).trim_end().to_owned()
        } else {
            self.form_tooltip(tooltip, keys)
        })
    }

    /// Tests: the label and tooltip the button running `line` in form
    /// `form` shows for the keys held now.
    #[cfg(test)]
    pub(super) fn form_button_for_test(&self, form: &str, line: &str) -> (String, String) {
        let items = cf::plan(&self.forms, form, Proficiency::Advanced, &|_| true);
        let control = items
            .into_iter()
            .flat_map(|i| match i {
                Item::Button(c, _) => vec![*c],
                Item::Row(cs, _, _) => cs,
                _ => vec![],
            })
            .find(|c| c.command.as_deref() == Some(line))
            .unwrap_or_else(|| panic!("{form} has no button running {line}"));
        let shown = self.held_variant(&control);
        (
            shown_label(&control, line, shown).to_owned(),
            self.button_tooltip(&control, line).unwrap_or_default(),
        )
    }

    /// The variant a button with labelled variants shows for the keys held
    /// now (MB-01..05: app-wide, exactly that key, Cmd never), or `None`
    /// for its own line. Buttons without labelled variants never swap.
    fn held_variant<'c>(&self, control: &'c Control) -> Option<&'c Variant> {
        if !control.variants.shown() {
            return None;
        }
        variant_for(control, self.button_variants.held())
    }

    /// A labelled-variant button's click runs what it showed at the press
    /// (MB-10/12), its own line when no variant key was held.
    fn run_shown_variant(&mut self, ui: &Ui, control: &Control, line: &str, response: &Response) {
        let press = Id::new(("form-variant-press", line));
        if let Some(mods) = self.button_variants.press_on(ui, response.rect) {
            ui.ctx().data_mut(|d| d.insert_temp(press, mods));
        }
        if !response.clicked() {
            return;
        }
        let mods = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<Mods>(press))
            .unwrap_or_else(|| self.button_variants.held());
        // As the MB variant buttons: nothing while a modal, menu,
        // navigation drag or layout gesture owns the input (PM-T08/T09).
        if !self.editor_input_allowed(ui.ctx())
            || self.navigation.is_some()
            || self.layout_owned_input
        {
            return;
        }
        let run = variant_for(control, mods)
            .map_or(line, Variant::line)
            .to_owned();
        self.execute(&run);
    }

    /// The tooltip a button running `line` shows when its control has
    /// none: a tool's or a tool preset's own, as their old buttons did.
    fn line_tooltip(&self, line: &str) -> Option<String> {
        // A preset with layers first: Bend's line starts its preset.
        if let Some(row) = operandi_core::tool_preset::row(line.trim())
            .filter(|r| !r.layers.is_empty() && r.actor.app.is_none())
        {
            let binding = self.shortcut(&row.label);
            return Some(format!(
                "{} · {}. Click again to drop. Shortcut: {}",
                row.label,
                row.tooltip,
                if binding.is_empty() {
                    "unassigned"
                } else {
                    &binding
                }
            ));
        }
        let tool = tool_info::by_name(&line.trim().to_lowercase())?;
        let binding = self
            .bindings
            .get(tool.label())
            .filter(|b| !b.is_empty())
            .map_or("unassigned", String::as_str);
        Some(format!(
            "{} · {}. Click again to drop. Shortcut: {binding}",
            tool.label(),
            tool_info::info(tool).tooltip
        ))
    }

    /// A tool flyout (`command_form::Style::Flyout`): a button wearing one
    /// of `controls`' commands (the face, `start` until one is picked).
    /// A click runs the face; holding the press or a right-click opens the
    /// list, and the command picked there runs and becomes the face. Lit
    /// while any of its commands is on, greyed (with the first reason)
    /// only while none can run (UIC-223).
    fn form_flyout(&mut self, ui: &mut Ui, key: &str, start: usize, controls: &[Control]) {
        let face_id = Id::new(("form-flyout-face", key));
        let open_id = Id::new(("form-flyout-open", key));
        let held_id = Id::new(("form-flyout-held", key));
        let lines: Vec<&str> = controls
            .iter()
            .map(|c| c.command.as_deref().unwrap_or_default())
            .collect();
        let states: Vec<CommandState> = lines
            .iter()
            .map(|l| {
                self.line_state(l).unwrap_or_else(|| CommandState {
                    unavailable: Some(format!("Unknown command: {l}")),
                    on: false,
                    title: None,
                })
            })
            .collect();
        let label_of = |i: usize| {
            if controls[i].label.is_empty() {
                lines[i]
            } else {
                controls[i].label.as_str()
            }
        };
        let face = ui
            .ctx()
            .data(|d| d.get_temp::<usize>(face_id))
            .unwrap_or(start)
            .min(controls.len() - 1);
        let (control, line) = (&controls[face], lines[face]);
        let why = if states.iter().any(|s| s.unavailable.is_none()) {
            None
        } else {
            states.iter().find_map(|s| s.unavailable.clone())
        };
        let button = CommandButton {
            active: states.iter().any(|s| s.on),
            unavailable: why.as_deref(),
            ..CommandButton::row(ui, line, label_of(face), flat_style(control, line))
        };
        let response = self.command_button(ui, button);
        self.form_rects
            .insert(format!("flyout:{key}"), response.rect);
        // The corner triangle.
        let r = response.rect;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                r.right_bottom() - vec2(2., 2.),
                r.right_bottom() - vec2(8., 2.),
                r.right_bottom() - vec2(2., 8.),
            ],
            theme::chrome::text(),
            Stroke::NONE,
        ));
        let mut open = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(open_id))
            .unwrap_or(false);
        let held = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(held_id))
            .unwrap_or(false);
        if response.is_pointer_button_down_on() && !open {
            let (now, pressed) = ui.input(|i| (i.time, i.pointer.press_start_time()));
            if pressed.is_some_and(|t| now - t >= HOLD_SECONDS) {
                open = true;
                ui.ctx().data_mut(|d| d.insert_temp(held_id, true));
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(HOLD_SECONDS));
            }
        }
        if response.secondary_clicked() {
            open = !open;
        }
        if response.clicked() {
            if held {
                // The release ending a hold doesn't also run the face.
                ui.ctx().data_mut(|d| d.insert_temp(held_id, false));
            } else if let Some(reason) = &states[face].unavailable {
                self.status = reason.clone();
            } else {
                self.run_form_line(ui, control, line);
            }
        }
        let face_tip = if !control.tooltip.is_empty() {
            self.form_tooltip(&control.tooltip, line)
        } else if let Some(reason) = &states[face].unavailable {
            reason.clone()
        } else {
            self.line_tooltip(line)
                .unwrap_or_else(|| label_of(face).to_owned())
        };
        let names: Vec<&str> = (0..controls.len()).map(label_of).collect();
        let response = response.on_hover_text(format!(
            "{face_tip} · hold or right-click for {}",
            names.join(", ")
        ));
        if open {
            let mut picked = None;
            let area = egui::Area::new(Id::new(("form-flyout-list", key)))
                .order(egui::Order::Foreground)
                .fixed_pos(r.right_top() + vec2(2., 0.))
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(170.);
                        for (index, state) in states.iter().enumerate() {
                            let row = list_item(
                                ui,
                                &controls[index],
                                lines[index],
                                label_of(index),
                                state.on,
                            );
                            let item = ui.gated(state.unavailable.as_deref(), row);
                            self.form_rects.insert(lines[index].to_owned(), item.rect);
                            if item.clicked() {
                                picked = Some(index);
                            }
                        }
                    });
                });
            if let Some(index) = picked {
                open = false;
                ui.ctx().data_mut(|d| d.insert_temp(face_id, index));
                self.execute(lines[index]);
            } else if ui.input(|i| i.pointer.any_pressed())
                && !area.response.contains_pointer()
                && !response.contains_pointer()
            {
                open = false;
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
    }

    /// A hold cell (`command_form::Style::Hold`, batch 7: the Create tab's
    /// Tube): a click runs the first of `controls`; a right-click or a
    /// press held [`HOLD_CELL_SECONDS`] opens a menu of them all, each lit
    /// from its answer. It wears the first lit command's icon (else the
    /// first's), lights while any is on and greys (with the first reason)
    /// only while none can run (UIC-223).
    fn form_hold(
        &mut self,
        ui: &mut Ui,
        tooltip: &str,
        controls: &[Control],
        icons: IconMode,
        size: Vec2,
    ) {
        if controls.is_empty() {
            return;
        }
        let lines: Vec<&str> = controls
            .iter()
            .map(|c| c.command.as_deref().unwrap_or_default())
            .collect();
        let states: Vec<CommandState> = lines
            .iter()
            .map(|l| {
                self.line_state(l).unwrap_or_else(|| CommandState {
                    unavailable: Some(format!("Unknown command: {l}")),
                    on: false,
                    title: None,
                })
            })
            .collect();
        let label_of = |i: usize| {
            if controls[i].label.is_empty() {
                lines[i]
            } else {
                controls[i].label.as_str()
            }
        };
        let worn = states.iter().position(|s| s.on).unwrap_or(0);
        let icon = controls[worn]
            .icon
            .as_deref()
            .or(controls[0].icon.as_deref())
            .and_then(icons::Icon::from_slug);
        let style = match (icons, icon) {
            (IconMode::Icon, Some(icon)) => ButtonStyle::Cell(icon),
            (IconMode::Both, Some(icon)) => ButtonStyle::IconRow(icon),
            _ => ButtonStyle::TextRow,
        };
        let why = if states.iter().any(|s| s.unavailable.is_none()) {
            None
        } else {
            states.iter().find_map(|s| s.unavailable.clone())
        };
        let line = lines[0];
        let cell = CommandButton {
            size,
            active: states.iter().any(|s| s.on),
            unavailable: why.as_deref(),
            shortcut: (!controls[0].no_shortcut).then_some(line),
            ..CommandButton::row(ui, line, label_of(0), style)
        };
        let response = self.command_button(ui, cell);
        self.form_rects.insert(line.to_owned(), response.rect);
        // UIC-25: the pop-up cue in the lower-right corner.
        let c = response.rect.right_bottom() - vec2(2., 2.);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![c, c - vec2(0., 4.), c - vec2(4., 0.)],
            theme::text(),
            Stroke::NONE,
        ));
        let pressed_for = ui.input(|i| i.pointer.press_start_time().map(|t| i.time - t));
        let holding = response.is_pointer_button_down_on() && response.contains_pointer();
        let held = holding && pressed_for.is_some_and(|t| t > HOLD_CELL_SECONDS);
        // Remembered until the release, which then doesn't also run the
        // cell (N24: with the short hold, that release still counts as a
        // click).
        let held_id = Id::new(("form-hold-held", line));
        if held {
            ui.ctx().data_mut(|d| d.insert_temp(held_id, true));
        }
        if holding && !held {
            let left = HOLD_CELL_SECONDS - pressed_for.unwrap_or(0.) + 0.01;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(left.max(0.01)));
        }
        let open =
            (response.secondary_clicked() || held).then_some(egui::SetOpenCommand::Bool(true));
        let chosen = egui::Popup::menu(&response)
            .id(Id::new(("form-hold-popup", line)))
            .open_memory(open)
            .show(|ui| {
                self.menu_owned_input = true;
                let mut picked = None;
                for (index, state) in states.iter().enumerate() {
                    let row = list_item(
                        ui,
                        &controls[index],
                        lines[index],
                        label_of(index),
                        state.on,
                    );
                    let item = ui.gated(state.unavailable.as_deref(), row);
                    if item.clicked() {
                        picked = Some(index);
                    }
                }
                if picked.is_some() {
                    ui.close();
                }
                picked
            })
            .and_then(|r| r.inner);
        if response.clicked() {
            let was_held = ui
                .ctx()
                .data_mut(|d| d.remove_temp::<bool>(held_id))
                .unwrap_or(false);
            if !held && !was_held {
                self.execute(line);
            }
        } else if !holding {
            ui.ctx().data_mut(|d| d.remove::<bool>(held_id));
        }
        if let Some(index) = chosen {
            self.execute(lines[index]);
        }
        let tip = if tooltip.is_empty() {
            label_of(0).to_owned()
        } else {
            self.form_tooltip(tooltip, line)
        };
        response.on_hover_text(tip);
    }

    /// A tooltip with `{shortcut}` replaced by the line's binding (the
    /// editable input map), or "unassigned".
    /// `{binding}` is the binding as a button draws it ("  [P] "), or
    /// nothing when unbound.
    fn form_tooltip(&self, tooltip: &str, line: &str) -> String {
        let mut tooltip = tooltip.to_owned();
        // `{binding:<line>}`: another line's binding (batch 7).
        while let Some(start) = tooltip.find("{binding:") {
            let Some(len) = tooltip[start..].find('}') else {
                break;
            };
            let named = tooltip[start + "{binding:".len()..start + len].to_owned();
            tooltip.replace_range(start..=start + len, &self.shortcut(&named));
        }
        if tooltip.contains("{andBinding}") {
            let binding = self.shortcut(line);
            let and = if binding.is_empty() {
                String::new()
            } else {
                format!(" · {binding}")
            };
            tooltip = tooltip.replace("{andBinding}", &and);
        }
        let mut tooltip = tooltip.replace("{binding}", &self.shortcut(line));
        if tooltip.contains("{chord}") {
            // As the input map displays it (batch 6: the sculpt brushes).
            let chord = self
                .bindings
                .get(line)
                .filter(|b| !b.is_empty())
                .map_or_else(|| "Unassigned".to_owned(), |b| input::display_chord(b));
            tooltip = tooltip.replace("{chord}", &chord);
        }
        if !tooltip.contains("{shortcut}") {
            return tooltip;
        }
        let binding = self
            .bindings
            .get(line)
            .filter(|b| !b.is_empty())
            .cloned()
            .unwrap_or_else(|| "unassigned".into());
        tooltip.replace("{shortcut}", &binding)
    }

    #[cfg(test)]
    pub(super) fn form_tooltip_for_test(&self, tooltip: &str, line: &str) -> String {
        self.form_tooltip(tooltip, line)
    }

    /// A flyout: a button opening a menu of `controls`' lines, each with
    /// its shortcut. Greyed, with the first reason, while none can run
    /// (UIC-223).
    fn form_menu(&mut self, ui: &mut Ui, label: &str, tooltip: &str, controls: &[Control]) {
        let lines: Vec<&str> = controls
            .iter()
            .filter_map(|c| c.command.as_deref())
            .collect();
        let states: Vec<Option<CommandState>> = lines.iter().map(|l| self.line_state(l)).collect();
        let why = if states
            .iter()
            .any(|s| s.as_ref().is_some_and(|s| s.unavailable.is_none()))
        {
            None
        } else {
            states
                .iter()
                .flatten()
                .find_map(|s| s.unavailable.clone())
                .or_else(|| Some("Nothing in this menu can run now".to_owned()))
        };
        let button = CommandButton {
            unavailable: why.as_deref(),
            shortcut: None,
            ..CommandButton::row(ui, label, label, ButtonStyle::TextRow)
        };
        let response = self.command_button(ui, button);
        self.form_rects
            .insert(format!("menu:{label}"), response.rect);
        let mut run = None;
        egui::Popup::menu(&response).show(|ui| {
            for (control, state) in controls.iter().zip(&states) {
                let Some(line) = control.command.as_deref() else {
                    continue;
                };
                let name = if control.label.is_empty() {
                    line
                } else {
                    control.label.as_str()
                };
                let why = match state {
                    Some(s) => s.unavailable.as_deref(),
                    None => Some("Unknown command"),
                };
                let item = ui.gated(why, Button::new(format!("{name}{}", self.shortcut(line))));
                self.form_rects.insert(line.to_owned(), item.rect);
                if item.clicked() {
                    run = Some(line.to_owned());
                    ui.close();
                }
            }
        });
        if let Some(line) = run {
            self.execute(&line);
        }
        if !tooltip.is_empty() {
            response.on_hover_text(tooltip);
        }
    }

    /// A click: the variant line for the keys held, else the line.
    fn run_form_line(&mut self, ui: &Ui, control: &Control, line: &str) {
        let mods = ui.input(|i| i.modifiers);
        let run = control
            .variants
            .pick(line, mods.command || mods.ctrl, mods.shift, mods.alt)
            .to_owned();
        self.execute(&run);
    }

    /// UIC-221: a button whose line names no command, drawn red; it does
    /// nothing when clicked.
    fn unknown_button(&mut self, ui: &mut Ui, size: Vec2, label: &str, line: &str) -> Rect {
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let red = theme::error();
        ui.painter()
            .rect_stroke(rect, 2., Stroke::new(1.0f32, red), StrokeKind::Inside);
        let label = crate::button_variants::fit_label(
            ui,
            label,
            &crate::typography::standard_font(),
            rect.width() - 12.,
        );
        ui.painter().text(
            rect.left_center() + vec2(8., 0.),
            Align2::LEFT_CENTER,
            label,
            crate::typography::standard_font(),
            red,
        );
        response.on_hover_text(format!(
            "Unknown command: {line}\nThis button does nothing until a command has that name."
        ));
        rect
    }

    /// Revert a form (LAY-87, `operandi.form.revert`).
    pub(super) fn revert_form(&mut self, id: &str) {
        let id = id.trim();
        if self.forms.shipped(id).is_none() && !self.forms.is_changed(id) {
            self.status = format!("No form called {id}");
            return;
        }
        self.workbench_reverted(id);
        self.status = match self.forms.revert(id) {
            Ok(true) => format!("Form {id} is back to its shipped version"),
            Ok(false) => format!("Form {id} has no changes to revert"),
            Err(e) => format!("Couldn't revert form {id}: {e}"),
        };
    }
}
