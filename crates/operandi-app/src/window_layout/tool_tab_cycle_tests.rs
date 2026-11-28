//! Ctrl+Tab / Ctrl+Shift+Tab page the tool tabs (tool-tab-cycling v1.1).
//! The T-numbers are the spec's acceptance cases. T9 (Cmd+Tab is the
//! system's) and T13 (Tab in a dialog) need the real OS / are covered by
//! the dialog tests; T12 (Tab between number fields) by numeric_tab_tests.
use super::tests::{box_app, draw, pointer};
use super::*;
use crate::toolbox::{Category, ToolSet};

const SIZE: Vec2 = vec2(1440., 900.);

fn press(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}
fn ctrl_tab() -> Event {
    press(Key::Tab, Modifiers::CTRL, false)
}
fn ctrl_shift_tab() -> Event {
    press(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT, false)
}

/// A box scene showing the Model set's strip, at `tab`.
fn model_at(ctx: &Context, tab: Category) -> Operandi {
    theme::install(ctx);
    let mut a = box_app();
    a.category = tab;
    for _ in 0..2 {
        let _ = draw(&mut a, ctx, SIZE, vec![]);
    }
    assert_eq!(a.layouts.tool_set, ToolSet::Model);
    assert!(
        a.toolbox_rects.contains_key("Category Create"),
        "the strip is drawn"
    );
    a
}

fn keys(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    let _ = draw(a, ctx, SIZE, events);
}

#[test]
fn ctrl_tab_opens_the_tab_to_the_right_on_the_press() {
    // T1, T11: Operandi's strip has Select right of Create. The press
    // alone switches: no release, no Enter.
    let ctx = Context::default();
    let mut a = model_at(&ctx, Category::Create);
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, Category::Select, "{}", a.status);
    assert_eq!(a.status, "Select tab");
    keys(&mut a, &ctx, vec![ctrl_shift_tab()]);
    assert_eq!(a.category, Category::Create);
}

#[test]
fn the_keys_wrap_at_both_ends() {
    // T2, T3.
    let ctx = Context::default();
    let tabs = ToolSet::Model.tabs();
    let mut a = model_at(&ctx, *tabs.last().unwrap());
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, tabs[0]);
    keys(&mut a, &ctx, vec![ctrl_shift_tab()]);
    assert_eq!(a.category, *tabs.last().unwrap());
}

#[test]
fn n_presses_come_back_to_the_start_and_repeats_count() {
    // T4, spec 5: one tab per key repeat.
    let ctx = Context::default();
    let tabs = ToolSet::Model.tabs();
    let mut a = model_at(&ctx, Category::Edge);
    let mut seen = vec![];
    for i in 0..tabs.len() {
        keys(&mut a, &ctx, vec![press(Key::Tab, Modifiers::CTRL, i > 0)]);
        seen.push(a.category);
    }
    assert_eq!(a.category, Category::Edge);
    let mut sorted = seen.clone();
    sorted.sort_by_key(|c| tabs.iter().position(|t| t == c));
    sorted.dedup();
    assert_eq!(sorted.len(), tabs.len(), "every tab once: {seen:?}");
    // Two presses in one frame move two tabs.
    keys(&mut a, &ctx, vec![ctrl_tab(), ctrl_tab()]);
    assert_eq!(a.category, Category::Topology);
}

#[test]
fn a_hidden_tab_is_skipped() {
    // T5: the Setup set hides Rig while the animation switch is off.
    let ctx = Context::default();
    theme::install(&ctx);
    for (animation, after) in [(false, Category::Setup), (true, Category::Rig)] {
        crate::anim_switch::set_for_test(animation);
        let mut a = box_app();
        a.execute("operandi.toolbox.set setup");
        a.category = Category::Weighting;
        for _ in 0..2 {
            keys(&mut a, &ctx, vec![]);
        }
        keys(&mut a, &ctx, vec![ctrl_tab()]);
        assert_eq!(a.category, after, "animation {animation}");
    }
    crate::anim_switch::set_for_test(true);
}

#[test]
fn one_tab_or_no_strip_does_nothing() {
    // Spec 3 and 7 (Q2: a collapsed panel isn't reopened).
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.execute("operandi.toolbox.set uv");
    keys(&mut a, &ctx, vec![]);
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, Category::Uv);

    let mut a = model_at(&ctx, Category::Create);
    a.layouts.left_collapsed = true;
    keys(&mut a, &ctx, vec![]);
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, Category::Create, "collapsed panel");
    assert!(a.layouts.left_collapsed, "not reopened");
}

#[test]
fn the_active_tool_stays_and_a_tool_properties_field_keeps_focus() {
    // T6, spec 4: Bevel stays active; focus outside the drawer stays put.
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.category = Category::Polygon;
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    a.run(Command::Preview(
        a.editor.session.live.as_ref().unwrap().params.clone(),
    ));
    for _ in 0..2 {
        keys(&mut a, &ctx, vec![]);
    }
    let field = a.field_rects["inset"].center();
    keys(
        &mut a,
        &ctx,
        vec![
            pointer(field, true, Modifiers::NONE),
            pointer(field, false, Modifiers::NONE),
        ],
    );
    let focused = ctx.memory(|m| m.focused());
    assert!(focused.is_some(), "the field is being edited");
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, Category::Topology, "{}", a.status);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Bevel)
    );
    assert_eq!(ctx.memory(|m| m.focused()), focused, "focus stays");
}

