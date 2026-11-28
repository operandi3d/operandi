//! Contour in the app (missing-tools-g4 MT13-14): the Topology tab button
//! starts it, and a drag across the background in the viewport lays the
//! curve loop on release.
use super::tests::draw;
use super::*;

#[test]
fn a_drag_across_the_background_lays_a_curve_on_release() {
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::NewMesh,
    });
    a.run(Command::NewMesh);
    a.camera.view = View::Front;
    a.category = toolbox::Category::Topology;
    draw(&mut a, &ctx, size, vec![]);
    super::toolbox_form_tests::button_by_line(&a, "operandi.topology.tool \"Contour\"");
    a.execute("Contour");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Contour),
        "{}",
        a.status
    );
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    // Across the sphere's middle, well past both sides.
    let (left, right) = (
        r.center() - vec2(r.width() * 0.4, 0.),
        r.center() + vec2(r.width() * 0.4, 0.),
    );
    let drag = match a.slice_press(left, r, Modifiers::NONE, PointerButton::Primary) {
        Some(input::GestureKind::Slice(d)) => d,
        _ => panic!("Contour takes the press: {}", a.status),
    };
    a.slice_drag(&drag, right, r, Modifiers::NONE);
    // Fast Slice: nothing yet while dragging.
    assert!(a.editor.scene.active().unwrap().mesh.faces.is_empty());
    a.slice_release(&drag);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 1, "{}", a.status);
    assert_eq!(mesh.faces[0].vertices.len(), 10);
    assert!(mesh.is_polyline(0), "a Curve");
}
