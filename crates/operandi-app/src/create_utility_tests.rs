//! Create panel Edit, Merge, Align and Measure groups driven through the
//! toolbox buttons and viewport clicks with synthetic egui events.
use super::*;
use operandi_core::Mesh;

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 2000.))),
            focused: true,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keep_utility_exclusive();
            a.keyboard(ctx);
        },
    );
}
fn pointer(p: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: p,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Primary,
    }
}
fn click(a: &mut Operandi, ctx: &Context, p: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(p)]);
    frame(a, ctx, vec![pointer(p, true), pointer(p, false)]);
    frame(a, ctx, vec![]);
}
fn press_button(a: &mut Operandi, ctx: &Context, key: &str) {
    frame(a, ctx, vec![]);
    let p = a
        .toolbox_rects
        .get(key)
        .unwrap_or_else(|| panic!("no {key} button"))
        .center();
    click(a, ctx, p);
}
/// An app with unit cubes at `positions`, the first active, Create tab open.
fn app(positions: &[DVec3]) -> (Operandi, Context, Vec<u64>) {
    let mut a = Operandi::empty();
    let mut ids = Vec::new();
    for &p in positions {
        a.run(Command::NewMesh);
        let item = a.editor.scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
        item.transform.translation = p;
        ids.push(item.id);
    }
    a.editor.synchronize_identities().unwrap();
    a.editor.undo.clear();
    a.run(Command::SelectItem(Some(ids[0])));
    a.category = toolbox::Category::Create;
    a.create_folded.clear();
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    (a, ctx, ids)
}

#[test]
fn copy_and_paste_buttons_add_a_selected_copy() {
    let (mut a, ctx, _) = app(&[DVec3::ZERO]);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    press_button(&mut a, &ctx, "Copy Selection");
    assert!(a.editor.clipboard.is_some(), "{}", a.status);
    let undo = a.editor.undo.len();
    press_button(&mut a, &ctx, "Paste Selection");
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 7);
    assert_eq!(a.editor.undo.len(), undo + 1);
    press_button(&mut a, &ctx, "Cut Selection");
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
}

/// Jason, 2026-11-10: polygons, Cmd+C, another mesh, Cmd+V. With no text on
/// the system clipboard the window now sends an empty Paste event
/// (vendor/egui-winit), which must still paste into the other mesh.
#[test]
fn cmd_c_then_cmd_v_in_another_mesh_pastes_there() {
    let (mut a, ctx, ids) = app(&[DVec3::ZERO, DVec3::X * 3.]);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    frame(&mut a, &ctx, vec![Event::Copy]);
    assert!(a.editor.clipboard.is_some(), "{}", a.status);
    a.run(Command::SelectItem(Some(ids[1])));
    a.run(Command::Mode(Mode::Polygon));
    frame(&mut a, &ctx, vec![Event::Paste(String::new())]);
    let mesh = |id| &a.editor.scene.item(id).unwrap().mesh;
    assert_eq!(mesh(ids[1]).faces.len(), 8, "{}", a.status);
    assert_eq!(mesh(ids[0]).faces.len(), 6);
    // Pasted where they were copied: 3 units left of the second cube.
    let left = mesh(ids[1])
        .vertices
        .iter()
        .map(|v| v.x)
        .fold(f64::MAX, f64::min);
    assert!(left < -2., "{left}");
}

#[test]
fn paste_tool_adds_a_copy_where_the_viewport_is_clicked() {
    let (mut a, ctx, _) = app(&[DVec3::ZERO]);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.copy_selection();
    press_button(&mut a, &ctx, "Paste Tool");
    assert!(matches!(
        a.utility,
        Some(create_utility::Utility::PasteTool(_))
    ));
    let r = a.viewport_rect;
    let p = r.center() + vec2(-120., 60.);
    let at = a
        .camera
        .plane_point(p, r, a.actual_plane(), a.editor.scene.workplane.center)
        .unwrap();
    click(&mut a, &ctx, p);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 7, "{}", a.status);
    let corners: Vec<DVec3> = mesh.faces[6]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    let (lo, hi) = corners.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    assert!(((lo + hi) / 2. - at).length() < 1e-6, "{at} vs {lo} {hi}");
    // A second click adds another; Q puts the tool away (CV-03: Space
    // now opens Mini Properties).
    click(&mut a, &ctx, p + vec2(50., 0.));
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 8);
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(a.utility.is_none());
}

#[test]
fn merge_and_unmerge_buttons() {
    let (mut a, ctx, ids) = app(&[DVec3::ZERO, DVec3::new(3., 0., 0.)]);
    a.run(Command::ToggleItemSelection(ids[1]));
    press_button(&mut a, &ctx, "Item Merge");
    assert_eq!(
        a.editor.scene.items.iter().filter(|i| i.is_mesh()).count(),
        1,
        "{}",
        a.status
    );
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 12);
    press_button(&mut a, &ctx, "Item Unmerge");
    assert_eq!(
        a.editor.scene.items.iter().filter(|i| i.is_mesh()).count(),
        2,
        "{}",
        a.status
    );
    assert_eq!(a.editor.scene.active().unwrap().id, ids[0]);
}

