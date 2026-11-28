//! Tool pipe Falloff step in the Mesh Stack (batch 7): steps above it
//! that move points move them only by the falloff's weight.
use crate::mesh_stack_tools::{add, cube, ok, with_base};
use glam::DVec3;
use operandi_core::mesh_stack::OpStatus;
use operandi_core::*;
use serde_json::json;

/// Full at the cube's top (y = 0.5), nothing at its bottom (y = -0.5).
fn ramp() -> Falloff {
    Falloff::Linear(LinearFalloff {
        start: DVec3::new(0., 0.5, 0.),
        end: DVec3::new(0., -0.5, 0.),
        ..Default::default()
    })
}
/// A step 0.3 grid moves every cube point from ±0.5 to ±0.6.
fn grow() -> serde_json::Value {
    json!({ "stepX": 0.3, "stepY": 0.3, "stepZ": 0.3 })
}

#[test]
fn falloff_shapes_the_point_moves_above_it() {
    let c = cube();
    let (mut e, id) = with_base(c.clone());
    e.dispatch(Command::SpatialFalloff(Some(ramp()))).unwrap();
    add(&mut e, id, "falloff.meshop.item", json!({}));
    // The step keeps its own copy: turning the active falloff off later
    // changes nothing.
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    add(&mut e, id, "xfrm.quantize.item", grow());
    let m = ok(&e, id);
    for (p, was) in m.vertices.iter().zip(&c.vertices) {
        if was.y > 0. {
            assert!(p.distance(*was * 1.2) < 1e-9, "full weight: snapped, {p}");
        } else {
            assert_eq!(p, was, "zero weight: stays");
        }
    }
}

#[test]
fn falloff_only_reaches_up_and_no_layers_sets_nothing() {
    let c = cube();
    let all_snapped = |m: &Mesh| {
        m.vertices
            .iter()
            .zip(&c.vertices)
            .all(|(p, was)| p.distance(*was * 1.2) < 1e-9)
    };
    // Below the step it moves: full strength.
    let (mut e, id) = with_base(c.clone());
    e.dispatch(Command::SpatialFalloff(Some(ramp()))).unwrap();
    add(&mut e, id, "xfrm.quantize.item", grow());
    add(&mut e, id, "falloff.meshop.item", json!({}));
    assert!(all_snapped(&ok(&e, id)));
    // A Falloff step with no layers: full strength, mesh passes through.
    let (mut e, id) = with_base(c.clone());
    add(&mut e, id, "falloff.meshop.item", json!({}));
    assert_eq!(ok(&e, id), c);
    add(&mut e, id, "xfrm.quantize.item", grow());
    assert!(all_snapped(&ok(&e, id)));
}

#[test]
fn steps_that_add_points_are_not_shaped_and_say_so() {
    let (mut e, id) = with_base(cube());
    e.dispatch(Command::SpatialFalloff(Some(ramp()))).unwrap();
    add(&mut e, id, "falloff.meshop.item", json!({}));
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    add(&mut e, id, "prim.teapot.item", json!({}));
    let r = e
        .mesh_stack
        .current(&e.scene, e.scene.item(id).unwrap())
        .unwrap();
    let OpStatus::Noted(notes) = &r.statuses[1] else {
        panic!("{:?}", r.statuses)
    };
    assert!(notes[0].text.contains("isn't shaped"));
    assert!(r.mesh.vertices.len() > cube().vertices.len());
}

/// #43: Polygon Bevel and Smooth Shift take the Falloff below too (their
/// tools do): on the cube, the top's bevel shifts by the full amount, the
/// bottom's by none, and the step isn't marked as unshaped.
#[test]
fn bevel_and_smooth_shift_follow_the_falloff_below() {
    for (id, values) in [
        ("poly.bevel.item", json!({ "shift": 0.2, "inset": 0.0 })),
        ("poly.smshift.item", json!({ "shift": 0.2 })),
    ] {
        let c = cube();
        let (mut e, item) = with_base(c.clone());
        e.dispatch(Command::SpatialFalloff(Some(ramp()))).unwrap();
        add(&mut e, item, "falloff.meshop.item", json!({}));
        add(&mut e, item, id, values.clone());
        let m = ok(&e, item);
        let top = m.vertices.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        let bottom = m.vertices.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        assert!(
            (top - 0.7).abs() < 1e-6,
            "{id}: the top shifts in full: {top}"
        );
        assert!(
            (bottom + 0.5).abs() < 1e-6,
            "{id}: the bottom doesn't move: {bottom}"
        );
        let r = e
            .mesh_stack
            .current(&e.scene, e.scene.item(item).unwrap())
            .unwrap();
        assert!(
            !format!("{:?}", r.statuses[1]).contains("isn't shaped"),
            "{id}: {:?}",
            r.statuses[1]
        );
        // Without the falloff both ends move.
        let (mut e, item) = with_base(c.clone());
        add(&mut e, item, id, values);
        let plain = ok(&e, item);
        let bottom = plain.vertices.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        assert!(
            (bottom + 0.7).abs() < 1e-6,
            "{id}: full shift without it: {bottom}"
        );
    }
}
