//! UV editing stage 1 in the app (specifications/uv-editing-2026-11-16):
//! the E7 island buttons (Fit, Flip, Mirror, Axis Align) on the Edit tab's
//! UV group with their option dialogs, and the UV Maps list panel.
//!
//! The buttons are rows of the tool preset table (operation `uv_island`):
//! a plain click runs the row with the remembered values, Shift and Alt
//! pick the rule table's other groupings (UVT-50..53), and Ctrl/Cmd+click
//! opens the rule's dialog first (UVT-08; a build guess for all but Fit).
//! They are text rows until their icons are drawn and approved (UIC-22).
use super::*;
use crate::toolbox::row_height;
use crate::tree_list::{
    self, Cell, ClickHow, Column, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow, TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::tool_preset;
use operandi_core::uv_edit::{
    AlignTo, Click, FitOptions, Grouping, IslandOp, IslandRule, RuleKind, UvAxis, gaps_to_pixels,
    pixels_to_gaps,
};
use operandi_core::uv_maps::{UvMapEdit, current_map};

const UV_AXES: [&str; 2] = ["U", "V"];
const FITS: [&str; 3] = ["Entire UVs", "Per UV Polygon", "Per UV Island"];
const FIT_VALUES: [Grouping; 3] = [Grouping::Entire, Grouping::Polygon, Grouping::Island];
const DIRECTIONS: [&str; 4] = ["Up", "Down", "Left", "Right"];
const ALIGN_TO: [&str; 4] = ["Low", "Average", "Center", "High"];
const ALIGN_VALUES: [AlignTo; 4] = [
    AlignTo::Low,
    AlignTo::Average,
    AlignTo::Center,
    AlignTo::High,
];

/// How an E7 button was clicked: `operandi.uv.preset`'s `click`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IslandClick {
    Plain,
    Shift,
    Alt,
    /// Ctrl/Cmd: the rule's dialog first.
    Options,
}

impl IslandClick {
    pub(super) fn named(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "plain" | "" => Some(Self::Plain),
            "shift" => Some(Self::Shift),
            "alt" => Some(Self::Alt),
            "options" | "ctrl" => Some(Self::Options),
            _ => None,
        }
    }
}

/// The E7 buttons, in toolbox order: (preset id, row of two or one).
pub(super) const ISLAND_ROWS: &[&[&str]] = &[
    &["uv.fitUVs"],
    &["uv.flipU", "uv.flipV"],
    &["uv.mirrorU", "uv.mirrorV"],
    &["uv.alignUp", "uv.alignDown"],
    &["uv.alignLeft", "uv.alignRight"],
];

fn axis_index(a: UvAxis) -> usize {
    match a {
        UvAxis::U => 0,
        UvAxis::V => 1,
    }
}

fn axis_of(i: usize) -> UvAxis {
    if i == 1 { UvAxis::V } else { UvAxis::U }
}

