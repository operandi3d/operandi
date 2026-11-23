//! Panel corner dot (panel_dot.rs): the dot on each panel, its drags,
//! menu, locks and the copies it makes.
use super::*;
use crate::floating::Panel;
use crate::panel_dot::{DockTarget, Dot, Gesture, LayoutLocks, MenuAction, Over, Zone};
use crate::tests::{box_app, pointer};
use operandi_layout::{Pane, Side};

const SIZE: Vec2 = vec2(1440., 900.);

/// One frame with these events and modifiers held.
fn frame(
    a: &mut Operandi,
    ctx: &Context,
    events: Vec<Event>,
    modifiers: Modifiers,
) -> egui::FullOutput {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
        events,
        modifiers,
        focused: true,
        ..Default::default()
    };
    ctx.run(raw, |ctx| {
        a.begin_input(ctx);
        a.compose(ctx);
        a.keyboard(ctx);
    })
}

fn idle(a: &mut Operandi, ctx: &Context) {
    for _ in 0..2 {
        frame(a, ctx, vec![], Modifiers::NONE);
    }
}

/// Press at `from`, move to `to` over a few frames, release there.
/// A drag in UI build mode (the only time panels and tabs move), which is
/// put back as it was afterwards. `drag_raw` drags outside it.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2, m: Modifiers) {
    let was = a.dots.build_mode;
    if !was {
        a.dots.build_mode = true;
        // The covers take input from the frame after they first draw.
        idle(a, ctx);
    }
    drag_raw(a, ctx, from, to, m);
    a.dots.build_mode = was;
}

/// Turn UI build mode on and let its covers draw once.
fn enter_build_mode(a: &mut Operandi, ctx: &Context) {
    a.dots.build_mode = true;
    idle(a, ctx);
}

fn drag_raw(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2, m: Modifiers) {
    frame(a, ctx, vec![Event::PointerMoved(from)], m);
    frame(a, ctx, vec![pointer(from, true, m)], m);
    for step in 1..=6 {
        let at = from.lerp(to, step as f32 / 6.);
        frame(a, ctx, vec![Event::PointerMoved(at)], m);
    }
    frame(a, ctx, vec![pointer(to, false, m)], m);
    idle(a, ctx);
}

/// A panel move, in UI build mode (the only time panels move), which is
/// put back as it was afterwards.
fn drag_dot(a: &mut Operandi, ctx: &Context, d: Dot, to: Pos2, m: Modifiers) {
    let from = dot(a, d);
    drag(a, ctx, from, to, m);
}

fn click_dot(a: &mut Operandi, ctx: &Context, d: Dot) {
    click(a, ctx, d.grip_center(a.dots.rects[&d]));
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    frame(
        a,
        ctx,
        vec![pointer(at, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        a,
        ctx,
        vec![pointer(at, false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    idle(a, ctx);
}

fn dot(a: &Operandi, d: Dot) -> Pos2 {
    a.dots.rects[&d].center()
}

/// Two views side by side, the first active.
fn two_views() -> (Operandi, Context) {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    let ctx = Context::default();
    a.split_view();
    a.activate_view(0);
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![], Modifiers::NONE);
    }
    (a, ctx)
}

#[test]
fn every_panel_has_a_corner_grip_and_a_press_makes_its_panel_active() {
    let (mut a, ctx) = two_views();
    for d in [
        Dot::View(0),
        Dot::View(1),
        Dot::Side(Panel::Toolbox),
        Dot::Side(Panel::Items),
        Dot::Side(Panel::Properties),
    ] {
        assert!(a.dots.rects.contains_key(&d), "{d:?}");
    }
    assert!(a.dot_is_active(Dot::View(0)));
    assert!(!a.dot_is_active(Dot::View(1)));
    // A click on view 1's dot: view 1 is active, and nothing else changed.
    let selection = a.editor.scene.selection.clone();
    click_dot(&mut a, &ctx, Dot::View(1));
    assert_eq!(a.active_view, 1);
    assert!(a.dot_is_active(Dot::View(1)));
    assert_eq!(a.editor.scene.selection, selection);
    assert_eq!(a.views.viewports(), [0, 1]);
    // The Toolbox's dot makes the Toolbox the active panel.
    click_dot(&mut a, &ctx, Dot::Side(Panel::Toolbox));
    assert!(a.dot_is_active(Dot::Side(Panel::Toolbox)));
    assert!(!a.dot_is_active(Dot::View(1)));
    // A press inside a view brings the active panel back to the views.
    let inside = a.view_rects[&0].center();
    click(&mut a, &ctx, inside);
    assert!(a.dot_is_active(Dot::View(0)));
}

#[test]
fn hover_grip_reserves_no_toolbox_or_tool_tab_space() {
    use crate::layouts_tests::{data_root, frames, launch};
    let ctx = Context::default();
    let mut a = launch(&data_root(), &ctx);
    frames(&mut a, &ctx, 2);
    let toolbox = a.dot_panel_rect(Dot::Side(Panel::Toolbox)).unwrap();
    let transforms = a.toolbox_rects[Tool::Transform.label()];
    assert_eq!(transforms.top(), toolbox.top() + toolbox::DOCK_PANEL_INSET);

    let tabs = a.dot_panel_rect(Dot::ToolTabs).unwrap();
    assert_eq!(
        a.toolbox_rects["Category Create"].left(),
        tabs.left() + toolbox::DOCK_PANEL_INSET
    );
}

fn dot_fill(out: &egui::FullOutput, center: Pos2) -> Option<Color32> {
    out.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Circle(c) if c.center == center && c.radius == crate::panel_dot::DOT / 2. => {
            Some(c.fill)
        }
        _ => None,
    })
}

#[test]
fn view_dots_are_visible_at_rest_and_open_the_view_menu() {
    for pane in [Pane::Viewport(1), Pane::Uv(1), Pane::Missing(1)] {
        let (mut a, ctx) = two_views();
        if pane != Pane::Viewport(1) {
            a.views.replace(Pane::Viewport(1), pane).unwrap();
        }
        if matches!(pane, Pane::Missing(_)) {
            a.views.missing.insert(1, operandi_layout::EMPTY.into());
        }
        let out = frame(&mut a, &ctx, vec![], Modifiers::NONE);
        let d = Dot::View(1);
        let hit = a.dots.rects[&d];
        let view = a.dot_panel_rect(d).unwrap();
        assert_eq!(hit.center(), view.min + vec2(9., 12.5), "{pane:?}");
        assert_eq!(hit.size(), vec2(11., 11.), "{pane:?}");
        assert_eq!(
            dot_fill(&out, hit.center()),
            Some(theme::panel_dot()),
            "{pane:?}"
        );

        let edge = view.center_top() + vec2(0., 2.);
        let out = right_click_panel(&mut a, &ctx, edge);
        assert!(menu_label(&out, "Split Horizontal").is_none(), "{pane:?}");

        let out = right_click_panel(&mut a, &ctx, hit.center());
        for label in ["Duplicate", "Detach", "Maximize", "Split Horizontal"] {
            assert!(menu_label(&out, label).is_some(), "{pane:?}: {label}");
        }
    }
}

#[test]
fn hover_grip_stays_hidden_outside_build_mode() {
    // Panels move only in UI build mode, so outside it the corner grip
    // never shows, even under the pointer.
    let (mut a, ctx) = two_views();
    let d = Dot::Side(Panel::Toolbox);
    let from = d.grip_center(a.dots.rects[&d]);
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    for _ in 0..12 {
        let out = frame(&mut a, &ctx, vec![], Modifiers::NONE);
        assert!(dot_fill(&out, from).is_none());
    }
}

#[test]
fn clicking_a_control_under_the_hover_grip_still_reaches_the_control() {
    let (mut a, ctx) = two_views();
    let grip = a.dots.rects[&Dot::ToolProperties];
    let overlap = a.toolbox_rects["Tool Properties"].intersect(grip);
    assert!(overlap.is_positive(), "the grip overlaps the disclosure");
    let was_open = a.tool_properties_open;
    click(&mut a, &ctx, overlap.center());
    assert_ne!(a.tool_properties_open, was_open);
    assert!(a.dots.drag.is_none());
}

#[test]
fn shift_command_or_control_middle_drag_docks_from_the_panel_body() {
    for m in [
        Modifiers {
            shift: true,
            command: true,
            mac_cmd: true,
            ..Modifiers::NONE
        },
        Modifiers {
            shift: true,
            ctrl: true,
            ..Modifiers::NONE
        },
    ] {
        let (mut a, ctx) = two_views();
        enter_build_mode(&mut a, &ctx);
        let from = a.corner.areas[&0].center();
        let to = a.corner.areas[&1].center();
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Middle,
            pressed,
            modifiers: m,
        };
        let before = a.camera.clone();
        frame(&mut a, &ctx, vec![Event::PointerMoved(from)], m);
        frame(&mut a, &ctx, vec![button(from, true)], m);
        frame(&mut a, &ctx, vec![Event::PointerMoved(to)], m);
        let drag = a
            .dots
            .drag
            .as_ref()
            .expect("shortcut started a docking drag");
        assert_eq!(drag.dot, Dot::View(0));
        assert_eq!(drag.gesture, Gesture::Move);
        frame(&mut a, &ctx, vec![button(to, false)], m);
        idle(&mut a, &ctx);
        assert_eq!(
            tab_panes(&a, Pane::Viewport(0)),
            (
                vec![Pane::Viewport(1), Pane::Viewport(0)],
                Pane::Viewport(0)
            )
        );
        assert_eq!(a.camera.target, before.target);
        assert!(a.navigation.is_none());
    }
}

fn right_click_panel(a: &mut Operandi, ctx: &Context, at: Pos2) -> egui::FullOutput {
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(a, ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    frame(a, ctx, vec![button(true)], Modifiers::NONE);
    frame(a, ctx, vec![button(false)], Modifiers::NONE);
    frame(a, ctx, vec![], Modifiers::NONE)
}

fn menu_label(out: &egui::FullOutput, label: &str) -> Option<Pos2> {
    out.shapes.iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == label => {
            Some(t.pos + t.galley.rect.center().to_vec2())
        }
        _ => None,
    })
}

#[test]
fn panel_header_menu_floats_and_docks_at_existing_work_area_edges() {
    let (mut a, ctx) = two_views();
    let toolbox = a.dot_panel_rect(Dot::Side(Panel::Toolbox)).unwrap();
    let header = pos2(toolbox.center().x, toolbox.top() + 8.);
    let out = right_click_panel(&mut a, &ctx, header);
    for label in [
        "Dock at Left",
        "Dock at Right",
        "Dock at Top",
        "Dock at Bottom",
    ] {
        assert!(menu_label(&out, label).is_some(), "missing {label}");
    }
    click(&mut a, &ctx, menu_label(&out, "Float").expect("Float"));
    assert!(a.is_floating(Panel::Toolbox));

    let floating = a.dot_panel_rect(Dot::Side(Panel::Toolbox)).unwrap();
    let at = pos2(floating.center().x, floating.top() + 8.);
    let out = right_click_panel(&mut a, &ctx, at);
    click(
        &mut a,
        &ctx,
        menu_label(&out, "Dock at Right").expect("Dock at Right"),
    );
    assert!(!a.is_floating(Panel::Toolbox), "{}", a.status);
    assert!(dock_leaf(&a, Pane::Toolbox).left() >= dock_leaf(&a, Pane::Viewport(1)).right());
}

#[test]
fn mode_bar_menu_only_offers_top_and_bottom() {
    let (mut a, ctx) = one_view();
    let row = a.layouts.mode_bar_rect.unwrap();
    let out = right_click_panel(&mut a, &ctx, row.center());
    for label in ["Float", "Dock at Left", "Dock at Right"] {
        assert!(menu_label(&out, label).is_none(), "unexpected {label}");
    }
    let label = if a.layouts.mode_bar_bottom {
        "Dock at Top"
    } else {
        "Dock at Bottom"
    };
    let was_bottom = a.layouts.mode_bar_bottom;
    assert!(menu_label(&out, "Dock at Top").is_some());
    assert!(menu_label(&out, "Dock at Bottom").is_some());
    click(&mut a, &ctx, menu_label(&out, label).unwrap());
    assert_ne!(a.layouts.mode_bar_bottom, was_bottom);
}

#[test]
fn mode_bar_docks_from_the_corner_or_middle_shortcut_and_escape_cancels() {
    let (mut a, ctx) = one_view();
    enter_build_mode(&mut a, &ctx);
    let started_bottom = a.layouts.mode_bar_bottom;
    let row = a.layouts.mode_bar_rect.unwrap();
    let to = pos2(
        row.center().x,
        if started_bottom { 120. } else { SIZE.y - 100. },
    );
    drag(
        &mut a,
        &ctx,
        crate::panel_dot::grip_rect(row).center(),
        to,
        Modifiers::NONE,
    );
    assert_ne!(a.layouts.mode_bar_bottom, started_bottom);

    let row = a.layouts.mode_bar_rect.unwrap();
    let from = row.center();
    let to = pos2(from.x, if started_bottom { SIZE.y - 100. } else { 120. });
    let m = Modifiers {
        ctrl: true,
        shift: true,
        ..Modifiers::NONE
    };
    let middle = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Middle,
        pressed,
        modifiers: m,
    };
    for cancel in [true, false] {
        frame(&mut a, &ctx, vec![Event::PointerMoved(from)], m);
        frame(&mut a, &ctx, vec![middle(from, true)], m);
        frame(&mut a, &ctx, vec![Event::PointerMoved(to)], m);
        assert_eq!(a.layouts.mode_bar_drag, Some(PointerButton::Middle));
        if cancel {
            frame(&mut a, &ctx, vec![crate::tests::key(Key::Escape)], m);
        }
        frame(&mut a, &ctx, vec![middle(to, false)], m);
        idle(&mut a, &ctx);
        assert_eq!(a.layouts.mode_bar_bottom, started_bottom != cancel);
    }
}

