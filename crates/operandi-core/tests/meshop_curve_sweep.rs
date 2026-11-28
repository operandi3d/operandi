//! Curve Sweep as a Mesh Stack op (`curve.sweep`).
use crate::mesh_stack_tools::{add, curve_squares, ok, with_base};
use glam::DVec3;
use operandi_core::*;
use serde_json::json;

/// An open line along X, 2 m long.
fn line() -> Mesh {
    let m = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X * 2.],
        ..Default::default()
    };
    let mut m = curve_commands::make_curve(&m, &[0, 1], false).unwrap().0;
    m.attributes.representations[0] = PolygonRepresentation::Polyline {
        closed: false,
        curve: CurveKind::Line,
    };
    m
}

#[test]
fn curve_sweep_puts_a_tube_along_a_line() {
    let (mut e, id) = with_base(line());
    add(&mut e, id, "curve.sweep", json!({}));
    let m = ok(&e, id);
    // The line stays; a pen path has one section per point: one ring of 8.
    assert!(m.is_polyline(0));
    assert_eq!(m.faces.len(), 1 + 8);
    for f in 1..m.faces.len() {
        for &v in &m.faces[f].vertices {
            let p = m.vertices[v as usize];
            assert!((DVec3::new(0., p.y, p.z).length() - 0.125).abs() < 1e-9);
        }
    }
    let (mut e, id) = with_base(line());
    add(&mut e, id, "curve.sweep", json!({ "capMode": 3 }));
    assert_eq!(ok(&e, id).faces.len(), 1 + 8 + 2);
}

#[test]
fn curve_sweep_follows_curves_by_steps_and_index() {
    let base = curve_squares(&[DVec3::ZERO, DVec3::X * 3.], false);
    let (mut e, id) = with_base(base.clone());
    add(&mut e, id, "curve.sweep", json!({ "steps": 6 }));
    assert_eq!(ok(&e, id).faces.len(), 2 + 2 * 6 * 8);
    let (mut e, id) = with_base(base);
    add(
        &mut e,
        id,
        "curve.sweep",
        json!({ "steps": 6, "index": "1" }),
    );
    let m = ok(&e, id);
    assert_eq!(m.faces.len(), 2 + 6 * 8);
    assert!(m.vertices[m.faces[2].vertices[0] as usize].x > 2.);
}

#[test]
fn curve_sweep_off_or_on_polygons_passes_through() {
    let (mut e, id) = with_base(line());
    add(&mut e, id, "curve.sweep", json!({ "makePolys": false }));
    assert_eq!(ok(&e, id), line());
    let (mut e, id) = with_base(crate::mesh_stack_tools::cube());
    add(&mut e, id, "curve.sweep", json!({}));
    assert_eq!(ok(&e, id), crate::mesh_stack_tools::cube());
}

/// The angles round the X axis of the swept points at `x`, sorted.
fn ring_angles(m: &Mesh, x: f64) -> Vec<f64> {
    let mut a: Vec<f64> = m
        .vertices
        .iter()
        .filter(|p| (p.x - x).abs() < 1e-9 && DVec3::new(0., p.y, p.z).length() > 1e-6)
        .map(|p| p.z.atan2(p.y).rem_euclid(std::f64::consts::TAU))
        .collect();
    a.sort_by(f64::total_cmp);
    a
}

/// #51: Rotation turns every ring, the first one too, so the tube rolls as
/// a whole (with Flip as well).
#[test]
fn curve_sweep_rotation_turns_every_ring() {
    for flip in [false, true] {
        let (mut e, id) = with_base(line());
        add(&mut e, id, "curve.sweep", json!({ "flip": flip }));
        let plain = ok(&e, id);
        let (mut e, id) = with_base(line());
        add(
            &mut e,
            id,
            "curve.sweep",
            json!({ "flip": flip, "rotate": std::f64::consts::PI / 8. }),
        );
        let turned = ok(&e, id);
        for x in [0., 2.] {
            let (before, after) = (ring_angles(&plain, x), ring_angles(&turned, x));
            assert_eq!(before.len(), 8, "flip {flip}, x {x}");
            assert_eq!(after.len(), 8, "flip {flip}, x {x}");
            // An eighth of a turn's half: no point stays where it was.
            for b in &before {
                assert!(
                    after.iter().all(|a| (a - b).abs() > 0.1),
                    "flip {flip}: the ring at x {x} didn't turn: {before:?} {after:?}"
                );
            }
        }
    }
}

/// #51: a closed curve sweeps into a closed tube: the last ring is the
/// first (no doubled points) and there are no end caps, whatever End Caps
/// says.
#[test]
fn curve_sweep_closes_round_a_closed_curve() {
    let square = [
        DVec3::ZERO,
        DVec3::X * 2.,
        DVec3::new(2., 0., 2.),
        DVec3::Z * 2.,
    ];
    for cap_mode in [0, 3] {
        let m = Mesh {
            vertices: square.to_vec(),
            ..Default::default()
        };
        let mut m = curve_commands::make_curve(&m, &[0, 1, 2, 3], true)
            .unwrap()
            .0;
        m.attributes.representations[0] = PolygonRepresentation::Polyline {
            closed: true,
            curve: CurveKind::Line,
        };
        let (mut e, id) = with_base(m);
        add(&mut e, id, "curve.sweep", json!({ "capMode": cap_mode }));
        let out = ok(&e, id);
        // Four rings of 8, one quad per side per span: 4 × 8 quads.
        assert_eq!(out.faces.len(), 1 + 4 * 8, "caps {cap_mode}");
        let swept: Vec<DVec3> = out.vertices[4..].to_vec();
        assert_eq!(swept.len(), 4 * 8, "no doubled ring");
        for f in 1..out.faces.len() {
            assert_eq!(out.faces[f].vertices.len(), 4, "only quads, no caps");
        }
    }
}
