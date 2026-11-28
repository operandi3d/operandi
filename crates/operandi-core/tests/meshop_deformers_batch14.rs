//! Mesh Stack deformers, batch 14: Wrap. Points stick to spots on a
//! Target mesh (saved with the step) and follow it; Rebind measures again.
use crate::mesh_stack_tools::{add, cube, with_base};
use glam::DVec3;
use operandi_core::mesh_stack::{MeshStack, OpStatus};
use operandi_core::meshop_wrap::bind;
use operandi_core::*;
use serde_json::json;

/// A cube to wrap (half size) inside a unit cube target, and the step.
fn setup() -> (Editor, u64, u64, mesh_stack_editor::OpRef) {
    let small = cube_geometry(DVec3::ZERO, DVec3::splat(0.5)).unwrap();
    let (mut e, id) = with_base(small);
    e.scene.new_mesh().unwrap();
    let target = e.scene.items.last().unwrap().id;
    e.scene.item_mut(target).unwrap().mesh = cube().into();
    let op = add(&mut e, id, "deform.wrap.item", json!({ "target": target }));
    (e, id, target, op)
}
fn rebind(e: &mut Editor, op: mesh_stack_editor::OpRef) {
    let binding = bind(&e.scene, op.item, op.op).unwrap();
    let values = json!({ "binding": binding }).as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
}
fn result(e: &Editor, id: u64) -> (Mesh, Vec<OpStatus>) {
    let r = e
        .mesh_stack
        .current(&e.scene, e.scene.item(id).unwrap())
        .unwrap();
    ((*r.mesh).clone(), r.statuses)
}
fn noted(statuses: &[OpStatus], what: &str) -> bool {
    matches!(statuses.last(), Some(OpStatus::Noted(n)) if n[0].text.contains(what))
}

#[test]
fn wrap_unbound_or_without_target_passes_through() {
    let (mut e, id, _, op) = setup();
    let base = (*e.scene.item(id).unwrap().mesh).clone();
    let (m, s) = result(&e, id);
    assert_eq!(m, base);
    assert!(noted(&s, "Rebind"), "{s:?}");
    let values = json!({ "target": 0 }).as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    assert!(
        bind(&e.scene, id, op.op).is_err(),
        "no Target: Rebind refuses"
    );
    assert_eq!(result(&e, id).0, base);
}

#[test]
fn wrap_binding_holds_points_still_then_follows_the_target() {
    let (mut e, id, target, op) = setup();
    let base = (*e.scene.item(id).unwrap().mesh).clone();
    rebind(&mut e, op);
    let (m, s) = result(&e, id);
    assert_eq!(s.last(), Some(&OpStatus::Ok));
    for (p, was) in m.vertices.iter().zip(&base.vertices) {
        assert!(
            p.distance(*was) < 1e-9,
            "binding moves nothing: {p} vs {was}"
        );
    }
    // Move the target: every point moves with it.
    e.scene.item_mut(target).unwrap().transform.translation = DVec3::new(2., 0., 0.);
    let m = result(&e, id).0;
    for (p, was) in m.vertices.iter().zip(&base.vertices) {
        assert!(p.distance(*was + DVec3::X * 2.) < 1e-9, "{p}");
    }
    // Reshape the target (twice the size): every point keeps its 0.25
    // depth under the target's surface.
    e.scene.item_mut(target).unwrap().transform.translation = DVec3::ZERO;
    let mut big = cube();
    big.vertices.iter_mut().for_each(|p| *p *= 2.);
    e.scene.item_mut(target).unwrap().mesh = big.into();
    let m = result(&e, id).0;
    for p in &m.vertices {
        let depth = (0..3).map(|a| 1. - p[a].abs()).fold(f64::MAX, f64::min);
        assert!((depth - 0.25).abs() < 1e-9, "{p}");
    }
}

#[test]
fn wrap_strength_limits_the_move() {
    let (mut e, id, target, op) = setup();
    let base = (*e.scene.item(id).unwrap().mesh).clone();
    rebind(&mut e, op);
    let values = json!({ "strength": 0.5 }).as_object().unwrap().clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    e.scene.item_mut(target).unwrap().transform.translation = DVec3::new(0., 1., 0.);
    let m = result(&e, id).0;
    for (p, was) in m.vertices.iter().zip(&base.vertices) {
        assert!(
            p.distance(*was + DVec3::Y * 0.5) < 1e-9,
            "half strength: {p}"
        );
    }
}

#[test]
fn wrap_asks_for_rebind_when_things_change_under_it() {
    let (mut e, id, target, op) = setup();
    rebind(&mut e, op);
    // The target loses polygons: the saved spots are gone.
    let mut fewer = cube();
    fewer.faces.truncate(1);
    e.scene.item_mut(target).unwrap().mesh = fewer.into();
    assert!(noted(&result(&e, id).1, "polygons changed"));
    // Rebind fixes it.
    rebind(&mut e, op);
    assert_eq!(result(&e, id).1.last(), Some(&OpStatus::Ok));
    // The mesh below gains points.
    e.scene.item_mut(id).unwrap().mesh = cube().into();
    let more = {
        let mut m = cube();
        m.vertices.push(DVec3::ZERO);
        m
    };
    e.scene.item_mut(id).unwrap().mesh = more.into();
    assert!(noted(&result(&e, id).1, "changed its points"));
}

#[test]
fn wrap_binding_is_saved_with_the_step_and_undoes() {
    let (mut e, id, target, op) = setup();
    rebind(&mut e, op);
    let stack = e.scene.item(id).unwrap().mesh_stack.clone().unwrap();
    let reopened: MeshStack =
        serde_json::from_str(&serde_json::to_string(&stack).unwrap()).unwrap();
    assert_eq!(reopened, stack, "the binding survives save and reopen");
    e.scene.item_mut(target).unwrap().transform.translation = DVec3::Z;
    let followed = result(&e, id).0;
    e.dispatch(Command::Undo).unwrap();
    let values = &e.scene.item(id).unwrap().mesh_stack.as_ref().unwrap().ops[0]
        .kind
        .values();
    assert_eq!(
        values["binding"],
        json!([]),
        "undo takes the binding back off"
    );
    assert_ne!(result(&e, id).0, followed);
}

/// Scripts can press Rebind: `deformer.rebind` binds exactly as the button
/// does, in one undo step, and refuses ops that aren't a Wrap.
#[test]
fn wrap_rebind_command_binds_like_the_button() {
    let (mut e, id, _, op) = setup();
    let line = format!("deformer.rebind deformer:op:{}:{}", op.item, op.op);
    command_line::run_line(&mut e, &line)
        .expect("headless")
        .unwrap();
    let values = e.scene.item(id).unwrap().mesh_stack.as_ref().unwrap().ops[0]
        .kind
        .values();
    assert_eq!(values["binding"], bind(&e.scene, op.item, op.op).unwrap());
    assert_eq!(result(&e, id).1.last(), Some(&OpStatus::Ok));
    e.dispatch(Command::Undo).unwrap();
    let values = e.scene.item(id).unwrap().mesh_stack.as_ref().unwrap().ops[0]
        .kind
        .values();
    assert_eq!(values["binding"], json!([]), "one undo step");
    // Not a Wrap: refused, nothing changes.
    let twist = add(&mut e, id, "deform.twist.item", json!({}));
    let line = format!("deformer.rebind deformer:op:{}:{}", twist.item, twist.op);
    assert!(
        command_line::run_line(&mut e, &line)
            .expect("headless")
            .is_err()
    );
}
