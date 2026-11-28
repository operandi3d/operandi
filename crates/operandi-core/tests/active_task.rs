//! Modeling tasks are exclusive: starting one task from any other leaves at
//! most one task slot open, and `active_task` reports the one that is.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0, 1];
    e.synchronize_identities().unwrap();
    e
}

/// Every way to start a task: each live tool, the numeric sessions and Pen.
fn starts() -> Vec<Command> {
    let mut all: Vec<Command> = Tool::ALL.iter().map(|&t| Command::Activate(t)).collect();
    all.extend([
        Command::ArmCone,
        Command::ArmEllipsoid,
        Command::ArmSpike,
        Command::ArmBridge,
        Command::ArmReduction,
        Command::PenStart,
    ]);
    all
}

#[test]
fn starting_any_task_from_any_other_leaves_one_open() {
    let mut opened = 0;
    for first in starts() {
        for second in starts() {
            let mut e = editor();
            // Refusals are fine (some tasks need a particular selection);
            // what matters is that no command leaves two tasks open.
            let _ = e.dispatch(first.clone());
            assert!(e.session.open_task_slots() <= 1);
            opened += usize::from(e.session.has_active_task());
            let _ = e.dispatch(second.clone());
            assert!(e.session.open_task_slots() <= 1);
            assert_eq!(
                e.session.has_active_task(),
                e.session.open_task_slots() == 1
            );
            let _ = e.dispatch(Command::Drop);
            assert!(e.session.open_task_slots() <= 1);
        }
    }
    // Most starts succeed on this fixture, so the transitions are exercised.
    assert!(opened >= starts().len().pow(2) / 2, "{opened}");
}

#[test]
fn active_task_names_the_open_task() {
    let mut e = editor();
    assert_eq!(e.session.active_task(), None);
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    assert_eq!(e.session.active_task(), Some(ActiveTask::Live(Tool::Bevel)));
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::ArmCone).unwrap();
    assert_eq!(e.session.active_task(), Some(ActiveTask::Cone));
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.session.active_task(), None);
}

#[test]
fn numeric_pen_refuses_and_ellipsoid_unit_drops_another_open_task() {
    // Gaps closed 2026-10-31: these guards used to skip some task kinds.
    let pen = PenInput::parse(
        r#"{"positions_m":[[-2,-1,0],[4,0,0],[3,2,0],[-1,1,0]],"type":"vertices","flip":false,"close":false,"select_new":true}"#,
    )
    .unwrap();
    let mut checked = 0;
    for arm in [Command::ArmBridge, Command::ArmSpike] {
        let mut e = editor();
        if e.dispatch(arm).is_err() || !e.session.has_active_task() {
            continue;
        }
        checked += 1;
        let before = e.scene.clone();
        assert!(e.dispatch(Command::Pen(pen.clone())).is_err());
        assert_eq!(e.scene, before);
        // Unit insertion drops the open task first (2026-11-05).
        e.dispatch(Command::InsertEllipsoidUnit(UnitDestination::NewMesh))
            .unwrap();
        assert!(!e.session.has_active_task());
        assert_eq!(e.scene.items.len(), before.items.len() + 1);
    }
    assert!(checked > 0, "no task opened on the fixture");
}
