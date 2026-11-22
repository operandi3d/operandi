//! Join / Join Averaged with Symmetry on (Jason, 2026-11-15; provisional,
//! SYM-JOIN-Q1). Symmetric picking selects each pick's mirror too, so a
//! two-edge Join arrives with four edges.
//!
//! Acceptance criteria:
//! 1. Edge Join with a pair on each side joins each side's pair; the mesh
//!    stays symmetric (Join and Join Averaged).
//! 2. Pairs that touch the centre line share their plane points: those weld
//!    once and the result is still symmetric.
//! 3. A single pair on the centre line, or one side's pair, joins as before.
//! 4. A selection that isn't two edges or two per side refuses, saying what
//!    it found; the mesh is unchanged.
//! 5. Vertex Join with points on each side welds each side by itself; a
//!    point and its mirror still weld into one.
//! 6. One undo restores the mesh.
use glam::{DVec3, dvec3};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Quads mirrored across X = 0. Each side has two separate quads with a gap
/// between them in Y (edges y = 1 and y = 1.2 face each other); a strip
/// across the plane (two quads sharing x = 0) faces another the same way
/// at y = 6..7 and 7.2..8.2; a quad crossing the plane faces another at
/// y = 3..4 and 4.2..5.2.
fn scene_mesh() -> Mesh {
    let mut vertices: Vec<DVec3> = Vec::new();
    let mut faces: Vec<Face> = Vec::new();
    let quad = |vertices: &mut Vec<DVec3>, faces: &mut Vec<Face>, x0, x1, y0, y1| {
        let base = vertices.len() as u32;
        vertices.extend([
            dvec3(x0, y0, 0.),
            dvec3(x1, y0, 0.),
            dvec3(x1, y1, 0.),
            dvec3(x0, y1, 0.),
        ]);
        faces.push(face(vec![base, base + 1, base + 2, base + 3]));
    };
    for (x0, x1) in [(1., 2.), (-2., -1.)] {
        quad(&mut vertices, &mut faces, x0, x1, 0., 1.);
        quad(&mut vertices, &mut faces, x0, x1, 1.2, 2.2);
    }
    quad(&mut vertices, &mut faces, -1., 1., 3., 4.);
    quad(&mut vertices, &mut faces, -1., 1., 4.2, 5.2);
    // Strips across the plane sharing their x = 0 points.
    for (y0, y1) in [(6., 7.), (7.2, 8.2)] {
        let base = vertices.len() as u32;
        vertices.extend([
            dvec3(-1., y0, 0.),
            dvec3(0., y0, 0.),
            dvec3(1., y0, 0.),
            dvec3(1., y1, 0.),
            dvec3(0., y1, 0.),
            dvec3(-1., y1, 0.),
        ]);
        faces.push(face(vec![base, base + 1, base + 4, base + 5]));
        faces.push(face(vec![base + 1, base + 2, base + 3, base + 4]));
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

fn editor(mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = scene_mesh().into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}
fn mesh(e: &Editor) -> Mesh {
    Mesh::clone(&e.scene.active().unwrap().mesh)
}
fn find(m: &Mesh, p: DVec3) -> u32 {
    m.vertices
        .iter()
        .position(|q| q.distance(p) < 1e-9)
        .unwrap_or_else(|| panic!("no point at {p}")) as u32
}
/// A selected edge (selections hold sorted endpoint pairs).
fn edge(m: &Mesh, a: [f64; 2], b: [f64; 2]) -> (u32, u32) {
    let a = find(m, dvec3(a[0], a[1], 0.));
    let b = find(m, dvec3(b[0], b[1], 0.));
    (a.min(b), a.max(b))
}
fn has(m: &Mesh, p: DVec3) -> bool {
    m.vertices.iter().any(|q| q.distance(p) < 1e-9)
}
fn symmetric(m: &Mesh) {
    let bad = symmetry::unpaired_points(m, Symmetry::x(), 1e-9);
    assert!(bad.is_empty(), "unpaired {bad:?}: {:?}", m.vertices);
}
/// The gap pair on +X and its mirror, picked as symmetric picking does
/// (each pick followed by its mirror).
fn side_pairs(m: &Mesh) -> Vec<(u32, u32)> {
    vec![
        edge(m, [2., 1.], [1., 1.]),
        edge(m, [-2., 1.], [-1., 1.]),
        edge(m, [1., 1.2], [2., 1.2]),
        edge(m, [-1., 1.2], [-2., 1.2]),
    ]
}

#[test]
fn edge_join_joins_each_sides_pair() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    e.scene.selection.edges = side_pairs(&before);
    e.dispatch(Command::EdgeCommand(EdgeCommand::Join)).unwrap();
    let after = mesh(&e);
    assert_eq!(after.vertices.len(), before.vertices.len() - 4);
    // The first edge moved onto the second, on both sides.
    for x in [1., 2., -1., -2.] {
        assert!(!has(&after, dvec3(x, 1., 0.)));
        assert!(has(&after, dvec3(x, 1.2, 0.)));
    }
    symmetric(&after);
    assert!(e.scene.selection.edges.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), before);
}