/// A frame that also draws a focusable button at `at` and gives it focus.
fn frame_with_button(a: &mut Operandi, ctx: &Context, at: Pos2, events: Vec<Event>) -> Id {
    let mut raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
        events,
        focused: true,
        ..Default::default()
    };
    a.take_tool_tab_keys(ctx, &mut raw);
    a.hold_bound_tab(ctx, &mut raw);
    let mut id = Id::NULL;
    let _ = ctx.run(raw, |ctx| {
        a.begin_input(ctx);
        a.compose(ctx);
        egui::Area::new(Id::new("test-focus-button"))
            .fixed_pos(at)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                let response = ui.button("focus me");
                id = response.id;
                if ctx.memory(|m| m.focused()).is_none() && a.category == Category::Create {
                    response.request_focus();
                }
            });
        a.keyboard(ctx);
    });
    id
}

#[test]
fn focus_in_the_old_tabs_drawer_goes_back_to_the_viewport() {
    // T7 and spec 4, with a control elsewhere as the counter case.
    let ctx = Context::default();
    let drawer = {
        let a = model_at(&ctx, Category::Create);
        a.toolbox_rects[crate::tool_tab_cycle::DRAWER]
    };
    for (at, inside) in [
        (drawer.center(), true),
        (pos2(SIZE.x / 2., SIZE.y / 2.), false),
    ] {
        let mut a = model_at(&ctx, Category::Create);
        ctx.memory_mut(|m| m.stop_text_input());
        let id = frame_with_button(&mut a, &ctx, at, vec![]);
        let _ = frame_with_button(&mut a, &ctx, at, vec![]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(id), "focused at {at:?}");
        let _ = frame_with_button(&mut a, &ctx, at, vec![ctrl_tab()]);
        assert_eq!(a.category, Category::Select);
        let still = ctx.memory(|m| m.focused()) == Some(id);
        assert_eq!(still, !inside, "inside the drawer: {inside}");
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
}

#[test]
fn a_modal_dialog_or_a_drag_keeps_the_tab() {
    // T8, spec 6.
    let ctx = Context::default();
    let mut a = model_at(&ctx, Category::Create);
    a.scenes.confirm_close_all = true;
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, Category::Create, "modal");
    a.scenes.confirm_close_all = false;
    let view = a.viewport_rect.center();
    keys(&mut a, &ctx, vec![pointer(view, true, Modifiers::NONE)]);
    keys(&mut a, &ctx, vec![ctrl_tab()]);
    assert_eq!(a.category, Category::Create, "drag");
}

#[test]
fn plain_tab_stays_the_subdivision_key() {
    // T10: no tab change from plain Tab or Shift+Tab.
    let ctx = Context::default();
    let mut a = model_at(&ctx, Category::Create);
    let view = a.viewport_rect.center();
    keys(
        &mut a,
        &ctx,
        vec![
            Event::PointerMoved(view),
            press(Key::Tab, Modifiers::NONE, false),
            press(Key::Tab, Modifiers::SHIFT, false),
        ],
    );
    assert_eq!(a.category, Category::Create);
}

#[test]
fn the_keys_are_in_the_input_map_and_the_commands_match() {
    let a = box_app();
    assert_eq!(a.bindings[crate::tool_tab_cycle::NEXT], "Ctrl+Tab");
    assert_eq!(
        a.bindings[crate::tool_tab_cycle::PREVIOUS],
        "Ctrl+Shift+Tab"
    );
    assert_eq!(a.bindings["Layout Switcher"], "", "the key moved");
    let ctx = Context::default();
    let mut a = model_at(&ctx, Category::Curve);
    a.execute("Next Tool Tab");
    assert_eq!(a.category, Category::Create, "{}", a.status);
    a.execute("Previous Tool Tab");
    assert_eq!(a.category, Category::Curve, "{}", a.status);
}

#[test]
fn a_saved_map_with_the_old_switcher_key_hands_it_over() {
    let dir = std::env::temp_dir().join(format!("operandi-tool-tabs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bindings.json");
    std::fs::write(&path, r#"{"Layout Switcher":"Ctrl+Shift+Tab"}"#).unwrap();
    let map = input::load_bindings(&path);
    assert_eq!(map["Layout Switcher"], "");
    assert_eq!(map[crate::tool_tab_cycle::PREVIOUS], "Ctrl+Shift+Tab");
    // A user's own Ctrl+Tab elsewhere wins: Next Tool Tab starts unset.
    std::fs::write(&path, r#"{"Frame All":"Ctrl+Tab"}"#).unwrap();
    let map = input::load_bindings(&path);
    assert_eq!(map[crate::tool_tab_cycle::NEXT], "");
    let _ = std::fs::remove_dir_all(&dir);
}