/// The dialog rows of one rule (UVE-1 sections 3, 5, 6; UVT-50..53), filled
/// from the remembered values and the clicked button's axis.
pub(super) fn island_rows(
    kind: RuleKind,
    axis: UvAxis,
    to: Option<AlignTo>,
    memory: &operandi_core::uv_edit::IslandMemory,
) -> Vec<form::Row> {
    use form::{Row, Unit, When};
    let rows = match kind {
        RuleKind::Fit => {
            let f = memory.fit;
            vec![
                Row::choice(
                    "fit",
                    "Fit",
                    &FITS,
                    FIT_VALUES
                        .iter()
                        .position(|g| *g == memory.fit_grouping)
                        .unwrap_or(0),
                )
                .tip("The selection as one block, each polygon on its own, or each island on its own"),
                Row::check("keep_proportion", "Keep Proportion", f.keep_proportion)
                    .tip("Scale evenly until the longer side touches the tile; off stretches to fill"),
                Row::number("gaps", "Gaps", Unit::Suffix(" %"), f.gaps)
                    .range(0., 49.9)
                    .tip("Margin left inside the tile, as a percentage of it (not with Gaps by Pixel)"),
                Row::check("gaps_by_pixel", "Gaps by Pixel", false)
                    .tip("Give the margin in pixels of a 1024 texture instead"),
                Row::number("pixels", "Pixel Size", Unit::Suffix(" px"), gaps_to_pixels(f.gaps))
                    .range(0., 511.)
                    .when(When::Checked("gaps_by_pixel"))
                    .tip("Margin in pixels; 1024 px is the whole tile"),
                Row::number("udim", "UDIM", Unit::Count, f.udim as f64)
                    .range(1001., 1999.)
                    .tip("The tile to fit into: 1001 is 0-1"),
            ]
        }
        RuleKind::Flip => vec![
            Row::choice("axis", "Axis", &UV_AXES, axis_index(axis))
                .tip("U flips left to right, V top to bottom"),
            Row::check("per_polygon", "Per Polygon", memory.flip_per_polygon)
                .tip("Each polygon flips about its own centre"),
        ],
        RuleKind::Mirror => vec![
            Row::choice("axis", "Axis", &UV_AXES, axis_index(axis))
                .tip("U flips left to right, V top to bottom"),
            Row::number("center", "Center", Unit::Scalar, memory.mirror_center)
                .tip("Where the mirror line sits on that axis"),
        ],
        RuleKind::Align => {
            let direction = match (axis, to) {
                (UvAxis::V, Some(AlignTo::High)) => 0,
                (UvAxis::V, _) => 1,
                (UvAxis::U, Some(AlignTo::High)) => 3,
                (UvAxis::U, _) => 2,
            };
            let to = to.unwrap_or(memory.align_to);
            vec![
                Row::choice("direction", "Direction", &DIRECTIONS, direction)
                    .tip("Up and Down move V; Left and Right move U"),
                Row::choice(
                    "align_to",
                    "Align To",
                    &ALIGN_TO,
                    ALIGN_VALUES.iter().position(|v| *v == to).unwrap_or(0),
                )
                .tip("Which value of the selection's box on that axis the UVs move to"),
                Row::check("per_row", "Per Edge Row", memory.align_per_row)
                    .tip("Each run of connected selected edges on its own"),
            ]
        }
    };
    rows.into_iter().map(form::Row::fresh).collect()
}

/// The run a rule's dialog values make.
pub(super) fn island_op(kind: RuleKind, v: &form::Values) -> IslandOp {
    match kind {
        RuleKind::Fit => {
            let gaps = if v.bool("gaps_by_pixel") {
                pixels_to_gaps(v.number("pixels"))
            } else {
                v.number("gaps")
            };
            IslandOp {
                rule: IslandRule::Fit(FitOptions {
                    keep_proportion: v.bool("keep_proportion"),
                    gaps,
                    udim: v.number("udim").round().clamp(1001., 1999.) as u32,
                }),
                grouping: FIT_VALUES[v.choice("fit").min(2)],
            }
        }
        RuleKind::Flip => IslandOp {
            rule: IslandRule::Flip {
                axis: axis_of(v.choice("axis")),
            },
            grouping: if v.bool("per_polygon") {
                Grouping::Polygon
            } else {
                Grouping::Entire
            },
        },
        RuleKind::Mirror => IslandOp {
            rule: IslandRule::Mirror {
                axis: axis_of(v.choice("axis")),
                center: v.number("center"),
            },
            grouping: Grouping::Entire,
        },
        RuleKind::Align => IslandOp {
            rule: IslandRule::Align {
                axis: if v.choice("direction") < 2 {
                    UvAxis::V
                } else {
                    UvAxis::U
                },
                to: ALIGN_VALUES[v.choice("align_to").min(3)],
            },
            grouping: if v.bool("per_row") {
                Grouping::EdgeRow
            } else {
                Grouping::Entire
            },
        },
    }
}

/// Keys of the UV Maps list: the map's place in the mesh's (sorted) list.
type MapKey = usize;

pub(super) fn uv_maps_spec() -> TreeSpec {
    TreeSpec {
        id: "uv-maps-tree",
        columns: vec![Column::name()],
        own_search: false,
        last_fills: false,
    }
}

/// The UV Maps list's right-click menu.
struct MapHooks<'a> {
    app: &'a mut Operandi,
    names: Vec<String>,
}

