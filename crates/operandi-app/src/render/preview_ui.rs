//! The Preview window (F8; RP-*): a floating OS window with a progressive
//! Cycles render of the scene that starts over whenever something it shows
//! changes. It opens paused; Play (or the Status button) starts it.
//!
//! Each start renders a coarse pass first (an eighth of the pixels across,
//! one sample) so something shows at once, then the full-size pass with
//! adaptive sampling up to the sample target and denoising at the end
//! (RP-Q3). Both run on their own thread, so the main window never waits.
//! A final render (F9) pauses the Preview and resumes it afterwards, so two
//! Cycles renders never run flat out together.
use crate::camera::View;
use crate::input::NavigationKind;
use crate::preview::{
    self, Effect, Elapsed, Lut, Options, Request, Saved, Shading, Status, StatusClick, Threads,
};
use crate::unavailable::Gate;
use crate::{Camera, Operandi};
use eframe::egui::{
    self, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Ui, Vec2, ViewportCommand,
    pos2, vec2,
};
use glam::DVec3;
use operandi_cycles::{Image, RenderControl, RenderMesh, Settings};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

/// The renderer the preview calls; tests swap in a stand-in.
pub(crate) type PreviewRenderer =
    fn(&[RenderMesh], &Settings, Option<Vec<f32>>, &RenderControl) -> operandi_core::Result<Image>;

/// Wait this long after the last change before starting over, so a window
/// resize or a drag doesn't start a render every frame.
const SETTLE: Duration = Duration::from_millis(120);
/// How often the scene is compared with the one being rendered.
const SCENE_CHECK: Duration = Duration::from_millis(200);

/// What the render thread sends back.
enum Message {
    /// The coarse pass, to show while the full pass renders.
    Coarse(Image),
    Done(Result<Image, String>),
}

struct Job {
    control: Arc<RenderControl>,
    rx: mpsc::Receiver<Message>,
    /// The full-size pass has begun (the coarse picture arrived).
    full: bool,
    seen: u64,
}

/// The picture shown: linear RGBA, top row first.
struct Picture {
    width: u32,
    height: u32,
    rgba: Vec<f32>,
    /// Coarse pixels are drawn as big squares (nearest filtering).
    coarse: bool,
}

/// Lock: what the picture keeps while locked.
#[derive(Clone, Copy)]
struct Locked {
    view: operandi_cycles::View,
    width: u32,
    height: u32,
    /// The lock turned on Final Render Quality (Ctrl+click), and unlocking
    /// turns it off again.
    final_quality: bool,
}

pub(crate) struct PreviewState {
    pub(crate) open: bool,
    pub(crate) saved: Saved,
    saved_changed: Option<Instant>,
    pub(crate) options: Options,
    pub(crate) effect: Effect,
    pub(crate) lut: Lut,
    pub(crate) shading: Shading,
    /// Camera popup: `None` is "(Render Camera)".
    pub(crate) camera: Option<u64>,
    /// The preview camera after navigating in the picture (RP-Q8); `None`
    /// looks through the chosen camera as it is.
    nav: Option<Camera>,
    locked: Option<Locked>,
    job: Option<Job>,
    picture: Option<Picture>,
    texture: Option<TextureHandle>,
    texture_stale: bool,
    /// The last render ran to its sample target.
    finished: bool,
    /// What the job (or the finished picture) is for.
    request: Option<Request>,
    /// A wanted request differing from `request`, and since when.
    pending: Option<(Request, Instant)>,
    /// The picture is out of date but the preview is paused: start over on
    /// Resume rather than carrying on.
    stale: bool,
    elapsed: Elapsed,
    /// Scene change tracking.
    scene_serial: u64,
    applied_serial: u64,
    last_scene: Option<operandi_core::Scene>,
    last_check: Option<Instant>,
    /// A final render paused the preview; resume it when that ends.
    pub(crate) held_for_final: bool,
    /// Shift+wheel image zoom (picture pixels to screen pixels), `None` =
    /// fit; and the image pan in points.
    image_zoom: Option<f32>,
    image_pan: Vec2,
    settings_open: bool,
    /// Where the picture went last frame, in points (for tests and picks).
    pub(super) picture_rect: Rect,
    /// The picture area's size in device pixels last frame.
    area_pixels: [u32; 2],
    pub(crate) error: Option<String>,
    /// Bring the window to the front on the next frame.
    focus: bool,
    /// Start rendering once the window has drawn (OPERANDI_QA_PREVIEW).
    pub(crate) autostart: bool,
    renderer: PreviewRenderer,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            open: false,
            saved: Saved::default(),
            saved_changed: None,
            options: Options::default(),
            effect: Effect::Shading,
            lut: Lut::Srgb,
            shading: Shading::Full,
            camera: None,
            nav: None,
            locked: None,
            job: None,
            picture: None,
            texture: None,
            texture_stale: false,
            finished: false,
            request: None,
            pending: None,
            stale: false,
            elapsed: Elapsed::default(),
            scene_serial: 0,
            applied_serial: 0,
            last_scene: None,
            last_check: None,
            held_for_final: false,
            image_zoom: None,
            image_pan: Vec2::ZERO,
            settings_open: false,
            picture_rect: Rect::NOTHING,
            area_pixels: [640, 400],
            error: None,
            focus: false,
            autostart: false,
            // One Cycles kept while the Preview is open: a restart only
            // rebuilds what changed (operandi_cycles::Session).
            renderer: operandi_cycles::render_kept,
        }
    }
}

impl PreviewState {
    /// A preview render is running.
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }
    #[cfg(test)]
    pub(crate) fn set_renderer(&mut self, renderer: PreviewRenderer) {
        self.renderer = renderer;
    }

    /// Paused (never started, held, or held for a final render),
    /// Rendering, or Finished.
    pub(crate) fn status(&self) -> Status {
        match &self.job {
            Some(job) if job.control.pause_requested() => Status::Paused,
            Some(_) => Status::Rendering,
            None if self.finished && !self.stale => Status::Finished,
            None => Status::Paused,
        }
    }

    /// Samples done and the target, for the status bar.
    pub(crate) fn samples(&self) -> (u32, u32) {
        let target = self
            .request
            .as_ref()
            .map_or(self.saved.samples, |r| r.samples);
        match &self.job {
            Some(job) if job.full => {
                let p = job.control.progress();
                (
                    p.samples_done,
                    if p.samples_total > 0 {
                        p.samples_total
                    } else {
                        target
                    },
                )
            }
            Some(_) => (0, target),
            None if self.finished => (target, target),
            None => (0, target),
        }
    }

    pub(crate) fn has_picture(&self) -> bool {
        self.picture.is_some()
    }

    pub(crate) fn locked(&self) -> bool {
        self.locked.is_some()
    }

    fn mark_saved(&mut self) {
        self.saved_changed = Some(Instant::now());
    }

    /// Stop the render thread (it finishes on its own) and forget it.
    fn drop_job(&mut self) {
        if let Some(job) = self.job.take() {
            job.control.cancel();
        }
    }
}

