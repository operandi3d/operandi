//! Painting test drive (2026-11-26): the GUI checklists' gestures run
//! through the app's whole frame (`draw`: layout, viewport input, the
//! tool's press, drag and release), one test per workflow the checklists
//! walk through by hand: every live paint tool's stroke is one undo step
//! that undoes and redoes exactly; painted images survive a scene save and
//! reopen; a 4096 image takes strokes without stalling (opt-in probe).
use super::*;
use crate::tests::{draw, pointer};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

/// A sphere with a black texture of `resolution`, laid out and framed.
fn sphere(resolution: u32) -> (Operandi, Context) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute(&format!(
        "operandi.paint.addTexture kind:color resolution:{resolution} color:0,0,0"
    ));
    assert!(a.paint_target().is_some(), "{}", a.status);
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

fn pixels(a: &Operandi) -> Arc<operandi_core::paint::pixels::PaintImage> {
    let image = a.current_image().unwrap();
    a.editor.scene.paint.buffers[&image].clone()
}

/// A left-drag through the view's centre, `steps` moves to the right.
fn stroke(a: &mut Operandi, ctx: &Context, from: Vec2, steps: usize) {
    let c = a.viewport_rect.center() + from;
    let mut frames = vec![
        vec![Event::PointerMoved(c)],
        vec![pointer(c, true, Modifiers::NONE)],
    ];
    for k in 1..=steps {
        frames.push(vec![Event::PointerMoved(c + vec2(k as f32 * 4., 0.))]);
    }
    frames.push(vec![pointer(
        c + vec2(steps as f32 * 4., 0.),
        false,
        Modifiers::NONE,
    )]);
    for events in frames {
        let _ = draw(a, ctx, SIZE, events);
    }
    let _ = draw(a, ctx, SIZE, vec![]);
}

/// A white band down the middle of the sphere, for the tools to cross.
fn vertical(a: &mut Operandi, ctx: &Context) {
    let c = a.viewport_rect.center() + vec2(0., -70.);
    let _ = draw(a, ctx, SIZE, vec![Event::PointerMoved(c)]);
    let _ = draw(a, ctx, SIZE, vec![pointer(c, true, Modifiers::NONE)]);
    for k in 1..=35 {
        let _ = draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(c + vec2(0., k as f32 * 4.))],
        );
    }
    let end = c + vec2(0., 140.);
    let _ = draw(a, ctx, SIZE, vec![pointer(end, false, Modifiers::NONE)]);
    let _ = draw(a, ctx, SIZE, vec![]);
}

/// Stage 2 #2, stage 3 #1-6, stage 5 #7: each live tool's left-drag (or
/// click) is one undo step; Undo puts the picture back exactly and Redo
/// brings the painted one back.
#[test]
fn every_tool_stroke_is_one_step_that_undoes_and_redoes() {
    let (mut a, ctx) = sphere(256);
    // Something to smudge, blur and fill around.
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.set size:30 fg:1,1,1");
    vertical(&mut a, &ctx);
    for (row, tool) in [
        "airbrush",
        "paintbrush",
        "eraser",
        "smudge",
        "blur",
        "sharpen",
        "line",
        "fill",
        "linearGradient",
        "radialGradient",
    ]
    .into_iter()
    .enumerate()
    {
        // Airbrush is already on (picking the lit tool again turns it off).
        if row > 0 {
            a.execute(&format!("operandi.paint.{tool}"));
        }
        if tool == "fill" {
            a.execute("operandi.paint.set fg:1,0,0");
        }
        let before = pixels(&a);
        let steps = a.editor.history.undo.len();
        // Each tool on its own row, crossing the band's edges.
        stroke(&mut a, &ctx, vec2(-30., -55. + row as f32 * 12.), 15);
        assert_eq!(
            a.editor.history.undo.len(),
            steps + 1,
            "{tool}: one undo step; status {}",
            a.status
        );
        let painted = pixels(&a);
        assert_ne!(*painted, *before, "{tool} changed the picture");
        a.execute("app.undo");
        assert_eq!(*pixels(&a), *before, "{tool}: undo is exact");
        a.execute("app.redo");
        assert_eq!(*pixels(&a), *painted, "{tool}: redo brings it back");
        a.execute("operandi.paint.set fg:1,1,1");
    }
}

