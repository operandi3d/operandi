//! Pick order (rig-pick-order): the item picked last is primary, earlier
//! picks stay in the order they were picked, so Parent, Constrain, Bind
//! and IK read the selection the right way round.
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn three() -> (Editor, [u64; 3]) {
    let mut e = Editor::default();
    for name in ["Passenger", "Car", "Road"] {
        e.scene.new_mesh_named(Some(name.into())).unwrap();
        let cube = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 0).unwrap();
        e.scene.items.last_mut().unwrap().mesh = cube.into();
    }
    let ids = [
        e.scene.items[0].id,
        e.scene.items[1].id,
        e.scene.items[2].id,
    ];
    (e, ids)
}

#[test]
fn toggling_makes_the_new_pick_primary_and_keeps_pick_order() {
    let (mut e, [p, c, r]) = three();
    e.dispatch(Command::SelectItem(Some(p))).unwrap();
    e.dispatch(Command::ToggleItemSelection(c)).unwrap();
    assert_eq!(e.scene.selection.item, Some(c), "the last pick is primary");
    assert_eq!(e.scene.picked_items(), vec![p, c]);
    e.dispatch(Command::ToggleItemSelection(r)).unwrap();
    assert_eq!(e.scene.picked_items(), vec![p, c, r]);
    assert_eq!(e.scene.selected_items(), vec![r, p, c], "primary first");
    // Taking the primary out promotes the most recent remaining pick.
    e.dispatch(Command::ToggleItemSelection(r)).unwrap();
    assert_eq!(e.scene.selection.item, Some(c));
    assert_eq!(e.scene.picked_items(), vec![p, c]);
    // Taking an earlier pick out keeps the primary.
    e.dispatch(Command::ToggleItemSelection(p)).unwrap();
    assert_eq!(e.scene.selection.item, Some(c));
    assert_eq!(e.scene.picked_items(), vec![c]);
}

#[test]
fn parent_and_constrain_go_the_way_they_were_picked() {
    // Click Passenger, Cmd-click Car, Parent: the Car is the parent.
    let (mut e, [p, c, _]) = three();
    e.dispatch(Command::SelectItem(Some(p))).unwrap();
    e.dispatch(Command::ToggleItemSelection(c)).unwrap();
    ok(&mut e, "operandi.rig.parent");
    assert_eq!(
        e.scene.item(p).unwrap().parent,
        Some(c),
        "Passenger under Car"
    );
    assert_eq!(e.scene.item(c).unwrap().parent, None);
    // Constrain: the first picked follows the second.
    let (mut e, [p, c, _]) = three();
    e.dispatch(Command::SelectItem(Some(p))).unwrap();
    e.dispatch(Command::ToggleItemSelection(c)).unwrap();
    ok(&mut e, "operandi.rig.constrain kind:position");
    let piece = &e.scene.animation.rig.pieces[0];
    assert_eq!((piece.item, piece.targets.clone()), (p, vec![c]));
}

#[test]
fn a_mesh_keeps_its_components_when_another_becomes_primary() {
    let (mut e, [p, c, _]) = three();
    e.dispatch(Command::SelectItem(Some(p))).unwrap();
    e.scene.selection.vertices = vec![0, 1];
    e.dispatch(Command::ToggleItemSelection(c)).unwrap();
    assert!(
        e.scene.selection.vertices.is_empty(),
        "the new primary has its own (none)"
    );
    assert_eq!(
        e.scene.selection.others.get(&p).map(|o| o.vertices.clone()),
        Some(vec![0, 1])
    );
    e.dispatch(Command::ToggleItemSelection(c)).unwrap();
    assert_eq!(e.scene.selection.item, Some(p));
    assert_eq!(
        e.scene.selection.vertices,
        vec![0, 1],
        "back with its components"
    );
}
