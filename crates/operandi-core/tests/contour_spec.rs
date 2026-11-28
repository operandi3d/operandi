//! Contour against specifications/missing-tools-g4 (G5-CONTOUR v1, CT-1,
//! CT-4..8; Q-CT-1 decided 2026-11-21: Lines and Face space like Curve).
//! Background: Sphere Item (globe, 24 sides × 12 segments, radius 0.5) at
//! the origin; the active mesh is empty; line (−1,0,0)→(1,0,0), direction
//! (0,0,1) unless noted.
use glam::DVec3;
use operandi_core::contour::{
    ContourKind, ContourParameters, MISSES, Piece, SlicingSide, contour_pieces,
};
use operandi_core::*;

fn scene() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::NewMesh,
    })
    .unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    e
}

fn params() -> ContourParameters {
    ContourParameters {
        start: DVec3::new(-1., 0., 0.),
        end: DVec3::new(1., 0., 0.),
        direction: DVec3::Z,
        placed: true,
        ..Default::default()
    }
}

fn one(e: &Editor, p: &ContourParameters) -> Piece {
    let pieces = contour_pieces(&e.scene, p).unwrap();
    assert_eq!(pieces.len(), 1, "{pieces:?}");
    pieces.into_iter().next().unwrap()
}

/// The angle of `q` round the Y axis, degrees, in the XZ plane.
fn angle(q: DVec3) -> f64 {
    q.z.atan2(q.x).to_degrees()
}

/// Consecutive angle steps of a closed loop, all about `step` degrees.
fn even_angles(points: &[DVec3], step: f64, tol: f64) {
    let n = points.len();
    for i in 0..n {
        let d = (angle(points[(i + 1) % n]) - angle(points[i])).rem_euclid(360.);
        let d = d.min(360. - d);
        assert!((d - step).abs() < tol, "step {i}: {d}° not {step}°");
    }
}

/// On the sphere's faceted surface: between the polygon's inner radius
/// and the sphere's.
fn on_sphere(q: DVec3, ring: f64) {
    let r = DVec3::new(q.x, 0., q.z).length();
    let inner = ring * 7.5f64.to_radians().cos();
    assert!(r <= ring + 1e-9 && r >= inner - 1e-9, "{q}: {r}");
}

#[test]
fn ct_1_defaults_a_closed_curve_of_ten_points_on_the_equator() {
    let e = scene();
    let c = one(&e, &params());
    assert!(c.closed);
    assert_eq!(c.points.len(), 10);
    for &q in &c.points {
        assert!(q.y.abs() < 1e-12);
        on_sphere(q, 0.5);
    }
    even_angles(&c.points, 36., 1.);
    // MT13-11: the loop starts at the cut point nearest the line's start.
    assert!(c.points[0].distance(DVec3::new(-0.5, 0., 0.)) < 1e-9);
}

#[test]
fn ct_4_automatic_takes_the_crossed_edges_plus_one() {
    let e = scene();
    let c = one(
        &e,
        &ContourParameters {
            automatic: true,
            ..params()
        },
    );
    assert!(c.closed);
    assert_eq!(c.points.len(), 25);
    even_angles(&c.points, 14.4, 0.6);
}

#[test]
fn ct_5_uniform_span_off_steps_through_the_cut_points() {
    let e = scene();
    let c = one(
        &e,
        &ContourParameters {
            segments: 4,
            uniform: false,
            ..params()
        },
    );
    assert_eq!(c.points.len(), 4);
    even_angles(&c.points, 90., 1e-6);
    for &q in &c.points {
        assert!((q.length() - 0.5).abs() < 1e-9, "on the sphere's points");
    }
}

#[test]
fn ct_6_a_raised_line_cuts_a_smaller_ring() {
    let e = scene();
    let c = one(
        &e,
        &ContourParameters {
            start: DVec3::new(-1., 0.25, 0.),
            end: DVec3::new(1., 0.25, 0.),
            ..params()
        },
    );
    assert!(c.closed);
    assert_eq!(c.points.len(), 10);
    for &q in &c.points {
        assert!((q.y - 0.25).abs() < 1e-12);
        on_sphere(q, 0.5 * 30f64.to_radians().cos());
    }
}

