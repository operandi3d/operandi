//! N04: Make Polygon with Symmetry on (provisional). Symmetric picking
//! selects each pick's mirror right after it, so a four-point gap arrives as
//! eight alternating points.
//!
//! Acceptance criteria:
//! 1. Picks on one side make that polygon and its mirror, both wound like
//!    their neighbours; the mesh stays valid and symmetric; one undo.
//! 2. Two picks whose mirrors close the shape make one polygon across the
//!    plane.
//! 3. With Symmetry off, Make works on the selection as before.
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

fn pick(e: &mut Editor, p: [f64; 2]) {
    let item = e.scene.selection.item.unwrap();
    let v = find(&mesh(e), dvec3(p[0], p[1], 0.));
    e.dispatch(Command::Select {
        item,
        hit: Component::Vertex(v),
        replace: false,
        remove: false,
    })
    .unwrap();
}

#[test]
fn picks_on_one_side_make_the_polygon_and_its_mirror() {
    let mut e = editor(Mode::Vertex);
    let before = mesh(&e);
    for p in [[1., 1.], [2., 1.], [2., 1.2], [1., 1.2]] {
        pick(&mut e, p);
    }
    assert_eq!(e.scene.selection.vertices.len(), 8);
    e.dispatch(Command::MakePolygon).unwrap();
    let after = mesh(&e);
    assert_eq!(after.faces.len(), before.faces.len() + 2);
    after.validate().unwrap();
    let bad = symmetry::unpaired_points(&after, Symmetry::x(), 1e-9);
    assert!(bad.is_empty());
    // Both face the same way as their neighbours (+Z).
    for f in [before.faces.len(), before.faces.len() + 1] {
        assert!(after.normal(f).z > 0.9, "{:?}", after.normal(f));
        assert_eq!(after.faces[f].vertices.len(), 4);
    }
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), before);
}

#[test]
fn two_picks_make_one_polygon_across_the_plane() {
    let mut e = editor(Mode::Vertex);
    let before = mesh(&e);
    for p in [[1., 4.], [1., 4.2]] {
        pick(&mut e, p);
    }
    e.dispatch(Command::MakePolygon).unwrap();
    let after = mesh(&e);
    assert_eq!(after.faces.len(), before.faces.len() + 1);
    after.validate().unwrap();
    let f = after.faces.len() - 1;
    assert_eq!(after.faces[f].vertices.len(), 4);
    assert!(after.normal(f).z > 0.9);
}

#[test]
fn without_symmetry_make_uses_the_selection_as_is() {
    let mut e = editor(Mode::Vertex);
    e.dispatch(Command::Symmetry(Symmetry::default())).unwrap();
    let before = mesh(&e);
    for p in [[1., 1.], [2., 1.], [2., 1.2], [1., 1.2]] {
        pick(&mut e, p);
    }
    assert_eq!(e.scene.selection.vertices.len(), 4);
    e.dispatch(Command::MakePolygon).unwrap();
    assert_eq!(mesh(&e).faces.len(), before.faces.len() + 1);
}
