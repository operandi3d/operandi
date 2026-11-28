//! Lock items and geometry, hide/unhide (specifications/nav-lock-hide-2026-11-07):
//! LCK-T1..T8 and HID-T1..T6, the parts that live in the core.
use glam::DVec3;
use operandi_core::item_common::{Choice, ItemCommonEdit};
use operandi_core::*;

/// A 2 m cube, current, in Polygon mode, nothing selected.
fn cube() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        position: DVec3::ZERO,
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::ClearSelection).unwrap();
    (e, id)
}
fn select_faces(e: &mut Editor, id: u64, faces: &[u32]) {
    e.dispatch(Command::ClearSelection).unwrap();
    for &f in faces {
        e.dispatch(Command::Select {
            item: id,
            hit: Component::Polygon(f),
            replace: false,
            remove: false,
        })
        .unwrap();
    }
}
fn elements(e: &Editor) -> &ElementState {
    &e.scene.active().unwrap().elements
}
fn lock(e: &mut Editor, op: ElementOp) {
    e.dispatch(Command::EditElements {
        kind: ElementKind::Lock,
        op,
    })
    .unwrap();
}
fn hide(e: &mut Editor, op: ElementOp) {
    e.dispatch(Command::EditElements {
        kind: ElementKind::Hide,
        op,
    })
    .unwrap();
}
fn move_selection(e: &mut Editor, delta: DVec3) {
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn select_all(e: &mut Editor) {
    e.dispatch(Command::Selection(SelectionOperation::All))
        .unwrap();
}

fn set_item_lock(e: &mut Editor, id: u64, on: bool) {
    e.dispatch(Command::EditItemCommon {
        items: vec![id],
        edit: ItemCommonEdit::Lock(if on { Choice::Yes } else { Choice::No }),
    })
    .unwrap();
}

/// LCK-T1/T3: a locked item refuses item moves, typed transforms, the eye
/// and parenting; unlocking lets it move again. One undo step per toggle.
#[test]
fn lck_t1_t3_item_lock_refuses_item_edits() {
    let (mut e, id) = cube();
    e.dispatch(Command::NewMesh).unwrap();
    let other = e.scene.items.last().unwrap().id;
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    let steps = e.history.undo.len();
    set_item_lock(&mut e, id, true);
    assert_eq!(e.history.undo.len(), steps + 1);
    assert!(e.scene.active().unwrap().is_locked());
    let moved = Transform {
        translation: DVec3::X,
        ..Default::default()
    };
    let err = e
        .dispatch(Command::ItemTransform {
            item: id,
            transform: moved.clone(),
        })
        .unwrap_err();
    assert!(err.contains("locked"), "{err}");
    assert!(e.dispatch(Command::Visibility(id)).is_err());
    assert!(
        e.dispatch(Command::SetParent {
            item: id,
            parent: Some(other),
            keep_position: false,
        })
        .is_err()
    );
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(
        e.dispatch(Command::Preview(Parameters {
            delta: DVec3::X,
            ..Default::default()
        }))
        .is_err()
    );
    e.dispatch(Command::Cancel).ok();
    // LCK-T3.
    set_item_lock(&mut e, id, false);
    e.dispatch(Command::ItemTransform {
        item: id,
        transform: moved,
    })
    .unwrap();
    assert_eq!(e.scene.active().unwrap().transform.translation, DVec3::X);
}

/// LCK-T2 / LCK-06: item lock leaves component editing alone.
#[test]
fn lck_t2_locked_item_components_still_edit() {
    let (mut e, id) = cube();
    set_item_lock(&mut e, id, true);
    select_faces(&mut e, id, &[0]);
    let before = points(&e);
    move_selection(&mut e, DVec3::Y);
    assert_ne!(points(&e), before);
}

/// LCK-T4: two locked polygons are skipped by Select All and stay put when
/// the whole mesh moves; their shared points never move.
#[test]
fn lck_t4_locked_polygons_skip_selection_and_stay_put() {
    let (mut e, id) = cube();
    // Two neighbours (sharing an edge), so some points stay free.
    let adj = {
        let m = &e.scene.active().unwrap().mesh;
        (1..6u32)
            .find(|&f| {
                m.faces[f as usize]
                    .vertices
                    .iter()
                    .filter(|v| m.faces[0].vertices.contains(v))
                    .count()
                    == 2
            })
            .unwrap()
    };
    select_faces(&mut e, id, &[0, adj]);
    lock(&mut e, ElementOp::Selected);
    assert_eq!(
        elements(&e)
            .locked
            .faces
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, adj]
    );
    // LCK-22: locking drops them from the selection.
    assert!(e.scene.selection.faces.is_empty());
    select_all(&mut e);
    let mut faces = e.scene.selection.faces.clone();
    faces.sort();
    assert_eq!(faces.len(), 4);
    assert!(!faces.contains(&0) && !faces.contains(&adj));
    // Clicking a locked polygon selects nothing.
    select_faces(&mut e, id, &[0]);
    assert!(e.scene.selection.faces.is_empty());
    // Move everything (nothing selected): locked points stay.
    let mesh = e.scene.active().unwrap().mesh.clone();
    let pinned: Vec<usize> = [0usize, adj as usize]
        .iter()
        .flat_map(|&f| mesh.faces[f].vertices.iter().map(|&v| v as usize))
        .collect();
    let before = points(&e);
    move_selection(&mut e, DVec3::new(0., 3., 0.));
    let after = points(&e);
    for i in 0..after.len() {
        if pinned.contains(&i) {
            assert_eq!(after[i], before[i], "locked point {i} moved");
        }
    }
    assert!((0..after.len()).any(|i| after[i] != before[i]));
    // Undo and Redo replay exactly that.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(points(&e), after);
}

