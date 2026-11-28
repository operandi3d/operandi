//! Action Center Local's per-island axes (ACL-01..04, behaviour check
//! 2026-11-18: AC-move-Local-islands and AC-move-Local). Each island moves
//! along its own local X: the principal direction of its points nearest
//! the item's X, pointed along +X; a tie in spread (a square) gives the
//! item's X. Expected directions come from the cases' words, or from an
//! independent power iteration where the words say "major axis".
use glam::{DMat3, DVec3};
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A mesh item with these points and polygons, at `pos` turned `rot_y_deg`
/// about Y, every polygon selected, Action Center Local.
fn scene(points: &[[f64; 3]], polygons: &[&[u32]], pos: DVec3, rot_y_deg: f64) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh {
        vertices: points.iter().map(|&p| DVec3::from_array(p)).collect(),
        faces: polygons.iter().map(|p| face(p)).collect::<Vec<_>>().into(),
        ..Default::default()
    }
    .into();
    item.transform.translation = pos;
    item.transform.rotation = DVec3::new(0., rot_y_deg.to_radians(), 0.);
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = (0..polygons.len() as u32).collect();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Local,
        axis: AxisOrientation::Local,
    }))
    .unwrap();
    e
}

/// World points after a Move of `delta` (along the handle's axes).
fn moved(e: &mut Editor, delta: DVec3) -> (Vec<DVec3>, Vec<DVec3>) {
    let world = |e: &Editor| {
        let item = e.scene.active().unwrap();
        let w = e.scene.world_matrix(item.id).unwrap();
        item.mesh
            .vertices
            .iter()
            .map(|&v| w.transform_point3(v))
            .collect::<Vec<_>>()
    };
    let before = world(e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = delta;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    (before, world(e))
}

/// The direction of most spread of `points` about their average, pointed
/// along +X: power iteration on the covariance (apart from the tool's
/// Jacobi solve).
fn major_axis(points: &[DVec3]) -> DVec3 {
    let n = points.len() as f64;
    let mean = points.iter().copied().sum::<DVec3>() / n;
    let c = points.iter().fold(DMat3::ZERO, |c, &p| {
        let d = p - mean;
        c + DMat3::from_cols(d * d.x, d * d.y, d * d.z) / n
    });
    let mut v = DVec3::new(1., 0.2, 0.1).normalize();
    for _ in 0..5000 {
        v = (c * v).normalize();
    }
    if v.x < 0. { -v } else { v }
}

fn top_view(deg: f64) -> DVec3 {
    let r = deg.to_radians();
    DVec3::new(r.cos(), 0., r.sin())
}

const POINTS: [[f64; 3]; 37] = [
    [-0.64086, 0.0, -0.29884],
    [-0.29884, 0.0, 0.64086],
    [0.64086, 0.0, 0.29884],
    [0.29884, 0.0, -0.64086],
    [2.39406, 0.0, 0.164],
    [2.96764, 0.0, 0.98315],
    [4.60594, 0.0, -0.164],
    [4.03236, 0.0, -0.98315],
    [6., 0., 0.],
    [6., 0.5, 1.],
    [7., 0.5, 1.],
    [7., 0., 0.],
    [8., 1., 1.],
    [8., 0.5, 0.],
    [10., 0., 0.],
    [10., 0., 1.],
    [11., 0., 1.],
    [11., 0., 0.],
    [12., 0., 1.],
    [12., 0., 0.],
    [10., 0., 2.],
    [11., 0., 2.],
    [14., 0., 0.],
    [14.4, 0., 1.2],
    [15.5, 0., 0.3],
    [17.0, 0.0, -0.5],
    [17.0, 0.0, 1.5],
    [18.0, 0.0, 1.5],
    [18.0, 0.0, -0.5],
    [19.5, -0.25, 0.06699],
    [19.5, 0.25, 0.93301],
    [21.5, 0.25, 0.93301],
    [21.5, -0.25, 0.06699],
    [23.93301, 0.0, -0.61603],
    [23.06699, 0.0, -0.11603],
    [24.06699, 0.0, 1.61603],
    [24.93301, 0.0, 1.11603],
];

/// ACL-01..04, AC-move-Local-islands: eight islands, every polygon
/// selected, Move 2.55 along the handle's X. Each island moves 2.55 along
/// its own X.
#[test]
fn ac_move_local_islands() {
    let polygons: [&[u32]; 11] = [
        &[0, 1, 2, 3],
        &[4, 5, 6, 7],
        &[8, 9, 10, 11],
        &[11, 10, 12, 13],
        &[14, 15, 16, 17],
        &[17, 16, 18, 19],
        &[15, 20, 21, 16],
        &[22, 23, 24],
        &[25, 26, 27, 28],
        &[29, 30, 31, 32],
        &[33, 34, 35, 36],
    ];
    let pts = |r: std::ops::RangeInclusive<usize>| -> Vec<DVec3> {
        r.map(|i| DVec3::from_array(POINTS[i])).collect()
    };
    let islands: [(&str, std::ops::RangeInclusive<usize>, DVec3); 8] = [
        // ACL-03: a turned square has no preferred direction.
        ("turned square", 0..=3, DVec3::X),
        // 2×1 turned 35°: its long side, -35° in the top view.
        ("2x1 turned 35", 4..=7, top_view(-35.)),
        // Tilted two-quad strip: its 3D major axis.
        ("tilted strip", 8..=13, major_axis(&pts(8..=13))),
        // Axis-aligned L of three quads: the -45° diagonal (ACL-Q3).
        ("L of three quads", 14..=21, top_view(-45.)),
        // Scalene triangle: its major axis.
        ("scalene triangle", 22..=24, major_axis(&pts(22..=24))),
        // 1×2, long side along Z: its short side, world X.
        ("1x2 along Z", 25..=28, DVec3::X),
        // 2×1 tilted 30° about X: world X, its major axis.
        ("2x1 tilted about X", 29..=32, DVec3::X),
        // 2×1 turned -60°: its short side, -30°, nearer world X.
        ("2x1 turned -60", 33..=36, top_view(-30.)),
    ];
    let mut e = scene(&POINTS, &polygons, DVec3::ZERO, 0.);
    let (before, after) = moved(&mut e, DVec3::new(2.55, 0., 0.));
    for (name, range, dir) in islands {
        let strip = &pts(8..=13);
        if name == "tilted strip" {
            let s = major_axis(strip);
            assert!(s.x > 0.8 && s.y > 0. && s.z > 0., "strip axis {s:?}");
        }
        for i in range {
            let want = before[i] + dir * 2.55;
            // The case's points carry five decimals.
            assert!(
                (after[i] - want).length() < 1e-4,
                "{name}, point {i}: {:?} != {want:?}",
                after[i]
            );
        }
    }
}

/// ACL-04, AC-move-Local: the check mesh's two islands in its item at
/// (2, 0, 1) turned 30° about Y. The strip moves along its 3D major axis,
/// in the item's space (ACL-Q4); its square island is a tie, not checked
/// by the case, and here goes along the item's X.
#[test]
fn ac_move_local_in_a_turned_item() {
    let points = [
        [0., 0., 0.],
        [1., 0., 0.],
        [2., 0.5, 0.],
        [0., 0.5, 1.],
        [1., 0.5, 1.],
        [2., 1., 1.],
        [3., 0., 0.],
        [4., 0., 0.],
        [4., 0., 1.],
        [3., 0., 1.],
    ];
    let polygons: [&[u32]; 3] = [&[0, 3, 4, 1], &[1, 4, 5, 2], &[6, 9, 8, 7]];
    let mut e = scene(&points, &polygons, DVec3::new(2., 0., 1.), 30.);
    let turn = glam::DQuat::from_rotation_y(30f64.to_radians());
    let strip: Vec<DVec3> = points[..6].iter().map(|&p| DVec3::from_array(p)).collect();
    let along = turn * major_axis(&strip);
    let (before, after) = moved(&mut e, DVec3::new(0.25, 0., 0.));
    for i in 0..6 {
        let want = before[i] + along * 0.25;
        assert!((after[i] - want).length() < 1e-6, "strip point {i}");
    }
    for i in 6..10 {
        let want = before[i] + turn * DVec3::X * 0.25;
        assert!((after[i] - want).length() < 1e-6, "square point {i}");
    }
}