/// An orbit camera placed at `view`, aimed `distance` ahead.
fn orbit_from(view: &operandi_cycles::View, distance: f64) -> Camera {
    let toward = -view.forward.normalize_or(-DVec3::Z);
    Camera {
        view: View::Perspective,
        target: view.position - toward * distance,
        distance,
        yaw: Camera::yaw_pitch_of(toward).0,
        pitch: Camera::yaw_pitch_of(toward).1,
        roll: 0.,
        lens: 0.5 / view.half_height_tan.max(1e-6),
        through: false,
        synced: None,
        settling: None,
        through_item: None,
        light: None,
        near_depth: f64::INFINITY,
        near_distance: f64::INFINITY,
    }
}

/// `view` moved to where `camera` looks from; lens, film and clipping stay.
fn view_through(base: operandi_cycles::View, camera: &Camera) -> operandi_cycles::View {
    let (position, right, up, forward) = camera.basis();
    operandi_cycles::View {
        position,
        right,
        up,
        forward,
        ..base
    }
}

/// One navigation step on the preview camera, as a viewport would take it.
fn navigate(camera: &mut Camera, kind: NavigationKind, delta: Vec2, height: f32, zoom: f64) {
    match kind {
        NavigationKind::Pan => {
            let (_, right, up, _) = camera.basis();
            camera.target += (-right * delta.x as f64 + up * delta.y as f64) * camera.distance
                / (height.max(1.0) as f64 * camera.lens);
        }
        NavigationKind::Zoom => camera.distance = (camera.distance * zoom).clamp(0.03, 1e7),
        NavigationKind::Orbit => camera.orbit_by(delta.x as f64, delta.y as f64),
        NavigationKind::Look => camera.look_around(delta.x as f64, delta.y as f64),
        // The Preview window keeps its three gestures.
        NavigationKind::Roll | NavigationKind::PanVertical | NavigationKind::BoxZoom => {}
    }
}

/// Small drawn glyphs for the Preview's own controls (no icon artwork for
/// these yet; RP-Q10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Glyph {
    Pan,
    Orbit,
    Zoom,
    Maximise,
    Cog,
    Triangle,
    Padlock,
}

fn paint_glyph(painter: &egui::Painter, rect: Rect, glyph: Glyph, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.38;
    let stroke = Stroke::new(1.3_f32, color);
    match glyph {
        Glyph::Pan => {
            for dir in [vec2(1., 0.), vec2(-1., 0.), vec2(0., 1.), vec2(0., -1.)] {
                let tip = c + dir * r;
                painter.line_segment([c, tip], stroke);
                let side = vec2(-dir.y, dir.x) * r * 0.3;
                painter.line_segment([tip, tip - dir * r * 0.35 + side], stroke);
                painter.line_segment([tip, tip - dir * r * 0.35 - side], stroke);
            }
        }
        Glyph::Orbit => {
            let points: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = 0.4 + i as f32 / 20. * 5.2;
                    c + vec2(a.cos(), a.sin()) * r
                })
                .collect();
            let tip = *points.last().unwrap_or(&c);
            painter.add(egui::Shape::line(points, stroke));
            painter.line_segment([tip, tip + vec2(r * 0.45, -r * 0.1)], stroke);
            painter.line_segment([tip, tip + vec2(r * 0.05, r * 0.45)], stroke);
        }
        Glyph::Zoom => {
            let lens = c - vec2(r * 0.25, r * 0.25);
            painter.circle_stroke(lens, r * 0.6, stroke);
            painter.line_segment([lens + vec2(r * 0.43, r * 0.43), c + vec2(r, r)], stroke);
        }
        Glyph::Maximise => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(r * 1.7)),
                1.,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Glyph::Cog => {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let d = vec2(a.cos(), a.sin());
                painter.line_segment([c + d * r * 0.6, c + d * r], Stroke::new(2.0_f32, color));
            }
            painter.circle_stroke(c, r * 0.6, stroke);
        }
        Glyph::Triangle => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + vec2(-r * 0.4, -r * 0.6),
                    c + vec2(r * 0.6, 0.),
                    c + vec2(-r * 0.4, r * 0.6),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Glyph::Padlock => {
            let body = Rect::from_center_size(c + vec2(0., r * 0.3), vec2(r * 1.5, r * 1.1));
            painter.rect_filled(body, 1.5, color);
            let points: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI + i as f32 / 12. * std::f32::consts::PI;
                    c + vec2(0., -r * 0.25) + vec2(a.cos(), a.sin()) * r * 0.5
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
        }
    }
}

/// A flat square button showing `glyph`.
fn glyph_button(ui: &mut Ui, glyph: Glyph, selected: bool, tip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.), Sense::click_and_drag());
    let visuals = ui.style().interact_selectable(&response, selected);
    if response.hovered() || selected || response.dragged() {
        ui.painter().rect_filled(rect, 3., visuals.weak_bg_fill);
    }
    paint_glyph(ui.painter(), rect, glyph, visuals.fg_stroke.color);
    response.on_hover_text(tip)
}

impl Operandi {
    /// F8 / Preview button: open the Preview, or bring it to the front.
    /// It never closes it.
    pub(crate) fn open_preview(&mut self) {
        self.preview.focus = self.preview.open;
        // Preferences > Rendering > Start Paused off: a newly opened
        // Preview starts rendering at once.
        if !self.preview.open && !self.render_preferences.preview_start_paused {
            self.preview.autostart = true;
        }
        self.preview.open = true;
    }

    /// F9 / Render button: start a final render, or, while one runs, bring
    /// the Render window to the front.
    pub(crate) fn render_key(&mut self) {
        if self.render.busy() {
            self.render.open = true;
            self.render.focus = true;
        } else {
            self.begin_render(self.render.pixels_per_point, false);
        }
    }

    /// Closing the Preview stops its render and lets go of the picture.
    pub(crate) fn close_preview(&mut self) {
        let p = &mut self.preview;
        p.drop_job();
        // Let go of the kept renderer once the cancelled render is out of
        // it, off the UI thread (that can take a moment).
        std::thread::spawn(operandi_cycles::release_kept);
        p.open = false;
        p.picture = None;
        p.texture = None;
        p.request = None;
        p.pending = None;
        p.finished = false;
        p.stale = false;
        p.nav = None;
        p.locked = None;
        p.held_for_final = false;
        p.image_zoom = None;
        p.image_pan = Vec2::ZERO;
        p.settings_open = false;
        p.last_scene = None;
        p.elapsed.reset();
        self.save_preview_settings();
    }

    pub(crate) fn save_preview_settings(&mut self) {
        self.preview.saved_changed = None;
        if let Err(e) = self
            .preview
            .saved
            .save(&self.environment.paths().preview_window())
        {
            self.status = format!("Preview settings changed; could not save them: {e}");
        }
    }

    /// A final render is starting: hold a running preview until it ends.
    pub(crate) fn hold_preview_for_final(&mut self) {
        let p = &mut self.preview;
        if let Some(job) = &p.job
            && !job.control.pause_requested()
        {
            job.control.pause();
            p.elapsed.stop(Instant::now());
            p.held_for_final = true;
        }
    }

