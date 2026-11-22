//! Drag timings on a generated 1M-polygon scene (perf task 1, 2026-11-13),
//! waiting for the viewport's own graphics device every frame, so a frame
//! includes the GPU drawing. The scenes come from
//! `operandi-core/tests/perf_snapshot.rs`; the helpers follow
//! `perf_snapshot_tests.rs`, kept apart so that file stays as measured.
use super::*;
use std::time::Instant;

pub(super) struct Bench {
    pub(super) ctx: Context,
    pub(super) device: egui_wgpu::wgpu::Device,
    pub(super) size: Vec2,
}

impl Bench {
    /// One frame, GPU finished; ms.
    pub(super) fn frame(&self, a: &mut Operandi, events: Vec<Event>) -> f64 {
        let t = Instant::now();
        std::hint::black_box(super::tests::draw(a, &self.ctx, self.size, events));
        let _ = self
            .device
            .poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
        t.elapsed().as_secs_f64() * 1000.
    }
}

pub(super) fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}

/// avg / p95 / max of per-frame times, and the fps the average gives.
pub(super) fn report(name: &str, times: &mut [f64]) {
    times.sort_by(f64::total_cmp);
    let n = times.len().max(1);
    let avg = times.iter().sum::<f64>() / n as f64;
    let p95 = times[((n as f64 * 0.95).ceil() as usize).clamp(1, n) - 1];
    println!(
        "PERF {name} avg_ms={avg:.2} p95_ms={p95:.2} max_ms={:.2} fps={:.1} frames={n}",
        times[n - 1],
        1000. / avg.max(1e-3)
    );
}

pub(super) fn one(name: &str, value: f64) {
    println!("PERF {name} ms={value:.1}");
}

/// Press on the tool's handle (or anywhere off it), drag `frames` frames,
/// release; returns the drag frame times and the release (drop) time.
pub(super) fn drag(b: &Bench, a: &mut Operandi, frames: usize) -> (Vec<f64>, f64) {
    // OPERANDI_PERF_DRAG_FRAMES: longer drags, to attach a sampling profiler.
    let frames = std::env::var("OPERANDI_PERF_DRAG_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(frames);
    for _ in 0..2 {
        b.frame(a, vec![]);
    }
    let r = a.viewport_rect;
    let start = (0..200)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.left_top() + vec2(x as f32 * 4. + 20., y as f32 * 4. + 20.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .unwrap_or(r.center() + vec2(-r.width() * 0.4, r.height() * 0.4));
    b.frame(
        a,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let mut times = Vec::with_capacity(frames);
    let mut end = start;
    let before = a.rebuild_totals();
    for k in 0..frames {
        let s = (k % 40) as f32;
        end = start + vec2(s * 0.7, s * -0.5);
        times.push(b.frame(a, vec![Event::PointerMoved(end)]));
        if k == 4 {
            // What the drag's steady frames rebuild, after its first ones.
            println!(
                "PERF   rebuilds over first 5 frames: {:?}",
                a.rebuild_totals()
            );
        }
    }
    println!(
        "PERF   drag dragging={} before={before:?} after={:?}",
        a.navigation.is_some(),
        a.rebuild_totals()
    );
    let release = b.frame(a, vec![super::tests::pointer(end, false, Modifiers::NONE)]);
    (times, release)
}

/// Undo then redo of the last step, each with the frame that shows it.
pub(super) fn undo_redo(b: &Bench, a: &mut Operandi, name: &str) {
    let t = Instant::now();
    a.run(Command::Undo);
    b.frame(a, vec![]);
    one(&format!("{name}/undo"), ms(t));
    let t = Instant::now();
    a.run(Command::Redo);
    b.frame(a, vec![]);
    one(&format!("{name}/redo"), ms(t));
}

/// The 1M-polygon drags alone (no orbit or mode switches), for working on
/// drag speed: `OPERANDI_PERF_DIR=<folder> cargo test --release -p
/// operandi-app perf_drags -- --ignored --nocapture`. Opens `edit1m` (or
/// `OPERANDI_PERF_SCENES`' first scene) and runs each drag, its drop, undo
/// and redo.
#[test]
#[ignore = "opt-in drag timings on a generated scene (OPERANDI_PERF_DIR)"]
fn perf_drags() {
    let Some(dir) = std::env::var_os("OPERANDI_PERF_DIR").map(PathBuf::from) else {
        println!("OPERANDI_PERF_DIR not set");
        return;
    };
    let name = std::env::var("OPERANDI_PERF_SCENES").unwrap_or_else(|_| "edit1m".into());
    let name = name.split(',').next().unwrap_or("edit1m").to_owned();
    let state = super::performance_tests::headless_gpu().expect("graphics device");
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), 2f32));
    let b = Bench {
        ctx,
        device: state.device.clone(),
        size: vec2(1440., 900.),
    };
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.camera.view = crate::camera::View::Perspective;
    b.frame(&mut a, vec![]);
    a.open(dir.join(format!("{name}.opscene")));
    b.frame(&mut a, vec![]);
    let polygons = a.editor.scene.items[0].mesh.faces.len();
    a.run(Command::Mode(Mode::Polygon));
    a.frame(false);
    b.frame(&mut a, vec![]);
    let mut still: Vec<f64> = (0..30).map(|_| b.frame(&mut a, vec![])).collect();
    report(&format!("{name}/still_frame"), &mut still);
    let only = std::env::var("OPERANDI_PERF_DRAG").ok();
    for (label, mode, count) in [
        ("drag_1_polygon", Mode::Polygon, 1usize),
        ("drag_10k_polygons", Mode::Polygon, 10_000),
        ("drag_100_points", Mode::Vertex, 100),
    ] {
        if only.as_deref().is_some_and(|o| o != label) {
            continue;
        }
        a.run(Command::Mode(mode));
        let mid = polygons / 2;
        match mode {
            Mode::Vertex => {
                let mesh = &a.editor.scene.active().unwrap().mesh;
                let start = mesh.faces[mid].vertices[0] as usize;
                a.editor.scene.selection.vertices =
                    (start..start + count).map(|v| v as u32).collect();
            }
            _ => {
                a.editor.scene.selection.faces = (mid..mid + count).map(|f| f as u32).collect();
            }
        }
        a.activate(Tool::Move);
        let (mut times, release) = drag(&b, &mut a, 60);
        report(&format!("{name}/{label}"), &mut times);
        one(&format!("{name}/{label}_release"), release);
        // OPERANDI_PERF_UNDO_LOOPS: undo and redo again and again, to
        // attach a sampling profiler.
        let loops = std::env::var("OPERANDI_PERF_UNDO_LOOPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        for _ in 0..loops {
            undo_redo(&b, &mut a, &format!("{name}/{label}"));
        }
        a.run(Command::Drop);
        b.frame(&mut a, vec![]);
    }
}
