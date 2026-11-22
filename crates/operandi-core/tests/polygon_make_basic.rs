//! MK-A01 core acceptance for MP-C01 Make (MK-C01..03).
//!
//! Inputs come from the admitted original fixture MK-F01; every expected Scene
//! is written out literally (fixture loops and tags, hand-listed attribute
//! rows). Nothing expected is produced by running Make.
use glam::DVec3;
use operandi_core::*;

fn make_editor(points: &[(f64, f64)], loops: &[&[u32]], edges: Vec<(u32, u32)>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: points.iter().map(|&(x, y)| DVec3::new(x, y, 0.)).collect(),
        faces: loops
            .iter()
            .map(|l| Face {
                vertices: l.to_vec(),
                uv: None,
                material: "Grid".into(),
                material_label: Some("Grid".into()),
                part: None,
                smooth: true,
            })
            .collect(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = edges;
    e.scene.validate().unwrap();
    e
}

#[test]
fn make_completes_a_missing_corner_with_a_new_point() {
    // An L of three quads; the inner corner's two edges get a fourth point.
    let points = [
        (0., 0.),
        (1., 0.),
        (2., 0.),
        (0., 1.),
        (1., 1.),
        (2., 1.),
        (0., 2.),
        (1., 2.),
    ];
    let mut e = make_editor(
        &points,
        &[&[0, 1, 4, 3], &[1, 2, 5, 4], &[3, 4, 7, 6]],
        vec![(4, 5), (4, 7)],
    );
    e.dispatch(Command::MakePolygon).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.vertices.len(), 9);
    assert_eq!(m.vertices[8], DVec3::new(2., 2., 0.));
    let made = m.faces.last().unwrap();
    assert_eq!(made.vertices.len(), 4);
    assert_eq!(
        made.material_label.as_deref(),
        Some("Grid"),
        "tags from the first edge's polygon"
    );
    // Consistent with its neighbours: no exceptional edge uses.
    assert!(m.attributes.extrude_edge_uses.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 8);
}

#[test]
fn make_refuses_two_separate_edges() {
    // Two quads with a gap between x = 1 and x = 2.
    let points = [
        (0., 0.),
        (1., 0.),
        (1., 1.),
        (0., 1.),
        (2., 0.),
        (3., 0.),
        (3., 1.),
        (2., 1.),
    ];
    let mut e = make_editor(
        &points,
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
        vec![(1, 2), (4, 7)],
    );
    // make-edges-2026-11-06 A2: border edges of two separate quads make
    // nothing (was a bridging quad before that spec).
    assert!(e.dispatch(Command::MakePolygon).is_err());
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 2);
}
