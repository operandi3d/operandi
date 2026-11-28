//! Render > Render Viewport (Cycles): a final-quality still of the current
//! scene from the scene's render camera (CL-1; the viewport camera when the
//! scene has no camera), rendered off the UI thread. Only in builds made
//! with `--features cycles`. Uses the GPU (Metal) and denoising when
//! available; the first GPU render compiles Cycles' kernels, which takes a
//! while once.
//!
//! The picture appears in its own Render window, the Image Viewer
//! (image_viewer_ui.rs), and fills in as Cycles works. While it renders:
//! Pause/Resume (P), Finish (stop adding samples, denoise and keep what's
//! there) and Cancel. Drag a rectangle on the picture, then Render Region to
//! re-render only that area into the picture (a click clears the
//! rectangle). Every finished render joins the window's history strip as
//! float OpenEXR files in the Recent Frames cache (IV-12); it is saved to
//! ~/Pictures/Operandi Renders only when Preferences > Rendering > Also Save
//! Every Render to Pictures is on (Jason, 2026-11-07), or for an MCP call.
use crate::camera::View;
use crate::image_viewer::{self, History, Slot, SlotKind, Stats, ViewerPreferences};
use crate::{Camera, Operandi};
use eframe::egui::{self, Pos2, Rect, TextureHandle, Vec2};
use operandi_cycles::{Image, Region, RenderControl, RenderMesh, Settings};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

/// What the render thread sends back once a render is done.
pub(crate) struct Done {
    pub image: Image,
    /// Where it was saved outside the cache (Pictures, or an MCP path).
    pub saved: Option<PathBuf>,
    /// The other Render Outputs' pictures by name (render outputs v1),
    /// then the light groups'.
    pub pictures: Vec<(String, Vec<f32>)>,
    /// Per cached output (the picture, `pictures`): kind and colour space.
    pub meta: Vec<image_viewer::OutputMeta>,
    /// Writing the cache files: an error leaves the render out of the strip.
    pub cached: Result<(), String>,
}
type Finished = Result<Done, String>;

/// The function that renders; tests swap in a stand-in for Cycles.
pub(crate) type Renderer =
    fn(&[RenderMesh], &Settings, Option<Vec<f32>>, &RenderControl) -> operandi_core::Result<Image>;

/// A render running on its own thread.
pub(crate) struct Job {
    rx: mpsc::Receiver<Finished>,
    pub(crate) control: Arc<RenderControl>,
    /// The part being rendered; the whole picture when `None`.
    pub(crate) region: Option<Region>,
    /// The last preview pasted into the picture.
    seen: u64,
    started: Instant,
    paused_since: Option<Instant>,
    paused_for: Duration,
    /// The history slot the finished render becomes.
    slot: u64,
    /// For its Statistics (IV-35b).
    pub(crate) samples_total: u32,
    pub(crate) triangles: u64,
    pub(crate) camera: String,
    scene: String,
}

impl Job {
    /// Render time so far, leaving out time spent paused.
    pub(crate) fn elapsed(&self) -> Duration {
        let paused = self.paused_for + self.paused_since.map_or(Duration::ZERO, |t| t.elapsed());
        self.started.elapsed().saturating_sub(paused)
    }
}

/// The picture being rendered into, or the last one rendered: linear RGBA
/// floats, top row first (never 8-bit; IV-14 transforms only for display).
pub(crate) struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<f32>,
    /// The history slot it is, once finished.
    pub slot: Option<u64>,
}

/// What an MCP `render` call changes about the next render.
#[derive(Clone, Debug, Default)]
pub(crate) struct RenderOverrides {
    /// Picture size in pixels instead of the camera's render size.
    pub size: Option<[u32; 2]>,
    pub samples: Option<u32>,
    /// The file to write instead of the next numbered one.
    pub path: Option<PathBuf>,
    /// Render through this camera instead of the render camera.
    pub camera: Option<u64>,
}

/// A finished render, for MCP `render` / `render_status`.
#[derive(Clone, Debug)]
pub(crate) struct RenderDone {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub seconds: f64,
    pub device: String,
    pub denoised: bool,
}

pub(crate) struct RenderState {
    pub(crate) job: Option<Job>,
    /// Applied to the next render, then cleared.
    pub(crate) overrides: Option<RenderOverrides>,
    /// Renders finished (or failed) so far, so a waiting MCP call can tell
    /// its render from an earlier one.
    pub(crate) finished: u64,
    /// The last render's outcome.
    pub(crate) last: Option<Result<RenderDone, String>>,
    /// The file the running render will write.
    pub(crate) rendering_to: Option<PathBuf>,
    pub(crate) picture: Option<Picture>,
    /// The Render window is showing.
    pub(crate) open: bool,
    /// The rectangle drawn on the picture, in picture pixels.
    pub(crate) region: Option<Region>,
    /// Picture pixel where the current drag began.
    pub(crate) drag_from: Option<[u32; 2]>,
    pub(crate) texture: Option<TextureHandle>,
    pub(crate) texture_stale: bool,
    renderer: Renderer,
    /// Where pictures are saved; ~/Pictures/Operandi Renders when `None`.
    folder: Option<PathBuf>,
    /// The Image Viewer's slots (IV-08..13b), app-wide.
    pub(crate) history: History,
    /// Preferences > Rendering > Render Window.
    pub(crate) prefs: ViewerPreferences,
    /// What the window shows and how (image_viewer_ui.rs).
    pub(crate) viewer: crate::image_viewer_ui::ViewerState,
    /// The picture's area in the window, as of the last frame.
    pub(crate) viewer_area: Rect,
    /// Bring the Render window to the front on the next frame.
    pub(crate) focus: bool,
    /// The main window's points-to-pixels scale, as of the last frame.
    pub(crate) pixels_per_point: f32,
    /// The last render's other Render Outputs (render outputs v1), and
    /// which one the window shows (`None`: the Final Color picture).
    pub(crate) outputs: Vec<(String, Vec<f32>)>,
    pub(crate) shown: Option<usize>,
    /// The picture's and `outputs`' kinds and colour spaces, as
    /// `Slot::meta` (output k is entry k + 1).
    pub(crate) meta: Vec<image_viewer::OutputMeta>,
    /// The shown picture with Bloom and Glare (post_effects_ui.rs).
    pub(crate) post: crate::post_effects_ui::PostCache,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            job: None,
            overrides: None,
            finished: 0,
            last: None,
            rendering_to: None,
            picture: None,
            open: false,
            region: None,
            drag_from: None,
            texture: None,
            texture_stale: false,
            renderer: operandi_cycles::render_live,
            folder: None,
            // The app points this at Recent Frames when it starts.
            history: History::empty(
                std::env::temp_dir().join(format!("operandi-recent-frames-{}", std::process::id())),
            ),
            prefs: ViewerPreferences::default(),
            viewer: Default::default(),
            viewer_area: Rect::from_min_size(Pos2::ZERO, Vec2::new(720., 480.)),
            focus: false,
            pixels_per_point: 1.0,
            outputs: Vec::new(),
            meta: Vec::new(),
            shown: None,
            post: Default::default(),
        }
    }
}

