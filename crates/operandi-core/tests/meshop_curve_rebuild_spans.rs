//! Curve Rebuild Spans (`curve.rebuildSpan`, MESHOPS-TIDY TB-30..37): the
//! spec's worked examples on a 3-span curve along X.
use crate::mesh_stack_tools::{add, ok, quad, with_base};
use glam::DVec3;
use operandi_core::curves::{CurveKind, bezier_anchors};
use operandi_core::meshop_selection::{SelKind, SelOp};
use operandi_core::*;
use serde_json::json;

/// An open Curve through (0,0,0) .. (3,0,0).
fn line_curve() -> Mesh {
    let m = Mesh {
        vertices: (0..4).map(|i| DVec3::new(i as f64, 0., 0.)).collect(),
        ..Default::default()
    };
    operandi_core::curve_commands::make_curve(&m, &[0, 1, 2, 3], false)
        .unwrap()
        .0
}
fn setup() -> (Editor, u64, mesh_stack_editor::OpRef) {
    let (mut e, id) = with_base(line_curve());
    let op = add(&mut e, id, "curve.rebuildSpan", json!({}));
    (e, id, op)
}
fn set(e: &mut Editor, op: mesh_stack_editor::OpRef, values: serde_json::Value) {
    let values = values.as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
}
/// The result's one curve: its kind and knots.
fn knots(e: &Editor, id: u64) -> (CurveKind, Vec<DVec3>) {
    let m = ok(e, id);
    assert_eq!(m.faces.len(), 1);
    let (kind, closed) = operandi_core::curve_convert::curve_of(&m, 0).unwrap();
    let points: Vec<DVec3> = m.faces[0]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    let knots = if kind == CurveKind::Bezier {
        bezier_anchors(&points, closed)
            .iter()
            .map(|a| a[1])
            .collect()
    } else {
        points
    };
    (kind, knots)
}
fn xs(k: &[DVec3]) -> Vec<f64> {
    k.iter().map(|p| p.x).collect()
}
fn near(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-6, "{got:?} vs {want:?}");
    }
}
fn thirds() -> Vec<f64> {
    (0..10).map(|i| i as f64 / 3.).collect()
}

#[test]
fn rebuild_spans_splits_each_span_and_makes_a_bezier() {
    let (mut e, id, op) = setup();
    let (kind, k) = knots(&e, id);
    assert_eq!(kind, CurveKind::Bezier);
    near(&xs(&k), &thirds());
    assert!(k.iter().all(|p| p.y.abs() < 1e-9 && p.z.abs() < 1e-9));
    set(&mut e, op, json!({ "breakCount": 2 }));
    near(&xs(&knots(&e, id).1), &[0., 0.5, 1., 1.5, 2., 2.5, 3.]);
    set(&mut e, op, json!({ "breakCount": 1 }));
    let (kind, k) = knots(&e, id);
    assert_eq!(kind, CurveKind::Bezier, "Count 1 still makes a Bezier");
    near(&xs(&k), &[0., 1., 2., 3.]);
}

#[test]
fn rebuild_spans_distance_mode_floors_length_over_distance() {
    let (mut e, id, op) = setup();
    for (d, per_span) in [(0.2, 5), (0.26, 3), (0.3, 3), (0.35, 2), (0.5, 2), (1.0, 1)] {
        set(&mut e, op, json!({ "rebuildMode": 1, "breakDist": d }));
        assert_eq!(knots(&e, id).1.len(), 3 * per_span + 1, "distance {d}");
    }
}