/// Stage 1 #6: a painted image with a file is saved with the scene (Scene
/// Save Also Saves Changed Images) and the reopened scene shows the paint.
#[test]
fn painted_image_comes_back_after_scene_save_and_reopen() {
    let (mut a, ctx) = sphere(128);
    let dir = std::env::temp_dir().join(format!("operandi-paint-drive-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let image_file = dir.join("skin.png");
    let scene_file = dir.join("painted.opscene");
    a.execute(&format!(
        "operandi.image.saveAs filename:{}",
        image_file.display()
    ));
    assert!(image_file.is_file(), "{}", a.status);
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:40 fg:1,1,1");
    stroke(&mut a, &ctx, vec2(-20., 0.), 15);
    let painted = pixels(&a);
    assert!(
        (0..painted.pixel_count()).any(|i| painted.get(i)[0] > 0.9),
        "paint landed"
    );
    let image = a.current_image().unwrap();
    assert!(a.editor.image_changed(image), "the image has a *");
    assert!(a.save(scene_file.clone()), "{}", a.status);
    assert!(
        !a.editor.image_changed(image),
        "the scene save saved the image too"
    );
    assert!(!a.dirty(), "nothing left unsaved");
    // Reopen in a fresh app, as after a restart.
    let mut b = Operandi::empty();
    b.open(scene_file);
    let reopened = b
        .editor
        .scene
        .images
        .iter()
        .find(|i| i.name == a.editor.scene.images[0].name)
        .map(|i| i.id)
        .expect("the image is in the reopened scene");
    // Pixels load from the file the first time they're asked for.
    let back = b
        .editor
        .image_pixels(reopened)
        .expect("its pixels load from the saved file");
    let white = |p: &operandi_core::paint::pixels::PaintImage| {
        (0..p.pixel_count()).filter(|&i| p.get(i)[0] > 0.9).count()
    };
    assert_eq!(white(&back), white(&painted), "the same paint came back");
    std::fs::remove_dir_all(&dir).ok();
}

/// A few strokes on a 4096 texture (plain sphere): press, slowest drag
/// frame and release times. Opt-in, optimised:
/// `cargo test --profile qa -p operandi-app paint_4096_stroke_probe -- --ignored --nocapture`
#[test]
#[ignore = "speed probe: run optimised with --ignored --nocapture"]
fn paint_4096_stroke_probe() {
    let Some(state) = super::performance_tests::headless_gpu() else {
        println!("no graphics device");
        return;
    };
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    // OPERANDI_PROBE_RESOLUTION compares another size (default 4096).
    let res: u32 = std::env::var("OPERANDI_PROBE_RESOLUTION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4096);
    let t = Instant::now();
    a.execute(&format!(
        "operandi.paint.addTexture kind:color resolution:{res} color:0,0,0"
    ));
    println!(
        "add {res} texture: {:.0} ms",
        t.elapsed().as_secs_f64() * 1e3
    );
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    let t = Instant::now();
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:40");
    for _ in 0..4 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    println!(
        "tool on + first frames: {:.0} ms",
        t.elapsed().as_secs_f64() * 1e3
    );
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    for (n, at) in [
        vec2(0., 0.),
        vec2(-60., 30.),
        vec2(40., -40.),
        vec2(-20., 60.),
    ]
    .into_iter()
    .enumerate()
    {
        let c = a.viewport_rect.center() + at;
        let _ = draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(c)]);
        let t = Instant::now();
        let _ = draw(&mut a, &ctx, SIZE, vec![pointer(c, true, Modifiers::NONE)]);
        let press = t.elapsed();
        let mut worst = Duration::ZERO;
        let mut total = Duration::ZERO;
        for k in 1..30 {
            let t = Instant::now();
            let _ = draw(
                &mut a,
                &ctx,
                SIZE,
                vec![Event::PointerMoved(c + vec2(k as f32 * 4., k as f32))],
            );
            worst = worst.max(t.elapsed());
            total += t.elapsed();
        }
        let t = Instant::now();
        let _ = draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(c + vec2(120., 30.), false, Modifiers::NONE)],
        );
        let release = t.elapsed();
        let t = Instant::now();
        a.execute("app.undo");
        let undo = t.elapsed();
        a.execute("app.redo");
        println!(
            "{res} stroke {n}: press {:.1} ms, drag mean {:.1} ms, slowest {:.1} ms, release {:.1} ms, undo {:.1} ms",
            ms(press),
            ms(total) / 29.,
            ms(worst),
            ms(release),
            ms(undo)
        );
    }
}

