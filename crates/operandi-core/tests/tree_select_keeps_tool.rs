//! Clicking another item in the Items tree while Move, Rotate, Scale or
//! Transform is live keeps the tool and re-targets it (Jason, 2026-11-08):
//! the open drag becomes its own undo step, the handle jumps to the new item
//! and the offset starts at zero. Original test data.
use glam::DVec3;
use operandi_core::*;

/// Two box meshes, A at the origin and B at x = 10, in `mode`.
fn fixture(mode: Mode) -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for x in [0., 10.] {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.items.last_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
            .unwrap()
            .into();
        item.transform.translation = DVec3::new(x, 0., 0.);
        ids.push(item.id);
    }
    e.dispatch(Command::Mode(mode)).unwrap();
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    e.history.undo.clear();
    (e, ids[0], ids[1])
}
fn item(e: &Editor, id: u64) -> &Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap()
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

#[test]
fn move_retargets_to_the_clicked_item() {
    let (mut e, a, b) = fixture(Mode::Item);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    preview(&mut e, |p| p.delta = DVec3::Y);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);

    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.scene.selection.item, Some(b));
    assert_eq!(e.session.live.as_ref().unwrap().params.delta, DVec3::ZERO);
    assert!((center(&e) - DVec3::new(10., 0., 0.)).length() < 1e-9);
    // A's move is recorded as its own step.
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);

    // The next drag moves B only; A stays where its drag left it.
    preview(&mut e, |p| p.delta = DVec3::Z);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);
    assert_eq!(item(&e, b).transform.translation, DVec3::new(10., 0., 1.));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 2);

    // Undo takes B's move back, then A's.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(item(&e, b).transform.translation, DVec3::new(10., 0., 0.));
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(item(&e, a).transform.translation, DVec3::ZERO);
}

#[test]
fn rotate_and_scale_retarget_with_fresh_values() {
    for t in [Tool::Rotate, Tool::Scale, Tool::Transform] {
        let (mut e, a, b) = fixture(Mode::Item);
        e.dispatch(Command::Activate(t)).unwrap();
        let before_a = item(&e, a).transform.clone();
        match t {
            // Rotate (degrees) and Scale (factors) keep their value in `delta`.
            Tool::Rotate => preview(&mut e, |p| p.delta = DVec3::new(0., 30., 0.)),
            Tool::Scale => preview(&mut e, |p| p.delta = DVec3::splat(2.)),
            _ => preview(&mut e, |p| {
                p.delta = DVec3::X;
                p.rotation_delta = DVec3::new(0., 30., 0.);
                p.scale_delta = DVec3::splat(2.);
            }),
        }
        let moved_a = item(&e, a).transform.clone();
        assert_ne!(moved_a, before_a, "{t:?}");

        e.dispatch(Command::SelectItem(Some(b))).unwrap();
        assert_eq!(tool(&e), Some(t), "{t:?}");
        let p = &e.session.live.as_ref().unwrap().params;
        let zero = if t == Tool::Scale {
            DVec3::ONE
        } else {
            DVec3::ZERO
        };
        assert_eq!(p.delta, zero, "{t:?}");
        assert_eq!(p.rotation_delta, DVec3::ZERO, "{t:?}");
        assert_eq!(p.scale_delta, DVec3::ONE, "{t:?}");
        assert!((center(&e) - DVec3::new(10., 0., 0.)).length() < 1e-9);
        assert_eq!(e.history.undo.len(), 1, "{t:?}");
        assert_eq!(item(&e, a).transform, moved_a, "{t:?}");
    }
}

#[test]
fn component_mode_move_follows_the_clicked_mesh() {
    let (mut e, _a, b) = fixture(Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    preview(&mut e, |p| p.delta = DVec3::Y);
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert_eq!(e.scene.selection.item, Some(b));
    assert_eq!(e.history.undo.len(), 1);
    let before = item(&e, b).mesh.vertices.clone();
    preview(&mut e, |p| p.delta = DVec3::Z);
    let after = &item(&e, b).mesh.vertices;
    assert!(
        before
            .iter()
            .zip(after.iter())
            .all(|(p, q)| *q == *p + DVec3::Z)
    );
}

#[test]
fn deselecting_drops_the_tool_without_error() {
    let (mut e, a, _b) = fixture(Mode::Item);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    preview(&mut e, |p| p.delta = DVec3::Y);
    e.dispatch(Command::SelectItem(None)).unwrap();
    assert_eq!(tool(&e), None);
    assert_eq!(e.scene.selection.item, None);
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);
}

#[test]
fn options_carry_over() {
    let (mut e, _a, b) = fixture(Mode::Item);
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    e.session.live.as_mut().unwrap().params.negative_scale = true;
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert!(e.session.live.as_ref().unwrap().params.negative_scale);
    // An untouched tool records nothing on the way.
    assert!(e.history.undo.is_empty());
}

#[test]
fn a_drop_failure_refuses_and_changes_nothing() {
    let (mut e, _a, b) = fixture(Mode::Item);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    preview(&mut e, |p| p.delta = DVec3::Y);
    e.session.live.as_mut().unwrap().preview_error = Some("bad field".into());
    let before = e.clone();
    assert!(e.dispatch(Command::SelectItem(Some(b))).is_err());
    assert_eq!(e.scene, before.scene);
    assert!(e.session.live == before.session.live);
    assert_eq!(e.history.undo.len(), before.history.undo.len());
}

#[test]
fn toggle_range_and_group_selects_keep_the_tool() {
    let (mut e, a, b) = fixture(Mode::Item);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::ToggleItemSelection(b)).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert!(e.scene.selected_items().contains(&b));
    e.dispatch(Command::SelectItems(vec![b, a])).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.scene.selection.item, Some(b));
    // The next drag moves every selected item (multiple active meshes,
    // stage 2: item transforms act on all of them together).
    preview(&mut e, |p| p.delta = DVec3::Y);
    assert_eq!(item(&e, a).transform.translation, DVec3::Y);
    assert_eq!(item(&e, b).transform.translation, DVec3::new(10., 1., 0.));
}

#[test]
fn other_tools_settle_for_item_clicks() {
    // UIC-307: Bevel's work settles and the click selects the other item;
    // Bevel starts again there if it can.
    let (mut e, _a, b) = fixture(Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert_eq!(e.scene.selection.item, Some(b));
    assert!(
        e.session
            .live
            .as_ref()
            .is_none_or(|l| l.tool == Tool::Bevel)
    );
}
