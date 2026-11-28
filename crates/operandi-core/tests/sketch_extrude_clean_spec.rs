//! Sketch Extrude against `specifications/sketch-extrude-2026-11-05` (v0):
//! switching the tool on doesn't change the mesh, Precision starts at 30,
//! the last use's value carries over and Reset restores it (SE-D0).
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

fn precision(e: &Editor) -> f64 {
    e.session.live.as_ref().unwrap().params.sketch_precision
}

#[test]
fn defaults_carry_over_and_reset() {
    let mut e = editor();
    let source = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Activate(Tool::SketchExtrude)).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, source);
    assert_eq!(precision(&e), 30.);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.sketch_precision = 50.;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::SketchExtrude)).unwrap();
    assert_eq!(precision(&e), 50., "the last use's value carries over");
    e.dispatch(Command::ResetSketchExtrude).unwrap();
    assert_eq!(precision(&e), 30.);
}
