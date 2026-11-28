//! Bevel by dragging against `specifications/bevel-drag-2026-11-06`. The
//! viewport runs Continue when a new handle drag starts after a released
//! one, so each drag is a new bevel ring (G1, G2). Decided Q-BD-01: editing
//! Inset or Shift right after a drag updates that last bevel live.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    e.scene.selection.mode = Mode::Polygon;
    let top = {
        let m = &e.scene.active().unwrap().mesh;
        (0..m.faces.len()).find(|&f| m.normal(f).y > 0.5).unwrap() as u32
    };
    e.scene.selection.faces = vec![top];
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    e
}

fn drag(e: &mut Editor, shift: f64) {
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = shift;
    e.dispatch(Command::Preview(p)).unwrap();
}

fn counts(e: &Editor) -> (usize, usize) {
    let m = &e.scene.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}

fn top_y(e: &Editor) -> f64 {
    let m = &e.scene.active().unwrap().mesh;
    m.vertices.iter().map(|v| v.y).fold(f64::MIN, f64::max)
}

#[test]
fn each_drag_is_a_new_bevel_ring() {
    let mut e = editor();
    drag(&mut e, 0.042);
    assert_eq!(counts(&e), (12, 10), "G1");
    // The next drag starts with Continue: the first ring is committed.
    e.dispatch(Command::Continue).unwrap();
    drag(&mut e, 0.0235);
    assert_eq!(counts(&e), (16, 14), "G2");
    assert!((top_y(&e) - (0.5 + 0.042 + 0.0235)).abs() < 1e-9);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(counts(&e), (16, 14));
}

#[test]
fn editing_inset_after_a_drag_updates_that_bevel_live() {
    let mut e = editor();
    drag(&mut e, 0.042);
    let before = e.scene.active().unwrap().mesh.vertices.clone();
    // A panel edit right after the drag: the same (last) bevel, re-applied.
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.06;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(counts(&e), (12, 10), "still one ring");
    assert_ne!(e.scene.active().unwrap().mesh.vertices, before);
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(counts(&e), (8, 6), "one undo step");
}
