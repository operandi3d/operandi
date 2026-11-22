use super::toolbox_form_tests::button_by_line;
use super::*;
#[test]
fn flip_command_rejects_arguments_and_uses_one_shot_history() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
            .unwrap()
            .into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.editor.undo.clear();
    let source = a.editor.scene.clone();
    a.execute("poly.flip extra");
    assert_eq!(a.editor.scene, source);
    assert!(a.editor.undo.is_empty());
    a.execute("poly.flip");
    assert_ne!(a.editor.scene, source);
    assert_eq!(a.editor.undo.last().unwrap().label, "Flip");
    a.execute("undo");
    assert_eq!(a.editor.scene, source);
}
#[test]
fn polygon_button_and_factory_f_match_command() {
    use super::tests::{draw, key, pointer};
    fn app() -> Operandi {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
                .unwrap()
                .into();
        a.editor.scene.selection.mode = Mode::Polygon;
        a.editor.scene.selection.faces = vec![0];
        a.editor.undo.clear();
        a.category = toolbox::Category::Polygon;
        a
    }
    let mut expected = app();
    expected.execute("poly.flip");
    let want = expected.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 2000.);
    let mut a = app();
    draw(&mut a, &ctx, size, vec![]);
    let pos = button_by_line(&a, "Flip").center();
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(pos, true, Modifiers::NONE),
            pointer(pos, false, Modifiers::NONE),
        ],
    );
    assert_eq!(a.editor.scene, want);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut b = app();
    draw(&mut b, &ctx, size, vec![]);
    let viewport = b.viewport_rect.center();
    draw(
        &mut b,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), key(Key::F)],
    );
    assert_eq!(b.editor.scene, want);
    let mut c = app();
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut c, &ctx, size, vec![]);
    let before = c.editor.scene.clone();
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(
        &mut c,
        &ctx,
        size,
        vec![key(Key::F), Event::Text("f".into())],
    );
    assert_eq!(c.editor.scene, before);
    assert_eq!(c.command, "f");
    let mut d = app();
    d.bindings.insert("Flip".into(), "K".into());
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut d, &ctx, size, vec![]);
    let start = d.editor.scene.clone();
    let viewport = d.viewport_rect.center();
    draw(
        &mut d,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), key(Key::F)],
    );
    assert_eq!(d.editor.scene, start);
    draw(&mut d, &ctx, size, vec![key(Key::K)]);
    assert_eq!(d.editor.scene, want);
}
