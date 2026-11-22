//! UV Unwrap... and Pack UVs... in the app (UV spec v1.1): the Edit tab's UV
//! rows open the dialogs at the remembered values, and OK runs the command
//! as one undo step.
use super::tests::{draw, pointer};
use super::*;

/// A 2 x 2 grid of unit quads in XZ, seen from the top.
fn grid_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=2u32 {
        for i in 0..=2u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..2u32 {
        for i in 0..2u32 {
            let v = j * 3 + i;
            mesh.faces
                .push(super::tests::face(vec![v, v + 3, v + 4, v + 1]));
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Top;
    a.frame(false);
    a
}

fn click(a: &mut Operandi, ctx: &Context, size: Vec2, at: Pos2) {
    draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    draw(a, ctx, size, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, size, vec![pointer(at, false, Modifiers::NONE)]);
}

#[test]
fn uv_tab_unwrap_and_pack() {
    let ctx = Context::default();
    let size = vec2(1440., 2200.);
    let mut a = grid_app(&ctx);
    a.category = toolbox::Category::Edit;
    draw(&mut a, &ctx, size, vec![]);
    let row = a.form_rects["UV Unwrap"].center();
    click(&mut a, &ctx, size, row);
    let Some(uv_dialogs::UvDialog::Unwrap(form)) = &a.uv_dialog else {
        panic!("UV Unwrap didn't open: {}", a.status);
    };
    // The spec's starting values (UVW-01), from the remembered options.
    let o = uv_dialogs::unwrap_options(&form.values, &a.editor.uv_unwrap_preset);
    assert_eq!(o, a.editor.uv_unwrap_preset);
    assert_eq!(o.iterations, 100);
    // OK.
    a.uv_dialog = None;
    let steps = a.editor.undo.len();
    a.run(Command::UvUnwrap(o));
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert!(mesh.uv_maps.contains_key("Texture"), "{}", a.status);
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert_eq!(a.editor.undo.last().unwrap().label, "UV Unwrap");

    let row = a.form_rects["Pack UVs"].center();
    click(&mut a, &ctx, size, row);
    let Some(uv_dialogs::UvDialog::Pack(form)) = &a.uv_dialog else {
        panic!("Pack UVs didn't open: {}", a.status);
    };
    let p = uv_dialogs::pack_options(&form.values);
    assert_eq!(p, a.editor.uv_pack_preset);
    assert!(p.stretch && !p.orient);
    a.uv_dialog = None;
    a.run(Command::PackUvs(p));
    assert_eq!(
        a.editor.undo.last().unwrap().label,
        "Pack UVs",
        "{}",
        a.status
    );
}

#[test]
fn typed_names_open_the_dialogs() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    a.execute("UV Unwrap");
    assert!(
        matches!(a.uv_dialog, Some(uv_dialogs::UvDialog::Unwrap(_))),
        "{}",
        a.status
    );
    a.uv_dialog = None;
    a.execute("Pack UVs");
    assert!(
        matches!(a.uv_dialog, Some(uv_dialogs::UvDialog::Pack(_))),
        "{}",
        a.status
    );
}

/// UV view (UVV-01, 04, 05): View > UV View, starting display, a click
/// selects in UV space, Select Connected follows the UV island, A fits.
#[test]
fn uv_view_selects_and_follows_islands() {
    let ctx = Context::default();
    let size = vec2(1440., 1000.);
    let mut a = grid_app(&ctx);
    // Two islands: cut the grid down the middle.
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(1, 4), (4, 7)];
    a.run(Command::UvUnwrap(
        operandi_core::uv_tools::UnwrapOptions::default(),
    ));
    a.run(Command::Mode(Mode::Polygon));
    a.run(Command::ClearSelection);
    a.toggle_uv_view();
    assert!(a.is_uv_view(a.active_view));
    draw(&mut a, &ctx, size, vec![]);
    let v = a.uv_views[&a.active_view].clone();
    assert_eq!(
        (v.center, v.zoom),
        (glam::DVec2::splat(0.5), uv_view::START_ZOOM)
    );
    // Click inside polygon 0's UVs.
    let rows = &a.editor.scene.active().unwrap().mesh.uv_maps["Texture"][0];
    let c = rows.iter().map(|p| p.unwrap()).sum::<glam::DVec2>() / rows.len() as f64;
    let at = v.to_screen(v.rect, c);
    click(&mut a, &ctx, size, at);
    assert_eq!(a.editor.scene.selection.faces, vec![0], "{}", a.status);
    // Select Connected: the island, not the whole mesh.
    a.run(Command::Selection(
        operandi_core::SelectionOperation::Connected,
    ));
    let n = a.editor.scene.selection.faces.len();
    assert!(n == 2, "island of {n} polygons: {}", a.status);
    // A frames the UVs.
    a.frame(false);
    assert_ne!(a.uv_views[&a.active_view].zoom, uv_view::START_ZOOM);
    // Back to 3D.
    a.toggle_uv_view();
    assert!(!a.is_uv_view(a.active_view));
}

/// A UV panel (`Pane::Uv`) draws UV space, can't switch to 3D, and its
/// copy keeps the map and framing.
#[test]
fn uv_panel_draws_uv_and_copies_its_state() {
    use operandi_layout::{Pane, Side};
    let ctx = Context::default();
    let size = vec2(1440., 1000.);
    let mut a = grid_app(&ctx);
    a.run(Command::UvUnwrap(
        operandi_core::uv_tools::UnwrapOptions::default(),
    ));
    let uv = a.views.new_viewport_key();
    a.views
        .insert_beside(Pane::Uv(uv), Pane::Viewport(0), Side::Left)
        .unwrap();
    draw(&mut a, &ctx, size, vec![]);
    assert!(a.is_uv_view(uv));
    a.activate_view(uv);
    a.toggle_uv_view();
    assert!(a.is_uv_view(uv), "a UV panel stays UV");
    a.frame(false);
    let framed = a.uv_views[&uv].zoom;
    let copy = a.copy_view(uv);
    assert_eq!(a.uv_views[&copy].zoom, framed);
}