impl RenderState {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// The running render's slot and how many previews it has shown.
    pub(crate) fn live_generation(&self) -> Option<(u64, u64)> {
        self.job.as_ref().map(|j| (j.slot, j.seen))
    }

    #[cfg(test)]
    pub(crate) fn set_renderer(&mut self, renderer: Renderer, folder: PathBuf) {
        self.renderer = renderer;
        self.history = History::empty(folder.join("Recent Frames"));
        self.folder = Some(folder);
    }

    /// The running render's control, if any.
    pub(crate) fn control(&self) -> Option<&RenderControl> {
        self.job.as_ref().map(|j| j.control.as_ref())
    }

    pub(crate) fn toggle_pause(&self) {
        if let Some(c) = self.control() {
            if c.pause_requested() {
                c.resume();
            } else {
                c.pause();
            }
        }
    }
}

/// The viewport camera in the renderer's terms. Matches `Camera::project`:
/// the viewport's half height spans 1/1.8 at unit depth.
pub(crate) fn render_view(camera: &Camera) -> Result<operandi_cycles::View, String> {
    if camera.view != View::Perspective {
        return Err("Render Viewport works from the Perspective view for now".into());
    }
    let (position, right, up, forward) = camera.basis();
    Ok(operandi_cycles::View::perspective(
        position,
        right,
        up,
        forward,
        1.0 / 1.8,
    ))
}

/// ~/Pictures/Operandi Renders, where pictures are saved.
pub(crate) fn render_folder() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or("No home folder to save the render in")?;
    Ok(PathBuf::from(home)
        .join("Pictures")
        .join("Operandi Renders"))
}

/// The next free "<prefix> NNN.png" in `folder` (the render folder when
/// `None`), creating the folder.
pub(crate) fn numbered_path(
    folder: Option<&PathBuf>,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let folder = match folder {
        Some(f) => f.clone(),
        None => render_folder()?,
    };
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    (1..100_000)
        .map(|n| folder.join(format!("{prefix} {n:03}.{extension}")))
        .find(|p| !p.exists())
        .ok_or_else(|| "Render folder is full".into())
}

/// Where light group `label`'s pass of the render saved at `render` goes:
/// "Render 3.png" → "Render 3 (Key).png" (in the render's own format), the
/// label kept to letters, digits, spaces, '-' and '_'.
pub(crate) fn light_group_path(render: &std::path::Path, label: &str) -> PathBuf {
    let safe: String = label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = render
        .file_stem()
        .map_or("Render".into(), |s| s.to_string_lossy().into_owned());
    let extension = render
        .extension()
        .map_or("png".into(), |e| e.to_string_lossy().into_owned());
    render.with_file_name(format!("{stem} ({safe}).{extension}"))
}

/// Where a Render Output beside the render at `render` saves, and in which
/// format (RO-Q04 v2): its Output Filename (relative to the render's
/// folder) or "Render N (Name)", with its Format's extension; `None` keeps
/// the render's own format.
pub(crate) fn output_file(
    render: &std::path::Path,
    output: &operandi_core::render_outputs::RenderOutput,
) -> (PathBuf, Option<crate::image_viewer::SaveFormat>) {
    use crate::image_viewer::SaveFormat;
    let s = &output.settings;
    let format = SaveFormat::per_output().find(|f| f.label() == s.format.trim());
    let base = if s.filename.trim().is_empty() {
        light_group_path(render, &output.name)
    } else {
        let named = PathBuf::from(s.filename.trim());
        if named.is_absolute() {
            named
        } else {
            render
                .parent()
                .map_or(named.clone(), |folder| folder.join(&named))
        }
    };
    let path = match format {
        Some(f) => crate::image_viewer::with_extension(&base, f),
        None if base.extension().is_none() => base.with_extension(
            render
                .extension()
                .map_or("png".into(), |e| e.to_string_lossy().into_owned()),
        ),
        None => base,
    };
    (path, format)
}

/// 8-bit RGBA: sRGB-encoded, or the linear numbers as they are.
fn rgba8(rgba: &[f32], srgb: bool) -> Vec<u8> {
    if srgb {
        return operandi_cycles::srgb8(rgba);
    }
    rgba.iter()
        .map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
        .collect()
}

fn output_path(folder: Option<&PathBuf>, extension: &str) -> Result<PathBuf, String> {
    numbered_path(folder, "Render", extension)
}

/// Where a picture of `size` pixels sits, scaled to fit inside `area`
/// (never enlarged past 1:1) and centred.
pub(crate) fn fit_picture(area: Rect, size: [u32; 2]) -> Rect {
    let (w, h) = (size[0].max(1) as f32, size[1].max(1) as f32);
    let scale = (area.width() / w).min(area.height() / h).min(1.0);
    Rect::from_center_size(area.center(), Vec2::new(w * scale, h * scale))
}

