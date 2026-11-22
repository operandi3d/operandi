//! Painting on Sub-D meshes (Tab), through real input, with the picture
//! read back from the graphics card: paint lands under the brush and
//! shows on the patch-drawn surface.
use super::*;
use crate::tests::{draw, pointer};

const SIZE: egui::Vec2 = egui::vec2(900., 600.);

fn bright(rgba: &[u8], size: [u32; 2], at: Pos2, r: i32) -> f32 {
    let (w, h) = (size[0] as i32, size[1] as i32);
    let mut sum = 0f32;
    let mut n = 0f32;
    for dy in -r..=r {
        for dx in -r..=r {
            let (x, y) = (at.x as i32 + dx, at.y as i32 + dy);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            sum += f32::from(rgba[i]) + f32::from(rgba[i + 1]) + f32::from(rgba[i + 2]);
            n += 3.;
        }
    }
    sum / n.max(1.)
}

fn paint_subd(tool: Tool, level_steps: usize) -> Option<(f32, f32, usize)> {
    paint_subd_with(tool, level_steps, false)
}

fn paint_subd_with(tool: Tool, level_steps: usize, subd_first: bool) -> Option<(f32, f32, usize)> {
    paint_subd_at(tool, level_steps, subd_first, vec2(0., 0.))
}

fn paint_subd_at(
    tool: Tool,
    level_steps: usize,
    subd_first: bool,
    offset: egui::Vec2,
) -> Option<(f32, f32, usize)> {
    paint_subd_path(tool, level_steps, subd_first, offset, true)
}

/// `patches`: force the GPU patch path (tests), else the app's own choice
/// (small surfaces draw through the stored, CPU-refined path).
fn paint_subd_path(
    tool: Tool,
    level_steps: usize,
    subd_first: bool,
    offset: egui::Vec2,
    patches: bool,
) -> Option<(f32, f32, usize)> {
    let state = super::performance_tests::headless_gpu()?;
    crate::subd_gpu::set_min_pieces(if patches {
        0
    } else {
        operandi_core::subdivision::PATCH_MIN_PIECES
    });
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    if subd_first {
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::Subdivision,
        ));
        a.execute("operandi.paint.addTexture kind:color resolution:2048 color:0,0,0");
    } else {
        a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::Subdivision,
        ));
    }
    for _ in 0..level_steps {
        a.execute("subdivision.levelUp");
    }
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:20");
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    let c = a.viewport_rect.center() + offset;
    let before = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    let mut events = vec![
        vec![Event::PointerMoved(c)],
        vec![pointer(c, true, Modifiers::NONE)],
    ];
    for k in 1..6 {
        events.push(vec![Event::PointerMoved(c + vec2(k as f32 * 2., 0.))]);
    }
    for e in events {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    // Mid-stroke: the live paint is due again after 33 ms.
    std::thread::sleep(std::time::Duration::from_millis(40));
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(c + vec2(11., 0.))],
    );
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    let during = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(c + vec2(11., 0.), false, Modifiers::NONE)],
    );
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    let after = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    let patched = a.viewport_gpu.as_ref().unwrap().patched_items();
    // The view rect's centre in the read-back picture (its own pixels).
    let scale = before.1[0] as f32 / a.viewport_rect.width();
    let at = pos2(
        (c.x - a.viewport_rect.left() + 5.) * scale,
        (c.y - a.viewport_rect.top()) * scale,
    );
    eprintln!(
        "{tool:?} before {} during {} after {}",
        bright(&before.2, before.1, at, 3),
        bright(&during.2, during.1, at, 3),
        bright(&after.2, after.1, at, 3)
    );
    assert!(
        bright(&during.2, during.1, at, 3) > bright(&before.2, before.1, at, 3) + 40.,
        "paint shows while dragging"
    );
    Some((
        bright(&before.2, before.1, at, 3),
        bright(&after.2, after.1, at, 3),
        patched,
    ))
}

#[test]
fn painting_a_subd_sphere_shows_under_the_brush() {
    let Some((before, after, patched)) = paint_subd(Tool::Sphere, 0) else {
        println!("no graphics device");
        return;
    };
    assert_eq!(patched, 1, "drawn as patches");
    assert!(after > before + 40., "before {before} after {after}");
}

#[test]
fn painting_a_subd_cube_shows_under_the_brush() {
    let Some((before, after, patched)) = paint_subd(Tool::Box, 0) else {
        println!("no graphics device");
        return;
    };
    assert_eq!(patched, 1, "drawn as patches");
    assert!(after > before + 40., "before {before} after {after}");
}

#[test]
fn painting_a_subd_sphere_at_a_higher_level_shows_under_the_brush() {
    let Some((before, after, _)) = paint_subd(Tool::Sphere, 2) else {
        println!("no graphics device");
        return;
    };
    assert!(after > before + 40., "before {before} after {after}");
}

#[test]
fn subd_first_then_a_new_texture_paints_too() {
    let Some((before, after, _)) = paint_subd_with(Tool::Sphere, 0, true) else {
        println!("no graphics device");
        return;
    };
    assert!(after > before + 40., "before {before} after {after}");
}

#[test]
fn off_centre_paint_lands_under_the_brush_on_a_subd_cube() {
    for offset in [vec2(40., 25.), vec2(-45., -30.), vec2(55., 0.)] {
        let Some((before, after, _)) = paint_subd_at(Tool::Box, 0, false, offset) else {
            println!("no graphics device");
            return;
        };
        assert!(
            after > before + 40.,
            "{offset:?}: before {before} after {after}"
        );
    }
}

/// The app's own path for a small Sub-D sphere (no forced patches), as
/// the shipped app draws it.
#[test]
fn a_subd_sphere_on_the_apps_own_path_shows_paint_under_the_brush() {
    let Some((before, after, patched)) =
        paint_subd_path(Tool::Sphere, 0, true, vec2(0., 0.), false)
    else {
        println!("no graphics device");
        return;
    };
    // The stored CPU surface (a small mesh draws no patches); the drag's
    // frames are checked inside `paint_subd_path`.
    assert_eq!(patched, 0, "drawn as the stored surface");
    assert!(after > before + 40., "before {before} after {after}");
}
