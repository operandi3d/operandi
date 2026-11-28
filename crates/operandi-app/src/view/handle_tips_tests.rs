//! MN-09 in the view: the hovered handle's tooltip comes from the table.
use super::tests::{box_app, draw};
use super::*;
use operandi_core::LinearFalloff;

const SIZE: Vec2 = vec2(1440., 900.);

#[test]
fn a_hovered_falloff_handle_says_what_it_is() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::Linear(
        LinearFalloff::default(),
    ))));
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    let (start, _) = a.falloff_handles(r)[0];
    let at = a.camera.project(start, r).unwrap().0;
    let text = a.other_handle_tip(at, r).expect("a tooltip");
    assert!(text.contains("Start") && text.contains("Drag"), "{text}");
}

#[test]
fn bevel_handles_name_shift_and_inset() {
    let t = crate::handle_tips::live_tool_tip(
        Tool::Bevel,
        operandi_core::tool_card::HandleKind::Pair,
        0,
    );
    assert!(t.starts_with("Bevel: ") && t.contains("Shift"), "{t}");
    let t = crate::handle_tips::live_tool_tip(
        Tool::Bevel,
        operandi_core::tool_card::HandleKind::Pair,
        1,
    );
    assert!(t.contains("Inset"), "{t}");
}

#[test]
fn an_armed_tool_says_click_to_start() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.editor.auto_activate_flags = true;
    a.editor.session.armed = Some(Tool::Bevel);
    a.activate(Tool::Bevel);
    a.editor.session.armed = Some(Tool::Bevel);
    let r = a.viewport_rect;
    if a.editor.is_armed() {
        let text = a.other_handle_tip(r.center(), r).unwrap();
        assert!(text.contains("Click to start"), "{text}");
    }
}