/// The picture pixel under `pos`, clamped to the picture.
pub(crate) fn pixel_at(shown: Rect, size: [u32; 2], pos: Pos2) -> [u32; 2] {
    let u = ((pos.x - shown.left()) / shown.width().max(1.0)).clamp(0.0, 1.0);
    let v = ((pos.y - shown.top()) / shown.height().max(1.0)).clamp(0.0, 1.0);
    [
        ((u * size[0] as f32) as u32).min(size[0].saturating_sub(1)),
        ((v * size[1] as f32) as u32).min(size[1].saturating_sub(1)),
    ]
}

/// The rectangle between two picture pixels, both included. `None` when it
/// is too small to mean anything (a click rather than a drag).
pub(crate) fn region_between(a: [u32; 2], b: [u32; 2]) -> Option<Region> {
    let (x0, x1) = (a[0].min(b[0]), a[0].max(b[0]));
    let (y0, y1) = (a[1].min(b[1]), a[1].max(b[1]));
    let region = Region {
        x: x0,
        y: y0,
        width: x1 - x0 + 1,
        height: y1 - y0 + 1,
    };
    (region.width >= 4 && region.height >= 4).then_some(region)
}

/// Picture pixels to window points.
pub(crate) fn region_rect(shown: Rect, size: [u32; 2], r: Region) -> Rect {
    let sx = shown.width() / size[0].max(1) as f32;
    let sy = shown.height() / size[1].max(1) as f32;
    Rect::from_min_size(
        shown.min + Vec2::new(r.x as f32 * sx, r.y as f32 * sy),
        Vec2::new(r.width as f32 * sx, r.height as f32 * sy),
    )
}

impl Operandi {
    /// Picture size and view: the scene's render camera at the film-back
    /// render frame (720 × 480, provisional CL-Q02), else the viewport
    /// camera at the viewport's on-screen pixel size.
    pub(crate) fn render_frame(
        &self,
        pixels_per_point: f32,
    ) -> Result<(u32, u32, operandi_cycles::View), String> {
        let scene = &self.editor.scene;
        if let Some(id) = scene.render_camera() {
            // CP-V17: the camera's Resolution Override, else the frame.
            let [w, h] = crate::camera_light_ui::camera_render_size(scene, id);
            if let Some(view) =
                operandi_cycles::camera_view(scene, id, f64::from(w) / f64::from(h.max(1)))
            {
                return Ok((w, h, view));
            }
        }
        let view = render_view(&self.camera)?;
        let r = self.viewport_rect;
        let scale = pixels_per_point.max(1.0);
        let (w, h) = ((r.width() * scale) as u32, (r.height() * scale) as u32);
        if w < 16 || h < 16 {
            return Err("The viewport is too small to render".into());
        }
        Ok((w, h, view))
    }

    /// `render_frame` with an MCP call's camera and size.
    fn render_frame_with(
        &self,
        pixels_per_point: f32,
        overrides: &RenderOverrides,
    ) -> Result<(u32, u32, operandi_cycles::View), String> {
        let scene = &self.editor.scene;
        let camera = match overrides.camera {
            Some(id) => {
                if !scene.items.iter().any(|i| i.id == id && i.camera.is_some()) {
                    return Err(format!("Item {id} is not a camera"));
                }
                Some(id)
            }
            None => scene.render_camera(),
        };
        let Some(id) = camera else {
            return match overrides.size {
                // A given size needs no viewport of that size.
                Some([w, h]) => Ok((w, h, render_view(&self.camera)?)),
                None => self.render_frame(pixels_per_point),
            };
        };
        let [w, h] = overrides
            .size
            .unwrap_or_else(|| crate::camera_light_ui::camera_render_size(scene, id));
        let view = operandi_cycles::camera_view(scene, id, f64::from(w) / f64::from(h.max(1)))
            .ok_or("The camera can't render (check its settings)")?;
        Ok((w, h, view))
    }

    /// File > Render Viewport: render the whole picture.
    pub(crate) fn start_render(&mut self, ctx: &egui::Context) {
        self.begin_render(ctx.pixels_per_point(), false);
    }

    /// Starts a render in the Render window; `region` re-renders only the
    /// rectangle drawn on the current picture.
    pub(crate) fn begin_render(&mut self, pixels_per_point: f32, region: bool) {
        if self.render.busy() {
            return;
        }
        self.render.open = true;
        // RP-Q9: never two Cycles renders flat out; the Preview waits.
        self.hold_preview_for_final();
        match self.prepare_render(pixels_per_point, region) {
            Ok(note) => {
                self.status = if self.render.job.as_ref().is_some_and(|j| j.region.is_some()) {
                    "Rendering the region with Cycles…".into()
                } else {
                    "Rendering with Cycles… (the first GPU render takes longer)".into()
                };
                if let Some(note) = note {
                    self.status = format!("{} · {note}", self.status);
                }
                // CL-Q14/Q15, GR-Q04: say what the lights and shaders can't carry over.
                let mut notes = operandi_cycles::light_notes(&self.editor.scene);
                notes.extend(operandi_cycles::shading_notes(&self.editor.scene));
                if !notes.is_empty() {
                    self.status = format!("{} · {}", self.status, notes.join(" · "));
                }
            }
            Err(e) => {
                self.status = e;
                self.release_preview_after_final();
            }
        }
    }

