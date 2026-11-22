use glam::DVec3;
use operandi_core::*;
#[test]
fn parked_extrude_refusal_preserves_real_redo() {
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
    assert!(e.i2_can_redo());
    let before = e.clone();
    // Nothing to quadruple: a quiet no-op (CMD-B3-03) that keeps redo.
    let _ = e.dispatch(Command::QuadruplePolygon);
    assert!(e == before);
    assert!(e.i2_can_redo());
}

#[test]
fn two_rows_of_five_become_four_quads() {
    // Pen-style polygon: bottom row left to right, top row right to left.
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let bottom = (0..5).map(|x| DVec3::new(x as f64, 0., 0.));
    let top = (0..5)
        .rev()
        .map(|x| DVec3::new(x as f64, 1. + 0.1 * x as f64, 0.));
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: bottom.chain(top).collect(),
        faces: vec![Face {
            vertices: (0..10).collect(),
            uv: None,
            material: String::new(),
            material_label: None,
            smooth: false,
            part: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.synchronize_identities().unwrap();
    e.dispatch(Command::QuadruplePolygon).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 4);
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
    // Rungs join opposite row corners: 1-8, 2-7, 3-6.
    for (a, b) in [(1, 8), (2, 7), (3, 6)] {
        assert!(m.edges().contains(&(a, b)), "missing rung {a}-{b}");
    }
}
