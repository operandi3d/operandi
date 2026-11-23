//! The Mesh Ops tab (MeshOps v1, MO-10..28): the selected mesh's
//! operation stack, top = last evaluated, with its Base Mesh at the bottom,
//! and the selected op's Properties under it. Every change goes through a
//! command (meshops_editor.rs), so buttons, menus and scripts agree.
//!
//! Provisional for v1: the list shows only the selected mesh (Single item
//! view, MO-28); Add Operator is a menu, not the preset browser (MO-15); no
//! Ghosted/Hide/Filter header buttons yet (MO-25), no Defer/Pause (MO-29).
use super::*;
use crate::tree_list::{
    Cell, Column, DropAt, Heading, RowIcon, Selected, TreeEvent, TreeHooks, TreeRow, TreeSpec,
};
use crate::unavailable::Gate;
use operandi_core::meshop_selection::{Blend, SelKind, SelOp};
use operandi_core::meshop_types::FieldKind;
use operandi_core::meshops::{OpKind, OpStatus, op_defs};
use operandi_core::meshops_editor::{IndexEdit, OpRef, OpRow, SelOpEdit};

/// The Properties panel's one category while an op is selected.
pub(crate) const MESH_OP_TAB: &str = "Mesh Op";

/// A row of the Mesh Operations list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum OpKey {
    Op(u64, u64),
    Base(u64),
}

/// The ghost of the full result in Edit Base Mesh (MO-26/27). Provisional
/// fixed tint; the spec makes it a preference.
pub(crate) fn ghost_color() -> Color32 {
    Color32::from_rgba_unmultiplied(110, 190, 255, 110)
}

fn spec() -> TreeSpec {
    TreeSpec {
        id: "mesh-ops-tree",
        columns: vec![
            Column::state(
                Heading::Eye,
                "On or off · off passes its input straight through",
            ),
            Column::name(),
        ],
        own_search: false,
        last_fills: false,
    }
}

/// Choices for an item link: "(none)", the other meshes, and the linked
/// id itself when it names no mesh any more (SAFE-08 "missing input").
fn item_options(meshes: &[(u64, String)], current: u64) -> Vec<(u64, &str)> {
    let mut out: Vec<(u64, &str)> = vec![(0, "(none)")];
    out.extend(meshes.iter().map(|(id, n)| (*id, n.as_str())));
    if current != 0 && !meshes.iter().any(|(id, _)| *id == current) {
        out.push((current, "(missing item)"));
    }
    out
}

