use glam::DVec3;
use operandi_core::*;

fn params(mode: ExtrudeMode, z: f64) -> Parameters {
    Parameters {
        extent: DVec3::new(0., 0., z),
        extrude_mode: mode,
        extrude_sides: 1,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}

fn cube() -> Editor {
    let scene = persistence::decode(include_bytes!("fixtures/i1/cube-source.opscene")).unwrap();
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(params(ExtrudeMode::Shift, 0.5)))
        .unwrap();
    e
}

fn assert_refuses_unchanged(mut e: Editor, command: Command) {
    let ticket = e.identity_ticket().unwrap();
    let ids = &e.identities.current.meshes[&1];
    let handles: Vec<_> = e
        .scene
        .selection
        .faces
        .iter()
        .map(|&f| FaceHandle {
            scope: ids.scope,
            face: ids.faces[f as usize],
        })
        .collect();
    let before = e.clone();
    let next = e.identities.next_id().unwrap();
    let result = e.dispatch_faces(ticket, &handles, command);
    assert!(
        result.is_err(),
        "fresh displayed targets must not authorize editing a different armed basis"
    );
    assert!(
        e == before,
        "refusal must preserve controller/session/history"
    );
    assert_eq!(e.identities.next_id().unwrap(), next);
}

#[test]
fn r1_changed_displayed_face_preview_refuses_fresh_targets() {
    let mut e = cube();
    assert!(
        e.dispatch(Command::SelectPolygonSet("bottom".into()))
            .is_err()
    );
    e.scene.selection.faces = vec![0]; // Existing public legacy selection boundary.
    assert_refuses_unchanged(e, Command::Preview(params(ExtrudeMode::Shift, 0.5)));
}

#[test]
fn r1_changed_displayed_face_apply_refuses_fresh_targets() {
    let mut e = cube();
    e.scene.selection.faces = vec![0];
    assert_refuses_unchanged(e, Command::Apply);
}
