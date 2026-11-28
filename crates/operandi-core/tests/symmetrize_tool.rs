//! Symmetrize, the Duplicate tab tool (BEHAVIOR-DUPLICATE-PANEL DX-40..45).
//! Acceptance criteria:
//!
//! 1. Starting values: Axis X, Positive to Negative, Merge on, Offset 0,
//!    Topology off (DX-41).
//! 2. DX-42 probe box (x −0.3..0.7, a ring at 0.2), Positive to Negative:
//!    points at x −0.7, −0.2, 0, 0.2, 0.7; 20 points, 18 polygons, no
//!    polygon on the plane, closed and facing out.
//! 3. Negative to Positive: x −0.3, 0, 0.3; 12 points, 10 polygons.
//! 4. Merge off leaves the seam open (points doubled on the plane); Offset
//!    moves the plane.
//! 5. A component selection is ignored: the whole mesh is used (DX-43).
//! 6. Live from the start, one undo step on Drop, Cancel restores, and the
//!    result saves and reopens unchanged; options carry to the next use.
//! 7. Nothing to work on (an empty mesh) is an error that changes nothing.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Signed volume: positive when every polygon faces outward.
fn volume(m: &Mesh) -> f64 {
    m.faces
        .iter()
        .map(|f| {
            let p: Vec<DVec3> = f.vertices.iter().map(|&v| m.vertices[v as usize]).collect();
            (1..p.len() - 1)
                .map(|i| p[0].dot(p[i].cross(p[i + 1])) / 6.)
                .sum::<f64>()
        })
        .sum()
}

/// A 1 m box along X through the rings at `xs`, closed at both ends,
/// facing out.
fn ringed_box(xs: &[f64]) -> Mesh {
    let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let mut m = Mesh::default();
    for &x in xs {
        for &(y, z) in &corners {
            m.vertices.push(DVec3::new(x, y, z));
        }
    }
    let at = |ring: usize, j: usize| (ring * 4 + j % 4) as u32;
    m.faces.push(face((0..4).rev().map(|j| at(0, j)).collect()));
    m.faces
        .push(face((0..4).map(|j| at(xs.len() - 1, j)).collect()));
    for r in 0..xs.len() - 1 {
        for j in 0..4 {
            m.faces.push(face(vec![
                at(r, j),
                at(r, j + 1),
                at(r + 1, j + 1),
                at(r + 1, j),
            ]));
        }
    }
    if volume(&m) < 0. {
        for f in &mut m.faces {
            f.vertices.reverse();
        }
    }
    m.validate().unwrap();
    m
}

fn probe_box() -> Mesh {
    ringed_box(&[-0.3, 0.2, 0.7])
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}
fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}
fn params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn set(e: &mut Editor, f: impl FnOnce(&mut symmetrize_tool::Symmetrize)) {
    let mut p = params(e);
    f(&mut p.duplicate.symmetrize);
    e.dispatch(Command::Preview(p)).unwrap();
}
fn xs(m: &Mesh) -> Vec<f64> {
    let mut xs: Vec<f64> = m
        .vertices
        .iter()
        .map(|v| (v.x * 1e6).round() / 1e6)
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    xs
}
fn closed(m: &Mesh) -> bool {
    m.edge_counts().values().all(|&n| n == 2)
}

#[test]
fn starting_values_follow_the_spec() {
    let s = symmetrize_tool::Symmetrize::default();
    assert_eq!(s.axis, 0);
    assert_eq!(s.direction, SymmetrizeSide::Positive);
    assert!(s.merge);
    assert_eq!(s.offset, 0.);
    assert!(!s.topology);
    assert_eq!(Tool::Symmetrize.label(), "Symmetrize");
    assert!(Tool::Symmetrize.is_duplicate());
}

#[test]
fn positive_to_negative_cuts_keeps_and_welds() {
    let mut e = editor(probe_box(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    let m = mesh(&e).clone();
    assert_eq!(xs(&m), vec![-0.7, -0.2, 0., 0.2, 0.7]);
    assert_eq!(m.vertices.len(), 20);
    assert_eq!(m.faces.len(), 18);
    let on_plane = m.faces.iter().any(|f| {
        f.vertices
            .iter()
            .all(|&v| m.vertices[v as usize].x.abs() < 1e-9)
    });
    assert!(!on_plane, "no polygon left on the plane");
    assert!(closed(&m));
    assert!(volume(&m) > 0., "faces out");
    assert!((volume(&m) - 1.4).abs() < 1e-9);
}

#[test]
fn negative_to_positive_keeps_the_minus_side() {
    let mut e = editor(probe_box(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    set(&mut e, |s| s.direction = SymmetrizeSide::Negative);
    let m = mesh(&e);
    assert_eq!(xs(m), vec![-0.3, 0., 0.3]);
    assert_eq!(m.vertices.len(), 12);
    assert_eq!(m.faces.len(), 10);
    assert!(closed(m));
    assert!(volume(m) > 0.);
}

#[test]
fn merge_off_and_offset() {
    let mut e = editor(probe_box(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    set(&mut e, |s| s.merge = false);
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 24, "the seam's four points doubled");
    assert!(!closed(m));
    set(&mut e, |s| {
        s.merge = true;
        s.offset = 0.2;
    });
    // The plane through the ring at 0.2: the + side (0.2..0.7) mirrored.
    assert_eq!(xs(mesh(&e)), vec![-0.3, 0.2, 0.7]);
    assert_eq!(mesh(&e).faces.len(), 10);
    set(&mut e, |s| {
        s.offset = 0.;
        s.axis = 1;
    });
    // Y: the box is already symmetrical about y = 0 (cut through its sides).
    assert!(closed(mesh(&e)));
    assert!((volume(mesh(&e)) - 1.).abs() < 1e-9);
}

#[test]
fn a_component_selection_is_ignored() {
    let mut e = editor(probe_box(), Mode::Polygon);
    // Only the + end cap selected.
    let cap = mesh(&e)
        .faces
        .iter()
        .position(|f| {
            f.vertices
                .iter()
                .all(|&v| mesh(&e).vertices[v as usize].x > 0.6)
        })
        .unwrap();
    e.scene.selection.faces = vec![cap as u32];
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 20);
    assert_eq!(m.faces.len(), 18);
    assert!(e.scene.selection.faces.is_empty());
}

#[test]
fn drop_is_one_step_cancel_restores_and_it_saves() {
    let mut e = editor(probe_box(), Mode::Polygon);
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    set(&mut e, |s| s.axis = 2);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Symmetrize");
    let after = e.scene.clone();
    let dir = std::env::temp_dir().join(format!("op-symz-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("symmetrize.opscene");
    e.save_committed(&path).unwrap();
    let reopened = operandi_core::persistence::load(&path).unwrap();
    assert_eq!(reopened.items[0].mesh, after.items[0].mesh);
    std::fs::remove_dir_all(&dir).ok();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Activate(Tool::Symmetrize)).unwrap();
    assert_eq!(
        params(&e).duplicate.symmetrize.axis,
        2,
        "options carry over"
    );
}

#[test]
fn an_empty_mesh_is_refused_and_unchanged() {
    let mut e = editor(Mesh::default(), Mode::Polygon);
    let before = e.scene.clone();
    let _ = e.dispatch(Command::Activate(Tool::Symmetrize));
    if e.session.live.is_some() {
        assert!(e.session.live.as_ref().unwrap().preview_error.is_some());
        e.dispatch(Command::Drop).ok();
    }
    assert_eq!(e.scene.items[0].mesh, before.items[0].mesh);
}
