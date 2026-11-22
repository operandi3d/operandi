use glam::DVec3;
use operandi_core::*;

#[test]
fn pen_triple_merge_remove_preserves_boundary_points_and_history() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let unrelated = e.scene.selection.item.unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::new(4., 0., 0.), DVec3::ONE, 1)
        .unwrap()
        .into();
    e.scene.active_mut().unwrap().mesh.faces[0].part = Some("unrelated".into());
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.select_item(Some(target));
    e.scene.selection.mode = Mode::Polygon;
    e.synchronize_identities().unwrap();
    let initial = e.scene.clone();
    let initial_ids = e.identities.current.clone();
    e.dispatch(Command::PenStart).unwrap();
    let points = [DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y];
    for p in points {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
    let draft = e.clone();
    assert!(e.dispatch(Command::RemovePolygons).is_err());
    assert!(e == draft);
    e.dispatch(Command::Drop).unwrap();
    let quad = e.scene.clone();
    let quad_ids = e.identities.current.clone();
    let m = &quad.active().unwrap().mesh;
    assert_eq!(m.vertices, points);
    assert_eq!(m.faces[0].vertices, vec![0, 1, 2, 3]);
    assert_eq!(
        quad.items.iter().find(|i| i.id == unrelated),
        initial.items.iter().find(|i| i.id == unrelated)
    );
    e.dispatch(Command::SetPart(PartAssignment {
        name: "Road".into(),
        expected: None,
    }))
    .unwrap();
    let tagged = e.scene.clone();
    let tagged_ids = e.identities.current.clone();
    assert_eq!(
        tagged.active().unwrap().mesh.faces[0].part.as_deref(),
        Some("Road")
    );
    e.dispatch(Command::Triple).unwrap();
    let triple = e.scene.clone();
    let triple_ids = e.identities.current.clone();
    assert_eq!(triple.active().unwrap().mesh.faces.len(), 2);
    for f in &triple.active().unwrap().mesh.faces {
        assert_eq!(f.part.as_deref(), Some("Road"));
        assert_eq!(f.uv, None);
    }
    e.dispatch(Command::FlipPolygons).unwrap();
    let flipped = e.scene.clone();
    let flipped_ids = e.identities.current.clone();
    e.dispatch(Command::MergePolygons).unwrap();
    let merged = e.scene.clone();
    let merged_ids = e.identities.current.clone();
    let face = &merged.active().unwrap().mesh.faces[0];
    assert_eq!(face.vertices.len(), 4);
    assert_eq!(face.part.as_deref(), Some("Road"));
    assert_eq!(face.uv, None);
    assert!(merged.active().unwrap().mesh.uv_maps.is_empty());
    for i in 0..4 {
        assert_eq!((face.vertices[(i + 1) % 4] + 4 - face.vertices[i]) % 4, 3);
    }
    assert_eq!(merged.active().unwrap().mesh.vertices, points);
    // Merge leaves the merged polygon selected (CMD-O-merge-*), so Select
    // All changes nothing and records no step.
    assert_eq!(merged.selection.faces, vec![0]);
    e.dispatch(Command::Selection(SelectionOperation::All))
        .unwrap();
    assert_eq!(e.scene, merged);
    e.dispatch(Command::RemovePolygons).unwrap();
    let removed = e.scene.clone();
    let removed_ids = e.identities.current.clone();
    assert!(removed.active().unwrap().mesh.faces.is_empty());
    assert_eq!(removed.active().unwrap().mesh.vertices, points);
    for (expected, ids) in [
        (&merged, &merged_ids),
        (&flipped, &flipped_ids),
        (&triple, &triple_ids),
        (&tagged, &tagged_ids),
        (&quad, &quad_ids),
        (&initial, &initial_ids),
    ] {
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(&e.scene, expected);
        assert_eq!(&e.identities.current, ids);
    }
    for (expected, ids) in [
        (&quad, &quad_ids),
        (&tagged, &tagged_ids),
        (&triple, &triple_ids),
        (&flipped, &flipped_ids),
        (&merged, &merged_ids),
        (&removed, &removed_ids),
    ] {
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(&e.scene, expected);
        assert_eq!(&e.identities.current, ids);
    }
}