#[test]
fn edge_join_averaged_meets_halfway_on_both_sides() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    e.scene.selection.edges = side_pairs(&before);
    e.dispatch(Command::EdgeCommand(EdgeCommand::JoinAveraged))
        .unwrap();
    let after = mesh(&e);
    for x in [1., 2., -1., -2.] {
        assert!(has(&after, dvec3(x, 1.1, 0.)));
    }
    symmetric(&after);
}

#[test]
fn pairs_touching_the_centre_line_weld_their_shared_points_once() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    let m = &before;
    e.scene.selection.edges = vec![
        edge(m, [0., 7.], [1., 7.]),
        edge(m, [0., 7.], [-1., 7.]),
        edge(m, [0., 7.2], [1., 7.2]),
        edge(m, [0., 7.2], [-1., 7.2]),
    ];
    e.dispatch(Command::EdgeCommand(EdgeCommand::Join)).unwrap();
    let after = mesh(&e);
    assert_eq!(after.vertices.len(), before.vertices.len() - 3);
    for x in [-1., 0., 1.] {
        assert!(!has(&after, dvec3(x, 7., 0.)));
        assert!(has(&after, dvec3(x, 7.2, 0.)));
    }
    symmetric(&after);
}

#[test]
fn a_single_pair_on_the_centre_line_joins_as_before() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    e.scene.selection.edges = vec![
        edge(&before, [-1., 4.], [1., 4.]),
        edge(&before, [-1., 4.2], [1., 4.2]),
    ];
    e.dispatch(Command::EdgeCommand(EdgeCommand::Join)).unwrap();
    let after = mesh(&e);
    assert_eq!(after.vertices.len(), before.vertices.len() - 2);
    assert!(!has(&after, dvec3(1., 4., 0.)));
    symmetric(&after);
}

#[test]
fn one_sides_pair_joins_as_before() {
    let m = scene_mesh();
    let pair = [edge(&m, [2., 1.], [1., 1.]), edge(&m, [1., 1.2], [2., 1.2])];
    let on = edge_join_symmetric_geometry(&m, &pair, Symmetry::x(), false).unwrap();
    let off = edge_command_geometry(&m, &pair, EdgeCommand::Join)
        .unwrap()
        .0;
    assert_eq!(on, off);
}

#[test]
fn wrong_selections_refuse_with_what_was_found() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    let mut three = side_pairs(&before);
    three.pop();
    e.scene.selection.edges = three;
    let error = e
        .dispatch(Command::EdgeCommand(EdgeCommand::Join))
        .unwrap_err();
    assert!(
        error.contains("two on each side") && error.contains("2 on +X, 1 on -X"),
        "{error}"
    );
    // Two per side plus a centre-line edge is not a pairing either.
    let mut five = side_pairs(&before);
    five.push(edge(&before, [-1., 4.], [1., 4.]));
    e.scene.selection.edges = five;
    let error = e
        .dispatch(Command::EdgeCommand(EdgeCommand::Join))
        .unwrap_err();
    assert!(error.contains("1 on the centre line"), "{error}");
    assert_eq!(mesh(&e), before);
    // Without Symmetry the old message stands.
    let m = scene_mesh();
    let error =
        edge_join_symmetric_geometry(&m, &side_pairs(&m), Symmetry::OFF, false).unwrap_err();
    assert!(error.contains("exactly two selected edges"), "{error}");
}

#[test]
fn vertex_join_welds_each_side_by_itself() {
    let mut e = editor(Mode::Vertex);
    let before = mesh(&e);
    let p = |x: f64, y: f64| find(&before, dvec3(x, y, 0.));
    e.scene.selection.vertices = vec![p(1., 1.), p(-1., 1.), p(1., 1.2), p(-1., 1.2)];
    e.dispatch(Command::VertexCommand(VertexCommand::Join {
        average: false,
        options: Default::default(),
    }))
    .unwrap();
    let after = mesh(&e);
    assert_eq!(after.vertices.len(), before.vertices.len() - 2);
    assert!(has(&after, dvec3(1., 1.2, 0.)) && has(&after, dvec3(-1., 1.2, 0.)));
    assert!(!has(&after, dvec3(1., 1., 0.)) && !has(&after, dvec3(-1., 1., 0.)));
    symmetric(&after);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), before);
}

#[test]
fn a_point_and_its_mirror_still_weld_into_one() {
    let m = scene_mesh();
    let pair = [find(&m, dvec3(1., 4., 0.)), find(&m, dvec3(-1., 4., 0.))];
    let options = Default::default();
    let (out, kept) =
        vertex_join_symmetric_geometry(&m, &pair, Symmetry::x(), true, options).unwrap();
    assert_eq!(out.vertices.len(), m.vertices.len() - 1);
    assert!(out.vertices[kept[0] as usize].distance(dvec3(0., 4., 0.)) < 1e-12);
}
