//! Draw tools' curve edits (DRAW-00/01).
use glam::DVec3;
use operandi_core::*;

fn curve(points: &[DVec3], face: Option<u32>) -> Command {
    Command::SetCurve(Box::new(CurveEdit {
        item: None,
        face,
        points: points.to_vec(),
        closed: false,
        kind: CurveKind::Curve {
            start_control: false,
            end_control: false,
        },
        label: "Curve Point".into(),
    }))
}

#[test]
fn each_edit_is_one_step_and_rebuilds_the_last_polygon() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.synchronize_identities().unwrap();
    e.undo.clear();
    e.dispatch(curve(&[DVec3::ZERO, DVec3::X], None)).unwrap();
    e.dispatch(curve(&[DVec3::ZERO, DVec3::X, DVec3::Y], Some(6)))
        .unwrap();
    assert_eq!(e.undo.len(), 2);
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 7);
    assert_eq!(m.vertices.len(), 8 + 3, "old curve points went");
    assert!(m.is_polyline(6));
    assert!(matches!(
        m.attributes.representations[6],
        PolygonRepresentation::Polyline {
            curve: CurveKind::Curve { .. },
            ..
        }
    ));
    e.dispatch(curve(&[], Some(6))).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 8);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 7);
}

#[test]
fn curves_save_and_load() {
    let mut e = Editor::default();
    e.dispatch(curve(&[DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z], None))
        .unwrap();
    let json = serde_json::to_string(&e.scene).unwrap();
    let back: Scene = serde_json::from_str(&json).unwrap();
    assert_eq!(back, e.scene);
    // A plain polyline doesn't gain a "curve" field.
    let mut m = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X],
        ..Default::default()
    };
    m.faces.push(Face {
        vertices: vec![0, 1],
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    });
    m.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: false,
        curve: CurveKind::Line,
    }];
    assert!(!serde_json::to_string(&m).unwrap().contains("curve"));
}
