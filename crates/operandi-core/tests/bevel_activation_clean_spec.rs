//! Bevel activation against `specifications/bevel-activation-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;

fn cube_top_selected() -> Editor {
    let all = cases("bevel-activation-2026-11-05");
    let c = case(&all, "BA06");
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![1];
    e
}

fn counts(e: &Editor) -> (usize, usize) {
    let m = &e.scene.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}

/// BA06 (decided Q-BA-01): an apply with Inset 0 and Shift 0 still builds
/// the zero-size ring; points don't move.
#[test]
fn zero_apply_builds_the_ring() {
    let mut e = cube_top_selected();
    let source = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    assert_eq!(counts(&e), (8, 6), "switching on changes nothing (rule 2)");
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&e), (12, 10));
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(&m.vertices[..8], &source[..]);
    assert!(m.vertices[8..].iter().all(|v| source.contains(v)));
}

/// BA07: each apply is a new bevel of the current cap.
#[test]
fn each_apply_bevels_again() {
    let mut e = cube_top_selected();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.1;
    e.dispatch(Command::ConfigureBevel(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&e), (12, 10));
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(counts(&e), (16, 14));
    let m = &e.scene.active().unwrap().mesh;
    let top = m.vertices.iter().filter(|v| (v.y - 0.5).abs() < 1e-9);
    assert!(top.clone().any(|v| (v.x.abs() - 0.3).abs() < 1e-9));
}