#[test]
fn hovering_a_dot_outlines_its_whole_panel_in_the_accent() {
    let (mut a, ctx) = two_views();
    let at = dot(&a, Dot::View(1));
    frame(&mut a, &ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    let out = frame(&mut a, &ctx, vec![], Modifiers::NONE);
    let pane = a.corner.areas[&1];
    let outlined = out.shapes.iter().any(|s| {
        matches!(&s.shape, egui::Shape::Rect(r)
            if r.stroke.color == theme::orange()
                && (r.rect.width() - pane.width()).abs() < 2.
                && (r.rect.height() - pane.height()).abs() < 2.)
    });
    assert!(outlined, "the hovered dot's panel is outlined");
    // Away from the dot: no outline.
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(pane.center())],
        Modifiers::NONE,
    );
    let out = frame(&mut a, &ctx, vec![], Modifiers::NONE);
    assert!(
        !out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if r.stroke.color == theme::orange() && (r.rect.width() - pane.width()).abs() < 2.))
    );
}

#[test]
fn drop_zones_are_a_middle_half_and_four_edge_quarters() {
    use crate::panel_dot::{landing, split_side, zone};
    let r = Rect::from_min_size(pos2(100., 100.), vec2(400., 200.));
    assert_eq!(zone(r, r.center()), Zone::Center);
    assert_eq!(zone(r, pos2(140., 200.)), Zone::Edge(Side::Left));
    assert_eq!(zone(r, pos2(470., 200.)), Zone::Edge(Side::Right));
    assert_eq!(zone(r, pos2(300., 110.)), Zone::Edge(Side::Top));
    assert_eq!(zone(r, pos2(300., 290.)), Zone::Edge(Side::Bottom));
    // A corner goes to the nearer edge.
    assert_eq!(zone(r, pos2(102., 120.)), Zone::Edge(Side::Left));
    assert_eq!(landing(r, Zone::Edge(Side::Right)).left(), 300.);
    assert_eq!(landing(r, Zone::Center), r);
    assert_eq!(split_side(vec2(30., 5.)), Side::Right);
    assert_eq!(split_side(vec2(-30., 5.)), Side::Left);
    assert_eq!(split_side(vec2(3., 40.)), Side::Bottom);
    assert_eq!(split_side(vec2(3., -40.)), Side::Top);
}

