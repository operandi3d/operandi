//! Viewport speed measurement: an on-screen frame-time readout (Window >
//! Frame Time) and a scripted orbit benchmark ("operandi.bench.orbit N" on
//! the command line, or OPERANDI_BENCH_ORBIT at launch) that turns the camera
//! a full circle over N frames and reports average and 95th-percentile frame
//! times. Frame intervals include waiting for the display's refresh (vsync),
//! so they can't go below one refresh period; "work" is the time spent
//! building each frame on the CPU.
use super::*;
use std::time::Instant;

/// Rolling timings of the last frames.
#[derive(Default)]
pub(super) struct FrameStats {
    last_start: Option<Instant>,
    current_start: Option<Instant>,
    /// (interval since the previous frame, CPU work in this frame), ms.
    recent: std::collections::VecDeque<(f64, f64)>,
    /// The rebuild totals at the end of the previous frame, and what each
    /// of the last frames rebuilt.
    totals: Option<Rebuilds>,
    rebuilds: std::collections::VecDeque<Rebuilds>,
}

/// What the viewport rebuilt (Window > Frame Time): running totals from
/// the caches, or one frame's share of them. A drag on the fast path shows
/// zeros here after its first frames; a slow path shows which work repeats.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Rebuilds {
    /// Whole-mesh GPU geometry builds.
    pub gpu_meshes: u64,
    /// Cage polygons rewritten in place (a drag's patches).
    pub patched_polygons: u64,
    /// Bytes of mesh geometry sent to the graphics card.
    pub uploaded: u64,
    /// Whole subdivision surface evaluations, and local updates near
    /// moved points.
    pub subdivision_full: u64,
    pub subdivision_local: u64,
    /// Picking surface rebuilds.
    pub picking: u64,
    /// Topology results computed (edge lists, counts, validation).
    pub topology: u64,
}
impl Rebuilds {
    fn since(self, before: Rebuilds) -> Rebuilds {
        Rebuilds {
            gpu_meshes: self.gpu_meshes.saturating_sub(before.gpu_meshes),
            patched_polygons: self
                .patched_polygons
                .saturating_sub(before.patched_polygons),
            uploaded: self.uploaded.saturating_sub(before.uploaded),
            subdivision_full: self
                .subdivision_full
                .saturating_sub(before.subdivision_full),
            subdivision_local: self
                .subdivision_local
                .saturating_sub(before.subdivision_local),
            picking: self.picking.saturating_sub(before.picking),
            topology: self.topology.saturating_sub(before.topology),
        }
    }
    fn plus(self, o: Rebuilds) -> Rebuilds {
        Rebuilds {
            gpu_meshes: self.gpu_meshes + o.gpu_meshes,
            patched_polygons: self.patched_polygons + o.patched_polygons,
            uploaded: self.uploaded + o.uploaded,
            subdivision_full: self.subdivision_full + o.subdivision_full,
            subdivision_local: self.subdivision_local + o.subdivision_local,
            picking: self.picking + o.picking,
            topology: self.topology + o.topology,
        }
    }
    fn any(&self) -> bool {
        *self != Rebuilds::default()
    }
}
impl FrameStats {
    pub fn begin(&mut self) {
        let now = Instant::now();
        self.last_start = self.current_start.replace(now);
    }
    /// Close the frame; returns (interval, work) in ms.
    pub fn end(&mut self) -> Option<(f64, f64)> {
        let start = self.current_start?;
        let interval = start.duration_since(self.last_start?).as_secs_f64() * 1000.;
        let work = start.elapsed().as_secs_f64() * 1000.;
        if self.recent.len() == 60 {
            self.recent.pop_front();
        }
        self.recent.push_back((interval, work));
        Some((interval, work))
    }
    /// Record the caches' running totals at the end of a frame.
    pub fn record_rebuilds(&mut self, totals: Rebuilds) {
        if let Some(before) = self.totals {
            if self.rebuilds.len() == 60 {
                self.rebuilds.pop_front();
            }
            self.rebuilds.push_back(totals.since(before));
        }
        self.totals = Some(totals);
    }
    /// What the last frames rebuilt: the totals over them, how many frames
    /// rebuilt anything, and the busiest frame's patch and upload.
    pub fn rebuilds_label(&self) -> String {
        let sum = self
            .rebuilds
            .iter()
            .fold(Rebuilds::default(), |a, &b| a.plus(b));
        let busy = self.rebuilds.iter().filter(|r| r.any()).count();
        let peak_patch = self
            .rebuilds
            .iter()
            .map(|r| r.patched_polygons)
            .max()
            .unwrap_or(0);
        let peak_upload = self.rebuilds.iter().map(|r| r.uploaded).max().unwrap_or(0);
        let mb = |b: u64| b as f64 / 1_048_576.;
        format!(
            "Rebuilt in the last {} frames ({busy} busy):\n\
             GPU mesh {} · patched {} polygons · uploaded {:.1} MB\n\
             subdivision {} full, {} local · picking {} · topology {}\n\
             busiest frame: patched {peak_patch} · uploaded {:.1} MB",
            self.rebuilds.len(),
            sum.gpu_meshes,
            sum.patched_polygons,
            mb(sum.uploaded),
            sum.subdivision_full,
            sum.subdivision_local,
            sum.picking,
            sum.topology,
            mb(peak_upload),
        )
    }
    /// "16.7 ms · 60 fps · work 4.2 ms" over the last frames.
    pub fn label(&self) -> String {
        if self.recent.is_empty() {
            return "Measuring…".into();
        }
        let n = self.recent.len() as f64;
        let interval = self.recent.iter().map(|r| r.0).sum::<f64>() / n;
        let work = self.recent.iter().map(|r| r.1).sum::<f64>() / n;
        format!(
            "{interval:.1} ms · {:.0} fps · work {work:.1} ms",
            1000. / interval.max(1e-3)
        )
    }
}

