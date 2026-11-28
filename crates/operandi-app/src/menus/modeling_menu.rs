//! Cmd+click modeling pop-up menu (specifications/modeling-popup-menu,
//! POP-PHY-01…10): Command-click in the viewport opens a menu at the pointer.
//! Component modes show Modeling Operations, Items mode Item Operations, each
//! in the measured order with its separators. Entries run the same actions as
//! their toolbox rows and shortcuts.
//!
//! Departures, logged in SPECIFICATION_QUESTIONS.md (POP-*): the other root
//! entries (materials, shaders, selection sets, lasso style, recording) are
//! left out, and operations Operandi doesn't have yet are listed disabled so
//! the order stays familiar.
//!
//! macOS only: the Command key. Elsewhere Ctrl+click already removes from the
//! selection, so no gesture is taken over there.
use super::*;
use crate::unavailable::Gate;

/// What a menu entry does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// A named action from the input map (input.rs ACTIONS), run like its
    /// shortcut.
    Action(&'static str, &'static str),
    /// Arm Add Loop, like its Edge panel row.
    AddLoop,
    /// POP-PHY-10 plain Duplicate.
    Duplicate,
    /// The menu's Join (no "…", measured): joins at once with the Join
    /// dialog's starting options, where the Vertex panel's Join… asks.
    JoinPoints,
    /// Unparent the selected item, keeping its world position or not.
    Unparent(&'static str, bool),
    /// GL-04 Parent to Group Locator on the selected items.
    ParentToGroupLocator,
    /// Not in Operandi yet: shown disabled.
    Missing(&'static str),
    Separator,
}
use Entry::*;

impl Entry {
    pub(super) fn label(self) -> Option<&'static str> {
        match self {
            Action(label, _) | Missing(label) | Unparent(label, _) => Some(label),
            AddLoop => Some("Add Loop"),
            ParentToGroupLocator => Some(operandi_core::locator::PARENT_TO_GROUP_LOCATOR),
            Duplicate => Some("Duplicate"),
            JoinPoints => Some("Join"),
            Separator => None,
        }
    }
}

/// The submenu's title and entries for a selection mode, in the measured
/// order (POP-PHY-02, 04, 06, 09).
pub(super) fn entries(mode: Mode) -> (&'static str, &'static [Entry]) {
    match mode {
        Mode::Polygon => (
            "Modeling Operations",
            &[
                Action("Bevel", "Bevel"),
                Action("Extrude", "Extrude"),
                Action("Sketch Extrude", "Sketch Extrude"),
                Missing("Slide"),
                Action("Bridge", "Bridge"),
                AddLoop,
                Action("Smooth Shift", "Smooth Shift"),
                Action("Shift", "Shift"),
                Action("Inset", "Inset"),
                Action("Chamfer Edit", "Chamfer Edit"),
                Separator,
                Action("Remove", "Remove"),
                Action("Collapse", "Polygon Collapse"),
                // UIC-80: these open a dialog first.
                Action("Unify…", "Unify"),
                Action("Merge", "Polygon Merge"),
                Action("Flip", "Flip"),
                Action("Spin", "Spin Quad"),
                Action("Triple", "Triple"),
            ],
        ),
        Mode::Vertex => (
            "Modeling Operations",
            &[
                Action("Bevel", "Vertex Bevel"),
                Action("Extrude", "Vertex Extrude"),
                Action("Slide", "Vertex Slide"),
                Separator,
                Action("Remove", "Vertex Remove"),
                Action("Collapse", "Vertex Collapse"),
                Action("Split Vertex", "Vertex Split"),
                Action("Split Polygon", "Split"),
                Action("Merge…", "Vertex Merge"),
                JoinPoints,
                Missing("Center"),
                Action("Align…", "Vertex Align"),
            ],
        ),
        Mode::Edge => (
            "Modeling Operations",
            &[
                Action("Bevel", "Edge Bevel"),
                Action("Chamfer", "Edge Chamfer"),
                Action("Extrude", "Edge Extrude"),
                Action("Extend", "Extend"),
                Action("Slide", "Slide"),
                Action("Bridge", "Bridge"),
                AddLoop,
                Action("Edge Weight", "Edge Weight"),
                Separator,
                Action("Spin", "Edge Spin"),
                Action("Remove", "Edge Remove"),
                Action("Collapse", "Edge Collapse"),
                Action("Join", "Join"),
                Action("Split", "Edge Split"),
                Action("Grow Quads", "Grow Quads"),
                Action("Fill Quads", "Fill Quads"),
                Action("Split Normals", "Split Normals"),
                Action("Edges to Curves", "Edges To Curves"),
                Action("Unbevel", "Unbevel"),
            ],
        ),
        Mode::Item => (
            "Item Operations",
            &[
                Missing("Find in Item List"),
                Separator,
                Action("Delete", "Delete Item"),
                Separator,
                Missing("Duplicate…"),
                Duplicate,
                Missing("Duplicate Hierarchy"),
                Action("Instance", "Instance"),
                Missing("Instance Hierarchy"),
                Separator,
                Missing("Change Type"),
                Missing("Presets"),
                Missing("Add Mesh Operation"),
                Missing("Merge Meshes – Selected"),
                Separator,
                Missing("Parent"),
                Missing("Parent in Place"),
                ParentToGroupLocator,
                Unparent("Unparent", false),
                Unparent("Unparent in Place", true),
                Separator,
                Missing("Create Assembly Group"),
                Separator,
                Missing("Select Item Hierarchy"),
                Missing("Pick Walk"),
                Missing("Clear Selection Redirect"),
                Separator,
                Missing("Apply to Setup/Rest"),
                Missing("Restore Setup/Rest"),
                Missing("Restore Setup/Rest (Transforms Only)"),
                Separator,
                Missing("Select Source of Instance"),
                Missing("Select Instances"),
                Missing("Select Input Items"),
                Missing("Select Output Items"),
            ],
        ),
    }
}

