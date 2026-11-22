//! Performance snapshot (Jason, 2026-11-12): whole-frame timings on the
//! scenes written by `operandi-core/tests/perf_snapshot.rs`, on a real
//! (headless) GPU, waiting for the GPU to finish every frame. Each frame is
//! the app's own per-frame work (input, panels, viewport, picking, GPU
//! upload and draw) at 1440 × 900 points, Retina scale 2.
//!
//! `OPERANDI_PERF_DIR=<folder> OPERANDI_PERF_SCENES=sub10m,edit1m cargo test
//! --release -p operandi-app perf_snapshot -- --ignored --nocapture`
use super::*;
use std::time::Instant;

struct Bench {
    ctx: Context,
    device: egui_wgpu::wgpu::Device,
    size: Vec2,
}

impl Bench {
    /// One frame, GPU finished; ms.
    fn frame(&self, a: &mut Operandi, events: Vec<Event>) -> f64 {
        let t = Instant::now();
        std::hint::black_box(super::tests::draw(a, &self.ctx, self.size, events));
        let _ = self
            .device
            .poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
        t.elapsed().as_secs_f64() * 1000.
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}

/// avg / p95 / max of per-frame times, and the fps the average gives.
fn report(name: &str, times: &mut [f64]) {
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

fn one(name: &str, value: f64) {
    println!("PERF {name} ms={value:.1}");
}

/// Press on the tool's handle (or anywhere off it), drag `frames` frames,
/// release; returns the drag frame times and the release (drop) time.
fn drag(b: &Bench, a: &mut Operandi, frames: usize) -> (Vec<f64>, f64) {
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
fn undo_redo(b: &Bench, a: &mut Operandi, name: &str) {
    let t = Instant::now();
    a.run(Command::Undo);
    b.frame(a, vec![]);
    one(&format!("{name}/undo"), ms(t));
    let t = Instant::now();
    a.run(Command::Redo);
    b.frame(a, vec![]);
    one(&format!("{name}/redo"), ms(t));
}

fn orbit(b: &Bench, a: &mut Operandi, name: &str, frames: usize) {
    b.frame(a, vec![]);
    let mut times: Vec<f64> = (0..frames)
        .map(|_| {
            a.camera.yaw += std::f64::consts::TAU / frames as f64;
            b.frame(a, vec![])
        })
        .collect();
    report(&format!("{name}/orbit"), &mut times);
    let mut times: Vec<f64> = (0..frames / 2)
        .map(|k| {
            let s = if k % 20 < 10 { 1. } else { -1. };
            a.camera.target.x += s * a.camera.distance * 0.002;
            b.frame(a, vec![])
        })
        .collect();
    report(&format!("{name}/pan"), &mut times);
}

fn snapshot_scene(b: &Bench, path: &std::path::Path, name: &str) {
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(
        super::performance_tests::headless_gpu().expect("graphics device"),
    ));
    a.camera.view = crate::camera::View::Perspective;
    b.frame(&mut a, vec![]);
    let t = Instant::now();
    a.open(path.to_path_buf());
    let opened = ms(t);
    let first = b.frame(&mut a, vec![]);
    one(&format!("{name}/open_file"), opened);
    one(&format!("{name}/open_first_frame"), first);
    one(&format!("{name}/open_total"), opened + first);
    let items = a.editor.scene.items.len();
    let polygons: usize = a
        .editor
        .scene
        .items
        .iter()
        .map(|i| i.mesh.faces.len())
        .sum();
    println!("PERF {name}/info items={items} polygons={polygons}");

    // Viewport navigation in Item mode and in Polygon mode.
    a.run(Command::Mode(Mode::Item));
    orbit(b, &mut a, &format!("{name}/item_mode"), 120);
    a.run(Command::Mode(Mode::Polygon));
    b.frame(&mut a, vec![]);
    orbit(b, &mut a, &format!("{name}/polygon_mode"), 120);
    a.frame(false);
    b.frame(&mut a, vec![]);

    // Switching selection modes, each with the frame that shows it.
    for mode in [
        Mode::Vertex,
        Mode::Edge,
        Mode::Polygon,
        Mode::Item,
        Mode::Polygon,
    ] {
        let t = Instant::now();
        a.run(Command::Mode(mode));
        b.frame(&mut a, vec![]);
        one(&format!("{name}/mode_switch_{mode:?}"), ms(t));
    }

    if items == 1 {
        // Component selection: hover and click a polygon in the middle.
        let c = a.viewport_rect.center();
        let mut hover: Vec<f64> = (0..40)
            .map(|k| {
                let p = c + vec2((k % 10) as f32 * 3., (k / 10) as f32 * 3.);
                b.frame(&mut a, vec![Event::PointerMoved(p)])
            })
            .collect();
        report(&format!("{name}/polygon_hover"), &mut hover);
        let t = Instant::now();
        b.frame(
            &mut a,
            vec![super::tests::pointer(c, true, Modifiers::NONE)],
        );
        b.frame(
            &mut a,
            vec![super::tests::pointer(c, false, Modifiers::NONE)],
        );
        one(&format!("{name}/polygon_click_select"), ms(t));
        println!(
            "PERF {name}/selected_after_click faces={}",
            a.editor.scene.selection.faces.len()
        );
        // Component drags: a few polygons, then 10k, then every point.
        for (label, mode, count) in [
            ("drag_1_polygon", Mode::Polygon, 1usize),
            ("drag_10k_polygons", Mode::Polygon, 10_000),
            ("drag_100_points", Mode::Vertex, 100),
        ] {
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
                    let start = if a.editor.scene.selection.faces.len() == 1 && count == 1 {
                        a.editor.scene.selection.faces[0] as usize
                    } else {
                        mid
                    };
                    a.editor.scene.selection.faces =
                        (start..start + count).map(|f| f as u32).collect();
                }
            }
            a.activate(Tool::Move);
            let (mut times, release) = drag(b, &mut a, 60);
            report(&format!("{name}/{label}"), &mut times);
            one(&format!("{name}/{label}_release"), release);
            undo_redo(b, &mut a, &format!("{name}/{label}"));
            a.run(Command::Drop);
            b.frame(&mut a, vec![]);
        }
    } else {
        // Many objects: click one, then drag one, then drag them all.
        a.run(Command::Mode(Mode::Item));
        a.frame(false);
        b.frame(&mut a, vec![]);
        let c = a.viewport_rect.center();
        let t = Instant::now();
        b.frame(
            &mut a,
            vec![
                Event::PointerMoved(c),
                super::tests::pointer(c, true, Modifiers::NONE),
            ],
        );
        b.frame(
            &mut a,
            vec![super::tests::pointer(c, false, Modifiers::NONE)],
        );
        one(&format!("{name}/item_click_select"), ms(t));
        let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
        let t = Instant::now();
        a.editor.scene.select_item(Some(ids[ids.len() / 2]));
        b.frame(&mut a, vec![]);
        one(&format!("{name}/item_select_one"), ms(t));
        a.activate(Tool::Move);
        let (mut times, release) = drag(b, &mut a, 60);
        report(&format!("{name}/drag_1_item"), &mut times);
        one(&format!("{name}/drag_1_item_release"), release);
        undo_redo(b, &mut a, &format!("{name}/drag_1_item"));
        a.run(Command::Drop);
        let t = Instant::now();
        a.editor.scene.selection.item = Some(ids[0]);
        a.editor.scene.selection.also = ids[1..].to_vec();
        b.frame(&mut a, vec![]);
        one(&format!("{name}/item_select_all"), ms(t));
        a.activate(Tool::Move);
        let (mut times, release) = drag(b, &mut a, 60);
        report(&format!("{name}/drag_all_items"), &mut times);
        one(&format!("{name}/drag_all_items_release"), release);
        undo_redo(b, &mut a, &format!("{name}/drag_all_items"));
        a.run(Command::Drop);
    }