/// A running orbit benchmark.
pub(super) struct OrbitBench {
    frames: usize,
    done: usize,
    start_yaw: f64,
    samples: Vec<(f64, f64)>,
    /// Where to write the JSON report, and whether to quit afterwards
    /// (launch mode).
    output: Option<PathBuf>,
    quit: bool,
}

fn stats(values: &mut [f64]) -> serde_json::Value {
    values.sort_by(f64::total_cmp);
    let n = values.len().max(1);
    let avg = values.iter().sum::<f64>() / n as f64;
    let p95 = values
        .get(((n as f64 * 0.95).ceil() as usize).saturating_sub(1))
        .copied()
        .unwrap_or(0.);
    serde_json::json!({ "avg_ms": avg, "p95_ms": p95, "max_ms": values.last().copied().unwrap_or(0.) })
}

impl Operandi {
    /// Start orbiting the camera a full turn over `frames` frames.
    pub(super) fn start_orbit_bench(&mut self, frames: usize, output: Option<PathBuf>, quit: bool) {
        let frames = frames.clamp(10, 100_000);
        self.orbit_bench = Some(OrbitBench {
            frames,
            done: 0,
            start_yaw: self.camera.yaw,
            samples: Vec::with_capacity(frames),
            output,
            quit,
        });
        self.status = format!("Orbit benchmark: {frames} frames…");
    }

    /// Called at the end of every frame.
    /// The caches' running rebuild totals (Window > Frame Time).
    pub(super) fn rebuild_totals(&self) -> Rebuilds {
        Rebuilds {
            gpu_meshes: self.resident_cache.builds(),
            patched_polygons: self.resident_cache.patched_polygons(),
            uploaded: self.viewport_gpu.as_ref().map_or(0, |g| g.uploaded_bytes()),
            subdivision_full: self.subdivision_cache.full_builds(),
            subdivision_local: self.subdivision_cache.updates(),
            picking: self.surface_cache.builds(),
            topology: operandi_core::topology_cache::computed(),
        }
    }

