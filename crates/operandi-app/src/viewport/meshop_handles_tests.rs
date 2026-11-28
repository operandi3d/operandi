//! Handles for the selected primitive MeshOp: they show with the op's row
//! selected and no tool on, and dragging one edits the op live, one undo
//! step per drag.
use super::*;
use crate::viewport::falloff_view::FalloffHandle;
use crate::viewport::meshop_handles::OpHandle;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}
fn cylinder_op() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("prim.cylinder.item".into()));
    a
}
/// A Falloff step on its own (its falloff's points are its handles).
fn falloff_step() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("falloff.meshop.item".into()));
    a
}
fn op_values(a: &Operandi) -> serde_json::Value {
    let (_, v) = a
        .selected_primitive_op()
        .expect("a primitive op or Falloff step is selected");
    v
}

#[test]
fn a_selected_primitive_op_shows_its_handles() {
    let a = cylinder_op();
    let handles = a.meshop_handles(rect());
    let kinds: Vec<OpHandle> = handles.iter().map(|h| h.1).collect();
    assert_eq!(
        kinds,
        [
            OpHandle::Centre,
            OpHandle::Move(0),
            OpHandle::Move(1),
            OpHandle::Move(2),
            OpHandle::Plane(0),
            OpHandle::Plane(1),
            OpHandle::Plane(2),
            OpHandle::Size(0),
            OpHandle::Size(1),
            OpHandle::Size(2)
        ]
    );
    // The Y square sits on top: the cylinder's half height, 0.75 m.
    assert!((handles[8].0 - DVec3::new(0., 0.75, 0.)).length() < 1e-9);
}

#[test]
fn no_handles_while_a_tool_is_on_or_for_other_ops() {
    let mut a = cylinder_op();
    a.activate(Tool::Box);
    assert!(a.editor.live.is_some(), "{}", a.status);
    assert!(a.meshop_handles(rect()).is_empty());
    let mut b = Operandi::empty();
    b.run(Command::NewMesh);
    b.run(Command::MeshOpCreate("pmodel.triangulate.item".into()));
    assert!(b.meshop_handles(rect()).is_empty());
}

#[test]
fn dragging_a_size_square_edits_the_op_live_in_one_undo_step() {
    let mut a = cylinder_op();
    let r = rect();
    let (at, _) = a.meshop_handles(rect())[7];
    let q = a.camera.project(at, r).unwrap().0;
    let grab = a
        .meshop_handle_hit(q, r)
        .expect("the X square is under the pointer");
    assert_eq!(grab.handle, OpHandle::Size(0));
    let history = a.editor.history.undo.len();
    for step in 1..=4 {
        a.drag_meshop_handle(&grab, q, q + vec2(10. * step as f32, 0.), r);
    }
    let size = op_values(&a)["sizeX"].as_f64().unwrap();
    assert!(size > 0.5, "wider: {size}");
    assert_eq!(
        a.editor.history.undo.len(),
        history + 1,
        "one step for the drag"
    );
    a.run(Command::Undo);
    assert_eq!(op_values(&a)["sizeX"].as_f64().unwrap(), 0.5);
}

#[test]
fn a_falloff_steps_points_are_handles() {
    let mut a = falloff_step();
    let (r, _) = a.selected_primitive_op().unwrap();
    let mut values = serde_json::Map::new();
    values.insert(
        "falloff".into(),
        serde_json::json!({"first": {"Linear": {"start": [0., 0.75, 0.], "end": [0., -0.75, 0.]}}}),
    );
    a.run(Command::MeshOpSet { op: r, values });
    let handles = a.meshop_handles(rect());
    // GitHub #27: the tool falloff's handles: Start, End, then three
    // per-axis handles at each.
    assert_eq!(handles.len(), 8, "Start, End, 6 axis handles");
    let (end, h) = handles[1];
    assert_eq!(h, OpHandle::Falloff(FalloffHandle::Point(1)));
    let rr = rect();
    let q = a.camera.project(end, rr).unwrap().0;
    let grab = crate::viewport::meshop_handles::OpGrab {
        op: r,
        handle: h,
        start: op_values(&a),
        gesture: crate::colour_field::next_gesture(),
    };
    a.drag_meshop_handle(&grab, q, q + vec2(0., -60.), rr);
    let f = &op_values(&a)["falloff"]["first"]["Linear"]["end"];
    assert!(f[1].as_f64().unwrap() > -0.75, "End moved up: {f}");
}

