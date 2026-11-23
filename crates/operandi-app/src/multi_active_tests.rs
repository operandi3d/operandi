//! Multiple active meshes (operandi-core multi_active.rs): every selected
//! mesh picks, selects and lassos its own components in the viewport.
use super::*;
use operandi_core::cube_geometry;

/// Two cubes side by side seen from the front, both selected (`left`
/// primary), Polygon mode, nothing selected.
fn side_by_side() -> (Operandi, u64, u64, Rect) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::NewMesh);
    let (left, right) = (a.editor.scene.items[0].id, a.editor.scene.items[1].id);
    a.editor.scene.items[0].mesh = cube_geometry(DVec3::new(-2., 0., 0.), DVec3::ONE)
        .unwrap()
        .into();
    a.editor.scene.items[1].mesh = cube_geometry(DVec3::new(2., 0., 0.), DVec3::ONE)
        .unwrap()
        .into();
    a.run(Command::SelectItems(vec![left, right]));
    a.run(Command::Mode(Mode::Polygon));
    a.camera.view = View::Front;
    a.camera.distance = 12.;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
    (a, left, right, r)
}
fn at(a: &Operandi, world: DVec3, r: Rect) -> Pos2 {
    a.camera.project(world, r).unwrap().0
}

#[test]
fn clicks_select_components_on_every_active_mesh() {
    let (mut a, left, right, r) = side_by_side();
    let on_left = at(&a, DVec3::new(-2., 0., 0.5), r);
    let on_right = at(&a, DVec3::new(2., 0., 0.5), r);
    a.select_at(on_right, r, Modifiers::NONE);
    let s = &a.editor.scene.selection;
    assert_eq!(s.item, Some(left), "the primary stays");
    assert_eq!(s.faces_of(right).len(), 1);
    assert!(s.faces.is_empty());
    a.select_at(on_left, r, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.faces_of(left).len(), 1);
    assert_eq!(a.editor.scene.selection.faces_of(right).len(), 1);
    // A plain click clears both, then selects.
    a.select_at(on_left, r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.faces_of(left).len(), 1);
    assert!(a.editor.scene.selection.faces_of(right).is_empty());
    // Ctrl-click removes.
    a.select_at(on_left, r, Modifiers::CTRL);
    assert!(a.editor.scene.selection.faces.is_empty());
    // Empty space clears components but keeps both meshes active.
    a.select_at(on_right, r, Modifiers::NONE);
    a.select_at(pos2(5., 5.), r, Modifiers::NONE);
    assert!(a.editor.scene.selection.others.is_empty());
    assert_eq!(a.editor.scene.active_meshes(), vec![left, right]);
}

#[test]
fn vertices_on_the_other_mesh_pick() {
    let (mut a, left, right, r) = side_by_side();
    a.run(Command::Mode(Mode::Vertex));
    let corner = at(&a, DVec3::new(2.5, 0.5, 0.5), r);
    assert!(matches!(
        a.pick_component(corner, r),
        Some((id, Component::Vertex(_))) if id == right
    ));
    a.select_at(corner, r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.item, Some(left));
    assert_eq!(a.editor.scene.selection.vertices_of(right).len(), 1);
}

#[test]
fn lasso_selects_on_every_active_mesh() {
    let (mut a, left, right, r) = side_by_side();
    let region = [
        pos2(1., 1.),
        pos2(699., 1.),
        pos2(699., 699.),
        pos2(1., 699.),
    ];
    let hits = a.lasso_hits(&region, false, r).unwrap();
    let ids: Vec<u64> = hits.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![left, right]);
    a.run(Command::BeginSelection);
    a.finish_lasso(&region, false, Modifiers::NONE, r);
    assert!(!a.editor.scene.selection.faces_of(left).is_empty());
    assert!(!a.editor.scene.selection.faces_of(right).is_empty());
}

#[test]
fn shift_click_in_item_mode_makes_both_meshes_active() {
    let (mut a, left, right, r) = side_by_side();
    a.run(Command::Mode(Mode::Item));
    let on_left = at(&a, DVec3::new(-2., 0., 0.5), r);
    let on_right = at(&a, DVec3::new(2., 0., 0.5), r);
    a.select_at(on_left, r, Modifiers::NONE);
    assert_eq!(a.editor.scene.picked_items(), vec![left]);
    a.select_at(on_right, r, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.picked_items(), vec![left, right]);
    a.run(Command::Mode(Mode::Polygon));
    // The primary (last picked) comes first.
    assert_eq!(a.editor.scene.active_meshes(), vec![right, left]);
    a.run(Command::Mode(Mode::Item));
    a.select_at(on_right, r, Modifiers::CTRL);
    assert_eq!(a.editor.scene.picked_items(), vec![left]);
    // Ctrl toggles: again puts it back.
    a.select_at(on_right, r, Modifiers::CTRL);
    assert_eq!(a.editor.scene.picked_items(), vec![left, right]);
}
