//! Preview window state (RP-*): what the progressive Cycles preview renders
//! and how its controls behave, kept free of egui windows so it can be
//! tested on its own. The window itself is in preview_ui.rs.
//!
//! Every pick here is provisional (docs/SPECIFICATION_QUESTIONS.md RP-Q1..).
use eframe::egui::{Rect, Vec2};
use operandi_cycles::LightPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

/// RP-Q3: the sample target a preview refines to.
pub const DEFAULT_SAMPLES: u32 = 256;
/// RP-Q3: adaptive sampling noise threshold.
pub const DEFAULT_THRESHOLD: f32 = 0.05;
/// The coarse first pass: this many times fewer pixels across, one sample.
pub const COARSE_DIVIDER: u32 = 8;
/// Shift+wheel image zoom steps (RP-Q7).
pub const ZOOM_STEPS: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

/// The Status button and status bar text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Paused,
    Rendering,
    Finished,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Paused => "Paused",
            Status::Rendering => "Rendering",
            Status::Finished => "Finished",
        }
    }

    /// What clicking the Status button does in this state.
    pub fn click(self) -> StatusClick {
        match self {
            Status::Rendering => StatusClick::Pause,
            Status::Paused => StatusClick::Resume,
            Status::Finished => StatusClick::Restart,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusClick {
    /// Hold the render; samples are kept.
    Pause,
    /// Carry on from the samples already rendered (or start, if none).
    Resume,
    /// Throw the picture away and start again from coarse.
    Restart,
}

/// The status bar's left text.
pub fn status_text(status: Status, samples_done: u32, samples_total: u32) -> String {
    match status {
        Status::Rendering => format!(
            "Rendering – {} / {} samples",
            samples_done.min(samples_total),
            samples_total
        ),
        other => other.label().to_string(),
    }
}

/// Elapsed render time: counts only while rendering, freezes while paused
/// or finished, starts over on a restart.
#[derive(Clone, Copy, Debug, Default)]
pub struct Elapsed {
    banked: Duration,
    since: Option<Instant>,
}

impl Elapsed {
    pub fn run(&mut self, now: Instant) {
        if self.since.is_none() {
            self.since = Some(now);
        }
    }
    pub fn stop(&mut self, now: Instant) {
        if let Some(since) = self.since.take() {
            self.banked += now.saturating_duration_since(since);
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn running(&self) -> bool {
        self.since.is_some()
    }
    pub fn at(&self, now: Instant) -> Duration {
        self.banked
            + self
                .since
                .map_or(Duration::ZERO, |s| now.saturating_duration_since(s))
    }
}

/// HH:MM:SS for the status bar.
pub fn hms(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// The picture's place in `area`: the largest rectangle of `aspect`
/// (width / height) that fits, centred; the rest is black bars.
pub fn letterbox(area: Rect, aspect: f32) -> Rect {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    let (w, h) = (area.width().max(0.0), area.height().max(0.0));
    let size = if w / h.max(1e-6) > aspect {
        Vec2::new(h * aspect, h)
    } else {
        Vec2::new(w, w / aspect)
    };
    Rect::from_center_size(area.center(), size)
}

/// The next Shift+wheel zoom from `current` (the picture's on-screen scale
/// against its pixels): the nearest step above (in) or below (out).
pub fn zoom_step(current: f32, zoom_in: bool) -> f32 {
    if zoom_in {
        ZOOM_STEPS
            .iter()
            .copied()
            .find(|&z| z > current * 1.001)
            .unwrap_or(ZOOM_STEPS[ZOOM_STEPS.len() - 1])
    } else {
        ZOOM_STEPS
            .iter()
            .rev()
            .copied()
            .find(|&z| z < current * 0.999)
            .unwrap_or(ZOOM_STEPS[0])
    }
}

/// Effect popup (RP-Q5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Shading,
    FinalColour,
    Alpha,
}

impl Effect {
    pub const ALL: [Effect; 3] = [Effect::Shading, Effect::FinalColour, Effect::Alpha];
    pub fn label(self) -> &'static str {
        match self {
            Effect::Shading => "(Shading)",
            Effect::FinalColour => "Final Colour",
            Effect::Alpha => "Alpha",
        }
    }
}

/// LUT popup: how linear pixels become screen (and saved PNG/JPEG) pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lut {
    Srgb,
    Linear,
}

impl Lut {
    pub fn label(self) -> &'static str {
        match self {
            Lut::Srgb => "sRGB",
            Lut::Linear => "Linear",
        }
    }
}

/// Shading popup. Both render the same in v1 (RP-Q5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shading {
    Full,
    Fast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Threads {
    /// Every core but one, so the app stays responsive.
    Multiple,
    All,
}

impl Threads {
    /// Cycles' thread count: 0 means every core.
    pub fn count(self, cores: usize) -> u32 {
        match self {
            Threads::Multiple => cores.saturating_sub(1).max(1) as u32,
            Threads::All => 0,
        }
    }
}

/// The Options menu's ticks (RP-Q6). Not saved: a new Preview starts from
/// these defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub final_quality: bool,
    pub full_resolution: bool,
    pub light_paths: LightPaths,
    pub automatic_updates: bool,
    pub threads: Threads,
    pub only_selected: bool,
    pub only_visible: bool,
    /// What Final Render Quality replaced, put back when it is turned off.
    before_final: Option<(bool, LightPaths)>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            final_quality: false,
            full_resolution: false,
            light_paths: LightPaths::default(),
            automatic_updates: true,
            threads: Threads::Multiple,
            only_selected: false,
            only_visible: true,
            before_final: None,
        }
    }
}

