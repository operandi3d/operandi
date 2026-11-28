//! Deform tab (Tier 1B): its rows start the presets, Push and Smooth, and
//! drags anywhere in the view drive Push and Smooth.
use super::tests::{box_app, draw, pointer};
use super::*;
use crate::viewport::bend_view::BendPart;

const SIZE: Vec2 = vec2(1440., 1600.);

/// A Deform tab control's rect: a button by its command line (the tab is
/// a form, `toolbox.deform.json`), a flyout by `flyout:<name>`, a section
/// header by its title.
fn rect(a: &Operandi, key: &str) -> Rect {
    a.form_rects
        .get(key)
        .or_else(|| a.toolbox_rects.get(key))
        .copied()
        .unwrap_or_else(|| panic!("no {key:?} on the Deform tab"))
}

/// Sculpt opens the Brush tool set's Sculpt tab (Jason, 2026-11-23).
const SCULPT: &str = "operandi.toolbox.tab sculpt";

fn click(a: &mut Operandi, ctx: &Context, key: &str) {
    let point = rect(a, key).center();
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(point, true, Modifiers::NONE),
            pointer(point, false, Modifiers::NONE),
        ],
    );
    draw(a, ctx, SIZE, vec![]);
}

/// A 2 m box in Vertex mode with the Deform tab showing.
fn deform_tab(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = box_app();
    a.camera.view = View::Front;
    a.run(Command::Mode(Mode::Vertex));
    a.editor.history.undo.clear();
    for _ in 0..3 {
        draw(&mut a, ctx, SIZE, vec![]);
    }
    click(&mut a, ctx, "Category Deform");
    assert_eq!(a.category, toolbox::Category::Deform);
    for _ in 0..2 {
        draw(&mut a, ctx, SIZE, vec![]);
    }
    a
}

fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for step in 1..=4 {
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(from + by * (step as f32 / 4.))],
        );
    }
    draw(
        a,
        ctx,
        SIZE,
        vec![pointer(from + by, false, Modifiers::NONE)],
    );
}

fn corner(a: &Operandi) -> Pos2 {
    a.viewport_rect.left_top() + vec2(80., 80.)
}

#[test]
fn deform_tab_lists_its_rows_in_order() {
    let ctx = Context::default();
    let a = deform_tab(&ctx);
    // DEFORM-LAYOUT-CORRECTION: the reference's 18 buttons in its order,
    // unbuilt ones greyed out; flyout faces show Soft Move, Smooth, Soft
    // Selection Move and Bulge.
    let order = [
        "Move Tools".to_owned(),
        "Element Move".into(),
        "Element Push".into(),
        "Flex".into(),
        "flyout:Soft Drag".into(),
        "flyout:Smooth".into(),
        "flyout:Soft Selection Move".into(),
        "Rotate Tools".into(),
        "Twist".into(),
        "Bend".into(),
        "Vortex".into(),
        "Swirl".into(),
        "Scale Tools".into(),
        "Push".into(),
        SCULPT.into(),
        "Taper".into(),
        "flyout:Bulge".into(),
        "Align Tools".into(),
        "Radial Align".into(),
        "Linear Align".into(),
        "Quick Align".into(),
        "Commands".into(),
        "Relax Mesh".into(),
    ];
    for pair in order.windows(2) {
        let (a_rect, b_rect) = (rect(&a, &pair[0]), rect(&a, &pair[1]));
        assert!(a_rect.top() < b_rect.top(), "{} above {}", pair[0], pair[1]);
    }
    // The faces: Soft Move, Smooth, Soft Selection Move and Bulge.
    for (flyout, face) in [
        ("Soft Drag", "Soft Drag".to_owned()),
        ("Smooth", "Smooth".into()),
        ("Soft Selection Move", "Soft Selection Move".into()),
        ("Bulge", "Bulge".into()),
    ] {
        assert_eq!(
            a.toolbox_rects[&face],
            rect(&a, &format!("flyout:{flyout}")),
            "{flyout} shows {face}"
        );
    }
    // Shear is only in the Soft Drag flyout.
    assert!(!a.form_rects.contains_key("Shear"));
    assert!(a.form_rects.contains_key("Ground Align"));
}

