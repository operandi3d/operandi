//! Symmetry's movement path for directional tools (symmetry-2026-11-13
//! SYM3-10..13, class C): Extrude's offset, Edge Extend's offset, Edge Slide
//! and Vertex Slide are mirrored for the partner side.
//!
//! Acceptance criteria:
//! 1. Extrude both end faces of a box across the plane with an X offset:
//!    each cap moves outward (mirrored), the mesh stays exactly symmetric.
//! 2. Edge Extend of both open ends of a strip with an X offset: mirrored.
//! 3. Edge Slide / Vertex Slide of a symmetric selection: the result is
//!    exactly symmetric.
//! 4. A one-sided selection only moves that side (SYM3-10), and Symmetry
//!    Off is unchanged (an X offset moves both caps the same way).
//! 5. One undo restores.
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
fn long_box() -> Mesh {
    segmented_cube(DVec3::ZERO, dvec3(4., 1., 1.), [4, 1, 1]).unwrap()
}
fn face_at(m: &Mesh, center: DVec3) -> u32 {
    (0..m.faces.len())
        .find(|&f| m.center(f).distance(center) < 1e-9)
        .unwrap() as u32
}
fn find(m: &Mesh, p: DVec3) -> u32 {
    m.vertices
        .iter()
        .position(|q| q.distance(p) < 1e-9)
        .unwrap() as u32
}
fn symmetric(m: &Mesh) {
    let bad = symmetry::unpaired_points(m, Symmetry::x(), 1e-9);
    assert!(bad.is_empty(), "unpaired {bad:?}: {:?}", m.vertices);
}
fn extrude(e: &mut Editor, extent: DVec3) {
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.extent = extent;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn extrude_offset_is_mirrored_for_the_other_cap() {
    let m = long_box();
    let (right, left) = (
        face_at(&m, dvec3(2., 0., 0.)),
        face_at(&m, dvec3(-2., 0., 0.)),
    );
    let mut e = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![right, left];
    let before = e.scene.clone();
    extrude(&mut e, dvec3(0.5, 0., 0.));
    let out = mesh(&e);
    let xs: Vec<f64> = out.vertices.iter().map(|p| p.x).collect();
    assert!(xs.iter().any(|&x| (x - 2.5).abs() < 1e-12));
    assert!(xs.iter().any(|&x| (x + 2.5).abs() < 1e-12));
    symmetric(&out);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);

    // Off: both caps go +X.
    let m = long_box();
    let mut e = editor(m, Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![right, left];
    extrude(&mut e, dvec3(0.5, 0., 0.));
    assert!(mesh(&e).vertices.iter().any(|p| (p.x + 1.5).abs() < 1e-12));

    // One side picked: only that side moves, as entered.
    let m = long_box();
    let mut e = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![left];
    extrude(&mut e, dvec3(0.5, 0., 0.));
    assert!(mesh(&e).vertices.iter().any(|p| (p.x + 1.5).abs() < 1e-12));
}

/// An open strip of quads across the plane, x from −2 to 2, in z = 0.
fn strip() -> Mesh {
    let mut m = segmented_cube(DVec3::ZERO, dvec3(4., 1., 0.), [4, 1, 1]).unwrap();
    m.validate().unwrap();
    m.attributes.representations.clear();
    m
}

#[test]
fn edge_extend_offset_is_mirrored() {
    let m = strip();
    let edge = |x: f64| {
        let (a, b) = (find(&m, dvec3(x, -0.5, 0.)), find(&m, dvec3(x, 0.5, 0.)));
        (a.min(b), a.max(b))
    };
    let edges = vec![edge(2.), edge(-2.)];
    let mut e = editor(m, Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = edges;
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = dvec3(1., 0., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let out = mesh(&e);
    assert!(out.vertices.iter().any(|p| (p.x - 3.).abs() < 1e-12));
    assert!(out.vertices.iter().any(|p| (p.x + 3.).abs() < 1e-12));
    symmetric(&out);
}

#[test]
fn edge_and_vertex_slide_stay_symmetric() {
    let m = long_box();
    let ring = |x: f64| -> Vec<(u32, u32)> {
        let c = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
        (0..4)
            .map(|i| {
                let (y0, z0) = c[i];
                let (y1, z1) = c[(i + 1) % 4];
                let (a, b) = (find(&m, dvec3(x, y0, z0)), find(&m, dvec3(x, y1, z1)));
                (a.min(b), a.max(b))
            })
            .collect()
    };
    let mut edges = ring(1.);
    edges.extend(ring(-1.));
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = edges;
    e.dispatch(Command::Activate(Tool::EdgeSlide)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.5;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let out = mesh(&e);
    assert_ne!(out.vertices, m.vertices, "the slide moved points");
    symmetric(&out);

    // Vertex Slide along a world direction.
    let points: Vec<u32> = (0..m.vertices.len() as u32)
        .filter(|&v| m.vertices[v as usize].x.abs() == 1.)
        .collect();
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = points;
    e.dispatch(Command::Activate(Tool::VertexSlide)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = dvec3(1., 0., 0.);
    p.shift = 0.4;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let out = mesh(&e);
    assert_ne!(out.vertices, m.vertices);
    symmetric(&out);
}

#[test]
fn loop_slice_cuts_the_mirror_loop_too() {
    let m = long_box();
    // An edge along X on the box's top, in the segment from x = 1 to 2.
    let (a, b) = (find(&m, dvec3(1., 0.5, 0.5)), find(&m, dvec3(2., 0.5, 0.5)));
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.slice.loop_slice.rings = vec![(a, b)];
    p.slice.loop_slice.set_count(1);
    p.slice.loop_slice.set_position(0, 0.25);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let out = mesh(&e);
    assert_eq!(out.vertices.len(), m.vertices.len() + 8, "a loop each side");
    assert!(out.vertices.iter().any(|p| (p.x - 1.25).abs() < 1e-12));
    assert!(out.vertices.iter().any(|p| (p.x + 1.25).abs() < 1e-12));
    symmetric(&out);
    // Off: one loop.
    let mut e = editor(m.clone(), Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.dispatch(Command::Activate(Tool::LoopSlice)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.slice.loop_slice.rings = vec![(a, b)];
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh(&e).vertices.len(), m.vertices.len() + 4);
}

/// A lopsided box: the long box with its +X end pulled out and up.
fn lopsided() -> Mesh {
    let mut m = long_box();
    for p in &mut m.vertices {
        if p.x > 1.5 {
            p.x += 0.7;
            p.y += 0.3;
        }
    }
    m
}

#[test]
fn symmetrize_polygons_replaces_one_half_with_the_mirror_of_the_other() {
    let m = lopsided();
    let mut e = editor(m.clone(), Symmetry::x());
    let before = e.scene.clone();
    e.dispatch(Command::SymmetrizePolygons {
        side: SymmetrizeSide::Negative,
        merge: true,
    })
    .unwrap();
    let out = mesh(&e);
    symmetric(&out);
    assert_eq!(out.faces.len(), m.faces.len());
    assert_eq!(out.vertices.len(), m.vertices.len());
    // The negative end is kept, so the positive end now mirrors it.
    assert!(out.vertices.iter().any(|p| (p.x - 2.).abs() < 1e-12));
    assert!(!out.vertices.iter().any(|p| (p.x - 2.7).abs() < 1e-12));
    // Closed: every edge has two polygons (merged at the seam).
    let counts = out.edge_counts();
    assert!(counts.values().all(|&n| n == 2));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);

    // Positive to negative keeps the pulled end and mirrors it.
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::SymmetrizePolygons {
        side: SymmetrizeSide::Positive,
        merge: true,
    })
    .unwrap();
    let out = mesh(&e);
    symmetric(&out);
    assert!(out.vertices.iter().any(|p| (p.x + 2.7).abs() < 1e-12));

    // Merge off: the seam points are doubled.
    let mut e = editor(m, Symmetry::x());
    e.dispatch(Command::SymmetrizePolygons {
        side: SymmetrizeSide::Positive,
        merge: false,
    })
    .unwrap();
    assert_eq!(mesh(&e).vertices.len(), long_box().vertices.len() + 4);
}
