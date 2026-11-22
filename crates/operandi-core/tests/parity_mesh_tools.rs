//! Behaviour parity fixes PF-08, PF-09, PF-10 and PF-12 (mesh tools), with
//! the exact cases of BEHAVIOR-PARITY-FIXES.
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

fn editor(vertices: Vec<DVec3>, faces: Vec<Vec<u32>>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    mesh.vertices = vertices;
    mesh.faces = faces.into_iter().map(face).collect();
    e.synchronize_identities().unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn close(a: DVec3, b: DVec3, tol: f64) -> bool {
    (a - b).abs().max_element() <= tol
}

// ---- PF-08 v2 Flatten Ngons: Newell normal through the bbox centre

fn newell(points: &[DVec3]) -> DVec3 {
    let n = points.len();
    (0..n)
        .map(|i| points[i].cross(points[(i + 1) % n]))
        .sum::<DVec3>()
        .normalize()
}

fn bbox_centre(points: &[DVec3]) -> DVec3 {
    let lo = points
        .iter()
        .fold(DVec3::splat(f64::INFINITY), |a, &p| a.min(p));
    let hi = points
        .iter()
        .fold(DVec3::splat(f64::NEG_INFINITY), |a, &p| a.max(p));
    (lo + hi) * 0.5
}

/// The PF-08 pentagon beside a non-planar quad (points 5..8).
fn pentagon_and_quad() -> Editor {
    editor(
        vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 1., 0.3),
            DVec3::new(1., 2., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(5., 0., 0.),
            DVec3::new(6., 0., 0.),
            DVec3::new(6., 1., 0.4),
            DVec3::new(5., 1., 0.),
        ],
        vec![vec![0, 1, 2, 3, 4], vec![5, 6, 7, 8]],
    )
}

#[test]
fn pf08_points_move_along_the_newell_normal_onto_the_box_centre_plane() {
    let mut e = pentagon_and_quad();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(5, 6)];
    let before: Vec<DVec3> = mesh(&e).vertices.clone();
    let n = newell(&before[..5]);
    let c = bbox_centre(&before[..5]);
    assert!(close(c, DVec3::new(1., 1., 0.15), 1e-12));
    e.dispatch(Command::FlattenNgons).unwrap();
    let after = mesh(&e).vertices.clone();
    for i in 0..5 {
        // On the plane through C with normal n ...
        assert!((after[i] - c).dot(n).abs() < 1e-12, "{i}");
        // ... reached along n only.
        let moved = after[i] - before[i];
        assert!(moved.cross(n).length() < 1e-12, "{i}: {moved}");
        let want = before[i] - n * (before[i] - c).dot(n);
        assert!(close(after[i], want, 1e-12));
    }
    // The non-planar quad is untouched; nothing is selected in any mode.
    assert_eq!(&after[5..], &before[5..]);
    assert!(e.scene.selection.edges.is_empty());
    assert!(e.scene.selection.faces.is_empty() && e.scene.selection.vertices.is_empty());
}

#[test]
fn pf08_again_keeps_n_and_recentres_on_the_new_box() {
    let mut e = pentagon_and_quad();
    e.dispatch(Command::FlattenNgons).unwrap();
    let once: Vec<DVec3> = mesh(&e).vertices[..5].to_vec();
    let n = newell(&once);
    e.dispatch(Command::FlattenNgons).unwrap();
    let twice: Vec<DVec3> = mesh(&e).vertices[..5].to_vec();
    assert!((newell(&twice) - n).length() < 1e-9);
    let c = bbox_centre(&once);
    for p in &twice {
        assert!((*p - c).dot(n).abs() < 1e-9);
    }
}

// ---- PF-09 Freeze of a smooth Curve

fn smooth(points: &[[f64; 3]], closed: bool) -> Vec<DVec3> {
    let points: Vec<DVec3> = points.iter().map(|&p| DVec3::from(p)).collect();
    let kind = curves::CurveKind::Curve {
        start_control: false,
        end_control: false,
    };
    curves::sample(&points, kind, closed, curves::FREEZE_STEPS)
}