#[test]
fn preset_row_starts_the_preset_and_a_second_click_drops_it() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    click(&mut a, &ctx, "Twist");
    assert_eq!(a.editor.live_tool_label(), Some("Twist"), "{}", a.status);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Rotate);
    assert!(matches!(a.editor.spatial_falloff, Some(Falloff::Linear(_))));
    // Tool Properties shows the preset and its falloff.
    click(&mut a, &ctx, "Twist");
    assert!(a.editor.session.live.is_none());
    assert!(a.editor.spatial_falloff.is_none());
    // A neutral session leaves no step behind.
    assert!(a.editor.history.undo.is_empty());
}

#[test]
fn push_drags_up_to_push_out_and_drop_is_one_step() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    click(&mut a, &ctx, "Push");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Push)
    );
    let at = corner(&a);
    drag(&mut a, &ctx, at, vec2(0., -60.));
    let shift = a.editor.session.live.as_ref().unwrap().params.shift;
    assert!(shift > 0., "{}", a.status);
    let after = a.editor.scene.active().unwrap().mesh.vertices.clone();
    for (p, q) in before.iter().zip(&after) {
        assert!(q.length() > p.length(), "pushed out along the normals");
    }
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Push");
}

#[test]
fn smooth_drags_right_to_raise_strength() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    click(&mut a, &ctx, "Smooth");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Smooth)
    );
    let at = corner(&a);
    drag(&mut a, &ctx, at, vec2(100., 0.));
    let strength = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .smooth
        .strength;
    assert!(strength > 1.4 && strength < 1.6, "{strength}");
    // Each press-to-release is one step of the tool; Undo keeps it.
    a.run(Command::Undo);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Smooth)
    );
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .smooth
            .strength,
        1.
    );
}

#[test]
fn command_line_names_start_presets_push_and_smooth() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    for (name, label) in [
        ("soft move", "Soft Move"),
        ("shear", "Shear"),
        ("twist", "Twist"),
        ("swirl", "Swirl"),
        ("taper", "Taper"),
        ("bulge", "Bulge"),
        ("flex", "Flex"),
        ("push", "Push"),
        ("smooth", "Smooth"),
    ] {
        a.execute(name);
        assert_eq!(
            a.editor.live_tool_label(),
            Some(label),
            "{name}: {}",
            a.status
        );
        a.run(Command::Drop);
        assert!(a.editor.spatial_falloff.is_none(), "{name}");
    }
}

#[test]
fn sculpt_opens_the_brush_sculpt_tab() {
    // Jason, 2026-11-23: the Deform tab's Sculpt button opens the shipped
    // sculpting (Brush tool set, Sculpt tab) instead of a not-built note.
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    let before = a.editor.clone();
    assert_eq!(a.line_state(SCULPT).unwrap().unavailable, None);
    click(&mut a, &ctx, SCULPT);
    assert_eq!(a.category, toolbox::Category::Sculpt);
    assert_eq!(a.layouts.tool_set, toolbox::ToolSet::Brush);
    assert!(a.editor.session.live.is_none());
    assert!(a.editor.scene == before.scene);
}

fn right_click(a: &mut Operandi, ctx: &Context, key: &str) {
    let point = rect(a, key).center();
    let button = |pressed| Event::PointerButton {
        pos: point,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(point)]);
    draw(a, ctx, SIZE, vec![button(true), button(false)]);
    draw(a, ctx, SIZE, vec![]);
}

#[test]
fn soft_drag_flyout_picks_shear_and_keeps_it_on_the_face() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    right_click(&mut a, &ctx, "flyout:Soft Drag");
    // The list: Soft Drag, Soft Move, Shear.
    for key in ["Soft Drag", "Soft Move", "Shear"] {
        assert!(a.form_rects.contains_key(key), "{key}");
    }
    click(&mut a, &ctx, "Shear");
    assert_eq!(a.editor.live_tool_label(), Some("Shear"), "{}", a.status);
    // The face now shows Shear, lit; a plain click on it drops Shear.
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.toolbox_rects["Shear"], rect(&a, "flyout:Soft Drag"));
    assert!(a.line_state("Shear").unwrap().on);
    click(&mut a, &ctx, "flyout:Soft Drag");
    assert!(a.editor.session.live.is_none());
    click(&mut a, &ctx, "flyout:Soft Drag");
    assert_eq!(a.editor.live_tool_label(), Some("Shear"));
}

