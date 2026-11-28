//! Inset and Shift as live Polygon tools (tool audit Tier 1). Inset's
//! geometry is checked against its spec in inset_clean_spec.rs; Shift is
//! provisional (TA1-Q08).
use glam::DVec3;
use operandi_core::*;

fn cube() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Polygon;
    e.synchronize_identities().unwrap();
    e
}

fn mesh(e: &Editor) -> Mesh {
    e.scene.active().unwrap().mesh.to_owned_value()
}

fn run(e: &mut Editor, tool: Tool, amount: f64) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    if tool == Tool::Inset {
        p.inset = amount;
    } else {
        p.shift = amount;
    }
    e.dispatch(Command::Preview(p)).unwrap();
}

#[test]
fn inset_tool_frames_a_smaller_copy_and_keeps_it_selected() {
    let mut e = cube();
    e.scene.selection.faces = vec![0];
    let before = mesh(&e);
    let ring = before.faces[0].vertices.clone();
    run(&mut e, Tool::Inset, 0.25);
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 12);
    assert_eq!(m.faces.len(), 10, "inner polygon plus four side quads");
    assert_eq!(e.scene.selection.faces, vec![0]);
    // Each inner corner is 0.25 in from both of its edges on a 2 m square.
    let centre = before.center(0);
    for &v in &m.faces[0].vertices {
        assert!(!ring.contains(&v));
        let d = m.vertices[v as usize] - centre;
        let along: Vec<f64> = [d.x, d.y, d.z]
            .iter()
            .map(|c| c.abs())
            .filter(|c| *c > 1e-9)
            .collect();
        assert!(along.iter().all(|c| (c - 0.75).abs() < 1e-9), "{d}");
    }
    // One undo step for the whole session.
    e.dispatch(Command::Undo).unwrap();
    assert!(mesh(&e) == before);
}

#[test]
fn nothing_selected_insets_every_polygon_and_negative_grows() {
    let mut e = cube();
    run(&mut e, Tool::Inset, -0.1);
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 6 + 24);
    assert_eq!(m.vertices.len(), 8 + 24);
    // Outward: an inner copy is wider than its original.
    let span = |f: usize| {
        let vs = &m.faces[f].vertices;
        vs.iter()
            .map(|&v| m.vertices[v as usize])
            .fold(0f64, |a, p| a.max(p.distance(m.center(f))))
    };
    assert!(span(0) > 2f64.sqrt() + 0.1);
}

#[test]
fn shift_moves_each_polygon_along_its_own_normal_on_new_points() {
    let mut e = cube();
    e.scene.selection.faces = vec![0, 1];
    let before = mesh(&e);
    run(&mut e, Tool::Shift, 0.5);
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    m.validate().unwrap();
    assert_eq!(m.vertices.len(), 16, "no new points shared between the two");
    assert_eq!(m.faces.len(), 6 + 8);
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
    for f in [0usize, 1] {
        let normal = before.normal(f).normalize();
        let moved = m.center(f) - before.center(f);
        assert!(moved.distance(normal * 0.5) < 1e-9, "polygon {f}: {moved}");
        assert!((m.normal(f).normalize() - normal).length() < 1e-9);
    }
    // Side quads have area now (none left marked degenerate).
    assert!((6..m.faces.len()).all(|f| !m.attributes.degenerate_faces.contains(&(f as u32))));
    e.dispatch(Command::Undo).unwrap();
    assert!(mesh(&e) == before);
}

#[test]
fn shift_with_nothing_selected_shifts_every_polygon() {
    let mut e = cube();
    run(&mut e, Tool::Shift, 0.1);
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 6 + 24);
    for f in 0..6 {
        let c = m.center(f);
        assert!((c.abs().max_element() - 1.1).abs() < 1e-9, "{c}");
    }
}

#[test]
fn cancel_and_refusals_leave_the_scene_unchanged() {
    for tool in [Tool::Inset, Tool::Shift] {
        let mut e = cube();
        e.scene.selection.faces = vec![2];
        let before = mesh(&e);
        run(&mut e, tool, 0.3);
        e.dispatch(Command::Cancel).unwrap();
        assert!(mesh(&e) == before);
        assert!(e.session.live.is_none());
        // Polygon mode only.
        e.scene.selection.mode = Mode::Edge;
        assert!(e.dispatch(Command::Activate(tool)).is_err());
        // A non-finite amount is refused and the preview stays.
        e.scene.selection.mode = Mode::Polygon;
        run(&mut e, tool, 0.2);
        let shown = mesh(&e);
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.inset = f64::NAN;
        p.shift = f64::NAN;
        assert!(e.dispatch(Command::Preview(p)).is_err());
        assert!(mesh(&e) == shown);
    }
}

#[test]
fn shift_geometry_matches_inset_zero_topology() {
    let source = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    let a = inset_geometry(&source, &[3], 0.).unwrap();
    let b = shift_geometry(&source, &[3], 0.).unwrap();
    assert!(a.mesh.faces == b.mesh.faces);
    assert_eq!(a.selected, b.selected);
    assert!(shift_geometry(&source, &[3], f64::INFINITY).is_err());
}

#[test]
fn each_tweak_is_its_own_undo_step_inside_the_tool() {
    let mut e = cube();
    e.scene.selection.faces = vec![0];
    run(&mut e, Tool::Inset, 0.1);
    let first = mesh(&e);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.3;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(
        e.session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::Inset)
    );
    assert!(mesh(&e) == first);
    assert_eq!(e.session.live.as_ref().unwrap().params.inset, 0.1);
}