#[test]
fn dragging_onto_a_views_centre_tabs_the_two() {
    let (mut a, ctx) = two_views();
    enter_build_mode(&mut a, &ctx);
    let from = dot(&a, Dot::View(0));
    let to = a.corner.areas[&1].center();
    // Mid-drag the target shows its zones: the centre is under the pointer.
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(from, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    for step in 1..=6 {
        let at = from.lerp(to, step as f32 / 6.);
        frame(&mut a, &ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    }
    let drag = a.dots.drag.as_ref().expect("dragging");
    assert_eq!(drag.dot, Dot::View(0));
    assert_eq!(drag.gesture, Gesture::Move);
    assert_eq!(
        drag.over,
        Over::Dock(DockTarget::Compass(Dot::View(1), Zone::Center))
    );
    let out = frame(&mut a, &ctx, vec![], Modifiers::NONE);
    let pane = a.corner.areas[&1];
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if r.fill.a() > 0 && r.fill.a() < 255 && pane.contains_rect(r.rect)
            && r.rect.width() > pane.width() * 0.9)),
        "the landing area is filled"
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(to, false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    idle(&mut a, &ctx);
    assert_eq!(
        tab_panes(&a, Pane::Viewport(0)),
        (
            [Pane::Viewport(1), Pane::Viewport(0)].to_vec(),
            Pane::Viewport(0)
        )
    );
    assert!(a.corner.areas.contains_key(&0));
    assert!(!a.corner.areas.contains_key(&1));
    check_reopen(&a);
    assert!(a.dots.drag.is_none());
}

#[test]
fn dragging_onto_an_edge_docks_beside_at_half() {
    let (mut a, ctx) = two_views();
    let right = a.corner.areas[&1];
    let to = a.compass_point(Dot::View(1), Zone::Edge(Side::Bottom));
    drag_dot(&mut a, &ctx, Dot::View(0), to, Modifiers::NONE);
    // View 0 left its place (view 1 fills the width) and sits under view 1.
    let (v0, v1) = (a.corner.areas[&0], a.corner.areas[&1]);
    assert!(v1.width() > right.width() * 1.5, "{v1:?}");
    assert!(v0.top() >= v1.bottom() - 6., "{v0:?} under {v1:?}");
    assert!((v0.height() - v1.height()).abs() < 4., "half each");
    assert!(a.views.windows.is_empty());
}

#[test]
fn dragging_outside_every_panel_tears_off_and_dragging_back_docks() {
    let (mut a, ctx) = two_views();
    let size = a.corner.areas[&1].size();
    let release = pos2(700., 6.); // over the toolbar: no panel there
    drag_dot(&mut a, &ctx, Dot::View(1), release, Modifiers::NONE);
    assert_eq!(a.views.windows.len(), 1);
    let window = &a.views.windows[0];
    assert_eq!(window.position, Some([release.x, release.y]));
    assert!((window.size[0] - size.x).abs() < 1. && (window.size[1] - size.y).abs() < 1.);
    assert_eq!(a.views.window_of(Pane::Viewport(1)), Some(window.id));
    // The main view grew into the hole.
    assert!(a.corner.areas[&0].width() > SIZE.x * 0.4);
    // Dropped back on the main view's right edge, it docks there and the
    // empty window closes. (Tests draw windows inside the main one, where
    // their panels cover it, so the drop is made directly.)
    a.drop_for_test(
        Dot::View(1),
        Over::Panel(Dot::View(0), Zone::Edge(Side::Right)),
    );
    idle(&mut a, &ctx);
    assert!(a.views.windows.is_empty(), "{:?}", a.views.windows.len());
    assert!(a.corner.areas[&1].left() >= a.corner.areas[&0].right() - 6.);
}

#[test]
fn a_lone_floating_view_dragged_out_leaves_a_copy_and_stays() {
    let (mut a, ctx) = two_views();
    a.dot_menu_action(Dot::View(1), MenuAction::Detach);
    idle(&mut a, &ctx);
    assert_eq!(a.views.windows.len(), 1);
    let rules_delete = a.can_close_view(1);
    assert!(!rules_delete, "alone in its window: Delete is greyed");
    drag_dot(&mut a, &ctx, Dot::View(1), pos2(700., 6.), Modifiers::NONE);
    assert_eq!(a.views.windows.len(), 2, "a second window with a copy");
    assert!(
        a.views.alone_in_window(Pane::Viewport(1)),
        "the original stays"
    );
    assert_eq!(a.views.viewports().len(), 3);
}

#[test]
fn esc_during_a_drag_changes_nothing() {
    let (mut a, ctx) = two_views();
    enter_build_mode(&mut a, &ctx);
    let before = a.views.main.clone();
    let from = dot(&a, Dot::View(0));
    let to = a.corner.areas[&1].center();
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(from, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    for step in 1..=6 {
        frame(
            &mut a,
            &ctx,
            vec![Event::PointerMoved(from.lerp(to, step as f32 / 6.))],
            Modifiers::NONE,
        );
    }
    assert!(a.dots.drag.is_some());
    frame(
        &mut a,
        &ctx,
        vec![crate::tests::key(Key::Escape)],
        Modifiers::NONE,
    );
    assert!(a.dots.drag.is_none());
    frame(
        &mut a,
        &ctx,
        vec![pointer(to, false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    idle(&mut a, &ctx);
    assert_eq!(a.views.main, before);
    assert!(a.views.windows.is_empty());
}

#[test]
fn the_last_main_panel_only_splits() {
    let mut a = box_app();
    let ctx = Context::default();
    idle(&mut a, &ctx);
    // Plain and Shift drags out do nothing.
    for m in [Modifiers::NONE, Modifiers::SHIFT] {
        drag_dot(&mut a, &ctx, Dot::View(0), pos2(700., 6.), m);
        assert_eq!(a.views.viewports(), [0]);
        assert!(a.views.windows.is_empty());
    }
    // Delete and Detach are greyed (and do nothing if asked).
    assert!(!a.can_close_view(0));
    a.dot_menu_action(Dot::View(0), MenuAction::Delete);
    a.dot_menu_action(Dot::View(0), MenuAction::Detach);
    assert_eq!(a.views.viewports(), [0]);
    assert!(a.views.windows.is_empty());
    // Ctrl+drag down splits it, stacked; the new lower view is active.
    let from = dot(&a, Dot::View(0));
    drag(&mut a, &ctx, from, from + vec2(5., 200.), Modifiers::CTRL);
    assert_eq!(a.views.viewports().len(), 2);
    let new = a.active_view;
    assert_ne!(new, 0);
    assert!(a.corner.areas[&new].top() > a.corner.areas[&0].top() + 100.);
    assert!((a.corner.areas[&new].left() - a.corner.areas[&0].left()).abs() < 1.);
}

#[test]
fn shift_drag_copies_with_the_same_view_settings() {
    use crate::view_display::Style;
    let (mut a, ctx) = two_views();
    a.camera.view = View::Front;
    a.camera.distance = 17.;
    a.display.set_style(Style::Wireframe, false);
    idle(&mut a, &ctx);
    // Shift onto an edge docks a copy there.
    let right = a.corner.areas[&1];
    drag_dot(
        &mut a,
        &ctx,
        Dot::View(0),
        pos2(right.right() - 15., right.center().y),
        Modifiers::SHIFT,
    );
    assert_eq!(a.views.viewports().len(), 3);
    assert!(a.views.windows.is_empty());
    assert!(
        a.corner.areas[&a.active_view].left() > a.corner.areas[&1].left(),
        "docked right of view 1, active"
    );

    drag_dot(&mut a, &ctx, Dot::View(0), pos2(700., 6.), Modifiers::SHIFT);
    assert_eq!(a.views.windows.len(), 1);
    assert!(
        a.views.panes().contains(&Pane::Viewport(0)),
        "the original stays"
    );
    let copy = *a.views.viewports().last().unwrap();
    assert_eq!(
        a.views.window_of(Pane::Viewport(copy)),
        Some(a.views.windows[0].id)
    );
    let camera = &a.cameras[&copy];
    assert_eq!(camera.view, View::Front);
    assert_eq!(camera.distance, 17.);
    assert_eq!(a.displays[&copy].settings.active.style, Style::Wireframe);
    assert_eq!(a.views.viewports().len(), 4);
}

#[test]
fn the_menu_duplicates_copies_detaches_deletes_and_splits() {
    let (mut a, ctx) = two_views();
    a.camera.view = View::Top;
    idle(&mut a, &ctx);
    // Duplicate: a copy at the right of the layout.
    a.dot_menu_action(Dot::View(0), MenuAction::Duplicate);
    idle(&mut a, &ctx);
    let dup = *a.views.viewports().last().unwrap();
    assert_eq!(a.cameras[&dup].view, View::Top);
    assert!(a.corner.areas[&dup].left() > a.corner.areas[&1].left());
    // Copy: in a new window near the panel.
    a.dot_menu_action(Dot::View(0), MenuAction::Copy);
    idle(&mut a, &ctx);
    assert_eq!(a.views.windows.len(), 1);
    let area = a.corner.areas[&0];
    let [x, y] = a.views.windows[0].position.unwrap();
    assert!((x - area.left()).abs() < 40. && (y - area.top()).abs() < 40.);
    // Delete the copy's... no: a lone floating view can't be deleted; a
    // main one can.
    a.dot_menu_action(Dot::View(dup), MenuAction::Delete);
    idle(&mut a, &ctx);
    assert!(!a.views.viewports().contains(&dup));
    // Split Horizontal / Vertical: halves, the new one active.
    a.dot_menu_action(Dot::View(1), MenuAction::SplitHorizontal);
    idle(&mut a, &ctx);
    let h = a.active_view;
    assert!(a.corner.areas[&h].left() > a.corner.areas[&1].left());
    assert!((a.corner.areas[&h].width() - a.corner.areas[&1].width()).abs() < 3.);
    a.dot_menu_action(Dot::View(h), MenuAction::SplitVertical);
    idle(&mut a, &ctx);
    let v = a.active_view;
    assert!(a.corner.areas[&v].top() > a.corner.areas[&h].top());
    // The View menu's Split View is Split Horizontal on the active view.
    let count = a.views.viewports().len();
    a.menu_action(&ctx, crate::app_menu::line_action("viewport.split"));
    assert_eq!(a.views.viewports().len(), count + 1);
    assert_ne!(a.active_view, v);
}

#[test]
fn split_quad_makes_four_equal_views_with_one_linked_cross() {
    let mut a = box_app();
    let ctx = Context::default();
    idle(&mut a, &ctx);
    a.dot_menu_action(Dot::View(0), MenuAction::SplitQuad);
    idle(&mut a, &ctx);
    let views = a.views.viewports();
    assert_eq!(views.len(), 4);
    let mut areas: Vec<Rect> = views.iter().map(|v| a.corner.areas[v]).collect();
    areas.sort_by(|p, q| {
        (p.top(), p.left())
            .partial_cmp(&(q.top(), q.left()))
            .unwrap()
    });
    let [tl, tr, bl, br] = [areas[0], areas[1], areas[2], areas[3]];
    for r in [tr, bl, br] {
        assert!((r.width() - tl.width()).abs() < 2. && (r.height() - tl.height()).abs() < 2.);
    }
    // The bottom-right view is active.
    assert_eq!(a.corner.areas[&a.active_view], br);
    // Drag the column divider in the top row: the bottom row's moves too.
    let x = (tl.right() + tr.left()) / 2.;
    let from = pos2(x, tl.center().y);
    drag(&mut a, &ctx, from, from + vec2(-120., 0.), Modifiers::NONE);
    let mut moved: Vec<Rect> = views.iter().map(|v| a.corner.areas[v]).collect();
    moved.sort_by(|p, q| {
        (p.top(), p.left())
            .partial_cmp(&(q.top(), q.left()))
            .unwrap()
    });
    assert!(moved[0].right() < tl.right() - 60., "{:?}", moved[0]);
    assert!(
        (moved[0].right() - moved[2].right()).abs() < 1.,
        "one linked cross"
    );
    // The lone-view quad (Maximise's Four Views) is the same linked grid.
    let mut b = box_app();
    idle(&mut b, &ctx);
    b.toggle_maximized(0);
    assert_eq!(b.views.viewports().len(), 4);
}

#[test]
fn maximised_views_cant_be_deleted_and_the_menu_maximises() {
    let (mut a, ctx) = two_views();
    a.dot_menu_action(Dot::View(1), MenuAction::Maximize);
    idle(&mut a, &ctx);
    assert!(a.views.is_maximized(1));
    assert!(!a.can_close_view(1));
    a.dot_menu_action(Dot::View(1), MenuAction::Delete);
    assert!(a.views.viewports().contains(&1));
    a.dot_menu_action(Dot::View(1), MenuAction::Maximize);
    idle(&mut a, &ctx);
    assert!(!a.views.is_maximized(1));
}

#[test]
fn locks_stop_dot_drags_and_divider_drags_and_are_saved() {
    let (mut a, ctx) = two_views();
    // Outside UI build mode a dot drag moves nothing and says how to.
    let before = a.views.main.clone();
    let from = dot(&a, Dot::View(0));
    drag_raw(&mut a, &ctx, from, pos2(700., 6.), Modifiers::NONE);
    assert!(a.views.windows.is_empty());
    assert_eq!(a.views.main, before);
    assert!(a.status.contains("UI build mode"), "{}", a.status);
    // The dot's menu still works.
    a.dot_menu_action(Dot::View(0), MenuAction::Duplicate);
    assert_eq!(a.views.viewports().len(), 3);
    // Dividers: a drag on the split between views changes nothing.
    a.execute("layout.lockDividers");
    idle(&mut a, &ctx);
    let areas = a.corner.areas.clone();
    let x = (a.corner.areas[&0].right() + a.corner.areas[&1].left()) / 2.;
    let from = pos2(x, a.corner.areas[&0].center().y);
    drag(&mut a, &ctx, from, from + vec2(150., 0.), Modifiers::NONE);
    assert_eq!(a.corner.areas, areas);
    // Kept per layout in the layouts state file (LAY-48, LAY-60).
    let state =
        std::fs::read_to_string(a.environment.paths().data_root().join("layouts.json")).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state).unwrap();
    let current = a.layouts.current.clone();
    assert_eq!(state["locks"][&current]["dividers"], true);
    // Unlocked, the same divider drag resizes.
    a.execute("layout.lockDividers lock:off");
    assert_eq!(a.dots.locks, LayoutLocks::default());
    drag(&mut a, &ctx, from, from + vec2(150., 0.), Modifiers::NONE);
    assert_ne!(a.corner.areas, areas);
}

#[test]
fn closing_a_floating_window_discards_its_views() {
    let (mut a, ctx) = two_views();
    a.dot_menu_action(Dot::View(1), MenuAction::Detach);
    a.activate_view(1);
    idle(&mut a, &ctx);
    let window = a.views.windows[0].id;
    a.discard_view_window(window);
    idle(&mut a, &ctx);
    assert!(a.views.windows.is_empty());
    assert_eq!(a.views.viewports(), [0]);
    assert!(!a.cameras.contains_key(&1));
    assert_eq!(a.active_view, 0);
}

#[test]
fn a_side_panel_dragged_out_floats_and_dragged_back_docks() {
    let (mut a, ctx) = two_views();
    let release = pos2(700., 6.);
    drag_dot(
        &mut a,
        &ctx,
        Dot::Side(Panel::Items),
        release,
        Modifiers::NONE,
    );
    assert!(a.is_floating(Panel::Items));
    assert_eq!(a.dots.side_positions[&Panel::Items], [release.x, release.y]);
    // Its menu: Detach greyed now, Delete always (side panels stay).
    // Dragged onto the main window's view, it docks home.
    // (Made directly: in tests the floating window's panel covers the
    // main window.)
    a.drop_for_test(
        Dot::Side(Panel::Items),
        Over::Panel(Dot::View(0), Zone::Center),
    );
    assert!(!a.is_floating(Panel::Items));
    idle(&mut a, &ctx);
    let over = a.corner.areas[&0].center();
    // A docked side panel dropped on a view stays in the main window.
    drag_dot(
        &mut a,
        &ctx,
        Dot::Side(Panel::Properties),
        over,
        Modifiers::NONE,
    );
    assert!(!a.is_floating(Panel::Properties));
    // Detach floats it near where it was.
    a.dot_menu_action(Dot::Side(Panel::Toolbox), MenuAction::Detach);
    assert!(a.is_floating(Panel::Toolbox));
    for panel in Panel::ALL {
        a.set_floating(panel, false);
    }
}

/// One view alone in the main window, drawn.
fn one_view() -> (Operandi, Context) {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    let ctx = Context::default();
    idle(&mut a, &ctx);
    (a, ctx)
}

#[test]
fn ctrl_drag_splits_in_all_four_directions() {
    // T01, T02: the new view lands on the side the drag went, half each,
    // active, with the same angle and zoom.
    for (delta, side) in [
        (vec2(0., 80.), Side::Bottom),
        (vec2(80., 4.), Side::Right),
        (vec2(-80., 4.), Side::Left),
        (vec2(4., -80.), Side::Top),
    ] {
        let (mut a, ctx) = one_view();
        a.camera.distance = 23.;
        idle(&mut a, &ctx);
        let whole = a.corner.areas[&0];
        let from = dot(&a, Dot::View(0));
        drag(&mut a, &ctx, from, from + delta, Modifiers::CTRL);
        assert_eq!(a.views.viewports().len(), 2, "{side:?}");
        let new = a.active_view;
        assert_ne!(new, 0, "{side:?}: the new view is active");
        let (old, copy) = (a.corner.areas[&0], a.corner.areas[&new]);
        match side {
            Side::Bottom => assert!(copy.top() >= old.bottom() - 2., "{side:?}"),
            Side::Top => assert!(copy.bottom() <= old.top() + 2., "{side:?}"),
            Side::Right => assert!(copy.left() >= old.right() - 2., "{side:?}"),
            Side::Left => assert!(copy.right() <= old.left() + 2., "{side:?}"),
        }
        assert!(
            (old.area() - copy.area()).abs() < whole.area() * 0.02,
            "{side:?}: halves"
        );
        assert_eq!(a.cameras[&new].view, View::Perspective);
        assert_eq!(a.cameras[&new].distance, 23.);
        assert!(a.views.windows.is_empty());
    }
}

#[test]
fn only_the_direction_at_release_counts() {
    // T03: down first, then swinging right: a right split.
    let (mut a, ctx) = one_view();
    enter_build_mode(&mut a, &ctx);
    let from = dot(&a, Dot::View(0));
    let m = Modifiers::CTRL;
    frame(&mut a, &ctx, vec![Event::PointerMoved(from)], m);
    frame(&mut a, &ctx, vec![pointer(from, true, m)], m);
    for at in [
        vec2(0., 30.),
        vec2(0., 90.),
        vec2(60., 100.),
        vec2(160., 90.),
    ] {
        frame(&mut a, &ctx, vec![Event::PointerMoved(from + at)], m);
    }
    frame(
        &mut a,
        &ctx,
        vec![pointer(from + vec2(160., 90.), false, m)],
        m,
    );
    idle(&mut a, &ctx);
    let new = a.active_view;
    assert!(a.corner.areas[&new].left() >= a.corner.areas[&0].right() - 2.);
}

#[test]
fn ctrl_drag_esc_cancels_and_outside_still_splits() {
    // T04: Esc before release: no change.
    let (mut a, ctx) = one_view();
    let before = a.views.main.clone();
    let from = dot(&a, Dot::View(0));
    let m = Modifiers::CTRL;
    frame(&mut a, &ctx, vec![Event::PointerMoved(from)], m);
    frame(&mut a, &ctx, vec![pointer(from, true, m)], m);
    for step in 1..=4 {
        let at = from + vec2(0., 30. * step as f32);
        frame(&mut a, &ctx, vec![Event::PointerMoved(at)], m);
    }
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(from + vec2(0., 120.), false, m)],
        m,
    );
    idle(&mut a, &ctx);
    assert_eq!(a.views.main, before);
    assert_eq!(a.views.viewports(), [0]);
    // T05: released over the toolbar (outside every panel): a split, never
    // a tear-off.
    drag(&mut a, &ctx, from, pos2(700., 6.), m);
    assert_eq!(a.views.viewports().len(), 2);
    assert!(a.views.windows.is_empty());
}

#[test]
fn ctrl_drag_does_nothing_with_thumbs_locked_but_the_menu_splits() {
    // T06.
    let (mut a, ctx) = one_view();
    a.dots.locks.thumbs = true;
    let from = dot(&a, Dot::View(0));
    drag_raw(&mut a, &ctx, from, from + vec2(0., 120.), Modifiers::CTRL);
    assert_eq!(a.views.viewports(), [0]);
    a.dot_menu_action(Dot::View(0), MenuAction::SplitVertical);
    idle(&mut a, &ctx);
    assert_eq!(a.views.viewports().len(), 2);
}

#[test]
fn a_view_too_small_for_both_halves_isnt_split() {
    // T07: split side by side until a view is under twice the minimum
    // width (120 points).
    let (mut a, ctx) = one_view();
    let mut view = 0;
    while a.corner.areas[&view].width() >= 2. * 120. + 1. {
        a.dot_menu_action(Dot::View(view), MenuAction::SplitHorizontal);
        idle(&mut a, &ctx);
        view = a.active_view;
    }
    let narrow = a.corner.areas[&view];
    assert!(
        narrow.width() >= 120. - 1.,
        "{narrow:?}: every split kept the minimum"
    );
    let count = a.views.viewports().len();
    assert!(
        !a.view_splits(view, Side::Right),
        "Split Horizontal is greyed"
    );
    assert!(a.view_splits(view, Side::Bottom), "it still splits stacked");
    let from = dot(&a, Dot::View(view));
    drag(&mut a, &ctx, from, from + vec2(80., 0.), Modifiers::CTRL);
    assert_eq!(a.views.viewports().len(), count, "the drag is refused");
    a.dot_menu_action(Dot::View(view), MenuAction::SplitHorizontal);
    a.dot_menu_action(Dot::View(view), MenuAction::SplitQuad);
    a.menu_action(&ctx, crate::app_menu::line_action("viewport.split"));
    idle(&mut a, &ctx);
    assert_eq!(a.views.viewports().len(), count, "and so are the menus");
    // Stacked still works.
    drag(&mut a, &ctx, from, from + vec2(0., 80.), Modifiers::CTRL);
    assert_eq!(a.views.viewports().len(), count + 1);
}

#[test]
fn layout_changes_leave_the_undo_history_and_modified_flag_alone() {
    // T11, T12 (VST-39, VST-41).
    let (mut a, ctx) = one_view();
    let original = a.editor.scene.clone();
    a.execute("new mesh");
    idle(&mut a, &ctx);
    let steps = a.editor.undo.len();
    assert!(steps > 0, "the modelling change is a step");
    let dirty = a.dirty();
    assert!(dirty);
    let scene = a.editor.scene.clone();
    let from = dot(&a, Dot::View(0));
    drag(&mut a, &ctx, from, from + vec2(0., 120.), Modifiers::CTRL);
    let new = a.active_view;
    a.dot_menu_action(Dot::View(new), MenuAction::Detach);
    idle(&mut a, &ctx);
    assert_eq!(a.views.windows.len(), 1);
    a.drop_for_test(
        Dot::View(new),
        Over::Panel(Dot::View(0), Zone::Edge(Side::Left)),
    );
    idle(&mut a, &ctx);
    a.drop_for_test(Dot::View(new), Over::Panel(Dot::View(0), Zone::Center));
    a.dot_menu_action(Dot::View(0), MenuAction::Maximize);
    a.dot_menu_action(Dot::View(0), MenuAction::Maximize);
    a.dot_menu_action(Dot::View(new), MenuAction::Copy);
    idle(&mut a, &ctx);
    let window = a.views.windows[0].id;
    a.discard_view_window(window);
    idle(&mut a, &ctx);
    assert_eq!(a.editor.undo.len(), steps, "no layout step in the history");
    assert_eq!(a.dirty(), dirty, "the scene's modified flag is unchanged");
    assert!(same_document(&a.editor.scene, &scene));
    // Undo takes back the modelling change; the layout stays as it is.
    let layout = a.views.main.clone();
    a.execute("undo");
    idle(&mut a, &ctx);
    assert!(
        same_document(&a.editor.scene, &original),
        "the new mesh is undone"
    );
    assert_eq!(a.views.main, layout);
    // And a split on its own never marks a saved scene modified.
    a.saved = a.editor.scene.clone();
    assert!(!a.dirty());
    a.dot_menu_action(Dot::View(0), MenuAction::SplitVertical);
    idle(&mut a, &ctx);
    assert!(!a.dirty());
}

#[test]
fn detach_opens_over_the_panels_old_place() {
    // VST-31 (tests list no screens, so no clamp applies).
    let (mut a, ctx) = two_views();
    let area = a.corner.areas[&1];
    a.dot_menu_action(Dot::View(1), MenuAction::Detach);
    idle(&mut a, &ctx);
    let window = &a.views.windows[0];
    assert_eq!(window.position, Some([area.left(), area.top()]));
    assert!((window.size[0] - area.width()).abs() < 1.);
    assert!((window.size[1] - area.height()).abs() < 1.);
}

#[test]
fn a_torn_off_window_splits_like_the_main_one() {
    // T14: Ctrl+drag the torn-off view's dot down: two stacked views in
    // that window, not the main one.
    let (mut a, ctx) = two_views();
    a.dot_menu_action(Dot::View(1), MenuAction::Detach);
    idle(&mut a, &ctx);
    let window = a.views.windows[0].id;
    let from = dot(&a, Dot::View(1));
    drag(&mut a, &ctx, from, from + vec2(0., 100.), Modifiers::CTRL);
    assert_eq!(a.views.group_viewports(Some(window)).len(), 2);
    assert_eq!(a.views.group_viewports(None), [0]);
    let new = a.active_view;
    assert_eq!(a.views.window_of(Pane::Viewport(new)), Some(window));
}

#[test]
fn a_split_copy_starts_unmaximised() {
    // VST-15.
    let (mut a, ctx) = two_views();
    a.dot_menu_action(Dot::View(0), MenuAction::Maximize);
    idle(&mut a, &ctx);
    assert!(a.views.is_maximized(0));
    a.dot_menu_action(Dot::View(0), MenuAction::SplitHorizontal);
    idle(&mut a, &ctx);
    let new = a.active_view;
    assert_ne!(new, 0);
    assert!(!a.views.is_maximized(new));
}

#[test]
fn uv_views_split_and_copy_as_uv_views() {
    // T16 (the layout side; the UV view draws once its build lands).
    let (mut a, ctx) = one_view();
    let uv = a.views.new_viewport_key();
    a.views
        .insert_beside(Pane::Uv(uv), Pane::Viewport(0), Side::Left)
        .unwrap();
    a.dot_menu_action(Dot::View(uv), MenuAction::SplitHorizontal);
    let copy = a.active_view;
    assert_ne!(copy, uv);
    assert_eq!(a.views.view_pane(copy), Some(Pane::Uv(copy)));
    a.dot_menu_action(Dot::View(uv), MenuAction::Copy);
    idle(&mut a, &ctx);
    let window = a.views.windows[0].id;
    assert!(
        a.views
            .group_views(Some(window))
            .iter()
            .all(|&k| matches!(a.views.view_pane(k), Some(Pane::Uv(_))))
    );
}

/// Jason, 2026-11-19: numpad 0 (quad / maximise / restore) on a docked
/// layout replaced the whole dock tree, so the side panels vanished. The
/// toggle may only rearrange views; every panel stays, with its size.
/// Jason, 2026-11-19: maximise / restore / quad / split in some orders
/// left a blank cell beside the view with no way to close it. In random
/// sequences (seeded, so a failure repeats), the views always cover the
/// whole view area: nothing is left blank.
#[test]
fn docking_view_toggles_in_random_orders_never_leave_a_blank_cell() {
    use crate::layouts_tests::{data_root, launch};

    for seed in 1..=12u64 {
        let root = data_root();
        let ctx = Context::default();
        let mut a = launch(&root, &ctx);
        idle(&mut a, &ctx);
        let area = |a: &Operandi| {
            a.corner
                .areas
                .values()
                .fold(Rect::NOTHING, |r, x| r.union(*x))
        };
        let covered = |a: &Operandi| a.corner.areas.values().map(|r| r.area()).sum::<f32>();
        let start = area(&a);
        let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut steps = Vec::new();
        for _ in 0..14 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let pick = (state >> 33) % 4;
            let views = a.views.viewports();
            let view = views[(state >> 40) as usize % views.len()];
            match pick {
                0 | 1 => {
                    steps.push(format!("toggle {view}"));
                    a.toggle_maximized(view);
                }
                2 => {
                    steps.push("split".into());
                    a.split_view();
                }
                _ => {
                    steps.push(format!("toggle active {}", a.active_view));
                    let active = a.active_view;
                    a.toggle_maximized(active);
                }
            }
            idle(&mut a, &ctx);
            let now = area(&a);
            let gaps = (a.corner.areas.len().saturating_sub(1)) as f32 * 40. * now.height();
            assert!(
                (now.width() - start.width()).abs() < 2.
                    && (now.height() - start.height()).abs() < 2.,
                "seed {seed}: the views span the view area ({now:?} vs {start:?}) after {steps:?}\n{:#?}",
                a.views.main.tiles
            );
            assert!(
                covered(&a) + gaps >= now.area() - 1.,
                "seed {seed}: a cell is blank after {steps:?}: {:?}",
                a.corner.areas
            );
        }
    }
}

#[test]
fn docking_quad_and_maximise_toggles_keep_every_panel() {
    use crate::layouts_tests::{data_root, launch};

    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    idle(&mut a, &ctx);
    let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
    drag_dot(&mut a, &ctx, Dot::ToolProperties, to, Modifiers::NONE);
    assert!(a.layouts.mixed_dock, "{}", a.status);
    idle(&mut a, &ctx);
    let others = |a: &Operandi| -> Vec<Pane> {
        let mut v: Vec<Pane> = a
            .views
            .panes()
            .into_iter()
            .filter(|p| p.view_key().is_none())
            .collect();
        v.sort();
        v
    };
    let before = others(&a);
    assert!(before.contains(&Pane::ToolProperties));
    let size = dock_leaf(&a, Pane::ToolProperties).size();
    // Quad, maximise, restore, maximise, restore, maximise.
    let mut quad = None;
    for press in 0..6 {
        let view = a.active_view;
        a.toggle_maximized(view);
        idle(&mut a, &ctx);
        assert_eq!(others(&a), before, "press {press}: {}", a.status);
        let now = dock_leaf(&a, Pane::ToolProperties);
        assert!(now.width() > 1. && now.height() > 1., "press {press}");
        // The whole view area: every view's rect together.
        let area = a
            .corner
            .areas
            .values()
            .fold(Rect::NOTHING, |r, a| r.union(*a));
        if press == 0 {
            assert_eq!(a.views.viewports().len(), 4, "the view became a quad");
            quad = Some((area, a.corner.areas.clone()));
        } else {
            assert_eq!(now.width(), size.x, "press {press}: same width");
            let (quad_area, quad_rects) = quad.clone().unwrap();
            if press % 2 == 1 {
                // Jason, 2026-11-19: the maximised view fills the whole
                // quad area, not one column of it.
                assert_eq!(a.corner.areas.len(), 1, "press {press}");
                // Within float rounding of the summed split shares.
                assert!(
                    (area.min - quad_area.min).length() < 0.01
                        && (area.max - quad_area.max).length() < 0.01,
                    "press {press}: maximised fills the area ({area:?} vs {quad_area:?})"
                );
            } else {
                assert_eq!(
                    a.corner.areas, quad_rects,
                    "press {press}: quad restored exactly"
                );
            }
        }
    }
    check_reopen(&a);
}

/// Jason, 2026-11-19: docking beside the Toolbox (stacked over Tool
/// Properties) must split only the Toolbox's cell; Tool Properties keeps
/// its full width. Same the other way round.
#[test]
fn docking_beside_a_stacked_panel_splits_only_that_panel() {
    use crate::layouts_tests::{data_root, launch};

    for (target, other) in [
        (Dot::Side(Panel::Toolbox), Pane::ToolProperties),
        (Dot::ToolProperties, Pane::Toolbox),
    ] {
        let root = data_root();
        let ctx = Context::default();
        let mut a = launch(&root, &ctx);
        idle(&mut a, &ctx);
        let width = a
            .dot_panel_rect(if other == Pane::Toolbox {
                Dot::Side(Panel::Toolbox)
            } else {
                Dot::ToolProperties
            })
            .unwrap()
            .width();
        let to = a.compass_point(target, Zone::Edge(Side::Right));
        drag_dot(
            &mut a,
            &ctx,
            Dot::Side(Panel::Properties),
            to,
            Modifiers::NONE,
        );
        assert!(a.layouts.mixed_dock, "{target:?}: {}", a.status);
        idle(&mut a, &ctx);
        let moved = dock_leaf(&a, Pane::Properties);
        let beside = dock_leaf(&a, a.dock_pane(target));
        let kept = dock_leaf(&a, other);
        // The column widens for the newcomer: the other stacked panel
        // never gets narrower, and the target keeps its own width.
        assert!(
            kept.width() >= width - 1.,
            "{target:?}: {other:?} kept its width ({} vs {width})",
            kept.width()
        );
        assert!(
            beside.width() >= width - 2.,
            "{target:?}: the target kept its width ({} vs {width})",
            beside.width()
        );
        assert!(
            moved.left() >= beside.right() - 1.,
            "{target:?}: docked to its right"
        );
        assert!(
            moved.y_range().max <= beside.bottom() + 1. && moved.top() >= beside.top() - 1.,
            "{target:?}: only beside that panel: moved {moved:?} beside {beside:?} status {}",
            a.status
        );
        check_reopen(&a);
    }
}

/// Jason, 2026-11-19: Tool Tabs docked to the right of the Toolbox
/// reserved a wide column with empty space. The strip is exactly as wide
/// as its tabs, and neither the Toolbox nor Tool Properties gets narrower.
#[test]
fn docking_tool_tabs_beside_the_toolbox_is_a_tab_wide_strip() {
    use crate::layouts_tests::{data_root, launch};

    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    idle(&mut a, &ctx);
    let toolbox = a.dot_panel_rect(Dot::Side(Panel::Toolbox)).unwrap().width();
    let props = a.dot_panel_rect(Dot::ToolProperties).unwrap().width();
    let to = a.compass_point(Dot::Side(Panel::Toolbox), Zone::Edge(Side::Right));
    drag_dot(&mut a, &ctx, Dot::ToolTabs, to, Modifiers::NONE);
    idle(&mut a, &ctx);
    assert_eq!(a.layouts.tool_tabs_horizontal, Some(false), "{}", a.status);
    let strip = dock_leaf(&a, Pane::ToolTabs);
    let tools = dock_leaf(&a, Pane::Toolbox);
    assert!(
        (strip.width() - toolbox::VERTICAL_TOOL_TABS_WIDTH).abs() < 0.01,
        "strip width {}",
        strip.width()
    );
    assert!(strip.left() >= tools.right() - 1., "to the Toolbox's right");
    assert!(
        tools.width() >= toolbox - 2.,
        "Toolbox {} vs {toolbox}",
        tools.width()
    );
    let kept = dock_leaf(&a, Pane::ToolProperties).width();
    assert!(kept >= props - 1., "Tool Properties {kept} vs {props}");
    check_reopen(&a);
}

fn dock_leaf(a: &Operandi, pane: Pane) -> Rect {
    let tiles = &a.views.main.tiles;
    let id = tiles
        .find_pane(&pane)
        .unwrap_or_else(|| panic!("{pane:?} isn't in the main tree"));
    tiles.rect(id).unwrap()
}

#[test]
fn docking_layout_moves_tool_properties_beside_only_the_target_view_and_reopens() {
    use crate::layouts_tests::{data_root, frames, launch};

    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    a.split_view();
    idle(&mut a, &ctx);
    let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
    let other = a.camera.clone();
    let scene = a.editor.scene.clone();
    drag_dot(&mut a, &ctx, Dot::ToolProperties, to, Modifiers::NONE);
    assert!(a.layouts.mixed_dock, "{}", a.status);
    let props = dock_leaf(&a, Pane::ToolProperties);
    let target = dock_leaf(&a, Pane::Viewport(0));
    let sibling = dock_leaf(&a, Pane::Viewport(1));
    assert_eq!(props.x_range(), target.x_range());
    assert!(props.top() > target.bottom());
    assert!(sibling.left() > props.right());
    assert_eq!(target.top(), sibling.top());
    assert_eq!(props.bottom(), sibling.bottom());
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.camera.placement(), other.placement());

    a.upper_frame.show_page(1);
    frames(&mut a, &ctx, 2);
    let before = a.capture_layout().unwrap();
    let ctx = Context::default();
    let mut reopened = launch(&root, &ctx);
    frames(&mut reopened, &ctx, 2);
    assert!(before.same_as(&reopened.capture_layout().unwrap()));
    assert_eq!(reopened.upper_frame.front(), a.upper_frame.front());
    assert_eq!(dock_leaf(&reopened, Pane::ToolProperties), props);
    assert_eq!(dock_leaf(&reopened, Pane::Viewport(0)), target);
    assert_eq!(dock_leaf(&reopened, Pane::Viewport(1)), sibling);
}

