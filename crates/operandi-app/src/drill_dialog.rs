//! Solid Drill... and Axis Drill... (drills-2026-11-11 DRILL-06..08): the
//! Edit tab's "Booleans & Drill" group opens these dialogs, built with the
//! shared dialog (UIC-98). OK runs `poly.solidDrill` / `poly.drill` once
//! (one undo step, UIC-96); each dialog remembers its values (UIC-95). An
//! open Solid Drill driver gets the Boolean-style notice with Select
//! Problem Edges (DRILL-03).
use super::*;
use operandi_core::{DrillAxis, DrillKind, DrillOp, DrillOptions, DriveMesh};

/// The open drill dialog.
pub(super) struct DrillDialog {
    kind: DrillKind,
    form: form::Dialog,
    /// The Surface Item choice's items, in the choice's order.
    items: Vec<u64>,
}

impl DrillDialog {
    pub(super) fn kind(&self) -> DrillKind {
        self.kind
    }

    /// Rows, top to bottom. Material greys unless Operation is Stencil, and
    /// Surface Item unless Drive Mesh is Specific Surface Item: greyed in
    /// place, not hidden (UIC-92, Q8), each below the choice that controls
    /// it (UIC-93).
    fn rows(kind: DrillKind, o: &DrillOptions, names: &[String], chosen: usize) -> Vec<form::Row> {
        use form::{Row, When};
        let ops: Vec<&str> = DrillOp::DIALOG.iter().map(|o| o.label()).collect();
        let axes: Vec<&str> = DrillAxis::ALL.iter().map(|a| a.label()).collect();
        let drives: Vec<&str> = DriveMesh::ALL.iter().map(|d| d.label()).collect();
        let names: Vec<&str> = if names.is_empty() {
            vec!["(no other mesh items)"]
        } else {
            names.iter().map(String::as_str).collect()
        };
        let op = DrillOp::DIALOG.iter().position(|x| *x == o.op).unwrap_or(0);
        let stencil = DrillOp::DIALOG
            .iter()
            .position(|x| *x == DrillOp::Stencil)
            .unwrap();
        let axis = DrillAxis::ALL
            .iter()
            .position(|x| *x == o.axis)
            .unwrap_or(1);
        let drive = DriveMesh::ALL
            .iter()
            .position(|x| *x == o.drive)
            .unwrap_or(0);
        let mut rows = vec![
            Row::choice("operation", "Operation", &ops, op).fresh().tip(
                "Core: keep only the pieces inside the driver · Stencil: keep everything, cut along the driver, inside pieces get the Material · Tunnel: delete the inside pieces, leaving a hole (no walls are added) · Slice: keep everything, just cut",
            ),
        ];
        if kind == DrillKind::Axis {
            rows.push(
                Row::segments("axis", "Axis", &axes, axis).fresh().tip(
                    "The driver outline is pushed straight through everything along this axis",
                ),
            );
        }
        rows.extend([
            Row::text("material", "Material", &o.material)
                .fresh()
                .when(When::ChoiceIn("operation", vec![stencil]))
                .tip("The material Stencil gives the inside pieces; created if missing (Stencil only)"),
            Row::choice("drive", "Drive Mesh", &drives, drive).fresh().tip(
                "Background: every visible mesh that isn't selected drills the active one · Last Selected: the selected pieces of the active mesh drill the rest · Specific Surface Item: the item chosen below",
            ),
            Row::choice("item", "Surface Item", &names, chosen)
                .fresh()
                .when(When::ChoiceIn("drive", vec![2]))
                .tip("The mesh item, or the group whose meshes, drive the drill (Specific Surface Item only)"),
            Row::check("children", "Include Children", o.children)
                .fresh()
                .when(When::ChoiceIn("drive", vec![0, 2]))
                .tip("Also use the drivers' child mesh items"),
        ]);
        rows
    }

    fn options(&self, base: &DrillOptions) -> DrillOptions {
        let v = &self.form.values;
        DrillOptions {
            op: DrillOp::DIALOG[v.choice("operation").min(3)],
            axis: if self.kind == DrillKind::Axis {
                DrillAxis::ALL[v.choice("axis").min(2)]
            } else {
                base.axis
            },
            material: v.text("material").to_owned(),
            drive: DriveMesh::ALL[v.choice("drive").min(2)],
            item: self.items.get(v.choice("item")).copied(),
            children: v.bool("children"),
        }
    }
}

impl Operandi {
    fn drill_preset(&self, kind: DrillKind) -> DrillOptions {
        match kind {
            DrillKind::Solid => self.editor.solid_drill_preset.clone(),
            DrillKind::Axis => self.editor.axis_drill_preset.clone(),
        }
    }

