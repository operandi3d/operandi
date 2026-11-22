//! Edge-mode double-click: `Command::SelectEdgeLoop` selects the clicked
//! edge's loop (Shift adds it), one "Select Loop" undo step.
use glam::DVec3;
use operandi_core::*;

/// Grid4: 4×4 quads in XY; point = row × 5 + column.
fn grid4() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    for y in 0..5 {
        for x in 0..5 {
            mesh.vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    for row in 0..4u32 {
        for column in 0..4u32 {
            let a = row * 5 + column;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + 6, a + 5],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    e.scene.selection.mode = Mode::Edge;
    let item = e.scene.selection.item.unwrap();
    (e, item)
}

fn sorted(edges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut v = edges.to_vec();
    v.sort();
    v
}

#[test]
fn double_click_selects_the_loop_and_replaces_the_selection() {
    let (mut e, item) = grid4();
    e.scene.selection.edges = vec![(0, 5)];
    let history = e.undo.len();
    e.dispatch(Command::SelectEdgeLoop {
        item,
        a: 7,
        b: 6,
        add: false,
    })
    .unwrap();
    // Row 1 across the grid; the clicked edge stays first.
    assert_eq!(e.scene.selection.edges[0], (6, 7));
    assert_eq!(
        sorted(&e.scene.selection.edges),
        vec![(5, 6), (6, 7), (7, 8), (8, 9)]
    );
    assert_eq!(e.undo.len(), history + 1, "one Select Loop step");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.edges, vec![(0, 5)]);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 4);
}

#[test]
fn shift_double_click_adds_the_loop_after_the_kept_selection() {
    let (mut e, item) = grid4();
    e.scene.select_edge_loop(item, (6, 7), false).unwrap();
    let first = e.scene.selection.edges.clone();
    // The first Shift press of the double-click already added (0,5).
    // An inner column (column 0 is the open border, whose loop runs all
    // round it: PF-03).
    e.scene.selection.edges.push((1, 6));
    assert!(e.scene.select_edge_loop(item, (1, 6), true).unwrap());
    let edges = &e.scene.selection.edges;
    assert_eq!(&edges[..4], &first[..], "earlier loop kept, in order");
    assert_eq!(edges[4], (1, 6));
    assert_eq!(edges.len(), 8, "column 1 added once each: {edges:?}");
    // Adding a loop that's already selected changes nothing.
    let before = e.scene.selection.clone();
    e.scene.select_edge_loop(item, (8, 9), true).unwrap();
    assert_eq!(e.scene.selection, before);
}

#[test]
fn where_the_loop_cannot_walk_only_the_clicked_edge_is_selected() {
    let (mut e, item) = grid4();
    {
        let mesh = &mut e.scene.active_mut().unwrap().mesh;
        mesh.faces.truncate(1);
        mesh.faces[0].vertices = vec![0, 1, 6];
    }
    // PF-03: a lone triangle's edges are all open border meeting two at a
    // point, so its loop is the whole rim.
    e.scene.selection.edges = vec![(1, 6)];
    assert!(e.scene.select_edge_loop(item, (0, 1), false).unwrap());
    assert_eq!(
        sorted(&e.scene.selection.edges),
        vec![(0, 1), (0, 6), (1, 6)]
    );
}

#[test]
fn invalid_edges_are_refused_without_changes() {
    let (mut e, item) = grid4();
    e.scene.selection.edges = vec![(6, 7)];
    let before = e.scene.clone();
    assert!(
        e.dispatch(Command::SelectEdgeLoop {
            item,
            a: 0,
            b: 24,
            add: false,
        })
        .is_err()
    );
    assert_eq!(e.scene, before);
}

#[test]
fn a_cube_edge_stops_at_its_corners() {
    let (mut e, item) = grid4();
    *e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0).unwrap();
    let edge = e.scene.active().unwrap().mesh.edges()[0];
    assert!(!e.scene.select_edge_loop(item, edge, false).unwrap());
    assert_eq!(e.scene.selection.edges, vec![edge]);
}

