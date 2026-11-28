//! Symmetry phase 2 (symmetry-2026-11-13 README items 12–15): the rest of
//! the directional, slice and duplicate tools, and Vertex Create.
//!
//! Acceptance criteria (spec section 10, per tool):
//! 1. Do the tool on a symmetric mesh with a symmetric selection (or one
//!    pick) and Symmetry X: the mesh is still exactly symmetric.
//! 2. The point count matches doing it on both sides by hand.
//! 3. One undo restores the mesh.
//! 4. Symmetry Off gives today's result.
use glam::{DVec3, dvec3};
use operandi_core::*;

fn editor(mesh: Mesh, symmetry: Symmetry) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Symmetry(symmetry)).unwrap();
    e
}
fn mesh(e: &Editor) -> Mesh {
    Mesh::clone(&e.scene.active().unwrap().mesh)
}
/// One box across the plane, four segments along X.
fn long_box() -> Mesh {
    segmented_cube(DVec3::ZERO, dvec3(4., 1., 1.), [4, 1, 1]).unwrap()
}
fn face_at(m: &Mesh, center: DVec3) -> u32 {
    (0..m.faces.len())
        .find(|&f| m.center(f).distance(center) < 1e-9)
        .unwrap_or_else(|| panic!("no face at {center}")) as u32
}
fn find(m: &Mesh, p: DVec3) -> u32 {
    m.vertices
        .iter()
        .position(|q| q.distance(p) < 1e-9)
        .unwrap_or_else(|| panic!("no point at {p}")) as u32
}
fn unpaired(m: &Mesh) -> Vec<u32> {
    symmetry::unpaired_points(m, Symmetry::x(), 1e-9)
}
fn symmetric(m: &Mesh) {
    let bad = unpaired(m);
    let at: Vec<DVec3> = bad.iter().map(|&v| m.vertices[v as usize]).collect();
    assert!(bad.is_empty(), "unpaired {bad:?} at {at:?}");
}
fn live(e: &mut Editor, tool: Tool, edit: impl FnOnce(&mut Parameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}
/// The two end caps (x = ±2).
fn caps(m: &Mesh) -> Vec<u32> {
    vec![
        face_at(m, dvec3(2., 0., 0.)),
        face_at(m, dvec3(-2., 0., 0.)),
    ]
}
/// The top polygons of the two end segments (x = ±1.5).
fn tops(m: &Mesh) -> Vec<u32> {
    vec![
        face_at(m, dvec3(1.5, 0.5, 0.)),
        face_at(m, dvec3(-1.5, 0.5, 0.)),
    ]
}
fn polygons(e: &mut Editor, faces: Vec<u32>) {
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = faces;
}

#[test]
fn edge_and_vertex_extrude_stay_symmetric() {
    let m = long_box();
    let edge = |x: f64| {
        let (a, b) = (find(&m, dvec3(x, 0.5, 0.5)), find(&m, dvec3(x, 0.5, -0.5)));
        (a.min(b), a.max(b))
    };
    let edges = vec![edge(1.), edge(-1.)];
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = edges;
    live(&mut e, Tool::EdgeExtrude, |p| {
        p.shift = 0.3;
        p.inset = 0.2;
    });
    assert!(mesh(&e).vertices.len() > m.vertices.len());
    symmetric(&mesh(&e));

    let points = vec![
        find(&m, dvec3(2., 0.5, 0.5)),
        find(&m, dvec3(-2., 0.5, 0.5)),
    ];
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = points;
    live(&mut e, Tool::VertexExtrude, |p| {
        p.shift = 0.3;
        p.inset = 0.2;
    });
    assert!(mesh(&e).vertices.len() > m.vertices.len());
    symmetric(&mesh(&e));
}

#[test]
fn bend_mirrors_its_spine_for_the_other_end() {
    let m = long_box();
    let ends: Vec<u32> = (0..m.vertices.len() as u32)
        .filter(|&v| m.vertices[v as usize].x.abs() > 0.5)
        .collect();
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = ends;
    let before = e.scene.clone();
    live(&mut e, Tool::Bend, |p| p.bend.angle = 40.);
    let out = mesh(&e);
    assert_ne!(out.vertices, m.vertices, "bent");
    symmetric(&out);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn vertex_tear_mirrors_the_drag() {
    let m = long_box();
    let points = vec![
        find(&m, dvec3(2., 0.5, 0.5)),
        find(&m, dvec3(-2., 0.5, 0.5)),
    ];
    let faces = tops(&m);
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = points;
    e.scene.selection.faces = faces;
    live(&mut e, Tool::VertexTear, |p| p.delta = dvec3(0.3, 0.2, 0.));
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), m.vertices.len() + 2);
    assert!(
        out.vertices
            .iter()
            .any(|q| q.distance(dvec3(2.3, 0.7, 0.5)) < 1e-9)
    );
    assert!(
        out.vertices
            .iter()
            .any(|q| q.distance(dvec3(-2.3, 0.7, 0.5)) < 1e-9)
    );
    symmetric(&out);
}

