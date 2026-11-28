//! Missing tools group 2 (specifications/missing-tools-g2): Plane Item on
//! the Text button's Shift-click (MT5-04), and the Teapot tool and Teapot
//! Item from the command line and Item ▸ Primitive (MT6-08).
use super::toolbox_form_tests::click_line;
use super::*;

#[test]
fn shift_click_on_text_adds_a_plane_item() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let first = a.editor.scene.active().unwrap().id;
    a.category = toolbox::Category::Create;
    super::tests::draw(&mut a, &ctx, vec2(1440., 1400.), vec![]);
    click_line(&mut a, &ctx, "Text", Modifiers::SHIFT);
    let item = a.editor.scene.active().unwrap();
    assert_eq!(item.name, "Plane", "{}", a.status);
    assert_ne!(item.id, first);
    assert_eq!(item.mesh.faces.len(), 1);
    assert!(
        a.editor.session.shape.is_none(),
        "no tool, no drag (MT5-01)"
    );
}

#[test]
fn teapot_tool_and_items_run_from_their_command_lines() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.execute("Teapot");
    assert_eq!(
        a.editor.session.shape.as_ref().map(|s| s.kind()),
        Some(ShapeKind::Teapot),
        "{}",
        a.status
    );
    a.execute("Teapot");
    assert!(a.editor.session.shape.is_none(), "a second Teapot drops it");
    a.execute("Teapot Item");
    assert_eq!(a.editor.scene.active().unwrap().name, "Teapot");
    a.execute("Plane Item");
    assert_eq!(a.editor.scene.active().unwrap().name, "Plane");
    // Item ▸ Primitive lists them (MT6-08).
    let menu = &a.forms.get("menu.item.primitive").unwrap().controls;
    let lines: Vec<String> = menu.iter().filter_map(|c| c.command.clone()).collect();
    for line in ["Teapot", "Teapot Item", "Plane Item"] {
        assert!(lines.iter().any(|l| l == line), "{line} in {lines:?}");
    }
}
