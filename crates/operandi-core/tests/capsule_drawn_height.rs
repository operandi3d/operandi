//! Drawing a capsule's height (Jason, 2026-11-06): the rounded ends grow
//! first and the straight sides only appear once the height passes twice
//! the base radius. Before the fix the drag clamped End Size to its previous
//! value every frame, so the ends collapsed and the capsule came out as a
//! cylinder.
use glam::DVec3;
use operandi_core::*;

const EPS: f64 = 1e-9;

/// A base drawn on the XZ plane (axis Y) with radius `r`, then given
/// half-height `half`.
fn drawn(r: f64, half: f64) -> CapsuleParameters {
    let mut c = CapsuleParameters {
        position: DVec3::new(1., 2., 3.),
        radius: DVec3::new(r, 0., r),
        // A stale End Size from an earlier session must not matter.
        end_size: 0.001,
        ..Default::default()
    };
    c.set_drawn_height(half);
    c
}

fn local(c: &CapsuleParameters) -> Vec<DVec3> {
    let mesh = capsule_geometry(c).unwrap();
    mesh.vertices.iter().map(|v| *v - c.position).collect()
}

fn radial(v: DVec3) -> f64 {
    (v.x * v.x + v.z * v.z).sqrt()
}

#[test]
fn a_short_drag_is_rounded_with_no_straight_sides() {
    let (r, half) = (0.5, 0.2); // total height 0.4 < 2r = 1.0
    let c = drawn(r, half);
    assert_eq!(c.end_size, half, "ends squashed to fit the height");
    let mesh = capsule_geometry(&c).unwrap();
    let points = local(&c);
    // Its extent along the axis is exactly the dragged height.
    let top = points.iter().map(|v| v.y.abs()).fold(0., f64::max);
    assert!((top - half).abs() < EPS, "{top}");
    // Top and bottom are single pole points on the axis.
    for sign in [1., -1.] {
        let poles: Vec<_> = points
            .iter()
            .filter(|v| (v.y - sign * half).abs() < EPS)
            .collect();
        assert_eq!(poles.len(), 1, "one pole at {sign}");
        assert!(radial(*poles[0]) < EPS);
    }
    // No flat cap polygons: every polygon touching a pole is a triangle.
    for f in &mesh.faces {
        let at_pole = f
            .vertices
            .iter()
            .any(|&i| (points[i as usize].y.abs() - half).abs() < EPS);
        if at_pole {
            assert_eq!(f.vertices.len(), 3, "pole fan, not a flat cap");
        }
    }
    // No straight sides: only the middle ring reaches the full radius, and
    // every point lies on the squashed ellipsoid.
    for v in &points {
        if radial(*v) > r - EPS {
            assert!(v.y.abs() < EPS, "full radius off the middle: {v}");
        }
        let e = (radial(*v) / r).powi(2) + (v.y / half).powi(2);
        assert!((e - 1.).abs() < 1e-9, "{v} off the ellipsoid: {e}");
    }
}

#[test]
fn a_tall_drag_has_hemisphere_ends_and_straight_sides() {
    let (r, half) = (0.5, 2.); // total height 4 > 2r
    let c = drawn(r, half);
    assert_eq!(c.end_size, r, "full hemispheres");
    let body = half - r;
    let points = local(&c);
    let top = points.iter().map(|v| v.y.abs()).fold(0., f64::max);
    assert!((top - half).abs() < EPS);
    for v in &points {
        if v.y.abs() > body + EPS {
            // End points lie on a hemisphere of the base radius.
            let centre = DVec3::new(0., body * v.y.signum(), 0.);
            assert!(((*v - centre).length() - r).abs() < 1e-9, "{v}");
        } else {
            // Side points sit on the cylinder of the base radius.
            assert!((radial(*v) - r).abs() < 1e-9, "{v}");
        }
    }
    // The straight section spans height - 2r.
    let side_ys: Vec<f64> = points
        .iter()
        .filter(|v| (radial(**v) - r).abs() < 1e-9)
        .map(|v| v.y)
        .collect();
    let lo = side_ys.iter().copied().fold(f64::MAX, f64::min);
    let hi = side_ys.iter().copied().fold(f64::MIN, f64::max);
    assert!((hi - lo - (2. * half - 2. * r)).abs() < 1e-9, "{lo}..{hi}");
}

#[test]
fn ends_grow_back_after_dragging_down_and_up() {
    let mut c = drawn(0.5, 0.05);
    assert_eq!(c.end_size, 0.05);
    c.set_drawn_height(3.);
    assert_eq!(c.end_size, 0.5, "End Size doesn't ratchet down");
}

#[test]
fn drawn_height_matches_the_unit_capsule() {
    let unit = CapsuleParameters::default();
    let mut c = CapsuleParameters {
        radius: DVec3::new(unit.radius.x, 0., unit.radius.z),
        ..unit.clone()
    };
    c.set_drawn_height(unit.radius.y);
    assert_eq!(c, unit);
    assert_eq!(
        capsule_geometry(&c).unwrap(),
        capsule_geometry(&unit).unwrap()
    );
}