    /// Sets up the job; returns a note when a region render became a full
    /// one.
    fn prepare_render(
        &mut self,
        pixels_per_point: f32,
        region: bool,
    ) -> Result<Option<String>, String> {
        // An MCP call always gets a file it can read back.
        let from_mcp = self.render.overrides.is_some();
        let overrides = self.render.overrides.take().unwrap_or_default();
        // Render fix F6: a scene with no polygons still renders its
        // environment (handy for checking an HDRI and its rotation).
        let mut meshes = operandi_cycles::scene_meshes(
            &self
                .editor
                .evaluated(operandi_core::evaluate::Purpose::Final),
        )?;
        // Render outputs v1 §1: every enabled output; none, nothing to do.
        let outputs = self.editor.scene.shading_tree().render_outputs();
        if outputs.is_empty() {
            return Err("Nothing to render: no Render Output is enabled".into());
        }
        let (w, h, view) = self.render_frame_with(pixels_per_point, &overrides)?;
        let triangles = meshes.iter().map(|m| m.triangles.len() as u64).sum();
        let camera = match overrides.camera.or(self.editor.scene.render_camera()) {
            Some(id) => self
                .editor
                .scene
                .item(id)
                .map_or_else(|| "Camera".into(), |i| i.name.clone()),
            None => "Viewport camera".into(),
        };
        // Render fix F3: only the scene's own lights, none added.
        let mut settings = operandi_cycles::Settings::studio(w, h, view)
            .with_only_scene_lights(&self.editor.scene)
            .with_scene_world(&self.editor.scene);
        settings.passes = operandi_cycles::output_passes(&outputs);
        settings.values = operandi_cycles::output_values(&outputs);
        settings.ao_distance = operandi_cycles::ao_distance(&outputs);
        // Preferences > Rendering > Render Threads.
        settings.threads = self.render_preferences.render_threads();
        // The scene's render settings; an MCP call's samples win.
        let scene_render = self.editor.scene.render.clone();
        settings.samples = overrides.samples.unwrap_or(scene_render.samples);
        // RI-Q05: the Final Color output's Denoising row.
        settings.denoise = self.editor.scene.render_denoise();
        settings.adaptive_threshold = scene_render.adaptive_threshold as f32;
        settings.rays = operandi_cycles::Rays::from_render(&scene_render);
        // Group D: Light Path Expressions' light groups, Shaded AA
        // Samples' pass, and the notes they leave for the status line.
        let output_context = operandi_cycles::OutputContext::of(&self.editor.scene, &settings);
        let output_notes =
            operandi_cycles::prepare_outputs(&mut settings, &mut meshes, &outputs, &output_context);
        let mut note = None;
        let mut base = None;
        if region {
            match (&self.render.picture, self.render.region) {
                (Some(p), Some(r)) if p.width == w && p.height == h && r.fits(w, h) => {
                    settings.region = Some(r);
                    base = Some(p.rgba.clone());
                }
                (Some(_), Some(_)) => {
                    note = Some("the picture size changed, so the whole picture".into());
                    self.render.region = None;
                }
                _ => {}
            }
        }
        if settings.region.is_none() {
            // A fresh picture fills in as it renders.
            self.render.picture = Some(Picture {
                width: w,
                height: h,
                rgba: vec![0.0; w as usize * h as usize * 4],
                slot: None,
            });
            self.render.outputs.clear();
            self.render.shown = None;
            self.render.texture_stale = true;
        }
        // The window follows the render while it runs.
        self.render.viewer.show_live();
        // The render becomes this history slot; its files go to the cache.
        let slot_id = self.render.history.reserve_id();
        let cache = self.render.history.folder.clone();
        let thumb_view = self.render.prefs.default_view;
        // Preferences > File I/O: the picture's format and encoding, when
        // it is also saved outside the cache.
        let path = match overrides.path {
            Some(path) => {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                Some(path)
            }
            None if from_mcp || self.render.prefs.save_to_pictures => Some(output_path(
                self.render.folder.as_ref(),
                self.file_io_preferences.image_format.extension(),
            )?),
            None => None,
        };
        let image_options = self.file_io_preferences.image();
        let post = self.editor.scene.render.post.clone();
        let samples_total = settings.samples;
        self.render.rendering_to = Some(path.clone().unwrap_or_else(|| {
            Slot {
                id: slot_id,
                ..Default::default()
            }
            .output_file(&cache, 0)
        }));
        // CP-V26: the render camera's Background Image shows through
        // where nothing covers it; the film renders transparent for it.
        let background = self.render_camera_background();
        if background.is_some() {
            settings.transparent = true;
        }
        let control = Arc::new(RenderControl::default());
        let (tx, rx) = mpsc::channel();
        let renderer = self.render.renderer;
        let job_region = settings.region;
        let thread_control = Arc::clone(&control);
        std::thread::Builder::new()
            .name("cycles-render".into())
            .spawn(move || {
                let result = renderer(&meshes, &settings, base, &thread_control)
                    .and_then(|mut image| {
                        // Group D (D-Q2): the lamps-only, shadowless helper
                        // render after the main one, when an output needs it.
                        if let Some(helper) =
                            operandi_cycles::unshadowed_settings(&settings, &outputs)
                        {
                            thread_control.set_step(Some(operandi_cycles::UNSHADOWED_STEP));
                            let done = renderer(&meshes, &helper, None, &thread_control);
                            thread_control.set_step(None);
                            operandi_cycles::attach_unshadowed(&mut image, done?);
                        }
                        Ok(image)
                    })
                    .map(|mut image| {
                        if let Some((pixels, overscan)) = &background {
                            crate::camera_props_ui::composite_background(
                                &mut image.rgba,
                                image.width,
                                image.height,
                                pixels,
                                *overscan,
                            );
                        }
                        image
                    })
                    .and_then(|mut image| {
                        // Each output's picture, processed; the first Final
                        // Color output's processing is the picture's own.
                        let mut pictures =
                            operandi_cycles::output_pictures_in(&outputs, &image, &output_context);
                        // Light Path Expressions' hidden light groups are
                        // neither shown nor saved (RO-Q25).
                        image
                            .light_groups
                            .retain(|(g, _)| !operandi_cycles::is_hidden_group(g));
                        let first_final = outputs.iter().position(|o| {
                            o.effect == operandi_core::shader_tree::OutputEffect::FinalColor
                        });
                        if let Some(k) = first_final {
                            image.rgba = pictures.remove(k).1;
                        }
                        // Group C: each cached output's kind and colour
                        // space, the picture first.
                        let mut meta = vec![first_final.map_or(
                            image_viewer::OutputMeta {
                                effect: Some(operandi_core::shader_tree::OutputEffect::FinalColor),
                                ..Default::default()
                            },
                            |k| image_viewer::OutputMeta::of(&outputs[k]),
                        )];
                        meta.extend(
                            outputs
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| Some(*i) != first_final)
                                .map(|(_, o)| image_viewer::OutputMeta::of(o)),
                        );
                        meta.extend(
                            image
                                .light_groups
                                .iter()
                                .map(|_| image_viewer::OutputMeta::default()),
                        );
                        // IV-12: every output, as float EXR, in the cache.
                        let mut all: Vec<&[f32]> = vec![&image.rgba];
                        all.extend(pictures.iter().map(|(_, p)| p.as_slice()));
                        all.extend(image.light_groups.iter().map(|(_, p)| p.as_slice()));
                        let slot = Slot {
                            id: slot_id,
                            width: image.width,
                            height: image.height,
                            outputs: vec![String::new(); all.len()],
                            ..Default::default()
                        };
                        let mut cached =
                            image_viewer::write_slot_files(&cache, &slot, &all, thumb_view);
                        // Cryptomatte's raw ranked ids, for an OpenEXR
                        // Multilayer save later.
                        let crypto = operandi_cycles::cryptomatte_layers(&outputs, &image);
                        if cached.is_ok() && !crypto.is_empty() {
                            cached = crate::exr_multilayer::write(
                                &slot.crypto_file(&cache),
                                image.width,
                                image.height,
                                &[],
                                &crypto,
                            );
                        }
                        let Some(path) = path else {
                            let mut pictures = pictures;
                            pictures.extend(image.light_groups.iter().cloned());
                            return Ok(Done {
                                image,
                                saved: None,
                                pictures,
                                meta,
                                cached,
                            });
                        };
                        // The picture in its colour space (8-bit: sRGB
                        // unless set Linear).
                        let picture_srgb = meta[0].writes_srgb(
                            image_viewer::SaveFormat::Png8,
                            image_viewer::ViewTransform::Srgb,
                        );
                        // Bloom and Glare go on the saved picture; the
                        // cache and the slot keep it as rendered, so the
                        // Render window can change them later.
                        let saved_rgba = if post.is_active() {
                            let mut px = image.rgba.clone();
                            operandi_core::post_effects::apply(
                                &post,
                                &mut px,
                                image.width,
                                image.height,
                            );
                            std::borrow::Cow::Owned(px)
                        } else {
                            std::borrow::Cow::Borrowed(&image.rgba)
                        };
                        crate::file_io_preferences::save_rgba(
                            &path,
                            &rgba8(&saved_rgba, picture_srgb),
                            image.width,
                            image.height,
                            image_options,
                        )?;
                        // The other outputs next to it: "Render 3 (Depth Output).png".
                        let others = outputs
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| Some(*i) != first_final)
                            .map(|(_, o)| o);
                        for (((_, rgba), output), meta) in
                            pictures.iter().zip(others).zip(&meta[1..])
                        {
                            // The picture already carries its alpha: an
                            // untouched Alpha output isn't saved apart (RO-Q04).
                            if output.effect == operandi_core::shader_tree::OutputEffect::Alpha
                                && output.settings.is_default()
                            {
                                continue;
                            }
                            let (file, format) = output_file(&path, output);
                            // Group C: the output's colour space decides
                            // the numbers written.
                            match format {
                                Some(format) => crate::image_viewer::save_output(
                                    &file,
                                    rgba,
                                    image.width,
                                    image.height,
                                    format,
                                    *meta,
                                    image_viewer::ViewTransform::Srgb,
                                    image_options,
                                )?,
                                None => crate::file_io_preferences::save_rgba(
                                    &file,
                                    &rgba8(
                                        rgba,
                                        meta.writes_srgb(
                                            image_viewer::SaveFormat::Png8,
                                            image_viewer::ViewTransform::Srgb,
                                        ),
                                    ),
                                    image.width,
                                    image.height,
                                    image_options,
                                )?,
                            }
                        }
                        // LP-22: each light group's pass next to it, in the
                        // same format (Preferences > File I/O).
                        for (label, rgba) in &image.light_groups {
                            let pass = light_group_path(&path, label);
                            crate::file_io_preferences::save_rgba(
                                &pass,
                                &operandi_cycles::srgb8(rgba),
                                image.width,
                                image.height,
                                image_options,
                            )?;
                        }
                        pictures.extend(image.light_groups.iter().cloned());
                        Ok(Done {
                            image,
                            saved: Some(path),
                            pictures,
                            meta,
                            cached,
                        })
                    });
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        self.render.job = Some(Job {
            rx,
            control,
            region: job_region,
            seen: 0,
            started: Instant::now(),
            paused_since: None,
            paused_for: Duration::ZERO,
            slot: slot_id,
            samples_total,
            triangles,
            camera,
            scene: self.scene_label(),
        });
        let notes: Vec<String> = note.into_iter().chain(output_notes).collect();
        Ok((!notes.is_empty()).then(|| notes.join(" · ")))
    }

