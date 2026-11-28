//! Edit tab and the Slice tools' viewport gestures (toolbox-panels-2026-11-06).
use super::tests::{draw, key, pointer};
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

fn click(a: &mut Operandi, ctx: &Context, size: Vec2, at: Pos2, mods: Modifiers) {
    draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    draw(a, ctx, size, vec![pointer(at, true, mods)]);
    draw(a, ctx, size, vec![pointer(at, false, mods)]);
}

fn drag(a: &mut Operandi, ctx: &Context, size: Vec2, from: Pos2, to: Pos2) {
    draw(a, ctx, size, vec![Event::PointerMoved(from)]);
    draw(a, ctx, size, vec![pointer(from, true, Modifiers::NONE)]);
    for k in 1..=4 {
        let at = from + (to - from) * (k as f32 / 4.);
        draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    }
    draw(a, ctx, size, vec![pointer(to, false, Modifiers::NONE)]);
}

fn live_tool(a: &Operandi) -> Option<Tool> {
    a.editor.session.live.as_ref().map(|l| l.tool)
}

#[test]
fn edit_tab_sits_before_vertex_and_its_buttons_start_the_tools() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    let y = |a: &Operandi, label: &str| a.toolbox_rects[&format!("Category {label}")].top();
    assert!(y(&a, "Select") < y(&a, "Edit") && y(&a, "Edit") < y(&a, "Vertex"));
    a.category = toolbox::Category::Edit;
    draw(&mut a, &ctx, size, vec![]);
    let rows: Vec<f32> = ["Polygon Slice", "Edge Slice", "Loop Slice"]
        .iter()
        .map(|k| a.toolbox_rects[*k].top())
        .collect();
    assert!(rows[0] < rows[1] && rows[1] < rows[2], "spec order");
    for tool in [Tool::PolygonSlice, Tool::EdgeSlice, Tool::LoopSlice] {
        let row = a.toolbox_rects[tool.label()].center();
        click(&mut a, &ctx, size, row, Modifiers::NONE);
        assert_eq!(live_tool(&a), Some(tool), "{}", a.status);
        // Its Tool Properties show.
        draw(&mut a, &ctx, size, vec![]);
        click(&mut a, &ctx, size, row, Modifiers::NONE);
        assert_eq!(live_tool(&a), None, "a second click drops it");
    }
}

#[test]
fn slice_shortcuts_are_bound_without_conflicts() {
    let bindings = input::default_bindings();
    assert_eq!(bindings["Edge Slice"], "G");
    assert_eq!(bindings["Loop Slice"], "Alt+C");
    assert_eq!(bindings["Polygon Slice"], "Shift+C");
    assert!(input::bindings_valid(&bindings));
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    let middle = a.viewport_rect.center();
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(middle)]);
    draw(&mut a, &ctx, size, vec![key(Key::G)]);
    assert_eq!(live_tool(&a), Some(Tool::EdgeSlice), "{}", a.status);
}

#[test]
fn loop_slice_bar_drag_moves_the_slice_as_one_undo_step() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    a.run(Command::Mode(Mode::Edge));
    // Edge (0,0)-(1,0): its ring is the column x = 0..1.
    a.editor.scene.selection.edges = vec![(0, 1)];
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::LoopSlice);
    assert_eq!(faces(&a), 6, "{}", a.status);
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    // The bar: 200 points long at (20, 50); the slice's triangle at 50 %.
    let triangle = r.left_top() + vec2(20. + 100., 50. + 4.);
    drag(&mut a, &ctx, size, triangle, triangle + vec2(50., 0.));
    let o = &a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .slice
        .loop_slice;
    assert!((o.positions[0] - 0.75).abs() < 1e-6, "{:?}", o.positions);
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(
        m.vertices
            .iter()
            .any(|v| v.distance(DVec3::new(0.75, 0., 2.)) < 1e-6)
    );
    // One tweak for the whole drag: Undo goes back to 50 %, tool still on.
    a.run(Command::Undo);
    let o = &a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .slice
        .loop_slice;
    assert!((o.positions[0] - 0.5).abs() < 1e-9);
    assert_eq!(live_tool(&a), Some(Tool::LoopSlice));
    // A click on another edge picks its ring instead.
    let row = screen(&a, DVec3::new(0., 0., 1.5));
    click(&mut a, &ctx, size, row, Modifiers::NONE);
    let rings = &a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .slice
        .loop_slice
        .rings;
    assert_eq!(rings, &vec![(3, 6)], "{}", a.status);
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Loop Slice");
}

