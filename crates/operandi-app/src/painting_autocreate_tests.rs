//! The "painting does nothing after Auto Create Textures" report: a bare
//! sphere, the Paint layout, Paintbrush, a press (which offers a texture),
//! the New Image panel's defaults, then strokes, all through real frames.
use super::*;
use crate::tests::{draw, pointer};
use std::sync::Arc;

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

fn stroke(a: &mut Operandi, ctx: &Context, from: Pos2) {
    let mut events = vec![
        vec![Event::PointerMoved(from)],
        vec![pointer(from, true, Modifiers::NONE)],
    ];
    for k in 1..12 {
        events.push(vec![Event::PointerMoved(from + vec2(k as f32 * 4., 0.))]);
    }
    events.push(vec![pointer(from + vec2(44., 0.), false, Modifiers::NONE)]);
    for e in events {
        let _ = draw(a, ctx, SIZE, e);
    }
    frames(a, ctx, 1);
}

fn auto_create_then_paint(subd: Option<operandi_core::subdivision::Toggle>) {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    if let Some(t) = subd {
        a.run(Command::ToggleSubdivision(t));
    }
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("Paint"));
    // The Paint layout turns Paintbrush on.
    assert_eq!(
        a.painting.brush.tool,
        Some(operandi_core::paint_stroke::PaintTool::Paintbrush)
    );
    frames(&mut a, &ctx, 2);
    let c = a.viewport_rect.center();
    stroke(&mut a, &ctx, c);
    assert!(
        a.painting.ask_add,
        "the first press offers a texture: {}",
        a.status
    );
    // "Yes": the New Image panel, OK with its defaults (Set Color off).
    a.painting.ask_add = false;
    a.execute("operandi.paint.addTexture kind:color");
    assert!(a.has_modal(), "the New Image panel");
    // Its OK runs the full line and closes it.
    a.close_new_image_for_test();
    a.execute("operandi.paint.addTexture kind:color resolution:2048 format:rgba float:false");
    frames(&mut a, &ctx, 1);
    let image = a.current_image().expect("a paint target");
    let before = a.editor.scene.paint.buffers[&image].clone();
    stroke(&mut a, &ctx, c);
    let after = a.editor.scene.paint.buffers[&image].clone();
    let layer = a.paint_target();
    let item = a.editor.scene.selection.item;
    let problem = operandi_core::paint::paint_target_problem_on(&a.editor.scene, layer, item);
    assert!(
        !Arc::ptr_eq(&before, &after),
        "the stroke painted: {} | ask {} | problem {problem:?} | layer {layer:?} item {item:?} | tool {:?} | notice {:?} | undo {:?}",
        a.status,
        a.painting.ask_add,
        a.painting.brush.tool,
        a.painting.brush.notice,
        a.editor
            .undo
            .iter()
            .map(|u| u.label.clone())
            .collect::<Vec<_>>()
    );
    let lit = (0..after.pixel_count())
        .filter(|&i| {
            let p = after.get(i);
            p[0] > 0.9 && p[3] > 0.9
        })
        .count();
    assert!(lit > 50, "{lit} white opaque pixels");
    // The view's material shows them.
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
    let m = resident.gpu_materials().expect("the view reads the image");
    assert!(
        m.images
            .iter()
            .any(|img| img.rgba.chunks(4).any(|p| p[0] > 230 && p[3] > 230)),
        "the view's texture carries the paint"
    );
}

#[test]
fn auto_create_then_paint_lands_and_shows_in_the_view() {
    auto_create_then_paint(None);
}

#[test]
fn a_subdivision_sphere_paints_too() {
    auto_create_then_paint(Some(operandi_core::subdivision::Toggle::Subdivision));
}

#[test]
fn a_catmull_clark_sphere_paints_too() {
    auto_create_then_paint(Some(operandi_core::subdivision::Toggle::CatmullClark));
}

#[test]
fn close_the_scene_then_a_new_sphere_auto_creates_and_paints() {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    let ctx = Context::default();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Box,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("Paint"));
    // The Paint layout turns Paintbrush on.
    assert_eq!(
        a.painting.brush.tool,
        Some(operandi_core::paint_stroke::PaintTool::Paintbrush)
    );
    frames(&mut a, &ctx, 2);
    let c = a.viewport_rect.center();
    stroke(&mut a, &ctx, c);
    // Close it (discarding) and start again with a sphere.
    a.close_active_scene();
    frames(&mut a, &ctx, 2);
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    frames(&mut a, &ctx, 2);
    let c = a.viewport_rect.center();
    stroke(&mut a, &ctx, c);
    assert!(a.painting.ask_add, "{}", a.status);
    a.painting.ask_add = false;
    a.execute("operandi.paint.addTexture kind:color");
    a.close_new_image_for_test();
    let t = std::time::Instant::now();
    a.execute("operandi.paint.addTexture kind:color resolution:2048 format:rgba float:false");
    assert!(
        t.elapsed().as_secs() < 20,
        "Add Color Texture took {:?}",
        t.elapsed()
    );
    frames(&mut a, &ctx, 1);
    let image = a.current_image().expect("a paint target");
    let before = a.editor.scene.paint.buffers[&image].clone();
    stroke(&mut a, &ctx, c);
    assert!(
        !Arc::ptr_eq(&before, &a.editor.scene.paint.buffers[&image]),
        "{}",
        a.status
    );
}