#[test]
fn bulge_and_smooth_are_flyouts() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    right_click(&mut a, &ctx, "flyout:Bulge");
    assert!(a.form_rects.contains_key("Flare"));
    click(&mut a, &ctx, "Bulge");
    assert_eq!(a.editor.live_tool_label(), Some("Bulge"));
    a.run(Command::Drop);
    right_click(&mut a, &ctx, "flyout:Smooth");
    for key in ["Smooth", "Jitter", "Quantize"] {
        assert!(a.form_rects.contains_key(key), "{key}");
    }
}

#[test]
fn element_move_is_the_reference_command_and_tweak_still_works() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    for line in [
        "tool.set ElementMove on",
        "tool.set tweak on",
        "element move",
        "tweak",
    ] {
        a.execute(line);
        assert_eq!(
            a.editor.live_tool_label(),
            Some("Element Move"),
            "{line}: {}",
            a.status
        );
        a.run(Command::Drop);
    }
    assert_eq!(a.bindings["Element Move"], "T");
}

fn right_drag(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![button(from, true)]);
    for step in 1..=4 {
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(from + by * (step as f32 / 4.))],
        );
    }
    draw(a, ctx, SIZE, vec![button(from + by, false)]);
}

/// SPEC-SELECTION-FALLOFF-FLEX §2: the Flex row starts Transform with a
/// Selection falloff and the Selection Border action centre; a second
/// click drops it and gives back the user's falloff and action centre.
#[test]
fn flex_button_starts_flex_and_drops_back() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    let action = a.editor.action_context;
    click(&mut a, &ctx, "Flex");
    assert_eq!(a.editor.live_tool_label(), Some("Flex"), "{}", a.status);
    assert!(matches!(
        a.editor.spatial_falloff,
        Some(Falloff::Selection(_))
    ));
    assert_eq!(
        a.editor.action_context.center,
        operandi_core::CenterMode::SelectionBorder
    );
    click(&mut a, &ctx, "Flex");
    assert!(a.editor.session.live.is_none());
    assert_eq!(a.editor.spatial_falloff, None);
    assert_eq!(a.editor.action_context, action);
}

/// Shift+Y starts Flex; its tooltip shows it.
#[test]
fn shift_y_starts_flex() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    assert!(a.shortcut("Flex").contains("Shift+Y"));
    let point = corner(&a);
    let event = |pressed| Event::Key {
        key: Key::Y,
        physical_key: Some(Key::Y),
        pressed,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(point)]);
    draw(&mut a, &ctx, SIZE, vec![event(true)]);
    draw(&mut a, &ctx, SIZE, vec![event(false)]);
    assert_eq!(a.editor.live_tool_label(), Some("Flex"), "{}", a.status);
}

/// §1: with Flex open, a right-drag changes Steps (one per 20 px right,
/// fewer to the left, never below 1) and leaves the mesh's points alone.
#[test]
fn right_drag_changes_selection_falloff_steps() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    a.editor.scene.selection.vertices = vec![0, 1, 2, 3];
    click(&mut a, &ctx, "Flex");
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let steps = |a: &Operandi| match a.editor.spatial_falloff {
        Some(Falloff::Selection(f)) => f.steps,
        other => panic!("{other:?}"),
    };
    assert_eq!(steps(&a), 2);
    let from = corner(&a);
    right_drag(&mut a, &ctx, from, vec2(65., 10.));
    assert_eq!(steps(&a), 5, "{}", a.status);
    right_drag(&mut a, &ctx, from, vec2(-300., 0.));
    assert_eq!(steps(&a), 1);
    assert_eq!(a.editor.live_tool_label(), Some("Flex"));
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, before);
}

fn bend(a: &Operandi) -> operandi_core::BendOptions {
    a.editor.session.live.as_ref().unwrap().params.bend
}
fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

