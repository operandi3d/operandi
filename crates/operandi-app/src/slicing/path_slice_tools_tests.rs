//! Edit tab waves 2 and 3 in the app: the tab's rows, the viewport gestures
//! of Pen, Curve and Primitive Slice and Polygon Axis Slice, typed fields
//! and the name-map tool ids.
use super::tests::{draw, pointer};
use super::*;

/// A 2 x 2 grid of unit quads in XZ, seen from the top.
fn grid_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=2u32 {
        for i in 0..=2u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..2u32 {
        for i in 0..2u32 {
            let v = j * 3 + i;
            mesh.faces
                .push(super::tests::face(vec![v, v + 3, v + 4, v + 1]));
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Top;
    a.frame(false);
    a
}

fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}

fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

fn click(a: &mut Operandi, ctx: &Context, size: Vec2, at: Pos2) {
    draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    draw(a, ctx, size, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, size, vec![pointer(at, false, Modifiers::NONE)]);
}

fn live_params(a: &Operandi) -> Parameters {
    a.editor.session.live.as_ref().unwrap().params.clone()
}

fn live_tool(a: &Operandi) -> Option<Tool> {
    a.editor.session.live.as_ref().map(|l| l.tool)
}

#[test]
fn slice_rows_follow_the_spec_order_and_axis_slice_has_its_section() {
    let ctx = Context::default();
    let size = vec2(1440., 1400.);
    let mut a = grid_app(&ctx);
    a.category = toolbox::Category::Edit;
    draw(&mut a, &ctx, size, vec![]);
    let order = [
        "Polygon Slice",
        "Curve Slice",
        "Edge Slice",
        "Pen Slice",
        "Loop Slice",
        "Primitive Slice",
        "Edit Axis Slice",
        // TC-54: one Axis Slice flyout (Slice, Axis Slice, Pen Slice,
        // Primitive Slice, Dicer); Julienne is Axis Slice itself.
        "flyout:Axis Slice",
    ];
    // Rows by the line they run, the heading by its key.
    let tops: Vec<f32> = order
        .iter()
        .map(|k| {
            a.form_rects
                .get(*k)
                .or_else(|| a.toolbox_rects.get(*k))
                .unwrap_or_else(|| panic!("{k}"))
                .top()
        })
        .collect();
    assert!(tops.windows(2).all(|w| w[0] < w[1]), "{tops:?}");
    for tool in [
        Tool::CurveSlice,
        Tool::PenSlice,
        Tool::PrimitiveSlice,
        Tool::AxisSlice,
    ] {
        let row = a.toolbox_rects[tool.label()].center();
        click(&mut a, &ctx, size, row);
        assert_eq!(live_tool(&a), Some(tool), "{}", a.status);
        draw(&mut a, &ctx, size, vec![]);
        click(&mut a, &ctx, size, row);
        assert_eq!(live_tool(&a), None, "a second click drops it");
    }
    // No separate Julienne (Jason, 2026-11-17); Dicer opens its dialog
    // (cut_dialogs.rs).
    assert!(!a.form_rects.contains_key("operandi.julienne"));
    a.execute("operandi.dicer");
    assert_eq!(live_tool(&a), None);
    assert!(a.cut_dialog.is_some(), "{}", a.status);
}

#[test]
fn pen_slice_clicks_draw_a_closed_path_that_cuts_and_drop_is_one_step() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::PenSlice);
    draw(&mut a, &ctx, size, vec![]);
    for (x, z) in [(0.5, 0.5), (1.5, 0.5), (1.5, 1.5), (0.5, 1.5)] {
        let at = screen(&a, DVec3::new(x, 0., z));
        click(&mut a, &ctx, size, at);
    }
    assert_eq!(live_params(&a).slice.pen_slice.points.len(), 4);
    // The square's corners land inside the four polygons: each is split.
    assert_eq!(faces(&a), 8, "{}", a.status);
    // Undo takes back the last click only.
    a.run(Command::Undo);
    assert_eq!(live_params(&a).slice.pen_slice.points.len(), 3);
    assert_eq!(live_tool(&a), Some(Tool::PenSlice));
    a.run(Command::Redo);
    assert_eq!(faces(&a), 8);
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Pen Slice");
    a.run(Command::Undo);
    assert_eq!(faces(&a), 4);
}

#[test]
fn curve_slice_points_drag_and_remove_last() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::CurveSlice);
    draw(&mut a, &ctx, size, vec![]);
    for (x, z) in [(-0.5, 0.5), (1., 0.8), (2.5, 0.5)] {
        let at = screen(&a, DVec3::new(x, 0., z));
        click(&mut a, &ctx, size, at);
    }
    assert!(faces(&a) > 4, "{}", a.status);
    // Drag the middle point across to the other row.
    let from = screen(&a, DVec3::new(1., 0., 0.8));
    let to = screen(&a, DVec3::new(1., 0., 1.5));
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(to)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    let p = live_params(&a);
    assert_eq!(p.slice.curve_slice.points.len(), 3);
    assert!((p.slice.curve_slice.points[1].z - 1.5).abs() < 0.05);
    a.path_slice_remove_last(Tool::CurveSlice);
    assert_eq!(live_params(&a).slice.curve_slice.points.len(), 2);
}

