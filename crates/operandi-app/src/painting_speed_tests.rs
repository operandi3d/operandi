//! Painting: how long a stroke's start takes (press to first dab on
//! screen) and its frames, on a Sub-D sphere with a 2048 image. Opt-in
//! probe, run optimised:
//! `cargo test --release -p operandi-app --features painting paint_stroke_speed_probe -- --ignored --nocapture`
use super::*;
use crate::tests::{draw, pointer};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

/// A Sub-D sphere with a black 2048 texture and Paintbrush on, drawn.
pub(crate) fn subd_sphere_2048() -> Option<(Operandi, Context)> {
    subd_sphere(2048)
}

fn subd_sphere(resolution: u32) -> Option<(Operandi, Context)> {
    let state = super::performance_tests::headless_gpu()?;
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::Subdivision,
    ));
    a.execute(&format!(
        "operandi.paint.addTexture kind:color resolution:{resolution} color:0,0,0"
    ));
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set size:20");
    for _ in 0..4 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    Some((a, ctx))
}

/// One stroke: (press frame, slowest drag frame, release frame).
pub(crate) fn timed_stroke(a: &mut Operandi, ctx: &Context, at: egui::Vec2) -> [Duration; 3] {
    let c = a.viewport_rect.center() + at;
    let _ = draw(a, ctx, SIZE, vec![Event::PointerMoved(c)]);
    let t = Instant::now();
    let _ = draw(a, ctx, SIZE, vec![pointer(c, true, Modifiers::NONE)]);
    let press = t.elapsed();
    let mut worst = Duration::ZERO;
    for k in 1..12 {
        let t = Instant::now();
        let _ = draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(c + vec2(k as f32 * 3., 0.))],
        );
        worst = worst.max(t.elapsed());
    }
    let t = Instant::now();
    let _ = draw(
        a,
        ctx,
        SIZE,
        vec![pointer(c + vec2(33., 0.), false, Modifiers::NONE)],
    );
    let release = t.elapsed();
    for _ in 0..2 {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
    [press, worst, release]
}

/// Optimised: every stroke's press frame (first dab on screen) and drag
/// frames stay within a 60 Hz frame (16 ms), its release within two.
/// Before the stroke-start work (2026-11-18): press 260-310 ms, drag
/// frames ~51 ms, release ~48 ms.
#[test]
#[ignore = "speed probe: run with --release --ignored --nocapture"]
fn paint_stroke_speed_probe() {
    let Some((mut a, ctx)) = subd_sphere_2048() else {
        println!("no graphics device");
        return;
    };
    for (n, at) in [
        vec2(0., 0.),
        vec2(-40., 20.),
        vec2(30., -30.),
        vec2(10., 40.),
    ]
    .into_iter()
    .enumerate()
    {
        let [press, worst, release] = timed_stroke(&mut a, &ctx, at);
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        println!(
            "stroke {n}: press {:.1} ms, slowest drag frame {:.1} ms, release {:.1} ms",
            ms(press),
            ms(worst),
            ms(release)
        );
        if !cfg!(debug_assertions) {
            assert!(ms(press) < 16., "stroke {n}: press {:.1} ms", ms(press));
            assert!(
                ms(worst) < 16.,
                "stroke {n}: drag frame {:.1} ms",
                ms(worst)
            );
            assert!(
                ms(release) < 32.,
                "stroke {n}: release {:.1} ms",
                ms(release)
            );
        }
    }
}

/// A press builds nothing the last stroke already had (the texels are kept
/// per mesh, level, map and size, and made before the first press), and a
/// stroke sends only its changed rectangles to the card: no whole picture
/// is uploaded while it runs or at its release.
#[test]
fn a_stroke_starts_without_rebuilding_or_uploading() {
    let Some((mut a, ctx)) = subd_sphere(512) else {
        println!("no graphics device");
        return;
    };
    let builds = operandi_core::paint_stroke::surface_builds();
    let (uploads, patches) = a.viewport_gpu.as_ref().unwrap().image_traffic();
    let painted = |a: &Operandi| {
        let image = a.current_image().unwrap();
        a.editor.scene.paint.buffers[&image].clone()
    };
    let before = painted(&a);
    for at in [vec2(0., 0.), vec2(-30., 10.), vec2(20., -20.)] {
        let _ = timed_stroke(&mut a, &ctx, at);
    }
    assert_eq!(
        operandi_core::paint_stroke::surface_builds(),
        builds,
        "made before the first press, kept after"
    );
    let (now_uploads, now_patches) = a.viewport_gpu.as_ref().unwrap().image_traffic();
    assert_eq!(now_uploads, uploads, "no whole picture uploaded");
    assert!(now_patches > patches, "rectangles written instead");
    // The strokes are recorded, one undo step each.
    assert!(!Arc::ptr_eq(&before, &painted(&a)));
    for _ in 0..3 {
        a.execute("app.undo");
    }
    assert!(Arc::ptr_eq(&before, &painted(&a)), "undone to the start");
    // An undo puts the old picture on the card: a fresh upload.
    for _ in 0..2 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert!(a.viewport_gpu.as_ref().unwrap().image_traffic().0 > uploads);
}