/// Loop selection in Edge mode skips edges of locked polygons.
#[test]
fn lck_t4_loop_skips_locked_edges() {
    let (mut e, id) = cube();
    select_faces(&mut e, id, &[0, 1]);
    lock(&mut e, ElementOp::Selected);
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    select_all(&mut e);
    let item = e.scene.active().unwrap();
    let p = Protection::of(&item.mesh, &item.elements).unwrap();
    assert!(!e.scene.selection.edges.is_empty());
    assert!(
        e.scene
            .selection
            .edges
            .iter()
            .all(|&(a, b)| !p.edge_locked(a, b))
    );
}

/// LCK-T5: J with nothing selected locks everything; I unlocks it all.
#[test]
fn lck_t5_lock_all_then_unlock() {
    let (mut e, _) = cube();
    lock(&mut e, ElementOp::Selected);
    assert_eq!(elements(&e).locked.faces.len(), 6);
    select_all(&mut e);
    assert!(e.scene.selection.faces.is_empty());
    lock(&mut e, ElementOp::Clear);
    assert!(elements(&e).locked.is_empty());
    // Nothing left to unlock: refused, no empty undo step.
    let steps = e.history.undo.len();
    assert!(
        e.dispatch(Command::EditElements {
            kind: ElementKind::Lock,
            op: ElementOp::Clear,
        })
        .is_err()
    );
    assert_eq!(e.history.undo.len(), steps);
}

/// LCK-T6: Invert after T4 unlocks the two and locks the other four.
#[test]
fn lck_t6_invert() {
    let (mut e, id) = cube();
    select_faces(&mut e, id, &[0, 1]);
    lock(&mut e, ElementOp::Selected);
    lock(&mut e, ElementOp::Invert);
    assert_eq!(
        elements(&e)
            .locked
            .faces
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 3, 4, 5]
    );
    // Lock Unselected: everything but the selection.
    lock(&mut e, ElementOp::Clear);
    select_faces(&mut e, id, &[3]);
    lock(&mut e, ElementOp::Unselected);
    assert_eq!(elements(&e).locked.faces.len(), 5);
    assert!(!elements(&e).locked.faces.contains(&3));
}

/// LCK-T8 / HID-T5: geometry lock is saved; hidden state is not.
#[test]
fn lck_t8_hid_t5_save_and_reopen() {
    let (mut e, id) = cube();
    set_item_lock(&mut e, id, true);
    select_faces(&mut e, id, &[0, 1]);
    lock(&mut e, ElementOp::Selected);
    select_faces(&mut e, id, &[4]);
    hide(&mut e, ElementOp::Selected);
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    let item = back.items.iter().find(|i| i.id == id).unwrap();
    assert!(item.is_locked());
    assert_eq!(
        item.elements
            .locked
            .faces
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(item.elements.hidden.is_empty());
}

/// HID-T1: hidden polygons leave the selection and Select All; the eye is
/// unchanged.
#[test]
fn hid_t1_hide_two_polygons() {
    let (mut e, id) = cube();
    select_faces(&mut e, id, &[0, 1]);
    hide(&mut e, ElementOp::Selected);
    assert!(e.scene.active().unwrap().visible);
    assert!(e.scene.selection.faces.is_empty());
    select_all(&mut e);
    let mut faces = e.scene.selection.faces.clone();
    faces.sort();
    assert_eq!(faces, vec![2, 3, 4, 5]);
    // HID-T4: Invert shows the two and hides the other four.
    hide(&mut e, ElementOp::Invert);
    assert_eq!(
        elements(&e)
            .hidden
            .faces
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 3, 4, 5]
    );
}

