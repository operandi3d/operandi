//! Grow and Shrink (Shift+Up/Down) while Move, Rotate, Scale or Transform
//! is live keep the tool and re-target it, as an Items-tree click does
//! (UIC-306, Jason 2026-11-27). Original test data.
use glam::DVec3;
use operandi_core::*;

/// One 2-unit box at the origin in Polygon mode with one face selected.
fn fixture() -> (Editor, u32) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    let mesh = item.mesh.clone();
    let id = item.id;
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let top = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).dot(DVec3::Y) > 0.9)
        .unwrap() as u32;
    e.scene.selection.faces = vec![top];
    e.history.undo.clear();
    (e, top)
}
fn tool(e: &Editor) -> Option<Tool> {
    e.session.live.as_ref().map(|l| l.tool)
}
fn preview(e: &mut Editor, f: impl FnOnce(&mut Parameters)) {
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    f(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
}
fn center(e: &Editor) -> DVec3 {
    e.session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}

#[test]
fn grow_keeps_move_and_transform_live() {
    for t in [Tool::Move, Tool::Transform] {
        let (mut e, top) = fixture();
        e.dispatch(Command::Activate(t)).unwrap();
        assert!(e.selection_keeps_tool());
        assert!((center(&e) - DVec3::Y).length() < 1e-9, "{t:?}");
        preview(&mut e, |p| p.delta = DVec3::X);
        let dragged = points(&e);

        e.dispatch(Command::Selection(SelectionOperation::Grow))
            .unwrap();
        assert_eq!(tool(&e), Some(t), "{t:?}");
        assert_eq!(e.scene.selection.faces.len(), 5, "{t:?}");
        assert!(e.scene.selection.faces.contains(&top));
        assert_eq!(e.session.live.as_ref().unwrap().params.delta, DVec3::ZERO);
        // The drag so far is one step and the selection change the next.
        assert_eq!(e.history.undo.len(), 2, "{t:?}");
        assert_eq!(points(&e), dragged, "{t:?}");
        // The handle sits on the grown selection: the whole box, whose top
        // was moved by X (bounds x -1..2).
        assert!(
            (center(&e) - DVec3::new(0.5, 0., 0.)).length() < 1e-9,
            "{t:?} {}",
            center(&e)
        );

        // The next drag moves the grown selection (here every point).
        preview(&mut e, |p| p.delta = DVec3::Z);
        let after = points(&e);
        assert!(dragged.iter().zip(&after).all(|(p, q)| *q == *p + DVec3::Z));
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.len(), 3, "{t:?}");

        // Undo takes the second drag, the grow, then the first drag back.
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(points(&e), dragged);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.selection.faces, vec![top]);
        e.dispatch(Command::Undo).unwrap();
        assert!(points(&e).iter().all(|p| p.x.abs() == 1.));
    }
}

#[test]
fn shrink_keeps_rotate_and_scale_live() {
    for t in [Tool::Rotate, Tool::Scale] {
        let (mut e, top) = fixture();
        e.dispatch(Command::Selection(SelectionOperation::Grow))
            .unwrap();
        e.history.undo.clear();
        e.dispatch(Command::Activate(t)).unwrap();
        let start = center(&e);
        match t {
            Tool::Rotate => preview(&mut e, |p| p.delta = DVec3::new(0., 30., 0.)),
            _ => preview(&mut e, |p| p.delta = DVec3::splat(1.5)),
        }
        e.dispatch(Command::Selection(SelectionOperation::Shrink))
            .unwrap();
        assert_eq!(tool(&e), Some(t), "{t:?}");
        assert_eq!(e.scene.selection.faces, vec![top], "{t:?}");
        let zero = if t == Tool::Scale {
            DVec3::ONE
        } else {
            DVec3::ZERO
        };
        assert_eq!(e.session.live.as_ref().unwrap().params.delta, zero, "{t:?}");
        assert_eq!(e.history.undo.len(), 2, "{t:?}");
        assert_ne!(center(&e), start, "{t:?}");
    }
}

#[test]
fn an_untouched_tool_records_only_the_selection_step() {
    let (mut e, _) = fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Selection(SelectionOperation::Grow))
        .unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.history.undo.len(), 1);
}

#[test]
fn other_tools_and_other_selection_commands_settle_and_start_again() {
    // UIC-307: they no longer refuse. The tool's work settles, the
    // selection changes, and the same tool starts on the new selection.
    let (mut e, top) = fixture();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    assert!(!e.selection_keeps_tool());
    e.dispatch(Command::Selection(SelectionOperation::Grow))
        .unwrap();
    assert_eq!(tool(&e), Some(Tool::Bevel));
    assert!(e.scene.selection.faces.len() > 1);
    assert!(e.scene.selection.faces.contains(&top));

    let (mut e, top) = fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let r = e.dispatch(Command::Selection(SelectionOperation::More));
    assert_eq!(tool(&e), Some(Tool::Move), "{r:?}");
    if r.is_ok() {
        assert!(e.scene.selection.faces.contains(&top));
    }
}
