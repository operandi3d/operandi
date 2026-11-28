//! Original regression cases for the bounded review findings and approved PR01.
use glam::DVec3;
use operandi_core::*;
fn armed() -> Editor {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/i2/expectations/graphs/G0.json");
    let mut e = Editor {
        scene: serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e
}
fn moved() -> Editor {
    let mut e = armed();
    e.dispatch(Command::BeginExtrude).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.extent = DVec3::Z * 0.5;
    e.dispatch(Command::ConfigureExtrude(p)).unwrap();
    e
}
fn parked() -> Editor {
    let mut e = moved();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e
}
fn refused_redo(e: &mut Editor) {
    let before = e.clone();
    let bank = format!("{:?}", e.identities);
    let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
    assert!(!e.i2_can_redo(), "menu must refuse the same local Redo");
    assert!(e.dispatch(Command::Redo).is_err());
    assert!(*e == before);
    assert_eq!(format!("{:?}", e.identities), bank);
    assert_eq!(
        (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap()),
        marks
    );
}
#[test]
fn pr01_parked_context_refusal_and_supported_restoration() {
    for context in [
        ActionContext {
            center: CenterMode::Origin,
            ..Default::default()
        },
        ActionContext {
            axis: AxisOrientation::Workplane(1),
            ..Default::default()
        },
    ] {
        let mut e = parked();
        let stage = e.i2.selected.clone();
        let scene = e.scene.clone();
        let ids = e.identities.current.clone();
        e.dispatch(Command::ActionContext(context)).unwrap();
        refused_redo(&mut e);
        assert_eq!(e.action_context, context);
        assert_eq!(e.i2.selected, stage);
        assert_eq!(e.scene, scene);
        assert_eq!(e.identities.current, ids);
        e.dispatch(Command::ActionContext(Default::default()))
            .unwrap();
        assert!(e.i2_can_redo());
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 1);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 2);
    }
}
#[test]
fn pr01_refusal_retains_merely_armed_move_owner_and_restoration_resumes() {
    let mut e = parked();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        ..Default::default()
    }))
    .unwrap();
    refused_redo(&mut e);
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Move);
    e.dispatch(Command::ActionContext(Default::default()))
        .unwrap();
    assert!(e.i2_can_redo());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Extrude);
    assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 1);
}
#[test]
fn pr01_parked_falloff_refusal_and_supported_restoration() {
    let mut e = parked();
    let stage = e.i2.selected.clone();
    let falloff = LinearFalloff {
        start: DVec3::ZERO,
        end: DVec3::X,
        ..Default::default()
    };
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Linear(falloff))))
        .unwrap();
    refused_redo(&mut e);
    assert_eq!(e.spatial_falloff, Some(Falloff::Linear(falloff)));
    assert_eq!(e.i2.selected, stage);
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    assert!(e.i2_can_redo());
    e.dispatch(Command::Redo).unwrap();
}
#[test]
fn rf05_parked_scene_and_identity_anchor_refuse_atomically() {
    for identity in [false, true] {
        let mut e = parked();
        if identity {
            e.identities
                .current
                .meshes
                .get_mut(&1)
                .unwrap()
                .scope
                .generation += 1;
        } else {
            e.scene.items[0].name.push_str(" altered");
        }
        refused_redo(&mut e);
    }
}
#[test]
fn rf05_document_redo_below_anchor_precedes_pr01_local_guard() {
    let mut e = armed();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::SelectPolygonSet("bottom".into()))
        .unwrap();
    e.dispatch(Command::SelectPolygonSet("top".into())).unwrap();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        ..Default::default()
    }))
    .unwrap();
    assert!(!e.i2_can_redo());
    assert_eq!(e.history.redo.len(), 1);
    e.dispatch(Command::Redo).unwrap();
    assert!(e.history.redo.is_empty());
    assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 0);
    refused_redo(&mut e);
}
#[test]
fn rf03_established_numeric_switches_finish_once_and_cancel_new_owner_only() {
    for phase in ["zero", "moved", "local_begin", "resumed"] {
        for cmd in [Command::ArmSpike, Command::ArmEllipsoid] {
            let mut e = moved();
            match phase {
                "zero" => {
                    e.dispatch(Command::Cancel).unwrap();
                    e.dispatch(Command::BeginExtrude).unwrap();
                }
                "local_begin" => e.dispatch(Command::Undo).unwrap(),
                "resumed" => {
                    e.dispatch(Command::Undo).unwrap();
                    e.dispatch(Command::Undo).unwrap();
                    e.dispatch(Command::Redo).unwrap();
                    e.dispatch(Command::Redo).unwrap();
                }
                _ => {}
            }
            let accepted = e.scene.clone();
            let accepted_ids = e.identities.current.clone();
            let base = e.i2.selected.as_ref().unwrap().base.scene.clone();
            let base_ids = e.i2.selected.as_ref().unwrap().base.identities.clone();
            e.dispatch(cmd).unwrap();
            assert_eq!(e.scene, accepted);
            assert_eq!(e.identities.current, accepted_ids);
            assert_eq!(e.history.undo.len(), 1);
            assert!(e.i2.selected.is_none());
            assert!(e.session.live.is_none());
            e.dispatch(Command::Cancel).unwrap();
            assert_eq!(e.scene, accepted);
            e.dispatch(Command::Undo).unwrap();
            assert_eq!(e.scene, base);
            assert_eq!(e.identities.current, base_ids);
            e.dispatch(Command::Redo).unwrap();
            assert_eq!(e.scene, accepted);
            assert_eq!(e.identities.current, accepted_ids);
            assert!(
                e.session.live.is_none()
                    && e.session.spike.is_none()
                    && e.session.ellipsoid.is_none()
            );
        }
    }
}
#[test]
fn rf03_failed_numeric_target_keeps_full_stage() {
    let mut e = moved();
    // A valid I2 source may have an invisible owner when directly restored by a
    // caller; Ellipsoid's existing activation requires a visible target.
    e.scene.items[0].visible = false;
    let before = e.clone();
    let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
    let error = e.dispatch(Command::ArmEllipsoid).unwrap_err();
    assert!(error.contains("visible"), "{error}");
    assert!(e == before);
    assert_eq!(
        (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap()),
        marks
    );
}
#[test]
fn rf03_cone_drops_the_stage_and_new_mesh_idle_contract_is_preserved() {
    // Picking Cone drops the active stage first, like every other tool.
    let mut e = moved();
    e.dispatch(Command::ArmCone).unwrap();
    assert!(!e.i2_active());
    assert!(e.session.cone.is_some());
    let mut e = moved();
    let before = e.clone();
    assert!(e.dispatch(Command::NewMesh).is_err());
    assert!(e == before);
}
