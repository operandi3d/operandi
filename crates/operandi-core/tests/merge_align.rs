//! Create panel Merge and Align groups (MRG/UNM/GAL/QAL).
use glam::DVec3;
use operandi_core::*;

fn editor_with_cubes(positions: &[DVec3]) -> (Editor, Vec<u64>) {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for &p in positions {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
        item.transform.translation = p;
        ids.push(item.id);
    }
    e.synchronize_identities().unwrap();
    e.undo.clear();
    (e, ids)
}
fn world_points(e: &Editor, id: u64) -> Vec<DVec3> {
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    item.mesh
        .vertices
        .iter()
        .map(|v| e.scene.world_point(id, *v))
        .collect()
}

#[test]
fn merge_keeps_the_first_selected_and_world_positions() {
    let (mut e, ids) = editor_with_cubes(&[DVec3::ZERO, DVec3::new(3., 0., 0.)]);
    let second_world = world_points(&e, ids[1]);
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    e.dispatch(Command::ToggleItemSelection(ids[1])).unwrap();
    assert_eq!(e.scene.picked_items(), ids);
    e.dispatch(Command::MergeItems).unwrap();
    assert_eq!(e.scene.items.len(), 1);
    assert_eq!(e.scene.items[0].id, ids[0]);
    let merged = world_points(&e, ids[0]);
    assert_eq!(merged.len(), 16, "not welded");
    assert_eq!(&merged[8..], &second_world[..]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items.len(), 2);
}

#[test]
fn merge_with_one_mesh_does_nothing() {
    let (mut e, ids) = editor_with_cubes(&[DVec3::ZERO]);
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    let before = e.scene.clone();
    e.dispatch(Command::MergeItems).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn merged_away_children_keep_their_world_place() {
    let (mut e, ids) =
        editor_with_cubes(&[DVec3::ZERO, DVec3::new(3., 0., 0.), DVec3::new(0., 1., 0.)]);
    e.dispatch(Command::SetParent {
        item: ids[2],
        parent: Some(ids[1]),
        keep_position: false,
    })
    .unwrap();
    let child_world = world_points(&e, ids[2]);
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    e.dispatch(Command::ToggleItemSelection(ids[1])).unwrap();
    e.dispatch(Command::MergeItems).unwrap();
    let child: Vec<DVec3> = world_points(&e, ids[2]);
    for (a, b) in child.iter().zip(&child_world) {
        assert!((*a - *b).length() < 1e-9);
    }
}

#[test]
fn unmerge_splits_by_connectivity() {
    let (mut e, ids) = editor_with_cubes(&[DVec3::ZERO, DVec3::new(3., 0., 0.)]);
    e.dispatch(Command::SelectItem(Some(ids[0]))).unwrap();
    e.dispatch(Command::ToggleItemSelection(ids[1])).unwrap();
    e.dispatch(Command::MergeItems).unwrap();
    let name = e.scene.items[0].name.clone();
    e.dispatch(Command::UnmergeItem).unwrap();
    assert_eq!(e.scene.items.len(), 2);
    assert_eq!(e.scene.items[1].name, format!("{name} (2)"));
    assert_eq!(e.scene.items[0].mesh.faces.len(), 6);
    assert_eq!(e.scene.items[1].mesh.faces.len(), 6);
    assert_eq!(e.scene.selection.item, Some(ids[0]));
    // A one-piece mesh is left alone.
    let before = e.scene.clone();
    e.dispatch(Command::UnmergeItem).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items.len(), 1);
}

#[test]
fn ground_align_drops_to_ground_and_origin() {
    let (mut e, ids) = editor_with_cubes(&[DVec3::new(1., 2., 3.)]);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::GroundAlign(GroundAlign::default()))
        .unwrap();
    let w = world_points(&e, ids[0]);
    let min_y = w.iter().map(|p| p.y).fold(f64::MAX, f64::min);
    assert!(min_y.abs() < 1e-9);
    assert_eq!(
        e.scene.items[0].transform.translation,
        DVec3::new(1., 2., 3.),
        "item unchanged"
    );
    e.dispatch(Command::GroundAlign(GroundAlign {
        mode: GroundMode::DropToOrigin,
        ..Default::default()
    }))
    .unwrap();
    let w = world_points(&e, ids[0]);
    let (lo, hi) = w.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(l, h), p| (l.min(*p), h.max(*p)),
    );
    assert!((DVec3::new((lo.x + hi.x) / 2., lo.y, (lo.z + hi.z) / 2.)).length() < 1e-9);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(world_points(&e, ids[0])[0].y, 1.5);
}

#[test]
fn ground_align_refuses_items_mode() {
    let (mut e, _) = editor_with_cubes(&[DVec3::ZERO]);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    let err = e
        .dispatch(Command::GroundAlign(GroundAlign::default()))
        .unwrap_err();
    assert!(err.contains("vertices, edges or polygons"), "{err}");
}

#[test]
fn rotate_to_ground_turns_the_selected_face_down() {
    let (mut e, ids) = editor_with_cubes(&[DVec3::ZERO]);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    let up = (0..6).find(|&f| mesh.normal(f).y > 0.9).unwrap() as u32;
    e.scene.selection.faces = vec![up];
    e.dispatch(Command::GroundAlign(GroundAlign {
        mode: GroundMode::RotateToGround,
        ..Default::default()
    }))
    .unwrap();
    let mesh = &e.scene.items.iter().find(|i| i.id == ids[0]).unwrap().mesh;
    assert!(mesh.normal(up as usize).y < -0.999);
}

#[test]
fn quick_align_lines_up_one_axis_and_flattens() {
    let (mut e, ids) = editor_with_cubes(&[DVec3::ZERO]);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    let top = (0..6).find(|&f| mesh.normal(f).y > 0.9).unwrap() as u32;
    e.scene.selection.faces = vec![top];
    let q = QuickAlign {
        target: DVec3::new(9., 2., 9.),
        kind: QuickAlignKind::Position(1),
        connected: false,
        flatten: false,
        islands: SelectionAxisMode::SelectionIsland,
    };
    e.dispatch(Command::QuickAlign(q)).unwrap();
    let w = world_points(&e, ids[0]);
    let mesh = &e.scene.active().unwrap().mesh;
    let top_points: Vec<DVec3> = mesh.faces[top as usize]
        .vertices
        .iter()
        .map(|&v| w[v as usize])
        .collect();
    assert!(top_points.iter().all(|p| (p.y - 2.).abs() < 1e-9));
    // Only the selection moved (Connected off): the bottom stays.
    assert!(w.iter().any(|p| (p.y + 0.5).abs() < 1e-9));
    // Connected moves the whole piece.
    e.dispatch(Command::QuickAlign(QuickAlign {
        target: DVec3::new(0., 5., 0.),
        connected: true,
        ..q
    }))
    .unwrap();
    let w = world_points(&e, ids[0]);
    assert!(w.iter().all(|p| p.y > 2.));
}