/// A 256 image on a sphere filling the view puts its texels several
/// points apart on screen. Smudge and Clone read colour there too (they
/// used to find no texel within two points and paint nothing).
#[test]
fn smudge_and_clone_work_on_a_small_image_seen_close_up() {
    let (mut a, ctx) = sphere(256);
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.set size:30 fg:1,1,1");
    vertical(&mut a, &ctx);
    let changed = |a: &Operandi, before: &operandi_core::paint::pixels::PaintImage| {
        let after = pixels(a);
        (0..after.pixel_count())
            .filter(|&i| after.get(i) != before.get(i))
            .count()
    };
    a.execute("operandi.paint.smudge");
    let before = pixels(&a);
    stroke(&mut a, &ctx, vec2(0., -20.), 15);
    assert!(changed(&a, &before) > 50, "smudge dragged the band along");
    // Clone: Ctrl+click the band (the source), then drag over black.
    a.execute("operandi.paint.clone");
    let source = a.viewport_rect.center();
    let _ = draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(source)]);
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(source, true, Modifiers::CTRL)],
    );
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(source, false, Modifiers::CTRL)],
    );
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    let before = pixels(&a);
    stroke(&mut a, &ctx, vec2(-80., 30.), 10);
    assert!(
        changed(&a, &before) > 50,
        "clone copied the band: {}",
        a.status
    );
}

/// Zoomed out on a 1024 image (dozens of texels per screen point), a
/// Smudge or Clone dab looked up each texel's source by scanning the
/// 16-point cells around it, and one stroke froze the app (test drive
/// 2026-11-27, the live bridge smoke test). It now takes moments.
#[test]
fn smudge_and_clone_stay_quick_zoomed_out_on_a_big_image() {
    let (mut a, ctx) = sphere(1024);
    a.camera.distance *= 8.;
    for _ in 0..2 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.set size:30 fg:1,1,1");
    stroke(&mut a, &ctx, vec2(-15., 0.), 8);
    let t = Instant::now();
    a.execute("operandi.paint.smudge");
    let before = pixels(&a);
    // Down from inside the white line out over the black.
    let c = a.viewport_rect.center();
    let _ = draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(c)]);
    let _ = draw(&mut a, &ctx, SIZE, vec![pointer(c, true, Modifiers::NONE)]);
    for k in 1..=10 {
        let q = c + vec2(0., k as f32 * 3.);
        let _ = draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(q)]);
    }
    let end = c + vec2(0., 30.);
    let _ = draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(end, false, Modifiers::NONE)],
    );
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    assert_ne!(*pixels(&a), *before, "smudge moved paint");
    a.execute("operandi.paint.clone");
    let _ = draw(&mut a, &ctx, SIZE, vec![pointer(c, true, Modifiers::CTRL)]);
    let _ = draw(&mut a, &ctx, SIZE, vec![pointer(c, false, Modifiers::CTRL)]);
    stroke(&mut a, &ctx, vec2(-20., 20.), 8);
    // Generous for an unoptimised build; the frozen version took minutes.
    assert!(
        t.elapsed() < Duration::from_secs(60),
        "took {:?}",
        t.elapsed()
    );
}