#[test]
fn edge_slice_clicks_cut_and_undo_takes_back_one_click() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::EdgeSlice);
    draw(&mut a, &ctx, size, vec![]);
    // Ctrl places the cut at the middle.
    let first = screen(&a, DVec3::new(0.3, 0., 0.));
    click(&mut a, &ctx, size, first, Modifiers::CTRL);
    let second = screen(&a, DVec3::new(0.5, 0., 1.));
    click(&mut a, &ctx, size, second, Modifiers::CTRL);
    assert_eq!(faces(&a), 5, "{}", a.status);
    let third = screen(&a, DVec3::new(0.5, 0., 2.));
    click(&mut a, &ctx, size, third, Modifiers::CTRL);
    assert_eq!(faces(&a), 6, "{}", a.status);
    a.run(Command::Undo);
    assert_eq!(faces(&a), 5);
    assert_eq!(live_tool(&a), Some(Tool::EdgeSlice));
    draw(&mut a, &ctx, size, vec![key(Key::Q)]);
    assert_eq!(live_tool(&a), None);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Edge Slice");
    a.run(Command::Undo);
    assert_eq!(faces(&a), 4);
}

#[test]
fn polygon_slice_drag_cuts_along_the_line() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::PolygonSlice);
    draw(&mut a, &ctx, size, vec![]);
    let from = screen(&a, DVec3::new(-0.5, 0., 0.5));
    let to = screen(&a, DVec3::new(2.5, 0., 0.5));
    drag(&mut a, &ctx, size, from, to);
    assert_eq!(faces(&a), 6, "{}", a.status);
    // Shift+drag adds a second cut, keeping the first.
    let from = screen(&a, DVec3::new(-0.5, 0., 1.5));
    let to = screen(&a, DVec3::new(2.5, 0., 1.5));
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(from, true, Modifiers::SHIFT)],
    );
    draw(&mut a, &ctx, size, vec![Event::PointerMoved(to)]);
    draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(to, false, Modifiers::SHIFT)],
    );
    assert_eq!(faces(&a), 8, "{}", a.status);
    a.run(Command::Cancel);
    assert_eq!(faces(&a), 4);
}

/// Loop Slice's greying rules (spec loop-slice-2026-11-15, section 4
/// table): which rows are greyed for a given set of options.
fn loop_slice_greyed(
    a: &Operandi,
    change: impl FnOnce(&mut operandi_core::LoopSlice),
) -> Vec<String> {
    let mut p = operandi_core::Parameters::default();
    change(&mut p.slice.loop_slice);
    let mut f = crate::tool_form::ToolForm::new();
    a.slice_form(Tool::LoopSlice, &p, &mut f);
    f.fields()
        .iter()
        .filter(|f| f.disabled_tip.is_some())
        .map(|f| f.row.label.clone())
        .collect()
}

#[test]
fn loop_slice_current_stops_at_the_last_slice_instead_of_refusing() {
    let ctx = Context::default();
    let a = grid_app(&ctx);
    let mut p = operandi_core::Parameters::default();
    let mut f = crate::tool_form::ToolForm::new();
    a.slice_form(Tool::LoopSlice, &p, &mut f);
    let fields = f.fields();
    let current = fields.iter().find(|f| f.row.label == "Current").unwrap();
    let crate::tool_form::Bind::Number { set, .. } = &current.bind else {
        panic!("Current is a number field");
    };
    // One slice: nudging Current up used to say "Current must be a whole
    // number from 0 to 0" (Jason, 2026-11-20).
    assert_eq!(set(&mut p, 1.), Ok(()));
    assert_eq!(p.slice.loop_slice.current, 0);
    p.slice.loop_slice.set_count(3);
    assert_eq!(set(&mut p, 7.), Ok(()));
    assert_eq!(p.slice.loop_slice.current, 2);
    assert!(set(&mut p, f64::NAN).is_err());
}

