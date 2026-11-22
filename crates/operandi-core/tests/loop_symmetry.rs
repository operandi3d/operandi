//! Loop-type selection commands with Symmetry on (SYM-LOOP-01..04, bug
//! reports #3 and #4): only the picks on the clicked side count, then the
//! mirror side is added.
//!
//! Acceptance criteria:
//! 1. Two adjacent polygons picked with Symmetry X, then Loop: the loop and
//!    its mirrored loop, nothing refused (#4).
//! 2. The same, then Close Loop: the closed loop on each side (#3).
//! 3. Close Loop with Symmetry off works from two adjacent polygons.
//! 4. A loop that is its own mirror (it crosses the plane) isn't doubled.
//! 5. Edge Loop from two picked points and Ring from one edge, Symmetry X:
//!    both sides.
//! 6. Loop under Symmetry is one undo step; undo restores the picks.
//! 7. Symmetry off: Loop from four seeds still refuses as before.
use glam::{DVec3, dvec3};
use operandi_core::*;
use std::collections::BTreeSet;

fn editor(mesh: Mesh, symmetry: Symmetry) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    let id = e.scene.active().unwrap().id;
    (e, id)
}

/// A 4 x 1 x 4 box across the X plane, four segments along X and Z: the
/// polygon loop through two top polygons in one X column runs around the
/// box at that column (10 polygons) and stays on one side.
fn slab() -> Mesh {
    segmented_cube(DVec3::ZERO, dvec3(4., 1., 4.), [4, 1, 4]).unwrap()
}

fn centre(m: &Mesh, f: usize) -> DVec3 {
    let v = &m.faces[f].vertices;
    v.iter().map(|&i| m.vertices[i as usize]).sum::<DVec3>() / v.len() as f64
}

fn face_at(m: &Mesh, p: DVec3) -> u32 {
    (0..m.faces.len())
        .find(|&f| centre(m, f).distance(p) < 1e-9)
        .unwrap_or_else(|| panic!("no polygon at {p}")) as u32
}

fn point_at(m: &Mesh, p: DVec3) -> u32 {
    m.vertices
        .iter()
        .position(|q| q.distance(p) < 1e-9)
        .unwrap() as u32
}

fn pick(e: &mut Editor, id: u64, hit: Component) {
    e.dispatch(Command::Select {
        item: id,
        hit,
        replace: false,
        remove: false,
    })
    .unwrap();
}

fn selected_centres(e: &Editor) -> Vec<DVec3> {
    let m = &e.scene.active().unwrap().mesh;
    e.scene
        .selection
        .faces
        .iter()
        .map(|&f| centre(m, f as usize))
        .collect()
}

/// Two adjacent top polygons in the x = 1.5 column, Symmetry X.
fn two_top_polygons(symmetry: Symmetry) -> (Editor, u64) {
    let m = slab();
    let a = face_at(&m, dvec3(1.5, 0.5, -1.5));
    let b = face_at(&m, dvec3(1.5, 0.5, -0.5));
    let (mut e, id) = editor(m, symmetry);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    pick(&mut e, id, Component::Polygon(a));
    pick(&mut e, id, Component::Polygon(b));
    (e, id)
}

#[test]
fn polygon_loop_from_two_picks_selects_both_mirrored_loops() {
    let (mut e, _) = two_top_polygons(Symmetry::x());
    assert_eq!(
        e.scene.selection.faces.len(),
        4,
        "each pick adds its mirror"
    );
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let centres = selected_centres(&e);
    assert_eq!(centres.len(), 20, "a 10-polygon loop on each side");
    assert_eq!(
        centres.iter().filter(|c| (c.x - 1.5).abs() < 1e-9).count(),
        10
    );
    assert_eq!(
        centres.iter().filter(|c| (c.x + 1.5).abs() < 1e-9).count(),
        10
    );
    // The picks stay first, in order.
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(
        centre(m, e.scene.selection.faces[0] as usize),
        dvec3(1.5, 0.5, -1.5)
    );
}