/// The op's selection ops (MO-50..53) under its Select by Index: each
/// with its eye, Invert, Blend Mode and own values, a Remove button, and
/// an Add Selection menu. Returns the one change made this frame.
fn selection_ops(
    ui: &mut Ui,
    op: &operandi_core::meshops::MeshOp,
    below: &[operandi_core::meshops::MeshOp],
) -> (Option<SelOpEdit>, Option<egui::Rect>) {
    use crate::form::{Unit, rows};
    let mut edit = None;
    for (n, s) in op.selops.iter().enumerate() {
        let mut next = s.clone();
        let mut remove = false;
        ui.push_id(("selop", n), |ui| {
            rows::value(ui, &format!("{}. {}", n + 1, s.kind.label()), "");
            rows::check_d(
                ui,
                "Enable",
                &mut next.enable,
                true,
                "Off: as if it weren't there",
            );
            rows::check_d(
                ui,
                "Invert",
                &mut next.invert,
                false,
                "Pick what it doesn't",
            );
            let blends: Vec<(Blend, &str)> = Blend::ALL.iter().map(|b| (*b, b.label())).collect();
            rows::choice_d(
                ui,
                "Blend Mode",
                &mut next.blend,
                &Blend::Override,
                &blends,
                "How it combines with the selection ops below it",
            );
            match &mut next.kind {
                SelKind::Index { index } => {
                    rows::text_d(ui, "Index", index, "", "Polygon numbers, like 0-9,12");
                }
                SelKind::Range { start, end, step } => {
                    rows::number_d(
                        ui,
                        "Start",
                        start,
                        0,
                        Unit::Count,
                        0..=u32::MAX,
                        "First polygon",
                    );
                    rows::number_d(
                        ui,
                        "End",
                        end,
                        0,
                        Unit::Count,
                        0..=u32::MAX,
                        "Last polygon; 0 = the last one",
                    );
                    rows::number_d(
                        ui,
                        "Step",
                        step,
                        1,
                        Unit::Count,
                        1..=u32::MAX,
                        "Every nth polygon",
                    );
                }
                SelKind::Pattern { pattern } => {
                    rows::text_d(
                        ui,
                        "Pattern",
                        pattern,
                        "1,0",
                        "Repeating 1/0 list: 1,0 = every other",
                    );
                }
                SelKind::Previous { source, name } => {
                    let mut sources: Vec<(u64, String)> = vec![(0, "(none)".into())];
                    sources.extend(below.iter().map(|o| (o.id, o.kind.label())));
                    let options: Vec<(u64, &str)> =
                        sources.iter().map(|(id, l)| (*id, l.as_str())).collect();
                    rows::choice_d(
                        ui,
                        "Source Item",
                        source,
                        &0,
                        &options,
                        "An operation lower in this stack",
                    );
                    // PG-03: show labels, store the short ids.
                    let groups = below
                        .iter()
                        .find(|o| o.id == *source)
                        .and_then(|o| o.kind.def())
                        .map(|d| d.groups())
                        .unwrap_or_default();
                    let mut names: Vec<(String, String)> = vec![(String::new(), "(none)".into())];
                    names.extend(groups);
                    if !names.iter().any(|(id, _)| id == name) {
                        names.push((name.clone(), name.clone()));
                    }
                    let options: Vec<(String, &str)> = names
                        .iter()
                        .map(|(id, l)| (id.clone(), l.as_str()))
                        .collect();
                    rows::choice_d(
                        ui,
                        "Name",
                        name,
                        &String::new(),
                        &options,
                        "The group it picks",
                    );
                }
                SelKind::SelectionSet { name } => {
                    rows::text_d(
                        ui,
                        "Selection Set",
                        name,
                        "",
                        "A polygon selection set on the mesh below",
                    );
                }
            }
            remove = rows::button(ui, "Remove").clicked();
        });
        if remove {
            edit = Some(SelOpEdit::Remove(n));
        } else if next != *s {
            edit = Some(SelOpEdit::Set(n, next));
        }
    }
    let b = ui.button("Add Selection…");
    (edit, b.clicked().then_some(b.rect))
}

pub(crate) const ADD_OPERATOR: &str = "add-operator";
pub(crate) const ADD_SELECTION: &str = "add-selection";