/// A Falloff step with a Radial falloff, centred at the origin with Size
/// 1 on every axis.
fn radial_op() -> (Operandi, operandi_core::mesh_stack_editor::OpRef) {
    let mut a = falloff_step();
    let (r, _) = a.selected_primitive_op().unwrap();
    let mut values = serde_json::Map::new();
    values.insert(
        "falloff".into(),
        serde_json::json!({"first": {"Radial": {"center": [0., 0., 0.], "size": [1., 1., 1.]}}}),
    );
    a.run(Command::MeshOpSet { op: r, values });
    (a, r)
}

#[test]
fn an_ops_radial_falloff_has_the_tools_size_boxes_and_axis_handles() {
    // GitHub #27: the same handles as the tool's Radial falloff.
    let (a, _) = radial_op();
    let falloff: Vec<FalloffHandle> = a
        .meshop_handles(rect())
        .into_iter()
        .filter_map(|(_, h)| match h {
            OpHandle::Falloff(f) => Some(f),
            _ => None,
        })
        .collect();
    let tool_like = [
        FalloffHandle::Point(0),
        FalloffHandle::Point(1),
        FalloffHandle::Point(2),
        FalloffHandle::Point(3),
        FalloffHandle::Axis(0, 0),
        FalloffHandle::Axis(0, 1),
        FalloffHandle::Axis(0, 2),
    ];
    assert_eq!(falloff, tool_like);
}

#[test]
fn dragging_an_ops_size_box_resizes_its_falloff_in_one_undo_step() {
    let (mut a, op) = radial_op();
    let rr = rect();
    let (at, h) = a
        .meshop_handles(rr)
        .into_iter()
        .find(|(_, h)| *h == OpHandle::Falloff(FalloffHandle::Point(1)))
        .expect("the X Size box");
    let q = a.camera.project(at, rr).unwrap().0;
    let grab = crate::viewport::meshop_handles::OpGrab {
        op,
        handle: h,
        start: op_values(&a),
        gesture: crate::colour_field::next_gesture(),
    };
    let history = a.editor.history.undo.len();
    for step in 1..=3 {
        a.drag_meshop_handle(&grab, q, q + vec2(15. * step as f32, 0.), rr);
    }
    let size = &op_values(&a)["falloff"]["first"]["Radial"]["size"];
    assert!(size[0].as_f64().unwrap() > 1., "X grew: {size}");
    assert_eq!(size[1].as_f64(), Some(1.), "only X: {size}");
    assert_eq!(
        a.editor.history.undo.len(),
        history + 1,
        "one step for the drag"
    );
    a.run(Command::Undo);
    let size = &op_values(&a)["falloff"]["first"]["Radial"]["size"];
    assert_eq!(size[0].as_f64(), Some(1.));
}

fn handle_at(a: &Operandi, h: OpHandle) -> Pos2 {
    let (p, _) = a
        .meshop_handles(rect())
        .into_iter()
        .find(|x| x.1 == h)
        .unwrap();
    a.camera.project(p, rect()).unwrap().0
}

#[test]
fn a_move_arrow_moves_the_op_along_its_axis_only() {
    let mut a = cylinder_op();
    let r = rect();
    let tip = handle_at(&a, OpHandle::Move(0));
    let grab = a.meshop_handle_hit(tip, r).expect("the X arrow");
    assert_eq!(grab.handle, OpHandle::Move(0));
    let history = a.editor.history.undo.len();
    for step in 1..=3 {
        a.drag_meshop_handle(&grab, tip, tip + vec2(15. * step as f32, -20.), r);
    }
    let v = op_values(&a);
    assert!(v["cenX"].as_f64().unwrap().abs() > 1e-6, "moved along X");
    assert_eq!(v["cenY"].as_f64().unwrap(), 0.);
    assert_eq!(v["cenZ"].as_f64().unwrap(), 0.);
    assert_eq!(
        a.editor.history.undo.len(),
        history + 1,
        "one step for the drag"
    );
}

#[test]
fn a_plane_square_moves_the_op_across_its_plane() {
    let mut a = cylinder_op();
    let r = rect();
    let at = handle_at(&a, OpHandle::Plane(1));
    let grab = a.meshop_handle_hit(at, r).expect("the XZ square");
    assert_eq!(grab.handle, OpHandle::Plane(1));
    a.drag_meshop_handle(&grab, at, at + vec2(30., -30.), r);
    let v = op_values(&a);
    assert_eq!(v["cenY"].as_f64().unwrap(), 0., "stays in the Y plane");
}

