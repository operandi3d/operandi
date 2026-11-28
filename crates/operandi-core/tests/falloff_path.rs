//! Falloff batch 6 (falloff spec 2026-11-15): Path (FO-53) and Curvature
//! (FO-51). Original test data; the Curvature formula is FO-Q07's
//! provisional reading.
use glam::DVec3;
use operandi_core::falloff_path::{self as fpath};
use operandi_core::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A 5 × 5 point grid in XZ (1 apart, y = 0), quads facing +Y; `lift`
/// moves the middle point up (a bump) or down (a dip).
fn grid(lift: f64) -> Mesh {
    let mut vertices = Vec::new();
    for z in 0..5 {
        for x in 0..5 {
            let y = if (x, z) == (2, 2) { lift } else { 0. };
            vertices.push(DVec3::new(x as f64, y, z as f64));
        }
    }
    let mut faces = Vec::new();
    for z in 0..4u32 {
        for x in 0..4u32 {
            let a = z * 5 + x;
            faces.push(face(vec![a, a + 5, a + 6, a + 1]));
        }
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

/// FO-53: the curve runs through every control point; weights are
/// Smooth(d / Size) around it; no points, no weight.
#[test]
fn path_tube_around_the_curve() {
    let p = PathFalloff::default();
    assert_eq!((p.size, p.world), (1., false));
    let controls = [DVec3::ZERO, DVec3::new(2., 0., 0.), DVec3::new(4., 0., 2.)];
    let curve = fpath::path_curve(&controls);
    for c in controls {
        assert!(curve.iter().any(|q| q.distance(c) < 1e-9));
    }
    let mesh = grid(0.);
    let w = fpath::path_weights(&mesh, glam::DMat4::IDENTITY, p, &controls[..2]);
    // Points on the straight path (z = 0, x ≤ 2) weigh 1; z = 0.5 away…
    assert_eq!(w[0], 1.);
    assert_eq!(w[2], 1.);
    // …z = 1 is Size away: 0.
    assert_eq!(w[5], 0.);
    let half = fpath::path_weights(
        &mesh,
        glam::DMat4::IDENTITY,
        PathFalloff { size: 2., ..p },
        &controls[..2],
    );
    assert!(close(half[5], FalloffShape::Smooth.weight(0.5)));
    assert!(
        fpath::path_weights(&mesh, glam::DMat4::IDENTITY, p, &[])
            .iter()
            .all(|&w| w == 0.)
    );
}

/// FO-51: flat weighs 0.5, a bump more, a dip less; a negative Scale swaps
/// them; measured default Scale 1.
#[test]
fn curvature_tells_bumps_from_dips() {
    assert_eq!(CurvatureFalloff::default().scale, 1.);
    let c = CurvatureFalloff::default();
    let flat = fpath::curvature_weights(&grid(0.), c);
    assert!(close(flat[12], 0.5));
    let bump = fpath::curvature_weights(&grid(0.3), c);
    let dip = fpath::curvature_weights(&grid(-0.3), c);
    assert!(bump[12] > 0.5 && dip[12] < 0.5, "{} {}", bump[12], dip[12]);
    let swapped = fpath::curvature_weights(&grid(0.3), CurvatureFalloff { scale: -1. });
    assert!(swapped[12] < 0.5);
}

/// The editor's Path points drive a Move, and Cancel restores.
#[test]
fn path_points_drive_a_move() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = grid(0.).into();
    e.scene.selection.mode = Mode::Vertex;
    let before = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Path(
        PathFalloff::default(),
    ))))
    .unwrap();
    e.dispatch(Command::FalloffPath(vec![
        DVec3::ZERO,
        DVec3::new(4., 0., 0.),
    ]))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..p
    }))
    .unwrap();
    let after = e.scene.active().unwrap().mesh.vertices.clone();
    // The z = 0 row lifts, the rest stays.
    for (v, (a, b)) in after.iter().zip(&before).enumerate() {
        let want = if v < 5 { 1. } else { 0. };
        assert!(close(a.y - b.y, want), "point {v}");
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, before);
    assert!(e.dispatch(Command::FalloffPath(vec![DVec3::NAN])).is_err());
}

#[test]
fn commands_take_their_options() {
    let parsed = |line: &str| match command_line::parse_sculpt(line) {
        Some(command_line::SculptLine::Falloff { falloff, .. }) => falloff,
        other => panic!("{line}: {other:?}"),
    };
    assert_eq!(
        parsed("falloff.path size:0.5"),
        Falloff::Path(PathFalloff {
            size: 0.5,
            ..Default::default()
        })
    );
    assert_eq!(
        parsed("falloff.curvature scale:-2"),
        Falloff::Curvature(CurvatureFalloff { scale: -2. })
    );
}