    pub(super) fn end_frame(&mut self, ctx: &Context) {
        let timing = self.frame_stats.end();
        let totals = self.rebuild_totals();
        self.frame_stats.record_rebuilds(totals);
        if self.show_frame_time {
            ctx.request_repaint();
        }
        if self.drag_bench.is_some() {
            self.step_drag_bench(ctx, timing);
            return;
        }
        let Some(bench) = &mut self.orbit_bench else {
            return;
        };
        // The first frame frames the scene (the viewport has its size now)
        // and only starts the clock.
        if bench.done == 0 {
            bench.start_yaw = self.camera.yaw;
        }
        if bench.done > 0
            && let Some(t) = timing
        {
            bench.samples.push(t);
        }
        if bench.done < bench.frames {
            if bench.done == 0 && bench.quit {
                self.frame(false);
            }
            let Some(bench) = &mut self.orbit_bench else {
                return;
            };
            bench.done += 1;
            self.camera.yaw =
                bench.start_yaw + std::f64::consts::TAU * bench.done as f64 / bench.frames as f64;
            ctx.request_repaint();
            return;
        }
        let bench = self.orbit_bench.take().expect("checked above");
        let mut intervals: Vec<f64> = bench.samples.iter().map(|s| s.0).collect();
        let mut work: Vec<f64> = bench.samples.iter().map(|s| s.1).collect();
        let polygons: usize = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| self.editor.scene.shows(i))
            .map(|i| i.mesh.faces.len())
            .sum();
        let report = serde_json::json!({
            "frames": bench.samples.len(),
            "polygons": polygons,
            "subdivided_items": self.editor.scene.items.iter()
                .filter(|i| self.editor.scene.shows(i) && operandi_core::subdivision::has_smooth(&i.mesh)).count(),
            "viewport": [self.viewport_rect.width(), self.viewport_rect.height()],
            "interval": stats(&mut intervals),
            "work": stats(&mut work),
            "note": "interval includes waiting for display refresh (vsync); work is CPU time per frame",
        });
        self.status = format!(
            "Orbit {} frames: {:.1} ms avg ({:.1} p95) per frame, work {:.1} ms avg",
            bench.samples.len(),
            report["interval"]["avg_ms"].as_f64().unwrap_or(0.),
            report["interval"]["p95_ms"].as_f64().unwrap_or(0.),
            report["work"]["avg_ms"].as_f64().unwrap_or(0.),
        );
        let path = bench.output.unwrap_or_else(|| {
            self.environment
                .paths()
                .runtime_viewport()
                .with_file_name("orbit-benchmark.json")
        });
        if let Err(e) = std::fs::write(
            &path,
            serde_json::to_vec_pretty(&report).unwrap_or_default(),
        ) {
            self.status = format!("{} · report not written: {e}", self.status);
        }
        if bench.quit {
            self.allow_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// OPERANDI_BENCH_ORBIT="scene path|frames|report path" at launch: open
    /// the scene (any format Open reads), frame it, orbit, write the report
    /// and quit.
    pub(super) fn launch_orbit_bench(&mut self) {
        let Ok(spec) = std::env::var("OPERANDI_BENCH_ORBIT") else {
            return;
        };
        let mut parts = spec.split('|');
        let scene = parts.next().map(PathBuf::from);
        let frames = parts
            .next()
            .and_then(|f| f.trim().parse().ok())
            .unwrap_or(360);
        let output = parts.next().map(PathBuf::from);
        if let Some(scene) = scene.filter(|p| !p.as_os_str().is_empty()) {
            self.open(scene);
        }
        self.start_orbit_bench(frames, output, true);
    }
}

/// A running item drag benchmark: the largest mesh dragged in Item mode
/// with Move by scripted pointer events (press on the Move handle, or off
/// the handles with `anywhere`), through the app's real input path. The
/// drag is undone at the end, so the scene is left as it was.
pub(super) struct DragBench {
    frames: usize,
    done: usize,
    anywhere: bool,
    start: Pos2,
    /// Frames on which a pointer drag was under way.
    dragging: usize,
    samples: Vec<(f64, f64)>,
    output: Option<PathBuf>,
    quit: bool,
    /// Pointer events for the next frame (raw_input_hook takes them).
    pub(super) events: Vec<Event>,
}

impl Operandi {
    pub(super) fn start_drag_bench(
        &mut self,
        frames: usize,
        anywhere: bool,
        output: Option<PathBuf>,
        quit: bool,
    ) {
        let frames = frames.clamp(10, 100_000);
        self.drag_bench = Some(DragBench {
            frames,
            done: 0,
            anywhere,
            start: Pos2::ZERO,
            dragging: 0,
            samples: Vec::with_capacity(frames),
            output,
            quit,
            events: Vec::new(),
        });
        self.status = format!("Drag benchmark: {frames} frames… (keep the mouse still)");
    }