#[test]
fn a_size_square_on_an_arrow_shaft_still_wins() {
    let a = cylinder_op();
    let q = handle_at(&a, OpHandle::Size(0));
    assert_eq!(
        a.meshop_handle_hit(q, rect()).unwrap().handle,
        OpHandle::Size(0)
    );
}

#[test]
fn dragging_a_lattice_point_moves_it_in_one_undo_step() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("deform.lattice.item".into()));
    let rr = rect();
    let handles = a.meshop_handles(rr);
    assert_eq!(handles.len(), 27, "3 x 3 x 3 control points");
    let at = handle_at(&a, OpHandle::Lattice(26));
    let grab = a.meshop_handle_hit(at, rr).expect("the corner point");
    assert_eq!(grab.handle, OpHandle::Lattice(26));
    let history = a.editor.history.undo.len();
    for step in 1..=3 {
        a.drag_meshop_handle(&grab, at, at + vec2(0., -20. * step as f32), rr);
    }
    let offsets = &op_values(&a)["offsets"];
    assert_eq!(offsets.as_array().unwrap().len(), 27);
    assert!(offsets[26][1].as_f64().unwrap() > 0., "moved up: {offsets}");
    assert_eq!(
        a.editor.history.undo.len(),
        history + 1,
        "one step for the drag"
    );
}

/// #42: the Arc and Text ops had no handles at all. They get the centre,
/// move arrows and plane squares (their sizes are single numbers, so no
/// per-axis size squares).
#[test]
fn arc_and_text_ops_have_move_handles() {
    for id in ["prim.arc.item", "prim.text.item"] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.run(Command::MeshOpCreate(id.into()));
        let kinds: Vec<OpHandle> = a.meshop_handles(rect()).iter().map(|h| h.1).collect();
        assert_eq!(
            kinds,
            [
                OpHandle::Centre,
                OpHandle::Move(0),
                OpHandle::Move(1),
                OpHandle::Move(2),
                OpHandle::Plane(0),
                OpHandle::Plane(1),
                OpHandle::Plane(2),
            ],
            "{id}"
        );
    }
}

/// #42: dragging the Arc op's X arrow moves the whole arc: its Center and
/// the Start and End points that set its direction, so its shape stays.
#[test]
fn moving_the_arc_op_carries_start_and_end_along() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("prim.arc.item".into()));
    let before = op_values(&a);
    let r = rect();
    let (op, _) = a.selected_primitive_op().unwrap();
    let grab = crate::viewport::meshop_handles::OpGrab {
        op,
        handle: OpHandle::Move(0),
        start: before.clone(),
        gesture: crate::colour_field::next_gesture(),
    };
    let q = r.center();
    a.drag_meshop_handle(&grab, q, q + vec2(80., 0.), r);
    let after = op_values(&a);
    let moved = |k: &str| after[k].as_f64().unwrap() - before[k].as_f64().unwrap();
    let dx = moved("centerX");
    assert!(dx.abs() > 1e-6, "the centre moved along X");
    for k in ["startX", "endX"] {
        assert!((moved(k) - dx).abs() < 1e-9, "{k} moved with it");
    }
    for k in ["centerY", "centerZ", "startY", "startZ", "endY", "endZ"] {
        assert!(moved(k).abs() < 1e-9, "{k} stays");
    }
}

/// #42: dragging the Text op's Y arrow moves its Position.
#[test]
fn moving_the_text_op_sets_its_position() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::MeshOpCreate("prim.text.item".into()));
    let before = op_values(&a);
    let r = rect();
    let (op, _) = a.selected_primitive_op().unwrap();
    let grab = crate::viewport::meshop_handles::OpGrab {
        op,
        handle: OpHandle::Move(1),
        start: before.clone(),
        gesture: crate::colour_field::next_gesture(),
    };
    let q = r.center();
    a.drag_meshop_handle(&grab, q, q + vec2(0., -80.), r);
    let after = op_values(&a);
    let dy = after["posY"].as_f64().unwrap() - before["posY"].as_f64().unwrap();
    assert!(dy > 1e-6, "Position Y went up: {dy}");
}