#[test]
fn rebuild_spans_offsets_new_knots() {
    let (mut e, id, op) = setup();
    let ys = |e: &Editor| -> Vec<f64> { knots(e, id).1.iter().map(|p| p.y).collect() };
    let pattern = |new: f64, old: f64| -> Vec<f64> {
        (0..10)
            .map(|i| if i % 3 == 0 { old } else { new })
            .collect()
    };
    set(&mut e, op, json!({ "pushMode": 1, "pushAmount": 0.5 }));
    near(&ys(&e), &pattern(0.5, 0.));
    // Handles move with their knot (Broken Tangents off): the curve
    // stays smooth through it.
    set(&mut e, op, json!({ "apply": 0.5 }));
    near(&ys(&e), &pattern(0.25, 0.));
    set(&mut e, op, json!({ "apply": 1.0, "effectAllPoints": true }));
    near(&ys(&e), &pattern(0.5, 0.5));
    set(
        &mut e,
        op,
        json!({ "effectAllPoints": false, "falloffParam": true }),
    );
    near(&ys(&e), &pattern(1. / 3., 0.));
    // Ease In at t = 1/3: 1 − (1 − 2/3)² = 8/9.
    set(&mut e, op, json!({ "falloffMode": 1 }));
    near(&ys(&e), &pattern(0.5 * 8. / 9., 0.));
    // The vector as typed, not normalised.
    set(
        &mut e,
        op,
        json!({ "falloffParam": false, "pushDirection.Y": 0.0, "pushDirection.Z": 2.0, "pushAmount": 1.0 }),
    );
    let k = knots(&e, id).1;
    assert!((k[1].z - 2.).abs() < 1e-9 && k[0].z.abs() < 1e-9, "{k:?}");
}

#[test]
fn rebuild_spans_snaps_to_an_alignment_mesh() {
    let (mut e, id, op) = setup();
    // A big quad at y = 1, facing +Y.
    let mut big = quad();
    big.vertices
        .iter_mut()
        .for_each(|p| *p = *p * 10. + DVec3::Y);
    e.scene.new_mesh().unwrap();
    let guide = e.scene.items.last().unwrap().id;
    e.scene.item_mut(guide).unwrap().mesh = big.into();
    let ys = |e: &Editor| -> Vec<f64> { knots(e, id).1.iter().map(|p| p.y).collect() };
    let pattern =
        |new: f64| -> Vec<f64> { (0..10).map(|i| if i % 3 == 0 { 0. } else { new }).collect() };
    set(
        &mut e,
        op,
        json!({ "alignmentMesh": guide, "pushRootMode": 1 }),
    );
    near(&ys(&e), &pattern(1.));
    set(&mut e, op, json!({ "searchMaxD": 0.5 }));
    near(&ys(&e), &pattern(0.));
    set(&mut e, op, json!({ "searchMaxD": 0.0, "pushAmount": 0.25 }));
    near(&ys(&e), &pattern(1.25));
    // Cast along Y finds it either way; along X (parallel) finds nothing.
    for (mode, flip, want) in [(2, false, 1.25), (2, true, 1.25), (1, false, 0.)] {
        set(
            &mut e,
            op,
            json!({ "searchMode": mode, "searchVecFlip": flip }),
        );
        near(&ys(&e), &pattern(want));
    }
}

#[test]
fn rebuild_spans_follows_an_edge_selection() {
    let (mut e, id, op) = setup();
    let m = line_curve();
    let middle = m.edges().iter().position(|&e| e == (1, 2)).unwrap();
    e.dispatch(Command::MeshOpSelOp {
        op,
        edit: mesh_stack_editor::SelOpEdit::Add(SelOp::new(SelKind::Index {
            index: middle.to_string(),
        })),
    })
    .unwrap();
    near(&xs(&knots(&e, id).1), &[0., 1., 4. / 3., 5. / 3., 2., 3.]);
}

#[test]
fn rebuild_spans_saves_its_values() {
    let (mut e, id, op) = setup();
    set(
        &mut e,
        op,
        json!({ "reduceMode": 2, "falloffDistance": true }),
    );
    let stack = e.scene.item(id).unwrap().mesh_stack.clone().unwrap();
    let text = serde_json::to_string(&stack).unwrap();
    assert!(
        text.contains("\"pushDirection.Y\"") && text.contains("reduceMode"),
        "{text}"
    );
    let reopened: mesh_stack::MeshStack = serde_json::from_str(&text).unwrap();
    assert_eq!(reopened, stack);
    // TB-37: Span Reduction and Falloff By Distance change nothing.
    near(&xs(&knots(&e, id).1), &thirds());
}
