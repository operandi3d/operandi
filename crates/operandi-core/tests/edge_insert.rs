//! ME-T08 Add Loop and ME-T09 Add Point.
use glam::DVec3;
use operandi_core::*;

fn cube() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Edge;
    e.synchronize_identities().unwrap();
    e
}

fn plain(points: &[(f64, f64)], loops: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|&(x, y)| DVec3::new(x, y, 0.)).collect(),
        faces: loops
            .iter()
            .map(|l| Face {
                vertices: l.to_vec(),
                uv: None,
                material: "M".into(),
                material_label: None,
                smooth: false,
                part: Some("P".into()),
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn add_point_splits_the_edge_in_every_polygon() {
    let mut e = cube();
    // Edge 0-1 is shared by the bottom face and a side face.
    e.dispatch(Command::AddEdgePoint {
        a: 0,
        b: 1,
        t: 0.25,
    })
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.vertices.len(), 9);
    let p = m.vertices[8];
    let want = m.vertices[0].lerp(m.vertices[1], 0.25);
    assert!(p.distance(want) < 1e-12);
    let users = m.faces.iter().filter(|f| f.vertices.contains(&8)).count();
    assert_eq!(users, 2, "no T-junction");
    assert_eq!(e.undo.last().unwrap().label, "Add Point");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices.len(), 8);
    // Refusals.
    for (a, b, t) in [(0, 6, 0.5), (0, 1, 0.), (0, 1, 1.), (0, 1, f64::NAN)] {
        let before = e.clone();
        assert!(e.dispatch(Command::AddEdgePoint { a, b, t }).is_err());
        assert!(e == before);
    }
}

#[test]
fn add_loop_closes_around_a_cube() {
    let mut e = cube();
    e.dispatch(Command::AddEdgeLoop { a: 0, b: 1, t: 0.5 })
        .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    // A closed loop through the four faces around edge 0-1's band.
    assert_eq!(m.faces.len(), 10);
    assert_eq!(m.vertices.len(), 12);
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
    assert_eq!(e.scene.selection.edges.len(), 4, "loop edges selected");
}

#[test]
fn add_loop_runs_to_the_boundary_and_stops_at_a_triangle() {
    // Strip of three quads; edge 0-4 is at the left end.
    let pts = [
        (0., 0.),
        (1., 0.),
        (2., 0.),
        (3., 0.),
        (0., 1.),
        (1., 1.),
        (2., 1.),
        (3., 1.),
    ];
    let strip = plain(&pts, &[&[0, 1, 5, 4], &[1, 2, 6, 5], &[2, 3, 7, 6]]);
    // Cut across the strip: start on the bottom edge 1-2 (the band runs
    // vertically, one quad high), so one cut through quad 1.
    let (out, edges) = add_loop(&strip, 1, 2, 0.5).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 4);
    assert_eq!(edges.len(), 1);
    // Along the strip: the band of edge 0-4 crosses all three quads.
    let (out, edges) = add_loop(&strip, 0, 4, 0.5).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 6);
    assert_eq!(edges.len(), 3);
    assert!(out.faces.iter().all(|f| f.part.as_deref() == Some("P")));
    // A triangle at the end takes the point but is not cut.
    let pts = [(0., 0.), (1., 0.), (0., 1.), (1., 1.), (2., 0.5)];
    let capped = plain(&pts, &[&[0, 1, 3, 2], &[1, 4, 3]]);
    let (out, edges) = add_loop(&capped, 0, 2, 0.5).unwrap();
    out.validate().unwrap();
    assert_eq!(edges.len(), 1);
    let tri = out.faces.iter().find(|f| f.vertices.contains(&4)).unwrap();
    assert_eq!(tri.vertices.len(), 4, "triangle gained the loop point");
}