#[test]
fn pf09_open_curves_match_the_check_numbers() {
    let tol = 1e-5;
    let line = smooth(
        &[[0., 0., 0.], [1., 1., 0.], [2., 0., 0.], [3., 1., 0.]],
        false,
    );
    assert_eq!(line.len(), 109);
    for (i, p) in [
        (0, [0., 0., 0.]),
        (36, [1., 1., 0.]),
        (72, [2., 0., 0.]),
        (108, [3., 1., 0.]),
    ] {
        assert!(close(line[i], DVec3::from(p), 1e-12), "{i}");
    }
    assert!(
        close(line[18], DVec3::new(0.5, 0.702254, 0.), tol),
        "{}",
        line[18]
    );

    let line = smooth(
        &[[0., 0., 0.], [1., 1., 0.], [3., 0., 0.], [6., 0., 0.]],
        false,
    );
    assert!(
        close(line[18], DVec3::new(0.436887, 0.68735, 0.), tol),
        "{}",
        line[18]
    );
    assert!(
        close(line[54], DVec3::new(1.870047, 0.599762, 0.), tol),
        "{}",
        line[54]
    );

    let line = smooth(&[[0., 0., 0.], [1., 0., 0.], [2., 1., 0.]], false);
    assert_eq!(line.len(), 73);
    assert!(
        close(line[18], DVec3::new(0.518952, -0.09753, 0.), tol),
        "{}",
        line[18]
    );
}

#[test]
fn pf09_closed_curve_matches_the_check_numbers() {
    let line = smooth(
        &[[0., 0., 0.], [3., 0., 0.], [3., 1., 0.], [0., 2., 0.]],
        true,
    );
    assert_eq!(line.len(), 36 * 4);
    assert!(close(line[0], DVec3::ZERO, 1e-12));
    assert!(
        close(line[18], DVec3::new(1.55625, -0.24375, 0.), 1e-5),
        "{}",
        line[18]
    );
    assert!(
        close(line[54], DVec3::new(3.183845, 0.46875, 0.), 1e-5),
        "{}",
        line[54]
    );
}

#[test]
fn pf09_two_points_stay_a_straight_line() {
    let line = smooth(&[[0., 0., 0.], [2., 1., 0.]], false);
    for p in &line {
        assert!((p.y - p.x / 2.).abs() < 1e-12, "{p}");
    }
}

// ---- PF-10 Edge Split on a closed loop

fn g3() -> Editor {
    let points = (0..16)
        .map(|k| DVec3::new((k % 4) as f64, (k / 4) as f64, 0.))
        .collect();
    let mut quads = Vec::new();
    for j in 0..3u32 {
        for i in 0..3u32 {
            let a = 4 * j + i;
            quads.push(vec![a, a + 1, a + 5, a + 4]);
        }
    }
    editor(points, quads)
}

#[test]
fn pf10_edge_split_round_a_loop_leaves_nothing_selected() {
    let mut e = g3();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(5, 6), (6, 10), (9, 10), (5, 9)];
    e.dispatch(Command::EdgeCommand(EdgeCommand::Split))
        .unwrap();
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 20);
    assert_eq!(m.faces.len(), 9);
    let mut new: Vec<[i64; 3]> = m.vertices[16..]
        .iter()
        .map(|p| [p.x.round() as i64, p.y.round() as i64, p.z.round() as i64])
        .collect();
    new.sort();
    assert_eq!(new, [[1, 1, 0], [1, 2, 0], [2, 1, 0], [2, 2, 0]]);
    assert!(
        e.scene.selection.edges.is_empty(),
        "{:?}",
        e.scene.selection.edges
    );
}

// ---- PF-12 Edge Extend at reset values

#[test]
fn pf12_edge_extend_at_reset_still_adds_the_polygon() {
    let points = (0..9)
        .map(|k| DVec3::new((k % 3) as f64, 0., (k / 3) as f64))
        .collect();
    let mut e = editor(
        points,
        vec![
            vec![0, 3, 4, 1],
            vec![1, 4, 5, 2],
            vec![3, 6, 7, 4],
            vec![4, 7, 8, 5],
        ],
    );
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(0, 1)];
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 11, "{}", m.vertices.len());
    assert!(close(m.vertices[9], DVec3::new(1., 0., 0.), 1e-12));
    assert!(close(m.vertices[10], DVec3::new(0., 0., 0.), 1e-12));
    // [1, 9, 10, 0], from whichever corner it is stored.
    let cyclic = |v: &[u32]| {
        (0..v.len()).any(|k| [v[k..].to_vec(), v[..k].to_vec()].concat() == [1, 9, 10, 0])
    };
    assert!(
        m.faces.iter().any(|f| cyclic(&f.vertices)),
        "{:?} {:?}",
        m.faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>(),
        m.vertices
    );
    assert_eq!(e.scene.selection.edges, [(9, 10)]);
    // ... and its points, for Vertex mode.
    let mut points = e.scene.selection.vertices.clone();
    points.sort();
    assert_eq!(points, [9, 10]);
}