/// True when a viewport press should open the menu instead of selecting:
/// a plain Command click (no Shift, which is Fill; no Alt, which navigates;
/// no Ctrl).
pub(super) fn wants_menu(modifiers: Modifiers) -> bool {
    modifiers.mac_cmd && !modifiers.shift && !modifiers.alt && !modifiers.ctrl
}

impl Operandi {
    /// Whether an entry can run now. Unparent needs a parented item.
    #[cfg(test)]
    fn entry_enabled(&self, entry: Entry) -> bool {
        self.entry_unavailable(entry).is_none()
    }
    /// Why an entry is greyed now (`None` = it can run).
    fn entry_unavailable(&self, entry: Entry) -> Option<String> {
        use crate::unavailable::{TOOL_ACTIVE, first, unless};
        let mode = self.editor.scene.selection.mode;
        let live = self.editor_tool_active();
        let idle = unless(!live, TOOL_ACTIVE);
        let why = match entry {
            Action(_, action) => return input::unavailable_reason(action, mode, live),
            // As its action would be.
            JoinPoints => return input::unavailable_reason("Vertex Join Averaged", mode, live),
            AddLoop => None,
            Duplicate | ParentToGroupLocator => first([
                idle,
                unless(
                    self.editor.scene.selection.item.is_some(),
                    "Select an item first",
                ),
            ]),
            Unparent(..) => first([
                idle,
                unless(
                    self.editor
                        .scene
                        .active()
                        .is_some_and(|item| item.parent.is_some()),
                    "The item has no parent",
                ),
            ]),
            Missing(_) => Some("Not available in Operandi yet"),
            Separator => Some(""),
        };
        why.map(str::to_owned)
    }