    /// Solid Drill... / Axis Drill... open their dialog (the Edit tab row,
    /// the menu, the shortcut, the bare command).
    pub(super) fn open_drill(&mut self, kind: DrillKind) {
        if self.drill.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = format!("Close the open dialog before {}", kind.label());
            return;
        }
        let o = self.drill_preset(kind);
        let scene = &self.editor.scene;
        let active = scene.selection.item;
        let (items, names): (Vec<u64>, Vec<String>) = scene
            .items
            .iter()
            .filter(|i| Some(i.id) != active && (i.is_mesh() || i.locator.is_some()))
            .map(|i| (i.id, i.name.clone()))
            .unzip();
        let chosen = o
            .item
            .and_then(|id| items.iter().position(|&i| i == id))
            .unwrap_or(0);
        let rows = DrillDialog::rows(kind, &o, &names, chosen);
        self.drill = Some(DrillDialog {
            kind,
            form: self.form_dialog(kind.label(), rows),
            items,
        });
    }

    /// Run a drill: the result on the status line, or the refusal (a notice
    /// with Select Problem Edges when the driver's geometry is the problem).
    pub(super) fn run_drill(&mut self, kind: DrillKind, o: DrillOptions) {
        let cmd = match kind {
            DrillKind::Solid => Command::SolidDrill(o.clone()),
            DrillKind::Axis => Command::AxisDrill(o.clone()),
        };
        if self.run_ok(cmd) {
            if let Some(note) = self.editor.drill_note.take() {
                self.status = note;
            }
            return;
        }
        if let Some(problem) = self.editor.drill_problem(kind, &o) {
            let message = self.status.clone();
            let form =
                form::Dialog::notice(kind.label(), &message).extras(&["Select Problem Edges"]);
            self.boolean_notice = Some(boolean_dialog::BooleanNotice::new(form, problem));
        }
    }

    /// `poly.solidDrill` / `poly.drill` typed or scripted: bare opens the
    /// dialog (UIC-162); otherwise `mode:` (required), `axis:` (required for
    /// poly.drill), `material:`, `cutmesh:`, `item:` (id or name), `children:`.
    pub(super) fn drill_command(&mut self, kind: DrillKind, args: &[String]) {
        if args.is_empty() {
            self.open_drill(kind);
            return;
        }
        match self.drill_arguments(kind, args) {
            Ok(o) => self.run_drill(kind, o),
            Err(e) => self.status = e,
        }
    }

    fn drill_arguments(
        &self,
        kind: DrillKind,
        args: &[String],
    ) -> std::result::Result<DrillOptions, String> {
        let mut o = DrillOptions::default();
        let names: &[&str] = match kind {
            DrillKind::Solid => &["mode", "material", "cutmesh", "item", "children"],
            DrillKind::Axis => &["mode", "axis", "material", "cutmesh", "item", "children"],
        };
        let command = kind.command();
        let (mut mode, mut axis) = (false, false);
        for (i, raw) in args.iter().enumerate() {
            let (name, value) = match raw.split_once(':') {
                Some((k, v)) if names.iter().any(|n| n.eq_ignore_ascii_case(k)) => {
                    (k.to_ascii_lowercase(), v)
                }
                _ => (
                    names
                        .get(i)
                        .ok_or_else(|| format!("{command} takes {}", names.join(", ")))?
                        .to_string(),
                    raw.as_str(),
                ),
            };
            match name.as_str() {
                "mode" => {
                    o.op = DrillOp::parse(value)?;
                    mode = true;
                }
                "axis" => {
                    o.axis = DrillAxis::parse(value)?;
                    axis = true;
                }
                "material" => o.material = value.to_owned(),
                "cutmesh" => o.drive = DriveMesh::parse(value)?,
                "item" => {
                    let items = &self.editor.scene.items;
                    let id = value
                        .parse::<u64>()
                        .ok()
                        .filter(|id| items.iter().any(|i| i.id == *id))
                        .or_else(|| items.iter().find(|i| i.name == value).map(|i| i.id))
                        .ok_or_else(|| format!("{command}: no item {value}"))?;
                    o.item = Some(id);
                }
                _ => o.children = operandi_core::command_args::boolean("children", value)?,
            }
        }
        if !mode {
            return Err(format!(
                "{command} requires mode (core, tunnel, stencil or slice)"
            ));
        }
        if kind == DrillKind::Axis && !axis {
            return Err("poly.drill requires axis (x, y or z)".into());
        }
        Ok(o)
    }

    /// The open drill dialog, once per frame (its refusal notice is the
    /// Boolean notice).
    pub(super) fn drill_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.drill.take() else {
            return;
        };
        let kind = dialog.kind;
        if self.has_modal() {
            self.status = format!("{} closed; nothing changed", kind.label());
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => {
                self.status = format!("{} canceled; nothing changed", kind.label());
            }
            form::Outcome::Ok => {
                let o = dialog.options(&self.drill_preset(kind));
                if o.drive == DriveMesh::Item && o.item.is_none() {
                    let e = format!("Choose the Surface Item that drives the {}", kind.label());
                    self.status = e.clone();
                    dialog.form.error = Some(e);
                    self.drill = Some(dialog);
                    return;
                }
                // UIC-95: remembered for next time, whatever the outcome.
                match kind {
                    DrillKind::Solid => self.editor.solid_drill_preset = o.clone(),
                    DrillKind::Axis => self.editor.axis_drill_preset = o.clone(),
                }
                self.run_drill(kind, o);
            }
            _ => self.drill = Some(dialog),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Operandi {
        Operandi::with_environment(crate::app_environment::AppEnvironment::from_inputs(
            None,
            None,
            Vec::new(),
            env!("CARGO_MANIFEST_DIR"),
        ))
    }

    /// Target: a 4x4 grid (-2..2, Z 0); driver: cube -1..1; target selected.
    fn reference(a: &mut Operandi) -> (u64, u64) {
        let mut ids = Vec::new();
        a.editor.dispatch(Command::NewMesh).unwrap();
        let item = a.editor.scene.active_mut().unwrap();
        let mut m = operandi_core::Mesh::default();
        for j in 0..=4 {
            for i in 0..=4 {
                m.vertices
                    .push(DVec3::new(i as f64 - 2., j as f64 - 2., 0.));
            }
        }
        for j in 0..4u32 {
            for i in 0..4u32 {
                let at = |i: u32, j: u32| j * 5 + i;
                m.faces.push(operandi_core::Face {
                    vertices: vec![at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                    uv: None,
                    material: "G".into(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
            }
        }
        item.mesh = m.into();
        ids.push(item.id);
        a.editor.dispatch(Command::NewMesh).unwrap();
        let item = a.editor.scene.active_mut().unwrap();
        item.mesh = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
            .unwrap()
            .into();
        ids.push(item.id);
        a.editor
            .dispatch(Command::SelectItem(Some(ids[0])))
            .unwrap();
        (ids[0], ids[1])
    }

    fn faces(a: &Operandi, id: u64) -> usize {
        a.editor.scene.item(id).unwrap().mesh.faces.len()
    }

    #[test]
    fn typed_commands_run_and_bare_opens_the_dialog() {
        let mut a = app();
        let (target, _) = reference(&mut a);
        a.execute("poly.solidDrill");
        assert!(a.drill.is_some(), "bare poly.solidDrill opens the dialog");
        a.drill = None;
        a.execute("poly.drill");
        assert!(a.drill.is_some(), "bare poly.drill opens the dialog");
        a.drill = None;
        a.execute("poly.solidDrill mode:tunnel");
        assert_eq!(faces(&a, target), 12, "{}", a.status);
        assert!(a.status.starts_with("Solid Drill Tunnel"), "{}", a.status);
        a.execute("poly.drill 0");
        assert!(a.status.contains("requires axis"), "{}", a.status);
    }

    #[test]
    fn arguments_take_numbers_words_and_item_names() {
        let mut a = app();
        let (_, driver) = reference(&mut a);
        let item = a.editor.scene.item_mut(driver).unwrap();
        item.name = "Cutter".into();
        let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        let o = a
            .drill_arguments(
                DrillKind::Axis,
                &args("mode:2 axis:z material:Paint cutmesh:item item:Cutter children:true"),
            )
            .unwrap();
        assert_eq!(o.op, DrillOp::Stencil);
        assert_eq!(o.axis, DrillAxis::Z);
        assert_eq!(o.material, "Paint");
        assert_eq!(o.drive, DriveMesh::Item);
        assert_eq!(o.item, Some(driver));
        assert!(o.children);
        let o = a
            .drill_arguments(DrillKind::Axis, &args("slice x"))
            .unwrap();
        assert_eq!((o.op, o.axis), (DrillOp::Slice, DrillAxis::X));
        assert!(
            a.drill_arguments(DrillKind::Solid, &args("mode:9"))
                .is_err()
        );
        assert!(
            a.drill_arguments(DrillKind::Solid, &args("cutmesh:item"))
                .is_err()
        );
    }

    #[test]
    fn dialog_lists_the_documented_order_and_greys_material() {
        // DRILL-07, Q8 (UIC-92).
        let mut a = app();
        reference(&mut a);
        a.open_drill(DrillKind::Axis);
        let d = a.drill.as_ref().unwrap();
        let rows = &d.form.rows;
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            keys,
            ["operation", "axis", "material", "drive", "item", "children"]
        );
        let material = rows.iter().find(|r| r.key == "material").unwrap();
        assert!(!d.form.values.enabled(material), "Core greys Material");
        let o = d.options(&DrillOptions::default());
        assert_eq!(o.op, DrillOp::Core);
        assert_eq!(o.axis, DrillAxis::Y, "Q3");
        assert_eq!(o.material, "Stencil", "Q5");
    }

    #[test]
    fn open_driver_gets_a_notice_with_its_problem_edges() {
        let mut a = app();
        let (_, driver) = reference(&mut a);
        let item = a.editor.scene.item_mut(driver).unwrap();
        let mut m: operandi_core::Mesh = (*item.mesh).clone();
        m.faces.pop();
        item.mesh = m.into();
        let before = a.editor.scene.items.clone();
        a.run_drill(
            DrillKind::Solid,
            DrillOptions {
                op: DrillOp::Tunnel,
                ..Default::default()
            },
        );
        assert_eq!(a.editor.scene.items, before, "nothing changed");
        assert!(a.boolean_notice.is_some(), "a notice");
        assert!(a.status.contains("4 open edges"), "{}", a.status);
    }
}