#[test]
fn poly_haul_tear_hauls_the_mirror_corner() {
    let m = long_box();
    let point = find(&m, dvec3(2., 0.5, 0.5));
    let polygon = face_at(&m, dvec3(1.5, 0.5, 0.));
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    live(&mut e, Tool::PolyHaul, |p| {
        p.poly_haul_mode = PolyHaulMode::Tear;
        p.poly_haul_target = HaulTarget::Point { point, polygon };
        p.poly_haul_amount.vector = dvec3(0.3, 0.2, 0.);
    });
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), m.vertices.len() + 2);
    symmetric(&out);
    // Off: one corner.
    let mut e = editor(m.clone(), Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    live(&mut e, Tool::PolyHaul, |p| {
        p.poly_haul_mode = PolyHaulMode::Tear;
        p.poly_haul_target = HaulTarget::Point { point, polygon };
        p.poly_haul_amount.vector = dvec3(0.3, 0.2, 0.);
    });
    assert_eq!(mesh(&e).vertices.len(), m.vertices.len() + 1);
}

#[test]
fn sketch_extrude_follows_the_mirrored_path() {
    let m = long_box();
    let mut e = editor(m.clone(), Symmetry::x());
    polygons(&mut e, caps(&m));
    live(&mut e, Tool::SketchExtrude, |p| {
        p.sketch_path = vec![dvec3(2., 0., 0.), dvec3(2.5, 0.3, 0.)];
    });
    let out = mesh(&e);
    assert!(out.vertices.iter().any(|q| (q.x - 2.5).abs() < 1e-9));
    assert!(out.vertices.iter().any(|q| (q.x + 2.5).abs() < 1e-9));
    symmetric(&out);
    assert_eq!(e.scene.selection.faces.len(), 2, "both caps selected");
}

/// Points added by `tool` on the long box, with Symmetry X and Off.
fn slice_counts(tool: Tool, set: impl Fn(&Mesh, &mut Parameters)) -> (usize, usize) {
    let m = long_box();
    let mut added = [0; 2];
    for (i, symmetry) in [Symmetry::x(), Symmetry::OFF].into_iter().enumerate() {
        let mut e = editor(m.clone(), symmetry);
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        live(&mut e, tool, |p| set(&m, p));
        let out = mesh(&e);
        if i == 0 {
            symmetric(&out);
        }
        added[i] = out.vertices.len() - m.vertices.len();
    }
    (added[0], added[1])
}

#[test]
fn slices_cut_the_mirror_side_too() {
    // Edge Slice across the top of the right end segment.
    let (on, off) = slice_counts(Tool::EdgeSlice, |m, p| {
        let edge = |z: f64| {
            let (a, b) = (find(m, dvec3(1., 0.5, z)), find(m, dvec3(2., 0.5, z)));
            SlicePoint::Edge { a, b, t: 0.25 }
        };
        p.slice.edge_slice.points = vec![edge(-0.5), edge(0.5)];
    });
    assert_eq!((on, off), (4, 2), "Edge Slice");
    // Polygon Slice: the plane x = 1.5.
    let (on, off) = slice_counts(Tool::PolygonSlice, |_, p| {
        p.slice.polygon_slice.cuts = vec![PlaneCut {
            start: dvec3(1.5, 0., -5.),
            end: dvec3(1.5, 0., 5.),
            up: DVec3::Y,
        }];
    });
    assert_eq!((on, off), (8, 4), "Polygon Slice");
    // A cut on the plane is made once.
    let (on, off) = slice_counts(Tool::PolygonSlice, |_, p| {
        p.slice.polygon_slice.cuts = vec![PlaneCut {
            start: dvec3(0.5, 0., -5.),
            end: dvec3(0.5, 0., 5.),
            up: DVec3::Y,
        }];
        p.slice.polygon_slice.cuts.push(PlaneCut {
            start: dvec3(-0.5, 0., -5.),
            end: dvec3(-0.5, 0., 5.),
            up: DVec3::Y,
        });
    });
    assert_eq!((on, off), (8, 8), "a pair already mirrored isn't doubled");
    // Axis Slice with an Offset across the plane.
    let (on, off) = slice_counts(Tool::AxisSlice, |_, p| {
        p.slice.axis_slice.number = [1, 0, 0];
        p.slice.axis_slice.offset = dvec3(0.3, 0., 0.);
    });
    assert_eq!((on, off), (8, 4), "Axis Slice");
    // Pen Slice from the top: an open line at x = 1.5.
    let (on, off) = slice_counts(Tool::PenSlice, |_, p| {
        let o = &mut p.slice.pen_slice;
        o.points = vec![dvec3(1.5, 1., -5.), dvec3(1.5, 1., 5.)];
        o.closed = false;
        o.plane_normal = DVec3::Y;
    });
    assert!(off > 0, "Pen Slice cut");
    assert_eq!(on, 2 * off, "Pen Slice");
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
fn run_duplicate(e: &mut Editor, tool: Tool, set: impl FnOnce(&mut DuplicateParameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    set(&mut p.duplicate);
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn clone_and_radial_array_mirror_their_placement() {
    let m = two_boxes();
    let mut e = editor(m.clone(), Symmetry::x());
    polygons(&mut e, (0..12).collect());
    let before = e.scene.clone();
    run_duplicate(&mut e, Tool::Clone, |d| {
        d.clone.count = 1;
        d.clone.offset = dvec3(1.5, 0., 0.);
    });
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), 32);
    assert!(out.vertices.iter().any(|q| (q.x - 4.).abs() < 1e-9));
    assert!(out.vertices.iter().any(|q| (q.x + 4.).abs() < 1e-9));
    symmetric(&out);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);

    let mut e = editor(m.clone(), Symmetry::x());
    polygons(&mut e, (0..12).collect());
    run_duplicate(&mut e, Tool::RadialArray, |d| {
        d.radial.circle.count = 3;
        d.radial.circle.center = dvec3(3., 0., 0.);
        d.radial.circle.direction = DVec3::Y;
        d.radial.circle.end = 90.;
    });
    let out = mesh(&e);
    assert!(out.vertices.len() > 16);
    symmetric(&out);
}