#[test]
fn ground_align_click_puts_the_selection_on_the_ground() {
    let (mut a, ctx, ids) = app(&[DVec3::new(0., 2., 0.)]);
    a.run(Command::Mode(Mode::Polygon));
    a.run(Command::Selection(operandi_core::SelectionOperation::All));
    press_button(&mut a, &ctx, "Ground Align");
    assert!(a.utility.is_some(), "{}", a.status);
    let center = a.viewport_rect.center();
    click(&mut a, &ctx, center);
    let item = a.editor.scene.item(ids[0]).unwrap();
    let low = item
        .mesh
        .vertices
        .iter()
        .map(|v| a.editor.scene.world_point(ids[0], *v).y)
        .fold(f64::MAX, f64::min);
    assert!(low.abs() < 1e-9, "lowest point at {low}: {}", a.status);
}

#[test]
fn align_refuses_items_mode() {
    let (mut a, ctx, _) = app(&[DVec3::ZERO]);
    a.run(Command::Mode(Mode::Item));
    press_button(&mut a, &ctx, "Quick Align");
    assert!(a.utility.is_none());
    assert!(a.status.contains("selection mode"), "{}", a.status);
}

#[test]
fn ruler_end_drags_and_reads_its_length() {
    let (mut a, ctx, _) = app(&[DVec3::new(5., 0., 5.)]);
    press_button(&mut a, &ctx, "Ruler");
    let Some(create_utility::Utility::Ruler(before)) = a.utility else {
        panic!("Ruler off: {}", a.status);
    };
    assert!(((before.end - before.start).length() - 0.2).abs() < 1e-9);
    let r = a.viewport_rect;
    let (end, _) = a.camera.project(before.end, r).unwrap();
    let to = end + vec2(80., 0.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(end)]);
    frame(&mut a, &ctx, vec![pointer(end, true)]);
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)]);
    frame(&mut a, &ctx, vec![pointer(to, false)]);
    let Some(create_utility::Utility::Ruler(after)) = a.utility else {
        panic!("Ruler dropped");
    };
    assert_eq!(after.start, before.start);
    assert!((after.end - before.end).length() > 1e-3, "end didn't move");
    // Pressing the button again puts it away.
    press_button(&mut a, &ctx, "Ruler");
    assert!(a.utility.is_none());
}

#[test]
fn utility_groups_start_open_and_fold_on_click() {
    // Every Create group starts open (Jason, 2026-11-05).
    let mut a = Operandi::empty();
    a.category = toolbox::Category::Create;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    assert!(a.toolbox_rects.contains_key("Ruler"));
    press_button(&mut a, &ctx, "Measure");
    frame(&mut a, &ctx, vec![]);
    assert!(!a.toolbox_rects.contains_key("Ruler"));
}

#[test]
fn align_shares_one_row_and_measure_is_one_row_of_square_cells() {
    // Jason, 2026-11-05: Quick/Ground Align side by side at half width;
    // Ruler, Protractor and Dimensions icon-only on one row.
    let mut a = Operandi::empty();
    a.category = toolbox::Category::Create;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    let rect = |key: &str| a.toolbox_rects[key];
    let (quick, ground) = (rect("Quick Align"), rect("Ground Align"));
    assert_eq!(quick.top(), ground.top());
    assert!(quick.right() <= ground.left());
    assert!((quick.width() - ground.width()).abs() < 0.5);
    let measure = [rect("Ruler"), rect("Protractor"), rect("Dimensions")];
    for pair in measure.windows(2) {
        assert_eq!(pair[0].top(), pair[1].top());
        assert!(pair[0].right() <= pair[1].left());
    }
    for cell in measure {
        assert_eq!(cell.height(), toolbox::cell_height());
    }
    // The Edit cells share the Measure cells' height.
    assert_eq!(rect("Cut Selection").height(), toolbox::cell_height());
}

#[test]
fn a_modeling_tool_turns_a_utility_off() {
    let (mut a, ctx, _) = app(&[DVec3::ZERO]);
    press_button(&mut a, &ctx, "Protractor");
    assert!(a.utility.is_some());
    a.run(Command::Activate(Tool::Move));
    frame(&mut a, &ctx, vec![]);
    assert!(a.utility.is_none());
}

#[test]
fn shift_r_turns_quick_align_on() {
    let (mut a, ctx, _) = app(&[DVec3::ZERO]);
    a.run(Command::Mode(Mode::Polygon));
    let p = a.viewport_rect.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(p)]);
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::R,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        }],
    );
    assert!(
        matches!(a.utility, Some(create_utility::Utility::QuickAlign(_))),
        "{}",
        a.status
    );
}

#[test]
fn ground_align_right_click_turns_the_selected_face_down() {
    let (mut a, ctx, ids) = app(&[DVec3::new(0., 2., 0.)]);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    let normal_before = a.editor.scene.active().unwrap().mesh.normal(0);
    assert!(
        normal_before.y > -0.999,
        "pick a face that isn't already down"
    );
    press_button(&mut a, &ctx, "Ground Align");
    let p = a.viewport_rect.center();
    let right = |pressed| Event::PointerButton {
        pos: p,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    frame(&mut a, &ctx, vec![Event::PointerMoved(p)]);
    frame(&mut a, &ctx, vec![right(true), right(false)]);
    frame(&mut a, &ctx, vec![]);
    let item = a.editor.scene.item(ids[0]).unwrap();
    let n = item.mesh.normal(0);
    assert!(n.y < -0.999, "normal {n}: {}", a.status);
}
