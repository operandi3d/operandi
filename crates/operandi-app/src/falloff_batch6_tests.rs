//! Falloff batch 6 (falloff spec 2026-11-15): Path points clicked and
//! dragged in the view (FO-53), Curvature's section (FO-51).
use super::tests::{box_app, draw};
use super::*;
use operandi_core::{CurvatureFalloff, PathFalloff};

const SIZE: Vec2 = vec2(1440., 900.);

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Primary,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![button(at, true)]);
    draw(a, ctx, SIZE, vec![button(at, false)]);
}

/// FO-53: with point adding on, each click adds a point; a drag on a
/// point moves it.
#[test]
fn clicks_add_path_points_and_drags_move_them() {
    let ctx = Context::default();
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Path(
        PathFalloff::default(),
    ))));
    a.falloff_path_editing = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    click(&mut a, &ctx, r.left_top() + vec2(100., 100.));
    click(&mut a, &ctx, r.left_top() + vec2(300., 120.));
    assert_eq!(a.editor.falloff_path.len(), 2, "{}", a.status);
    let first = a.editor.falloff_path[0];
    let q = a.camera.project(first, r).unwrap().0;
    assert_eq!(a.falloff_hit(q, r), Some(0));
    a.falloff_path_editing = false;
    a.path_drag_base = Some(a.editor.falloff_path.clone());
    a.drag_falloff(
        0,
        Falloff::Path(PathFalloff::default()),
        q,
        q + vec2(40., 0.),
        r,
    );
    assert_ne!(a.editor.falloff_path[0], first);
    assert_eq!(a.editor.falloff_path.len(), 2);
}

#[test]
fn curvature_section_has_scale() {
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Curvature(
        CurvatureFalloff::default(),
    ))));
    let p = crate::falloff_panel::FalloffFields::params(&a).unwrap();
    use crate::tool_form::ToolFields;
    let f = crate::falloff_panel::FalloffFields::form(&a, &p);
    let labels: Vec<String> = f.fields().iter().map(|x| x.row.label.clone()).collect();
    assert_eq!(labels, ["Scale", "Mix Mode"]);
}