#[test]
fn docking_layout_divider_clamps_at_a_moved_panel_minimum_and_can_be_locked() {
    let (mut a, ctx) = two_views();
    let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
    drag_dot(&mut a, &ctx, Dot::ToolProperties, to, Modifiers::NONE);
    assert!(a.layouts.mixed_dock, "{}", a.status);
    let props = dock_leaf(&a, Pane::ToolProperties);
    let from = pos2(props.center().x, props.top() - 0.5);
    let to = pos2(from.x, props.bottom() - 8.);
    let scene = a.editor.scene.clone();
    a.dots.locks.dividers = true;
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    assert_eq!(dock_leaf(&a, Pane::ToolProperties), props);
    a.dots.locks.dividers = false;
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    let height = dock_leaf(&a, Pane::ToolProperties).height();
    assert!(
        (height - Pane::ToolProperties.min_size()[1]).abs() <= 1.,
        "{height}"
    );
    assert_eq!(a.editor.scene, scene);
}

#[test]
fn docking_layout_tabs_the_toolbox_with_a_tab_group_without_losing_either() {
    let (mut a, ctx) = two_views();
    a.upper_frame.show_page(1);
    let at = a.compass_point(Dot::Side(Panel::Items), Zone::Center);
    let from = dot(&a, Dot::Side(Panel::Toolbox));
    drag(&mut a, &ctx, from, at, Modifiers::NONE);
    assert!(a.layouts.mixed_dock, "{}", a.status);
    assert_eq!(
        tab_panes(&a, Pane::Toolbox),
        ([Pane::Items, Pane::Toolbox].to_vec(), Pane::Toolbox)
    );
    assert!(dock_leaf(&a, Pane::Toolbox).contains(at));
    assert_eq!(
        a.upper_frame.front(),
        Some(panel_registry::PanelKind::ShaderTree)
    );
    assert!(a.toolbox_rects.contains_key("Move"));
    assert!(
        !a.visible_scene_lists
            .contains_key(&panel_registry::PanelKind::ShaderTree)
    );
    check_reopen(&a);
}

fn tab_panes(a: &Operandi, pane: Pane) -> (Vec<Pane>, Pane) {
    let tiles = &a.views.main.tiles;
    let group = a.views.dock_slot(pane).unwrap();
    let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) = tiles.get(group)
    else {
        panic!("expected tabs for {pane:?}");
    };
    (
        tabs.children
            .iter()
            .map(|&id| *tiles.get_pane(&id).unwrap())
            .collect(),
        *tiles.get_pane(&tabs.active.unwrap()).unwrap(),
    )
}