impl Options {
    /// Final Render Quality: on turns on Full Resolution and every light
    /// path (and render-level subdivision); off puts the earlier ticks back.
    pub fn set_final_quality(&mut self, on: bool) {
        if on == self.final_quality {
            return;
        }
        if on {
            self.before_final = Some((self.full_resolution, self.light_paths));
            self.full_resolution = true;
            self.light_paths = LightPaths::default();
        } else if let Some((full, paths)) = self.before_final.take() {
            self.full_resolution = full;
            self.light_paths = paths;
        }
        self.final_quality = on;
    }

    /// Render-level subdivision only at Final Render Quality.
    pub fn render_subdivision(&self) -> bool {
        self.final_quality
    }
}

/// Preview Settings (the cog), saved per user with the window's place.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub version: u8,
    pub samples: u32,
    pub threshold: f32,
    pub denoise: bool,
    pub show_controls: bool,
    pub show_status_bar: bool,
    /// Header rows shown (the small triangle at the top right).
    pub show_header: bool,
    /// Window inner size in points.
    pub size: [f32; 2],
    /// Window outer position in points, once known.
    pub position: Option<[f32; 2]>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            samples: DEFAULT_SAMPLES,
            threshold: DEFAULT_THRESHOLD,
            denoise: true,
            show_controls: true,
            show_status_bar: true,
            show_header: true,
            size: [640., 520.],
            position: None,
        }
    }
}

impl Saved {
    pub fn valid(&self) -> bool {
        self.version == 1
            && (1..=1_000_000).contains(&self.samples)
            && (0.0..=1.0).contains(&self.threshold)
            && self
                .size
                .iter()
                .all(|v| v.is_finite() && (100.0..=20_000.0).contains(v))
            && self
                .position
                .is_none_or(|p| p.iter().all(|v| v.is_finite()))
    }
    pub fn load(path: &Path) -> Self {
        std::fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 4096)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

/// Everything a preview render depends on except the scene itself. A
/// change restarts the preview.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub width: u32,
    pub height: u32,
    pub view: operandi_cycles::View,
    pub samples: u32,
    pub threshold: f32,
    pub denoise: bool,
    /// The Render item's Preview tab Max Render Time, seconds (0 none).
    pub time_limit: f32,
    pub light_paths: LightPaths,
    pub threads: u32,
    pub transparent: bool,
    pub render_subdivision: bool,
    pub only_selected: bool,
    pub only_visible: bool,
    /// Bumped when the scene changes in a way the picture shows.
    pub scene_serial: u64,
}