    /// OPERANDI_BENCH_DRAG="scene path|frames|report path|anywhere" at
    /// launch: open the scene, drag its largest mesh, write the report and
    /// quit.
    pub(super) fn launch_drag_bench(&mut self) {
        let Ok(spec) = std::env::var("OPERANDI_BENCH_DRAG") else {
            return;
        };
        let mut parts = spec.split('|');
        let scene = parts.next().map(PathBuf::from);
        let frames = parts
            .next()
            .and_then(|f| f.trim().parse().ok())
            .unwrap_or(240);
        let output = parts.next().filter(|p| !p.is_empty()).map(PathBuf::from);
        let anywhere = parts.next() == Some("anywhere");
        if let Some(scene) = scene.filter(|p| !p.as_os_str().is_empty()) {
            self.open(scene);
        }
        self.start_drag_bench(frames, anywhere, output, true);
    }

    fn step_drag_bench(&mut self, ctx: &Context, timing: Option<(f64, f64)>) {
        ctx.request_repaint();
        let Some(bench) = &mut self.drag_bench else {
            return;
        };
        let frames = bench.frames;
        let done = bench.done;
        bench.done += 1;
        // Wait for an import (launch mode) to finish first.
        if done == 0 && self.import_job.is_some() {
            if let Some(b) = &mut self.drag_bench {
                b.done = 0;
            }
            return;
        }
        // An LXO import's dialogs: accept a partial import and close the
        // result notice, as the user would, so the presses reach the view.
        if done == 0 {
            if let Some(ready) = self.import_ready.take() {
                self.commit_import(ready);
            }
            self.import_message = None;
        }
        let abort = |a: &mut Operandi, why: &str| {
            a.drag_bench = None;
            a.status = format!("Drag benchmark stopped: {why}");
        };
        match done {
            0 => {
                let Some(id) = self
                    .editor
                    .scene
                    .items
                    .iter()
                    .filter(|i| self.editor.scene.shows(i))
                    .max_by_key(|i| i.mesh.faces.len())
                    .map(|i| i.id)
                else {
                    return abort(self, "no mesh");
                };
                if self.drag_bench.as_ref().is_some_and(|b| b.quit) {
                    self.frame(false);
                }
                self.run(Command::SelectItem(Some(id)));
                self.run(Command::Mode(Mode::Item));
                if self.editor.session.live.as_ref().map(|l| l.tool) != Some(Tool::Move) {
                    self.activate(Tool::Move);
                }
                if self.editor.session.live.as_ref().map(|l| l.tool) != Some(Tool::Move) {
                    let why = format!("Move didn't start ({})", self.status);
                    abort(self, &why);
                }
            }
            1 => {
                let r = self.viewport_rect;
                let anywhere = self.drag_bench.as_ref().is_some_and(|b| b.anywhere);
                let start = if anywhere {
                    // Near the middle of the view, clear of every handle.
                    (0..64)
                        .flat_map(|ring| (0..16).map(move |k| (ring, k)))
                        .map(|(ring, k)| {
                            let angle = k as f32 * std::f32::consts::TAU / 16.;
                            r.center() + vec2(angle.cos(), angle.sin()) * (60. + ring as f32 * 6.)
                        })
                        .find(|&p| r.shrink(40.).contains(p) && self.handle_hit(p, r).is_none())
                } else {
                    (0..300)
                        .flat_map(|y| (0..300).map(move |x| (x, y)))
                        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 300., y as f32 * 2. - 300.))
                        .find(|&p| self.handle_hit(p, r).is_some())
                };
                let Some(start) = start else {
                    return abort(self, "no place to press");
                };
                let bench = self.drag_bench.as_mut().expect("checked above");
                bench.start = start;
                bench.events = vec![
                    Event::PointerMoved(start),
                    Event::PointerButton {
                        pos: start,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                ];
            }
            d if d < frames + 2 => {
                let dragging = self.navigation.is_some();
                let bench = self.drag_bench.as_mut().expect("checked above");
                // The first two drag frames start the gesture; time the rest.
                if d > 3
                    && let Some(t) = timing
                {
                    bench.samples.push(t);
                }
                bench.dragging += usize::from(dragging);
                let t = d as f32 * 0.15;
                let p = bench.start + vec2(t.sin() * 60., (t * 0.7).cos() * 30. - 30.);
                bench.events = vec![Event::PointerMoved(p)];
            }
            d if d == frames + 2 => {
                let bench = self.drag_bench.as_mut().expect("checked above");
                let pos = bench.start;
                bench.events = vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    },
                ];
            }
            _ => {
                let bench = self.drag_bench.take().expect("checked above");
                // The release committed one step: undo it.
                if bench.dragging > 0 {
                    self.run(Command::Undo);
                }
                let mut intervals: Vec<f64> = bench.samples.iter().map(|s| s.0).collect();
                let mut work: Vec<f64> = bench.samples.iter().map(|s| s.1).collect();
                let polygons = self.editor.scene.active().map_or(0, |i| i.mesh.faces.len());
                let report = serde_json::json!({
                    "frames": bench.samples.len(),
                    "dragging_frames": bench.dragging,
                    "off_handles": bench.anywhere,
                    "press": [bench.start.x, bench.start.y],
                    "status_before_report": self.status,
                    "polygons": polygons,
                    "viewport": [self.viewport_rect.width(), self.viewport_rect.height()],
                    "interval": stats(&mut intervals),
                    "work": stats(&mut work),
                    "note": "interval includes waiting for display refresh (vsync); work is CPU time per frame",
                });
                let interval = report["interval"]["avg_ms"].as_f64().unwrap_or(0.);
                self.status = format!(
                    "Drag {} frames ({} dragging): {:.1} ms avg ({:.0} fps, {:.1} p95), work {:.1} ms avg",
                    bench.samples.len(),
                    bench.dragging,
                    interval,
                    1000. / interval.max(1e-3),
                    report["interval"]["p95_ms"].as_f64().unwrap_or(0.),
                    report["work"]["avg_ms"].as_f64().unwrap_or(0.),
                );
                let path = bench.output.unwrap_or_else(|| {
                    self.environment
                        .paths()
                        .runtime_viewport()
                        .with_file_name("drag-benchmark.json")
                });
                if let Err(e) = std::fs::write(
                    &path,
                    serde_json::to_vec_pretty(&report).unwrap_or_default(),
                ) {
                    self.status = format!("{} · report not written: {e}", self.status);
                }
                if bench.quit {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At rest the app draws no frames: the window title goes to the OS
    /// only when it changes, since each viewport command asks for another
    /// frame (it was sent every frame, redrawing at 120 fps forever).
    #[test]
    fn window_title_is_sent_only_when_it_changes() {
        use egui::{RawInput, ViewportCommand, ViewportId};
        let mut a = Operandi::empty();
        let ctx = Context::default();
        let frame = |a: &mut Operandi| {
            let out = ctx.run(RawInput::default(), |ctx| a.sync_window_title(ctx));
            let root = &out.viewport_output[&ViewportId::ROOT];
            let titles = root
                .commands
                .iter()
                .filter(|c| matches!(c, ViewportCommand::Title(_)))
                .count();
            (titles, root.repaint_delay)
        };
        assert_eq!(frame(&mut a).0, 1, "the first frame sets the title");
        // Let egui's own first-frame repaints settle.
        frame(&mut a);
        frame(&mut a);
        let (titles, delay) = frame(&mut a);
        assert_eq!(titles, 0, "an unchanged title is not sent again");
        assert_eq!(delay, std::time::Duration::MAX, "nothing asks for a frame");
        a.path = Some("Robot.opscene".into());
        assert_eq!(frame(&mut a).0, 1, "a new file name is sent");
    }

    #[test]
    fn orbit_benchmark_turns_the_camera_and_writes_a_report() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
                .unwrap()
                .into();
        let ctx = Context::default();
        theme::install(&ctx);
        let report =
            std::env::temp_dir().join(format!("operandi-orbit-{}.json", std::process::id()));
        a.start_orbit_bench(10, Some(report.clone()), false);
        let yaw = a.camera.yaw;
        for _ in 0..12 {
            a.frame_stats.begin();
            super::super::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            a.end_frame(&ctx);
        }
        assert!(a.orbit_bench.is_none());
        assert!(a.status.starts_with("Orbit 10 frames"), "{}", a.status);
        // A full turn ends where it started.
        assert!((a.camera.yaw - yaw - std::f64::consts::TAU).abs() < 1e-9);
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(json["frames"], 10);
        assert_eq!(json["polygons"], 6);
        assert!(json["work"]["avg_ms"].as_f64().unwrap() > 0.);
        let _ = std::fs::remove_file(report);
    }

    /// The drag benchmark presses on the Move handle, drags the item with
    /// pointer events, undoes the drag and reports.
    #[test]
    fn drag_benchmark_drags_the_item_and_puts_it_back() {
        for anywhere in [false, true] {
            let mut a = Operandi::empty();
            a.run(Command::NewMesh);
            a.editor.scene.active_mut().unwrap().mesh =
                operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
                    .unwrap()
                    .into();
            a.frame(false);
            let ctx = Context::default();
            theme::install(&ctx);
            let report = std::env::temp_dir().join(format!(
                "operandi-drag-{}-{anywhere}.json",
                std::process::id()
            ));
            let placement = a.editor.scene.active().unwrap().transform.clone();
            a.start_drag_bench(10, anywhere, Some(report.clone()), false);
            let mut moved = false;
            for _ in 0..30 {
                let events = a
                    .drag_bench
                    .as_mut()
                    .map(|b| std::mem::take(&mut b.events))
                    .unwrap_or_default();
                a.frame_stats.begin();
                super::super::tests::draw(&mut a, &ctx, vec2(1440., 900.), events);
                moved |= a.editor.scene.active().unwrap().transform != placement;
                a.end_frame(&ctx);
            }
            assert!(a.drag_bench.is_none(), "{}", a.status);
            assert!(moved, "the item never moved ({})", a.status);
            assert_eq!(a.editor.scene.active().unwrap().transform, placement);
            let json: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
            assert!(json["dragging_frames"].as_u64().unwrap() >= 8, "{json}");
            let _ = std::fs::remove_file(report);
        }
    }

    /// Frame Time counts what each of the last frames rebuilt, from the
    /// caches' running totals.
    #[test]
    fn frame_time_counts_rebuilds_per_frame() {
        let mut stats = FrameStats::default();
        let at = |gpu, patched, picking| Rebuilds {
            gpu_meshes: gpu,
            patched_polygons: patched,
            picking,
            uploaded: patched * 1000,
            ..Default::default()
        };
        stats.record_rebuilds(at(5, 100, 2));
        stats.record_rebuilds(at(6, 100, 2));
        stats.record_rebuilds(at(6, 130, 3));
        stats.record_rebuilds(at(6, 130, 3));
        let label = stats.rebuilds_label();
        assert!(label.contains("last 3 frames (2 busy)"), "{label}");
        assert!(
            label.contains("GPU mesh 1 · patched 30 polygons"),
            "{label}"
        );
        assert!(label.contains("picking 1"), "{label}");
        assert!(label.contains("busiest frame: patched 30"), "{label}");
    }

    /// In the app: a whole-mesh rebuild shows up, and a fast-path item drag
    /// then counts nothing.
    #[test]
    fn frame_time_shows_a_fast_item_drag_as_zero() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1)
                .unwrap()
                .into();
        a.force_resident = true;
        a.frame(false);
        let ctx = Context::default();
        theme::install(&ctx);
        let frame = |a: &mut Operandi| {
            a.frame_stats.begin();
            super::super::tests::draw(a, &ctx, vec2(800., 600.), vec![]);
            a.end_frame(&ctx);
        };
        frame(&mut a);
        // The box's points moved: its 6 polygons are patched, counted.
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1)
                .unwrap()
                .into();
        frame(&mut a);
        let label = a.frame_stats.rebuilds_label();
        assert!(label.contains("(1 busy)"), "{label}");
        assert!(label.contains("patched 6 polygons"), "{label}");
        a.run(Command::Mode(Mode::Item));
        a.activate(Tool::Move);
        let base = a.editor.session.live.as_ref().unwrap().params.clone();
        for k in 1..3 {
            let mut p = base.clone();
            p.delta.x = 0.1 * k as f64;
            a.run(Command::Preview(p));
            frame(&mut a);
        }
        let before = a.rebuild_totals();
        for k in 3..8 {
            let mut p = base.clone();
            p.delta.x = 0.1 * k as f64;
            a.run(Command::Preview(p));
            frame(&mut a);
        }
        assert_eq!(a.rebuild_totals(), before);
        let last: Vec<_> = a.frame_stats.rebuilds.iter().rev().take(5).collect();
        assert!(last.iter().all(|r| !r.any()), "{last:?}");
    }

    #[test]
    fn command_line_starts_it_and_the_menu_toggles_the_readout() {
        let mut a = Operandi::empty();
        a.execute("operandi.bench.orbit 20");
        assert!(a.orbit_bench.is_some());
        a.execute("operandi.bench.orbit lots");
        assert!(a.status.starts_with("Usage"));
        let ctx = Context::default();
        a.orbit_bench = None;
        a.menu_action(&ctx, app_menu::line_action("operandi.window.frameTime"));
        assert!(a.show_frame_time);
    }
}
