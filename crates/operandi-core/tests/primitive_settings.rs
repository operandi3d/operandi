//! Jason decision 2026-11-01 (BOX-Q09/SP-Q08/CYL-Q11): Cube, Sphere and
//! Cylinder keep their segment settings between uses, like Cone and
//! Ellipsoid; Reset Tool Attributes restores the starting values.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e
}

/// Draw one primitive with custom segment settings and Drop it.
fn draw(e: &mut Editor, tool: Tool) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.size = DVec3::splat(2.);
    p.segments = [3, 2, 4];
    if let Some(r) = p.radial.as_mut() {
        r.sides = 9;
        r.segments = 5;
    }
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

fn live(e: &Editor) -> &Parameters {
    &e.session.live.as_ref().unwrap().params
}

#[test]
fn next_use_starts_with_the_last_segment_settings() {
    let mut e = editor();
    draw(&mut e, Tool::Box);
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    assert_eq!(live(&e).segments, [3, 2, 4]);
    e.dispatch(Command::Cancel).unwrap();
    for tool in [Tool::Sphere, Tool::Cylinder] {
        draw(&mut e, tool);
        e.dispatch(Command::Activate(tool)).unwrap();
        let r = live(&e).radial.unwrap();
        assert_eq!((r.sides, r.segments), (9, 5), "{tool:?}");
        e.dispatch(Command::Cancel).unwrap();
    }
}

#[test]
fn first_use_and_cancel_keep_the_starting_values() {
    let mut e = editor();
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        e.dispatch(Command::Activate(tool)).unwrap();
        let start = live(&e).clone();
        let mut p = start.clone();
        p.segments = [5, 5, 5];
        if let Some(r) = p.radial.as_mut() {
            r.sides = 7;
        }
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Cancel).unwrap();
        e.dispatch(Command::Activate(tool)).unwrap();
        assert!(live(&e).segments == start.segments, "{tool:?}");
        assert!(live(&e).radial == start.radial, "{tool:?}");
        e.dispatch(Command::Cancel).unwrap();
    }
}

#[test]
fn reset_restores_starting_values_for_this_and_later_uses() {
    let mut e = editor();
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        e.dispatch(Command::Activate(tool)).unwrap();
        let start = live(&e).clone();
        e.dispatch(Command::Cancel).unwrap();
        draw(&mut e, tool);

        // Reset before drawing changes settings but creates no geometry.
        e.dispatch(Command::Activate(tool)).unwrap();
        let scene = e.scene.clone();
        let history = e.history.undo.len();
        e.dispatch(Command::ResetPrimitive).unwrap();
        assert_eq!(e.scene, scene, "{tool:?}: no geometry from Reset");
        assert_eq!(e.history.undo.len(), history);
        assert!(live(&e).segments == start.segments, "{tool:?}");
        assert!(live(&e).radial == start.radial, "{tool:?}");
        e.dispatch(Command::Drop).unwrap();

        e.dispatch(Command::Activate(tool)).unwrap();
        assert!(live(&e).segments == start.segments, "{tool:?} later use");
        assert!(live(&e).radial == start.radial, "{tool:?} later use");
        e.dispatch(Command::Cancel).unwrap();
    }
}

#[test]
fn reset_after_drawing_rebuilds_the_preview_with_starting_values() {
    let mut e = editor();
    let empty = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let mut p = live(&e).clone();
    p.segments = [4, 4, 4];
    e.dispatch(Command::Preview(p)).unwrap();
    let segmented = e.scene.active().unwrap().mesh.faces.len();
    e.dispatch(Command::ResetPrimitive).unwrap();
    assert_eq!(live(&e).segments, Parameters::default().segments);
    let faces = e.scene.active().unwrap().mesh.faces.len();
    assert!(faces < segmented, "{faces} < {segmented}");
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, empty);
}

#[test]
fn reset_refuses_without_a_live_primitive() {
    let mut e = editor();
    let before = e.clone();
    assert!(e.dispatch(Command::ResetPrimitive).is_err());
    assert!(e == before);
}