impl Request {
    /// Cycles settings for the full-size pass.
    pub fn settings(&self, scene: &operandi_core::Scene) -> operandi_cycles::Settings {
        // ENV §6: the scene's environments replace the studio sky.
        let mut s = operandi_cycles::Settings::studio(self.width, self.height, self.view)
            .with_only_scene_lights(scene)
            .with_scene_world(scene);
        s.samples = self.samples.max(1);
        s.denoise = self.denoise;
        s.adaptive_threshold = self.threshold;
        s.light_paths = self.light_paths;
        // The Render item's ray depths, clamp and caustics (RI-04); the
        // Preview keeps its own samples and noise threshold (RP spec).
        s.rays = operandi_cycles::Rays::from_render(&scene.render);
        s.time_limit = self.time_limit;
        s.threads = self.threads;
        s.transparent = self.transparent;
        s
    }

    /// The coarse first pass: big pixels, one sample. It denoises when the
    /// full pass does: the kept Cycles session starts afresh when Denoise
    /// changes between renders (operandi_cycles::Session), which would
    /// throw away the scene the coarse pass just loaded.
    pub fn coarse(&self, scene: &operandi_core::Scene) -> operandi_cycles::Settings {
        let mut s = self.settings(scene);
        s.width = (self.width / COARSE_DIVIDER).max(8).min(self.width.max(1));
        s.height = (self.height / COARSE_DIVIDER)
            .max(8)
            .min(self.height.max(1));
        s.samples = 1;
        s.adaptive_threshold = 0.0;
        s
    }
}

/// The scene the preview renders: the Filter options applied. Lights and
/// cameras always stay.
pub fn filtered_scene(scene: &operandi_core::Scene, options: &Options) -> operandi_core::Scene {
    let mut out = scene.clone();
    if !options.only_visible {
        for item in &mut out.items {
            item.visible = true;
        }
    }
    if options.only_selected {
        let selected: Vec<u64> = scene
            .selection
            .item
            .into_iter()
            .chain(scene.selection.also.iter().copied())
            .collect();
        for item in &mut out.items {
            if item.is_mesh() && !selected.contains(&item.id) {
                item.visible = false;
            }
        }
    }
    out
}

/// True when two scenes would render differently: everything but the
/// selection, the work plane and the import record.
pub fn renders_differently(a: &operandi_core::Scene, b: &operandi_core::Scene) -> bool {
    if a.items.len() != b.items.len()
        || a.smoothing != b.smoothing
        || a.materials != b.materials
        || a.render_camera != b.render_camera
        || a.render != b.render
        || a.shader_tree != b.shader_tree
        || a.images != b.images
    {
        return true;
    }
    a.items.iter().zip(&b.items).any(|(x, y)| {
        x.id != y.id
            || x.visible != y.visible
            || x.hides_children != y.hides_children
            || x.parent != y.parent
            || x.transform != y.transform
            || x.subdivision != y.subdivision
            || x.camera != y.camera
            || x.light != y.light
            || x.environment != y.environment
            || !(operandi_core::Shared::same(x.mesh.shared(), y.mesh.shared())
                || *x.mesh == *y.mesh)
    })
}

