use glam::DVec3;
use operandi_core::*;
#[test]
fn s1_t03_t04_field_normalization() {
    assert_eq!(
        BridgeParameters::twist_field(-2147483648.).unwrap(),
        -2147483647
    );
    assert_eq!(
        BridgeParameters::twist_field(2147483648.).unwrap(),
        2147483647
    );
    assert_eq!(BridgeParameters::twist_field(1.5).unwrap(), 1);
    assert_eq!(BridgeParameters::segments_field(-2.).unwrap(), 0);
    assert_eq!(BridgeParameters::segments_field(1.5).unwrap(), 1);
    assert!(BridgeParameters::segments_field(1e100).is_err());
}

#[test]
fn bridge_blocks_parked_i2_resume_and_cancel_closes_bridge() {
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
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 0);
    e.dispatch(Command::ArmBridge).unwrap();
    assert!(e.session.bridge.is_some());
    let before = e.scene.clone();
    let failure = e
        .dispatch_history_reported(HistoryRequest::Redo)
        .unwrap_err();
    assert!(matches!(
        failure.reason,
        HistoryFailureReason::History(HistoryRefusal::ResumeOwner)
    ));
    assert_eq!(e.scene, before);
    let cancel = e.dispatch_history_reported(HistoryRequest::Cancel).unwrap();
    assert!(cancel.foreground_closed);
    assert!(e.session.bridge.is_none());
    assert_eq!(e.scene, before);
}

#[test]
fn bridge_switches_from_active_i2_without_overlapping_owners() {
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
    e.dispatch(Command::ArmBridge).unwrap();
    assert!(e.session.bridge.is_some());
    assert!(e.session.live.is_none());
    assert!(!e.i2.selected.as_ref().is_some_and(|s| s.cursor > 0));
    e.scene.validate().unwrap();
}

#[test]
fn bridge_two_same_facing_clusters_into_a_handle() {
    // Row of five quads on z = 0, all facing +Z; bridge [0, 1] to [3, 4].
    let vertices: Vec<DVec3> = (0..2)
        .flat_map(|y| (0..6).map(move |x| DVec3::new(x as f64, y as f64, 0.)))
        .collect();
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: "Strip".into(),
        material_label: None,
        smooth: false,
        part: None,
    };
    let mesh = Mesh {
        vertices,
        faces: (0..5u32)
            .map(|x| face(vec![x, x + 1, x + 7, x + 6]))
            .collect(),
        ..Default::default()
    };
    mesh.validate().unwrap();
    let p = BridgeParameters {
        segments: 4,
        ..Default::default()
    };
    let out = bridge_geometry(&mesh, &[0, 1, 3, 4], p).unwrap();
    out.mesh.validate().unwrap();
    // Sections removed (Remove Polygons default); 6 loop edges x 4 segments.
    assert_eq!(out.mesh.faces.len(), 1 + 6 * 4);
    // Curve mode arcs off the plane between same-facing sections.
    assert!(out.mesh.vertices.iter().any(|v| v.z.abs() > 0.1));
    // A single cluster is not enough.
    assert!(bridge_geometry(&mesh, &[0, 1], p).is_err());
}