impl TreeHooks<MapKey> for MapHooks<'_> {
    fn has_row_menu(&self, _key: MapKey) -> bool {
        self.app.editor.session.live.is_none()
    }
    fn row_menu(&mut self, ui: &mut Ui, key: MapKey, rename: &mut dyn FnMut()) {
        self.app.menu_owned_input = true;
        let Some(name) = self.names.get(key).cloned() else {
            return;
        };
        if ui.button("Rename…").clicked() {
            rename();
            ui.close();
        }
        if ui
            .button("Copy")
            .on_hover_text("Copy this whole map's values")
            .clicked()
        {
            self.app.run(Command::UvMap(UvMapEdit::Copy(name.clone())));
            self.app.status = format!("Copied UV map {name}");
            ui.close();
        }
        let copied = self.app.editor.uv_map_clipboard.is_some();
        if ui
            .gated(
                crate::unavailable::unless(copied, "Copy a UV map first"),
                Button::new("Paste"),
            )
            .on_hover_text("Replace this map's values with the copied map's")
            .clicked()
        {
            self.app.run(Command::UvMap(UvMapEdit::Paste(name.clone())));
            ui.close();
        }
        ui.separator();
        let picked: Vec<String> = self
            .app
            .uv_maps_picked
            .iter()
            .filter_map(|&k| self.names.get(k).cloned())
            .collect();
        let several = picked.len() > 1 && picked.contains(&name);
        let label = if several {
            format!("Delete {} Maps", picked.len())
        } else {
            "Delete".to_owned()
        };
        if ui.button(label).clicked() {
            let names = if several { picked } else { vec![name] };
            self.app.run(Command::UvMap(UvMapEdit::Delete(names)));
            self.app.uv_maps_picked.clear();
            ui.close();
        }
    }
}