    // Save the scene as it is now.
    let out = std::env::temp_dir().join(format!("perf-snapshot-{name}.opscene"));
    let t = Instant::now();
    a.save(out.clone());
    one(&format!("{name}/save"), ms(t));
    let _ = std::fs::remove_file(out);
}

#[test]
#[ignore = "opt-in performance snapshot on generated scenes (OPERANDI_PERF_DIR)"]
fn perf_snapshot() {
    let Some(dir) = std::env::var_os("OPERANDI_PERF_DIR").map(PathBuf::from) else {
        println!("OPERANDI_PERF_DIR not set");
        return;
    };
    let scenes = std::env::var("OPERANDI_PERF_SCENES")
        .unwrap_or_else(|_| "edit1m,objects10k,sub10m,flat10m,sub50m".into());
    let state = super::performance_tests::headless_gpu().expect("graphics device");
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), 2f32));
    let b = Bench {
        ctx,
        device: state.device.clone(),
        size: vec2(1440., 900.),
    };
    println!("PERF adapter {:?}", state.adapter.get_info());
    for name in scenes.split(',') {
        let path = dir.join(format!("{name}.opscene"));
        if !path.exists() {
            println!("PERF {name} missing {}", path.display());
            continue;
        }
        snapshot_scene(&b, &path, name);
    }
}