fn check_reopen(a: &Operandi) {
    use operandi_layout::WindowLayout;
    let saved = a.capture_layout().unwrap();
    let mut reopened = box_app();
    reopened
        .apply_layout_doc(&WindowLayout::from_json(&saved.to_json()).unwrap())
        .unwrap();
    assert!(saved.same_as(&reopened.capture_layout().unwrap()));
}

/// Press a panel's dot in UI build mode and hold it over `to`;
/// `release_dot` drops it and leaves the mode.
fn hold_dot(a: &mut Operandi, ctx: &Context, source: Dot, to: Pos2) {
    if !a.dots.build_mode {
        a.dots.build_mode = true;
        idle(a, ctx);
    }
    let from = dot(a, source);
    frame(a, ctx, vec![Event::PointerMoved(from)], Modifiers::NONE);
    frame(
        a,
        ctx,
        vec![pointer(from, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(a, ctx, vec![Event::PointerMoved(to)], Modifiers::NONE);
    assert!(a.dots.drag.is_some());
}

fn release_dot(a: &mut Operandi, ctx: &Context, to: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(to)], Modifiers::NONE);
    frame(
        a,
        ctx,
        vec![pointer(to, false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    idle(a, ctx);
    a.dots.build_mode = false;
}

#[test]
fn docking_compass_always_targets_the_exact_view_and_survives_reopen() {
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        let (mut a, ctx) = two_views();
        let source = Dot::ToolProperties;
        let compass = DockTarget::Compass(Dot::View(0), Zone::Edge(side));
        let to = a.compass_point(Dot::View(0), Zone::Edge(side));
        hold_dot(&mut a, &ctx, source, to);
        assert_eq!(a.dock_hit(source, to), Some(compass));
        release_dot(&mut a, &ctx, to);
        let props = dock_leaf(&a, Pane::ToolProperties);
        let view = dock_leaf(&a, Pane::Viewport(0));
        match side {
            Side::Left => assert!(props.right() < view.left()),
            Side::Right => assert!(props.left() > view.right()),
            Side::Top => assert!(props.bottom() < view.top()),
            Side::Bottom => assert!(props.top() > view.bottom()),
        }
        assert!(dock_leaf(&a, Pane::Viewport(1)).left() > view.left().max(props.left()));
        check_reopen(&a);
    }
}

#[test]
fn docking_outer_arrows_dock_against_the_whole_work_area_and_reopen() {
    use egui_tiles::{Container, LinearDir, Tile};
    for index in 0..4 {
        let (mut a, ctx) = two_views();
        let source = Dot::View(1);
        let at = a.compass_point(Dot::View(0), Zone::Center);
        hold_dot(&mut a, &ctx, source, at);
        let (target, at) = a.dock_test_strips()[index];
        assert_eq!(a.dock_hit(source, at), Some(target));
        release_dot(&mut a, &ctx, at);
        assert!(a.layouts.mixed_dock, "{}", a.status);
        let pane = dock_leaf(&a, Pane::Viewport(1));
        let leaves = [
            Pane::Toolbox,
            Pane::ToolProperties,
            Pane::Viewport(0),
            Pane::Items,
            Pane::Properties,
        ]
        .map(|kind| dock_leaf(&a, kind));
        match index {
            0 => assert!(pane.bottom() < leaves.iter().map(|r| r.top()).reduce(f32::min).unwrap()),
            1 => assert!(pane.top() > leaves.iter().map(|r| r.bottom()).reduce(f32::max).unwrap()),
            2 => assert!(pane.right() < leaves.iter().map(|r| r.left()).reduce(f32::min).unwrap()),
            3 => assert!(pane.left() > leaves.iter().map(|r| r.right()).reduce(f32::max).unwrap()),
            _ => unreachable!(),
        }
        let tree = &a.views.main;
        let id = tree.tiles.find_pane(&Pane::Viewport(1)).unwrap();
        let parent = tree.tiles.parent_of(id).unwrap();
        let Some(Tile::Container(Container::Linear(split))) = tree.tiles.get(parent) else {
            panic!("expected split for {target:?}");
        };
        let expected = if matches!(index, 0 | 1) {
            LinearDir::Vertical
        } else {
            LinearDir::Horizontal
        };
        assert_eq!(split.dir, expected);
        check_reopen(&a);
    }
}

#[test]
fn docking_guide_pops_up_only_over_the_panel_under_the_pointer() {
    let (mut a, ctx) = two_views();
    let source = Dot::ToolProperties;
    let over_view = a.compass_point(Dot::View(0), Zone::Center);
    hold_dot(&mut a, &ctx, source, over_view);
    assert_eq!(a.dock_guide_at(Some(source), over_view), Some(Dot::View(0)));
    let other = a.compass_point(Dot::View(1), Zone::Center);
    assert_eq!(a.dock_guide_at(Some(source), other), Some(Dot::View(1)));
    // Over the dragged panel itself there is no guide and no target.
    let own = a.dot_panel_rect(source).unwrap().center();
    assert_eq!(a.dock_guide_at(Some(source), own), None);
    assert_eq!(a.dock_hit(source, own), None);
    // Only the cross's buttons and the outer arrows are targets: the rest
    // of the hovered panel is not.
    let view = a.dot_panel_rect(Dot::View(0)).unwrap();
    let off = view.left_top() + vec2(40., 60.);
    assert_eq!(a.dock_guide_at(Some(source), off), Some(Dot::View(0)));
    assert_eq!(a.dock_hit(source, off), None);
    release_dot(&mut a, &ctx, over_view);
}

#[test]
fn docking_drop_off_every_target_floats_the_panel() {
    for source in [Dot::ToolProperties, Dot::View(1)] {
        let (mut a, ctx) = two_views();
        let view = a.dot_panel_rect(Dot::View(0)).unwrap();
        let off = view.left_top() + vec2(40., 60.);
        hold_dot(&mut a, &ctx, source, off);
        assert_eq!(a.dock_hit(source, off), None);
        assert_eq!(a.dots.drag.as_ref().unwrap().over, Over::Outside);
        release_dot(&mut a, &ctx, off);
        let pane = a.dock_pane(source);
        assert!(
            a.views.window_of(pane).is_some(),
            "{source:?}: {}",
            a.status
        );
        check_reopen(&a);
    }
    // The main window keeps its last view: off target, nothing happens.
    let (mut a, ctx) = one_view();
    let view = a.dot_panel_rect(Dot::View(0)).unwrap();
    let off = view.left_top() + vec2(60., 60.);
    hold_dot(&mut a, &ctx, Dot::View(0), off);
    assert_eq!(a.dots.drag.as_ref().unwrap().over, Over::Nothing);
    release_dot(&mut a, &ctx, off);
    assert!(a.views.window_of(Pane::Viewport(0)).is_none());
}

#[test]
fn docking_tab_row_and_centre_merge_in_order_with_the_dragged_front_tab() {
    use panel_registry::PanelKind as K;
    for center in [false, true] {
        let (mut a, ctx) = two_views();
        a.upper_frame.show_page(1);
        let source = Dot::Side(Panel::Items);
        let target = Dot::Side(Panel::Properties);
        idle(&mut a, &ctx);
        let row = a.lower_frame.rects[0];
        let insert = pos2(row.left() + 2., row.center().y);
        let at = if center {
            a.compass_point(target, Zone::Center)
        } else {
            insert
        };
        hold_dot(&mut a, &ctx, source, at);
        assert_eq!(
            a.dock_hit(source, at),
            Some(if center {
                DockTarget::Compass(target, Zone::Center)
            } else {
                DockTarget::PanelTabs(target, 0)
            })
        );
        let mut expected = a.lower_frame.tabs.clone();
        let at_index = if center { expected.len() } else { 0 };
        expected.splice(at_index..at_index, a.upper_frame.tabs.iter().copied());
        release_dot(&mut a, &ctx, at);
        assert_eq!(a.lower_frame.tabs, expected);
        assert_eq!(a.lower_frame.front(), Some(K::ShaderTree));
        assert!(a.upper_frame.tabs.is_empty());
        assert!(!a.views.panes().contains(&Pane::Items));
        check_reopen(&a);
    }
}

#[test]
fn docking_tab_merge_keeps_destination_duplicate_and_activates_it() {
    use panel_registry::PanelKind as K;
    for center in [false, true] {
        let (mut a, ctx) = two_views();
        a.upper_frame.tabs = vec![K::Items, K::ShaderTree, K::Groups];
        a.upper_frame.show_page(1);
        a.lower_frame.tabs = vec![K::Properties, K::ShaderTree];
        idle(&mut a, &ctx);
        let source = Dot::Side(Panel::Items);
        let dest = Dot::Side(Panel::Properties);
        let row = a.lower_frame.rects[0];
        let at = if center {
            a.compass_point(dest, Zone::Center)
        } else {
            pos2(row.left() + 2., row.center().y)
        };
        drag_dot(&mut a, &ctx, source, at, Modifiers::NONE);
        let expected = if center {
            [K::Properties, K::ShaderTree, K::Items, K::Groups]
        } else {
            [K::Items, K::Groups, K::Properties, K::ShaderTree]
        };
        assert_eq!(a.lower_frame.tabs, expected);
        assert_eq!(a.lower_frame.front(), Some(K::ShaderTree));
        assert!(a.upper_frame.tabs.is_empty());
        check_reopen(&a);
    }
}

#[test]
fn docking_tab_groups_beside_each_other_keeps_both_rows() {
    use panel_registry::PanelKind as K;
    for strip in [None, Some(0)] {
        let (mut a, ctx) = two_views();
        a.upper_frame.tabs = vec![K::Items, K::ShaderTree, K::Groups];
        a.upper_frame.show_page(1);
        a.lower_frame.tabs = vec![K::Properties, K::ShaderTree];
        idle(&mut a, &ctx);
        let source = Dot::Side(Panel::Items);
        let dest = Dot::Side(Panel::Properties);
        let mut at = a.compass_point(dest, Zone::Edge(Side::Top));
        hold_dot(&mut a, &ctx, source, at);
        let target = if let Some(index) = strip {
            let (target, point) = a.dock_test_strips()[index];
            assert!(matches!((index, target), (0, DockTarget::Edge(..))));
            at = point;
            target
        } else {
            DockTarget::Compass(dest, Zone::Edge(Side::Top))
        };
        assert_eq!(a.dock_hit(source, at), Some(target));
        release_dot(&mut a, &ctx, at);
        assert!(a.layouts.mixed_dock, "{}", a.status);
        assert_ne!(
            a.views.dock_slot(Pane::Items),
            a.views.dock_slot(Pane::Properties)
        );
        assert_eq!(a.upper_frame.tabs, [K::Items, K::ShaderTree, K::Groups]);
        assert_eq!(a.upper_frame.front(), Some(K::ShaderTree));
        assert_eq!(a.lower_frame.tabs, [K::Properties, K::ShaderTree]);
        assert_eq!(a.lower_frame.front(), Some(K::Properties));
    }
}

#[test]
fn docking_hidden_shader_tab_does_not_suppress_viewport_delete() {
    use panel_registry::PanelKind as K;
    let (mut a, ctx) = two_views();
    a.upper_frame.show_page(1);
    idle(&mut a, &ctx);
    // The active Shader Tree tab is now hidden behind another dock tab.
    let source = Dot::Side(Panel::Toolbox);
    let at = a.compass_point(Dot::Side(Panel::Items), Zone::Center);
    drag_dot(&mut a, &ctx, source, at, Modifiers::NONE);
    assert_eq!(a.upper_frame.front(), Some(K::ShaderTree));
    assert!(!a.visible_scene_lists.contains_key(&K::ShaderTree));
    // A focus ID left by the hidden Shader Tree must not route Delete either.
    ctx.memory_mut(|m| m.request_focus(Id::new("scene-list-focus")));
    a.scene_focus_tab = Some(K::ShaderTree);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    let polygons = a.editor.scene.active().unwrap().mesh.faces.len();
    let view = a.corner.areas[&0].center();
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(view), crate::tests::key(Key::Delete)],
        Modifiers::NONE,
    );
    assert_eq!(a.editor.scene.items.len(), 1);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        polygons - 1
    );
}

#[test]
fn docking_tree_tab_row_inserts_at_the_pointer_position() {
    let (mut a, ctx) = two_views();
    let at = a.compass_point(Dot::View(1), Zone::Center);
    drag_dot(&mut a, &ctx, Dot::View(0), at, Modifiers::NONE);
    let row = a.views.dock_slot(Pane::Viewport(0)).unwrap();
    let source = Dot::ToolProperties;
    let center = a.compass_point(Dot::View(0), Zone::Center);
    hold_dot(&mut a, &ctx, source, center);
    let at = a.dock_tree_tab_point(row, 1);
    assert_eq!(a.dock_hit(source, at), Some(DockTarget::TreeTabs(row, 1)));
    release_dot(&mut a, &ctx, at);
    assert_eq!(
        tab_panes(&a, Pane::ToolProperties),
        (
            vec![Pane::Viewport(1), Pane::ToolProperties, Pane::Viewport(0)],
            Pane::ToolProperties
        )
    );
    check_reopen(&a);
}

#[test]
fn docking_targets_are_absent_from_shipped_layouts_at_rest() {
    let root = crate::layouts_tests::data_root();
    let ctx = Context::default();
    let mut a = crate::layouts_tests::launch(&root, &ctx);
    for name in ["Model", "UV"] {
        a.execute(&format!("layout.restore {name}"));
        assert_eq!(a.layouts.current, name);
        let expected = a.capture_layout().unwrap();
        idle(&mut a, &ctx);
        assert_eq!(a.layouts.mixed_dock, name == "Model");
        assert!(a.dock_slots_at_rest());
        assert!(expected.same_as(&a.capture_layout().unwrap()));
    }
}

