//! The whole-selection commands behind Shift+double-click on empty space
//! (Jason, 2026-11-23): Connected (Vertices, Polygons) and Loop (Edges)
//! only add to the selection, per piece, as one undo step.
use glam::DVec3;
use operandi_core::*;

fn quad(a: u32) -> Face {
    Face {
        vertices: vec![a, a + 3, a + 4, a + 1],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Two separate 2x2 quad grids in one mesh: piece A is points 0..9 and
/// faces 0..4, piece B points 9..18 and faces 4..8.
fn two_pieces(mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    for left in [-3., 1.] {
        for row in 0..3 {
            for column in 0..3 {
                mesh.vertices
                    .push(DVec3::new(left + column as f64, 0., row as f64));
            }
        }
    }
    mesh.faces = [0u32, 9]
        .iter()
        .flat_map(|&base| [0, 1, 3, 4].map(|corner| quad(base + corner)))
        .collect::<Vec<_>>()
        .into();
    e.scene.selection.mode = mode;
    e
}

fn sorted<T: Ord + Clone>(v: &[T]) -> Vec<T> {
    let mut v = v.to_vec();
    v.sort();
    v
}

/// Runs `operation` and checks it made exactly one undo step that undoes
/// back to the selection before it.
fn one_step(e: &mut Editor, operation: SelectionOperation) {
    let before = e.scene.selection.clone();
    let history = e.undo.len();
    e.dispatch(Command::Selection(operation)).unwrap();
    assert_eq!(e.undo.len(), history + 1, "one undo step");
    let after = e.scene.selection.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.selection, after);
}

#[test]
fn connected_vertices_add_only_the_selected_pieces() {
    let mut e = two_pieces(Mode::Vertex);
    e.scene.selection.vertices = vec![4];
    one_step(&mut e, SelectionOperation::Connected);
    let v = &e.scene.selection.vertices;
    assert_eq!(sorted(v), (0..9).collect::<Vec<u32>>());

    e.scene.selection.vertices = vec![2, 16];
    one_step(&mut e, SelectionOperation::Connected);
    assert_eq!(
        sorted(&e.scene.selection.vertices),
        (0..18).collect::<Vec<u32>>()
    );
}

#[test]
fn connected_polygons_add_only_the_selected_piece() {
    let mut e = two_pieces(Mode::Polygon);
    e.scene.selection.faces = vec![6];
    one_step(&mut e, SelectionOperation::Connected);
    let f = &e.scene.selection.faces;
    assert_eq!(sorted(f), vec![4, 5, 6, 7]);
}

#[test]
fn loop_adds_each_selected_edges_loop_and_keeps_the_seeds() {
    let mut e = two_pieces(Mode::Edge);
    // Piece A's middle row, piece B's middle column.
    e.scene.selection.edges = vec![(3, 4), (10, 13)];
    one_step(&mut e, SelectionOperation::Loop);
    let edges = &e.scene.selection.edges;
    assert_eq!(edges[..2], [(3, 4), (10, 13)], "the seeds stay first");
    assert_eq!(sorted(edges), vec![(3, 4), (4, 5), (10, 13), (13, 16)]);

    // Again: nothing more to add is refused and records nothing.
    let history = e.undo.len();
    assert!(
        e.dispatch(Command::Selection(SelectionOperation::Loop))
            .is_err()
    );
    assert_eq!(e.undo.len(), history);
    assert_eq!(e.scene.selection.edges.len(), 4);
}