/// HID-T2: H with nothing selected hides everything; U shows it again.
#[test]
fn hid_t2_hide_all_unhide() {
    let (mut e, _) = cube();
    hide(&mut e, ElementOp::Selected);
    assert_eq!(elements(&e).hidden.faces.len(), 6);
    hide(&mut e, ElementOp::Clear);
    assert!(elements(&e).hidden.is_empty());
}

/// HID-13: Unhide with no item selected shows every item.
#[test]
fn hid_13_unhide_without_selection_reaches_every_item() {
    let (mut e, _) = cube();
    hide(&mut e, ElementOp::Selected);
    e.dispatch(Command::SelectItem(None)).unwrap();
    hide(&mut e, ElementOp::Clear);
    assert!(e.scene.items.iter().all(|i| i.elements.hidden.is_empty()));
}

/// HID-T3: Hide Unselected leaves only the selected polygon.
#[test]
fn hid_t3_hide_unselected() {
    let (mut e, id) = cube();
    select_faces(&mut e, id, &[2]);
    hide(&mut e, ElementOp::Unselected);
    let item = e.scene.active().unwrap();
    let p = Protection::of(&item.mesh, &item.elements).unwrap();
    let shown: Vec<usize> = (0..6).filter(|&f| !p.face_hidden[f]).collect();
    assert_eq!(shown, vec![2]);
}

/// HID-03: hiding a point hides every polygon using it.
#[test]
fn hid_03_hidden_point_hides_its_polygons() {
    let (mut e, _) = cube();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![0];
    hide(&mut e, ElementOp::Selected);
    let item = e.scene.active().unwrap();
    let p = Protection::of(&item.mesh, &item.elements).unwrap();
    let using: Vec<usize> = (0..6)
        .filter(|&f| item.mesh.faces[f].vertices.contains(&0))
        .collect();
    assert_eq!(using.len(), 3);
    for f in 0..6 {
        assert_eq!(p.face_hidden[f], using.contains(&f));
    }
}

/// HID-T6: hide is one undo step.
#[test]
fn hid_t6_undo_brings_it_back() {
    let (mut e, id) = cube();
    select_faces(&mut e, id, &[0]);
    hide(&mut e, ElementOp::Selected);
    assert!(!elements(&e).hidden.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert!(elements(&e).hidden.is_empty());
}

/// A topology edit away from locked polygons carries the lock to their
/// new indices; one that would change them is refused.
#[test]
fn topology_edits_carry_or_refuse() {
    let (mut e, id) = cube();
    select_faces(&mut e, id, &[5]);
    lock(&mut e, ElementOp::Selected);
    let locked_points: Vec<DVec3> = {
        let item = e.scene.active().unwrap();
        item.mesh.faces[5]
            .vertices
            .iter()
            .map(|&v| item.mesh.vertices[v as usize])
            .collect()
    };
    // Remove polygon 0: polygon 5 shifts down to 4.
    select_faces(&mut e, id, &[0]);
    e.dispatch(Command::RemovePolygons).unwrap();
    let item = e.scene.active().unwrap();
    assert_eq!(item.mesh.faces.len(), 5);
    let f = *item.elements.locked.faces.iter().next().unwrap();
    let now: Vec<DVec3> = item.mesh.faces[f as usize]
        .vertices
        .iter()
        .map(|&v| item.mesh.vertices[v as usize])
        .collect();
    assert_eq!(now, locked_points);
    // Bevel of everything (nothing selected) would change it: refused.
    e.dispatch(Command::ClearSelection).unwrap();
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let r = e.dispatch(Command::Preview(Parameters {
        inset: 0.1,
        shift: 0.1,
        ..e.session.live.as_ref().unwrap().params.clone()
    }));
    assert!(r.is_err(), "bevel over locked polygon must be refused");
    e.dispatch(Command::Cancel).ok();
    assert_eq!(
        e.scene.active().unwrap().mesh.vertices,
        before.active().unwrap().mesh.vertices
    );
}

/// Geometry lock is native version 28: a file with locks says so, and the
/// same data under version 27 is refused rather than read wrongly.
#[test]
fn geometry_lock_is_native_version_28() {
    let (mut e, _) = cube();
    lock(&mut e, ElementOp::Selected);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 28"), "{}", &text[..200]);
    let older = text.replacen("\"version\": 28", "\"version\": 27", 1);
    assert!(persistence::decode(older.as_bytes()).is_err());
    // Without locks the file stays at the older version.
    lock(&mut e, ElementOp::Clear);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 28"));
}
