//! Symmetry's creation path with the Weld seam policy
//! (symmetry-2026-11-13 SYM3-20..26), through Pen.
//!
//! Acceptance criteria:
//! 1. Symmetry X: a Pen polygon drawn on one side gets a mirror copy; the
//!    points drawn on the plane are shared, so the two halves are one
//!    connected surface (the seam edge has two polygons), and the mesh is
//!    exactly symmetric (section 10 check 1).
//! 2. The mirror copy faces the mirrored way (normals stay outward).
//! 3. A polygon crossing the plane is made once, with the status message
//!    (SYM3-25); one lying on the plane is made once, silently (SYM3-21).
//! 4. Make Quads strips weld at the seam; a strip running along the seam
//!    stays single.
//! 5. One undo removes both halves (SYM3-26); Symmetry Off is unchanged.
//! 6. Axis Y with an Offset, and a moved mesh item (the plane follows it).
//! 7. The numeric Pen route mirrors the same way.
//! 8. The live preview shows exactly the half Drop adds (SYM3-24).
use glam::{DVec3, dvec3};
use operandi_core::*;

fn editor(symmetry: Symmetry) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e
}
fn pen(e: &mut Editor, options: PenOptions, points: &[DVec3]) {
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenOptions(options)).unwrap();
    for &p in points {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
}
fn drop(e: &mut Editor) {
    e.dispatch(Command::Drop).unwrap();
}
fn mesh(e: &Editor) -> Mesh {
    Mesh::clone(&e.scene.active().unwrap().mesh)
}
fn plane_z() -> PenOptions {
    PenOptions {
        plane_normal: DVec3::Z,
        ..Default::default()
    }
}
fn symmetric(m: &Mesh, s: Symmetry) {
    let bad = symmetry::unpaired_points(m, s, 1e-9);
    assert!(
        bad.is_empty(),
        "unpaired points {bad:?} in {:?}",
        m.vertices
    );
}
fn no_doubled_points(m: &Mesh) {
    for (i, a) in m.vertices.iter().enumerate() {
        for b in &m.vertices[i + 1..] {
            assert!(a.distance(*b) > 1e-9, "coincident points at {a:?}");
        }
    }
}
fn edge_uses(m: &Mesh, a: DVec3, b: DVec3) -> usize {
    let find = |p: DVec3| {
        m.vertices
            .iter()
            .position(|q| q.distance(p) < 1e-9)
            .unwrap() as u32
    };
    let (a, b) = (find(a), find(b));
    m.faces
        .iter()
        .filter(|f| {
            let n = f.vertices.len();
            (0..n).any(|i| {
                let (x, y) = (f.vertices[i], f.vertices[(i + 1) % n]);
                (x, y) == (a, b) || (x, y) == (b, a)
            })
        })
        .count()
}
const HALF: [DVec3; 4] = [
    dvec3(0., 0., 0.),
    dvec3(1., 0., 0.),
    dvec3(1., 1., 0.),
    dvec3(0., 1., 0.),
];

#[test]
fn pen_polygon_mirrors_and_welds_at_the_seam() {
    let mut e = editor(Symmetry::x());
    pen(&mut e, plane_z(), &HALF);
    let before = e.undo.len();
    drop(&mut e);
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.vertices.len(), 6, "the two seam points are shared");
    symmetric(&m, Symmetry::x());
    no_doubled_points(&m);
    assert_eq!(edge_uses(&m, HALF[0], HALF[3]), 2, "one connected surface");
    // Outward: the copy's normal is the mirror of the original's.
    let (n0, n1) = (m.normal(0), m.normal(1));
    assert!((Symmetry::x().mirror_vector(n0) - n1).length() < 1e-12);
    assert_eq!(e.symmetry_note, None);
    // The new geometry is selected, both halves.
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
    // One undo step removes both halves.
    assert_eq!(e.undo.len(), before + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 0);
}

