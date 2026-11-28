//! Poly Haul against `specifications/poly-haul-2026-11-05` (v0): only the
//! defaults are specified; Offset mode starts at Offset 0 and Reset
//! returns to it (PH-D0).
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.synchronize_identities().unwrap();
    e
}

#[test]
fn offset_starts_at_zero_and_reset_returns_to_it() {
    let mut e = editor();
    let source = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::Activate(Tool::PolyHaul)).unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().params.poly_haul_offset, 0.);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.poly_haul_offset = 0.25;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_ne!(e.scene.active().unwrap().mesh.vertices, source);
    e.dispatch(Command::ResetPolyHaul).unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().params.poly_haul_offset, 0.);
    assert_eq!(e.scene.active().unwrap().mesh.vertices, source);
}

#[test]
fn reset_needs_the_tool() {
    let mut e = editor();
    assert!(e.dispatch(Command::ResetPolyHaul).is_err());
}
