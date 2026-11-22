//! Symmetry phase 3 (symmetry-2026-11-13 README item 16, section 8).
//!
//! Acceptance criteria:
//! 1. Toggle Symmetry turns it off and back on with the last setting (axis,
//!    offset, topology); X the first time.
//! 2. A tool started with Symmetry on, on points without mirror partners,
//!    lists them and the status notice says how many and how to fix them.
//! 3. A symmetric selection gives no warning; Symmetry off gives none.
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
fn long_box() -> Mesh {
    segmented_cube(DVec3::ZERO, dvec3(4., 1., 1.), [4, 1, 1]).unwrap()
}

#[test]
fn toggle_symmetry_keeps_the_last_setting() {
    let mut e = editor(long_box(), Symmetry::OFF);
    e.dispatch(Command::ToggleSymmetry).unwrap();
    assert_eq!(e.symmetry, Symmetry::x(), "X the first time");
    let z = Symmetry {
        axis: Some(2),
        offset: 0.25,
        ..Symmetry::x()
    };
    e.dispatch(Command::Symmetry(z)).unwrap();
    e.dispatch(Command::ToggleSymmetry).unwrap();
    assert_eq!(e.symmetry.axis, None);
    e.dispatch(Command::ToggleSymmetry).unwrap();
    assert_eq!(e.symmetry, z, "back on as it was");
}

#[test]
fn a_tool_on_unpaired_points_warns_and_lists_them() {
    let mut m = long_box();
    let moved = m
        .vertices
        .iter()
        .position(|p| p.distance(dvec3(-2., 0.5, 0.5)) < 1e-9)
        .unwrap() as u32;
    m.vertices[moved as usize].y += 0.1;
    let all: Vec<u32> = (0..m.vertices.len() as u32).collect();
    let mut e = editor(m.clone(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = all.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // The moved point and the partner it lost.
    assert_eq!(e.symmetry_unpaired.len(), 2, "{:?}", e.symmetry_unpaired);
    assert!(e.symmetry_unpaired.contains(&moved));
    let notice = e.symmetry_notice().unwrap();
    assert!(
        notice.starts_with("2 points have no mirror partner"),
        "{notice}"
    );
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.symmetry_notice(), None, "no tool, no warning");

    // A symmetric mesh: no warning.
    let mut e = editor(long_box(), Symmetry::x());
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = all.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(e.symmetry_unpaired.is_empty());
    assert_eq!(e.symmetry_notice(), None);

    // Symmetry off: none either.
    let mut e = editor(m, Symmetry::OFF);
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = all;
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(e.symmetry_unpaired.is_empty());
}