/// Add Operator's contents (AB §5): every op type in its folder under
/// Mesh Operations. Unbuilt ops aren't in the table, and folders without
/// ops don't show (AB-41).
pub(crate) fn operator_catalog() -> crate::add_browser::Catalog {
    let mut entries = Vec::new();
    for d in op_defs() {
        let mut folder = vec!["Mesh Operations".to_string()];
        folder.extend(d.folder().split('/').map(str::to_owned));
        entries.push(crate::add_browser::Entry {
            id: d.id().into(),
            label: d.label().into(),
            folder,
            icon: op_icon(d.id()),
        });
    }
    crate::add_browser::Catalog {
        roots: vec![
            "Mesh Operations".into(),
            "Deformers".into(),
            "Mesh Operation Assemblies".into(),
        ],
        entries,
        plain: false,
    }
}
/// An op type's icon in the Add browser: its direct tool's icon from the
/// generator where there is one; `None` = the default cube.
pub(crate) fn op_icon(id: &str) -> Option<crate::icons::Icon> {
    let slug = match id {
        "prim.cube.item" => "cube",
        "operandi.skin.item" => "skin-mesh",
        "prim.sphere.item" => "sphere",
        "prim.cylinder.item" => "cylinder",
        "poly.bevel.item" => "bevel",
        "poly.smshift.item" => "smooth-shift",
        "subdivide.tool.item" => "subdivide-smooth",
        "pmodel.triangulate.item" => "triple",
        "mirror.clone.item" => "mirror",
        "array.clone.item" => "array",
        "radial.array.item" => "radial-array",
        "linear.clone.item" => "clone",
        "boolean.meshop.item" | "solidDrill.meshop.item" | "axisDrill.meshop.item" => "boolean",
        "pmodel.meshmerge" | "pmodel.mergePolygons.item" => "merge",
        "poly.extrude.item" => "extrude",
        "symmetrize.meshop.item" => "symmetry",
        "detriangulate.meshop.item" => "detriangulate",
        "poly.unsubdiv.item" => "unsubdivide",
        "pmodel.materialTag.item" => "tag-material",
        "poly.spikey.item" => "spike",
        "edge.bevel.item" => "edge-bevel",
        "edge.chamfer.item" => "edge-chamfer",
        "edge.extrude.item" => "edge-extrude",
        "edge.relax.item" => "edge-relax",
        "pmodel.edge.subdivide" => "edge-add-point",
        "pmodel.splitEdges.item" => "edge",
        // Curve tools and batch 5 (provisional: borrowed icons until their own).
        "curve.circle.item"
        | "curve.ellipse.item"
        | "curve.rectangle.item"
        | "curve.diamond.item"
        | "curve.nsided.item"
        | "curve.star.item" => "bezier-curve",
        "curve.extrude.item" => "sketch-extrude",
        "curve.clone.item" | "pen.clone.item" | "scatter.clone.item" => "clone",
        "surfaceConstraint.meshop.item" => "pen",
        "uvrect.meshop.item" | "uvfit.meshop.item" | "vertexmaptransfer" => "tree-uv-map",
        "vert.bevel.item" => "vertex-bevel",
        "vert.extrude.item" => "vertex-extrude",
        "pmodel.createVertex" => "vertex-create",
        "vert.merge.item" => "vertex-merge",
        "vert.split.meshop.item" => "vertex-tear",
        "vert.setpos.meshop.item" => "vertex",
        "poly.knife.item" => "polygon-slice",
        "poly.julienne.item" => "julienne",
        "poly.reduct.item" => "poly-reduct",
        "xfrm.jitter.item" => "jitter",
        "xfrm.quantize.item" => "quantize",
        "xfrm.radialAlign.item" => "radial-align",
        "xfrm.linearAlign.item" => "linear-align",
        "xfrm.push.item" => "push",
        "smooth.meshop.item" => "smooth",
        "uv.unwrap.item" | "uvpack.meshop.item" | "pmodel.transformMap" => "tree-uv-map",
        _ => return None,
    };
    crate::icons::Icon::from_slug(slug)
}
/// Add Selection Operation's contents: the selection ops, a plain list.
fn selection_catalog() -> crate::add_browser::Catalog {
    crate::add_browser::Catalog {
        roots: vec!["Selection".into()],
        entries: SelKind::menu()
            .into_iter()
            .map(|(label, _)| crate::add_browser::Entry {
                id: label.into(),
                label: label.into(),
                folder: vec!["Selection".into()],
                icon: Some(crate::icons::Icon::TagSelectionSet),
            })
            .collect(),
        plain: true,
    }
}

impl Operandi {
    /// Draws the Add browser `key` while it's open; the id of what was
    /// added, if anything.
    pub(crate) fn show_add_browser(
        &mut self,
        ctx: &egui::Context,
        key: &'static str,
        title: &str,
        catalog: crate::add_browser::Catalog,
    ) -> Option<String> {
        let (state, anchor) = self.add_browsers.get_mut(key)?;
        let out = crate::add_browser::show(ctx, key, title, *anchor, &catalog, state);
        if state.open {
            self.menu_owned_input = true;
        }
        out.added
    }
}

