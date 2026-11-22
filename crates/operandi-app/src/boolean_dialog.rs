//! Boolean... (booleans/BOOLEANS.md BOOL-23): the Edit tab's "Booleans &
//! Drill" group opens this dialog, built with the shared dialog
//! (UIC-80..99). OK runs `poly.boolean` once (one undo step, UIC-96);
//! the dialog remembers its values (UIC-95). A refused input (BOOL-19/21)
//! gets a notice naming the item and the problem, with Select Problem
//! Edges.
use super::*;
use operandi_core::{BooleanOp, BooleanOptions, BooleanProblem, DriveMesh};

/// The open Boolean dialog.
pub(super) struct BooleanDialog {
    form: form::Dialog,
    /// The Surface Item choice's items, in the choice's order.
    items: Vec<u64>,
}

/// The refusal notice (BOOL-19): its message and the edges to select.
pub(super) struct BooleanNotice {
    form: form::Dialog,
    problem: BooleanProblem,
}

impl BooleanNotice {
    /// The same notice for Solid Drill's driver (DRILL-03).
    pub(super) fn new(form: form::Dialog, problem: BooleanProblem) -> Self {
        Self { form, problem }
    }
}

impl BooleanDialog {
    /// BOOL-23 rows, top to bottom. Surface Item and Include Children grey
    /// in place when the Drive Mesh choice doesn't use them (UIC-92/93).
    fn rows(o: &BooleanOptions, names: &[String], chosen: usize) -> Vec<form::Row> {
        use form::{Row, When};
        let ops: Vec<&str> = BooleanOp::ALL.iter().map(|o| o.label()).collect();
        let drives: Vec<&str> = DriveMesh::ALL.iter().map(|d| d.label()).collect();
        let names: Vec<&str> = if names.is_empty() {
            vec!["(no other mesh items)"]
        } else {
            names.iter().map(String::as_str).collect()
        };
        let op = BooleanOp::ALL.iter().position(|x| *x == o.op).unwrap_or(0);
        let drive = DriveMesh::ALL
            .iter()
            .position(|x| *x == o.drive)
            .unwrap_or(0);
        vec![
            Row::choice("operation", "Operation", &ops, op).fresh().tip(
                "Union: the outside of both · Intersect: only where they overlap · Subtract: the active mesh minus the drivers · Add: both surfaces kept and stitched where they cross",
            ),
            Row::choice("drive", "Drive Mesh", &drives, drive).fresh().tip(
                "Background: every visible mesh that isn't selected cuts the active one (hide a mesh to leave it out) · Last Selected: the selected pieces of the active mesh cut the rest · Specific Surface Item: the item chosen below",
            ),
            Row::choice("item", "Surface Item", &names, chosen)
                .fresh()
                .when(When::ChoiceIn("drive", vec![2]))
                .tip("The mesh item, or the group whose meshes, drive the boolean (Specific Surface Item only)"),
            Row::check("children", "Include Children", o.children)
                .fresh()
                .when(When::ChoiceIn("drive", vec![0, 2]))
                .tip("Also use the drivers' child mesh items"),
        ]
    }

    fn options(&self) -> BooleanOptions {
        let v = &self.form.values;
        BooleanOptions {
            op: BooleanOp::ALL[v.choice("operation").min(3)],
            drive: DriveMesh::ALL[v.choice("drive").min(2)],
            item: self.items.get(v.choice("item")).copied(),
            children: v.bool("children"),
        }
    }
}

