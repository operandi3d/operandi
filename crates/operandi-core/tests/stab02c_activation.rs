//! Activation ordering/rollback gaps; raw context combinations characterize
//! existing API precedence, not newly admitted native workflows.
use glam::DVec3;
use operandi_core::*;

fn fixture() -> Editor {
    let mut e = Editor {
        scene: serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json"))
            .unwrap(),
        ..Default::default()
    };
    e.scene.selection.faces.clear();
    e.synchronize_identities().unwrap();
    e
}

fn assert_preserved(e: &Editor, before: &Editor) {
    assert!(e == before);
    assert_eq!(e.revision, before.revision);
    assert_eq!(e.identities.current, before.identities.current);
    assert_eq!(e.identities.publication, before.identities.publication);
    assert_eq!(e.identities.request, before.identities.request);
    // Allocation reservations shared by candidates are deliberately not an
    // invariant here, especially when private Ellipsoid Drop appended geometry.
}

#[test]
fn act_c01_falloff_precedes_cone_refusal_and_same_live_toggle() {
    for cone in [true, false] {
        let mut e = fixture();
        e.dispatch(if cone {
            Command::ArmCone
        } else {
            Command::Activate(Tool::Move)
        })
        .unwrap();
        e.spatial_falloff = Some(Falloff::Linear(LinearFalloff {
            start: DVec3::ZERO,
            end: DVec3::X,
            ..Default::default()
        }));
        let before = e.clone();
        // Move works with a falloff since the falloff spec; Extrude doesn't.
        assert_eq!(
            e.dispatch(Command::Activate(Tool::Extrude)).unwrap_err(),
            operandi_core::tool_card::falloff_refusal()
        );
        assert_preserved(&e, &before);
    }
}

#[test]
fn act_c01_numeric_drop_error_precedes_target_refusal() {
    let mut e = fixture();
    e.dispatch(Command::ArmEllipsoid).unwrap();
    e.dispatch(Command::ConfigureEllipsoid(EllipsoidParameters::default()))
        .unwrap();
    let invalid = EllipsoidParameters {
        sides: u32::MAX,
        ..Default::default()
    };
    let error = e
        .dispatch(Command::ConfigureEllipsoid(invalid))
        .unwrap_err();
    let before = e.clone();
    assert_eq!(
        e.dispatch(Command::Activate(Tool::Bevel)).unwrap_err(),
        format!("Correct Ellipsoid fields or Cancel before Apply/Drop: {error}")
    );
    assert_preserved(&e, &before);
}

#[test]
fn act_c02_numeric_drop_then_target_refusal_preserves_original_owner() {
    for ellipsoid in [false, true] {
        let mut e = fixture();
        if ellipsoid {
            e.dispatch(Command::ArmEllipsoid).unwrap();
            e.dispatch(Command::ConfigureEllipsoid(EllipsoidParameters::default()))
                .unwrap();
            assert!(e.session.ellipsoid.as_ref().unwrap().preview.is_some());
        } else {
            e.dispatch(Command::ArmSpike).unwrap();
            e.dispatch(Command::ConfigureSpike(0.25)).unwrap();
        }
        let before = e.clone();
        // Establish that private Drop succeeds and actually closes the owner;
        // Ellipsoid also appends and records before target validation refuses.
        let mut dropped = e.clone();
        dropped.dispatch(Command::Drop).unwrap();
        assert!(dropped.session.spike.is_none() && dropped.session.ellipsoid.is_none());
        assert_eq!(
            dropped.history.undo.len(),
            e.history.undo.len() + usize::from(ellipsoid)
        );
        if ellipsoid {
            assert_ne!(dropped.scene, e.scene);
        }
        // Edge Bevel needs selected edges (Bevel and Extrude work on every
        // polygon with nothing selected since PB-O-01 / PE-O-03).
        let target_error = dropped
            .dispatch(Command::Activate(Tool::EdgeBevel))
            .unwrap_err();
        assert_eq!(
            e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap_err(),
            target_error
        );
        assert_preserved(&e, &before);
    }
}
