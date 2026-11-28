//! Nits batch 5 (Jason, 2026-11-17): regression tests.
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
fn symmetric(m: &Mesh) {
    let bad = symmetry::unpaired_points(m, Symmetry::x(), 1e-9);
    assert!(bad.is_empty(), "unpaired {bad:?}");
}

/// N28: with Symmetry on, Bridge in Edge mode bridges each side's pair of
/// border runs (picked with their mirrors, four edges in all) and the mesh
/// stays symmetric; one undo takes both back.
#[test]
fn n28_bridge_with_symmetry_bridges_each_sides_pair() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    e.scene.selection.edges = vec![
        edge(&before, [2., 1.], [1., 1.]),
        edge(&before, [-2., 1.], [-1., 1.]),
        edge(&before, [1., 1.2], [2., 1.2]),
        edge(&before, [-1., 1.2], [-2., 1.2]),
    ];
    e.dispatch(Command::ArmBridge).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let after = mesh(&e);
    after.validate().unwrap();
    assert!(
        after.faces.len() >= before.faces.len() + 2,
        "both gaps bridged"
    );
    symmetric(&after);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), before);
}

/// N28: a pair picked on one side only (before Symmetry was on, say)
/// brings its mirror pair along, so both sides are bridged.
#[test]
fn n28_one_sided_pair_bridges_its_mirror_too() {
    let mut e = editor(Mode::Edge);
    let before = mesh(&e);
    e.scene.selection.edges = vec![
        edge(&before, [2., 1.], [1., 1.]),
        edge(&before, [1., 1.2], [2., 1.2]),
    ];
    e.dispatch(Command::ArmBridge).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let after = mesh(&e);
    after.validate().unwrap();
    assert!(
        after.faces.len() >= before.faces.len() + 2,
        "both sides bridged"
    );
    symmetric(&after);
}

/// N29: Bevel works on a mesh that has a named polygon selection set; the
/// set keeps its polygons (the beveled cap keeps its index).
#[test]
fn n29_bevel_keeps_named_selection_sets() {
    let mut e = editor(Mode::Polygon);
    let mut m = mesh(&e);
    m.attributes
        .polygon_sets
        .insert("R-Ear".into(), [0u32, 1].into_iter().collect());
    e.scene.active_mut().unwrap().mesh = m.into();
    e.dispatch(Command::Symmetry(Symmetry::default())).unwrap();
    e.scene.selection.faces = vec![0];
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.2;
    p.inset = 0.1;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let after = mesh(&e);
    after.validate().unwrap();
    assert!(after.faces.len() > 8, "beveled: {}", after.faces.len());
    assert_eq!(
        after.attributes.polygon_sets["R-Ear"],
        [0u32, 1].into_iter().collect()
    );
}

/// N30, revised by ACS-01 (2026-11-19): with Symmetry, the handle is the
/// centre of the selection's + half, for a piece across the plane too, and
/// whatever the pick order.
#[test]
fn n30_a_piece_across_the_plane_takes_its_plus_half() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut m = Mesh::default();
    for y in 0..2 {
        for x in 0..5 {
            m.vertices.push(DVec3::new(x as f64 - 2., y as f64, 0.));
        }
    }
    for x in 0..4u32 {
        m.faces.push(face(vec![x, x + 1, x + 6, x + 5]));
    }
    e.scene.active_mut().unwrap().mesh = m.into();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    // The two middle polygons: one piece across x = 0.
    // Changed for the ACS rulebook: ACS-01 takes the centre from the
    // selected points strictly on the + side (the plane's points don't
    // count, ACS-Q5 provisional), so a piece across the plane no longer
    // keeps its centre on the plane (N30 superseded, SPECIFICATION_QUESTIONS
    // ACS-B2).
    let centre = |e: &Editor| {
        e.pivot_request(&e.scene, Tool::Move)
            .frame()
            .unwrap()
            .center
    };
    e.scene.selection.faces = vec![2, 1];
    let c = centre(&e);
    assert!((c.x - 1.).abs() < 1e-9, "{c}");
    // The two end polygons: the + half whatever the pick order (ACS-01).
    e.scene.selection.faces = vec![0, 3];
    let c = centre(&e);
    assert!((c.x - 1.5).abs() < 1e-9, "{c}");
}
