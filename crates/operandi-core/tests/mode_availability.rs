use operandi_core::*;
fn fixture() -> Editor {
    let mut e = Editor {
        scene: serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json"))
            .unwrap(),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}
fn begun() -> Editor {
    let mut e = fixture();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e
}
fn refusal(e: &mut Editor, expected: &str) {
    for mode in [e.scene.selection.mode, Mode::Edge] {
        let before = e.clone();
        let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
        assert_eq!(e.dispatch(Command::Mode(mode)).unwrap_err(), expected);
        assert!(*e == before);
        assert_eq!(
            marks,
            (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
        );
    }
}
#[test]
fn mv01_direct_equal_and_changed_mode_preserve_memory_and_branch_context() {
    let mut e = begun();
    e.dispatch(Command::Undo).unwrap();
    // Raw retained prior is diagnostic storage, not a newly admitted workflow.
    e.i2.prior = e.i2.selected.clone();
    let before = e.clone();
    let arrays = (
        e.scene.selection.vertices.clone(),
        e.scene.selection.edges.clone(),
        e.scene.selection.faces.clone(),
    );
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    assert!(e.i2 == before.i2);
    assert_eq!(e.revision, before.revision + 1);
    assert_eq!(e.identities.publication, before.identities.publication + 1);
    assert_eq!(e.identities.request, before.identities.request + 1);
    for mode in [Mode::Vertex, Mode::Edge, Mode::Item, Mode::Polygon] {
        let history = e.history.undo.len();
        let mesh = e.scene.active().unwrap().mesh.clone();
        e.dispatch(Command::Mode(mode)).unwrap();
        assert!(e.i2.selected.is_none() && e.i2.prior.is_none());
        assert_eq!(e.history.undo.len(), history);
        assert_eq!(e.scene.active().unwrap().mesh, mesh);
        assert_eq!(
            (
                e.scene.selection.vertices.clone(),
                e.scene.selection.edges.clone(),
                e.scene.selection.faces.clone()
            ),
            arrays
        );
    }
    e.scene.select_item(None);
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    let mut e = fixture();
    e.dispatch(Command::BeginSelection).unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    assert!(e.selection_start.is_some()); // existing API characterization only
}
#[test]
fn mv03_surface_drop_then_mode_matches_existing_nested_boundary() {
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        for substantive in [false, true] {
            let mut e = fixture();
            e.dispatch(Command::Activate(tool)).unwrap();
            if substantive {
                let mut p = e.session.live.as_ref().unwrap().params.clone();
                p.smooth_shift.offset = 0.1;
                e.dispatch(Command::Preview(p)).unwrap();
            }
            let before = e.clone();
            let mut manual = e.clone();
            manual.dispatch(Command::Drop).unwrap();
            manual.dispatch(Command::Mode(Mode::Edge)).unwrap();
            e.dispatch(Command::Mode(Mode::Edge)).unwrap();
            assert_eq!(e.scene, manual.scene);
            assert!(e.history.undo == manual.history.undo);
            assert!(e.history.redo == manual.history.redo);
            assert_eq!(
                *e.presets.get::<operandi_core::SmoothShiftPreset>(),
                *manual.presets.get::<operandi_core::SmoothShiftPreset>()
            );
            assert_eq!(
                *e.presets.get::<operandi_core::ThickenPreset>(),
                *manual.presets.get::<operandi_core::ThickenPreset>()
            );
            assert!(e.session.live.is_none());
            assert_eq!(e.revision, manual.revision);
            assert_eq!(e.identities.publication, manual.identities.publication + 1);
            assert_eq!(e.identities.request, manual.identities.request + 1);
            assert_eq!(
                e.history.undo.len(),
                before.history.undo.len() + usize::from(substantive)
            );
            if substantive {
                assert_eq!(e.history.undo.last().unwrap().label, tool.label());
            }
        }
    }
}
#[test]
fn mv03_surface_errors_and_cone_overlap_keep_authoritative_drop_order() {
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        let mut e = fixture();
        e.dispatch(Command::Activate(tool)).unwrap();
        e.session.live.as_mut().unwrap().preview_error = Some("surface invalid".into());
        refusal(
            &mut e,
            "Correct the current stage or Cancel before Drop: surface invalid",
        );
        let target = e.scene.selection.item.unwrap();
        e.session.cone = Some(NumericCone {
            target,
            requested: Default::default(),
            preview: None,
            error: Some("cone invalid".into()),
            applied_step: None,
        });
        refusal(
            &mut e,
            "Correct Cone fields or Cancel before Drop: cone invalid",
        );
        // A raw overlap: Drop closes Cone, but the surface tool stays open,
        // so the mode change refuses and nothing changes.
        e.session.cone.as_mut().unwrap().error = None;
        refusal(&mut e, "Finish the current tool before changing mode");
    }
}
#[test]
fn mv04_late_publication_failure_does_not_publish_mode_or_surface_drop() {
    let mut same = fixture();
    same.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    same.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    assert!(same.session.live.is_none());
    let mut e = fixture();
    e.identities.publication = u64::MAX;
    let before = e.clone();
    assert_eq!(
        e.dispatch(Command::Mode(Mode::Edge)).unwrap_err(),
        "Publication budget exhausted"
    );
    assert!(e == before);
}

#[test]
fn mv04_route_queries_are_pure_and_never_a_cached_permit() {
    let mut e = fixture();
    let before = e.clone();
    let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
    for _ in 0..100 {
        assert_eq!(e.mode_route_eligibility(), ModeRouteEligibility::Direct);
    }
    assert!(e == before);
    assert_eq!(
        marks,
        (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
    );
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert_eq!(
        e.mode_route_eligibility(),
        ModeRouteEligibility::DropsToolFirst
    );
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Activate(Tool::SmoothShift)).unwrap();
    assert_eq!(
        e.mode_route_eligibility(),
        ModeRouteEligibility::DropsToolFirst
    );
    let before = e.clone();
    let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
    for _ in 0..100 {
        assert_eq!(
            e.mode_route_eligibility(),
            ModeRouteEligibility::DropsToolFirst
        );
    }
    assert!(e == before);
    assert_eq!(
        marks,
        (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
    );
    e.session.live.as_mut().unwrap().preview_error = Some("changed after presentation".into());
    assert_eq!(
        e.mode_route_eligibility(),
        ModeRouteEligibility::DropsToolFirst
    );
    refusal(
        &mut e,
        "Correct the current stage or Cancel before Drop: changed after presentation",
    );
    let target = e.scene.selection.item.unwrap();
    e.session.cone = Some(NumericCone {
        target,
        requested: Default::default(),
        preview: None,
        error: Some("cone first".into()),
        applied_step: None,
    });
    assert_eq!(
        e.mode_route_eligibility(),
        ModeRouteEligibility::DropsToolFirst
    );
    refusal(
        &mut e,
        "Correct Cone fields or Cancel before Drop: cone first",
    );
}
