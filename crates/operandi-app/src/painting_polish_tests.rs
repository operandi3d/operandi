//! Painting stage 6 polish, through real frames and pointer events: the
//! Paint tab's tool-choice flyouts and Alt+click Radial (PNT-111),
//! Ctrl+Shift+click FG pick (PNT-64), the lasso's handles and staying on
//! the model (PNT-119, 120), and the viewport showing a picked PSD layer.
use super::*;
use crate::tests::{draw, draw_held, pointer};
use operandi_core::paint_stroke::PaintTool;

const SIZE: egui::Vec2 = egui::vec2(1400., 1400.);

fn app(colour: &str) -> Operandi {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute(&format!(
        "operandi.paint.addTexture kind:color resolution:256 color:{colour}"
    ));
    a
}

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

fn rect(a: &Operandi, key: &str) -> Rect {
    a.form_rects
        .get(key)
        .or_else(|| a.toolbox_rects.get(key))
        .copied()
        .unwrap_or_else(|| panic!("no {key:?}"))
}

fn click_held(a: &mut Operandi, ctx: &Context, p: Pos2, mods: Modifiers) {
    let _ = draw_held(a, ctx, SIZE, vec![Event::PointerMoved(p)], mods);
    let _ = draw_held(
        a,
        ctx,
        SIZE,
        vec![pointer(p, true, mods), pointer(p, false, mods)],
        mods,
    );
    frames(a, ctx, 1);
}

fn click_key(a: &mut Operandi, ctx: &Context, key: &str, mods: Modifiers) {
    let p = rect(a, key).center();
    click_held(a, ctx, p, mods);
}