    /// Called every frame: pastes in-progress pictures, keeps the pause
    /// clock, and reports a finished render.
    pub(crate) fn poll_render(&mut self, ctx: &egui::Context) {
        self.render.pixels_per_point = ctx.pixels_per_point();
        let Some(job) = &mut self.render.job else {
            if self.preview.held_for_final {
                self.release_preview_after_final();
            }
            return;
        };
        let progress = job.control.progress();
        match (progress.paused, job.paused_since) {
            (true, None) => job.paused_since = Some(Instant::now()),
            (false, Some(t)) => {
                job.paused_for += t.elapsed();
                job.paused_since = None;
            }
            _ => {}
        }
        if let Some((generation, area)) = job.control.preview_since(job.seen)
            && let Some(p) = &mut self.render.picture
        {
            job.seen = generation;
            let region = job.region.unwrap_or(Region::whole(p.width, p.height));
            operandi_cycles::paste_region(&mut p.rgba, p.width, region, &area);
            self.render.texture_stale = true;
        }
        match job.rx.try_recv() {
            Ok(Ok(Done {
                image,
                saved,
                pictures,
                meta,
                cached,
            })) => {
                // The output names, for the picker and the slot.
                let mut names = vec![image_viewer::FINAL_COLOR.to_string()];
                names.extend(pictures.iter().map(|(n, _)| n.clone()));
                self.render.outputs = pictures;
                self.render.meta = meta.clone();
                self.render.shown = None;
                let seconds = job.elapsed().as_secs_f64();
                let device = if image.denoised {
                    format!("{}, denoised", image.device)
                } else {
                    image.device.clone()
                };
                let early = job.control.finish_requested();
                let samples = if early {
                    format!(" · finished early at {} samples", image.samples)
                } else {
                    String::new()
                };
                let what = if job.region.is_some() {
                    "Region rendered"
                } else {
                    "Rendered"
                };
                let place = match &saved {
                    Some(path) => format!(" · saved {}", path.display()),
                    None => String::new(),
                };
                self.status = format!("{what} in {seconds:.1} s on {device}{samples}{place}");
                let slot = Slot {
                    id: job.slot,
                    number: 0,
                    kind: SlotKind::Render,
                    locked: false,
                    scene: job.scene.clone(),
                    time: image_viewer::now(),
                    width: image.width,
                    height: image.height,
                    outputs: names,
                    meta,
                    saved: saved.clone(),
                    stats: Stats {
                        seconds,
                        samples: image.samples,
                        samples_total: job.samples_total,
                        peak_memory: None,
                        triangles: job.triangles,
                        camera: job.camera.clone(),
                        device: image.device.clone(),
                        denoised: image.denoised,
                        region: job.region.is_some(),
                        finished_early: early,
                    },
                };
                let path = saved
                    .clone()
                    .unwrap_or_else(|| slot.output_file(&self.render.history.folder, 0));
                let slot_id = match cached {
                    Ok(()) => {
                        let mut slot = slot;
                        slot.number = self.render.history.take_number();
                        let id = slot.id;
                        let max = self.render.prefs.max_render_slots;
                        for gone in self.render.history.add(slot, max) {
                            self.render.viewer.forget(gone.id);
                        }
                        if let Err(e) = self.render.history.save_index() {
                            self.status =
                                format!("{} · the render history wasn't saved: {e}", self.status);
                        }
                        Some(id)
                    }
                    Err(e) => {
                        self.status =
                            format!("{} · not kept in the render history: {e}", self.status);
                        None
                    }
                };
                self.render.finished += 1;
                self.render.last = Some(Ok(RenderDone {
                    path: path.clone(),
                    width: image.width,
                    height: image.height,
                    samples: image.samples,
                    seconds,
                    device: image.device.clone(),
                    denoised: image.denoised,
                }));
                self.render.picture = Some(Picture {
                    width: image.width,
                    height: image.height,
                    rgba: image.rgba,
                    slot: slot_id,
                });
                if let Some(id) = slot_id {
                    self.render.viewer.select_only(id);
                }
                self.render.texture_stale = true;
                self.render.job = None;
            }
            Ok(Err(e)) => {
                self.status = if job.control.cancelled() {
                    "Render cancelled".into()
                } else {
                    format!("Render failed: {e}")
                };
                self.render.finished += 1;
                self.render.last = Some(Err(self.status.clone()));
                self.render.job = None;
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = "Render failed: the render thread stopped".into();
                self.render.finished += 1;
                self.render.last = Some(Err(self.status.clone()));
                self.render.job = None;
            }
        }
    }

