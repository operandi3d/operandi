//! Pick Other Meshes in Component Modes (off by default): component-mode
//! picks see only the active mesh, and other meshes don't block it.
use super::*;
use crate::prefs_file::PrefsFile;
use crate::selection_preferences::SelectionPreferences;
use operandi_core::cube_geometry;

/// A big cube near the eye (Front view looks down -Z) and a small one
/// behind it at the origin, which is active. Returns (app, front, back).
fn two_cubes() -> (Operandi, u64, u64, Rect) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::NewMesh);
    let (front, back) = (a.editor.scene.items[0].id, a.editor.scene.items[1].id);
    a.editor.scene.items[0].mesh = cube_geometry(DVec3::new(0., 0., 3.), DVec3::splat(3.))
        .unwrap()
        .into();
    a.editor.scene.items[1].mesh = cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap().into();
    a.editor.scene.selection.item = Some(back);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.camera.view = View::Front;
    a.camera.distance = 12.;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(700., 700.));
    (a, front, back, r)
}

fn at(a: &Operandi, world: DVec3, r: Rect) -> Pos2 {
    a.camera.project(world, r).unwrap().0
}

#[test]
fn off_by_default_polygon_ray_passes_through_other_mesh_to_active() {
    assert!(!SelectionPreferences::default().pick_other_meshes);
    let (mut a, _, back, r) = two_cubes();
    let p = r.center();
    assert!(matches!(
        a.pick_component(p, r),
        Some((id, Component::Polygon(_))) if id == back
    ));
    a.select_at(p, r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.item, Some(back));
    assert_eq!(a.editor.scene.selection.faces.len(), 1);
}

#[test]
fn off_vertex_and_edge_picks_see_through_other_meshes() {
    let (mut a, _, back, r) = two_cubes();
    a.editor.scene.selection.mode = Mode::Vertex;
    let corner = at(&a, DVec3::splat(0.5), r);
    assert!(matches!(
        a.pick_component(corner, r),
        Some((id, Component::Vertex(_))) if id == back
    ));
    a.editor.scene.selection.mode = Mode::Edge;
    let edge_mid = at(&a, DVec3::new(0., 0.5, 0.5), r);
    assert!(matches!(
        a.pick_component(edge_mid, r),
        Some((id, Component::Edge(..))) if id == back
    ));
    // Closest rollovers and Mouse Regions use the same limit.
    assert!(a.pick_closest(r.center(), r).is_some_and(|h| h.0 == back));
}

#[test]
fn off_click_on_other_mesh_leaves_the_selection_alone() {
    let (mut a, front, back, r) = two_cubes();
    // Move the active cube aside so nothing of it is under the front cube.
    a.editor.scene.items[1].mesh = cube_geometry(DVec3::new(5., 0., 0.), DVec3::ONE)
        .unwrap()
        .into();
    a.run(Command::Select {
        item: back,
        hit: Component::Polygon(0),
        replace: true,
        remove: false,
    });
    let before = a.editor.scene.selection.clone();
    a.select_at(r.center(), r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection, before);
    assert_ne!(a.editor.scene.selection.item, Some(front));
    // Empty space still clears, as before.
    a.select_at(Pos2::new(5., 5.), r, Modifiers::NONE);
    assert!(a.editor.scene.selection.faces.is_empty());
    assert_eq!(a.editor.scene.selection.item, Some(back));
}

#[test]
fn on_keeps_the_old_behaviour() {
    let (mut a, front, _, r) = two_cubes();
    a.selection_preferences.pick_other_meshes = true;
    assert!(
        a.pick_component(r.center(), r)
            .is_some_and(|h| h.0 == front)
    );
    a.select_at(r.center(), r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.item, Some(front));
    a.editor.scene.selection.mode = Mode::Vertex;
    // The front cube hides the active one's corner again.
    let corner = at(&a, DVec3::splat(0.5), r);
    assert!(a.pick_component(corner, r).is_none_or(|h| h.0 == front));
}

#[test]
fn no_shown_active_mesh_picks_any_mesh() {
    let (mut a, front, back, r) = two_cubes();
    a.editor.scene.selection.item = None;
    // VP-04: with nothing selected the last selected mesh is still the
    // active one, so picks still see only it.
    a.editor.scene.last_mesh = operandi_core::last_mesh::LastMesh(Some(back));
    assert!(a.pick_component(r.center(), r).is_some_and(|h| h.0 == back));
    // No mesh selected since the scene opened: any mesh picks.
    a.editor.scene.last_mesh = Default::default();
    assert!(
        a.pick_component(r.center(), r)
            .is_some_and(|h| h.0 == front)
    );
    // A hidden active mesh doesn't limit picking either.
    a.editor.scene.selection.item = Some(back);
    a.editor.scene.items[1].visible = false;
    assert!(a.editor.scene.active_meshes().is_empty());
    assert!(
        a.pick_component(r.center(), r)
            .is_some_and(|h| h.0 == front)
    );
}

#[test]
fn item_mode_is_unaffected() {
    let (mut a, front, _, r) = two_cubes();
    a.editor.scene.selection.mode = Mode::Item;
    assert_eq!(
        a.pick_component(r.center(), r),
        Some((front, Component::Item))
    );
    a.select_at(r.center(), r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.item, Some(front));
}

#[test]
fn preference_round_trips_and_old_files_load_it_off() {
    let dir = std::env::temp_dir().join(format!("operandi-pick-other-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("selection-preferences.json");
    std::fs::write(&path, br#"{"version":1,"mouse_regions":true}"#).unwrap();
    let old = SelectionPreferences::load(&path);
    assert!(old.mouse_regions);
    assert!(!old.pick_other_meshes);
    let mut on = old;
    on.pick_other_meshes = true;
    on.save(&path).unwrap();
    assert_eq!(SelectionPreferences::load(&path), on);
    std::fs::remove_dir_all(dir).unwrap();
}