    /// Run a chosen entry, as its row or shortcut does.
    pub(super) fn run_entry(&mut self, entry: Entry) {
        match entry {
            Action(_, action) => self.execute(action),
            AddLoop => {
                if self.edge_click != Some(edge_panel::EdgeClick::AddLoop) {
                    self.toggle_edge_click(edge_panel::EdgeClick::AddLoop);
                }
            }
            Duplicate => self.run(Command::DuplicateItem),
            JoinPoints => self.run_vertex_command(operandi_core::VertexCommand::Join {
                average: false,
                options: operandi_core::MergeOptions::default(),
            }),
            ParentToGroupLocator => self.run(Command::ParentToGroupLocator),
            Unparent(_, keep_position) => {
                if let Some(item) = self.editor.scene.selection.item {
                    self.run(Command::SetParent {
                        item,
                        parent: None,
                        keep_position,
                    });
                }
            }
            Missing(_) | Separator => {}
        }
    }

    /// Draw the menu while it is open; runs the chosen entry.
    pub(super) fn modeling_menu(&mut self, ui: &Ui) {
        let request = std::mem::take(&mut self.modeling_menu_requested);
        let (title, list) = entries(self.editor.scene.selection.mode);
        let mut chosen = None;
        egui::Popup::new(
            Id::new("modeling-menu"),
            ui.ctx().clone(),
            egui::PopupAnchor::PointerFixed,
            ui.layer_id(),
        )
        .open_memory(request.then_some(egui::SetOpenCommand::Bool(true)))
        .show(|ui| {
            self.menu_owned_input = true;
            ui.set_min_width(170.);
            ui.menu_button(title, |ui| {
                ui.set_min_width(190.);
                for &entry in list {
                    let Some(label) = entry.label() else {
                        ui.separator();
                        continue;
                    };
                    let shortcut = match entry {
                        Action(_, action) => self.shortcut(action),
                        _ => String::new(),
                    };
                    let why = self.entry_unavailable(entry);
                    let button =
                        ui.gated(why.as_deref(), Button::new(format!("{label}{shortcut}")));
                    if button.clicked() {
                        chosen = Some(entry);
                        ui.close();
                    }
                }
            });
        });
        if let Some(entry) = chosen {
            self.run_entry(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw, pointer};
    use egui::accesskit::{self, Role};

    const SIZE: Vec2 = vec2(1440., 900.);

    fn nodes(a: &mut Operandi, ctx: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
        draw(a, ctx, SIZE, events)
            .platform_output
            .accesskit_update
            .expect("AccessKit enabled")
            .nodes
            .into_iter()
            .map(|(_, n)| n)
            .collect()
    }

    fn button(nodes: &[accesskit::Node], starts: &str) -> Option<Rect> {
        nodes
            .iter()
            .find(|n| n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with(starts)))
            .and_then(|n| n.bounds())
            .map(|b| {
                Rect::from_min_max(
                    pos2(b.x0 as f32, b.y0 as f32),
                    pos2(b.x1 as f32, b.y1 as f32),
                )
            })
    }

    /// The test cube in the given mode, drawn and ready to click.
    fn app(mode: Mode) -> (Operandi, Context, Pos2) {
        let mut a = box_app();
        a.run(Command::Mode(mode));
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        for _ in 0..2 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        let at = a.viewport_rect.center();
        (a, ctx, at)
    }

    fn click(
        a: &mut Operandi,
        ctx: &Context,
        at: Pos2,
        modifiers: Modifiers,
    ) -> Vec<accesskit::Node> {
        nodes(
            a,
            ctx,
            vec![Event::PointerMoved(at), pointer(at, true, modifiers)],
        );
        nodes(a, ctx, vec![pointer(at, false, modifiers)]);
        nodes(a, ctx, vec![])
    }

    #[test]
    fn command_click_opens_the_menu_and_changes_nothing() {
        for (mode, title) in [
            (Mode::Polygon, "Modeling Operations"),
            (Mode::Item, "Item Operations"),
        ] {
            let (mut a, ctx, at) = app(mode);
            let (scene, history) = (a.editor.scene.clone(), a.editor.history.undo.len());
            let shown = click(&mut a, &ctx, at, Modifiers::MAC_CMD);
            assert!(egui::Popup::is_any_open(&ctx), "menu open");
            let root = button(&shown, title).expect("submenu entry");
            // Hovering the submenu entry opens it.
            let shown = nodes(&mut a, &ctx, vec![Event::PointerMoved(root.center())]);
            let shown = if button(&shown, "Remove").is_none() && button(&shown, "Delete").is_none()
            {
                click(&mut a, &ctx, root.center(), Modifiers::NONE)
            } else {
                shown
            };
            assert!(
                button(
                    &shown,
                    if mode == Mode::Item {
                        "Delete"
                    } else {
                        "Triple"
                    }
                )
                .is_some(),
                "{mode:?} submenu"
            );
            assert_eq!(a.editor.scene, scene, "no selection change");
            assert_eq!(a.editor.history.undo.len(), history);
            assert!(a.editor.session.live.is_none());
        }
    }

    /// POP-PHY-02/04/06: the measured entries, in order, with one separator.
    #[test]
    fn entries_follow_the_measured_order() {
        let labels = |mode| -> Vec<&str> {
            entries(mode)
                .1
                .iter()
                .map(|e| e.label().unwrap_or("—"))
                .collect()
        };
        assert_eq!(
            labels(Mode::Polygon),
            [
                "Bevel",
                "Extrude",
                "Sketch Extrude",
                "Slide",
                "Bridge",
                "Add Loop",
                "Smooth Shift",
                "Shift",
                "Inset",
                "Chamfer Edit",
                "—",
                "Remove",
                "Collapse",
                "Unify…",
                "Merge",
                "Flip",
                "Spin",
                "Triple"
            ]
        );
        assert_eq!(
            labels(Mode::Vertex),
            [
                "Bevel",
                "Extrude",
                "Slide",
                "—",
                "Remove",
                "Collapse",
                "Split Vertex",
                "Split Polygon",
                "Merge…",
                "Join",
                "Center",
                "Align…"
            ]
        );
        assert_eq!(labels(Mode::Edge).len(), 19);
        // Every Action names an action the input map knows.
        for mode in [Mode::Polygon, Mode::Edge, Mode::Vertex, Mode::Item] {
            for entry in entries(mode).1 {
                if let Action(_, action) = entry {
                    assert!(
                        input::ACTIONS.iter().any(|(name, _)| name == action),
                        "{action}"
                    );
                }
            }
        }
    }

    /// POP-PHY-03, 05, 07, 10: the measured chosen action in each mode.
    #[test]
    fn chosen_entries_run_their_operations() {
        let entry = |mode, label: &str| {
            *entries(mode)
                .1
                .iter()
                .find(|e| e.label() == Some(label))
                .unwrap()
        };
        let mut a = box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![1];
        a.run_entry(entry(Mode::Polygon, "Triple"));
        let mesh = &a.editor.scene.active().unwrap().mesh;
        assert_eq!((mesh.vertices.len(), mesh.faces.len()), (8, 7));

        let mut a = box_app();
        a.run(Command::Mode(Mode::Vertex));
        a.editor.scene.selection.vertices = vec![7];
        a.run_entry(entry(Mode::Vertex, "Split Vertex"));
        let mesh = &a.editor.scene.active().unwrap().mesh;
        assert_eq!(
            (mesh.vertices.len(), mesh.faces.len()),
            (10, 6),
            "{}",
            a.status
        );

        let mut a = box_app();
        a.run(Command::Mode(Mode::Edge));
        let (p, q) = a.editor.scene.active().unwrap().mesh.edges()[0];
        a.editor.scene.selection.edges = vec![(p, q)];
        a.run_entry(entry(Mode::Edge, "Collapse"));
        let mesh = &a.editor.scene.active().unwrap().mesh;
        assert_eq!(
            (mesh.vertices.len(), mesh.faces.len()),
            (7, 6),
            "{}",
            a.status
        );

        let mut a = box_app();
        a.run(Command::Mode(Mode::Item));
        let name = a.editor.scene.items[0].name.clone();
        a.run_entry(Duplicate);
        let items = &a.editor.scene.items;
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].name, format!("{name} (2)"));
        assert_eq!(items[1].mesh, items[0].mesh);
        assert_eq!(a.editor.scene.selection.item, Some(items[1].id));
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.items.len(), 1);
    }

    #[test]
    fn other_clicks_keep_their_meaning() {
        // Plain and Shift clicks select; Ctrl is untouched (deselect off macOS).
        for modifiers in [Modifiers::NONE, Modifiers::SHIFT, Modifiers::CTRL] {
            assert!(!wants_menu(modifiers), "{modifiers:?}");
        }
        // Shift+Cmd is Fill / select between, not the menu.
        assert!(!wants_menu(Modifiers::MAC_CMD | Modifiers::SHIFT));
        assert!(!wants_menu(Modifiers::MAC_CMD | Modifiers::ALT));
        assert!(wants_menu(Modifiers::MAC_CMD));
        let (mut a, ctx, at) = app(Mode::Polygon);
        click(&mut a, &ctx, at, Modifiers::NONE);
        assert!(!egui::Popup::is_any_open(&ctx));
        assert!(
            !a.editor.scene.selection.faces.is_empty(),
            "a plain click still selects"
        );
    }

    /// AL-PHY-04: with Add Loop armed, Command-click adds nothing and opens
    /// no menu.
    #[test]
    fn command_click_with_add_loop_does_nothing() {
        let (mut a, ctx, at) = app(Mode::Edge);
        a.toggle_edge_click(edge_panel::EdgeClick::AddLoop);
        let scene = a.editor.scene.clone();
        click(&mut a, &ctx, at, Modifiers::MAC_CMD);
        assert!(!egui::Popup::is_any_open(&ctx));
        assert_eq!(a.editor.scene, scene);
        assert_eq!(a.edge_click, Some(edge_panel::EdgeClick::AddLoop));
    }

    #[test]
    fn losing_focus_mid_press_opens_nothing() {
        let (mut a, ctx, at) = app(Mode::Polygon);
        nodes(
            &mut a,
            &ctx,
            vec![
                Event::PointerMoved(at),
                pointer(at, true, Modifiers::MAC_CMD),
            ],
        );
        nodes(&mut a, &ctx, vec![Event::WindowFocused(false)]);
        nodes(
            &mut a,
            &ctx,
            vec![
                Event::WindowFocused(true),
                pointer(at, false, Modifiers::NONE),
            ],
        );
        nodes(&mut a, &ctx, vec![]);
        assert!(!egui::Popup::is_any_open(&ctx));
    }

    /// GL-04: Item Operations > Parent to Group Locator is greyed with no
    /// item selected and puts the selection under a new locator.
    #[test]
    fn parent_to_group_locator_entry() {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Item));
        let cube = a.editor.scene.items[0].id;
        a.run(Command::SelectItem(None));
        assert!(!a.entry_enabled(ParentToGroupLocator));
        a.run(Command::SelectItem(Some(cube)));
        assert!(a.entry_enabled(ParentToGroupLocator));
        a.run_entry(ParentToGroupLocator);
        let g = a.editor.scene.selection.item.unwrap();
        assert!(
            a.editor
                .scene
                .items
                .iter()
                .any(|i| i.id == g && i.is_group_locator())
        );
        let parent = a.editor.scene.item(cube).unwrap().parent;
        assert_eq!(parent, Some(g));
        assert!(entries(Mode::Item).1.contains(&ParentToGroupLocator));
    }

    /// N11: Ctrl+G runs Parent to Group Locator on every selected item.
    #[test]
    fn n11_ctrl_g_groups_the_selected_items() {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Item));
        let cube = a.editor.scene.items[0].id;
        a.run(Command::NewMesh);
        let other = a.editor.scene.selection.item.unwrap();
        a.run(Command::Mode(Mode::Item));
        a.editor.scene.select_item(Some(cube));
        a.editor.scene.toggle_item(other);
        assert_eq!(
            a.bindings
                .get("Parent to Group Locator")
                .map(String::as_str),
            Some("Ctrl+G")
        );
        a.execute("Parent to Group Locator");
        let g = a.editor.scene.selection.item.unwrap();
        for id in [cube, other] {
            assert_eq!(a.editor.scene.item(id).unwrap().parent, Some(g));
        }
    }
}