/// Bend v2 (bend-handles-2026-11-15): the row starts Bend on Automatic
/// centre with the ring there; dragging the ring a quarter turn clockwise
/// on screen bends 90° toward +X; the spine's end drags the spine; a
/// sideways drag elsewhere moves the centre and never bends (G2); Ctrl+E
/// and the command line start it too; Cancel gives back the user's centre.
#[test]
fn bend_ring_spine_centre_and_shortcut() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    let user_context = a.editor.action_context;
    click(&mut a, &ctx, "Bend");
    assert_eq!(a.editor.live_tool_label(), Some("Bend"), "{}", a.status);
    assert_eq!(
        a.editor.action_context.center,
        operandi_core::CenterMode::Automatic
    );
    // BN-24 revised: a centre click at the box's centre in the front view
    // (XY): X and Y both reach 1 m to the far side, X wins the tie, so the
    // spine runs -X 1 m; the Axis is the plane's normal, Z (BN-30b).
    let plane = a.actual_plane();
    assert_eq!(plane, 2, "front view");
    a.run(Command::ToolClick {
        point: DVec3::ZERO,
        plane,
    });
    let fitted = bend(&a);
    assert!(fitted.base.length() < 1e-9, "{}", fitted.base);
    assert!(
        (fitted.spine - DVec3::new(-1., 0., 0.)).length() < 1e-9,
        "{}",
        fitted.spine
    );
    assert_eq!(fitted.axis, 2);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, before);
    // The ring: top to right is a quarter turn clockwise.
    let ring = a.bend_ring(
        &a.editor.session.live.as_ref().unwrap().params,
        a.viewport_rect,
    );
    let c = screen(&a, fitted.base);
    let radius = ring.iter().map(|p| p.distance(c)).fold(0., f32::max);
    assert!(radius > 40., "ring radius {radius}");
    let top = c - vec2(0., radius);
    assert_eq!(a.bend_hit(top, a.viewport_rect), Some(BendPart::Ring));
    drag(&mut a, &ctx, top, vec2(radius, radius));
    assert!(
        (bend(&a).angle - 90.).abs() < 0.5,
        "{} ({})",
        bend(&a).angle,
        a.status
    );
    let bent = a.editor.scene.active().unwrap().mesh.vertices.clone();
    assert_ne!(bent, before);
    // The spine's end: dragging it up lengthens the spine; Angle stays.
    let end = screen(&a, fitted.base + fitted.spine);
    assert_eq!(a.bend_hit(end, a.viewport_rect), Some(BendPart::SpineEnd));
    drag(&mut a, &ctx, end, vec2(0., -40.));
    assert!(bend(&a).spine.y > fitted.spine.y, "{}", a.status);
    assert!((bend(&a).angle - 90.).abs() < 0.5);
    // Off the handles, on the model: the centre moves, nothing bends more.
    let spine = bend(&a).spine;
    let inside = c + vec2(20., 25.);
    assert_eq!(a.bend_hit(inside, a.viewport_rect), None);
    drag(&mut a, &ctx, inside, vec2(30., 0.));
    assert!((bend(&a).angle - 90.).abs() < 0.5, "a sideways drag bent");
    assert!(bend(&a).base.x > 0.1, "{} ({})", bend(&a).base, a.status);
    // BN-24 revised: the centre click refits the spine (BH-Q05 answered).
    assert_ne!(bend(&a).spine, spine, "the spine refits");
    assert!(bend(&a).spine.x < -1., "{}", bend(&a).spine);
    // BN-46: a bare script Angle is degrees; a query answers radians.
    a.execute("tool.attr xfrm.bend angle 45");
    assert!((bend(&a).angle - 45.).abs() < 1e-9, "{}", a.status);
    a.execute("tool.attr xfrm.bend angle ?");
    let radians: f64 = a.status.parse().unwrap_or(f64::NAN);
    assert!((radians - 45f64.to_radians()).abs() < 1e-9, "{}", a.status);
    a.execute("tool.attr xfrm.bend angle 90");
    assert!((bend(&a).angle - 90.).abs() < 1e-9, "{}", a.status);
    a.run(Command::Cancel);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, before);
    assert_eq!(a.editor.action_context, user_context, "TP-12");
    assert!(a.shortcut("Bend").contains("Ctrl+E"));
    a.execute("bend");
    assert_eq!(a.editor.live_tool_label(), Some("Bend"));
    assert_eq!(
        a.editor.action_context.center,
        operandi_core::CenterMode::Automatic
    );
}

