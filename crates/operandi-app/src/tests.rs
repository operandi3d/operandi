//! Shared helpers for the app's headless UI tests. The tests themselves
//! live in `*_tests.rs` files grouped by area.
use super::*;

pub(super) fn box_app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    a.camera.view = View::Top;
    a
}
pub(super) fn draw(
    a: &mut Operandi,
    ctx: &Context,
    size: Vec2,
    events: Vec<Event>,
) -> egui::FullOutput {
    draw_held(a, ctx, size, events, Modifiers::NONE)
}
/// `draw` with modifier keys held for the whole frame (what
/// `ui.input(|i| i.modifiers)` reads).
pub(super) fn draw_held(
    a: &mut Operandi,
    ctx: &Context,
    size: Vec2,
    events: Vec<Event>,
    modifiers: Modifiers,
) -> egui::FullOutput {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let mut raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        modifiers,
        focused: true,
        ..Default::default()
    };
    let scale = ctx.data(|d| d.get_temp::<f32>(Id::new("simulation-scale")).unwrap_or(1.));
    raw.viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(scale);
    // The real app's raw-input hook steps: help mode's clicks,
    a.help_mode_take_input(&mut raw);
    // and the one that keeps bound Tab chords.
    crate::numeric_tab::take_tab(ctx, &mut raw);
    a.take_tool_tab_keys(ctx, &mut raw);
    a.hold_bound_tab(ctx, &mut raw);
    // And the one that hands puck button presses in as keys.
    a.space_mouse_take_events(&mut raw.events);
    a.space_mouse_take_numbers();
    // And the bridge's pointer_stroke frames.
    a.mcp_take_stroke_input(&mut raw);
    ctx.run(raw, |ctx| {
        a.begin_input(ctx);
        a.compose(ctx);
        a.keyboard(ctx);
    })
}
/// Tool Properties field `key`'s mini-slider arrows (UIC-53): just right of
/// its text, or of its group's link button on an X/Y/Z row (MN-06b).
pub(super) fn arrows_rect(a: &Operandi, key: &str) -> Rect {
    let field = a.field_rects[key];
    let linked = key
        .strip_suffix(['0', '1', '2'])
        .is_some_and(|base| a.field_rects.contains_key(&format!("{base}-link")));
    let left = if linked {
        field.right() + 2. + crate::link_mode::WIDTH + 2.
    } else {
        field.right() + 2.
    };
    Rect::from_min_size(
        pos2(left, field.top()),
        vec2(scrub::HANDLE_WIDTH, field.height()),
    )
}

pub(super) fn pointer(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        pressed,
        modifiers,
        button: PointerButton::Primary,
    }
}
/// What the window reports for the "Cmd" key of a binding: Command on
/// macOS, Control on Linux and Windows.
pub(crate) const PLATFORM_CMD: Modifiers = Modifiers {
    alt: false,
    ctrl: !cfg!(target_os = "macos"),
    shift: false,
    mac_cmd: cfg!(target_os = "macos"),
    command: true,
};
pub(super) fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
pub(super) fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

pub(super) fn pen_command(kind: &str, flag: bool) -> String {
    format!(
        "pen {}",
        serde_json::json!({"positions_m":[[-2.,-1.,0.],[4.,0.,0.],[3.,2.,0.],[-1.,1.,0.]],"type":kind,"flip":kind=="polygon"&&flag,"close":kind=="polyline"&&flag,"select_new":true})
    )
}