/// Linear RGBA to 8-bit RGBA for the screen and saved PNG/JPEG pictures,
/// through the effect and the LUT.
pub fn display_pixels(rgba: &[f32], effect: Effect, lut: Lut) -> Vec<u8> {
    let encode = |v: f32| -> u8 {
        let v = v.clamp(0.0, 1.0);
        let s = match lut {
            Lut::Srgb if v <= 0.003_130_8 => v * 12.92,
            Lut::Srgb => 1.055 * v.powf(1.0 / 2.4) - 0.055,
            Lut::Linear => v,
        };
        (s * 255.0 + 0.5) as u8
    };
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| match effect {
            Effect::Alpha => {
                let a = encode(p[3]);
                [a, a, a, 255]
            }
            Effect::Shading | Effect::FinalColour => {
                [encode(p[0]), encode(p[1]), encode(p[2]), 255]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Pos2;

    #[test]
    fn status_button_pauses_resumes_and_restarts() {
        assert_eq!(Status::Rendering.click(), StatusClick::Pause);
        assert_eq!(Status::Paused.click(), StatusClick::Resume);
        assert_eq!(Status::Finished.click(), StatusClick::Restart);
        assert_eq!(status_text(Status::Paused, 0, 256), "Paused");
        assert_eq!(
            status_text(Status::Rendering, 64, 256),
            "Rendering – 64 / 256 samples"
        );
        assert_eq!(status_text(Status::Finished, 256, 256), "Finished");
    }

    #[test]
    fn option_defaults_match_the_menu_ticks() {
        let o = Options::default();
        assert!(!o.final_quality && !o.full_resolution);
        assert_eq!(o.light_paths, LightPaths::default());
        assert!(o.automatic_updates && o.only_visible && !o.only_selected);
        assert_eq!(o.threads, Threads::Multiple);
        let s = Saved::default();
        assert_eq!((s.samples, s.threshold, s.denoise), (256, 0.05, true));
        assert!(s.show_controls && s.show_status_bar && s.show_header && s.valid());
    }

    #[test]
    fn final_render_quality_turns_things_on_and_puts_them_back() {
        let mut o = Options::default();
        o.light_paths.glossy = false;
        o.set_final_quality(true);
        assert!(o.full_resolution && o.render_subdivision());
        assert_eq!(o.light_paths, LightPaths::default());
        o.set_final_quality(false);
        assert!(!o.full_resolution && !o.render_subdivision());
        assert!(!o.light_paths.glossy && o.light_paths.diffuse);
    }

    #[test]
    fn light_path_ticks_reach_the_render_settings() {
        let o = Options {
            light_paths: LightPaths {
                shadows: false,
                diffuse: false,
                glossy: true,
                transmission: false,
            },
            ..Default::default()
        };
        let request = Request {
            width: 64,
            height: 32,
            view: operandi_cycles::View::perspective(
                glam::DVec3::Z * 5.,
                glam::DVec3::X,
                glam::DVec3::Y,
                -glam::DVec3::Z,
                0.5,
            ),
            samples: 256,
            time_limit: 0.0,
            threshold: 0.05,
            denoise: true,
            light_paths: o.light_paths,
            threads: Threads::Multiple.count(8),
            transparent: false,
            render_subdivision: false,
            only_selected: false,
            only_visible: true,
            scene_serial: 0,
        };
        let scene = operandi_core::Scene::default();
        let s = request.settings(&scene);
        assert!(!s.light_paths.shadows);
        assert_eq!(s.light_paths.bounce_limits([7, 7, 7]), [0, 7, 0]);
        assert_eq!((s.samples, s.threads, s.adaptive_threshold), (256, 7, 0.05));
        let c = request.coarse(&scene);
        assert_eq!((c.width, c.height, c.samples, c.denoise), (8, 8, 1, true));
        assert_eq!(c.adaptive_threshold, 0.0);
        assert_eq!(Threads::All.count(8), 0);
        assert_eq!(Threads::Multiple.count(1), 1);
    }

    #[test]
    fn elapsed_counts_only_while_rendering() {
        let t0 = Instant::now();
        let s = Duration::from_secs;
        let mut e = Elapsed::default();
        e.run(t0);
        assert_eq!(e.at(t0 + s(5)), s(5));
        e.stop(t0 + s(5));
        assert_eq!(e.at(t0 + s(60)), s(5), "frozen while paused");
        e.run(t0 + s(60));
        e.run(t0 + s(61)); // running already: no change
        assert_eq!(e.at(t0 + s(70)), s(15));
        e.reset();
        assert_eq!(e.at(t0 + s(80)), Duration::ZERO);
        assert_eq!(hms(s(3725)), "01:02:05");
    }

    #[test]
    fn the_picture_is_letterboxed_in_its_aspect() {
        let area = Rect::from_min_size(Pos2::new(0., 20.), Vec2::new(600., 600.));
        // Wide picture in a square window: width fills, bars above and below.
        let r = letterbox(area, 1.5);
        assert_eq!(r.size(), Vec2::new(600., 400.));
        assert_eq!(r.center(), area.center());
        // Window wider than the picture: height fills, bars at the sides.
        let wide = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 400.));
        assert_eq!(letterbox(wide, 1.5).size(), Vec2::new(600., 400.));
        assert_eq!(letterbox(wide, f32::NAN).size(), Vec2::new(400., 400.));
    }

    #[test]
    fn shift_wheel_zoom_snaps_to_steps() {
        assert_eq!(zoom_step(0.6, true), 1.0);
        assert_eq!(zoom_step(1.0, true), 2.0);
        assert_eq!(zoom_step(4.0, true), 4.0);
        assert_eq!(zoom_step(0.6, false), 0.5);
        assert_eq!(zoom_step(0.25, false), 0.25);
    }

    #[test]
    fn effects_and_luts_change_the_screen_pixels() {
        let px = [0.5, 0.25, 0.0, 0.5];
        assert_eq!(
            display_pixels(&px, Effect::Shading, Lut::Linear),
            [128, 64, 0, 255]
        );
        let srgb = display_pixels(&px, Effect::FinalColour, Lut::Srgb);
        assert_eq!(srgb, display_pixels(&px, Effect::Shading, Lut::Srgb));
        assert!(srgb[0] > 128);
        assert_eq!(
            display_pixels(&px, Effect::Alpha, Lut::Linear),
            [128, 128, 128, 255]
        );
    }

    #[test]
    fn filters_hide_unselected_or_show_hidden_items() {
        let mut scene = operandi_core::Scene::default();
        scene.new_mesh().unwrap();
        scene.new_mesh().unwrap();
        let ids: Vec<u64> = scene.items.iter().map(|i| i.id).collect();
        scene.items[1].visible = false;
        scene.selection.item = Some(ids[1]);
        let o = Options::default();
        assert!(!filtered_scene(&scene, &o).items[1].visible);
        let all = Options {
            only_visible: false,
            ..o
        };
        assert!(filtered_scene(&scene, &all).items[1].visible);
        let selected = Options {
            only_visible: false,
            only_selected: true,
            ..o
        };
        let f = filtered_scene(&scene, &selected);
        assert!(!f.items[0].visible && f.items[1].visible);
    }

    #[test]
    fn selection_changes_do_not_restart_but_edits_do() {
        let mut a = operandi_core::Scene::default();
        a.new_mesh().unwrap();
        let mut b = a.clone();
        b.selection.vertices.push(0);
        assert!(!renders_differently(&a, &b));
        b.items[0].transform.translation.x += 1.0;
        assert!(renders_differently(&a, &b));
        a.items[0].visible = false;
        assert!(renders_differently(&a, &a.clone().tap_visible()));
    }

    trait TapVisible {
        fn tap_visible(self) -> Self;
    }
    impl TapVisible for operandi_core::Scene {
        fn tap_visible(mut self) -> Self {
            self.items[0].visible = !self.items[0].visible;
            self
        }
    }

    #[test]
    fn saved_settings_round_trip_and_reject_nonsense() {
        let dir = std::env::temp_dir().join(format!("operandi-preview-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("preview.json");
        let s = Saved {
            samples: 64,
            position: Some([10., 20.]),
            ..Default::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Saved::load(&path), s);
        std::fs::write(&path, br#"{"version":1,"samples":0}"#).unwrap();
        assert_eq!(Saved::load(&path), Saved::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