/// BN-31: with snapping on, a press near a point puts the centre on it,
/// in any selection mode (Polygon here).
#[test]
fn bend_press_snaps_the_centre_to_a_point() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    a.run(Command::Mode(Mode::Polygon));
    click(&mut a, &ctx, "Bend");
    assert_eq!(a.editor.live_tool_label(), Some("Bend"), "{}", a.status);
    a.snapping.enabled = true;
    // The box corner facing the eye.
    let (eye, ..) = a.camera.basis();
    let point = DVec3::new(1., 1., eye.z.signum());
    let near = screen(&a, point) + vec2(-3., 2.);
    drag(&mut a, &ctx, near, vec2(0., 0.));
    assert!(
        (bend(&a).base - point).length() < 1e-6,
        "{} ({})",
        bend(&a).base,
        a.status
    );
    // Off the model too, the centre moves (to the work plane there).
    let at = corner(&a);
    drag(&mut a, &ctx, at, vec2(0., 0.));
    assert!((bend(&a).base - point).length() > 0.5, "{}", a.status);
}

/// BN-48: scripted runs honour the Auto Action Center X/Y/Z and the Axis
/// typed in the panel; BN-47: `tool.reset` gives Angle 0 and Spine
/// (0, 1 m, 0); BN-45: an axis along the spine moves nothing and says so.
#[test]
fn bend_script_centre_axis_reset_and_axis_along_spine() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    let before = a.editor.scene.active().unwrap().mesh.vertices.clone();
    a.execute("bend");
    assert_eq!(a.editor.live_tool_label(), Some("Bend"), "{}", a.status);
    // Up the box from its foot (the fitted spine is whatever the work
    // plane gives; BN-24).
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.bend.spine = DVec3::new(0., 2., 0.);
    a.run(Command::Preview(p));
    a.execute("tool.attr xfrm.bend operandi.center1 -1");
    a.execute("tool.attr xfrm.bend operandi.actionAxis X");
    a.execute("tool.attr xfrm.bend angle 90");
    assert_eq!(bend(&a).base, DVec3::new(0., -1., 0.), "{}", a.status);
    assert_eq!(bend(&a).axis, 0, "{}", a.status);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(0., -1., 0.)
    );
    // Axis X curls the box toward -Z; the bottom (y = -1) stays.
    let after = a.editor.scene.active().unwrap().mesh.vertices.clone();
    for (p, q) in before.iter().zip(&after) {
        if p.y < -0.999 {
            assert!((*p - *q).length() < 1e-9, "{p} -> {q}");
        } else {
            assert!(q.z <= p.z + 1e-9, "{p} -> {q}");
        }
    }
    assert_ne!(after, before);
    // BN-45: Axis Y runs along the spine; the mesh goes back unmoved.
    a.execute("tool.attr xfrm.bend operandi.actionAxis Y");
    // What every frame runs after composing (the test frames don't).
    a.sync_symmetry_notice();
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, before);
    assert_eq!(
        a.status,
        "The bend axis runs along the spine; pick another axis"
    );
    // BN-47: Reset.
    a.execute("tool.reset xfrm.bend");
    assert_eq!(bend(&a).angle, 0.);
    assert_eq!(bend(&a).spine, DVec3::Y);
    assert_eq!(
        bend(&a).base,
        DVec3::new(0., -1., 0.),
        "Reset keeps the centre"
    );
}