#[test]
fn a_sidebar_tab_tears_out_without_moving_its_group() {
    use crate::panel_registry::PanelKind as K;
    let (mut a, ctx) = one_view();
    let original = a.upper_frame.tabs.clone();
    let index = original.iter().position(|&k| k == K::ShaderTree).unwrap();
    let from = a.upper_frame.rects[index].center();
    let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    assert_eq!(
        a.upper_frame.tabs,
        original
            .into_iter()
            .filter(|&k| k != K::ShaderTree)
            .collect::<Vec<_>>()
    );
    assert_eq!(a.upper_frame.front(), Some(K::Items));
    let (&key, slot) = a.views.dock_tabs.iter().next().expect("standalone panel");
    assert_eq!(slot.kind, "Shader Tree");
    assert!(a.dots.rects.contains_key(&Dot::Tab(key)));
    assert!(a.front_panel(K::ShaderTree));
    check_reopen(&a);
}

#[test]
fn a_sidebar_tab_tear_out_escape_leaves_order_and_active_tab_intact() {
    let (mut a, ctx) = one_view();
    enter_build_mode(&mut a, &ctx);
    let before = a.capture_layout().unwrap();
    let from = a.upper_frame.rects[1].center();
    let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(from, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)], Modifiers::NONE);
    assert!(!a.dock_slots_at_rest(), "tear-out preview");
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(to, false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    idle(&mut a, &ctx);
    assert!(a.capture_layout().unwrap().same_as(&before));
    assert!(a.dock_slots_at_rest());
}

#[test]
fn a_tree_tab_tears_out_without_moving_the_other_views() {
    let (mut a, ctx) = two_views();
    a.drop_for_test(
        Dot::View(1),
        Over::Dock(DockTarget::Compass(Dot::View(0), Zone::Center)),
    );
    idle(&mut a, &ctx);
    let row = a.views.dock_slot(Pane::Viewport(0)).unwrap();
    let from = a.dock_tree_tab_point(row, 1);
    let to = a.compass_point(Dot::View(1), Zone::Edge(Side::Bottom));
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    assert_ne!(
        a.views.dock_slot(Pane::Viewport(0)),
        a.views.dock_slot(Pane::Viewport(1))
    );
    assert!(a.views.windows.is_empty());
    assert!(a.corner.areas.contains_key(&0) && a.corner.areas.contains_key(&1));
}

#[test]
fn tool_properties_floats_from_its_menu_and_docks_back_at_compass() {
    let (mut a, ctx) = two_views();
    // Float from the panel menu; the title drag runs in UI build mode.
    a.dot_menu_action(Dot::ToolProperties, MenuAction::Detach);
    enter_build_mode(&mut a, &ctx);
    let id = a
        .views
        .window_of(Pane::ToolProperties)
        .expect("Tool Properties floated");
    assert!(a.views.window_of(Pane::ToolProperties) == Some(id));
    // The embedded test window sits under the main panels' build-mode
    // covers, so the drop is aimed directly (a native window has its own).
    let target = DockTarget::Compass(Dot::View(1), Zone::Edge(Side::Bottom));
    a.drop_for_test(Dot::ToolProperties, Over::Dock(target));
    idle(&mut a, &ctx);
    assert!(a.views.windows.is_empty());
    assert!(dock_leaf(&a, Pane::ToolProperties).top() >= dock_leaf(&a, Pane::Viewport(1)).bottom());
}

#[test]
fn a_sidebar_tab_dropped_outside_reopens_with_its_own_window() {
    use crate::panel_registry::PanelKind as K;
    let (mut a, ctx) = one_view();
    let from = a.upper_frame.rects[1].center();
    let out = pos2(SIZE.x + 40., 280.);
    drag(&mut a, &ctx, from, out, Modifiers::NONE);
    assert_eq!(a.upper_frame.tabs, [K::Items, K::Groups, K::MeshStack]);
    let (&key, slot) = a
        .views
        .dock_tabs
        .iter()
        .next()
        .expect("detached Shader Tree");
    assert_eq!(slot.kind, "Shader Tree");
    assert!(a.views.window_of(Pane::DockTab(key)).is_some());

    a.persist_layout_file();
    let saved = std::fs::read_to_string(a.layout_file("Model").unwrap()).unwrap();
    let doc = operandi_layout::WindowLayout::from_json(&saved).unwrap();
    assert_eq!(doc.windows.len(), 1);
    assert_eq!(doc.windows[0].position, a.views.windows[0].position);
    let mut reopened = box_app();
    reopened.apply_layout_doc(&doc).unwrap();
    assert!(doc.same_as(&reopened.capture_layout().unwrap()));

    let id = reopened.views.windows[0].id;
    reopened
        .apply_window_drop(
            crate::floating::DockWindow::Tree(id),
            DockTarget::PanelTabs(Dot::Side(Panel::Items), 1),
        )
        .unwrap();
    assert!(reopened.views.windows.is_empty());
    assert!(reopened.views.dock_tabs.is_empty());
    assert_eq!(
        reopened.upper_frame.tabs,
        [K::Items, K::ShaderTree, K::Groups, K::MeshStack]
    );
    assert_eq!(reopened.upper_frame.front(), Some(K::ShaderTree));
}

#[test]
fn a_grip_floats_a_mixed_tab_group_and_returns_all_its_tabs() {
    let (mut a, ctx) = two_views();
    a.drop_for_test(
        Dot::ToolProperties,
        Over::Dock(DockTarget::Compass(Dot::View(0), Zone::Center)),
    );
    idle(&mut a, &ctx);
    let group = tab_panes(&a, Pane::ToolProperties);
    drag_dot(
        &mut a,
        &ctx,
        Dot::ToolProperties,
        pos2(SIZE.x + 35., 380.),
        Modifiers::NONE,
    );
    assert_eq!(a.views.windows.len(), 1);
    let id = a.views.windows[0].id;
    for pane in &group.0 {
        assert_eq!(a.views.window_of(*pane), Some(id));
    }
    check_reopen(&a);
    let root = a.views.main.root.unwrap();
    a.apply_window_drop(
        crate::floating::DockWindow::Tree(id),
        DockTarget::Edge(root, Side::Bottom),
    )
    .unwrap();
    assert_eq!(tab_panes(&a, Pane::ToolProperties), group);
    assert!(a.views.windows.is_empty());
}

#[test]
fn docking_tool_tabs_legacy_layouts_keep_the_vertical_strip_and_tool_positions() {
    let stock = layouts::Layouts::read(
        &[
            ("model.json", layouts::LEGACY_MODEL),
            ("uv.json", include_str!("../assets/layouts/uv.json")),
        ],
        None,
    );
    for entry in stock.list {
        let (mut a, ctx) = one_view();
        let name = entry.name();
        a.apply_layout_doc(entry.doc()).unwrap();
        idle(&mut a, &ctx);
        assert!(!a.layouts.mixed_dock, "{name}");
        assert_eq!(a.layouts.tool_tabs_horizontal, None, "{name}");
        assert!(!a.views.panes().contains(&Pane::ToolTabs), "{name}");
        assert!(a.dots.rects.contains_key(&Dot::ToolTabs), "{name}");
        let create = a.toolbox_rects["Category Create"];
        let select = a.toolbox_rects["Category Select"];
        assert_eq!(create.right(), select.right(), "{name}");
        assert!(create.bottom() < select.top(), "{name}");
        assert!(
            (create.right() - a.toolbox_rects["New Mesh"].right()).abs() <= 1.,
            "{name}"
        );
        assert!(
            create.top() >= a.toolbox_rects["New Mesh"].bottom(),
            "{name}"
        );
        assert!(
            select.bottom() < a.toolbox_rects["Tool Properties"].top(),
            "{name}"
        );
        assert!(
            !a.toolbox_rects.contains_key("Scroll tool tabs right"),
            "{name}"
        );
    }
}

#[test]
fn docking_tool_tabs_follow_each_work_area_side_and_reopen() {
    use operandi_layout::WindowLayout;

    for (index, side) in [Side::Top, Side::Bottom, Side::Left, Side::Right]
        .into_iter()
        .enumerate()
    {
        let (mut a, ctx) = two_views();
        let source = Dot::ToolTabs;
        let hold = a.compass_point(Dot::View(0), Zone::Center);
        hold_dot(&mut a, &ctx, source, hold);
        let (target, to) = a.dock_test_strips()[index];
        assert!(matches!(target, DockTarget::Edge(_, hit) if hit == side));
        assert_eq!(a.dock_hit(source, to), Some(target));
        release_dot(&mut a, &ctx, to);
        let horizontal = matches!(side, Side::Top | Side::Bottom);
        assert_eq!(
            a.layouts.tool_tabs_horizontal,
            Some(horizontal),
            "{}",
            a.status
        );
        let panel = dock_leaf(&a, Pane::ToolTabs);
        let view = dock_leaf(&a, Pane::Viewport(0));
        let beside = match side {
            Side::Top => panel.bottom() < view.top(),
            Side::Bottom => panel.top() > view.bottom(),
            Side::Left => panel.right() < view.left(),
            Side::Right => panel.left() > view.right(),
        };
        assert!(beside);
        let create = a.toolbox_rects["Category Create"];
        let select = a.toolbox_rects["Category Select"];
        assert_eq!(create.top() == select.top(), horizontal);
        assert_eq!(create.left() == select.left(), !horizontal);
        assert!(panel.contains_rect(create));
        if horizontal {
            assert!((panel.height() - toolbox::HORIZONTAL_TOOL_TABS_HEIGHT).abs() < 0.01);
            let divider = pos2(
                panel.center().x,
                if side == Side::Top {
                    panel.bottom() + operandi_layout::GAP / 2.
                } else {
                    panel.top() - operandi_layout::GAP / 2.
                },
            );
            drag(
                &mut a,
                &ctx,
                divider,
                divider + vec2(0., 120.),
                Modifiers::NONE,
            );
            assert_eq!(dock_leaf(&a, Pane::ToolTabs), panel);
        } else {
            // Jason, 2026-11-19: a vertical strip is exactly as wide as its
            // tabs, with no empty band beside it.
            assert!(
                (panel.width() - toolbox::VERTICAL_TOOL_TABS_WIDTH).abs() < 0.01,
                "{side:?}: strip width {}",
                panel.width()
            );
        }

        let saved = WindowLayout::from_json(&a.capture_layout().unwrap().to_json()).unwrap();
        let mut reopened = box_app();
        reopened.apply_layout_doc(&saved).unwrap();
        idle(&mut reopened, &Context::default());
        assert_eq!(reopened.layouts.tool_tabs_horizontal, Some(horizontal));
        if horizontal {
            assert!((dock_leaf(&reopened, Pane::ToolTabs).height() - panel.height()).abs() < 0.01);
        }
        assert!(saved.same_as(&reopened.capture_layout().unwrap()));
        assert_eq!(
            reopened.toolbox_rects["Category Create"].size(),
            create.size()
        );
    }
}

#[test]
fn docking_tool_tabs_narrow_horizontal_row_scrolls_without_wrapping() {
    let (mut a, ctx) = two_views();
    let source = Dot::ToolTabs;
    // Dock atop the narrow toolbox for overflow without a different window size.
    let to = a.compass_point(Dot::Side(Panel::Toolbox), Zone::Edge(Side::Top));
    drag_dot(&mut a, &ctx, source, to, Modifiers::NONE);
    let area = dock_leaf(&a, Pane::ToolTabs);
    let first = a.toolbox_rects["Category Create"];
    assert!(!a.toolbox_rects.contains_key("Category Curve"));
    let scene = a.editor.scene.clone();
    let undo_steps = a.editor.undo.len();
    let right = a.toolbox_rects["Scroll tool tabs right"].center();
    for _ in 0..12 {
        if a.toolbox_rects.contains_key("Category Curve") {
            break;
        }
        click(&mut a, &ctx, right);
    }
    assert_eq!(a.category, toolbox::Category::Create);
    let curve = a.toolbox_rects["Category Curve"];
    assert_eq!(first.y_range(), curve.y_range());
    assert!(area.contains_rect(curve));
    for (name, rect) in &a.toolbox_rects {
        if name.starts_with("Category ") && !name.ends_with(" ink") && !name.ends_with(" surface") {
            assert_eq!(rect.y_range(), first.y_range(), "{name} wrapped");
            assert!(area.contains_rect(*rect), "{name} was not clipped");
        }
    }
    click(&mut a, &ctx, curve.center());
    assert_eq!(a.category, toolbox::Category::Curve);
    let left = a.toolbox_rects["Scroll tool tabs left"].center();
    for _ in 0..12 {
        if a.toolbox_rects.get("Category Create") == Some(&first) {
            break;
        }
        click(&mut a, &ctx, left);
    }
    assert_eq!(a.toolbox_rects["Category Create"], first);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.undo.len(), undo_steps);
}

#[test]
fn docking_tool_tabs_float_reopen_and_redock_by_title() {
    use crate::floating::DockWindow;
    use operandi_layout::WindowLayout;

    let root = crate::layouts_tests::data_root();
    crate::layouts_tests::install_legacy_model(&root);
    let ctx = Context::default();
    let mut a = crate::layouts_tests::launch(&root, &ctx);
    a.split_view();
    idle(&mut a, &ctx);
    a.dot_menu_action(Dot::ToolTabs, MenuAction::Detach);
    enter_build_mode(&mut a, &ctx);
    let id = a
        .views
        .window_of(Pane::ToolTabs)
        .expect("Tool Tabs floated");
    assert_eq!(a.layouts.tool_tabs_horizontal, Some(false));
    a.persist_layout_file();
    let json = std::fs::read_to_string(a.layout_file("Model").unwrap()).unwrap();
    let saved = WindowLayout::from_json(&json).unwrap();
    assert_eq!(saved.windows.len(), 1);
    let mut reopened = box_app();
    reopened.apply_layout_doc(&saved).unwrap();
    assert!(reopened.views.window_of(Pane::ToolTabs).is_some());
    assert!(saved.same_as(&reopened.capture_layout().unwrap()));

    assert_eq!(a.views.window_of(Pane::ToolTabs), Some(id));
    // The floating window still has its title strip to dock it by.
    assert!(crate::floating::dock_title_rect(&ctx, DockWindow::Tree(id)).is_positive());
    let target = DockTarget::Compass(Dot::View(1), Zone::Edge(Side::Bottom));
    a.drop_for_test(Dot::ToolTabs, Over::Dock(target));
    idle(&mut a, &ctx);
    assert!(a.views.windows.is_empty(), "{}", a.status);
    assert_eq!(a.layouts.tool_tabs_horizontal, Some(true));
    check_reopen(&a);
}

