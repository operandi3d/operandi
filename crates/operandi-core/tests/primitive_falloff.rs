//! GitHub #15: the global falloff shapes the Cube, Sphere and Cylinder while
//! the tool is live. Turning the falloff on, switching it, moving it and
//! turning it off all keep the tool open and update the preview; Drop keeps
//! the shaped mesh in one undo step, and the primitive MeshOp builds the same
//! mesh from its copy of the falloff.
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e
}
fn live_cylinder(e: &mut Editor) {
    e.dispatch(Command::Activate(Tool::Cylinder)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.radial = Some(RadialParameters {
        radius: DVec3::new(0.5, 0.75, 0.5),
        sides: 12,
        segments: 6,
    });
    p.make_uvs = false;
    p.shape_falloff.scale = 0.;
    e.dispatch(Command::Preview(p)).unwrap();
}
fn top_radius(e: &Editor) -> f64 {
    let mesh = &e.scene.active().unwrap().mesh;
    mesh.vertices
        .iter()
        .filter(|v| (v.y - 0.75).abs() < 1e-9)
        .map(|v| v.x.hypot(v.z))
        .fold(0., f64::max)
}
fn ramp(top: f64) -> Falloff {
    Falloff::Linear(LinearFalloff {
        start: DVec3::new(0., top, 0.),
        end: DVec3::new(0., -0.75, 0.),
        ..Default::default()
    })
}

#[test]
fn the_falloff_can_be_turned_on_moved_and_off_without_dropping_the_tool() {
    let mut e = editor();
    live_cylinder(&mut e);
    assert!((top_radius(&e) - 0.5).abs() < 1e-9, "no falloff: unshaped");

    e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
        .unwrap();
    assert!(e.session.live.is_some(), "the tool stays live");
    assert!(top_radius(&e) < 1e-9, "Scale 0% at the full end: a point");

    // Moving the falloff's Start (its handle) re-shapes the live preview.
    e.dispatch(Command::SpatialFalloff(Some(ramp(1.5))))
        .unwrap();
    // The top is two thirds of the way from End to Start.
    assert!(
        (top_radius(&e) - 0.5 / 3.).abs() < 1e-9,
        "two-thirds weight at the top"
    );

    // Switching type keeps the tool too.
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Radial(
        RadialFalloff {
            center: DVec3::new(0., 0.75, 0.),
            size: DVec3::splat(10.),
            ..Default::default()
        },
    ))))
    .unwrap();
    assert!(e.session.live.is_some());

    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    assert!(e.session.live.is_some());
    assert!((top_radius(&e) - 0.5).abs() < 1e-9, "off again: unshaped");

    // Back to the primitive's own values, then on again and Drop.
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.radial.as_mut().unwrap().sides = 16;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
        .unwrap();
    let shown = e.scene.active().unwrap().mesh.clone();
    let history = e.history.undo.len();
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.scene.active().unwrap().mesh.vertices, shown.vertices);
    assert_eq!(e.history.undo.len(), history + 1, "one undo step");
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.faces.is_empty());
}

#[test]
fn cube_and_sphere_follow_the_falloff_too() {
    for tool in [Tool::Box, Tool::Sphere] {
        let mut e = editor();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.size = DVec3::splat(1.5);
        p.shape_falloff.offset = DVec3::new(1., 0., 0.);
        e.dispatch(Command::Preview(p)).unwrap();
        let plain = e.scene.active().unwrap().mesh.vertices.clone();
        e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
            .unwrap();
        let shaped = &e.scene.active().unwrap().mesh.vertices;
        assert_ne!(&plain, shaped, "{tool:?}");
        assert!(e.session.live.is_some(), "{tool:?}");
    }
}

#[test]
fn falloffs_that_need_existing_points_leave_the_shape_alone_and_say_why() {
    let mut e = editor();
    live_cylinder(&mut e);
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Selection(
        SelectionFalloff::default(),
    ))))
    .unwrap();
    assert!(e.session.live.is_some());
    assert!((top_radius(&e) - 0.5).abs() < 1e-9);
    assert!(e.primitive_falloff_note().unwrap().contains("Selection"));
}

#[test]
fn fit_falloff_to_shape_ramps_from_base_to_top() {
    let mut e = editor();
    live_cylinder(&mut e);
    let f = e.primitive_fit_falloff().unwrap();
    e.dispatch(Command::SpatialFalloff(Some(f))).unwrap();
    assert!(top_radius(&e) < 1e-9, "full at the top");
}

#[test]
fn the_cylinder_op_follows_the_falloff_step_below_and_matches_the_tool() {
    use crate::mesh_stack_tools::{add, ok, with_base};
    let (mut e, id) = with_base(Mesh::default());
    e.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
        .unwrap();
    // The Falloff step takes a copy of the active falloff.
    let op = add(&mut e, id, "falloff.meshop.item", serde_json::json!({}));
    add(
        &mut e,
        id,
        "prim.cylinder.item",
        serde_json::json!({"falloffScale": 0, "sides": 12, "segments": 6, "makeUVs": false}),
    );
    let op_mesh = ok(&e, id);

    let mut t = editor();
    t.dispatch(Command::SpatialFalloff(Some(ramp(0.75))))
        .unwrap();
    live_cylinder(&mut t);
    assert_eq!(op_mesh.vertices, t.scene.active().unwrap().mesh.vertices);

    // Clear the step's copy: the shape is round again.
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    let mut values = serde_json::Map::new();
    values.insert("falloff".into(), serde_json::Value::Null);
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    assert_ne!(ok(&e, id).vertices, op_mesh.vertices);
}

#[test]
fn an_old_primitive_ops_own_falloff_copy_is_read_and_ignored() {
    use operandi_core::meshop_types::{OpContext, op_def};
    let def = op_def("prim.cylinder.item").unwrap();
    let run = |values: &serde_json::Value| {
        let ctx = OpContext {
            input: &Mesh::default(),
            op: 1,
            selection: None,
            element: Default::default(),
            links: &[],
            time: Default::default(),
            notes: Default::default(),
            falloff: None,
        };
        def.run(&ctx, values).unwrap()
    };
    let plain = def.defaults();
    let mut old = plain.clone();
    old["falloff"] = serde_json::json!({ "first": serde_json::to_value(ramp(0.75)).unwrap() });
    assert_eq!(run(&old).vertices, run(&plain).vertices);
}