#[test]
fn loop_slice_options_grey_by_the_spec_table() {
    let ctx = Context::default();
    let a = grid_app(&ctx);
    // Starting values: Keep Quads needs Slice Selected, Tension needs
    // Preserve Curvature, the profile switches need a profile, Cap
    // Sections and Gap need Split. Split and Profile are never greyed.
    assert_eq!(
        loop_slice_greyed(&a, |_| {}),
        [
            "Keep Quads",
            "Reverse Direction",
            "Reverse Inset",
            "Keep Aspect",
            "Tension",
            "Cap Sections",
            "Gap"
        ]
    );
    assert!(!loop_slice_greyed(&a, |o| o.slice_selected = true).contains(&"Keep Quads".into()));
    let split = loop_slice_greyed(&a, |o| o.split = true);
    assert!(!split.contains(&"Cap Sections".into()) && !split.contains(&"Gap".into()));
    // A profile lights up its switches and, with Keep Aspect on, greys
    // Inset; Keep Aspect off gives Inset back.
    let profile = loop_slice_greyed(&a, |o| {
        o.profile.shape = Some(operandi_core::ProfileShape::TSlot)
    });
    // LS-Q13 (Jason, 2026-11-16): the profile sets the loops, so Current,
    // Count and Position grey too (and the Uniform button, below).
    assert_eq!(
        profile,
        [
            "Current",
            "Count",
            "Position",
            "Keep Quads",
            "Tension",
            "Inset",
            "Cap Sections",
            "Gap"
        ]
    );
    let by_hand = loop_slice_greyed(&a, |o| {
        o.profile.shape = Some(operandi_core::ProfileShape::TSlot);
        o.profile.keep_aspect = false;
    });
    assert!(!by_hand.contains(&"Inset".into()));
    // Every greyed row says why (UIC-117).
    let mut p = operandi_core::Parameters::default();
    p.slice.loop_slice.profile.shape = Some(operandi_core::ProfileShape::VGroove);
    let mut f = crate::tool_form::ToolForm::new();
    a.slice_form(Tool::LoopSlice, &p, &mut f);
    assert!(
        f.fields()
            .iter()
            .filter_map(|f| f.disabled_tip.as_deref())
            .all(|why| !why.is_empty())
    );
}

#[test]
fn loop_slice_profile_list_is_none_then_operandis_own_shapes() {
    let labels: Vec<&str> = crate::edit_panel::profile_options()
        .iter()
        .map(|o| o.1)
        .collect();
    assert_eq!(
        labels,
        [
            "None",
            "Round Groove",
            "V Groove",
            "Square Slot",
            "T Slot",
            "Saw Notch"
        ]
    );
}

#[test]
fn loop_slice_uniform_button_greys_while_a_profile_sets_the_loops() {
    let ctx = Context::default();
    let a = grid_app(&ctx);
    let uniform_enabled = |shape: Option<operandi_core::ProfileShape>| {
        let mut p = operandi_core::Parameters::default();
        p.slice.loop_slice.profile.shape = shape;
        let mut f = crate::tool_form::ToolForm::new();
        a.slice_form(Tool::LoopSlice, &p, &mut f);
        f.items
            .iter()
            .find_map(|item| match item {
                crate::tool_form::Item::Buttons(b) => {
                    b.iter().find(|b| b.label == "Uniform").map(|b| b.enabled)
                }
                _ => None,
            })
            .expect("Uniform button")
    };
    assert!(uniform_enabled(None));
    assert!(!uniform_enabled(Some(
        operandi_core::ProfileShape::RoundGroove
    )));
}