#[test]
fn primitive_slice_drag_sizes_on_the_face_and_lazy_apply_cuts_on_release() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::PrimitiveSlice);
    draw(&mut a, &ctx, size, vec![]);
    let from = screen(&a, DVec3::new(0.5, 0., 0.5));
    let to = screen(&a, DVec3::new(1.5, 0., 1.5));
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    for k in 1..=4 {
        let at = from + (to - from) * (k as f32 / 4.);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    }
    assert_eq!(faces(&a), 4, "Lazy Apply: nothing cut while dragging");
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    assert_eq!(faces(&a), 8, "{}", a.status);
    let o = live_params(&a).slice.primitive_slice;
    let place = o.place.unwrap();
    assert!((place.center - DVec3::new(1., 0., 1.)).length() < 0.05);
    assert!((o.half_x - 0.5).abs() < 0.05 && (o.half_y - 0.5).abs() < 0.05);
    a.run(Command::Drop);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Primitive Slice"
    );
}

#[test]
fn axis_slice_drag_moves_the_cuts() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::AxisSlice);
    // Numbers start at 0: no cuts (AX-11).
    assert_eq!(faces(&a), 4, "{}", a.status);
    // Number X 1: the cut at x = 1 runs along existing edges.
    a.fields.insert("axis_slice_number0".into(), "1".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(faces(&a), 4);
    draw(&mut a, &ctx, size, vec![]);
    let from = screen(&a, DVec3::new(1.2, 0., 1.2));
    let to = screen(&a, DVec3::new(1.5, 0., 1.2));
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(to)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    let offset = live_params(&a).slice.axis_slice.offset;
    assert!((offset.x - 0.3).abs() < 0.05, "{offset:?}");
    assert_eq!(faces(&a), 6);
}

#[test]
fn typed_slice_fields_reach_the_tool() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    // Regression: Tier 1A fields used to be confirmed without changing the
    // tool.
    a.activate(Tool::PolygonSlice);
    a.fields.insert("polygon_slice_gap".into(), "0.1".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert!((live_params(&a).slice.polygon_slice.gap - 0.1).abs() < 1e-12);
    a.run(Command::Drop);
    a.activate(Tool::CurveSlice);
    a.fields.insert("curve_slice_steps".into(), "12".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(live_params(&a).slice.curve_slice.steps, 12);
    a.fields.insert("curve_slice_steps".into(), "1.5".into());
    assert!(!a.confirm_fields());
    assert_eq!(live_params(&a).slice.curve_slice.steps, 12);
    a.clear_fields();
    a.run(Command::Drop);
    a.activate(Tool::AxisSlice);
    a.fields.insert("axis_slice_number2".into(), "3".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(live_params(&a).slice.axis_slice.number, [0, 0, 3]);
    // Z cuts at 0.5, 1 (along existing edges) and 1.5.
    assert_eq!(faces(&a), 4 + 2 * 2);
}

#[test]
fn name_map_tool_ids_start_the_tools() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    for (id, tool) in [
        ("\"Curve Slice\"", Tool::CurveSlice),
        ("PenKnife", Tool::PenSlice),
        ("\"Primitive Slice\"", Tool::PrimitiveSlice),
        ("poly.julienne", Tool::AxisSlice),
    ] {
        a.execute(&format!("tool.set {id} on"));
        assert_eq!(live_tool(&a), Some(tool), "{id}: {}", a.status);
        a.execute(&format!("tool.set {id} off"));
        assert_eq!(live_tool(&a), None, "{id}: {}", a.status);
    }
}

#[test]
fn axis_slice_number_arrows_and_drag_step_by_one() {
    // Jason's report: Number X/Y/Z didn't step (AX-12, UIC-53).
    let ctx = Context::default();
    let size = vec2(1440., 1400.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::AxisSlice);
    draw(&mut a, &ctx, size, vec![]);
    let field = *a
        .field_rects
        .get("axis_slice_number0")
        .expect("Number X shows in Tool Properties");
    // The arrows sit at the field's right end, just past the text
    // (scrub::split, UIC-53/113).
    let handle_left = field.right() + 2.;
    let up = pos2(handle_left + scrub::HANDLE_WIDTH - 2., field.center().y);
    let down = pos2(handle_left + 2., field.center().y);
    click(&mut a, &ctx, size, up);
    click(&mut a, &ctx, size, up);
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(
        live_params(&a).slice.axis_slice.number,
        [2, 0, 0],
        "{}",
        a.status
    );
    // Two cuts across x 0..2 at 2/3 and 4/3: each splits a column.
    assert_eq!(faces(&a), 8);
    click(&mut a, &ctx, size, down);
    click(&mut a, &ctx, size, down);
    click(&mut a, &ctx, size, down);
    draw(&mut a, &ctx, size, vec![]);
    assert_eq!(
        live_params(&a).slice.axis_slice.number,
        [0, 0, 0],
        "never below 0"
    );
    assert_eq!(faces(&a), 4);
    // Dragging the arrows steps whole numbers too.
    let at = pos2(handle_left + 6., field.center().y);
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, size, vec![pointer(at, true, Modifiers::NONE)]);
    for k in 1..=6 {
        let to = at + vec2(4. * k as f32, 0.);
        draw(&mut a, &ctx, size, vec![Event::PointerMoved(to)]);
    }
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(at + vec2(24., 0.), false, Modifiers::NONE)],
    );
    let n = live_params(&a).slice.axis_slice.number[0];
    assert!(n >= 3, "dragged to {n}");
}