#[test]
fn ct_7_a_short_line_keeps_the_open_piece_facing_the_viewer() {
    let e = scene();
    let c = one(
        &e,
        &ContourParameters {
            start: DVec3::new(-0.1, 0., 0.),
            end: DVec3::new(0.1, 0., 0.),
            ..params()
        },
    );
    assert!(!c.closed);
    assert_eq!(c.points.len(), 11);
    let (first, last) = (c.points[0], c.points[10]);
    assert!(
        first.distance(DVec3::new(0.1, 0., 0.4868)) < 2e-4,
        "{first}"
    );
    assert!(last.distance(DVec3::new(-0.1, 0., 0.4868)) < 2e-4, "{last}");
    // Even spacing by length along the cut.
    let steps: Vec<f64> = c.points.windows(2).map(|w| w[0].distance(w[1])).collect();
    let mean = steps.iter().sum::<f64>() / steps.len() as f64;
    assert!(steps.iter().all(|s| (s - mean).abs() < 2e-3), "{steps:?}");
}

#[test]
fn ct_8_positive_keeps_the_plus_z_half() {
    let e = scene();
    let c = one(
        &e,
        &ContourParameters {
            side: SlicingSide::Positive,
            ..params()
        },
    );
    assert!(!c.closed);
    assert_eq!(c.points.len(), 11);
    assert!(c.points.iter().all(|q| q.z >= -1e-9), "{:?}", c.points);
    // Negative keeps the other half.
    let n = one(
        &e,
        &ContourParameters {
            side: SlicingSide::Negative,
            ..params()
        },
    );
    assert!(n.points.iter().all(|q| q.z <= 1e-9));
}

#[test]
fn mt13_12_a_line_over_nothing_makes_nothing() {
    let e = scene();
    let err = contour_pieces(
        &e.scene,
        &ContourParameters {
            start: DVec3::new(-1., 3., 0.),
            end: DVec3::new(1., 3., 0.),
            ..params()
        },
    )
    .unwrap_err();
    assert_eq!(err, MISSES);
}

/// Runs the tool with `p` and drops it.
fn run(e: &mut Editor, p: ContourParameters) -> std::result::Result<(), String> {
    e.dispatch(Command::Activate(Tool::Contour))?;
    let mut params = e.session.live.as_ref().unwrap().params.clone();
    *params.contour = p;
    e.dispatch(Command::Preview(params))?;
    e.dispatch(Command::Drop)
}

#[test]
fn ct_1_curve_lines_and_face_add_one_polygon_each_with_ten_points() {
    for (kind, polyline) in [
        (ContourKind::Curve, true),
        (ContourKind::Lines, true),
        (ContourKind::Face, false),
    ] {
        let mut e = scene();
        run(&mut e, ContourParameters { kind, ..params() }).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(m.faces.len(), 1, "{kind:?}");
        assert_eq!(m.faces[0].vertices.len(), 10, "{kind:?}: ten points");
        assert_eq!(m.is_polyline(0), polyline, "{kind:?}");
        if polyline {
            // A closed line or curve: its last point joins its first.
            assert!(matches!(
                m.attributes.representations[0],
                PolygonRepresentation::Polyline { closed: true, .. }
            ));
        }
        // MT13-10: the new polygon is selected; the background untouched.
        assert_eq!(e.scene.selection.faces, vec![0]);
        assert_eq!(e.scene.items[0].mesh.faces.len(), 24 * 12);
    }
}

#[test]
fn the_tool_is_one_undo_step_and_cancel_leaves_nothing() {
    let mut e = scene();
    let steps = e.history.undo.len();
    run(&mut e, params()).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
    // Cancel: nothing is added.
    e.dispatch(Command::Activate(Tool::Contour)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    *p.contour = params();
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 1, "previewed");
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
    // A miss is refused with the message and changes nothing.
    let mut e = scene();
    let err = run(
        &mut e,
        ContourParameters {
            start: DVec3::new(-1., 3., 0.),
            end: DVec3::new(1., 3., 0.),
            ..params()
        },
    )
    .unwrap_err();
    assert!(err.contains(MISSES), "{err}");
}