/// A value's conversion between stored and shown.
type Convert = fn(f64) -> f64;

/// Right-click menus of the list.
struct Hooks {
    idle: bool,
    picked: Option<MenuPick>,
}
#[derive(Clone, Copy)]
enum MenuPick {
    Freeze(OpRef, bool),
    Delete(OpRef),
    EditBase(u64, bool),
}
impl TreeHooks<OpKey> for Hooks {
    fn has_row_menu(&self, _key: OpKey) -> bool {
        self.idle
    }
    fn row_menu(&mut self, ui: &mut Ui, key: OpKey, _rename: &mut dyn FnMut()) {
        match key {
            OpKey::Op(item, op) => {
                let r = OpRef { item, op };
                if ui
                    .button("Freeze")
                    .on_hover_text("Bake this op and those below it into the Base Mesh")
                    .clicked()
                {
                    self.picked = Some(MenuPick::Freeze(r, false));
                    ui.close();
                }
                if ui
                    .button("Duplicate and Freeze")
                    .on_hover_text("A new mesh item with this op and those below it baked in")
                    .clicked()
                {
                    self.picked = Some(MenuPick::Freeze(r, true));
                    ui.close();
                }
                ui.separator();
                if ui.button("Delete").clicked() {
                    self.picked = Some(MenuPick::Delete(r));
                    ui.close();
                }
            }
            OpKey::Base(item) => {
                if ui
                    .button("Edit Base Mesh")
                    .on_hover_text("Direct tools edit the Base Mesh; the result shows as a ghost")
                    .clicked()
                {
                    self.picked = Some(MenuPick::EditBase(item, true));
                    ui.close();
                }
            }
        }
    }
}

