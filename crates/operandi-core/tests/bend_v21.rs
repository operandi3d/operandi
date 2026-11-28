//! Bend spec v2.1 section 5 (bend-handles-2026-11-15, measured): acceptance
//! cases BD-01..05 through the editor on a strip along +Y from 0 to 2 m,
//! plus BN-43 (sign), BN-44 (spine off the world axes), BN-45 (axis along
//! the spine, BH-Q12 provisional) and BN-47 (values remembered, Reset).
//! Original test data.
use glam::{DMat3, DVec3};
use operandi_core::*;

/// 1.273 m: the radius of a 2 m spine bent 90° (2 / (π/2)).
const R: f64 = 4. / std::f64::consts::PI;
const EPS: f64 = 1e-9;

/// 10 segments, 0.2 m wide across X, y from `from` to 2.
fn strip_from(from: f64) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let rows = 10;
    let step = (2. - from) / rows as f64;
    let mut vertices = Vec::new();
    for i in 0..=rows {
        let y = from + step * i as f64;
        vertices.push(DVec3::new(-0.1, y, 0.));
        vertices.push(DVec3::new(0.1, y, 0.));
    }
    let faces = (0..rows as u32)
        .map(|i| Face {
            vertices: vec![2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces,
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.history.undo.clear();
    e
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn live(e: &Editor) -> BendOptions {
    e.session.live.as_ref().unwrap().params.bend
}
/// Start Bend with the centre at `centre`, spine (0, 2, 0) and `axis`, then
/// set Angle.
fn bend(e: &mut Editor, centre: DVec3, axis: usize, angle: f64, both: bool) {
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    e.dispatch(Command::SetActionCenter(centre)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.bend.spine = DVec3::new(0., 2., 0.);
    p.bend.axis = axis;
    p.bend.both_sides = both;
    p.bend.angle = angle;
    e.dispatch(Command::Preview(p)).unwrap();
}
/// The centre line's point at row `i` (between the strip's two columns),
/// and the direction the strip runs there (row i to row i + 1).
fn mid(p: &[DVec3], i: usize) -> DVec3 {
    (p[2 * i] + p[2 * i + 1]) / 2.
}

/// BD-01: 90° on Axis Z: the top lands at (1.273, 1.273, 0) pointing +X;
/// the centre line is a quarter circle of radius 1.273 m; the spine keeps
/// its length and the width across the strip is kept (BN-40).
#[test]
fn bd01_quarter_turn() {
    let mut e = strip_from(0.);
    bend(&mut e, DVec3::ZERO, 2, 90., false);
    let p = points(&e);
    let centre = DVec3::new(R, 0., 0.);
    for i in 0..=10 {
        let m = mid(&p, i);
        assert!(((m - centre).length() - R).abs() < EPS, "row {i}: {m}");
        // Width across the strip (along the axis... here X turns with the
        // cross-section, so measure the pair's distance).
        assert!(((p[2 * i] - p[2 * i + 1]).length() - 0.2).abs() < EPS);
        // A cross-section at s turns by Angle·s/L.
        let s = 0.2 * i as f64;
        let turned = (p[2 * i + 1] - p[2 * i]).normalize();
        let want = (90f64 * s / 2.).to_radians();
        assert!((turned.dot(DVec3::X) - want.cos()).abs() < 1e-9, "row {i}");
    }
    let top = mid(&p, 10);
    assert!((top - DVec3::new(R, R, 0.)).length() < EPS, "{top}");
    assert!((R - 1.273).abs() < 1e-3);
    // Pointing along +X: a point just past the end carries on that way.
    let o = live(&e);
    let end = o.bend_point(DVec3::new(0., 2., 0.), o.direction(DMat3::IDENTITY));
    let past = o.bend_point(DVec3::new(0., 2.1, 0.), o.direction(DMat3::IDENTITY));
    assert!(
        (past - end - DVec3::new(0.1, 0., 0.)).length() < EPS,
        "{past}"
    );
}

/// BD-02: 180°: the top end is at (1.273, 0, 0), pointing down (-Y).
#[test]
fn bd02_half_turn() {
    let mut e = strip_from(0.);
    bend(&mut e, DVec3::ZERO, 2, 180., false);
    let p = points(&e);
    let top = mid(&p, 10);
    assert!((top - DVec3::new(R, 0., 0.)).length() < EPS, "{top}");
    // The end's tangent: a point just past it carries straight on (BN-41).
    let o = live(&e);
    let end = o.bend_point(DVec3::new(0., 2., 0.), o.direction(DMat3::IDENTITY));
    let past = o.bend_point(DVec3::new(0., 2.1, 0.), o.direction(DMat3::IDENTITY));
    assert!(
        (past - end - DVec3::new(0., -0.1, 0.)).length() < EPS,
        "{past}"
    );
    // Radius 2/π now: the half circle's centre is at (2/π, 0, 0).
    let r = 2. / std::f64::consts::PI;
    for i in 0..=10 {
        let m = mid(&p, i);
        assert!(((m - DVec3::new(r, 0., 0.)).length() - r).abs() < EPS);
    }
}

/// BD-03 / BN-42: points at y < 0 don't move with Both Sides off; with it
/// on they are the exact mirror of the y > 0 side.
#[test]
fn bd03_behind_the_centre() {
    let before = points(&strip_from(-2.));
    let mut e = strip_from(-2.);
    bend(&mut e, DVec3::ZERO, 2, 90., false);
    let p = points(&e);
    for v in 0..10 {
        assert_eq!(p[v], before[v], "point {v} behind the centre moved");
    }
    assert_ne!(p[21], before[21]);
    let mut e = strip_from(-2.);
    bend(&mut e, DVec3::ZERO, 2, 90., true);
    let p = points(&e);
    for i in 0..5 {
        let up = p[2 * (5 + i)];
        let down = p[2 * (5 - i)];
        assert!(
            (down - DVec3::new(up.x, -up.y, up.z)).length() < EPS,
            "row {i}: {up} vs {down}"
        );
    }
    // The far ends are the mirror of BD-01's top: same lean, same amount.
    assert!((mid(&p, 10) - DVec3::new(R, R, 0.)).length() < EPS);
    assert!((mid(&p, 0) - DVec3::new(R, -R, 0.)).length() < EPS);
}

/// BD-04 / BN-43: Axis X: the same curve in the YZ plane, leaning -Z.
#[test]
fn bd04_axis_x_leans_minus_z() {
    let mut e = strip_from(0.);
    bend(&mut e, DVec3::ZERO, 0, 90., false);
    let p = points(&e);
    let top = mid(&p, 10);
    assert!((top - DVec3::new(0., R, -R)).length() < EPS, "{top}");
    for i in 0..=10 {
        let m = mid(&p, i);
        assert!(m.x.abs() < EPS);
        assert!(((m - DVec3::new(0., 0., -R)).length() - R).abs() < EPS);
    }
    // Across the strip (along X, the bend axis) the width is kept as is.
    for i in 0..=10 {
        assert!((p[2 * i + 1] - p[2 * i] - DVec3::new(0.2, 0., 0.)).length() < EPS);
    }
}

/// BD-05: centre at (0, 1, 0): nothing below y = 1 moves.
#[test]
fn bd05_centre_up_the_strip() {
    let before = points(&strip_from(0.));
    let mut e = strip_from(0.);
    bend(&mut e, DVec3::new(0., 1., 0.), 2, 90., false);
    let p = points(&e);
    for (v, (a, b)) in before.iter().zip(&p).enumerate() {
        if a.y <= 1. + EPS {
            assert!((*a - *b).length() < EPS, "point {v} at {a} moved to {b}");
        }
    }
    // Spine 2 m from y = 1: the strip's top (s = 1) is half way round.
    let half = (45f64).to_radians();
    let want = DVec3::new(R * (1. - half.cos()), 1. + R * half.sin(), 0.);
    assert!((mid(&p, 10) - want).length() < EPS, "{}", mid(&p, 10));
}

/// BN-43: in right-hand terms the bend turns by -Angle about the axis.
#[test]
fn bn43_turns_minus_angle_about_the_axis() {
    for axis in [0, 2] {
        let o = BendOptions {
            angle: 30.,
            spine: DVec3::new(0., 2., 0.),
            axis,
            ..Default::default()
        };
        let i = DMat3::IDENTITY;
        let past = o.bend_point(DVec3::new(0., 3., 0.), o.direction(i));
        let end = o.bend_point(o.spine, o.direction(i));
        let tangent = (past - end).normalize();
        let a = o.turn_axis(i);
        let want = glam::DQuat::from_axis_angle(a, -30f64.to_radians()) * DVec3::Y;
        assert!((tangent - want).length() < EPS, "axis {axis}: {tangent}");
    }
}

/// BN-41 / BN-44: a spine along (1, 1, 0) bends measured along the spine;
/// the bent region ends at s = √2 and points past it go on rigidly.
#[test]
fn bn44_spine_off_the_world_axes() {
    let o = BendOptions {
        angle: 90.,
        spine: DVec3::new(1., 1., 0.),
        axis: 2,
        ..Default::default()
    };
    let i = DMat3::IDENTITY;
    let d = o.direction(i);
    let u = DVec3::new(1., 1., 0.).normalize();
    let l = 2f64.sqrt();
    let end = o.bend_point(u * l, d);
    let past = o.bend_point(u * (l + 0.5), d);
    // Past the end: straight on along the end's tangent, 0.5 m further.
    let tangent = glam::DQuat::from_axis_angle(DVec3::Z, -90f64.to_radians()) * u;
    assert!((past - end - tangent * 0.5).length() < EPS, "{past}");
    // The end sits on a circle of radius √2 / (π/2) round base + d·r.
    let r = l / std::f64::consts::FRAC_PI_2;
    assert!(((end - d * r).length() - r).abs() < EPS);
}

/// BN-45 (BH-Q12, provisional): Axis Y with the spine up Y moves nothing,
/// and the editor says why.
#[test]
fn bn45_axis_along_the_spine_moves_nothing() {
    let before = points(&strip_from(0.));
    let mut e = strip_from(0.);
    bend(&mut e, DVec3::ZERO, 1, 90., false);
    assert_eq!(points(&e), before);
    assert_eq!(
        e.bend_notice(),
        Some("The bend axis runs along the spine; pick another axis")
    );
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.bend.axis = 2;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_ne!(points(&e), before);
    assert_eq!(e.bend_notice(), None);
}

/// BN-47: Angle, Spine and axis come back on the next use; Reset gives
/// Angle 0 and Spine (0, 1 m, 0) and keeps the centre.
#[test]
fn bn47_values_remembered_and_reset() {
    let mut e = strip_from(0.);
    bend(&mut e, DVec3::ZERO, 0, 60., true);
    let used = live(&e);
    e.dispatch(Command::Drop).unwrap();
    let bent = points(&e);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let again = live(&e);
    assert_eq!(again.angle, 60.);
    assert_eq!(again.spine, used.spine);
    assert_eq!(again.axis, 0);
    assert!(again.both_sides);
    assert_ne!(points(&e), bent, "the remembered Angle shows at once");
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), bent);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let centre = live(&e).base;
    e.dispatch(Command::ResetBend).unwrap();
    let reset = live(&e);
    assert_eq!(reset.angle, 0.);
    assert_eq!(reset.spine, DVec3::Y);
    assert_eq!(reset.base, centre);
    assert_eq!(points(&e), bent);
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.dispatch(Command::ResetBend).is_err());
}
