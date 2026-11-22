//! Painting stage 6: painting in the UV view (PNT-215), acceptance 30,
//! driven through real frames and pointer events in the UV layout.
use super::*;
use crate::tests::{draw, pointer};
use std::sync::Arc;

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn app() -> Operandi {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    a
}

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

fn uv_rect(a: &Operandi) -> Rect {
    a.uv_views
        .values()
        .map(|v| v.rect)
        .find(|r| r.is_positive())
        .expect("a UV view is on screen")
}

fn white(px: &operandi_core::paint::pixels::PaintImage) -> usize {
    (0..px.pixel_count())
        .filter(|&i| px.get(i)[0] > 0.5)
        .count()
}

#[test]
fn acceptance_30_painting_in_the_uv_view_changes_the_image_and_the_3d_view_at_once() {
    let mut a = app();
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("UV"));
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:12");
    frames(&mut a, &ctx, 3);
    let r = uv_rect(&a);
    let image = a.current_image().unwrap();
    let before = a.editor.scene.paint.buffers[&image].clone();
    let steps = a.editor.undo.len();
    let c = r.center();
    let mut events = vec![
        vec![Event::PointerMoved(c)],
        vec![pointer(c, true, Modifiers::NONE)],
    ];
    for k in 1..12 {
        events.push(vec![Event::PointerMoved(c + vec2(k as f32 * 4., 0.))]);
    }
    for e in events {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    // Live: the scene's image (what the 3D view draws) already changed.
    std::thread::sleep(std::time::Duration::from_millis(40));
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(c + vec2(50., 0.))],
    );
    let live = a.editor.scene.paint.buffers[&image].clone();
    assert!(!Arc::ptr_eq(&live, &before), "{}", a.status);
    assert!(white(&live) > 0, "paint shows during the stroke");
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(c + vec2(50., 0.), false, Modifiers::NONE)],
    );
    frames(&mut a, &ctx, 1);
    assert_eq!(a.editor.undo.len(), steps + 1, "one step: {}", a.status);
    assert_eq!(a.editor.undo.last().unwrap().label, "Paintbrush Stroke");
    let after = a.editor.scene.paint.buffers[&image].clone();
    // The UV view's centre is UV (0.5, 0.5): the middle row of the image.
    let (w, h) = (after.width as usize, after.height as usize);
    let mid = (h / 2) * w + w / 2 + 2;
    assert!(after.get(mid)[0] > 0.5, "painted where the UV view showed");
    // The 3D view's material reads the same image.
    let mut resident = crate::gpu_scene::Resident::default();
    let scene = &a.editor.scene;
    resident.sync_materials(scene, false);
    let item = scene.active().unwrap();
    let _ = resident.item(
        item,
        scene.world_matrix(item.id).unwrap(),
        None,
        None,
        &scene.smoothing,
    );
    let m = resident
        .gpu_materials()
        .expect("the viewport reads the image");
    assert!(
        m.images
            .iter()
            .any(|img| img.rgba.chunks(4).any(|p| p[0] > 200)),
        "the 3D view's texture carries the paint"
    );
    // The UV selection didn't change while painting.
    a.execute("app.undo");
    assert!(Arc::ptr_eq(&a.editor.scene.paint.buffers[&image], &before));
}

#[test]
fn fill_in_the_uv_view_is_one_click() {
    let mut a = app();
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("UV"));
    a.execute("operandi.paint.fill");
    frames(&mut a, &ctx, 3);
    let r = uv_rect(&a);
    let image = a.current_image().unwrap();
    let steps = a.editor.undo.len();
    for e in [
        vec![Event::PointerMoved(r.center())],
        vec![pointer(r.center(), true, Modifiers::NONE)],
        vec![pointer(r.center(), false, Modifiers::NONE)],
    ] {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    frames(&mut a, &ctx, 1);
    let labels: Vec<String> = a.editor.undo.iter().map(|u| u.label.clone()).collect();
    assert_eq!(a.editor.undo.len(), steps + 1, "{} {labels:?}", a.status);
    let px = &a.editor.scene.paint.buffers[&image];
    assert_eq!(white(px), px.pixel_count(), "the whole black image filled");
}

#[test]
fn without_a_paint_tool_the_uv_view_selects_as_before() {
    let mut a = app();
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("UV"));
    frames(&mut a, &ctx, 3);
    let r = uv_rect(&a);
    let image = a.current_image().unwrap();
    let before = a.editor.scene.paint.buffers[&image].clone();
    let c = r.center();
    for e in [
        vec![Event::PointerMoved(c)],
        vec![pointer(c, true, Modifiers::NONE)],
        vec![Event::PointerMoved(c + vec2(30., 0.))],
        vec![pointer(c + vec2(30., 0.), false, Modifiers::NONE)],
    ] {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    assert!(Arc::ptr_eq(&a.editor.scene.paint.buffers[&image], &before));
    assert!(!a.painting.brush.uv_live);
}
