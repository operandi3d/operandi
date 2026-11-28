//! Triple Fan and Collapse Selection (TOPOLOGY-CLEANUP-TOOLS TC-01..12),
//! and the Axis Rotate preset row (TC-30). Original test meshes built to
//! the probes' descriptions.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn editor(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Polygon;
    e.history.undo.clear();
    e
}

fn hexagon() -> Mesh {
    Mesh {
        vertices: (0..6)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 6.;
                DVec3::new(a.cos(), 0., a.sin())
            })
            .collect(),
        faces: vec![face((0..6).collect())].into(),
        ..Default::default()
    }
}

/// A 4 x 4 quad grid in XY (points 0..25, row by row).
fn grid4() -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..5 {
        for x in 0..5 {
            vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let mut faces = Vec::new();
    for y in 0..4u32 {
        for x in 0..4u32 {
            let i = y * 5 + x;
            faces.push(face(vec![i, i + 1, i + 6, i + 5]));
        }
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

#[test]
fn tf1_tf2_hexagon_fans_from_the_selected_corner() {
    let mut e = editor(hexagon());
    let normal = e.scene.active().unwrap().mesh.normal(0);
    e.scene.selection.faces = vec![0];
    e.scene.selection.vertices = vec![0];
    e.dispatch(Command::TripleFan).unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    assert_eq!(m.faces.len(), 4, "N corners -> N - 2 triangles");
    for (f, t) in m.faces.iter().enumerate() {
        assert_eq!(t.vertices.len(), 3);
        assert_eq!(t.vertices[0], 0, "every triangle meets at the pivot");
        assert!(m.normal(f).dot(normal) > 0.999, "facing unchanged");
    }
    let mut corners: Vec<Vec<u32>> = m.faces.iter().map(|f| f.vertices.clone()).collect();
    corners.sort();
    assert_eq!(
        corners,
        vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4], vec![0, 4, 5]]
    );
    // TF2: nothing selected, one undo step.
    assert!(e.scene.selection.faces.is_empty() && e.scene.selection.vertices.is_empty());
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 1);
}

#[test]
fn tf3_no_vertex_or_a_vertex_off_the_selection_changes_nothing() {
    let mut e = editor(grid4());
    e.scene.selection.faces = vec![0];
    assert!(e.dispatch(Command::TripleFan).is_err());
    e.scene.selection.vertices = vec![24];
    assert!(e.dispatch(Command::TripleFan).is_err());
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 16);
    assert!(e.history.undo.is_empty());
    // Two polygons sharing point 6 both fan from it.
    e.scene.selection.faces = vec![0, 1];
    e.scene.selection.vertices = vec![6];
    e.dispatch(Command::TripleFan).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 18);
}

#[test]
fn cs1_cs2_two_quads_collapse_to_a_fan_round_the_outline_centre() {
    let mut e = editor(grid4());
    // Two neighbouring quads: (1,1)-(2,2) and (2,1)-(3,2).
    e.scene.selection.faces = vec![5, 6];
    e.dispatch(Command::CollapseSelection).unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    assert_eq!(m.vertices.len(), 26);
    assert_eq!(m.faces.len(), 20);
    let centre = *m.vertices.last().unwrap();
    assert!(centre.distance(DVec3::new(2., 1.5, 0.)) < 1e-12);
    // Outline points stay put.
    for (a, b) in grid4().vertices.iter().zip(&m.vertices) {
        assert_eq!(a, b);
    }
    assert!(e.scene.selection.faces.is_empty());
    assert_eq!(e.history.undo.len(), 1);
    m.validate().unwrap();
}

#[test]
fn cs2_inner_points_go_and_cs3_regions_get_a_centre_each() {
    // A 2 x 2 block has one inner point (12).
    let mut e = editor(grid4());
    e.scene.selection.faces = vec![5, 6, 9, 10];
    e.dispatch(Command::CollapseSelection).unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    // 25 - 1 inner + 1 centre; 16 - 4 + 8 triangles.
    assert_eq!(m.vertices.len(), 25);
    assert_eq!(m.faces.len(), 20);
    assert!(
        m.vertices
            .iter()
            .any(|v| v.distance(DVec3::new(2., 2., 0.)) < 1e-12),
        "the centre sits where the inner point was"
    );
    // Two separate quads: one centre each.
    let mut e = editor(grid4());
    e.scene.selection.faces = vec![0, 15];
    e.dispatch(Command::CollapseSelection).unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    assert_eq!(m.vertices.len(), 27);
    assert_eq!(m.faces.len(), 14 + 8);
}

#[test]
fn collapse_selection_refuses_item_mode_and_a_whole_closed_mesh() {
    let mut e = editor(Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap());
    e.scene.selection.faces = (0..6).collect();
    assert!(e.dispatch(Command::CollapseSelection).is_err());
    e.scene.selection.mode = Mode::Item;
    assert!(e.dispatch(Command::CollapseSelection).is_err());
    assert!(e.history.undo.is_empty());
}

#[test]
fn ar3_axis_rotate_is_a_rotate_preset_with_one_ring() {
    let mut e = editor(grid4());
    e.dispatch(Command::ActivatePreset("AxisRotate".into()))
        .unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Rotate);
    assert_eq!(e.live_tool_label(), Some("Axis Rotate"));
    assert!(e.session.preset.as_ref().unwrap().row().actor.single_ring);
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.preset.is_none());
    // Item mode works too (it's a transform).
    e.scene.selection.mode = Mode::Item;
    e.dispatch(Command::ActivatePreset("AxisRotate".into()))
        .unwrap();
}