#[test]
fn docking_toolbox_leaves_tool_tabs_in_the_original_column() {
    let (mut a, ctx) = two_views();
    let source = Dot::Side(Panel::Toolbox);
    let to = a.compass_point(Dot::View(1), Zone::Edge(Side::Bottom));
    drag_dot(&mut a, &ctx, source, to, Modifiers::NONE);
    let tabs = dock_leaf(&a, Pane::ToolTabs);
    let tools = dock_leaf(&a, Pane::Toolbox);
    assert!(tabs.right() < tools.left());
    assert!(tabs.contains_rect(a.toolbox_rects["Category Create"]));
    assert!(tools.contains_rect(a.toolbox_rects["Move"]));
}

#[test]
fn docking_tool_tabs_guide_drop_changes_side_and_cancel_keeps_stock() {
    let (mut a, ctx) = two_views();
    let stock = a.capture_layout().unwrap();
    let source = Dot::ToolTabs;
    let hold = a.compass_point(Dot::View(0), Zone::Center);
    hold_dot(&mut a, &ctx, source, hold);
    let target = DockTarget::Compass(Dot::ToolProperties, Zone::Edge(Side::Top));
    let to = a.compass_point(Dot::ToolProperties, Zone::Edge(Side::Top));
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)], Modifiers::NONE);
    assert_eq!(a.dock_hit(source, to), Some(target));
    frame(
        &mut a,
        &ctx,
        vec![crate::tests::key(Key::Escape)],
        Modifiers::NONE,
    );
    release_dot(&mut a, &ctx, to);
    assert!(stock.same_as(&a.capture_layout().unwrap()));
    assert_eq!(a.layouts.tool_tabs_horizontal, None);

    hold_dot(&mut a, &ctx, source, hold);
    release_dot(&mut a, &ctx, to);
    assert_eq!(a.layouts.tool_tabs_horizontal, Some(true));
    let tabs = dock_leaf(&a, Pane::ToolTabs);
    assert!(tabs.top() > dock_leaf(&a, Pane::Toolbox).bottom());
    assert!(tabs.bottom() < dock_leaf(&a, Pane::ToolProperties).top());
    check_reopen(&a);
}

#[test]
fn items_torn_out_docked_beside_and_returned_keeps_the_original_row() {
    use crate::panel_registry::PanelKind as K;

    let (mut a, ctx) = one_view();
    let original = a.upper_frame.tabs.clone();
    let inventory = |a: &Operandi| {
        let mut names: Vec<_> = a
            .upper_frame
            .tabs
            .iter()
            .chain(&a.lower_frame.tabs)
            .map(|&kind| panel_registry::panel_type(kind).name.to_owned())
            .chain(a.views.dock_tabs.values().map(|slot| slot.kind.clone()))
            .collect();
        names.sort();
        assert!(names.windows(2).all(|pair| pair[0] != pair[1]));
        a.views.validate().unwrap();
        names
    };
    let expected = inventory(&a);
    let from = a.upper_frame.rects[0].center();
    drag(
        &mut a,
        &ctx,
        from,
        pos2(SIZE.x + 40., 280.),
        Modifiers::NONE,
    );
    assert_eq!(inventory(&a), expected);
    assert_eq!(a.upper_frame.tabs, original[1..]);

    let window = a.views.windows[0].id;
    a.apply_window_drop(
        crate::floating::DockWindow::Tree(window),
        DockTarget::Compass(Dot::Side(Panel::Items), Zone::Edge(Side::Left)),
    )
    .unwrap();
    idle(&mut a, &ctx);
    assert_eq!(inventory(&a), expected);
    assert_eq!(a.upper_frame.tabs, original[1..]);
    let Pane::DockTab(key) = a.registry_dock_pane(K::Items).unwrap() else {
        panic!("Items should be a standalone dock tile");
    };
    let tab = a.upper_frame.rects[0];
    let at = pos2(tab.left() + 2., tab.center().y);
    drag_dot(&mut a, &ctx, Dot::Tab(key), at, Modifiers::NONE);
    assert_eq!(inventory(&a), expected);
    assert!(a.views.dock_tabs.is_empty());
    assert!(a.views.windows.is_empty());
    assert_eq!(a.upper_frame.tabs, original);
    assert_eq!(a.upper_frame.front(), Some(K::Items));
    check_reopen(&a);
}

/// Jason, 2026-11-19: Items torn out, docked beside the group, then
/// dropped on the group's tree tab row came back as a nested "Items" tab
/// holding the whole group. A drop on a row that holds a sidebar group
/// joins that group's tabs; groups never nest.
#[test]
fn items_returned_on_the_groups_tree_tab_row_joins_without_nesting() {
    use crate::panel_registry::PanelKind as K;

    let (mut a, ctx) = one_view();
    let original = a.upper_frame.tabs.clone();
    let from = a.upper_frame.rects[0].center();
    drag(
        &mut a,
        &ctx,
        from,
        pos2(SIZE.x + 40., 280.),
        Modifiers::NONE,
    );
    let window = a.views.windows[0].id;
    a.apply_window_drop(
        crate::floating::DockWindow::Tree(window),
        DockTarget::Compass(Dot::Side(Panel::Items), Zone::Edge(Side::Left)),
    )
    .unwrap();
    idle(&mut a, &ctx);
    let Pane::DockTab(key) = a.registry_dock_pane(K::Items).unwrap() else {
        panic!("Items should be a standalone dock tile");
    };
    let row = a.views.dock_slot(Pane::Items).expect("the group's tab row");
    a.apply_dock_target(Dot::Tab(key), DockTarget::TreeTabs(row, 0))
        .unwrap();
    idle(&mut a, &ctx);
    let mut back = a.upper_frame.tabs.clone();
    back.sort();
    let mut want = original.clone();
    want.sort();
    assert_eq!(back, want, "every tab is back in the group, once");
    assert!(a.views.dock_tabs.is_empty(), "no stray tab tile is left");
    // No tree tab row holds the group pane beside another tab.
    let tiles = &a.views.main.tiles;
    for (_, tile) in tiles.iter() {
        if let egui_tiles::Tile::Container(egui_tiles::Container::Tabs(t)) = tile {
            let kids: Vec<_> = t
                .children
                .iter()
                .filter_map(|t| tiles.get_pane(t))
                .collect();
            if kids.contains(&&Pane::Items) {
                assert_eq!(kids.len(), 1, "the Items group sits alone: {kids:?}");
            }
        }
    }
    check_reopen(&a);
}

/// BARS, lost with docking (Jason, 2026-11-19): double-clicking the
/// divider between a side column and the views collapses that column;
/// the divider between the Toolbox and Tool Properties is plain resizing.
#[test]
fn docking_side_column_dividers_collapse_on_double_click() {
    use crate::layouts_tests::{data_root, launch};

    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    idle(&mut a, &ctx);
    assert!(a.layouts.mixed_dock);
    let props = dock_leaf(&a, Pane::ToolProperties);
    let tools = dock_leaf(&a, Pane::Toolbox);
    let items = dock_leaf(&a, Pane::Items);
    let view = dock_leaf(&a, Pane::Viewport(0));
    let left = pos2((props.right() + view.left()) / 2., view.center().y);
    let right = pos2((view.right() + items.left()) / 2., view.center().y);
    let inner = pos2(tools.center().x, (tools.bottom() + props.top()) / 2.);
    let tree = &a.views.main;
    assert_eq!(
        crate::views::side_column_divider_at(tree, left),
        Some("left")
    );
    assert_eq!(
        crate::views::side_column_divider_at(tree, right),
        Some("right")
    );
    assert_eq!(crate::views::side_column_divider_at(tree, inner), None);
    assert_eq!(
        crate::views::side_column_divider_at(tree, view.center()),
        None
    );

    // The same command the gesture runs collapses and restores the column.
    a.execute("operandi.bar.collapse bar:left on:true");
    idle(&mut a, &ctx);
    assert!(a.layouts.left_collapsed);
    a.execute("operandi.bar.collapse bar:left on:false");
    idle(&mut a, &ctx);
    assert!(!a.layouts.left_collapsed);
}

/// Jason, 2026-11-19: restoring a collapsed side column must bring back
/// exactly the width it had; a strip docked beside the Toolbox goes and
/// comes back with its column; only one message line ever shows.
#[test]
fn docking_side_column_collapse_restores_its_width() {
    use crate::layouts_tests::{data_root, launch};

    for with_strip in [false, true] {
        let root = data_root();
        let ctx = Context::default();
        let mut a = launch(&root, &ctx);
        idle(&mut a, &ctx);
        if with_strip {
            let to = a.compass_point(Dot::Side(Panel::Toolbox), Zone::Edge(Side::Right));
            drag_dot(&mut a, &ctx, Dot::ToolTabs, to, Modifiers::NONE);
            idle(&mut a, &ctx);
            assert_eq!(a.layouts.tool_tabs_horizontal, Some(false), "{}", a.status);
        }
        let before: Vec<Rect> = [Pane::Toolbox, Pane::ToolProperties, Pane::Items]
            .into_iter()
            .map(|p| dock_leaf(&a, p))
            .collect();
        let strip = with_strip.then(|| dock_leaf(&a, Pane::ToolTabs));
        for cycle in 0..2 {
            a.execute("operandi.bar.collapse bar:left on:true");
            idle(&mut a, &ctx);
            assert!(a.layouts.left_collapsed);
            if with_strip {
                assert!(
                    !a.toolbox_rects.contains_key("Category Create"),
                    "the side strip collapses with its column"
                );
            }
            // One message line: the bottom bar's row keeps it while the
            // column is collapsed, so the views don't draw a second one.
            assert!(a.layouts.mode_bar_bottom);
            assert!(a.layouts.message_line_here, "the row has the line");
            let lines = a.layouts.message_line_rect.iter().count();
            assert_eq!(lines, 1);
            a.execute("operandi.bar.collapse bar:left on:false");
            idle(&mut a, &ctx);
            let after: Vec<Rect> = [Pane::Toolbox, Pane::ToolProperties, Pane::Items]
                .into_iter()
                .map(|p| dock_leaf(&a, p))
                .collect();
            for (was, now) in before.iter().zip(&after) {
                assert!(
                    (was.width() - now.width()).abs() < 1.5,
                    "strip {with_strip}, cycle {cycle}: width {} -> {}",
                    was.width(),
                    now.width()
                );
            }
            if let Some(strip) = strip {
                assert!((dock_leaf(&a, Pane::ToolTabs).width() - strip.width()).abs() < 1.5);
            }
        }
    }
}

/// Jason, 2026-11-19: collapsing a side column by double-clicking its
/// divider, then double-clicking the thin line to reopen it, brought the
/// column back three to four times as wide. Drives the real gestures, for
/// both sides.
#[test]
fn docking_double_click_collapse_and_reopen_keep_the_column_width() {
    use crate::layouts_tests::{data_root, launch};

    fn double_click(a: &mut Operandi, ctx: &Context, at: Pos2) {
        frame(a, ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
        for _ in 0..2 {
            for pressed in [true, false] {
                frame(
                    a,
                    ctx,
                    vec![Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    }],
                    Modifiers::NONE,
                );
            }
        }
        idle(a, ctx);
    }

    for side in ["left", "right"] {
        let root = data_root();
        let ctx = Context::default();
        let mut a = launch(&root, &ctx);
        idle(&mut a, &ctx);
        let panes = [
            Pane::Toolbox,
            Pane::ToolProperties,
            Pane::Items,
            Pane::Properties,
        ];
        let widths = |a: &Operandi| panes.map(|p| dock_leaf(a, p).width());
        let before = widths(&a);
        let view = dock_leaf(&a, Pane::Viewport(0));
        let divider = if side == "left" {
            let props = dock_leaf(&a, Pane::ToolProperties);
            pos2((props.right() + view.left()) / 2., view.center().y)
        } else {
            let items = dock_leaf(&a, Pane::Items);
            pos2((view.right() + items.left()) / 2., view.center().y)
        };
        double_click(&mut a, &ctx, divider);
        let collapsed = if side == "left" {
            a.layouts.left_collapsed
        } else {
            a.layouts.right_collapsed
        };
        assert!(
            collapsed,
            "{side}: the divider double-click collapses ({})",
            a.status
        );
        let line = if side == "left" {
            pos2(crate::collapse_line::SIDE_AREA / 2., view.center().y)
        } else {
            pos2(
                SIZE.x - crate::collapse_line::SIDE_AREA / 2.,
                view.center().y,
            )
        };
        // A line that just appeared ignores clicks for a moment
        // (collapse_line::FRESH); let it settle like a person would.
        for _ in 0..40 {
            frame(&mut a, &ctx, vec![], Modifiers::NONE);
        }
        double_click(&mut a, &ctx, line);
        assert!(
            !a.layouts.left_collapsed && !a.layouts.right_collapsed,
            "{side}: reopened"
        );
        let after = widths(&a);
        for (pane, (was, now)) in panes.iter().zip(before.iter().zip(after)) {
            assert!(
                (was - now).abs() < 1.5,
                "{side}: {pane:?} came back {now} wide, was {was}"
            );
        }
    }
}