impl Operandi {
    /// The Mesh Ops panel body (registered in panel_registry.rs).
    pub(super) fn mesh_ops_section(&mut self, ui: &mut Ui, _area: Rect) {
        let idle = self.editor.live.is_none();
        let Some(item) = self.editor.scene.active().filter(|i| i.is_mesh()).cloned() else {
            ui.add_space(6.);
            crate::form::rows::note(ui, "Select a mesh item to see its Mesh Stack.");
            return;
        };
        let mut state = std::mem::take(&mut self.mesh_ops_tree);
        let mut add: Option<String> = None;
        let mut add_open: Option<egui::Rect> = None;
        let editing_base = self.editor.base_edit == Some(item.id);
        let mut base_toggle = None;
        tree_list::tool_row(ui, &mut state, false, |ui, side| match side {
            tree_list::Side::Left => {
                ui.gated_ui(
                    crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                    |ui| {
                        let b = ui.button("Add Operator…").on_hover_text(
                            "Add a mesh operation above the selected row (on top when none is \
                             selected); polygons selected on the Base Mesh become its Select by Index",
                        );
                        if b.clicked() {
                            add_open = Some(b.rect);
                        }
                    },
                );
            }
            tree_list::Side::Right => {
                if item.mesh_ops.is_some() {
                    let r = tree_list::toggle_button(ui, "Base", editing_base).on_hover_text(
                        if editing_base {
                            "Stop editing the Base Mesh"
                        } else {
                            "Edit Base Mesh: direct tools edit the Base Mesh, with the result \
                             drawn as a ghost"
                        },
                    );
                    if r.clicked() {
                        base_toggle = Some(!editing_base);
                    }
                }
            }
        });
        let evaluated = self.editor.meshops.result(&self.editor.scene, &item);
        let ops = item.mesh_ops.as_ref().map_or(&[][..], |s| &s.ops[..]);
        let row_sel = self.editor.meshop_row.filter(|r| r.item == item.id);
        let mut rows = Vec::new();
        for (i, op) in ops.iter().enumerate().rev() {
            let mut row = TreeRow::new(OpKey::Op(item.id, op.id), 0, op.kind.label(), 2);
            row.icon = RowIcon::Icon(crate::icons::Icon::TreeMesh);
            row.draggable = idle;
            row.selected = if row_sel
                == Some(OpRow {
                    item: item.id,
                    op: Some(op.id),
                }) {
                Selected::Primary
            } else {
                Selected::No
            };
            row.style.dimmed = !op.enabled;
            let status = evaluated.as_ref().and_then(|e| e.statuses.get(i));
            if let Some(OpStatus::Failed(reason)) = status {
                row.tint = Some(theme::error());
                row.tip = Some(format!("Failed, passed through: {reason}"));
            } else if op.kind.takes_selection() && op.selection.is_some() {
                row.tip = Some("Works on its Select by Index".into());
            }
            row.cells[0] = Cell::Toggle {
                on: op.enabled,
                tip: "On or off".into(),
                dimmed: false,
            };
            rows.push(row);
        }
        let mut base = TreeRow::new(OpKey::Base(item.id), 0, "Base Mesh", 2);
        base.icon = RowIcon::Icon(icons::Icon::Cube);
        base.fixed = true;
        base.style.italic = editing_base;
        base.selected = if row_sel
            == Some(OpRow {
                item: item.id,
                op: None,
            }) {
            Selected::Primary
        } else {
            Selected::No
        };
        base.tip = Some(format!(
            "{} polygons{}",
            item.mesh.faces.len(),
            if editing_base { " · being edited" } else { "" }
        ));
        rows.push(base);
        let mut hooks = Hooks { idle, picked: None };
        let mut editors = std::mem::take(&mut self.text_editors);
        let events = tree_list::show(ui, &spec(), &mut state, &rows, &mut hooks, &mut editors);
        self.text_editors = editors;
        state.retain(|k| rows.iter().any(|r| r.key == k));
        self.mesh_ops_tree = state;

        // AB-01: Add Operator opens the Add browser.
        if let Some(rect) = add_open {
            let entry = self
                .add_browsers
                .entry(ADD_OPERATOR)
                .or_insert_with(|| (Default::default(), egui::Rect::NOTHING));
            entry.0.open = !entry.0.open;
            entry.1 = rect;
        }
        if let Some(id) = self.show_add_browser(
            ui.ctx(),
            ADD_OPERATOR,
            "Add Mesh Operation",
            operator_catalog(),
        ) {
            add = Some(id);
        }
        if let Some(t) = add {
            self.run(Command::MeshOpCreate(t));
        }
        if let Some(on) = base_toggle {
            self.run(Command::MeshOpEditBase { item: item.id, on });
        }
        match hooks.picked {
            Some(MenuPick::Freeze(op, duplicate)) => {
                self.run(Command::MeshOpFreeze { op, duplicate })
            }
            Some(MenuPick::Delete(r)) => self.run(Command::MeshOpDelete(vec![r])),
            Some(MenuPick::EditBase(id, on)) => self.run(Command::MeshOpEditBase { item: id, on }),
            None => {}
        }
        for event in events {
            match event {
                TreeEvent::Click { key, how: _ } | TreeEvent::DoubleClick(key) => {
                    let row = match key {
                        OpKey::Op(item, op) => OpRow { item, op: Some(op) },
                        OpKey::Base(item) => OpRow { item, op: None },
                    };
                    self.run(Command::MeshOpSelect(Some(row)));
                }
                TreeEvent::Toggle {
                    key: OpKey::Op(item, op),
                    ..
                } => {
                    let on = ops.iter().find(|o| o.id == op).is_some_and(|o| o.enabled);
                    self.run(Command::MeshOpEnable {
                        op: OpRef { item, op },
                        on: !on,
                    });
                }
                TreeEvent::Drop { keys, at, .. } => {
                    // Rows show top = last evaluated, so "before" a row in the
                    // list is above it in the stack (MO-21).
                    let index_of = |op: u64| ops.iter().position(|o| o.id == op);
                    let target = match at {
                        DropAt::Before(OpKey::Op(_, o)) | DropAt::On(OpKey::Op(_, o)) => {
                            index_of(o).map(|i| i + 1)
                        }
                        DropAt::After(OpKey::Op(_, o)) => index_of(o),
                        DropAt::Before(OpKey::Base(_)) | DropAt::End => Some(0),
                        _ => None,
                    };
                    if let (Some(mut to), [OpKey::Op(item, op)]) = (target, keys.as_slice())
                        && let Some(from) = index_of(*op)
                    {
                        if to > from {
                            to -= 1;
                        }
                        self.run(Command::MeshOpMove {
                            op: OpRef {
                                item: *item,
                                op: *op,
                            },
                            position: to,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// The active mesh and its selected op row, when an op is selected.
    pub(super) fn selected_mesh_op(&self) -> Option<(operandi_core::scene::Item, u64)> {
        let item = self.editor.scene.active().filter(|i| i.is_mesh())?;
        let row = self.editor.meshop_row.filter(|r| r.item == item.id)?;
        Some((item.clone(), row.op?))
    }

    /// The Properties panel body for the selected op.
    pub(super) fn mesh_op_properties_panel(&mut self, ui: &mut Ui) {
        let Some((item, _)) = self.selected_mesh_op() else {
            return;
        };
        let evaluated = self.editor.meshops.result(&self.editor.scene, &item);
        self.mesh_op_properties(ui, &item, evaluated.as_ref());
    }

    /// The selected op's Properties (MO-19). A drag is one undo step.
    fn mesh_op_properties(
        &mut self,
        ui: &mut Ui,
        item: &operandi_core::scene::Item,
        evaluated: Option<&operandi_core::meshops::Evaluated>,
    ) {
        use crate::form::{Unit, rows};
        let Some(OpRow {
            op: Some(op_id), ..
        }) = self.editor.meshop_row.filter(|r| r.item == item.id)
        else {
            if self.editor.base_edit == Some(item.id) {
                rows::note(
                    ui,
                    "Editing the Base Mesh: direct tools change it and the stack re-runs on top. \
                     The full result is drawn as a ghost.",
                );
            }
            return;
        };
        let Some(stack) = item.mesh_ops.as_ref() else {
            return;
        };
        let Some(index) = stack.index_of(op_id) else {
            return;
        };
        let op = &stack.ops[index];
        let r = OpRef {
            item: item.id,
            op: op_id,
        };
        if let Some(OpStatus::Failed(reason)) = evaluated.and_then(|e| e.statuses.get(index)) {
            ui.colored_label(theme::error(), format!("Failed, passed through: {reason}"));
        }
        if let OpKind::Placeholder { reason, .. } = &op.kind {
            rows::note(ui, reason);
            return;
        }
        let Some(def) = op.kind.def() else {
            return;
        };
        let values = op.kind.values();
        let defaults = def.defaults();
        // Other items a link can name (SAFE-07 refuses loops when set).
        let meshes: Vec<(u64, String)> = self
            .editor
            .scene
            .items
            .iter()
            // Group locators too: a group links all its meshes (Boolean).
            .filter(|i| (i.is_mesh() || i.is_group_locator()) && i.kit.is_none() && i.id != item.id)
            .map(|i| (i.id, i.name.clone()))
            .collect();
        let mut changes = serde_json::Map::new();
        let mut index_edit = None;
        let mut selop_edit = None;
        let mut selop_open: Option<egui::Rect> = None;
        let mut dragged = false;
        rows::scope(ui, "mesh-op-properties", |ui| {
            rows::section(ui, &op.kind.label(), false, |ui| {
                // The form comes from the op's type (KIT-4: kits describe
                // their forms, the app draws them).
                for f in def.fields() {
                    if f.kind == FieldKind::Hidden {
                        continue;
                    }
                    let key: &str = &f.key;
                    let label: &str = &f.label;
                    let value = values.get(key).cloned().unwrap_or_default();
                    let default = defaults.get(key).cloned().unwrap_or_default();
                    let tip = format!("{label} ({key})");
                    let response = match &f.kind {
                        FieldKind::Check => {
                            let mut on = value.as_bool().unwrap_or(false);
                            let d = default.as_bool().unwrap_or(false);
                            let r = rows::check_d(ui, label, &mut on, d, &tip);
                            if r.changed() {
                                changes.insert(key.into(), on.into());
                            }
                            r
                        }
                        FieldKind::Choice(names) => {
                            let mut i = value.as_u64().unwrap_or(0) as usize;
                            let d = default.as_u64().unwrap_or(0) as usize;
                            let options: Vec<(usize, &str)> = names
                                .iter()
                                .enumerate()
                                .map(|(i, n)| (i, n.as_ref()))
                                .collect();
                            let r = rows::choice_d(ui, label, &mut i, &d, &options, &tip);
                            if r.changed() {
                                changes.insert(key.into(), (i as u64).into());
                            }
                            r
                        }
                        FieldKind::Count { min, max } => {
                            let mut n = value.as_u64().unwrap_or(0) as u32;
                            let d = default.as_u64().unwrap_or(0) as u32;
                            let r = rows::number_d(
                                ui,
                                label,
                                &mut n,
                                d,
                                Unit::Count,
                                *min..=*max,
                                &tip,
                            );
                            if r.changed() {
                                changes.insert(key.into(), n.into());
                            }
                            r
                        }
                        FieldKind::Text => {
                            let mut t = value.as_str().unwrap_or_default().to_string();
                            let d = default.as_str().unwrap_or_default();
                            let r = rows::text_d(ui, label, &mut t, d, &tip);
                            if r.changed() {
                                changes.insert(key.into(), t.into());
                            }
                            r
                        }
                        FieldKind::Item => {
                            let mut id = value.as_u64().unwrap_or(0);
                            let options = item_options(&meshes, id);
                            let r = rows::choice_d(ui, label, &mut id, &0, &options, &tip);
                            if r.changed() {
                                changes.insert(key.into(), id.into());
                            }
                            r
                        }
                        FieldKind::Items => {
                            // One row per linked item ("(none)" removes it)
                            // and an Add row.
                            let mut ids: Vec<u64> = value
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|v| v.as_u64())
                                .collect();
                            let mut changed = None;
                            for (n, id) in ids.clone().iter().enumerate() {
                                let mut pick = *id;
                                let options = item_options(&meshes, pick);
                                let row = format!("{label} {}", n + 1);
                                let r = rows::choice_d(ui, &row, &mut pick, id, &options, &tip);
                                if r.changed() {
                                    ids[n] = pick;
                                }
                                changed =
                                    Some(changed.map_or(r.clone(), |c: egui::Response| c | r));
                            }
                            let mut pick = 0u64;
                            let options = item_options(&meshes, 0);
                            let r = rows::choice_d(
                                ui,
                                &format!("Add {label}"),
                                &mut pick,
                                &0,
                                &options,
                                &tip,
                            );
                            if pick != 0 {
                                ids.push(pick);
                            }
                            let r = changed.map_or(r.clone(), |c| c | r);
                            ids.retain(|&id| id != 0);
                            let before: Vec<u64> = value
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|v| v.as_u64())
                                .collect();
                            if ids != before {
                                changes.insert(key.into(), ids.into());
                            }
                            r
                        }
                        FieldKind::Hidden => continue,
                        FieldKind::Number
                        | FieldKind::Distance
                        | FieldKind::Angle
                        | FieldKind::Percent => {
                            let (unit, shown, back): (Unit, Convert, Convert) = match f.kind {
                                FieldKind::Angle => (Unit::Angle, f64::to_degrees, f64::to_radians),
                                FieldKind::Percent => (Unit::Percent, |v| v * 100., |v| v / 100.),
                                FieldKind::Number => (Unit::Scalar, |v| v, |v| v),
                                _ => (Unit::Distance, |v| v, |v| v),
                            };
                            let mut x = shown(value.as_f64().unwrap_or(0.));
                            let d = shown(default.as_f64().unwrap_or(0.));
                            let r = rows::number_d(ui, label, &mut x, d, unit, -1e9..=1e9, &tip);
                            if r.changed() {
                                changes.insert(key.into(), back(x).into());
                            }
                            r
                        }
                    };
                    dragged |= response.dragged();
                }
            });
            if op.kind.takes_selection() {
                rows::section(ui, "Selection", false, |ui| {
                    let text = op
                        .selection
                        .as_ref()
                        .map_or_else(|| "Whole mesh".to_string(), |s| s.text());
                    rows::value(ui, "Select by Index", &text);
                    for (label, edit, tip) in [
                        (
                            "Set",
                            IndexEdit::Set,
                            "Use the polygons selected on the Base Mesh",
                        ),
                        (
                            "Add",
                            IndexEdit::Add,
                            "Add the polygons selected on the Base Mesh",
                        ),
                        (
                            "Remove",
                            IndexEdit::Remove,
                            "Take the selected polygons out",
                        ),
                        ("Clear", IndexEdit::Clear, "Work on the whole mesh"),
                    ] {
                        if rows::button(ui, label).on_hover_text(tip).clicked() {
                            index_edit = Some(edit);
                        }
                    }
                    rows::note(
                        ui,
                        "Polygon numbers can point at other polygons once an op below changes \
                         the topology. Select polygons in Edit Base Mesh to set them.",
                    );
                    let (edit, open) = selection_ops(ui, op, &stack.ops[..index]);
                    selop_edit = edit;
                    selop_open = open;
                });
            }
        });
        if let Some(edit) = index_edit {
            self.run(Command::MeshOpIndex { op: r, edit });
        }
        if let Some(rect) = selop_open {
            let entry = self
                .add_browsers
                .entry(ADD_SELECTION)
                .or_insert_with(|| (Default::default(), egui::Rect::NOTHING));
            entry.0.open = !entry.0.open;
            entry.1 = rect;
        }
        // AB-01: Add Selection Operation, a plain filtered list.
        if let Some(id) = self.show_add_browser(
            ui.ctx(),
            ADD_SELECTION,
            "Add Selection Operation",
            selection_catalog(),
        ) && let Some((_, kind)) = SelKind::menu().into_iter().find(|(l, _)| *l == id)
        {
            selop_edit = Some(SelOpEdit::Add(SelOp::new(kind)));
        }
        if let Some(edit) = selop_edit {
            self.run(Command::MeshOpSelOp { op: r, edit });
        }
        if changes.is_empty() {
            if !ui.input(|i| i.pointer.any_down()) {
                self.mesh_ops_gesture = None;
            }
            return;
        }
        // One undo step per drag (SAFE-13).
        let dragging = dragged || ui.input(|i| i.pointer.any_down());
        let gesture = match self.mesh_ops_gesture {
            Some(g) if dragging => g,
            _ => crate::colour_field::next_gesture(),
        };
        self.mesh_ops_gesture = dragging.then_some(gesture);
        let previous = self.editor.undo_fold.replace(gesture);
        self.run(Command::MeshOpSet {
            op: r,
            values: changes,
        });
        self.editor.undo_fold = previous;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_op_value_has_a_form_field() {
        for d in op_defs() {
            let values = d.defaults();
            for key in values.as_object().unwrap().keys() {
                assert!(
                    d.fields().iter().any(|f| f.key == key.as_str()),
                    "{}: {key} has no form field",
                    d.id()
                );
            }
        }
    }
}
