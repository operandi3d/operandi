//! Next Loop / Previous Loop: the measured Grid4 cases (SEL-LP-02/03,
//! SEL-B2-01, specifications/selection-2026-11-02) and the default sphere.
use glam::DVec3;
use operandi_core::*;

/// Grid4: 4×4 quads in XY; point = row × 5 + column, polygon
/// [a, a+1, a+6, a+5] with a = row × 5 + column.
fn grid4() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    for y in 0..5 {
        for x in 0..5 {
            mesh.vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    for row in 0..4u32 {
        for column in 0..4u32 {
            let a = row * 5 + column;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + 6, a + 5],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    e
}

fn step(e: &mut Editor, next: bool) -> std::result::Result<(), String> {
    let op = if next {
        SelectionOperation::NextLoop
    } else {
        SelectionOperation::PreviousLoop
    };
    e.dispatch(Command::Selection(op)).map(|_| ())
}

fn polygons(e: &mut Editor, start: &[u32], next: bool, steps: usize) -> Vec<Vec<u32>> {
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = start.to_vec();
    (0..steps)
        .map(|_| {
            step(e, next).unwrap();
            e.scene.selection.faces.clone()
        })
        .collect()
}

#[test]
fn polygon_next_and_previous_match_the_measured_grid() {
    let mut e = grid4();
    // SEL-B2-01: column at X=2.5, twice each way.
    assert_eq!(
        polygons(&mut e, &[2, 6, 10, 14], true, 2),
        [vec![3, 7, 11, 15], vec![14, 10, 6, 2]]
    );
    assert_eq!(
        polygons(&mut e, &[2, 6, 10, 14], false, 2),
        [vec![1, 5, 9, 13], vec![0, 4, 8, 12]]
    );
    // SEL-LP-02: row 4–7, Next → row 0–3, then Previous → back.
    assert_eq!(polygons(&mut e, &[4, 5, 6, 7], true, 1), [vec![0, 1, 2, 3]]);
    step(&mut e, false).unwrap();
    assert_eq!(e.scene.selection.faces, vec![4, 5, 6, 7]);
}

#[test]
fn edge_next_and_previous_match_the_measured_grid() {
    let x2 = vec![(2, 7), (7, 12), (12, 17), (17, 22)];
    let x1 = vec![(1, 6), (6, 11), (11, 16), (16, 21)];
    for next in [true, false] {
        // SEL-B2-01: both directions go to X=1, then the whole border.
        let mut e = grid4();
        e.scene.selection.mode = Mode::Edge;
        e.scene.selection.edges = x2.clone();
        step(&mut e, next).unwrap();
        assert_eq!(e.scene.selection.edges, x1, "next={next}");
        step(&mut e, next).unwrap();
        let mut border = e.scene.selection.edges.clone();
        assert_eq!(border.len(), 16);
        border.sort_unstable();
        let mut expected = vec![
            (0, 1),
            (0, 5),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 9),
            (5, 10),
            (9, 14),
            (10, 15),
            (14, 19),
            (15, 20),
            (19, 24),
            (20, 21),
            (21, 22),
            (22, 23),
            (23, 24),
        ];
        expected.sort_unstable();
        assert_eq!(border, expected);
        // SEL-LP-03: Next from the border goes back to the X=1 loop.
        step(&mut e, true).unwrap();
        assert_eq!(e.scene.selection.edges, x1);
    }
}

#[test]
fn each_step_is_one_undo_and_bad_input_reports() {
    let mut e = grid4();
    e.scene.selection.faces = vec![2, 6, 10, 14];
    let before = e.scene.selection.clone();
    step(&mut e, true).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection, before);

    for (mode, error) in [
        (Mode::Polygon, "Select a loop first"),
        (Mode::Vertex, "Select a loop first"),
    ] {
        e.scene.selection.mode = mode;
        e.scene.selection.faces.clear();
        e.scene.selection.vertices.clear();
        let scene = e.scene.clone();
        let message = step(&mut e, true).unwrap_err();
        assert!(message.contains(error), "{message}");
        assert_eq!(e.scene, scene);
    }
    // Point loops step too (PF-06); one polygon isn't a loop.
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![2, 7, 12, 17, 22];
    step(&mut e, true).unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![5];
    assert!(step(&mut e, true).unwrap_err().contains("loop"));
}

fn sphere() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e
}

/// The latitude (point height) of a polygon band or edge loop: all its
/// points' centre height.
fn height(e: &Editor) -> f64 {
    let m = &e.scene.active().unwrap().mesh;
    let s = &e.scene.selection;
    let points: Vec<u32> = match s.mode {
        Mode::Edge => s.edges.iter().flat_map(|&(a, b)| [a, b]).collect(),
        _ => s
            .faces
            .iter()
            .flat_map(|&f| m.faces[f as usize].vertices.clone())
            .collect(),
    };
    points
        .iter()
        .map(|&p| m.vertices[p as usize].y)
        .sum::<f64>()
        / points.len() as f64
}

#[test]
fn sphere_bands_and_rings_step_and_stop_at_the_poles() {
    // Polygons: the equator band steps one row; Previous returns.
    let mut e = sphere();
    let m = &e.scene.active().unwrap().mesh;
    let seeds: Vec<u32> = (0..m.faces.len() as u32)
        .filter(|&f| {
            let c = m.center(f as usize);
            m.faces[f as usize].vertices.len() == 4 && c.y > 0. && c.y < 0.1 && c.x > 0.9 * 0.5
        })
        .collect();
    e.scene.selection.faces = seeds[..2].to_vec();
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    let band = e.scene.selection.faces.clone();
    assert_eq!(band.len(), 24);
    let start = height(&e);
    step(&mut e, true).unwrap();
    assert_eq!(e.scene.selection.faces.len(), 24);
    let moved = height(&e);
    assert!((moved - start).abs() > 0.05);
    step(&mut e, false).unwrap();
    let mut back = e.scene.selection.faces.clone();
    back.sort_unstable();
    let mut sorted = band.clone();
    sorted.sort_unstable();
    assert_eq!(back, sorted);
    // Repeated steps one way reach the triangle cap and refuse.
    let mut steps = 0;
    let error = loop {
        match step(&mut e, true) {
            Ok(()) => steps += 1,
            Err(message) => break message,
        }
        assert!(steps < 12);
    };
    assert!(error.contains("pole"), "{error}");
    assert!(steps >= 3, "{steps}");

    // Edges: the equator loop steps to the next latitude loop of 24.
    let mut e = sphere();
    let m = &e.scene.active().unwrap().mesh;
    let on_equator = |v: u32| m.vertices[v as usize].y.abs() < 1e-9;
    let seed = m
        .edges()
        .into_iter()
        .find(|&(a, b)| on_equator(a) && on_equator(b))
        .unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![seed];
    e.dispatch(Command::Selection(SelectionOperation::Loop))
        .unwrap();
    step(&mut e, true).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 24);
    assert!(height(&e).abs() > 0.05);
    let mut steps = 0;
    let error = loop {
        match step(&mut e, true) {
            Ok(()) => steps += 1,
            Err(message) => break message,
        }
        assert!(steps < 12);
    };
    assert!(error.contains("pole"), "{error}");
}