    /// The final render ended: carry on with a preview it held.
    pub(crate) fn release_preview_after_final(&mut self) {
        if !std::mem::take(&mut self.preview.held_for_final) {
            return;
        }
        if self.preview.stale || self.preview.job.is_none() {
            self.restart_preview();
        } else if let Some(job) = &self.preview.job {
            job.control.resume();
            self.preview.elapsed.run(Instant::now());
        }
    }

    /// The view the preview looks through before navigation, and the
    /// aspect of its render frame.
    fn preview_base_view(&self) -> Option<(operandi_cycles::View, f64)> {
        let scene = &self.editor.scene;
        let id = self
            .preview
            .camera
            .filter(|id| {
                scene
                    .items
                    .iter()
                    .any(|i| i.id == *id && i.camera.is_some())
            })
            .or_else(|| scene.render_camera());
        let [w, h] = id.map_or(scene.render.frame, |id| {
            crate::camera_light_ui::camera_render_size(scene, id)
        });
        let aspect = f64::from(w) / f64::from(h.max(1));
        if let Some(view) = id.and_then(|id| operandi_cycles::camera_view(scene, id, aspect)) {
            return Some((view, aspect));
        }
        // No camera: the viewport camera, in the viewport's shape.
        let view = crate::render_ui::render_view(&self.camera).ok()?;
        let r = self.viewport_rect;
        let aspect = if r.height() > 1.0 {
            f64::from(r.width() / r.height())
        } else {
            aspect
        };
        Some((view, aspect.clamp(0.1, 10.0)))
    }

    /// What the preview should render now, from the window, the options
    /// and the scene.
    pub(crate) fn preview_request(&self) -> Option<Request> {
        let p = &self.preview;
        let (base, aspect) = self.preview_base_view()?;
        let (view, width, height) = if let Some(l) = p.locked {
            (l.view, l.width, l.height)
        } else {
            let view = p.nav.as_ref().map_or(base, |nav| view_through(base, nav));
            let (w, h) = if p.options.full_resolution {
                let [fw, fh] = self.editor.scene.render.frame;
                if (f64::from(fw) / f64::from(fh) - aspect).abs() < 1e-6 {
                    (fw, fh)
                } else {
                    (fh.max(fw), (f64::from(fh.max(fw)) / aspect) as u32)
                }
            } else {
                let [aw, ah] = p.area_pixels;
                let fit = preview::letterbox(
                    Rect::from_min_size(Pos2::ZERO, vec2(aw as f32, ah as f32)),
                    aspect as f32,
                );
                (fit.width() as u32, fit.height() as u32)
            };
            (view, w.clamp(16, 8192), h.clamp(16, 8192))
        };
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        Some(Request {
            width,
            height,
            view,
            // RI-P01/P02: the Render item's Convergence Target and Max
            // Render Time.
            samples: self.editor.scene.render.preview_samples(p.saved.samples),
            threshold: p.saved.threshold,
            denoise: p.saved.denoise,
            time_limit: self.editor.scene.render.preview_seconds() as f32,
            light_paths: p.options.light_paths,
            threads: p.options.threads.count(cores),
            transparent: p.effect == Effect::Alpha,
            render_subdivision: p.options.render_subdivision(),
            only_selected: p.options.only_selected,
            only_visible: p.options.only_visible,
            scene_serial: if p.options.automatic_updates {
                p.scene_serial
            } else {
                p.applied_serial
            },
        })
    }

