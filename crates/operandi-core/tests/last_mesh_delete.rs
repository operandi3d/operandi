//! The app asks before the scene's only mesh item, or all of its mesh
//! items, are deleted (Jason, 2026-11-21). `Scene::removes_last_mesh`
//! decides when it asks, `Scene::last_mesh_question` what it asks.
use operandi_core::camera_light::LightSettings;
use operandi_core::item_delete::ChildrenOnDelete;
use operandi_core::*;

fn scene_with(meshes: usize) -> (Scene, Vec<u64>, u64) {
    let mut s = Scene::default();
    let mut ids = Vec::new();
    for _ in 0..meshes {
        s.new_mesh().unwrap();
        ids.push(s.selection.item.unwrap());
    }
    let light = s
        .add_light(LightSettings::default(), Transform::default())
        .unwrap();
    (s, ids, light)
}

#[test]
fn only_the_last_mesh_counts() {
    let (s, ids, light) = scene_with(1);
    let mode = ChildrenOnDelete::default();
    assert!(s.removes_last_mesh(&ids, mode));
    assert!(s.removes_last_mesh(&[ids[0], light], mode));
    // A light isn't a mesh item: deleting it alone never asks.
    assert!(!s.removes_last_mesh(&[light], mode));
    // Two meshes: deleting one leaves a mesh; deleting both asks.
    let (two, ids, light) = scene_with(2);
    assert!(!two.removes_last_mesh(&ids[..1], mode));
    assert!(!two.removes_last_mesh(&[ids[1], light], mode));
    assert!(two.removes_last_mesh(&ids, mode));
    assert!(two.removes_last_mesh(&[ids[0], ids[1], light], mode));
    // No mesh at all: nothing to warn about.
    let (none, _, light) = scene_with(0);
    assert!(!none.removes_last_mesh(&[light], mode));
}

#[test]
fn a_child_mesh_counts_only_when_children_go_too() {
    let (mut s, ids, light) = scene_with(1);
    s.set_parent(ids[0], Some(light), true).unwrap();
    assert!(s.removes_last_mesh(&[light], ChildrenOnDelete::Delete));
    assert!(!s.removes_last_mesh(&[light], ChildrenOnDelete::MoveToRoot));
    assert_eq!(
        s.deleted_with(&[light], ChildrenOnDelete::Delete),
        [light, ids[0]].into_iter().collect()
    );
}

#[test]
fn the_question_fits_the_mesh_count() {
    let (one, _, _) = scene_with(1);
    assert_eq!(one.last_mesh_question(), item_delete::LAST_MESH_QUESTION);
    let (two, _, _) = scene_with(2);
    assert_eq!(two.last_mesh_question(), item_delete::ALL_MESHES_QUESTION);
    let (three, _, _) = scene_with(3);
    assert_eq!(three.last_mesh_question(), item_delete::ALL_MESHES_QUESTION);
}