    /// Closing the window cancels a render in progress.
    pub(crate) fn close_render_window(&mut self) {
        if let Some(c) = self.render.control() {
            c.cancel();
        }
        self.render.open = false;
        self.render.drag_from = None;
        self.save_render_window();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw};

    /// Stands in for Cycles: paints the rendered area a flat 0.5 grey over
    /// `base`, or, for a 100-sample render, waits to be finished or
    /// cancelled first.
    fn fake(
        _meshes: &[RenderMesh],
        s: &Settings,
        base: Option<Vec<f32>>,
        control: &RenderControl,
    ) -> operandi_core::Result<Image> {
        let _ = s;
        if s.samples == 100 {
            while !control.finish_requested() {
                if control.cancelled() {
                    return Err("Cancelled".into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let mut rgba = base.unwrap_or_else(|| vec![0.0; s.width as usize * s.height as usize * 4]);
        let r = s.region.unwrap_or(Region::whole(s.width, s.height));
        let area = vec![0.5; r.width as usize * r.height as usize * 4];
        operandi_cycles::paste_region(&mut rgba, s.width, r, &area);
        Ok(Image {
            width: s.width,
            height: s.height,
            rgba,
            device: "Test".into(),
            used_gpu: false,
            denoised: false,
            samples: s.samples,
            light_groups: Vec::new(),
            passes: Vec::new(),
            values: Vec::new(),
            cryptomatte_manifests: Vec::new(),
        })
    }

    fn app(name: &str) -> (Operandi, egui::Context, PathBuf) {
        let folder =
            std::env::temp_dir().join(format!("operandi-render-ui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let mut a = box_app();
        a.camera.view = View::Perspective;
        a.render.set_renderer(fake, folder.clone());
        let ctx = egui::Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
        }
        (a, ctx, folder)
    }

    fn wait(a: &mut Operandi, ctx: &egui::Context) {
        // Generous: writing the cache's EXR files is slow in debug builds.
        for _ in 0..12_000 {
            if !a.render.busy() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
            a.poll_render(ctx);
        }
        panic!("render never finished");
    }

    #[test]
    fn a_region_render_repaints_only_the_region_and_saves_the_picture() {
        let (mut a, ctx, folder) = app("region");
        a.start_render(&ctx);
        assert!(a.render.open && a.render.busy());
        wait(&mut a, &ctx);
        assert!(a.status.starts_with("Rendered in"), "{}", a.status);
        let p = a.render.picture.as_ref().unwrap();
        let (w, h) = (p.width, p.height);
        assert!(p.rgba.iter().all(|&v| v == 0.5));
        // IV-12: it is a slot, its floats in the cache, not in Pictures.
        let slot = a.render.history.get(p.slot.unwrap()).unwrap();
        assert!(slot.output_file(&a.render.history.folder, 0).exists());
        assert!(slot.saved.is_none());
        // Make the picture recognisable, then draw a region and render it.
        a.render.picture.as_mut().unwrap().rgba.fill(0.25);
        let r = Region {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        };
        a.render.region = Some(r);
        a.begin_render(1.0, true);
        wait(&mut a, &ctx);
        assert!(a.status.starts_with("Region rendered"), "{}", a.status);
        let p = a.render.picture.as_ref().unwrap();
        assert_eq!((p.width, p.height), (w, h));
        let at = |x: u32, y: u32| p.rgba[((y * w + x) * 4) as usize];
        assert_eq!(at(10, 20), 0.5);
        assert_eq!(at(39, 59), 0.5);
        assert_eq!(at(40, 59), 0.25);
        assert_eq!(at(9, 20), 0.25);
        assert_eq!(at(10, 60), 0.25);
        // Two slots, nothing in Pictures, and the region stays for another go.
        assert_eq!(a.render.history.slots.len(), 2);
        assert!(a.render.history.slots[0].stats.region);
        assert_eq!(
            std::fs::read_dir(&folder).unwrap().count(),
            1,
            "only the cache"
        );
        assert_eq!(a.render.region, Some(r));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_region_from_another_picture_size_renders_the_whole_picture() {
        let (mut a, ctx, folder) = app("resized");
        a.render.picture = Some(Picture {
            width: 64,
            height: 64,
            rgba: vec![0.25; 64 * 64 * 4],
            slot: None,
        });
        a.render.region = Some(Region {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        });
        a.begin_render(1.0, true);
        assert!(a.status.contains("whole picture"), "{}", a.status);
        wait(&mut a, &ctx);
        let p = a.render.picture.as_ref().unwrap();
        assert_ne!((p.width, p.height), (64, 64));
        assert!(p.rgba.iter().all(|&v| v == 0.5));
        assert_eq!(a.render.region, None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn finish_keeps_the_picture_and_closing_the_window_cancels() {
        let (mut a, ctx, folder) = app("steer");
        // The fake waits when asked for 100 samples.
        a.render.set_renderer(
            |m, s, b, c| {
                fake(
                    m,
                    &Settings {
                        samples: 100,
                        ..s.clone()
                    },
                    b,
                    c,
                )
            },
            folder.clone(),
        );
        a.start_render(&ctx);
        a.render.toggle_pause();
        assert!(a.render.control().unwrap().pause_requested());
        a.render.toggle_pause();
        assert!(!a.render.control().unwrap().pause_requested());
        a.render.control().unwrap().finish();
        wait(&mut a, &ctx);
        assert!(
            a.status.contains("finished early at 100 samples"),
            "{}",
            a.status
        );
        assert!(a.render.picture.as_ref().unwrap().slot.is_some());
        assert!(a.render.history.slots[0].stats.finished_early);
        // Again, but close the window: the render stops and keeps nothing.
        a.start_render(&ctx);
        a.close_render_window();
        assert!(!a.render.open);
        wait(&mut a, &ctx);
        assert_eq!(a.status, "Render cancelled");
        assert_eq!(
            a.render.history.slots.len(),
            1,
            "a cancelled render keeps nothing"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_render_window_draws_with_and_without_a_picture() {
        let (mut a, ctx, folder) = app("window");
        a.render.open = true;
        draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
        a.start_render(&ctx);
        wait(&mut a, &ctx);
        for _ in 0..2 {
            draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
        }
        assert!(a.render.texture.is_some());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn renders_go_to_pictures_only_when_asked_and_the_history_survives_a_restart() {
        let (mut a, ctx, folder) = app("pictures");
        a.start_render(&ctx);
        wait(&mut a, &ctx);
        assert!(!a.status.contains("saved"), "{}", a.status);
        a.render.prefs.save_to_pictures = true;
        a.start_render(&ctx);
        wait(&mut a, &ctx);
        assert!(a.status.contains("saved"), "{}", a.status);
        let pngs: Vec<_> = std::fs::read_dir(&folder)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
            .collect();
        assert_eq!(pngs.len(), 1);
        let newest = a.render.history.slots[0].clone();
        assert_eq!(newest.saved.as_deref(), Some(pngs[0].path().as_path()));
        assert_eq!((newest.number, a.render.history.slots[1].number), (2, 1));
        assert_eq!(newest.stats.device, "Test");
        assert_eq!(newest.stats.camera, "Viewport camera");
        assert!(newest.stats.triangles > 0);
        // Quit and come back: the same strip.
        let back = image_viewer::History::load(a.render.history.folder.clone());
        assert_eq!(back.slots, a.render.history.slots);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn max_render_slots_trims_the_strip_and_open_render_window_shows_the_newest() {
        let (mut a, ctx, folder) = app("cap");
        a.render.prefs.max_render_slots = 2;
        for _ in 0..3 {
            a.start_render(&ctx);
            wait(&mut a, &ctx);
        }
        let numbers: Vec<u32> = a.render.history.slots.iter().map(|s| s.number).collect();
        assert_eq!(numbers, vec![3, 2]);
        // Closed, then Open Render Window (no render): the newest shows.
        a.close_render_window();
        a.render.picture = None;
        a.render.viewer.show_live();
        a.open_render_window();
        assert!(a.render.open && !a.render.busy());
        assert_eq!(a.render.viewer.primary, Some(a.render.history.slots[0].id));
        // Key 2 picks the older slot, read back from the cache.
        a.viewer_key(crate::image_viewer_ui::ViewerKey::Slot(2));
        let older = a.render.history.slots[1].id;
        assert_eq!(a.render.viewer.primary, Some(older));
        assert!(
            a.shown_pixels()
                .is_some_and(|(_, _, px)| px.iter().all(|v| *v == 0.5))
        );
        for _ in 0..2 {
            draw(&mut a, &ctx, Vec2::new(1000., 700.), vec![]);
        }
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn save_image_and_load_image_go_through_the_strip() {
        let (mut a, ctx, folder) = app("save-load");
        a.start_render(&ctx);
        wait(&mut a, &ctx);
        let id = a.render.history.slots[0].id;
        let d = crate::image_viewer_ui::SaveDialog {
            slot: Some(id),
            output: None,
            format: image_viewer::SaveFormat::Exr,
        };
        let out = folder.join("saved");
        a.viewer_save_to(&out, d);
        let file = folder.join("saved.exr");
        assert!(file.exists(), "{}", a.status);
        assert_eq!(
            a.render.history.get(id).unwrap().saved.as_deref(),
            Some(file.as_path())
        );
        // Loaded back as a slot that doesn't count against the cap.
        a.render.prefs.max_render_slots = 1;
        a.viewer_load(&file);
        assert_eq!(a.render.history.slots.len(), 2, "{}", a.status);
        let loaded = &a.render.history.slots[0];
        assert_eq!(loaded.kind, image_viewer::SlotKind::Loaded);
        assert_eq!(a.render.viewer.primary, Some(loaded.id));
        let (w, h, px) = a.shown_pixels().unwrap();
        let p = a.render.picture.as_ref().unwrap();
        assert_eq!((w, h), (p.width, p.height));
        assert!(px.iter().all(|v| *v == 0.5));
        // Remove leaves a locked slot and takes an unlocked one.
        let loaded = loaded.id;
        a.toggle_slot_lock(loaded);
        a.remove_slot(loaded);
        assert!(a.render.history.get(loaded).is_some());
        a.toggle_slot_lock(loaded);
        a.remove_slot(loaded);
        assert!(a.render.history.get(loaded).is_none());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn render_camera_matches_the_viewport_projection() {
        let camera = Camera::default();
        let view = render_view(&camera).unwrap();
        // A point the viewport draws at the top edge of the view sits at
        // half_height_tan above the axis at unit depth.
        let r = eframe::egui::Rect::from_min_size(Default::default(), [800., 600.].into());
        let p = view.position + view.forward * 4.0 + view.up * 4.0 * view.half_height_tan;
        let (screen, _) = camera.project(p, r).unwrap();
        assert!((screen.y - r.top()).abs() < 1e-3, "{screen:?}");
        assert!((screen.x - r.center().x).abs() < 1e-3);
    }

    #[test]
    fn non_perspective_views_are_refused() {
        let camera = Camera {
            view: View::Top,
            ..Default::default()
        };
        assert!(render_view(&camera).is_err());
    }

    #[test]
    fn pictures_fit_the_window_without_enlarging() {
        let area = Rect::from_min_size(Pos2::new(0., 40.), Vec2::new(800., 400.));
        // Too big: scaled down to the area's height, centred.
        let shown = fit_picture(area, [1600, 1600]);
        assert_eq!(shown.size(), Vec2::new(400., 400.));
        assert_eq!(shown.center(), area.center());
        // Small: shown 1:1.
        assert_eq!(fit_picture(area, [200, 100]).size(), Vec2::new(200., 100.));
    }

    #[test]
    fn dragging_on_the_picture_makes_a_region_in_picture_pixels() {
        // A 720 × 480 picture shown at half size.
        let shown = Rect::from_min_size(Pos2::new(10., 20.), Vec2::new(360., 240.));
        let size = [720, 480];
        let a = pixel_at(shown, size, Pos2::new(10. + 50., 20. + 30.));
        let b = pixel_at(shown, size, Pos2::new(10. + 150., 20. + 130.));
        assert_eq!((a, b), ([100, 60], [300, 260]));
        // Either drag direction gives the same rectangle, corners included.
        let r = region_between(b, a).unwrap();
        assert_eq!(
            r,
            Region {
                x: 100,
                y: 60,
                width: 201,
                height: 201
            }
        );
        assert_eq!(region_between(a, b), Some(r));
        // Past the edge clamps to the picture.
        assert_eq!(pixel_at(shown, size, Pos2::new(900., -5.)), [719, 0]);
        // A click (or a sliver) is not a region.
        assert_eq!(region_between(a, [a[0] + 2, a[1] + 40]), None);
        // And back to window points for the outline.
        let rect = region_rect(shown, size, r);
        assert!((rect.min.x - 60.).abs() < 1e-3 && (rect.min.y - 50.).abs() < 1e-3);
        assert!((rect.width() - 100.5).abs() < 1e-3);
    }
}

#[cfg(test)]
mod output_file_tests {
    use super::*;
    use operandi_core::render_outputs::{OutputSettings, RenderOutput};
    use operandi_core::shader_tree::OutputEffect;

    fn output(filename: &str, format: &str) -> RenderOutput {
        RenderOutput {
            id: 9,
            name: "Depth Output".into(),
            effect: OutputEffect::Depth,
            settings: OutputSettings {
                filename: filename.into(),
                format: format.into(),
                ..Default::default()
            },
            in_group: false,
        }
    }

    /// RO-Q04 v2: empty settings keep "Render N (Name)" in the render's
    /// format; a Filename is relative to the render's folder; a Format
    /// sets the extension and the encoder.
    #[test]
    fn outputs_save_where_their_settings_say() {
        let render = std::path::Path::new("/r/Render 3.png");
        let (p, f) = output_file(render, &output("", ""));
        assert_eq!(p, PathBuf::from("/r/Render 3 (Depth Output).png"));
        assert!(f.is_none());
        let (p, _) = output_file(render, &output("depth/z", ""));
        assert_eq!(p, PathBuf::from("/r/depth/z.png"));
        let (p, f) = output_file(render, &output("z", "OpenEXR (float)"));
        assert_eq!(p, PathBuf::from("/r/z.exr"));
        assert_eq!(f, Some(crate::image_viewer::SaveFormat::Exr));
        let (p, _) = output_file(render, &output("/abs/z.tif", "TIFF (16-bit)"));
        assert_eq!(p, PathBuf::from("/abs/z.tif"));
    }
}
