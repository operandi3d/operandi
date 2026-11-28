use glam::{DVec2, DVec3};
use operandi_core::*;
fn fixture() -> Editor {
    let mut editor = Editor::default();
    editor.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut editor.scene.active_mut().unwrap().mesh;
    for island in 0..2 {
        let start = mesh.vertices.len() as u32;
        // Coincident but disconnected islands prove connectivity is topological.
        for y in 0..2 {
            for x in 0..3 {
                mesh.vertices.push(DVec3::new(x as f64, y as f64, 0.));
            }
        }
        for x in 0..2 {
            mesh.faces.push(Face {
                vertices: vec![start + x, start + x + 1, start + x + 4, start + x + 3],
                uv: Some(vec![DVec2::new(island as f64 + x as f64 * 0.25, 0.); 4]),
                material: format!("island{island}"),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    editor.scene.selection.faces = vec![0, 1];
    editor
}
#[test]
fn conversion_and_boundary_are_distinct_and_undoable() {
    let mut e = fixture();
    let original = e.scene.clone();
    e.dispatch(Command::Selection(SelectionOperation::Convert(Mode::Edge)))
        .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 7);
    assert_eq!(
        e.scene.active().unwrap().mesh,
        original.active().unwrap().mesh
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, original);
    e.dispatch(Command::Selection(SelectionOperation::Boundary))
        .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 6);
    assert_eq!(
        e.scene.active().unwrap().mesh,
        original.active().unwrap().mesh
    );
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    let path = std::env::temp_dir().join(format!(
        "operandi-m15-selection-{}.opscene",
        std::process::id()
    ));
    persistence::save(&path, &after).unwrap();
    assert_eq!(persistence::load(&path).unwrap(), after);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn conversion_requires_complete_defining_membership() {
    let mut e = fixture();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0, 1, 4];
    e.dispatch(Command::Selection(SelectionOperation::Convert(
        Mode::Polygon,
    )))
    .unwrap();
    assert!(e.scene.selection.faces.is_empty());
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices.push(3);
    e.dispatch(Command::Selection(SelectionOperation::Convert(
        Mode::Polygon,
    )))
    .unwrap();
    assert_eq!(e.scene.selection.faces, vec![0]);
    e.dispatch(Command::Selection(SelectionOperation::Convert(Mode::Edge)))
        .unwrap();
    e.dispatch(Command::Selection(SelectionOperation::Convert(
        Mode::Polygon,
    )))
    .unwrap();
    assert_eq!(e.scene.selection.faces, vec![0]);
}
#[test]
fn connected_excludes_coincident_disconnected_island_and_invert_is_involution() {
    let mut e = fixture();
    e.scene.selection.faces = vec![0];
    let original = e.scene.clone();
    e.dispatch(Command::Selection(SelectionOperation::Connected))
        .unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Selection(SelectionOperation::InvertConnected))
        .unwrap();
    assert_eq!(e.scene.selection.faces, vec![1]);
    e.dispatch(Command::Undo).unwrap();
    for _ in 0..2 {
        e.dispatch(Command::Selection(SelectionOperation::Invert))
            .unwrap();
    }
    assert_eq!(e.scene, original);
}
#[test]
fn invalid_or_unsupported_input_is_atomic() {
    let mut e = fixture();
    e.scene.selection.faces = vec![999];
    let original = e.scene.clone();
    assert!(
        e.dispatch(Command::Selection(SelectionOperation::Boundary))
            .is_err()
    );
    assert_eq!(e.scene, original);
    e.scene.selection.faces.clear();
    e.scene.selection.mode = Mode::Item;
    let original = e.scene.clone();
    assert!(
        e.dispatch(Command::Selection(SelectionOperation::All))
            .is_err()
    );
    assert_eq!(e.scene, original);
}
#[test]
fn independent_cube_sides_match_original_oracle() {
    let bounds = BoxBounds {
        min: DVec3::new(-1., 0., -0.5),
        max: DVec3::new(1., 2., 0.5),
    };
    let resized = bounds
        .edit_side(0, true, 1.5, BoxHandleMode::Resize)
        .unwrap();
    assert_eq!(resized.min, bounds.min);
    assert_eq!(resized.position(), DVec3::new(0.25, 1., 0.));
    assert_eq!(resized.size(), DVec3::new(2.5, 2., 1.));
    let centered = bounds
        .edit_side(0, true, 1.5, BoxHandleMode::FromCenter)
        .unwrap();
    assert_eq!(centered.position(), bounds.position());
    assert_eq!(centered.min.x, -1.5);
    let moved = bounds
        .edit_side(0, true, 1.5, BoxHandleMode::MoveAll)
        .unwrap();
    assert_eq!(moved.size(), bounds.size());
    assert_eq!(moved.position().x, 0.5);
    // No clamping (unit-primitive-2026-11-05 rule 5): Max past Min is kept
    // as a negative size.
    let crossed = bounds
        .edit_side(0, true, -2., BoxHandleMode::Resize)
        .unwrap();
    assert!(crossed.size().x < 0.);
}
#[test]
fn flat_preview_replaces_live_geometry_and_preserves_sparse_maps() {
    let mut e = fixture();
    let item = e.scene.active_mut().unwrap();
    item.mesh
        .uv_maps
        .insert("seam".into(), vec![vec![Some(DVec2::new(0.2, 0.7)); 4]; 4]);
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    for size in [
        DVec3::new(2., 0., 3.),
        DVec3::new(2., 1., 3.),
        DVec3::new(2., 0., 3.),
    ] {
        e.dispatch(Command::Preview(Parameters {
            size,
            ..Default::default()
        }))
        .unwrap();
        let m = &e.scene.active().unwrap().mesh;
        let count = if size.y == 0. { 1 } else { 6 };
        assert_eq!(m.faces.len(), 4 + count);
        assert_eq!(
            &m.uv_maps["seam"][..4],
            &before.active().unwrap().mesh.uv_maps["seam"]
        );
        assert!(m.uv_maps["seam"][4..].iter().flatten().all(Option::is_none));
        m.validate().unwrap();
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);
}
#[test]
fn regular_quad_ring_loop_and_grow_shrink_are_distinct() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::Selection(SelectionOperation::Grow))
        .unwrap();
    assert_eq!(e.scene.selection.faces.len(), 5);
    e.dispatch(Command::Selection(SelectionOperation::Shrink))
        .unwrap();
    assert_eq!(e.scene.selection.faces, vec![0]);
    e.scene.selection.faces = vec![0, 2];
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(e.scene.selection.faces.len(), 4);
    assert_eq!(&e.scene.selection.faces[..2], &[0, 2]);
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(0, 3)];
    e.dispatch(Command::Selection(SelectionOperation::Ring))
        .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 4);
    assert_eq!(e.scene.active().unwrap().mesh, mesh);
    let mut grid = fixture();
    grid.scene.selection.mode = Mode::Edge;
    grid.scene.selection.edges = vec![(0, 3)];
    grid.dispatch(Command::Selection(SelectionOperation::Ring))
        .unwrap();
    assert_eq!(grid.scene.selection.edges.len(), 3);
    // (0, 1) is open border: its loop runs round the whole border (PF-03).
    grid.scene.selection.edges = vec![(0, 1)];
    grid.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let edges = &grid.scene.selection.edges;
    assert!(edges.contains(&(0, 1)) && edges.contains(&(1, 2)) && edges.len() > 2);
}
#[test]
fn segmented_cube_shared_grid_counts_and_budget() {
    let mesh = primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 3., 4.), [2, 3, 4]).unwrap();
    let sum = 2 * 3 + 3 * 4 + 2 * 4;
    assert_eq!(mesh.faces.len(), 2 * sum);
    assert_eq!(mesh.edges().len(), 4 * sum);
    assert_eq!(mesh.vertices.len(), 2 * sum + 2);
    assert!(mesh.is_closed());
    mesh.validate().unwrap();
    let flat = primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 0., 4.), [2, 3, 4]).unwrap();
    assert_eq!(flat.faces.len(), 8);
    assert_eq!(flat.vertices.len(), 15);
    assert!(primitive::segmented_cube(DVec3::ZERO, DVec3::ONE, [u32::MAX; 3]).is_err());
    let mut e = fixture();
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    assert!(
        e.dispatch(Command::Preview(Parameters {
            segments: [u32::MAX; 3],
            ..Default::default()
        }))
        .is_err()
    );
    assert_eq!(e.scene, before);
}
#[test]
fn close_loop_selects_smaller_separated_region_and_refuses_ties() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 4., 2.), [1, 4, 1])
            .unwrap()
            .into();
    let mesh = &e.scene.active().unwrap().mesh;
    let a = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).x > 0.9 && (mesh.center(f).y + 0.5).abs() < 1e-9)
        .unwrap();
    let b = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).z > 0.9 && (mesh.center(f).y + 0.5).abs() < 1e-9)
        .unwrap();
    e.scene.selection.faces = vec![a as u32, b as u32];
    let original = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Selection(SelectionOperation::CloseLoop))
        .unwrap();
    assert_eq!(e.scene.selection.faces.len(), 9);
    assert_eq!(e.scene.active().unwrap().mesh, original);
    assert!(
        e.scene
            .selection
            .faces
            .iter()
            .all(|&f| original.center(f as usize).y < 0.)
    );
}
#[test]
fn inactive_apply_uses_retained_cube_once_without_arming() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let params = Parameters {
        size: DVec3::new(2., 3., 4.),
        ..Default::default()
    };
    e.dispatch(Command::Preview(params)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let checkpoint = e.history.undo.len();
    e.dispatch(Command::Apply).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), checkpoint + 1);
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 12);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
}
/// The loop through two polygons splits the mesh into two equal halves:
/// no smaller side, so Close Loop completes the ring itself (PF-05) and
/// the geometry stays as it was.
#[test]
fn close_loop_tie_completes_the_ring() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 3., 2.), [1, 3, 1])
            .unwrap()
            .into();
    let mesh = &e.scene.active().unwrap().mesh;
    let a = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).x > 0.9 && mesh.center(f).y.abs() < 1e-9)
        .unwrap();
    let b = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).z > 0.9 && mesh.center(f).y.abs() < 1e-9)
        .unwrap();
    e.scene.selection.faces = vec![a as u32, b as u32];
    let mesh = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Selection(SelectionOperation::CloseLoop))
        .unwrap();
    assert_eq!(e.scene.selection.faces.len(), 4);
    assert_eq!(e.scene.active().unwrap().mesh, mesh);
}
#[test]
fn unsupported_topology_and_empty_seeds_report_without_mutation() {
    let mut e = fixture();
    e.scene.active_mut().unwrap().mesh.faces[0].vertices.pop();
    e.scene.active_mut().unwrap().mesh.faces[0]
        .uv
        .as_mut()
        .unwrap()
        .pop();
    e.scene.selection.faces = vec![0];
    let original = e.scene.clone();
    // A one-polygon Loop needs a quad to know its direction; a triangle seed
    // reports without changing anything.
    assert!(
        e.dispatch(Command::Selection(SelectionOperation::Loop))
            .unwrap_err()
            .contains("needs a quad")
    );
    assert_eq!(e.scene, original);
    // Grow no longer needs an all-quad mesh (Jason, 2026-11-05): the
    // triangle grows to its neighbour.
    e.dispatch(Command::Selection(SelectionOperation::Grow))
        .unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
    let mut e = fixture();
    e.scene.selection.faces.clear();
    let original = e.scene.clone();
    assert!(
        e.dispatch(Command::Selection(SelectionOperation::Grow))
            .unwrap_err()
            .contains("Select something")
    );
    assert_eq!(e.scene, original);
}
