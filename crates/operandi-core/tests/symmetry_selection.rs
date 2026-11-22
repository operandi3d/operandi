//! Selection with Symmetry on (symmetry-2026-11-13 SYM3-30): every way of
//! selecting adds the mirror side.
//!
//! Acceptance criteria:
//! 1. Double-click Select Connected adds the mirror piece; Off doesn't.
//! 2. Double-click edge loop adds the mirror loop; Shift (add) too.
//! 3. A lasso or rectangle (a run of picks inside one selection gesture)
//!    adds each pick's partner and is one undo step; Ctrl removes both.
//! 4. A loop that is its own mirror (crossing the plane) is not doubled.
use glam::{DVec3, dvec3};
use operandi_core::*;

fn editor(mesh: Mesh, symmetry: Symmetry) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    let id = e.scene.active().unwrap().id;
    (e, id)
}
/// Two separate boxes, at x = +2 and x = −2.
fn two_boxes() -> Mesh {
    let mut m = segmented_cube(dvec3(2., 0., 0.), DVec3::ONE, [1, 1, 1]).unwrap();
    append_mesh(
        &mut m,
        segmented_cube(dvec3(-2., 0., 0.), DVec3::ONE, [1, 1, 1]).unwrap(),
    )
    .unwrap();
    m
}
/// One box across the plane, four segments along X.
fn long_box() -> Mesh {
    segmented_cube(DVec3::ZERO, dvec3(4., 1., 1.), [4, 1, 1]).unwrap()
}
fn find(m: &Mesh, p: DVec3) -> u32 {
    m.vertices
        .iter()
        .position(|q| q.distance(p) < 1e-9)
        .unwrap() as u32
}
fn symmetric_edges(e: &Editor, id: u64) {
    let s = e.active_symmetry();
    for &(a, b) in &e.scene.selection.edges {
        let mirror = e
            .scene
            .symmetric_counterpart(id, Component::Edge(a, b), s)
            .unwrap();
        let Component::Edge(x, y) = mirror else {
            panic!()
        };
        assert!(
            e.scene
                .selection
                .edges
                .iter()
                .any(|&(p, q)| (p.min(q), p.max(q)) == (x.min(y), x.max(y))),
            "edge {a}-{b} has no selected mirror"
        );
    }
}

#[test]
fn select_connected_adds_the_mirror_piece() {
    let (mut e, id) = editor(two_boxes(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::SelectConnectedPolygons {
        item: id,
        face: 0,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.faces.len(), 12);
    // Off: just the clicked piece.
    let (mut e, id) = editor(two_boxes(), Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::SelectConnectedPolygons {
        item: id,
        face: 0,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.faces.len(), 6);
}

#[test]
fn edge_loop_adds_the_mirror_loop() {
    let m = long_box();
    // An edge of the ring at x = 1 (a loop around the box).
    let (a, b) = (
        find(&m, dvec3(1., -0.5, -0.5)),
        find(&m, dvec3(1., 0.5, -0.5)),
    );
    let (mut e, id) = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.dispatch(Command::SelectEdgeLoop {
        item: id,
        a,
        b,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 8, "ring at x = 1 and x = -1");
    symmetric_edges(&e, id);
    // The ring on the plane is its own mirror: not doubled.
    let m = long_box();
    let (a, b) = (
        find(&m, dvec3(0., -0.5, -0.5)),
        find(&m, dvec3(0., 0.5, -0.5)),
    );
    let (mut e, id) = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.dispatch(Command::SelectEdgeLoop {
        item: id,
        a,
        b,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 4);
}

#[test]
fn lasso_picks_add_partners_in_one_step_and_ctrl_removes_both() {
    let m = long_box();
    let right: Vec<u32> = (0..m.vertices.len() as u32)
        .filter(|&v| m.vertices[v as usize].x > 1.5)
        .collect();
    let (mut e, id) = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    let steps = e.undo.len();
    // What the viewport's lasso and rectangle send.
    e.dispatch(Command::BeginSelection).unwrap();
    for &v in &right {
        e.dispatch(Command::Select {
            item: id,
            hit: Component::Vertex(v),
            replace: false,
            remove: false,
        })
        .unwrap();
    }
    e.dispatch(Command::EndSelection { cancel: false }).unwrap();
    assert_eq!(e.scene.selection.vertices.len(), 8);
    // Spec v3.1: the swept points first, then their partners.
    assert_eq!(e.scene.selection.vertices[..4], right[..]);
    let mesh = e.scene.active().unwrap().mesh.clone();
    assert!(
        e.scene.selection.vertices[4..]
            .iter()
            .all(|&v| mesh.vertices[v as usize].x < -1.5)
    );
    assert_eq!(e.undo.len(), steps + 1);
    e.dispatch(Command::BeginSelection).unwrap();
    e.dispatch(Command::Select {
        item: id,
        hit: Component::Vertex(right[0]),
        replace: false,
        remove: true,
    })
    .unwrap();
    e.dispatch(Command::EndSelection { cancel: false }).unwrap();
    assert_eq!(e.scene.selection.vertices.len(), 6);
}
