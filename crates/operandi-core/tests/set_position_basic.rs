//! Original SP fixture plus independently authored sparse attribute extension.
use operandi_core::*;
fn apply(e: &mut Editor, value: f64) -> Result<()> {
    e.dispatch(Command::SetPosition(PositionAssignment {
        space: PositionSpace::Local,
        axis: PositionAxis::Z,
        value,
        expected: None,
    }))
}

fn parked_extrude() -> Editor {
    let scene: Scene =
        serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json")).unwrap();
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.extent = glam::DVec3::Z * 0.5;
    e.dispatch(Command::ConfigureExtrude(p)).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.i2_active() && e.session.live.is_none() && e.i2_can_redo());
    e
}
#[test]
fn parked_extrude_redo_is_preserved_on_noop_and_refusal_and_branched_on_edit() {
    let mut e = parked_extrude();
    let before = e.scene.clone();
    let identity = e.identities.current.clone();
    let parked = e.i2.selected.clone();
    let undo = e.history.undo.clone();
    let redo = e.history.redo.clone();
    apply(&mut e, 1.).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(e.identities.current, identity);
    assert_eq!(e.i2.selected, parked);
    assert!(e.history.undo == undo && e.history.redo == redo && e.i2_can_redo());
    let before_refusal = e.clone();
    assert!(apply(&mut e, f64::NAN).is_err());
    assert!(e == before_refusal);
    assert_eq!(e.scene, before);
    assert_eq!(e.i2.selected, parked);
    assert!(e.history.undo == undo && e.history.redo == redo && e.i2_can_redo());
    let target = e.set_position_target().unwrap();
    let mut expected = before.clone();
    for v in [4_usize, 5, 6, 7] {
        expected.active_mut().unwrap().mesh.vertices[v].z = 2.;
    }
    e.dispatch(Command::SetPosition(PositionAssignment {
        space: PositionSpace::Local,
        axis: PositionAxis::Z,
        value: 2.,
        expected: Some(target),
    }))
    .unwrap();
    assert_eq!(e.scene, expected);
    assert!(e.i2.selected.is_none());
    assert!(!e.i2_can_redo());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(e.identities.current, identity);
}