/// Jason, 2026-11-19: with the logo clicked, the top bar collapses to the
/// same thin line as the side columns (no wider empty band), and the line
/// brings it back.
#[test]
fn top_bar_collapses_to_a_thin_line() {
    use crate::layouts_tests::{data_root, launch};

    let root = data_root();
    let ctx = Context::default();
    let mut a = launch(&root, &ctx);
    idle(&mut a, &ctx);
    assert!(a.layouts.mode_bar_bottom);
    let logo = a.layouts.logo_rect.unwrap();
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(logo.center())],
        Modifiers::NONE,
    );
    for pressed in [true, false] {
        frame(
            &mut a,
            &ctx,
            vec![Event::PointerButton {
                pos: logo.center(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
            Modifiers::NONE,
        );
    }
    idle(&mut a, &ctx);
    assert!(a.layouts.top_collapsed, "{}", a.status);
    let strip = a.layouts.strip_rect.expect("the collapsed line");
    // On Windows and Linux the menu bar keeps its own row above the line
    // (Jason, 2026-11-22); on macOS the line is the window's top edge.
    let menu = a.layouts.menu_bar_rect.map_or(0., |r| r.bottom());
    assert!(
        strip.height() <= crate::collapse_line::TOP_AREA + 0.5 && strip.top() <= menu + 1.,
        "a thin line at the top, under the menu row ({menu}): {strip:?}"
    );
    let tabs = dock_leaf(&a, Pane::ToolTabs);
    assert!(
        tabs.top() <= strip.bottom() + 2.,
        "the Tool Tabs row sits right under the line: {tabs:?}"
    );
    a.execute("operandi.bar.collapse bar:top on:false");
    idle(&mut a, &ctx);
    assert!(!a.layouts.top_collapsed);
}

#[test]
fn build_mode_toggles_from_the_command_the_menu_and_its_key() {
    let (mut a, ctx) = two_views();
    assert!(!a.dots.build_mode);
    a.execute("operandi.layout.buildMode");
    assert!(a.dots.build_mode);
    a.execute("operandi.layout.buildMode on:false");
    assert!(!a.dots.build_mode);
    a.menu_action(&ctx, app_menu::line_action("operandi.layout.buildMode"));
    assert!(a.dots.build_mode);
    // Not in the shortcuts spec, free there and in the app's own map.
    assert_eq!(a.bindings["UI Build Mode"], "Ctrl+Shift+L");
    let chord = Modifiers {
        ctrl: true,
        shift: true,
        ..Modifiers::NONE
    };
    let press = Event::Key {
        key: Key::L,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: chord,
    };
    frame(&mut a, &ctx, vec![press], chord);
    assert!(!a.dots.build_mode, "{}", a.status);
}

#[test]
fn build_mode_grab_anywhere_on_a_panel_docks_it() {
    let (mut a, ctx) = two_views();
    a.execute("operandi.layout.buildMode on:true");
    idle(&mut a, &ctx);
    let panel = a.dot_panel_rect(Dot::ToolProperties).unwrap();
    let from = panel.center();
    let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(from, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from + vec2(30., 30.))],
        Modifiers::NONE,
    );
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)], Modifiers::NONE);
    assert_eq!(
        a.dots.drag.as_ref().map(|d| d.dot),
        Some(Dot::ToolProperties)
    );
    release_dot(&mut a, &ctx, to);
    let props = dock_leaf(&a, Pane::ToolProperties);
    assert!(
        props.top() > dock_leaf(&a, Pane::Viewport(0)).bottom(),
        "{}",
        a.status
    );
    // Out of build mode nothing moves.
    a.execute("operandi.layout.buildMode on:false");
    let before = a.views.main.clone();
    let from = dot(&a, Dot::View(1));
    drag_raw(&mut a, &ctx, from, pos2(700., 6.), Modifiers::NONE);
    assert_eq!(a.views.main, before);
    check_reopen(&a);
}

#[test]
fn build_mode_press_on_a_pad_without_moving_changes_nothing() {
    let (mut a, ctx) = two_views();
    a.execute("operandi.layout.buildMode on:true");
    idle(&mut a, &ctx);
    let before = a.capture_layout().unwrap();
    let panel = a.dot_panel_rect(Dot::View(0)).unwrap();
    let at = panel.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        vec![pointer(at, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(at, false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    idle(&mut a, &ctx);
    assert!(a.dots.drag.is_none());
    assert!(before.same_as(&a.capture_layout().unwrap()));
}

#[test]
fn build_mode_pad_moves_the_mode_bar_only_to_the_top_or_bottom() {
    let (mut a, ctx) = one_view();
    a.execute("operandi.layout.buildMode on:true");
    idle(&mut a, &ctx);
    let started_bottom = a.layouts.mode_bar_bottom;
    let row = a.layouts.mode_bar_rect.unwrap();
    let from = row.center();
    let screen = ctx.content_rect();
    // Released at the far side, near the middle of the window's height.
    let to = pos2(
        screen.center().x + 200.,
        if started_bottom {
            screen.center().y - 40.
        } else {
            screen.center().y + 40.
        },
    );
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![pointer(from, true, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(from + vec2(0., 30.))],
        Modifiers::NONE,
    );
    assert!(a.layouts.mode_bar_drag.is_some());
    release_dot(&mut a, &ctx, to);
    assert_ne!(a.layouts.mode_bar_bottom, started_bottom, "{}", a.status);
    // It never floats or docks at a side.
    assert!(a.views.windows.is_empty());
}

#[test]
fn build_mode_blocks_viewport_clicks_and_keeps_tab_clicks() {
    use panel_registry::PanelKind as K;
    let (mut a, ctx) = two_views();
    a.upper_frame.show_page(0);
    a.execute("operandi.layout.buildMode on:true");
    idle(&mut a, &ctx);
    // A click in the 3D view selects nothing and moves nothing.
    let selection = a.editor.scene.selection.clone();
    let layout = a.capture_layout().unwrap();
    let view = a.dot_panel_rect(Dot::View(0)).unwrap();
    click(&mut a, &ctx, view.center());
    assert_eq!(a.editor.scene.selection, selection);
    assert!(layout.same_as(&a.capture_layout().unwrap()));
    // Tabs still switch: the cover leaves the tab row free.
    let row = a.upper_frame.rects[0];
    assert!(a.build_cover_of(Dot::Side(Panel::Items)).top() >= row.bottom() - 0.5);
    let second = a.upper_frame.rects[1].center();
    let front = a.upper_frame.tabs[1];
    click(&mut a, &ctx, second);
    assert_eq!(a.upper_frame.front(), Some(front));
    assert_ne!(front, K::Items);
    // Done leaves the mode.
    a.execute("operandi.layout.buildMode on:false");
    assert!(!a.dots.build_mode);
}

#[test]
fn tabs_show_the_grab_hand_only_in_build_mode() {
    let (mut a, ctx) = two_views();
    let at = a.compass_point(Dot::View(1), Zone::Center);
    drag_dot(&mut a, &ctx, Dot::View(0), at, Modifiers::NONE);
    let row = a.views.dock_slot(Pane::Viewport(0)).unwrap();
    let tab = a.dock_tree_tab_point(row, 0) + vec2(12., 0.);
    let cursor = |a: &mut Operandi| {
        frame(a, &ctx, vec![Event::PointerMoved(tab)], Modifiers::NONE)
            .platform_output
            .cursor_icon
    };
    assert_ne!(cursor(&mut a), egui::CursorIcon::Grab);
    enter_build_mode(&mut a, &ctx);
    assert_eq!(cursor(&mut a), egui::CursorIcon::Grab);
}

#[test]
fn build_mode_shader_tree_tab_docks_beside_items_on_each_side() {
    use crate::panel_registry::PanelKind as K;
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        for target in [Dot::Side(Panel::Items), Dot::Side(Panel::Properties)] {
            let (mut a, ctx) = one_view();
            let index = a
                .upper_frame
                .tabs
                .iter()
                .position(|&k| k == K::ShaderTree)
                .unwrap();
            let from = a.upper_frame.rects[index].center();
            let to = a.compass_point(target, Zone::Edge(side));
            drag(&mut a, &ctx, from, to, Modifiers::NONE);
            let shader = a
                .views
                .dock_tabs
                .iter()
                .find(|(_, slot)| slot.kind == "Shader Tree")
                .map(|(&key, _)| key)
                .unwrap_or_else(|| panic!("{side:?} {target:?}: {}", a.status));
            let mine = dock_leaf(&a, Pane::DockTab(shader));
            let other = dock_leaf(&a, a.dock_pane(target));
            let ok = match side {
                Side::Left => mine.right() <= other.left() + 1.,
                Side::Right => mine.left() >= other.right() - 1.,
                Side::Top => mine.bottom() <= other.top() + 1.,
                Side::Bottom => mine.top() >= other.bottom() - 1.,
            };
            assert!(ok, "{side:?} {target:?}: {mine:?} vs {other:?}");
            assert!(a.front_panel(K::ShaderTree) && a.front_panel(K::Items));
            check_reopen(&a);
        }
    }
}

#[test]
fn build_mode_tabs_pull_out_alone_and_the_body_moves_the_group_in_a_docked_layout() {
    use crate::panel_registry::PanelKind as K;
    for docked in [false, true] {
        let (mut a, ctx) = one_view();
        if docked {
            // Any dock makes the side columns real dock-tree panels, whose
            // tab rows sit a few points inside the panel.
            let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Bottom));
            drag_dot(&mut a, &ctx, Dot::ToolProperties, to, Modifiers::NONE);
            assert!(a.layouts.mixed_dock);
        }
        enter_build_mode(&mut a, &ctx);
        // The tab row stays out of the panel's cover.
        let row = a.upper_frame.rects[0];
        assert!(
            a.build_cover_of(Dot::Side(Panel::Items)).top() >= row.bottom() - 0.5,
            "docked: {docked}"
        );
        // A tab drag takes out that tab only, here beside its own group.
        let index = a
            .upper_frame
            .tabs
            .iter()
            .position(|&k| k == K::ShaderTree)
            .unwrap();
        let from = a.upper_frame.rects[index].center();
        let to = a.compass_point(Dot::Side(Panel::Items), Zone::Edge(Side::Bottom));
        drag(&mut a, &ctx, from, to, Modifiers::NONE);
        assert!(
            !a.upper_frame.contains(K::ShaderTree),
            "docked {docked}: {}",
            a.status
        );
        assert!(a.upper_frame.contains(K::Items));
        let (&key, _) = a
            .views
            .dock_tabs
            .iter()
            .find(|(_, slot)| slot.kind == "Shader Tree")
            .expect("Shader Tree on its own");
        assert!(
            dock_leaf(&a, Pane::DockTab(key)).top() >= dock_leaf(&a, Pane::Items).bottom() - 1.
        );
        // A grab on the group's body moves the whole group.
        let tabs = a.upper_frame.tabs.clone();
        enter_build_mode(&mut a, &ctx);
        let from = a.build_cover_of(Dot::Side(Panel::Items)).center();
        let to = a.compass_point(Dot::View(0), Zone::Edge(Side::Right));
        drag(&mut a, &ctx, from, to, Modifiers::NONE);
        assert_eq!(a.upper_frame.tabs, tabs);
        assert!(dock_leaf(&a, Pane::Items).left() >= dock_leaf(&a, Pane::Viewport(0)).right() - 1.);
        check_reopen(&a);
    }
}

/// Jason, 2026-11-20: with Shader Tree docked beside the Items group, the
/// right column's collapse left Shader Tree showing and filling the room,
/// and with the left column collapsed no divider could be dragged.
#[test]
fn docking_a_collapsed_column_hides_panels_docked_beside_it_and_others_still_resize() {
    use crate::layouts_tests::{data_root, launch};
    use crate::panel_registry::PanelKind as K;

    for shader_side in [Side::Left, Side::Right] {
        let root = data_root();
        let ctx = Context::default();
        let mut a = launch(&root, &ctx);
        idle(&mut a, &ctx);
        let index = a
            .upper_frame
            .tabs
            .iter()
            .position(|&k| k == K::ShaderTree)
            .unwrap();
        let from = a.upper_frame.rects[index].center();
        let to = a.compass_point(Dot::Side(Panel::Items), Zone::Edge(shader_side));
        drag(&mut a, &ctx, from, to, Modifiers::NONE);
        idle(&mut a, &ctx);
        let key = *a
            .views
            .dock_tabs
            .iter()
            .find(|(_, slot)| slot.kind == "Shader Tree")
            .expect("Shader Tree docked beside Items")
            .0;
        let shader = dock_leaf(&a, Pane::DockTab(key));
        let view = dock_leaf(&a, Pane::Viewport(0));
        assert!(shader.left() >= view.right() - 1., "{shader_side:?}");

        // The divider facing the views collapses the whole right column.
        let column_left = shader.left().min(dock_leaf(&a, Pane::Items).left());
        let gap = pos2((view.right() + column_left) / 2., view.center().y);
        assert_eq!(
            crate::views::side_column_divider_at(&a.views.main, gap),
            Some("right"),
            "{shader_side:?}"
        );
        a.execute("operandi.bar.collapse bar:right on:true");
        idle(&mut a, &ctx);
        let wide = dock_leaf(&a, Pane::Viewport(0));
        assert!(
            wide.right() > SIZE.x - 40.,
            "{shader_side:?}: the views take the column's room: {wide:?}"
        );
        a.execute("operandi.bar.collapse bar:right on:false");
        idle(&mut a, &ctx);
        let back = dock_leaf(&a, Pane::DockTab(key));
        assert!(
            (back.width() - shader.width()).abs() < 1.5,
            "{shader_side:?}"
        );

        // Left collapsed: the right column's divider still drags.
        let toolbox = dock_leaf(&a, Pane::Toolbox).width();
        a.execute("operandi.bar.collapse bar:left on:true");
        idle(&mut a, &ctx);
        let view = dock_leaf(&a, Pane::Viewport(0));
        let column_left = dock_leaf(&a, Pane::DockTab(key))
            .left()
            .min(dock_leaf(&a, Pane::Items).left());
        let gap = pos2((view.right() + column_left) / 2., view.center().y);
        drag_raw(&mut a, &ctx, gap, gap + vec2(-80., 0.), Modifiers::NONE);
        idle(&mut a, &ctx);
        let moved = dock_leaf(&a, Pane::Viewport(0)).right();
        assert!(
            moved < view.right() - 40.,
            "{shader_side:?}: {moved} vs {}",
            view.right()
        );
        // And the left column still comes back as wide as it was.
        a.execute("operandi.bar.collapse bar:left on:false");
        idle(&mut a, &ctx);
        assert!((dock_leaf(&a, Pane::Toolbox).width() - toolbox).abs() < 1.5);
        check_reopen(&a);
    }
}