impl Operandi {
    /// Boolean... opens its dialog (the Edit tab row, the menu, the
    /// shortcut, the command line's bare `poly.boolean`).
    pub(super) fn open_boolean(&mut self) {
        if self.boolean.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Boolean".into();
            return;
        }
        let o = self.editor.boolean_preset.clone();
        let scene = &self.editor.scene;
        let active = scene.selection.item;
        // Mesh items and groups other than the active mesh.
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
        let rows = BooleanDialog::rows(&o, &names, chosen);
        self.boolean = Some(BooleanDialog {
            form: self.form_dialog("Boolean", rows),
            items,
        });
    }

    /// Run `poly.boolean` with `o`: the result on the status line, or the
    /// refusal (a notice with Select Problem Edges when one input's
    /// geometry is the problem).
    pub(super) fn run_boolean(&mut self, o: BooleanOptions) {
        if self.run_ok(Command::Boolean(o.clone())) {
            if let Some(note) = self.editor.boolean_note.take() {
                self.status = note;
            }
            return;
        }
        if let Some(problem) = self.editor.boolean_problem(&o) {
            let message = self.status.clone();
            let form = form::Dialog::notice("Boolean", &message).extras(&["Select Problem Edges"]);
            self.boolean_notice = Some(BooleanNotice { form, problem });
        }
    }

    /// `poly.boolean` typed or scripted: bare opens the dialog (UIC-162);
    /// otherwise `mode:`, `cutmesh:`, `item:` (id or name) and `children:`.
    pub(super) fn boolean_command(&mut self, args: &[String]) {
        if args.is_empty() {
            self.open_boolean();
            return;
        }
        match self.boolean_arguments(args) {
            Ok(o) => self.run_boolean(o),
            Err(e) => self.status = e,
        }
    }

    fn boolean_arguments(&self, args: &[String]) -> std::result::Result<BooleanOptions, String> {
        let mut o = BooleanOptions::default();
        let names = ["mode", "cutmesh", "item", "children"];
        for (i, raw) in args.iter().enumerate() {
            let (name, value) = match raw.split_once(':') {
                Some((k, v)) if names.iter().any(|n| n.eq_ignore_ascii_case(k)) => {
                    (k.to_ascii_lowercase(), v)
                }
                _ => (
                    names
                        .get(i)
                        .ok_or("poly.boolean takes mode, cutmesh, item and children")?
                        .to_string(),
                    raw.as_str(),
                ),
            };
            match name.as_str() {
                "mode" => o.op = BooleanOp::parse(value)?,
                "cutmesh" => o.drive = DriveMesh::parse(value)?,
                "item" => {
                    let items = &self.editor.scene.items;
                    let id = value
                        .parse::<u64>()
                        .ok()
                        .filter(|id| items.iter().any(|i| i.id == *id))
                        .or_else(|| items.iter().find(|i| i.name == value).map(|i| i.id))
                        .ok_or_else(|| format!("poly.boolean: no item {value}"))?;
                    o.item = Some(id);
                }
                _ => o.children = operandi_core::command_args::boolean("children", value)?,
            }
        }
        Ok(o)
    }

    /// The dialog and the refusal notice, once per frame.
    pub(super) fn boolean_dialog(&mut self, ctx: &Context) {
        if let Some(mut notice) = self.boolean_notice.take() {
            match self.show_form(ctx, &mut notice.form) {
                form::Outcome::Extra(0) => self.select_boolean_problem(&notice.problem),
                form::Outcome::Open => self.boolean_notice = Some(notice),
                _ => {}
            }
            return;
        }
        let Some(mut dialog) = self.boolean.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Boolean closed; nothing changed".into();
            return;
        }
        match self.show_form(ctx, &mut dialog.form) {
            form::Outcome::Cancel => self.status = "Boolean canceled; nothing changed".into(),
            form::Outcome::Ok => {
                let o = dialog.options();
                if o.drive == DriveMesh::Item && o.item.is_none() {
                    let e = "Choose the Surface Item that drives the boolean".to_string();
                    self.status = e.clone();
                    dialog.form.error = Some(e);
                    self.boolean = Some(dialog);
                    return;
                }
                // UIC-95: remembered for next time, whatever the outcome.
                self.editor.boolean_preset = o.clone();
                self.run_boolean(o);
            }
            _ => self.boolean = Some(dialog),
        }
    }

    /// Select Problem Edges: that item, Edges mode, just those edges.
    fn select_boolean_problem(&mut self, problem: &BooleanProblem) {
        self.run(Command::SelectItem(Some(problem.item)));
        self.run(Command::Mode(Mode::Edge));
        self.run(Command::BeginSelection);
        self.run(Command::ClearSelection);
        for &(a, b) in &problem.edges {
            self.run(Command::Select {
                item: problem.item,
                hit: operandi_core::Component::Edge(a, b),
                replace: false,
                remove: false,
            });
        }
        self.run(Command::EndSelection { cancel: false });
        self.status = format!("Selected {} problem edges", problem.edges.len());
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

    /// Target cube -1..1 and driver cube 0..2, the target selected.
    fn reference(a: &mut Operandi) -> (u64, u64) {
        let mut ids = Vec::new();
        for (c, mat) in [(DVec3::ZERO, "A"), (DVec3::ONE, "B")] {
            a.editor.dispatch(Command::NewMesh).unwrap();
            let item = a.editor.scene.active_mut().unwrap();
            let mut m = operandi_core::Mesh::box_mesh(c, DVec3::splat(2.), 0).unwrap();
            for f in &mut m.faces {
                f.material = mat.into();
            }
            item.mesh = m.into();
            ids.push(item.id);
        }
        a.editor
            .dispatch(Command::SelectItem(Some(ids[0])))
            .unwrap();
        (ids[0], ids[1])
    }

    #[test]
    fn typed_command_runs_and_bare_opens_the_dialog() {
        let mut a = app();
        let (target, _) = reference(&mut a);
        a.execute("poly.boolean");
        assert!(a.boolean.is_some(), "bare poly.boolean opens the dialog");
        a.boolean = None;
        a.execute("poly.boolean mode:subtract");
        let m = &a.editor.scene.item(target).unwrap().mesh;
        assert_eq!(m.faces.len(), 9, "{}", a.status);
        assert!(a.status.starts_with("Boolean Subtract"), "{}", a.status);
        a.execute("poly.boolean 1");
        assert!(a.status.contains("Intersect"), "{}", a.status);
    }

    #[test]
    fn arguments_take_numbers_words_and_item_names() {
        let mut a = app();
        let (_, driver) = reference(&mut a);
        let item = a.editor.scene.item_mut(driver).unwrap();
        item.name = "Cutter".into();
        let name = item.name.clone();
        let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        let o = a
            .boolean_arguments(&args(&format!(
                "mode:3 cutmesh:item item:{name} children:true"
            )))
            .unwrap();
        assert_eq!(o.op, BooleanOp::Add);
        assert_eq!(o.drive, DriveMesh::Item);
        assert_eq!(o.item, Some(driver));
        assert!(o.children);
        assert!(a.boolean_arguments(&args("mode:9")).is_err());
        assert!(a.boolean_arguments(&args("item:nothing")).is_err());
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
        a.run_boolean(BooleanOptions::default());
        assert_eq!(a.editor.scene.items, before, "nothing changed");
        let notice = a.boolean_notice.take().expect("a notice");
        assert!(a.status.contains("4 open edges"), "{}", a.status);
        a.select_boolean_problem(&notice.problem);
        assert_eq!(a.editor.scene.selection.item, Some(driver));
        assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
        assert_eq!(a.editor.scene.selection.edges.len(), 4);
    }
}