fn right_click(a: &mut Operandi, ctx: &Context, key: &str) {
    let p = rect(a, key).center();
    let button = |pressed| Event::PointerButton {
        pos: p,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    let _ = draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    let _ = draw(a, ctx, SIZE, vec![button(true), button(false)]);
    frames(a, ctx, 1);
}

fn paint_tab(colour: &str, ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = app(colour);
    frames(&mut a, ctx, 2);
    a.execute("operandi.paint.tab");
    frames(&mut a, ctx, 3);
    a
}

#[test]
fn the_paint_tab_has_tool_choice_flyouts_that_keep_their_pick() {
    let ctx = Context::default();
    let mut a = paint_tab("0,0,0", &ctx);
    for key in [
        "flyout:Fill",
        "flyout:Blur",
        "flyout:Procedural",
        "flyout:Inks",
    ] {
        assert!(a.form_rects.contains_key(key), "{key}");
    }
    right_click(&mut a, &ctx, "flyout:Fill");
    for key in [
        "operandi.paint.fill",
        "operandi.paint.linearGradient",
        "operandi.paint.radialGradient",
    ] {
        assert!(a.form_rects.contains_key(key), "{key} in the list");
    }
    click_key(
        &mut a,
        &ctx,
        "operandi.paint.radialGradient",
        Modifiers::NONE,
    );
    assert_eq!(
        a.painting.brush.tool,
        Some(PaintTool::RadialGradient),
        "{}",
        a.status
    );
    // The face keeps Radial: clicking it turns the tool off, then on.
    click_key(&mut a, &ctx, "flyout:Fill", Modifiers::NONE);
    assert_eq!(a.painting.brush.tool, None);
    click_key(&mut a, &ctx, "flyout:Fill", Modifiers::NONE);
    assert_eq!(a.painting.brush.tool, Some(PaintTool::RadialGradient));
    // Blur / Sharp.
    right_click(&mut a, &ctx, "flyout:Blur");
    click_key(&mut a, &ctx, "operandi.paint.sharpen", Modifiers::NONE);
    assert_eq!(a.painting.brush.tool, Some(PaintTool::Sharpen));
    // The inks' flyout adds inks.
    right_click(&mut a, &ctx, "flyout:Inks");
    click_key(
        &mut a,
        &ctx,
        "operandi.paint.ink kind:random",
        Modifiers::NONE,
    );
    assert!(
        a.painting
            .brush
            .ink
            .inks
            .has(operandi_core::paint_inks::InkKind::Random)
    );
}

#[test]
fn alt_click_on_linear_gradient_gives_radial() {
    let ctx = Context::default();
    let mut a = paint_tab("0,0,0", &ctx);
    right_click(&mut a, &ctx, "flyout:Fill");
    click_key(
        &mut a,
        &ctx,
        "operandi.paint.linearGradient",
        Modifiers::NONE,
    );
    assert_eq!(a.painting.brush.tool, Some(PaintTool::LinearGradient));
    a.execute("operandi.paint.linearGradient");
    assert_eq!(a.painting.brush.tool, None);
    click_key(&mut a, &ctx, "flyout:Fill", Modifiers::ALT);
    assert_eq!(
        a.painting.brush.tool,
        Some(PaintTool::RadialGradient),
        "{}",
        a.status
    );
}

#[test]
fn ctrl_shift_click_picks_the_fg_colour_from_the_view() {
    let ctx = Context::default();
    let mut a = paint_tab("1,0,0", &ctx);
    a.execute("operandi.paint.paintbrush");
    frames(&mut a, &ctx, 1);
    let pick = Modifiers {
        shift: true,
        ..crate::tests::PLATFORM_CMD
    };
    let pick = if cfg!(target_os = "macos") {
        Modifiers {
            ctrl: true,
            mac_cmd: false,
            command: false,
            ..pick
        }
    } else {
        pick
    };
    let c = a.viewport_rect.center();
    click_held(&mut a, &ctx, c, pick);
    let fg = a.painting.brush.settings(PaintTool::Paintbrush).fg.color;
    assert!(
        fg[0] > 0.9 && fg[1] < 0.1 && fg[2] < 0.1,
        "{fg:?} {}",
        a.status
    );
}

#[test]
fn the_lasso_has_handles_and_stays_on_the_model_when_the_view_moves() {
    let ctx = Context::default();
    let mut a = paint_tab("0,0,0", &ctx);
    a.execute("operandi.paint.lassoSet style:rectangle");
    a.execute("operandi.paint.lasso");
    let c = a.viewport_rect.center();
    let (from, to) = (c - vec2(60., 60.), c + vec2(60., 60.));
    for e in [
        vec![Event::PointerMoved(from)],
        vec![pointer(from, true, Modifiers::NONE)],
        vec![Event::PointerMoved(c)],
        vec![Event::PointerMoved(to)],
        vec![pointer(to, false, Modifiers::NONE)],
    ] {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    let drawn = a.painting.brush.lasso.clone().expect("drawn");
    assert!(
        drawn.view.is_some(),
        "it remembers the view it was drawn in"
    );
    a.execute("operandi.paint.lasso");
    a.execute("operandi.paint.paintbrush");
    frames(&mut a, &ctx, 1);
    // Drag the centre handle 30 points right.
    let r = a.viewport_rect;
    let handles = a.lasso_handles(r).expect("handles show");
    let h = handles[0].1;
    for e in [
        vec![Event::PointerMoved(h)],
        vec![pointer(h, true, Modifiers::NONE)],
        vec![Event::PointerMoved(h + vec2(15., 0.))],
        vec![Event::PointerMoved(h + vec2(30., 0.))],
        vec![pointer(h + vec2(30., 0.), false, Modifiers::NONE)],
    ] {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    let moved = a.painting.brush.lasso.clone().unwrap();
    let (lo0, _) = drawn.bounds().unwrap();
    let (lo1, _) = moved.bounds().unwrap();
    assert!(
        (lo1.x - lo0.x - 30.).abs() < 1.,
        "{lo0} -> {lo1}: {}",
        a.status
    );
    assert_eq!(a.status, "Lasso adjusted", "the drag ran its command");
    // Orbit the view: the handles go, and the lasso still weighs the
    // surface through the view it was drawn in.
    a.camera.view = View::Right;
    frames(&mut a, &ctx, 1);
    assert!(a.lasso_handles(a.viewport_rect).is_none());
    let v = moved.view.unwrap();
    let inside = moved.points[0].lerp(moved.points[1], 0.5);
    let on_model = (0..2000)
        .map(|k| {
            let t = k as f64 / 2000. * std::f64::consts::TAU;
            glam::DVec3::new(0.5 * t.cos(), 0.5 * t.sin(), 0.5)
        })
        .find(|p| v.project(*p).is_some_and(|(q, _)| q.distance(inside) < 40.));
    if let Some(p) = on_model {
        assert!(moved.weight_for(glam::DVec2::new(-999., -999.), p) > 0.);
    }
}

#[test]
fn the_viewport_shows_the_picked_psd_layer_not_the_composite() {
    use operandi_core::paint::pixels::psd::{LayerKind, PsdDoc};
    use operandi_core::shader_tree::{NodeKind, TextureSource};
    let mut a = app("1,0,0");
    let image = a.current_image().unwrap();
    let layer = a.paint_target().unwrap();
    // A Photoshop document whose layer is red; the composite turned green.
    let red = a.editor.scene.paint.buffers[&image].clone();
    let doc = PsdDoc::new_from(&red, false).unwrap();
    let name = doc
        .rows()
        .into_iter()
        .find(|r| {
            r.kind == LayerKind::Pixel
                && r.pick != operandi_core::paint::pixels::psd::Pick::Composite
        })
        .expect("a pixel layer")
        .name;
    a.editor
        .scene
        .paint
        .psd
        .insert(image, std::sync::Arc::new(doc));
    let mut green = (*red).clone();
    for i in 0..green.pixel_count() {
        green.set(i, [0., 1., 0., 1.]);
    }
    a.editor
        .scene
        .paint
        .buffers
        .insert(image, std::sync::Arc::new(green));
    let shows = |a: &Operandi| -> [u8; 4] {
        let scene = &a.editor.scene;
        let mut resident = crate::gpu_scene::Resident::default();
        resident.sync_materials(scene, false);
        let item = scene.active().unwrap();
        let _ = resident.item(
            item,
            scene.world_matrix(item.id).unwrap(),
            None,
            None,
            &scene.smoothing,
        );
        let m = resident.gpu_materials().expect("an image");
        let img = m.images.iter().find(|i| i.rgba.len() >= 4).unwrap();
        [img.rgba[0], img.rgba[1], img.rgba[2], img.rgba[3]]
    };
    assert!(shows(&a)[1] > 200, "the composite without a picked layer");
    let mut node = a.editor.scene.shader_tree.get(layer).unwrap().clone();
    if let NodeKind::Texture(t) = &mut node.kind
        && let TextureSource::ImageMap { settings, .. } = &mut t.source
    {
        settings.psd_layer = Some(name);
    }
    a.editor.scene.shader_tree.replace(node).unwrap();
    let px = shows(&a);
    assert!(px[0] > 200 && px[1] < 50, "the picked layer: {px:?}");
}