#[test]
fn crossing_and_on_plane_polygons_are_made_once() {
    let mut e = editor(Symmetry::x());
    let crossing = [dvec3(-1., 0., 0.), dvec3(1., 0., 0.), dvec3(1., 1., 0.)];
    pen(&mut e, plane_z(), &crossing);
    drop(&mut e);
    assert_eq!(mesh(&e).faces.len(), 1);
    assert_eq!(
        e.symmetry_note.as_deref(),
        Some("Crosses the symmetry plane: not mirrored")
    );
    // A polygon lying in the plane x = 0 is its own mirror.
    let on_plane = [dvec3(0., 0., 0.), dvec3(0., 1., 0.), dvec3(0., 1., 1.)];
    pen(
        &mut e,
        PenOptions {
            plane_normal: DVec3::X,
            ..Default::default()
        },
        &on_plane,
    );
    drop(&mut e);
    assert_eq!(mesh(&e).faces.len(), 2);
    assert_eq!(e.symmetry_note, None);
}

#[test]
fn symmetry_off_is_unchanged() {
    let mut e = editor(Symmetry::OFF);
    pen(&mut e, plane_z(), &HALF);
    drop(&mut e);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (1, 4));
}

#[test]
fn make_quads_strip_welds_and_a_seam_strip_stays_single() {
    let quads = PenOptions {
        make_quads: true,
        ..plane_z()
    };
    // Starting edge on the seam, then two clicks outward: two quads a side.
    let mut e = editor(Symmetry::x());
    pen(
        &mut e,
        quads,
        &[
            dvec3(0., 0., 0.),
            dvec3(0., 1., 0.),
            dvec3(1., 0., 0.),
            dvec3(2., 0., 0.),
        ],
    );
    drop(&mut e);
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 4);
    assert_eq!(m.vertices.len(), 2 + 4 + 4);
    symmetric(&m, Symmetry::x());
    no_doubled_points(&m);
    assert_eq!(edge_uses(&m, dvec3(0., 0., 0.), dvec3(0., 1., 0.)), 2);

    // A strip lying in the plane x = 0 is made once.
    let mut e = editor(Symmetry::x());
    pen(
        &mut e,
        PenOptions {
            plane_normal: DVec3::X,
            ..quads
        },
        &[dvec3(0., 0., 0.), dvec3(0., 0., 1.), dvec3(0., 1., 0.)],
    );
    drop(&mut e);
    assert_eq!(mesh(&e).faces.len(), 1);
}

#[test]
fn axis_y_with_offset_and_a_moved_item() {
    let s = Symmetry {
        axis: Some(1),
        offset: 0.5,
        ..Symmetry::OFF
    };
    let mut e = editor(s);
    let above = HALF.map(|p| p + dvec3(0., 0.5, 0.));
    pen(&mut e, plane_z(), &above);
    drop(&mut e);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (2, 6));
    symmetric(&m, s);

    // The plane lives in the item's own space: move the item by +5 X and
    // the world seam is x = 5.
    let mut e = editor(Symmetry::x());
    let id = e.scene.active().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.translation = dvec3(5., 0., 0.);
    let world = HALF.map(|p| p + dvec3(5., 0., 0.));
    pen(&mut e, plane_z(), &world);
    drop(&mut e);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (2, 6));
    symmetric(&m, Symmetry::x());
}

#[test]
fn numeric_pen_route_mirrors_too() {
    let mut e = editor(Symmetry::x());
    let input = PenInput::parse(
        r#"{"positions_m":[[0,0,0],[1,0,0],[1,1,0],[0,1,0]],"type":"polygon","flip":false,"close":false,"select_new":true}"#,
    )
    .unwrap();
    e.dispatch(Command::Pen(input)).unwrap();
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (2, 6));
    symmetric(&m, Symmetry::x());
}

#[test]
fn live_preview_is_the_half_drop_adds() {
    let mut e = editor(Symmetry::x());
    pen(&mut e, plane_z(), &HALF);
    let preview = e.pen_mirror_preview().unwrap();
    assert_eq!(preview.creation, symmetry::Creation::Mirrored);
    assert_eq!(preview.rings.len(), 1);
    let ring = &preview.rings[0];
    assert!(ring.iter().all(|p| p.x <= 1e-12));
    drop(&mut e);
    let m = mesh(&e);
    let copy: Vec<DVec3> = m.faces[1]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    assert_eq!(&copy, ring);
    // Off: no preview.
    let mut e = editor(Symmetry::OFF);
    pen(&mut e, plane_z(), &HALF);
    assert!(e.pen_mirror_preview().is_none());
}