// Jason, 2026-11-06: with a loop selected, Shift+click another edge, then
// Loop: both loops are selected.
#[test]
fn loop_from_several_edges_joins_each_seeds_loop() {
    let (mut e, item) = grid4();
    e.scene.select_edge_loop(item, (6, 7), false).unwrap();
    let row = e.scene.selection.edges.clone();
    e.scene.selection.edges.push((6, 11));
    let history = e.undo.len();
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let edges = &e.scene.selection.edges;
    assert_eq!(&edges[..4], &row[..], "existing order kept");
    assert_eq!(edges[4], (6, 11));
    assert_eq!(
        sorted(&edges[4..]),
        vec![(1, 6), (6, 11), (11, 16), (16, 21)]
    );
    assert_eq!(e.undo.len(), history + 1);

    // Two seeds on parallel rows give both rows; a seed on a loop already
    // walked adds nothing twice.
    // (1, 2) is on the open border: its loop is the whole border (PF-03).
    e.scene.selection.edges = vec![(1, 2), (16, 17), (2, 3)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let mut want: Vec<(u32, u32)> = vec![(15, 16), (16, 17), (17, 18), (18, 19)];
    for i in 0..4 {
        want.extend([
            (i, i + 1),
            (20 + i, 21 + i),
            (5 * i, 5 * i + 5),
            (5 * i + 4, 5 * i + 9),
        ]);
    }
    assert_eq!(sorted(&e.scene.selection.edges), sorted(&want));
    assert_eq!(&e.scene.selection.edges[..3], &[(1, 2), (16, 17), (2, 3)]);
}

#[test]
fn a_seed_that_cannot_walk_stays_selected_beside_the_others() {
    let (mut e, _) = grid4();
    // Poles: a triangle fan replaces the polygon on 12, 13, 18, 17.
    {
        let mesh = &mut e.scene.active_mut().unwrap().mesh;
        mesh.vertices.push(DVec3::new(2.5, 2.5, 0.));
        let center = (mesh.vertices.len() - 1) as u32;
        let quad = mesh
            .faces
            .iter()
            .position(|f| f.vertices == vec![12, 13, 18, 17])
            .unwrap();
        let mut fan = mesh.faces.remove(quad);
        for (a, b) in [(12, 13), (13, 18), (18, 17), (17, 12)] {
            fan.vertices = vec![a, b, center];
            mesh.faces.push(fan.clone());
        }
    }
    // Points 12, 13, 17 and 18 each gain a spoke, so they are all poles:
    // the edge (12, 13) has a pole at both ends and can't walk.
    e.scene.selection.edges = vec![(12, 13), (0, 1)];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let edges = &e.scene.selection.edges;
    assert_eq!(&edges[..2], &[(12, 13), (0, 1)]);
    // (0, 1)'s loop is the whole open border (PF-03): 16 edges.
    assert_eq!(edges.len(), 2 + 15, "{edges:?}");

    // Every seed stuck: refused, nothing changed.
    e.scene.selection.edges = vec![(12, 13), (13, 18)];
    let before = e.scene.clone();
    assert!(
        e.dispatch(Command::Selection(SelectionOperation::Loop))
            .is_err()
    );
    assert_eq!(e.scene, before);
}

/// N02: a 4×4 grid with its middle 2×2 polygons removed. The hole's corner
/// points have two spokes as well as two border edges, so the straight walk
/// alone can't choose; a border edge follows the border around the hole.
#[test]
fn a_hole_border_loops_around_its_corners() {
    let (mut e, item) = grid4();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    // Polygons 5, 6, 9 and 10 are the middle 2×2.
    for index in [10, 9, 6, 5] {
        mesh.faces.remove(index);
    }
    e.dispatch(Command::SelectEdgeLoop {
        item,
        a: 6,
        b: 7,
        add: false,
    })
    .unwrap();
    assert_eq!(
        sorted(&e.scene.selection.edges),
        vec![
            (6, 7),
            (6, 11),
            (7, 8),
            (8, 13),
            (11, 16),
            (13, 18),
            (16, 17),
            (17, 18)
        ]
    );
}