impl Operandi {
    /// The E7 rows of the Edit tab's UV group.
    pub(super) fn uv_island_buttons(&mut self, ui: &mut Ui) {
        let idle = self.editor.session.live.is_none();
        for row in ISLAND_ROWS {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.;
                let width =
                    (ui.available_width() - 2. * (row.len() as f32 - 1.)) / row.len() as f32;
                for &id in *row {
                    let Some(preset) = tool_preset::row(id) else {
                        continue;
                    };
                    let b = ui.gated(
                        crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                        Button::new(preset.label.as_str()).min_size(vec2(width, row_height())),
                    );
                    self.toolbox_rects.insert(preset.label.clone(), b.rect);
                    if b.clicked() {
                        let mods = ui.input(|i| i.modifiers);
                        self.island_click(id, mods);
                    }
                    b.on_hover_text(format!("{} · {}", preset.label, preset.tooltip));
                }
            });
        }
    }

    /// A click on an E7 button: Ctrl/Cmd opens its dialog (UVT-08), Shift
    /// and Alt pick the other groupings.
    pub(super) fn island_click(&mut self, id: &str, mods: egui::Modifiers) {
        let click = if mods.command || mods.ctrl {
            IslandClick::Options
        } else if mods.shift {
            IslandClick::Shift
        } else if mods.alt {
            IslandClick::Alt
        } else {
            IslandClick::Plain
        };
        self.island_run(id, click);
    }

    /// `operandi.uv.preset <id> <click>`: what a click on an E7 button does.
    pub(super) fn island_run(&mut self, id: &str, click: IslandClick) {
        let Some(row) = tool_preset::row(id) else {
            self.status = format!("No UV button called {id}");
            return;
        };
        let Some(preset) = row.actor.uv else {
            self.status = format!("{id} isn't a UV button");
            return;
        };
        let click = match click {
            IslandClick::Options => {
                self.open_island_dialog(preset.rule, preset.axis.unwrap_or_default(), preset.to);
                return;
            }
            IslandClick::Shift => Click::Shift,
            IslandClick::Alt => Click::Alt,
            IslandClick::Plain => Click::Plain,
        };
        if self.run_ok(Command::UvIslandPreset {
            preset: row.id.clone(),
            click,
        }) {
            self.status = format!("{} done", row.label);
        }
    }

    /// Ctrl/Cmd+click on an E7 button: the rule's dialog.
    pub(super) fn open_island_dialog(&mut self, kind: RuleKind, axis: UvAxis, to: Option<AlignTo>) {
        if self.uv_dialog.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog first".into();
            return;
        }
        let title = match kind {
            RuleKind::Fit => "Fit UVs",
            RuleKind::Flip => "Flip UVs",
            RuleKind::Mirror => "Mirror UVs",
            RuleKind::Align => "Align UVs",
        };
        let rows = island_rows(kind, axis, to, &self.editor.uv_island_memory);
        self.uv_dialog = Some(uv_dialogs::UvDialog::Island(
            self.form_dialog(title, rows),
            kind,
        ));
    }

    /// OK in an E7 dialog: run it and remember the dialog-only values.
    pub(super) fn island_dialog_ok(
        &mut self,
        kind: RuleKind,
        values: &form::Values,
    ) -> std::result::Result<(), String> {
        let op = island_op(kind, values);
        op.validate()?;
        let memory = &mut self.editor.uv_island_memory;
        match (kind, op.rule) {
            (RuleKind::Fit, _) => memory.fit_grouping = op.grouping,
            (RuleKind::Flip, _) => memory.flip_per_polygon = op.grouping == Grouping::Polygon,
            (RuleKind::Align, IslandRule::Align { axis, to }) => {
                memory.align_axis = axis;
                memory.align_to = to;
                memory.align_per_row = op.grouping == Grouping::EdgeRow;
            }
            _ => {}
        }
        if self.run_ok(Command::UvIsland(op)) {
            self.status = format!("{} done", op.label());
        }
        Ok(())
    }

    /// The UV Maps panel (UVE-1 section 1, UVT-01): the active mesh's UV
    /// maps in the shared tree list (UIC-130). Click selects a map (it is
    /// the one the UV view and UV tools use); Ctrl/Shift pick several for
    /// Delete; rename in place (UIC-39) or from the right-click menu, which
    /// also has Copy, Paste and Delete.
    pub(super) fn uv_maps_section(&mut self, ui: &mut Ui, _area: Rect) {
        let idle = self.editor.session.live.is_none();
        let mut state = std::mem::take(&mut self.uv_maps_tree);
        let (names, current) = match self.editor.scene.active() {
            Some(i) => (
                i.mesh.uv_maps.keys().cloned().collect::<Vec<_>>(),
                current_map(&i.mesh),
            ),
            None => (Vec::new(), None),
        };
        let has_mesh = self.editor.scene.active().is_some();
        let mut new_map = false;
        tree_list::tool_row(ui, &mut state, true, |ui, side| {
            if side == tree_list::Side::Left {
                new_map = ui
                    .gated(
                        crate::unavailable::unless(
                            idle && has_mesh,
                            if has_mesh {
                                crate::unavailable::TOOL_ACTIVE
                            } else {
                                "Select a mesh first"
                            },
                        ),
                        Button::new("New Map"),
                    )
                    .on_hover_text(
                        "A new, empty UV map on the active mesh; it becomes the selected map",
                    )
                    .clicked();
            }
        });
        if new_map {
            self.run(Command::UvMap(UvMapEdit::New(None)));
        }
        self.uv_maps_picked.retain(|&k| k < names.len());
        let spec = uv_maps_spec();
        let rows: Vec<TreeRow<MapKey>> = names
            .iter()
            .enumerate()
            .map(|(k, name)| {
                let mut row = TreeRow::new(k, 0, name.clone(), 1);
                row.icon = RowIcon::Icon(crate::icons::Icon::TreeUvMap);
                row.selected = if current.as_deref() == Some(name.as_str()) {
                    Selected::Primary
                } else if self.uv_maps_picked.contains(&k) {
                    Selected::Yes
                } else {
                    Selected::No
                };
                row.renamable = idle;
                row.tip = Some(format!("UV map {name}"));
                row.cells = vec![Cell::Empty];
                row
            })
            .collect();
        let mut editors = std::mem::take(&mut self.text_editors);
        let mut hooks = MapHooks {
            app: self,
            names: names.clone(),
        };
        let events = tree_list::show(ui, &spec, &mut state, &rows, &mut hooks, &mut editors);
        self.text_editors = editors;
        self.uv_maps_tree = state;
        let order: Vec<MapKey> = (0..names.len()).collect();
        for event in events {
            match event {
                TreeEvent::Click { key, how } => {
                    let mut picked = std::mem::take(&mut self.uv_maps_picked);
                    tree_list::click_select(
                        &mut picked,
                        &mut self.uv_maps_tree.anchor,
                        &order,
                        key,
                        how,
                    );
                    self.uv_maps_picked = picked;
                    // The clicked map becomes the selected one, unless a
                    // Ctrl-click just took it out of the pick.
                    let dropped = how == ClickHow::Toggle && !self.uv_maps_picked.contains(&key);
                    if !dropped && let Some(name) = names.get(key) {
                        self.run(Command::UvMap(UvMapEdit::Select(name.clone())));
                    }
                }
                // UVT-200: a double-click on a map opens the UV Editor on it
                // (rename stays click-pause-click or the menu, UIC-39).
                TreeEvent::DoubleClick(key) => {
                    if let Some(name) = names.get(key) {
                        self.run(Command::UvMap(UvMapEdit::Select(name.clone())));
                    }
                    self.open_uv_editor();
                }
                TreeEvent::Rename { key, name } => {
                    if let Some(from) = names.get(key) {
                        self.run(Command::UvMap(UvMapEdit::Rename {
                            from: from.clone(),
                            to: name,
                        }));
                    }
                }
                _ => {}
            }
        }
    }
}