#[test]
fn lathe_turns_the_mirror_profile_the_other_way() {
    let right = [
        dvec3(2., 1., 0.),
        dvec3(3., 0.5, 0.),
        dvec3(2.5, 0., 0.),
        dvec3(2., 0., 0.),
    ];
    let mut m = Mesh::default();
    for side in [1., -1.] {
        let start = m.vertices.len() as u32;
        m.vertices
            .extend(right.iter().map(|q| dvec3(q.x * side, q.y, q.z)));
        m.faces.push(Face {
            vertices: (start..start + 4).collect(),
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        m.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed: false,
                curve: CurveKind::Line,
            });
    }
    m.validate().unwrap();
    let mut e = editor(m, Symmetry::x());
    polygons(&mut e, vec![0, 1]);
    run_duplicate(&mut e, Tool::Lathe, |d| {
        d.lathe.circle.count = 8;
        d.lathe.circle.center = dvec3(2., 0., 0.);
        d.lathe.circle.end = 270.;
    });
    let out = mesh(&e);
    assert!(out.faces.len() >= 2 * 24, "both profiles swept");
    symmetric(&out);
}

#[test]
fn vertex_create_adds_mirror_points_and_one_on_the_seam() {
    let mut e = editor(long_box(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    live(&mut e, Tool::VertexCreate, |p| {
        p.sketch_path = vec![dvec3(1., 3., 0.), dvec3(0., 4., 0.)];
    });
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), 20 + 3);
    symmetric(&out);
}

/// The long box with two negative-side points nudged: one by 0.5 mm, one
/// by 1 cm.
fn nudged() -> (Mesh, u32, u32) {
    let mut m = long_box();
    let near = find(&m, dvec3(-2., 0.5, 0.5));
    let far = find(&m, dvec3(-1., -0.5, 0.5));
    m.vertices[near as usize] += dvec3(0.0005, 0., 0.0003);
    m.vertices[far as usize] += dvec3(0., 0.01, 0.);
    (m, near, far)
}

#[test]
fn symmetry_tool_snaps_pairs_within_threshold() {
    let (m, near, far) = nudged();
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    let before = e.scene.clone();
    // Default Threshold 1 mm: the near point snaps, the far one stays.
    e.dispatch(Command::Activate(Tool::SymmetryTool)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    assert_eq!(p.symmetry_fix, symmetry_fix::SymmetryFix::default());
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let out = mesh(&e);
    assert!(out.vertices[near as usize].distance(dvec3(-2., 0.5, 0.5)) < 1e-12);
    assert_eq!(out.vertices[far as usize], m.vertices[far as usize]);
    assert_eq!(
        e.symmetry_unpaired.len(),
        2,
        "the far point and its partner"
    );
    // Dragging Threshold past 1 cm fixes the rest.
    let mut q = p.clone();
    q.symmetry_fix.threshold = 0.02;
    e.dispatch(Command::Preview(q)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    symmetric(&mesh(&e));
    assert!(e.symmetry_unpaired.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);

    // Reverse: the positive side moves to the nudged negative side.
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    live(&mut e, Tool::SymmetryTool, |p| {
        p.symmetry_fix.reverse = true;
        p.symmetry_fix.threshold = 0.02;
    });
    let out = mesh(&e);
    assert_eq!(out.vertices[near as usize], m.vertices[near as usize]);
    symmetric(&out);

    // Without Topology, by position only: the same result here.
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    live(&mut e, Tool::SymmetryTool, |p| {
        p.symmetry_fix.topology = false;
        p.symmetry_fix.threshold = 0.02;
    });
    symmetric(&mesh(&e));

    // Symmetry off: the tool says what to do.
    let mut e = editor(m, Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    let err = e
        .dispatch(Command::Activate(Tool::SymmetryTool))
        .unwrap_err();
    assert!(err.contains("Turn Symmetry on"), "{err}");
}