#[test]
fn close_loop_from_two_picks_works_on_each_side() {
    // Symmetry off first (criterion 3): the loop plus the smaller region it
    // cuts off (the +X end of the box beyond the column).
    let (mut off, _) = two_top_polygons(Symmetry::OFF);
    off.dispatch(Command::Selection(SelectionOperation::CloseLoop))
        .unwrap();
    let one_side = selected_centres(&off);
    assert!(
        one_side.len() > 10,
        "loop plus a region: {}",
        one_side.len()
    );
    assert!(one_side.iter().all(|c| c.x > 0.));

    let (mut e, _) = two_top_polygons(Symmetry::x());
    e.dispatch(Command::Selection(SelectionOperation::CloseLoop))
        .unwrap();
    let both = selected_centres(&e);
    assert_eq!(both.len(), 2 * one_side.len());
    for c in &one_side {
        let mirror = dvec3(-c.x, c.y, c.z);
        assert!(both.iter().any(|d| d.distance(mirror) < 1e-9), "{c} mirror");
    }
}

#[test]
fn loop_across_the_plane_is_not_doubled() {
    // Two top polygons in one Z row (x = 0.5 and 1.5): the loop runs along
    // X over the whole box and is its own mirror.
    let m = slab();
    let a = face_at(&m, dvec3(0.5, 0.5, -1.5));
    let b = face_at(&m, dvec3(1.5, 0.5, -1.5));
    let (mut e, id) = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    pick(&mut e, id, Component::Polygon(a));
    pick(&mut e, id, Component::Polygon(b));
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let faces: BTreeSet<u32> = e.scene.selection.faces.iter().copied().collect();
    assert_eq!(faces.len(), e.scene.selection.faces.len(), "no duplicates");
    assert_eq!(faces.len(), 10, "top 4, two ends, bottom 4");
}

#[test]
fn edge_loop_and_ring_add_the_mirror_side() {
    // Vertex loop from two consecutive points at x = 1 on the top.
    let m = slab();
    let a = point_at(&m, dvec3(1., 0.5, -2.));
    let b = point_at(&m, dvec3(1., 0.5, -1.));
    let (mut e, id) = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    pick(&mut e, id, Component::Vertex(a));
    pick(&mut e, id, Component::Vertex(b));
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    let xs: Vec<f64> = e
        .scene
        .selection
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize].x)
        .collect();
    assert!(xs.iter().any(|x| (x - 1.).abs() < 1e-9));
    assert_eq!(
        xs.iter().filter(|x| (*x - 1.).abs() < 1e-9).count(),
        xs.iter().filter(|x| (*x + 1.).abs() < 1e-9).count(),
        "same loop on both sides"
    );
    assert!(xs.len() > 4);

    // Ring from one edge along X at the top front: both sides.
    let m = slab();
    let (p, q) = (
        point_at(&m, dvec3(1., 0.5, -2.)),
        point_at(&m, dvec3(2., 0.5, -2.)),
    );
    let (mut e, id) = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    pick(&mut e, id, Component::Edge(p, q));
    assert_eq!(e.scene.selection.edges.len(), 2);
    e.dispatch(Command::Selection(SelectionOperation::Ring))
        .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    let mids: Vec<f64> = e
        .scene
        .selection
        .edges
        .iter()
        .map(|&(a, b)| (m.vertices[a as usize].x + m.vertices[b as usize].x) / 2.)
        .collect();
    let right = mids.iter().filter(|x| (*x - 1.5).abs() < 1e-9).count();
    let left = mids.iter().filter(|x| (*x + 1.5).abs() < 1e-9).count();
    assert!(right > 1);
    assert_eq!(right, left);
    assert_eq!(right + left, mids.len());
}

#[test]
fn loop_with_symmetry_is_one_undo_step() {
    let (mut e, _) = two_top_polygons(Symmetry::x());
    let picks = e.scene.selection.faces.clone();
    let steps = e.undo.len();
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.faces, picks);
}

#[test]
fn symmetry_off_four_seeds_still_refuse() {
    let m = slab();
    let faces = [
        face_at(&m, dvec3(1.5, 0.5, -1.5)),
        face_at(&m, dvec3(-1.5, 0.5, -1.5)),
        face_at(&m, dvec3(1.5, 0.5, -0.5)),
        face_at(&m, dvec3(-1.5, 0.5, -0.5)),
    ];
    let (mut e, id) = editor(m, Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    for f in faces {
        pick(&mut e, id, Component::Polygon(f));
    }
    let err = e
        .dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap_err();
    assert!(
        err.to_string().contains("two adjacent ordered seeds"),
        "{err}"
    );
}
