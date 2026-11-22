//! Right-drag lasso in Vertex mode through the viewport's own events, on
//! geometry the tools make (Jason's QA, 2026-11-10).
use super::tests::{draw, pointer};
use super::*;
use crate::draw_tools::DrawKind;

const SIZE: Vec2 = vec2(1440., 900.);

fn right(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers,
    }
}

/// Right-drag a closed loop through `path`, a frame per point.
fn lasso(a: &mut Operandi, ctx: &Context, path: &[Pos2], modifiers: Modifiers) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(path[0])]);
    draw(a, ctx, SIZE, vec![right(path[0], true, modifiers)]);
    for &p in &path[1..] {
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![Event::PointerMoved(path[0])]);
    draw(a, ctx, SIZE, vec![right(path[0], false, modifiers)]);
    draw(a, ctx, SIZE, vec![]);
}

/// A rectangle loop around the screen box from `lo` to `hi`, padded.
fn around(lo: Pos2, hi: Pos2, pad: f32) -> Vec<Pos2> {
    vec![
        pos2(lo.x - pad, lo.y - pad),
        pos2(hi.x + pad, lo.y - pad),
        pos2(hi.x + pad, hi.y + pad),
        pos2(lo.x - pad, hi.y + pad),
    ]
}

fn front_app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.undo.clear();
    a.camera.view = View::Front;
    a.camera.distance = 12.;
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

fn click(a: &mut Operandi, ctx: &Context, p: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    draw(a, ctx, SIZE, vec![pointer(p, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(p, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![]);
}

fn to_vertex_mode(a: &mut Operandi, ctx: &Context) {
    a.run(Command::Mode(Mode::Vertex));
    draw(a, ctx, SIZE, vec![]);
}

fn screen_bounds(a: &Operandi) -> (Pos2, Pos2) {
    let item = a.editor.scene.active().unwrap();
    let r = a.viewport_rect;
    let points: Vec<Pos2> = item
        .mesh
        .vertices
        .iter()
        .map(|&v| {
            a.camera
                .project(a.editor.scene.world_point(item.id, v), r)
                .unwrap()
                .0
        })
        .collect();
    let b = Rect::from_points(&points);
    (b.min, b.max)
}

fn selected(a: &Operandi) -> Vec<u32> {
    let mut v = a.editor.scene.selection.vertices.clone();
    v.sort_unstable();
    v
}

#[test]
fn front_view_lasso_takes_a_drawn_curves_points() {
    let (mut a, ctx) = front_app();
    a.category = toolbox::Category::Create;
    a.activate_draw(DrawKind::Curve);
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    for d in [vec2(-120., 0.), vec2(0., -80.), vec2(120., 0.)] {
        click(&mut a, &ctx, c + d);
    }
    a.drop_draw();
    draw(&mut a, &ctx, SIZE, vec![]);
    to_vertex_mode(&mut a, &ctx);
    let n = a.editor.scene.active().unwrap().mesh.vertices.len() as u32;
    assert_eq!(n, 3, "curve points: {}", a.status);
    let (lo, hi) = screen_bounds(&a);
    lasso(&mut a, &ctx, &around(lo, hi, 30.), Modifiers::NONE);
    assert_eq!(selected(&a), (0..n).collect::<Vec<_>>(), "{}", a.status);
    // Part of the curve: only the left point.
    lasso(
        &mut a,
        &ctx,
        &around(c + vec2(-150., -30.), c + vec2(-90., 30.), 0.),
        Modifiers::NONE,
    );
    assert_eq!(selected(&a).len(), 1, "{}", a.status);
    // Edges are the curve's spans; the curve is one element.
    a.run(Command::Mode(Mode::Edge));
    lasso(&mut a, &ctx, &around(lo, hi, 30.), Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.edges.len(), 2, "{}", a.status);
    a.run(Command::Mode(Mode::Polygon));
    lasso(&mut a, &ctx, &around(lo, hi, 30.), Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.faces.len(), 1, "{}", a.status);
}

#[test]
fn front_view_lasso_works_after_a_curve_then_a_box() {
    // Jason's steps one after the other: the box lands beside the curve,
    // and the curve used to make the whole lasso refuse.
    let (mut a, ctx) = front_app();
    a.category = toolbox::Category::Create;
    a.activate_draw(DrawKind::Curve);
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    for d in [vec2(-300., 200.), vec2(-250., 150.), vec2(-200., 200.)] {
        click(&mut a, &ctx, c + d);
    }
    a.drop_draw();
    a.activate(Tool::Box);
    draw(&mut a, &ctx, SIZE, vec![]);
    let (from, to) = (c + vec2(-100., -80.), c + vec2(100., 80.));
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            Event::PointerMoved(from),
            pointer(from, true, Modifiers::NONE),
        ],
    );
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(to)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    a.run(Command::Drop);
    draw(&mut a, &ctx, SIZE, vec![]);
    to_vertex_mode(&mut a, &ctx);
    lasso(&mut a, &ctx, &around(from, to, 20.), Modifiers::NONE);
    let got = a.editor.scene.selection.component_count();
    assert!(got >= 4, "plane corners: {got}, {}", a.status);
    // Around nothing: deselects.
    lasso(
        &mut a,
        &ctx,
        &around(c + vec2(250., 150.), c + vec2(320., 220.), 0.),
        Modifiers::NONE,
    );
    assert_eq!(
        a.editor.scene.selection.component_count(),
        0,
        "{}",
        a.status
    );
}

#[test]
fn front_view_lasso_takes_a_flat_box_tool_planes_points() {
    let (mut a, ctx) = front_app();
    a.activate(Tool::Box);
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    let (from, to) = (c + vec2(-100., -80.), c + vec2(100., 80.));
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            Event::PointerMoved(from),
            pointer(from, true, Modifiers::NONE),
        ],
    );
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(to)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(to, false, Modifiers::NONE)],
    );
    a.run(Command::Drop);
    draw(&mut a, &ctx, SIZE, vec![]);
    to_vertex_mode(&mut a, &ctx);
    let n = a.editor.scene.active().unwrap().mesh.vertices.len() as u32;
    assert!(n >= 4, "plane points: {n}, {}", a.status);
    let (lo, hi) = screen_bounds(&a);
    lasso(&mut a, &ctx, &around(lo, hi, 30.), Modifiers::NONE);
    assert_eq!(selected(&a), (0..n).collect::<Vec<_>>(), "{}", a.status);
}

#[test]
fn lasso_around_nothing_deselects_but_shift_and_ctrl_keep_the_selection() {
    let (mut a, ctx) = front_app();
    a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        faces: vec![operandi_core::Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    to_vertex_mode(&mut a, &ctx);
    a.editor.scene.selection.vertices = vec![0, 1];
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    let empty = around(
        r.left_bottom() + vec2(40., -120.),
        r.left_bottom() + vec2(120., -40.),
        0.,
    );
    for mods in [Modifiers::SHIFT, Modifiers::CTRL] {
        lasso(&mut a, &ctx, &empty, mods);
        assert_eq!(selected(&a), vec![0, 1], "{mods:?}: {}", a.status);
    }
    lasso(&mut a, &ctx, &empty, Modifiers::NONE);
    assert!(selected(&a).is_empty(), "{}", a.status);
    a.run(Command::Undo);
    assert_eq!(selected(&a), vec![0, 1], "one undo step brings it back");
}
