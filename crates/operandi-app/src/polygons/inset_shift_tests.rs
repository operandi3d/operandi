//! Inset and Shift (no toolbox buttons): reachable from the command line,
//! `tool.set`, the modeling menu and the viewport handle (tool audit Tier 1).
use super::tests::box_app;
use super::*;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

fn live_tool(a: &Operandi) -> Option<Tool> {
    a.editor.session.live.as_ref().map(|l| l.tool)
}

#[test]
fn inset_and_shift_start_from_their_names_and_tool_set() {
    for (name, id, tool) in [
        ("Inset", "poly.inset", Tool::Inset),
        ("Shift", "poly.shift", Tool::Shift),
    ] {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.execute(name);
        assert_eq!(live_tool(&a), Some(tool), "{}", a.status);
        a.run(Command::Drop);
        a.execute(&format!("tool.set {id} on"));
        assert_eq!(live_tool(&a), Some(tool), "{}", a.status);
    }
}

#[test]
fn modeling_menu_lists_them_in_polygon_mode() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    let (_, list) = modeling_menu::entries(Mode::Polygon);
    for (label, tool) in [("Inset", Tool::Inset), ("Shift", Tool::Shift)] {
        let entry = *list
            .iter()
            .find(|e| e.label() == Some(label))
            .expect("listed");
        assert!(matches!(entry, modeling_menu::Entry::Action(..)), "{label}");
        a.run_entry(entry);
        assert_eq!(live_tool(&a), Some(tool));
        a.run(Command::Drop);
    }
    assert!(!input::available("Inset", Mode::Edge, false));
}

#[test]
fn the_handle_drags_the_distance() {
    for (tool, label) in [(Tool::Inset, "Inset"), (Tool::Shift, "Shift")] {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![0];
        a.activate(tool);
        let handles = a.handles(rect());
        let (axis, origin, end, name) = handles.first().cloned().expect("one handle");
        assert_eq!(name, label);
        assert_eq!(handles.len(), 1);
        let base = a.editor.session.live.as_ref().unwrap().params.clone();
        a.update_handle(axis, base, origin, end, tool, rect());
        let p = &a.editor.session.live.as_ref().unwrap().params;
        let amount = if tool == Tool::Inset {
            p.inset
        } else {
            p.shift
        };
        assert!(amount != 0., "{tool:?}: {}", a.status);
        let faces = a.editor.scene.active().unwrap().mesh.faces.len();
        assert_eq!(faces, 6 + 4, "{tool:?}");
    }
}

#[test]
fn a_drag_away_from_the_handle_changes_the_distance() {
    for tool in [Tool::Inset, Tool::Shift] {
        assert!(viewport::off_handle_drag(tool));
        let mut a = box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![0];
        a.activate(tool);
        let base = a.editor.session.live.as_ref().unwrap().params.clone();
        let start = pos2(400., 300.);
        a.update_handle(
            viewport::OFF_HANDLE,
            base,
            start,
            start + vec2(-40., -40.),
            tool,
            rect(),
        );
        let p = &a.editor.session.live.as_ref().unwrap().params;
        let amount = if tool == Tool::Inset {
            p.inset
        } else {
            p.shift
        };
        assert!(amount > 0., "{tool:?}: {amount} {}", a.status);
    }
}

#[test]
fn geometry_polygon_menu_lists_shift_and_inset() {
    let a = box_app();
    let ctx = Context::default();
    let model = a.menu_model(&ctx);
    let geometry = model
        .iter()
        .find(|m| m.title == "Geometry")
        .expect("Geometry");
    let polygon = geometry
        .entries
        .iter()
        .find_map(|e| match e {
            app_menu::Entry::Submenu(m) if m.title == "Polygon" => Some(m),
            _ => None,
        })
        .expect("Geometry > Polygon");
    let actions: Vec<_> = polygon.leaves().iter().map(|i| i.action).collect();
    for tool in [Tool::Shift, Tool::Inset] {
        assert!(
            actions.contains(&app_menu::line_action(tool.label())),
            "{tool:?}"
        );
    }
}
