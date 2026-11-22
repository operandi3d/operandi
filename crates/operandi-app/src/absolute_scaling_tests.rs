//! Absolute Scaling window and `scale.absolute` command
//! (snaps-precision-2026-11-10 A).
use super::*;
use operandi_core::{AbsoluteMode, Mesh};

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 2000.))),
            focused: true,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keyboard(ctx);
        },
    );
}
/// A 1 × 1 × 1 box about the origin, selected, Create tab open.
fn app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let item = a.editor.scene.active_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    let id = item.id;
    a.editor.synchronize_identities().unwrap();
    a.editor.undo.clear();
    a.run(Command::SelectItem(Some(id)));
    a.category = toolbox::Category::Create;
    a.create_folded.clear();
    let ctx = Context::default();
    theme::install(&ctx);
    (a, ctx)
}
fn size(a: &Operandi) -> DVec3 {
    a.editor.absolute_box(false).unwrap().size()
}

#[test]
fn the_command_opens_and_closes_the_window() {
    let (mut a, ctx) = app();
    frame(&mut a, &ctx, vec![]);
    // Jason, 2026-11-11: not on the Measure shelf; the Snaps panel has it.
    assert!(!a.toolbox_rects.contains_key("Absolute Scaling"));
    a.execute("Absolute Scaling");
    assert!(a.absolute_open);
    // The window draws its rows without trouble, then the command shuts it.
    frame(&mut a, &ctx, vec![]);
    a.execute("Absolute Scaling");
    assert!(!a.absolute_open);
}

#[test]
fn grab_size_fills_the_fields_without_an_undo_step() {
    let (mut a, _) = app();
    a.execute("scale.absolute mode:grab");
    assert_eq!(a.absolute.size, DVec3::ONE);
    assert_eq!(a.absolute.uniform, 1.);
    assert!(a.editor.undo.is_empty());
}

#[test]
fn explicit_from_a_script_is_one_undo_step_and_refreshes_the_fields() {
    let (mut a, _) = app();
    a.execute("scale.absolute mode:explicit absolute_size_x:2 absolute_size_center_x:lowX");
    assert!(
        (size(&a) - DVec3::new(2., 1., 1.)).length() < 1e-9,
        "{}",
        a.status
    );
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let min_x = mesh.vertices.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    assert_eq!(min_x, -0.5, "Low X stays put");
    assert_eq!(a.absolute.size, DVec3::new(2., 1., 1.));
    assert_eq!(a.absolute.uniform, 2.);
    assert_eq!(a.editor.undo.len(), 1);
    a.run(Command::Undo);
    assert_eq!(size(&a), DVec3::ONE);
}

#[test]
fn uniform_from_the_window_fields() {
    let (mut a, _) = app();
    a.absolute.uniform = 3.;
    assert!(a.absolute_press(AbsoluteMode::Uniform));
    assert!((size(&a) - DVec3::splat(3.)).length() < 1e-9);
    assert!(a.status.starts_with("Uniform Scale"), "{}", a.status);
}

#[test]
fn a_flat_selection_says_which_axis_it_left() {
    let (mut a, _) = app();
    a.run(Command::Mode(Mode::Polygon));
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let top = (0..mesh.faces.len())
        .find(|&f| {
            mesh.faces[f]
                .vertices
                .iter()
                .all(|&v| mesh.vertices[v as usize].y == 0.5)
        })
        .unwrap() as u32;
    a.editor.scene.selection.faces = vec![top];
    a.execute("scale.absolute explicit absolute_size_x:2 absolute_size_y:5 absolute_size_z:1");
    assert!(a.status.contains("flat on Y"), "{}", a.status);
    assert_eq!(a.absolute.size, DVec3::new(2., 0., 1.));
}

#[test]
fn bad_arguments_change_nothing() {
    let (mut a, _) = app();
    a.execute("scale.absolute mode:sideways");
    assert!(a.status.contains("mode"), "{}", a.status);
    a.execute("scale.absolute mode:explicit absolute_size_x:abc");
    assert!(a.editor.undo.is_empty());
    assert_eq!(size(&a), DVec3::ONE);
}
