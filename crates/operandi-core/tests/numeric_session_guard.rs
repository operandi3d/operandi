//! Every numeric Arm (Cone, Ellipsoid, the shapes, Spike, Bridge, Polygon
//! Reduction) goes through one guard, so they refuse the same things and
//! open no task when they do.
use operandi_core::*;

const NAMES: [&str; 6] = ["Cone", "Ellipsoid", "Torus", "Spike", "Bridge", "Reduction"];

fn arm(i: usize) -> Command {
    match i {
        0 => Command::ArmCone,
        1 => Command::ArmEllipsoid,
        2 => Command::ArmShape(ShapeKind::Torus),
        3 => Command::ArmSpike,
        4 => Command::ArmBridge,
        _ => Command::ArmReduction,
    }
}

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
        .unwrap()
        .into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.history.undo.clear();
    e
}

#[test]
fn every_numeric_arm_refuses_during_a_selection_gesture() {
    for (i, name) in NAMES.iter().enumerate() {
        let mut e = editor();
        e.dispatch(Command::BeginSelection).unwrap();
        let err = e.dispatch(arm(i)).unwrap_err();
        assert!(err.starts_with("Finish selection before"), "{name}: {err}");
        assert!(!e.session.has_active_task(), "{name} opened a task");
        // The same Arm works once the gesture is finished.
        e.dispatch(Command::EndSelection { cancel: true }).unwrap();
        e.dispatch(arm(i)).unwrap();
        assert!(e.session.has_active_task(), "{name} did not arm");
        assert_eq!(e.session.open_task_slots(), 1);
    }
}

#[test]
fn arming_another_numeric_tool_replaces_the_open_one() {
    for (first, a) in NAMES.iter().enumerate() {
        for (second, b) in NAMES.iter().enumerate() {
            if first == second {
                continue;
            }
            let mut e = editor();
            e.dispatch(arm(first)).unwrap();
            e.dispatch(arm(second)).unwrap();
            assert_eq!(e.session.open_task_slots(), 1, "{a} then {b}");
        }
    }
}