/// Hands-on batch (2026-11-17): Soft Drag, Element Push and the Soft
/// Selection rows start their presets; a Soft Drag press centres the
/// circle on the pointer and a right-drag sizes it.
#[test]
fn hands_on_rows_start_their_presets() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    for (key, id) in [
        ("Element Push", "ElementPush"),
        ("flyout:Soft Drag", "xfrm.softDrag"),
        ("flyout:Soft Selection Move", "SoftSelectionMove"),
    ] {
        click(&mut a, &ctx, key);
        assert_eq!(
            a.editor.session.preset.as_ref().map(|p| p.id.as_str()),
            Some(id),
            "{key}: {}",
            a.status
        );
        a.run(Command::Drop);
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    right_click(&mut a, &ctx, "flyout:Soft Selection Move");
    assert!(a.form_rects.contains_key("Soft Selection Rotate"));
    // Soft Drag: a press moves the circle to the pointer.
    click(&mut a, &ctx, "flyout:Soft Drag");
    let at = a.viewport_rect.center() + vec2(30., -20.);
    drag(&mut a, &ctx, at, vec2(5., 0.));
    let Some(Falloff::Screen(s)) = a.editor.spatial_falloff else {
        panic!("Soft Drag uses a Screen falloff");
    };
    let local = at - a.viewport_rect.min;
    assert!((s.center.x - local.x as f64).abs() < 1. && (s.center.y - local.y as f64).abs() < 1.);
    assert_eq!(s.size, 20., "20 px to start");
}

/// Holding the press on a flyout opens its list; the release that ends
/// the hold doesn't also start the face.
#[test]
fn holding_a_flyout_opens_its_list() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    let point = rect(&a, "flyout:Bulge").center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(point)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(point, true, Modifiers::NONE)],
    );
    // Headless frames advance the clock by 1/60 s: about half a second.
    for _ in 0..30 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert!(a.form_rects.contains_key("Flare"), "the list is open");
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(point, false, Modifiers::NONE)],
    );
    assert!(
        a.editor.session.live.is_none(),
        "the release didn't start Bulge"
    );
}

/// Quick Align and Ground Align light while their mode is on, from their
/// commands' answers (UIC-223), and draw no shortcut on the half-width
/// buttons.
#[test]
fn align_row_lights_from_its_commands() {
    let ctx = Context::default();
    let mut a = deform_tab(&ctx);
    assert!(!a.line_state("Quick Align").unwrap().on);
    click(&mut a, &ctx, "Quick Align");
    assert!(a.line_state("Quick Align").unwrap().on, "{}", a.status);
    assert!(!a.line_state("Ground Align").unwrap().on);
    assert!(
        rect(&a, "Quick Align").width() < rect(&a, "Twist").width() / 2. + 1.,
        "half width"
    );
}

/// Every Deform tab button, built or not, has artwork of its own: no two
/// lines share an icon, and none borrows another tab's.
#[test]
fn deform_tab_icons_are_distinct() {
    use crate::form::command_form::{self as cf, Item};
    let a = box_app();
    let items = cf::plan(
        &a.forms,
        crate::toolbox_form::DEFORM_FORM,
        crate::form::Proficiency::Advanced,
        &|_| true,
    );
    let mut controls = Vec::new();
    for item in items {
        match item {
            Item::Button(c, _) => controls.push(*c),
            Item::Row(cs, _, _) | Item::Flyout { controls: cs, .. } => controls.extend(cs),
            _ => {}
        }
    }
    let mut seen = std::collections::BTreeMap::new();
    for c in &controls {
        let line = c.command.clone().unwrap();
        if line.contains("Relax Mesh") {
            continue;
        }
        let icon = c
            .icon
            .as_deref()
            .map(|slug| icons::Icon::from_slug(slug).unwrap_or_else(|| panic!("{slug}")))
            .or_else(|| tool_info::by_name(&line.to_lowercase()).map(|t| tool_info::info(t).icon))
            .unwrap_or_else(|| panic!("{line} has no icon"));
        if let Some(other) = seen.insert(icon, line.clone()) {
            panic!("{other} and {line} share {icon:?}");
        }
    }
    assert!(seen.len() >= 25, "{}", seen.len());
    for icon in [
        icons::Icon::Move,
        icons::Icon::Rotate,
        icons::Icon::Scale,
        icons::Icon::Transform,
        icons::Icon::PolyHaul,
        icons::Icon::EdgeRelax,
    ] {
        assert!(!seen.contains_key(&icon), "{icon:?} is borrowed");
    }
}