    /// Start over from coarse with the current request, running.
    pub(crate) fn restart_preview(&mut self) {
        self.preview.drop_job();
        self.preview.pending = None;
        self.preview.applied_serial = self.preview.scene_serial;
        let Some(request) = self.preview_request() else {
            self.preview.error = Some("Nothing to look through: no camera and no view".into());
            return;
        };
        self.preview.request = Some(request.clone());
        self.preview.finished = false;
        self.preview.stale = false;
        self.preview.elapsed.reset();
        self.preview.error = None;
        if self.render.busy() {
            // A final render runs: start once it ends.
            self.preview.stale = true;
            self.preview.held_for_final = true;
            return;
        }
        let scene = preview::filtered_scene(
            &self
                .editor
                .evaluated(operandi_core::evaluate::Purpose::Final),
            &self.preview.options,
        );
        let control = Arc::new(RenderControl::default());
        let (tx, rx) = mpsc::channel();
        let renderer = self.preview.renderer;
        let thread_control = Arc::clone(&control);
        let spawned = std::thread::Builder::new()
            .name("cycles-preview".into())
            .spawn(move || {
                let meshes =
                    match operandi_cycles::scene_meshes_at(&scene, request.render_subdivision) {
                        Ok(m) => m,
                        Err(e) => {
                            let _ = tx.send(Message::Done(Err(e)));
                            return;
                        }
                    };
                // Render fix F6: no polygons still shows the environment.
                match renderer(&meshes, &request.coarse(&scene), None, &thread_control) {
                    Ok(image) => {
                        if tx.send(Message::Coarse(image)).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Message::Done(Err(e)));
                        return;
                    }
                }
                let result = renderer(&meshes, &request.settings(&scene), None, &thread_control);
                let _ = tx.send(Message::Done(result));
            });
        match spawned {
            Ok(_) => {
                self.preview.job = Some(Job {
                    control,
                    rx,
                    full: false,
                    seen: 0,
                });
                self.preview.elapsed.run(Instant::now());
            }
            Err(e) => self.preview.error = Some(e.to_string()),
        }
    }

    /// The Status button (and Pause Render's tick): pause, resume or
    /// restart.
    pub(crate) fn preview_status_click(&mut self) {
        match self.preview.status().click() {
            StatusClick::Pause => {
                if let Some(job) = &self.preview.job {
                    job.control.pause();
                }
                self.preview.held_for_final = false;
                self.preview.elapsed.stop(Instant::now());
            }
            StatusClick::Resume => {
                let carry_on = !self.preview.stale && self.preview.job.is_some();
                if carry_on && !self.render.busy() {
                    if let Some(job) = &self.preview.job {
                        job.control.resume();
                    }
                    self.preview.elapsed.run(Instant::now());
                } else {
                    self.restart_preview();
                }
            }
            StatusClick::Restart => self.restart_preview(),
        }
    }

    /// Lock: freeze the camera, view and size. Ctrl+click also turns on
    /// Final Render Quality; unlocking puts it back.
    pub(crate) fn preview_lock(&mut self, with_final_quality: bool) {
        if let Some(l) = self.preview.locked.take() {
            if l.final_quality {
                self.preview.options.set_final_quality(false);
            }
            return;
        }
        let turn_on = with_final_quality && !self.preview.options.final_quality;
        if turn_on {
            self.preview.options.set_final_quality(true);
        }
        if let Some(r) = self.preview_request() {
            self.preview.locked = Some(Locked {
                view: r.view,
                width: r.width,
                height: r.height,
                final_quality: turn_on,
            });
        }
    }

    /// Once a frame while the Preview is open: watch the scene, start over
    /// when what it shows changed, take in new pictures.
    pub(crate) fn poll_preview(&mut self, ctx: &egui::Context) {
        if !self.preview.open {
            return;
        }
        let now = Instant::now();
        if let Some(t) = self.preview.saved_changed
            && now.duration_since(t) > Duration::from_millis(500)
        {
            self.save_preview_settings();
        }
        // Scene changes.
        if self
            .preview
            .last_check
            .is_none_or(|t| now.duration_since(t) >= SCENE_CHECK)
        {
            self.preview.last_check = Some(now);
            let changed = self
                .preview
                .last_scene
                .as_ref()
                .is_none_or(|s| preview::renders_differently(s, &self.editor.scene));
            if changed {
                if self.preview.last_scene.is_some() {
                    self.preview.scene_serial += 1;
                }
                self.preview.last_scene = Some(self.editor.scene.clone());
            }
        }
        // Start over when the wanted render differs, once it has settled.
        if self.preview.request.is_some()
            && let Some(want) = self.preview_request()
            && Some(&want) != self.preview.request.as_ref()
        {
            let since = match &self.preview.pending {
                Some((p, since)) if *p == want => *since,
                _ => {
                    self.preview.pending = Some((want, now));
                    now
                }
            };
            if self.preview.status() == Status::Paused {
                self.preview.stale = true;
            } else if now.duration_since(since) >= SETTLE {
                self.restart_preview();
            }
            ctx.request_repaint_after(SETTLE);
        }
        self.take_preview_pictures(ctx, now);
    }

    fn take_preview_pictures(&mut self, ctx: &egui::Context, now: Instant) {
        let p = &mut self.preview;
        let Some(job) = &mut p.job else {
            return;
        };
        // In-progress pictures of the full pass.
        if job.full
            && let Some(request) = &p.request
            && let Some((generation, area)) = job.control.preview_since(job.seen)
        {
            job.seen = generation;
            if area.len() == request.width as usize * request.height as usize * 4 {
                p.picture = Some(Picture {
                    width: request.width,
                    height: request.height,
                    rgba: area,
                    coarse: false,
                });
                p.texture_stale = true;
            }
        }
        let progress = job.control.progress();
        if job.control.pause_requested() || progress.paused {
            p.elapsed.stop(now);
        }
        loop {
            let Some(job) = &mut p.job else {
                return;
            };
            match job.rx.try_recv() {
                Ok(Message::Coarse(image)) => {
                    job.full = true;
                    job.seen = job.control.progress().preview_generation;
                    p.picture = Some(Picture {
                        width: image.width,
                        height: image.height,
                        rgba: image.rgba,
                        coarse: true,
                    });
                    p.texture_stale = true;
                }
                Ok(Message::Done(Ok(image))) => {
                    p.picture = Some(Picture {
                        width: image.width,
                        height: image.height,
                        rgba: image.rgba,
                        coarse: false,
                    });
                    p.texture_stale = true;
                    p.finished = true;
                    p.job = None;
                    p.elapsed.stop(now);
                    return;
                }
                Ok(Message::Done(Err(e))) => {
                    if !job.control.cancelled() {
                        p.error = Some(e);
                    }
                    p.job = None;
                    p.elapsed.stop(now);
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                    return;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    p.error = Some("The preview render stopped".into());
                    p.job = None;
                    p.elapsed.stop(now);
                    return;
                }
            }
        }
    }

    /// The Preview window: its own OS window (see native::install).
    pub(crate) fn preview_window(&mut self, ctx: &egui::Context) {
        if !self.preview.open {
            return;
        }
        let saved = self.preview.saved;
        // Opening size and place only, so a resize isn't fought (floating.rs).
        let (size, position) =
            crate::floating::opening_place(ctx, preview_id(), saved.size, saved.position);
        let mut builder = egui::ViewportBuilder::default()
            .with_title("Preview")
            .with_inner_size(size)
            .with_min_inner_size([320., 240.])
            // UIC-189: above the main window, not above other apps.
            .with_window_level(self.floating_window_level());
        if let Some(p) = position {
            builder = builder.with_position(p);
        }
        ctx.show_viewport_immediate(preview_id(), builder, |ctx, _class| {
            // A field's Tab moves to the next field (UIC-51), never a shortcut.
            crate::numeric_tab::take_tab_in_window(ctx);
            if std::mem::take(&mut self.preview.focus) {
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(Color32::BLACK))
                .show(ctx, |ui| self.preview_ui(ui));
            if self.preview.settings_open {
                self.preview_settings_window(ctx);
            }
            let (inner, outer) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect));
            if let Some(r) = inner {
                let size = [r.width(), r.height()];
                if size != self.preview.saved.size && size.iter().all(|v| *v >= 100.) {
                    self.preview.saved.size = size;
                    self.preview.mark_saved();
                }
            }
            if let Some(r) = outer {
                let at = Some([r.left(), r.top()]);
                if at != self.preview.saved.position {
                    self.preview.saved.position = at;
                    self.preview.mark_saved();
                }
            }
            // App shortcuts work while the Preview has the focus.
            if ctx.input(|i| i.viewport().focused == Some(true)) {
                self.keyboard(ctx);
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.close_preview();
            } else if std::mem::take(&mut self.preview.autostart) {
                self.preview_status_click();
            }
        });
    }

    fn preview_ui(&mut self, ui: &mut Ui) {
        ui.visuals_mut().override_text_color = Some(Color32::from_gray(210));
        let full = ui.max_rect();
        let mut area = full;
        if self.preview.saved.show_header {
            let header = egui::Frame::new()
                .fill(Color32::from_gray(28))
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.preview_header(ui);
                });
            area.min.y = header.response.rect.bottom();
        }
        if self.preview.saved.show_status_bar {
            area.max.y -= 22.;
            self.preview_status_bar(
                ui,
                Rect::from_min_max(pos2(full.left(), area.bottom()), full.max),
            );
        }
        self.preview_picture(ui, area);
        self.preview_corner(ui, area);
    }

    fn preview_header(&mut self, ui: &mut Ui) {
        let status = self.preview.status();
        ui.horizontal(|ui| {
            // Activity dot.
            let (dot, _) = ui.allocate_exact_size(vec2(10., 18.), Sense::hover());
            let lit = status == Status::Rendering;
            ui.painter().circle_filled(
                dot.center(),
                4.,
                if lit {
                    crate::theme::orange()
                } else {
                    Color32::from_gray(70)
                },
            );
            ui.menu_button("Options", |ui| self.preview_options_menu(ui));
            let effect = self.preview.effect;
            egui::ComboBox::from_id_salt("preview-effect")
                .selected_text(format!("Effect: {}", effect.label()))
                .truncate()
                .width(130.)
                .show_ui(ui, |ui| {
                    for e in Effect::ALL {
                        if ui.selectable_label(effect == e, e.label()).clicked() {
                            self.preview.effect = e;
                            self.preview.texture_stale = true;
                        }
                    }
                });
            let label = status.label();
            if ui
                .add(egui::Button::new(label).min_size(vec2(110., 0.)))
                .on_hover_text(match status {
                    Status::Rendering => "Pause the preview",
                    Status::Paused => "Carry on rendering",
                    Status::Finished => "Render again from the start",
                })
                .clicked()
            {
                self.preview_status_click();
            }
            let locked = self.preview.locked();
            let lock = glyph_button(
                ui,
                Glyph::Padlock,
                locked,
                "Lock the camera, view and size · Ctrl+click: also Final Render Quality",
            );
            if lock.clicked() {
                let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                self.preview_lock(ctrl);
            }
            let lut = self.preview.lut;
            egui::ComboBox::from_id_salt("preview-lut")
                .selected_text(format!("LUT: {}", lut.label()))
                .width(90.)
                .show_ui(ui, |ui| {
                    for l in [Lut::Srgb, Lut::Linear] {
                        if ui.selectable_label(lut == l, l.label()).clicked() {
                            self.preview.lut = l;
                            self.preview.texture_stale = true;
                        }
                    }
                });
        });
        ui.horizontal(|ui| {
            let cameras: Vec<(u64, String)> = self
                .editor
                .scene
                .items
                .iter()
                .filter(|i| i.camera.is_some())
                .map(|i| (i.id, i.name.clone()))
                .collect();
            let chosen = self.preview.camera;
            let name = chosen
                .and_then(|id| cameras.iter().find(|(c, _)| *c == id))
                .map_or("(Render Camera)".to_string(), |(_, n)| n.clone());
            egui::ComboBox::from_id_salt("preview-camera")
                .selected_text(name)
                .truncate()
                .width(150.)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(chosen.is_none(), "(Render Camera)")
                        .clicked()
                    {
                        self.preview.camera = None;
                        self.preview.nav = None;
                    }
                    for (id, name) in &cameras {
                        if ui.selectable_label(chosen == Some(*id), name).clicked() {
                            self.preview.camera = Some(*id);
                            self.preview.nav = None;
                        }
                    }
                });
            let shading = self.preview.shading;
            egui::ComboBox::from_id_salt("preview-shading")
                .selected_text(match shading {
                    Shading::Full => "Shading: Full",
                    Shading::Fast => "Shading: Fast",
                })
                .width(110.)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.preview.shading, Shading::Full, "Full");
                    ui.selectable_value(&mut self.preview.shading, Shading::Fast, "Fast")
                        .on_hover_text("Renders like Full for now");
                });
        });
    }

    fn preview_options_menu(&mut self, ui: &mut Ui) {
        self.menu_owned_input = true;
        let later = |ui: &mut Ui, label: &str, ticked: bool| {
            let mut t = ticked;
            ui.gated(
                Some("Not available yet"),
                egui::Checkbox::new(&mut t, label),
            );
        };
        let later_item = |ui: &mut Ui, label: &str| {
            ui.gated(Some("Not available yet"), egui::Button::new(label));
        };
        ui.set_min_width(220.);
        // Geometry.
        let _ = ui.radio(true, "Scene Preview");
        for label in ["Sphere", "Plane", "Preset Sphere", "Environment"] {
            ui.gated(
                Some("Not available yet"),
                egui::RadioButton::new(false, label),
            );
        }
        ui.separator();
        // Quality.
        let mut final_quality = self.preview.options.final_quality;
        if ui
            .checkbox(&mut final_quality, "Final Render Quality")
            .on_hover_text("Full resolution, every light path and render-level subdivision")
            .changed()
        {
            self.preview.options.set_final_quality(final_quality);
        }
        let o = &mut self.preview.options;
        if ui
            .checkbox(&mut o.full_resolution, "Full Resolution")
            .on_hover_text("Render at the scene's render frame size, scaled to fit")
            .changed()
        {
            self.preview.image_zoom = None;
            self.preview.image_pan = Vec2::ZERO;
        }
        later(ui, "Render All Outputs", false);
        ui.separator();
        // Light paths.
        let o = &mut self.preview.options;
        ui.checkbox(&mut o.light_paths.shadows, "Shadows");
        ui.checkbox(&mut o.light_paths.diffuse, "Global Illumination");
        ui.checkbox(&mut o.light_paths.glossy, "Reflections");
        ui.checkbox(&mut o.light_paths.transmission, "Refractions");
        for (label, ticked) in [
            ("Displacements", true),
            ("Draft Displacements", true),
            ("Fur", true),
            ("Draft Fur", false),
            ("Volumetrics", true),
            ("Motion Blur", false),
            ("Render Proxy", true),
        ] {
            later(ui, label, ticked);
        }
        ui.separator();
        ui.checkbox(
            &mut self.preview.options.automatic_updates,
            "Automatic Geometry Updates",
        )
        .on_hover_text("Off: edits show after Reset Render, or pausing and resuming");
        ui.separator();
        ui.radio_value(
            &mut self.preview.options.threads,
            Threads::Multiple,
            "Use Multiple Threads",
        )
        .on_hover_text("Every core but one");
        ui.radio_value(
            &mut self.preview.options.threads,
            Threads::All,
            "Use All Threads",
        );
        ui.separator();
        ui.checkbox(
            &mut self.preview.options.only_selected,
            "Show Only Selected Items",
        );
        ui.checkbox(
            &mut self.preview.options.only_visible,
            "Show Only Visible Items",
        );
        ui.separator();
        let mut paused = self.preview.status() == Status::Paused;
        if ui.checkbox(&mut paused, "Pause Render").changed() {
            if paused {
                self.preview_status_click();
            } else {
                // Unticking starts again from scratch.
                self.restart_preview();
            }
        }
        if ui.button("Reset Render").clicked() {
            self.restart_preview();
            ui.close();
        }
        ui.separator();
        later_item(ui, "Load Progressive Render…");
        later_item(ui, "Save Progressive Render…");
        if ui
            .gated(
                crate::unavailable::unless(self.preview.has_picture(), "Render a picture first"),
                egui::Button::new("Save Image…"),
            )
            .on_hover_text("PNG or JPEG, through the LUT")
            .clicked()
        {
            self.preview_save_image();
            ui.close();
        }
        later_item(ui, "Save Layered Image…");
        ui.separator();
        later(ui, "Update Under Mouse", true);
        ui.menu_button("Trackball Rotation", |ui| {
            let _ = ui.radio(true, "Off");
            ui.gated(
                Some("Not available yet"),
                egui::RadioButton::new(false, "On"),
            );
        });
        later(ui, "Tool Activation", true);
        ui.separator();
        let render = format!("Render{}", self.shortcut("Render"));
        if ui
            .gated(
                crate::unavailable::unless(
                    operandi_cycles::available() && !self.render.busy(),
                    if !operandi_cycles::available() {
                        "This build has no Cycles renderer"
                    } else {
                        "A render is already running"
                    },
                ),
                egui::Button::new(render),
            )
            .clicked()
        {
            let ctx = ui.ctx().clone();
            self.start_render(&ctx);
            ui.close();
        }
        later_item(ui, "Render Animation");
    }

    fn preview_settings_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let before = self.preview.saved;
        let (still_open, _) = self.palette_window(
            ctx,
            "preview-settings",
            "Preview Settings",
            [330., 380.],
            None,
            |app, ui| {
                let s = &mut app.preview.saved;
                let d = crate::preview::Saved::default();
                use crate::form::{Unit, rows};
                ui.set_width(300.);
                rows::scope(ui, "preview-settings", |ui| {
                    rows::number_d(
                        ui,
                        "Samples",
                        &mut s.samples,
                        d.samples,
                        Unit::Count,
                        1..=65_536,
                        "Samples per pixel the Preview refines to",
                    );
                    rows::number_d(
                        ui,
                        "Noise Threshold",
                        &mut s.threshold,
                        d.threshold,
                        Unit::Scalar,
                        0.0..=1.0,
                        "Adaptive sampling stops where noise is below this; 0 = off",
                    );
                    rows::check_d(
                        ui,
                        "Denoise",
                        &mut s.denoise,
                        d.denoise,
                        "Clean up the picture's noise once it settles",
                    );
                    rows::check_d(
                        ui,
                        "Show Viewport Controls",
                        &mut s.show_controls,
                        d.show_controls,
                        "Show the view's header buttons over the Preview",
                    );
                    rows::check_d(
                        ui,
                        "Show Status Bar",
                        &mut s.show_status_bar,
                        d.show_status_bar,
                        "Show samples and time under the picture",
                    );
                    // Heat Map, Bake Mode and Layer Overlay are left out until
                    // built (UIC-117).
                });
            },
        );
        open = open && still_open;
        if self.preview.saved != before {
            self.preview.mark_saved();
        }
        if !open {
            self.preview.settings_open = false;
        }
    }

    fn preview_status_bar(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., Color32::from_gray(24));
        let status = self.preview.status();
        let (done, total) = self.preview.samples();
        let left = match &self.preview.error {
            Some(e) => e.clone(),
            None => preview::status_text(status, done, total),
        };
        let color = Color32::from_gray(200);
        let font = egui::FontId::proportional(12.);
        painter.text(
            rect.left_center() + vec2(8., -1.),
            egui::Align2::LEFT_CENTER,
            left,
            font.clone(),
            color,
        );
        painter.text(
            rect.right_center() + vec2(-8., -1.),
            egui::Align2::RIGHT_CENTER,
            preview::hms(self.preview.elapsed.at(Instant::now())),
            font,
            color,
        );
        if total > 0 {
            let fill = (done as f32 / total as f32).clamp(0.0, 1.0);
            let strip = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.), rect.max);
            painter.rect_filled(
                Rect::from_min_size(strip.min, vec2(strip.width() * fill, 2.)),
                0.,
                crate::theme::orange(),
            );
        }
        if self.preview.elapsed.running() {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }
    }

    fn preview_picture(&mut self, ui: &mut Ui, area: Rect) {
        let ppp = ui.ctx().pixels_per_point();
        let area_pixels = [
            (area.width() * ppp).max(16.) as u32,
            (area.height() * ppp).max(16.) as u32,
        ];
        self.preview.area_pixels = area_pixels;
        let response = ui.interact(
            area,
            egui::Id::new("preview-picture"),
            Sense::click_and_drag(),
        );
        let aspect = self
            .preview
            .request
            .as_ref()
            .map(|r| r.width as f32 / r.height.max(1) as f32)
            .or_else(|| self.preview_base_view().map(|(_, a)| a as f32))
            .unwrap_or(1.5);
        // Where the picture goes: fitted, or at the Shift+wheel zoom.
        let rect = match (self.preview.image_zoom, &self.preview.request) {
            (Some(z), Some(r)) => Rect::from_center_size(
                area.center() + self.preview.image_pan,
                vec2(r.width as f32, r.height as f32) * z / ppp,
            ),
            _ => preview::letterbox(area, aspect).translate(self.preview.image_pan),
        };
        self.preview.picture_rect = rect;
        if self.preview.texture_stale
            && let Some(p) = &self.preview.picture
        {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [p.width as usize, p.height as usize],
                &preview::display_pixels(&p.rgba, self.preview.effect, self.preview.lut),
            );
            let options = if p.coarse {
                egui::TextureOptions::NEAREST
            } else {
                egui::TextureOptions::LINEAR
            };
            match &mut self.preview.texture {
                Some(t) => t.set(image, options),
                None => {
                    self.preview.texture =
                        Some(ui.ctx().load_texture("operandi-preview", image, options))
                }
            }
            self.preview.texture_stale = false;
        }
        let painter = ui.painter_at(area);
        if let (Some(t), true) = (&self.preview.texture, self.preview.picture.is_some()) {
            painter.image(
                t.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
                Color32::WHITE,
            );
        } else {
            // Opens paused: a big Play triangle.
            let r = area.width().min(area.height()) * 0.12;
            let c = area.center();
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + vec2(-r * 0.7, -r),
                    c + vec2(r, 0.),
                    c + vec2(-r * 0.7, r),
                ],
                Color32::from_gray(if response.hovered() { 220 } else { 190 }),
                Stroke::NONE,
            ));
            if response.clicked() && self.preview.job.is_none() {
                self.preview_status_click();
            }
        }
        // Navigation and image pan/zoom.
        let (shift, ctrl, delta) = ui.input(|i| {
            (
                i.modifiers.shift,
                i.modifiers.ctrl || i.modifiers.command,
                i.pointer.delta(),
            )
        });
        if response.dragged() && delta != Vec2::ZERO {
            if self.preview.options.full_resolution || self.preview.image_zoom.is_some() {
                self.preview.image_pan += delta;
            } else if !self.preview.locked() {
                let kind = if shift {
                    NavigationKind::Pan
                } else if ctrl {
                    NavigationKind::Zoom
                } else {
                    NavigationKind::Orbit
                };
                self.preview_navigate(kind, delta, rect.height());
            }
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta);
            if scroll != Vec2::ZERO {
                if shift {
                    let current = self.preview.image_zoom.unwrap_or_else(|| {
                        self.preview
                            .request
                            .as_ref()
                            .map_or(1.0, |r| rect.width() * ppp / r.width.max(1) as f32)
                    });
                    let zoom_in = scroll.x + scroll.y > 0.0;
                    self.preview.image_zoom = Some(preview::zoom_step(current, zoom_in));
                } else if !self.preview.locked() && !self.preview.options.full_resolution {
                    let factor = self.input_preferences.options.wheel_zoom_factor(scroll.y);
                    self.preview_zoom(factor);
                }
            }
        }
        if response.double_clicked() {
            self.preview.image_zoom = None;
            self.preview.image_pan = Vec2::ZERO;
        }
    }

    fn preview_nav_camera(&mut self) -> Option<&mut Camera> {
        if self.preview.nav.is_none() {
            let (base, _) = self.preview_base_view()?;
            // Aim at the origin's depth, or 5 m ahead when it's behind.
            let depth = (-base.position).dot(base.forward);
            self.preview.nav = Some(orbit_from(&base, if depth > 0.1 { depth } else { 5.0 }));
        }
        self.preview.nav.as_mut()
    }

    fn preview_navigate(&mut self, kind: NavigationKind, delta: Vec2, height: f32) {
        let zoom = self.input_preferences.options.drag_zoom_factor(delta);
        if let Some(camera) = self.preview_nav_camera() {
            navigate(camera, kind, delta, height, zoom);
        }
    }

    fn preview_zoom(&mut self, factor: f64) {
        if let Some(camera) = self.preview_nav_camera() {
            camera.distance = (camera.distance * factor).clamp(0.03, 1e7);
        }
    }

    /// Top-right corner controls, like a 3D viewport's.
    fn preview_corner(&mut self, ui: &mut Ui, area: Rect) {
        let show = self.preview.saved.show_controls;
        let count = if show { 6. } else { 2. };
        let rect = Rect::from_min_size(
            pos2(area.right() - count * 22. - 6., area.top() + 4.),
            vec2(count * 22., 22.),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 2.;
        let height = self.preview.picture_rect.height();
        if show {
            let free = !self.preview.locked() && !self.preview.options.full_resolution;
            for (glyph, kind, tip) in [
                (
                    Glyph::Pan,
                    NavigationKind::Pan,
                    "Drag to pan the preview camera",
                ),
                (
                    Glyph::Orbit,
                    NavigationKind::Orbit,
                    "Drag to orbit the preview camera",
                ),
                (
                    Glyph::Zoom,
                    NavigationKind::Zoom,
                    "Drag to zoom the preview camera",
                ),
            ] {
                let r = glyph_button(&mut child, glyph, false, tip);
                let delta = child.input(|i| i.pointer.delta());
                if r.dragged() && free && delta != Vec2::ZERO {
                    self.preview_navigate(kind, delta, height);
                }
            }
            let maximised = child.input(|i| i.viewport().maximized.unwrap_or(false));
            if glyph_button(&mut child, Glyph::Maximise, maximised, "Maximise").clicked() {
                child
                    .ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(!maximised));
            }
        }
        if glyph_button(
            &mut child,
            Glyph::Cog,
            self.preview.settings_open,
            "Preview Settings",
        )
        .clicked()
        {
            self.preview.settings_open = !self.preview.settings_open;
        }
        let header = self.preview.saved.show_header;
        if glyph_button(
            &mut child,
            Glyph::Triangle,
            !header,
            if header {
                "Hide the header"
            } else {
                "Show the header"
            },
        )
        .clicked()
        {
            self.preview.saved.show_header = !header;
            self.preview.mark_saved();
        }
    }

    /// Options > Save Image: PNG or JPEG through the effect and LUT.
    fn preview_save_image(&mut self) {
        let folder = self
            .project_folder(crate::project_files::RENDERS)
            .or_else(|| crate::render_ui::render_folder().ok())
            .unwrap_or_default();
        let extension = self.file_io_preferences.image_format.extension();
        let name = format!("Preview.{extension}");
        if !self.choose(
            crate::file_chooser::Choose::RenderImage { folder, name },
            crate::file_chooser::Purpose::PreviewImage,
        ) {
            match crate::render_ui::numbered_path(None, "Preview", extension) {
                Ok(path) => self.preview_save_to(path),
                Err(e) => self.status = e,
            }
        }
    }

    pub(crate) fn preview_save_to(&mut self, path: std::path::PathBuf) {
        let Some(p) = &self.preview.picture else {
            self.status = "The Preview has no picture to save yet".into();
            return;
        };
        let pixels = preview::display_pixels(&p.rgba, self.preview.effect, self.preview.lut);
        let result = crate::file_io_preferences::save_rgba(
            &path,
            &pixels,
            p.width,
            p.height,
            self.file_io_preferences.image(),
        );
        self.status = match result {
            Ok(()) => format!("Saved the Preview picture as {}", path.display()),
            Err(e) => format!("Couldn't save the Preview picture: {e}"),
        };
    }

    /// Top bar, right of Work Plane: Render (F9), then Preview (F8), styled
    /// like the other top-bar buttons.
    pub(crate) fn render_buttons(&mut self, ui: &mut Ui) {
        let key = self
            .bindings
            .get("Render")
            .filter(|b| !b.is_empty())
            .cloned()
            .unwrap_or_default();
        let mut job = egui::text::LayoutJob::default();
        let style = ui.style().clone();
        RichText::new("Render").append_to(
            &mut job,
            &style,
            egui::FontSelection::Default,
            egui::Align::Center,
        );
        if !key.is_empty() {
            RichText::new(format!("  {key}")).weak().append_to(
                &mut job,
                &style,
                egui::FontSelection::Default,
                egui::Align::Center,
            );
        }
        // Both buttons take the width their icon, label and hint need, even
        // when the bar is short of room (Jason's QA).
        let render = ui
            .add(
                crate::icons::button_with(ui, crate::icons::Icon::Render, job.into())
                    .wrap_mode(egui::TextWrapMode::Extend),
            )
            .on_hover_text(format!("Render{}", self.shortcut_hint("Render")));
        if render.clicked() {
            self.render_key();
        }
        let preview = ui
            .add(
                crate::icons::button(ui, crate::icons::Icon::Sphere, "Preview")
                    .wrap_mode(egui::TextWrapMode::Extend),
            )
            .on_hover_text(format!("Preview{}", self.shortcut_hint("Preview")));
        if preview.clicked() {
            self.open_preview();
        }
    }

    /// " (F9)" for a tooltip, or nothing when the action is unbound.
    fn shortcut_hint(&self, action: &str) -> String {
        self.bindings
            .get(action)
            .filter(|b| !b.is_empty())
            .map(|b| format!(" ({b})"))
            .unwrap_or_default()
    }
}

pub(crate) fn preview_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("operandi-preview-window")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw};
    use operandi_cycles::Region;

    /// Stands in for Cycles: a flat picture of the asked size; with more
    /// than one sample it waits to be cancelled (or finished) when
    /// `SLOW` is set, so tests can watch a running preview.
    fn fake(
        _meshes: &[RenderMesh],
        s: &Settings,
        _base: Option<Vec<f32>>,
        control: &RenderControl,
    ) -> operandi_core::Result<Image> {
        if s.samples > 1 && SLOW.load(std::sync::atomic::Ordering::SeqCst) {
            while !control.finish_requested() {
                if control.cancelled() {
                    return Err("Cancelled".into());
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        if control.cancelled() {
            return Err("Cancelled".into());
        }
        let value = if s.samples == 1 { 0.25 } else { 0.5 };
        let mut rgba = vec![value; s.width as usize * s.height as usize * 4];
        operandi_cycles::paste_region(&mut rgba, s.width, Region::whole(s.width, s.height), &[]);
        Ok(Image {
            width: s.width,
            height: s.height,
            rgba,
            device: "Test".into(),
            used_gpu: false,
            denoised: s.denoise,
            samples: s.samples,
            light_groups: Vec::new(),
            passes: Vec::new(),
            values: Vec::new(),
            cryptomatte_manifests: Vec::new(),
        })
    }

    static SLOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    /// Tests that use SLOW run one at a time.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn app() -> (Operandi, egui::Context) {
        let mut a = box_app();
        a.camera.view = View::Perspective;
        a.preview.set_renderer(fake);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
        }
        (a, ctx)
    }

    fn settle(a: &mut Operandi, ctx: &egui::Context, until: impl Fn(&Operandi) -> bool) {
        for _ in 0..500 {
            a.poll_preview(ctx);
            if until(a) {
                return;
            }
            std::thread::sleep(Duration::from_millis(4));
        }
        panic!("preview never got there: {:?}", a.preview.status());
    }

    #[test]
    fn opens_paused_then_renders_coarse_then_finishes() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        SLOW.store(false, std::sync::atomic::Ordering::SeqCst);
        let (mut a, ctx) = app();
        a.open_preview();
        a.poll_preview(&ctx);
        assert!(a.preview.open);
        assert_eq!(a.preview.status(), Status::Paused);
        assert!(!a.preview.has_picture());
        a.preview_status_click();
        assert_eq!(a.preview.status(), Status::Rendering);
        settle(&mut a, &ctx, |a| a.preview.status() == Status::Finished);
        let p = a.preview.picture.as_ref().unwrap();
        assert!(!p.coarse && p.rgba.iter().all(|&v| v == 0.5));
        let r = a.preview.request.clone().unwrap();
        assert_eq!((p.width, p.height), (r.width, r.height));
        assert_eq!(a.preview.samples(), (256, 256));
        // Finished: the Status button starts over.
        a.preview_status_click();
        assert_eq!(a.preview.status(), Status::Rendering);
        settle(&mut a, &ctx, |a| a.preview.status() == Status::Finished);
        // Closing lets go of everything; reopening is paused again.
        a.close_preview();
        assert!(!a.preview.open && !a.preview.has_picture());
        a.open_preview();
        assert_eq!(a.preview.status(), Status::Paused);
    }

    #[test]
    fn pause_keeps_samples_and_edits_restart() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        SLOW.store(true, std::sync::atomic::Ordering::SeqCst);
        let (mut a, ctx) = app();
        a.open_preview();
        a.poll_preview(&ctx);
        a.preview_status_click();
        settle(&mut a, &ctx, |a| {
            a.preview.picture.as_ref().is_some_and(|p| p.coarse)
        });
        // Pause and resume the same job.
        let first = Arc::as_ptr(&a.preview.job.as_ref().unwrap().control);
        a.preview_status_click();
        assert_eq!(a.preview.status(), Status::Paused);
        a.preview_status_click();
        assert_eq!(a.preview.status(), Status::Rendering);
        assert_eq!(Arc::as_ptr(&a.preview.job.as_ref().unwrap().control), first);
        // An edit restarts it (after the settle time).
        a.editor.scene.items[0].transform.translation.x += 1.0;
        a.preview.last_check = None;
        settle(&mut a, &ctx, |a| {
            a.preview
                .job
                .as_ref()
                .is_some_and(|j| Arc::as_ptr(&j.control) != first)
        });
        // A light path change restarts it too.
        let second = Arc::as_ptr(&a.preview.job.as_ref().unwrap().control);
        a.preview.options.light_paths.glossy = false;
        settle(&mut a, &ctx, |a| {
            a.preview
                .job
                .as_ref()
                .is_some_and(|j| Arc::as_ptr(&j.control) != second)
        });
        assert!(!a.preview.request.as_ref().unwrap().light_paths.glossy);
        a.close_preview();
        SLOW.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    #[test]
    fn automatic_updates_off_waits_for_reset() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        SLOW.store(false, std::sync::atomic::Ordering::SeqCst);
        let (mut a, ctx) = app();
        a.open_preview();
        a.preview.options.automatic_updates = false;
        a.poll_preview(&ctx);
        a.preview_status_click();
        settle(&mut a, &ctx, |a| a.preview.status() == Status::Finished);
        a.editor.scene.items[0].transform.translation.x += 1.0;
        a.preview.last_check = None;
        for _ in 0..50 {
            a.poll_preview(&ctx);
            std::thread::sleep(Duration::from_millis(4));
        }
        assert_eq!(a.preview.status(), Status::Finished, "edit ignored");
        a.restart_preview();
        assert_eq!(a.preview.status(), Status::Rendering);
        assert_eq!(
            a.preview.request.as_ref().unwrap().scene_serial,
            a.preview.scene_serial
        );
        a.close_preview();
    }

    #[test]
    fn lock_freezes_the_size_and_ctrl_lock_restores_quality() {
        let (mut a, _ctx) = app();
        a.open_preview();
        a.preview.area_pixels = [600, 400];
        let before = a.preview_request().unwrap();
        // Plain lock: resizing and navigating change nothing.
        a.preview_lock(false);
        a.preview.area_pixels = [300, 200];
        a.preview_navigate(NavigationKind::Orbit, vec2(40., 0.), 400.);
        assert_eq!(a.preview_request().unwrap(), before);
        a.preview_lock(false);
        assert!(!a.preview.locked());
        a.preview.nav = None;
        a.preview.area_pixels = [600, 400];
        // Ctrl+click: lock at Final Render Quality (full resolution).
        a.preview_lock(true);
        assert!(a.preview.locked() && a.preview.options.final_quality);
        let locked = a.preview_request().unwrap();
        a.preview.area_pixels = [300, 200];
        assert_eq!(a.preview_request().unwrap(), locked);
        a.preview_lock(false);
        assert!(!a.preview.locked() && !a.preview.options.final_quality);
        let after = a.preview_request().unwrap();
        assert!(after.width < before.width);
    }

    #[test]
    fn f9_holds_the_preview_and_resumes_it() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        SLOW.store(true, std::sync::atomic::Ordering::SeqCst);
        let (mut a, ctx) = app();
        a.open_preview();
        a.poll_preview(&ctx);
        a.preview_status_click();
        a.hold_preview_for_final();
        assert!(a.preview.held_for_final);
        assert_eq!(a.preview.status(), Status::Paused);
        a.release_preview_after_final();
        assert_eq!(a.preview.status(), Status::Rendering);
        a.close_preview();
        SLOW.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    #[test]
    fn navigation_moves_the_preview_camera_not_the_scene() {
        let (mut a, _ctx) = app();
        a.open_preview();
        a.preview.area_pixels = [600, 400];
        let scene = a.editor.scene.clone();
        let before = a.preview_request().unwrap().view;
        a.preview_navigate(NavigationKind::Orbit, vec2(40., 0.), 400.);
        let after = a.preview_request().unwrap().view;
        assert!((after.position - before.position).length() > 1e-3);
        assert!(!preview::renders_differently(&scene, &a.editor.scene));
        // An orbit camera made from a view looks the same way.
        let cam = orbit_from(&before, 5.0);
        let (pos, _, _, fwd) = cam.basis();
        assert!((pos - before.position).length() < 1e-6);
        assert!((fwd - before.forward).length() < 1e-6);
    }

    /// The real thing: Cycles renders the coarse pass, then the full pass
    /// to its (small) sample target. `--features cycles` only.
    #[cfg(feature = "cycles")]
    #[test]
    fn cycles_renders_the_preview() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let (mut a, ctx) = app();
        a.preview.set_renderer(operandi_cycles::render_live);
        a.preview.saved.samples = 8;
        a.preview.area_pixels = [96, 64];
        a.open_preview();
        a.preview_status_click();
        for _ in 0..3000 {
            a.poll_preview(&ctx);
            if a.preview.status() == Status::Finished || a.preview.error.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(a.preview.error, None);
        assert_eq!(a.preview.status(), Status::Finished);
        let p = a.preview.picture.as_ref().unwrap();
        assert!(!p.coarse && p.rgba.iter().all(|v| v.is_finite()));
        assert!(p.rgba.iter().any(|&v| v > 0.0));
        a.close_preview();
    }

    #[test]
    fn the_window_and_buttons_draw() {
        let (mut a, ctx) = app();
        a.open_preview();
        for _ in 0..3 {
            draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
        }
        assert!(a.preview.open);
    }
}
