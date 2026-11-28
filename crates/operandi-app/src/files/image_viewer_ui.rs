//! The Render window grown into the Image Viewer, stage 1 (IV-*): its own
//! OS window (never a tab of the main one; see native::install) with
//!
//! - the render controls (Render, Render Region, Pause/Resume, Finish,
//!   Cancel) and the header (IV-05): Output, Pass and Stereo (greyed until
//!   later stages), Display transform, Zoom, then the Options (IV-35,
//!   IV-35a): Display Channels and Show Clipping;
//! - the history strip (IV-08..13b) on the left with Save Image... and
//!   Load Image... above it (IV-36, IV-38);
//! - the side panel's tabs on the right: Image Processing (Bloom and Glare,
//!   post_effects_ui.rs) and Statistics (IV-35b); Compare comes later;
//! - the picture (IV-06): wheel and Cmd+Alt-drag zoom, middle-drag pan, a
//!   plain drag that draws the render region (or pans when Region is off),
//!   and the pixel readout under it (IV-07).
//!
//! Keys while the window has the focus: 1–9 and 0 pick a slot, Space hides
//! everything but the picture (IV-Q2), and the editable Image Viewer
//! shortcuts in the input map (T, P, I, C, S, Cmd+A, Cmd+S by default).
use crate::Operandi;
use crate::image_viewer::{
    self, Channels, FINAL_COLOR, SaveFormat, Slot, SlotKind, Tab, ViewTransform, WindowSaved,
};
use crate::theme::{self, ColorId};
use crate::unavailable::Gate;
use eframe::egui::{
    self, Color32, Event, Key, PointerButton, Pos2, Rect, Sense, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Image Viewer actions in the input map. They work only while the Render
/// window has the focus, so they may share keys with the main window's
/// actions (`input::contexts_overlap`).
pub const PREFIX: &str = "Image Viewer: ";
pub const STRIP: &str = "Image Viewer: History Strip";
pub const PAUSE: &str = "Image Viewer: Pause/Resume";
pub const TAB_PROCESSING: &str = "Image Viewer: Image Processing Tab";
pub const TAB_COMPARE: &str = "Image Viewer: Compare Tab";
pub const TAB_STATISTICS: &str = "Image Viewer: Statistics Tab";
pub const FIT: &str = "Image Viewer: Fit Image";
pub const SAVE: &str = "Image Viewer: Save Image";
/// Render > Open Render Window (IV-02), global.
pub const OPEN: &str = "Open Render Window";

pub fn is_viewer_action(action: &str) -> bool {
    action.starts_with(PREFIX)
}

/// Zoom limits, as screen pixels per picture pixel.
const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.01..=32.0;

/// The pixels of a slot that isn't the picture in `RenderState::picture`,
/// read back from the cache: output 0 is Final Color.
pub(crate) struct Loaded {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub outputs: Vec<(String, Vec<f32>)>,
}

/// A picture drag in progress.
#[derive(Clone, Copy)]
enum Drag {
    Pan,
    /// Cmd+Alt-drag: zoom about `anchor` from `zoom` and `pan` at the press.
    Zoom {
        anchor: Pos2,
        zoom: f32,
        pan: Vec2,
    },
}

/// Save Image... (IV-36): which slot and output, in what format.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SaveDialog {
    /// `None`: the picture being rendered or last rendered.
    pub slot: Option<u64>,
    /// `None`: Final Color, else output `k` after it.
    pub output: Option<usize>,
    pub format: SaveFormat,
}

pub(crate) struct ViewerState {
    pub saved: WindowSaved,
    saved_changed: bool,
    /// Ctrl/Cmd+click multi-selection (UIC-32); `primary` is shown.
    pub selected: Vec<u64>,
    /// The slot shown; `None` shows the picture being rendered (or the last
    /// one rendered).
    pub primary: Option<u64>,
    /// `None` fits the picture to the window.
    pub zoom: Option<f32>,
    pub pan: Vec2,
    /// IV-14; starts at Preferences' Default View Colourspace.
    pub transform: Option<ViewTransform>,
    /// Space: everything but the picture hidden.
    pub hide_all: bool,
    thumbs: HashMap<u64, Option<TextureHandle>>,
    pub loaded: Option<Loaded>,
    /// The picture pixel under the pointer and its linear RGBA (IV-07).
    pub hover: Option<([u32; 2], [f32; 4])>,
    pub save_dialog: Option<SaveDialog>,
    /// Statistics sections open: Render, Picture, Device.
    pub sections: [bool; 3],
    drag: Option<Drag>,
    texture_options: TextureOptions,
    /// A slot whose cache file couldn't be read: not tried every frame.
    load_failed: Option<u64>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            saved: WindowSaved::default(),
            saved_changed: false,
            selected: Vec::new(),
            primary: None,
            zoom: None,
            pan: Vec2::ZERO,
            transform: None,
            hide_all: false,
            thumbs: HashMap::new(),
            loaded: None,
            hover: None,
            save_dialog: None,
            sections: [true; 3],
            drag: None,
            texture_options: TextureOptions::LINEAR,
            load_failed: None,
        }
    }
}

impl ViewerState {
    /// Show the render in progress.
    pub fn show_live(&mut self) {
        self.primary = None;
        self.selected.clear();
        self.loaded = None;
    }
    pub fn select_only(&mut self, id: u64) {
        self.selected = vec![id];
        self.primary = Some(id);
    }
    /// Ctrl/Cmd+click: add or take out `id`, showing the last one picked.
    pub fn toggle_selected(&mut self, id: u64) {
        if let Some(i) = self.selected.iter().position(|s| *s == id) {
            self.selected.remove(i);
            if self.primary == Some(id) {
                self.primary = self.selected.last().copied().or(Some(id));
                if self.selected.is_empty() {
                    self.selected.push(id);
                }
            }
        } else {
            self.selected.push(id);
            self.primary = Some(id);
        }
    }
    /// A slot went away.
    pub fn forget(&mut self, id: u64) {
        self.thumbs.remove(&id);
        self.selected.retain(|s| *s != id);
        if self.primary == Some(id) {
            self.primary = self.selected.last().copied();
        }
        if self.loaded.as_ref().is_some_and(|l| l.id == id) {
            self.loaded = None;
        }
    }
    pub fn view(&self) -> ViewTransform {
        self.transform.unwrap_or_default()
    }
    fn mark(&mut self) {
        self.saved_changed = true;
    }
    pub fn fit(&mut self) {
        self.zoom = None;
        self.pan = Vec2::ZERO;
    }
}

/// The screen pixels per picture pixel that fit `size` into `area` (never
/// enlarged past 1:1), with `ppp` screen pixels per point.
pub fn fit_zoom(area: Rect, size: [u32; 2], ppp: f32) -> f32 {
    let (w, h) = (size[0].max(1) as f32, size[1].max(1) as f32);
    (area.width() * ppp / w)
        .min(area.height() * ppp / h)
        .min(1.0)
}

/// Where the picture sits: centred in `area`, moved by `pan` points.
pub fn picture_rect(area: Rect, size: [u32; 2], zoom: f32, ppp: f32, pan: Vec2) -> Rect {
    let points = Vec2::new(size[0] as f32, size[1] as f32) * zoom / ppp.max(0.01);
    Rect::from_center_size(area.center() + pan, points)
}

/// The pan that keeps the picture point under `at` still while the zoom
/// goes from `from` to `to`.
pub fn zoom_about(area: Rect, pan: Vec2, at: Pos2, from: f32, to: f32) -> Vec2 {
    let offset = at - (area.center() + pan);
    at - area.center() - offset * (to / from.max(1e-6))
}

/// The Show in Finder entry's name on this platform.
pub fn reveal_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Show in Finder"
    } else if cfg!(target_os = "windows") {
        "Show in Explorer"
    } else {
        "Show in File Manager"
    }
}

fn reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();
}

/// A key the strip's slot numbers answer to: 1–9, and 0 for the tenth.
/// The main row and the keypad both give these keys.
pub fn slot_key(key: Key) -> Option<usize> {
    let n = match key {
        Key::Num1 => 1,
        Key::Num2 => 2,
        Key::Num3 => 3,
        Key::Num4 => 4,
        Key::Num5 => 5,
        Key::Num6 => 6,
        Key::Num7 => 7,
        Key::Num8 => 8,
        Key::Num9 => 9,
        Key::Num0 => 10,
        _ => return None,
    };
    Some(n)
}

/// What the viewer's keys ask for, read from one frame's events.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ViewerKey {
    Slot(usize),
    HideAll,
    Action(&'static str),
}

/// The keys pressed this frame, matched against `bindings`.
pub(crate) fn viewer_keys(
    events: &[Event],
    bindings: &std::collections::BTreeMap<String, String>,
) -> Vec<ViewerKey> {
    const ACTIONS: [&str; 9] = [
        STRIP,
        PAUSE,
        TAB_PROCESSING,
        TAB_COMPARE,
        TAB_STATISTICS,
        FIT,
        SAVE,
        OPEN,
        "Render",
    ];
    let mut keys = Vec::new();
    for event in events {
        let Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        if modifiers.is_none() {
            if let Some(n) = slot_key(*key) {
                keys.push(ViewerKey::Slot(n));
                continue;
            }
            if *key == Key::Space {
                keys.push(ViewerKey::HideAll);
                continue;
            }
        }
        if let Some(action) = ACTIONS.into_iter().find(|a| {
            bindings
                .get(*a)
                .is_some_and(|b| crate::input::matches_chord(b, *key, *modifiers))
        }) {
            keys.push(ViewerKey::Action(action));
        }
    }
    keys
}

impl Operandi {
    /// " (T)" for a tooltip from the editable input map (UIC-04), or
    /// nothing when the action is unbound.
    fn viewer_hint(&self, action: &str) -> String {
        self.bindings
            .get(action)
            .filter(|b| !b.is_empty())
            .map(|b| format!(" ({})", crate::input::display_chord(b)))
            .unwrap_or_default()
    }

    /// Points the history at the Recent Frames Folder and loads it, with
    /// the window's saved place (called at startup and when the folder
    /// preference changes).
    pub(crate) fn load_render_history(&mut self) {
        let paths = self.environment.paths();
        let folder = self.render.prefs.folder(&paths.recent_frames());
        self.render.history = image_viewer::History::load(folder);
        self.render.viewer.thumbs.clear();
        self.render.viewer.loaded = None;
        self.render.viewer.selected.clear();
        self.render.viewer.primary = None;
        // Max Render Slots may have dropped since the last run.
        let removed = self.render.history.trim(self.render.prefs.max_render_slots);
        if !removed.is_empty() {
            let _ = self.render.history.save_index();
        }
    }

    pub(crate) fn save_render_window(&mut self) {
        self.render.viewer.saved_changed = false;
        if let Err(e) = self
            .render
            .viewer
            .saved
            .save(&self.environment.paths().render_window())
        {
            self.status = format!("Render window layout changed; could not save it: {e}");
        }
    }

    /// Render > Open Render Window (Alt+Shift+F9; IV-02): the window
    /// without rendering, showing the newest slot.
    pub(crate) fn open_render_window(&mut self) {
        self.render.open = true;
        self.render.focus = true;
        let v = &self.render.viewer;
        let nothing_shown = v.primary.is_none() && self.render.picture.is_none();
        if !self.render.busy()
            && nothing_shown
            && let Some(id) = self.render.history.slots.first().map(|s| s.id)
        {
            self.render.viewer.select_only(id);
            self.render.texture_stale = true;
        }
    }

    /// Whether `primary` is the picture in `RenderState::picture`.
    fn shows_picture(&self) -> bool {
        let v = &self.render.viewer;
        match (v.primary, &self.render.picture) {
            (None, _) => true,
            (Some(id), Some(p)) => p.slot == Some(id),
            (Some(_), None) => false,
        }
    }

    /// Read the shown slot's outputs from the cache when they aren't in
    /// memory yet.
    fn ensure_loaded(&mut self) {
        let Some(id) = self.render.viewer.primary else {
            return;
        };
        if self.shows_picture()
            || self
                .render
                .viewer
                .loaded
                .as_ref()
                .is_some_and(|l| l.id == id)
            || self.render.viewer.load_failed == Some(id)
        {
            return;
        }
        let Some(slot) = self.render.history.get(id).cloned() else {
            self.render.viewer.primary = None;
            return;
        };
        let folder = self.render.history.folder.clone();
        let mut outputs = Vec::new();
        for (k, name) in slot.outputs.iter().enumerate() {
            match image_viewer::read_output(&folder, &slot, k) {
                Ok(px) => outputs.push((name.clone(), px)),
                Err(e) if k == 0 => {
                    self.status = format!("Can't show {}: {e}", slot.base_name());
                    self.render.viewer.loaded = None;
                    self.render.viewer.load_failed = Some(id);
                    return;
                }
                // A missing extra output: the rest still show.
                Err(_) => {}
            }
        }
        self.render.viewer.loaded = Some(Loaded {
            id,
            width: slot.width,
            height: slot.height,
            outputs,
        });
        if self.render.shown.is_some_and(|k| {
            k + 1
                >= self
                    .render
                    .viewer
                    .loaded
                    .as_ref()
                    .map_or(0, |l| l.outputs.len())
        }) {
            self.render.shown = None;
        }
        self.render.texture_stale = true;
    }

    /// The other outputs' names (after Final Color) of what is shown.
    pub(crate) fn shown_output_names(&self) -> Vec<String> {
        if self.shows_picture() {
            self.render.outputs.iter().map(|(n, _)| n.clone()).collect()
        } else {
            self.render
                .viewer
                .loaded
                .as_ref()
                .map_or_else(Vec::new, |l| {
                    l.outputs.iter().skip(1).map(|(n, _)| n.clone()).collect()
                })
        }
    }

    /// The shown picture: size and the chosen output's linear RGBA.
    /// The shown output's kind and colour space (group C).
    pub(crate) fn shown_meta(&self) -> image_viewer::OutputMeta {
        let k = self.render.shown.map_or(0, |k| k + 1);
        if self.shows_picture() {
            return self.render.meta.get(k).copied().unwrap_or_default();
        }
        self.render
            .viewer
            .primary
            .and_then(|id| self.render.history.get(id))
            .map(|slot| slot.meta(k))
            .unwrap_or_default()
    }

    /// What the window shows: with Bloom and Glare for Final Color when
    /// they are on (post_effects_ui.rs).
    pub(crate) fn shown_pixels(&self) -> Option<(u32, u32, &[f32])> {
        let (w, h, raw) = self.shown_raw_pixels()?;
        let post = self.post_pixels().filter(|p| p.len() == raw.len());
        Some((w, h, post.unwrap_or(raw)))
    }

    /// The shown output as rendered, without Bloom and Glare.
    pub(crate) fn shown_raw_pixels(&self) -> Option<(u32, u32, &[f32])> {
        let k = self.render.shown;
        if self.shows_picture() {
            let p = self.render.picture.as_ref()?;
            let rgba = k
                .and_then(|k| self.render.outputs.get(k))
                .map(|(_, px)| px.as_slice())
                .filter(|px| px.len() == p.rgba.len())
                .unwrap_or(&p.rgba);
            Some((p.width, p.height, rgba))
        } else {
            let l = self.render.viewer.loaded.as_ref()?;
            let rgba = l
                .outputs
                .get(k.map_or(0, |k| k + 1))
                .or(l.outputs.first())?;
            Some((l.width, l.height, &rgba.1))
        }
    }

    /// Select slot `id` from a click; `toggle` is Ctrl/Cmd (UIC-32).
    pub(crate) fn click_slot(&mut self, id: u64, toggle: bool) {
        // A click tries a slot that failed to load again.
        self.render.viewer.load_failed = None;
        if toggle {
            self.render.viewer.toggle_selected(id);
        } else {
            self.render.viewer.select_only(id);
        }
        self.render.texture_stale = true;
        self.ensure_loaded();
    }

    /// The x, or Remove: no prompt, no undo (IV-11). Locked slots stay.
    pub(crate) fn remove_slot(&mut self, id: u64) {
        if self.render.history.remove(id) {
            self.render.viewer.forget(id);
            if self
                .render
                .picture
                .as_ref()
                .is_some_and(|p| p.slot == Some(id))
            {
                // The picture stays on screen until something else shows,
                // but it is no longer a slot.
                if let Some(p) = &mut self.render.picture {
                    p.slot = None;
                }
                if self.render.viewer.primary.is_none() && !self.render.busy() {
                    self.render.picture = None;
                    self.render.outputs.clear();
                    self.render.meta.clear();
                    self.render.shown = None;
                }
            }
            self.render.texture_stale = true;
            if let Err(e) = self.render.history.save_index() {
                self.status = format!("Render history changed; could not save it: {e}");
            }
        } else if self.render.history.get(id).is_some_and(|s| s.locked) {
            self.status = "That slot is locked: unlock it to remove it".into();
        }
    }

    pub(crate) fn toggle_slot_lock(&mut self, id: u64) {
        if let Some(s) = self.render.history.get_mut(id) {
            s.locked = !s.locked;
            if let Err(e) = self.render.history.save_index() {
                self.status = format!("Render history changed; could not save it: {e}");
            }
        }
    }

    /// Save Image... for the shown slot (or `slot`).
    pub(crate) fn open_save_dialog(&mut self, slot: Option<u64>) {
        if self.shown_pixels().is_none() && slot.is_none() {
            self.status = "Nothing to save: render or pick a slot first".into();
            return;
        }
        let slot = slot.or(self.render.viewer.primary);
        let output = if slot == self.render.viewer.primary {
            self.render.shown
        } else {
            None
        };
        self.render.viewer.save_dialog = Some(SaveDialog {
            slot,
            output,
            format: self.render.viewer.saved.save_format,
        });
    }

    /// The Save Image dialog's Save...: ask where.
    fn ask_save_path(&mut self, d: SaveDialog) {
        let name = d
            .slot
            .and_then(|id| self.render.history.get(id))
            .map_or_else(|| "Render".to_string(), Slot::base_name);
        // The project's renders folder; Recent Frames stay in app data.
        let folder = self
            .project_folder(crate::project_files::RENDERS)
            .or_else(|| crate::render_ui::render_folder().ok())
            .filter(|f| f.is_dir())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_default();
        let request = crate::file_chooser::Choose::ViewerSave {
            folder,
            name: format!("{name}.{}", d.format.extension()),
            format: d.format,
        };
        if !self.choose(request, crate::file_chooser::Purpose::ViewerSave(d)) {
            self.status = "No file panel to choose where to save".into();
        }
    }

    /// Write slot `d.slot`'s output to `path` (IV-36).
    pub(crate) fn viewer_save_to(&mut self, path: &Path, d: SaveDialog) {
        let path = image_viewer::with_extension(path, d.format);
        if d.format == SaveFormat::ExrMultilayer {
            let result = self.save_multilayer(&path, d.slot);
            self.note_saved(&path, d.slot, result);
            return;
        }
        let live = !d.slot.is_some_and(|id| {
            !self
                .render
                .picture
                .as_ref()
                .is_some_and(|p| p.slot == Some(id))
        });
        let k = d.output.map_or(0, |k| k + 1);
        let meta = if live {
            self.render.meta.get(k).copied().unwrap_or_default()
        } else {
            d.slot
                .and_then(|id| self.render.history.get(id))
                .map(|slot| slot.meta(k))
                .unwrap_or_default()
        };
        let pixels: Result<(u32, u32, Vec<f32>), String> = match d.slot {
            Some(id)
                if !self
                    .render
                    .picture
                    .as_ref()
                    .is_some_and(|p| p.slot == Some(id)) =>
            {
                match self.render.history.get(id) {
                    Some(slot) => image_viewer::read_output(
                        &self.render.history.folder,
                        slot,
                        d.output.map_or(0, |k| k + 1),
                    )
                    .map(|px| (slot.width, slot.height, px)),
                    None => Err("That slot is gone".into()),
                }
            }
            _ => match &self.render.picture {
                Some(p) => {
                    let rgba = d
                        .output
                        .and_then(|k| self.render.outputs.get(k))
                        .map_or(&p.rgba, |(_, px)| px);
                    Ok((p.width, p.height, rgba.clone()))
                }
                None => Err("Nothing to save".into()),
            },
        };
        let post = &self.editor.scene.render.post;
        let result = pixels.and_then(|(w, h, mut px)| {
            // Final Color in a picture format (PNG, JPEG, TIFF...) saves as
            // the window shows it, Bloom and Glare included. Float files
            // (EXR, HDR) and the other outputs keep the rendered numbers
            // (Jason, BG-Q4).
            if d.output.is_none() && !d.format.is_float() {
                operandi_core::post_effects::apply(post, &mut px, w, h);
            }
            image_viewer::save_output(
                &path,
                &px,
                w,
                h,
                d.format,
                meta,
                self.render.viewer.view(),
                self.file_io_preferences.image(),
            )
        });
        self.note_saved(&path, d.slot, result);
    }

    /// OpenEXR Multilayer (render outputs, RO-Q17): every output of the
    /// slot (or the picture on screen) as one file, the Cryptomatte layers
    /// and their metadata included. Each output's numbers follow its
    /// colour space for a float file.
    fn save_multilayer(&self, path: &Path, slot: Option<u64>) -> Result<(), String> {
        let folder = &self.render.history.folder;
        let live = self
            .render
            .picture
            .as_ref()
            .filter(|p| slot.is_none() || p.slot == slot);
        let (width, height, names, pixels, meta, crypto_slot) = match live {
            Some(p) => {
                let mut names = vec![image_viewer::FINAL_COLOR.to_string()];
                let mut pixels = vec![p.rgba.clone()];
                for (n, px) in &self.render.outputs {
                    names.push(n.clone());
                    pixels.push(px.clone());
                }
                let meta = self.render.meta.clone();
                (p.width, p.height, names, pixels, meta, p.slot)
            }
            None => {
                let s = slot
                    .and_then(|id| self.render.history.get(id))
                    .ok_or("That slot is gone")?;
                let mut pixels = Vec::new();
                for k in 0..s.outputs.len() {
                    pixels.push(image_viewer::read_output(folder, s, k)?);
                }
                let mut names = s.outputs.clone();
                if let Some(first) = names.first_mut() {
                    *first = image_viewer::FINAL_COLOR.to_string();
                }
                (s.width, s.height, names, pixels, s.meta.clone(), Some(s.id))
            }
        };
        // Every layer as rendered, Bloom and Glare left out (BG-Q4).
        let window = self.render.viewer.view();
        let encoded: Vec<Vec<f32>> = pixels
            .into_iter()
            .enumerate()
            .map(|(k, px)| {
                let m = meta.get(k).copied().unwrap_or_default();
                if m.effect.is_some() && m.writes_srgb(SaveFormat::ExrMultilayer, window) {
                    image_viewer::srgb_floats(&px)
                } else {
                    px
                }
            })
            .collect();
        let layers: Vec<crate::exr_multilayer::OutputLayer> = names
            .iter()
            .zip(&encoded)
            .enumerate()
            .map(|(k, (name, rgba))| crate::exr_multilayer::OutputLayer {
                // The picture is the main R, G, B, A.
                name: if k == 0 { "" } else { name.as_str() },
                rgba,
                heat_map: meta.get(k).and_then(|m| m.effect)
                    == Some(operandi_core::shader_tree::OutputEffect::ShadedAaSamples),
            })
            .collect();
        let crypto = crypto_slot
            .and_then(|id| self.render.history.get(id))
            .map(|s| s.crypto_file(folder))
            .filter(|f| f.is_file())
            .map(|f| crate::exr_multilayer::read_crypto(&f))
            .transpose()?
            .unwrap_or_default();
        crate::exr_multilayer::write(path, width, height, &layers, &crypto)
    }

    /// The status line (and the slot's Saved path) after a Save Image.
    fn note_saved(&mut self, path: &Path, slot: Option<u64>, result: Result<(), String>) {
        self.status = match result {
            Ok(()) => {
                if let Some(s) = slot.and_then(|id| self.render.history.get_mut(id)) {
                    s.saved = Some(path.to_path_buf());
                    let _ = self.render.history.save_index();
                }
                format!("Saved {}", path.display())
            }
            Err(e) => e,
        };
    }

    /// Load Image... (IV-38): ask for a file.
    fn ask_load_path(&mut self) {
        let folder = crate::render_ui::render_folder()
            .ok()
            .filter(|f| f.is_dir())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_default();
        if !self.choose(
            crate::file_chooser::Choose::ViewerLoad { folder },
            crate::file_chooser::Purpose::ViewerLoad,
        ) {
            self.status = "No file panel to choose an image".into();
        }
    }

    /// Add the picture at `path` as a slot; it doesn't count against Max
    /// Render Slots.
    pub(crate) fn viewer_load(&mut self, path: &Path) {
        let (w, h, rgba) = match image_viewer::load_image(path) {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let id = self.render.history.reserve_id();
        let slot = Slot {
            id,
            kind: SlotKind::Loaded,
            scene: path
                .file_name()
                .map_or("Image".into(), |n| n.to_string_lossy().into_owned()),
            time: image_viewer::now(),
            width: w,
            height: h,
            saved: Some(path.to_path_buf()),
            ..Default::default()
        };
        let folder = self.render.history.folder.clone();
        if let Err(e) =
            image_viewer::write_slot_files(&folder, &slot, &[&rgba], self.render.prefs.default_view)
        {
            self.status = format!("Can't keep {} in the render history: {e}", path.display());
            return;
        }
        let max = self.render.prefs.max_render_slots;
        for gone in self.render.history.add(slot, max) {
            self.render.viewer.forget(gone.id);
        }
        let _ = self.render.history.save_index();
        self.render.viewer.select_only(id);
        self.render.viewer.loaded = Some(Loaded {
            id,
            width: w,
            height: h,
            outputs: vec![(FINAL_COLOR.into(), rgba)],
        });
        self.render.shown = None;
        self.render.viewer.fit();
        self.render.texture_stale = true;
        self.status = format!("Loaded {}", path.display());
    }

    /// Show the picture file at `path` in the Render window (material
    /// editing items 5/6). A slot loaded earlier from the same, unchanged
    /// file is shown again; otherwise the file is loaded as a new Loaded
    /// slot, as Load Image... does (IV-38).
    pub(crate) fn show_image_in_viewer(&mut self, path: &Path) {
        self.render.open = true;
        self.render.focus = true;
        let modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        let earlier = self
            .render
            .history
            .slots
            .iter()
            .find(|s| {
                s.kind == SlotKind::Loaded && s.saved.as_deref() == Some(path) && s.time >= modified
            })
            .map(|s| s.id);
        match earlier {
            Some(id) => self.click_slot(id, false),
            None => self.viewer_load(path),
        }
    }

    /// Run what the viewer's keys asked for.
    pub(crate) fn viewer_key(&mut self, key: ViewerKey) {
        let v = &mut self.render.viewer;
        match key {
            ViewerKey::Slot(n) => {
                if let Some(id) = self.render.history.at_position(n) {
                    self.click_slot(id, false);
                }
            }
            ViewerKey::HideAll => v.hide_all = !v.hide_all,
            ViewerKey::Action(STRIP) => {
                v.saved.show_strip = !v.saved.show_strip;
                v.mark();
            }
            ViewerKey::Action(PAUSE) => self.render.toggle_pause(),
            ViewerKey::Action(a @ (TAB_PROCESSING | TAB_COMPARE | TAB_STATISTICS)) => {
                // UIC-21: tabs come forward only when the user asks.
                v.saved.tab = match a {
                    TAB_PROCESSING => Tab::ImageProcessing,
                    TAB_COMPARE => Tab::Compare,
                    _ => Tab::Statistics,
                };
                v.saved.show_panel = true;
                v.mark();
            }
            ViewerKey::Action(FIT) => v.fit(),
            ViewerKey::Action(SAVE) => self.open_save_dialog(None),
            ViewerKey::Action(OPEN) => self.render.focus = true,
            ViewerKey::Action("Render") => self.render_key(),
            ViewerKey::Action(_) => {}
        }
    }

    /// The Render window (IV-01): its own OS window, floating outside the
    /// main one. Its place and panels persist (UIC-73).
    pub(crate) fn render_window(&mut self, ctx: &egui::Context) {
        if !self.render.open {
            return;
        }
        if self.render.viewer.transform.is_none() {
            self.render.viewer.transform = Some(self.render.prefs.default_view);
        }
        let saved = self.render.viewer.saved.clone();
        let title = format!("Render — {}", self.environment.title());
        let id = egui::ViewportId::from_hash_of("operandi-render-window");
        // Opening size and place only, so a resize isn't fought (floating.rs).
        let (size, position) = crate::floating::opening_place(ctx, id, saved.size, saved.position);
        let mut builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(size)
            .with_min_inner_size([360., 260.])
            // UIC-189: above the main window, not above other apps.
            .with_window_level(self.floating_window_level());
        if let Some(p) = position {
            builder = builder.with_position(p);
        }
        ctx.show_viewport_immediate(id, builder, |ctx, _class| {
            // A field's Tab moves to the next field (UIC-51), never a shortcut.
            crate::numeric_tab::take_tab_in_window(ctx);
            if std::mem::take(&mut self.render.focus) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            self.ensure_loaded();
            self.refresh_post();
            let focused = ctx.input(|i| i.viewport().focused == Some(true));
            if focused && !ctx.wants_keyboard_input() && self.render.viewer.save_dialog.is_none() {
                let keys = ctx.input(|i| viewer_keys(&i.events, &self.bindings));
                for key in keys {
                    self.viewer_key(key);
                }
            }
            self.render_window_panels(ctx);
            self.save_image_dialog(ctx);
            let (inner, outer) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect));
            let v = &mut self.render.viewer;
            if let Some(r) = inner {
                let size = [r.width(), r.height()];
                if size != v.saved.size && size.iter().all(|s| *s >= 100.) {
                    v.saved.size = size;
                    v.mark();
                }
            }
            if let Some(r) = outer {
                let at = Some([r.left(), r.top()]);
                if at != v.saved.position {
                    v.saved.position = at;
                    v.mark();
                }
            }
            // Write the layout once the pointer lets go (not every
            // frame of a resize).
            if v.saved_changed && !ctx.input(|i| i.pointer.any_down()) {
                self.save_render_window();
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.close_render_window();
            }
        });
    }

    fn render_window_panels(&mut self, ctx: &egui::Context) {
        let hide = self.render.viewer.hide_all;
        if !hide {
            egui::TopBottomPanel::top("iv-controls").show(ctx, |ui| {
                self.render_controls(ui);
                ui.add_space(2.);
                self.viewer_header(ui);
            });
            egui::TopBottomPanel::bottom("iv-readout").show(ctx, |ui| self.pixel_readout(ui));
            if self.render.viewer.saved.show_strip {
                let r = egui::SidePanel::left("iv-strip")
                    .resizable(true)
                    .default_width(self.render.viewer.saved.strip_width)
                    .width_range(80.0..=400.0)
                    .show(ctx, |ui| self.history_strip(ui));
                let w = r.response.rect.width().round();
                if (w - self.render.viewer.saved.strip_width).abs() >= 1. {
                    self.render.viewer.saved.strip_width = w;
                    self.render.viewer.mark();
                }
            }
            if self.render.viewer.saved.show_panel {
                let r = egui::SidePanel::right("iv-panel")
                    .resizable(true)
                    .default_width(self.render.viewer.saved.panel_width)
                    .width_range(160.0..=600.0)
                    .show(ctx, |ui| self.viewer_side_panel(ui));
                let w = r.response.rect.width().round();
                if (w - self.render.viewer.saved.panel_width).abs() >= 1. {
                    self.render.viewer.saved.panel_width = w;
                    self.render.viewer.mark();
                }
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::color(ColorId::ImageViewerBackground)))
            .show(ctx, |ui| self.viewer_picture(ui));
    }

    /// Render, Render Region and Region, or Pause/Finish/Cancel while it
    /// renders.
    fn render_controls(&mut self, ui: &mut egui::Ui) {
        let ppp = ui.ctx().pixels_per_point();
        let pause_hint = self.viewer_hint(PAUSE);
        let render_hint = self.viewer_hint("Render");
        ui.horizontal(|ui| {
            if let Some(job) = &self.render.job {
                let c = &job.control;
                let label = if c.pause_requested() {
                    "Resume"
                } else {
                    "Pause"
                };
                let finishing = c.finish_requested();
                if ui
                    .gated(
                        crate::unavailable::unless(!finishing, "The render is finishing"),
                        egui::Button::new(label),
                    )
                    .on_hover_text(format!("Hold the render between samples{pause_hint}"))
                    .clicked()
                {
                    self.render.toggle_pause();
                }
                if ui
                    .gated(
                        crate::unavailable::unless(!finishing, "The render is already finishing"),
                        egui::Button::new("Finish"),
                    )
                    .on_hover_text("Stop adding samples, then denoise and keep the picture")
                    .clicked()
                {
                    c.finish();
                }
                if ui
                    .button("Cancel")
                    .on_hover_text("Stop and keep nothing")
                    .clicked()
                {
                    c.cancel();
                }
                let p = c.progress();
                let state = if c.cancelled() {
                    "Cancelling…"
                } else if finishing {
                    "Finishing…"
                } else if p.paused {
                    "Paused"
                } else if c.pause_requested() {
                    "Pausing…"
                } else if let Some(step) = p.step {
                    // Group D: the helper render after the main one.
                    step
                } else if p.samples_total == 0 {
                    "Preparing…"
                } else {
                    "Rendering"
                };
                let samples = if p.samples_total > 0 {
                    format!(" · sample {} of {}", p.samples_done, p.samples_total)
                } else {
                    String::new()
                };
                ui.label(format!(
                    "{state}{samples} · {:.1} s",
                    job.elapsed().as_secs_f64()
                ));
                if p.samples_total > 0 {
                    ui.add(
                        egui::ProgressBar::new(p.samples_done as f32 / p.samples_total as f32)
                            .desired_width(120.),
                    );
                }
            } else {
                if ui
                    .button("Render")
                    .on_hover_text(format!("Render the scene{render_hint}"))
                    .clicked()
                {
                    self.begin_render(ppp, false);
                }
                let can_region = self.render.region.is_some() && self.shown_pixels().is_some();
                if ui
                    .gated(
                        crate::unavailable::unless(
                            can_region,
                            "Drag on the picture to draw a region first",
                        ),
                        egui::Button::new("Render Region"),
                    )
                    .on_hover_text("Re-render only the rectangle drawn on the picture")
                    .clicked()
                {
                    self.render_region_of_shown(ppp);
                }
                if self.render.region.is_some()
                    && ui
                        .button("Clear Region")
                        .on_hover_text("Forget the rectangle (or click the picture)")
                        .clicked()
                {
                    self.render.region = None;
                }
            }
            let v = &mut self.render.viewer;
            if ui
                .selectable_label(v.saved.region_mode, "Region")
                .on_hover_text(
                    "On: a plain drag on the picture draws the render region. \
                     Off: a plain drag pans a zoomed picture. Middle-drag always pans.",
                )
                .clicked()
            {
                v.saved.region_mode = !v.saved.region_mode;
                v.mark();
            }
        });
    }

    /// Render Region over the shown slot: it becomes the picture first.
    fn render_region_of_shown(&mut self, ppp: f32) {
        if !self.shows_picture()
            && let Some(l) = self.render.viewer.loaded.take()
        {
            let mut outputs = l.outputs.into_iter();
            if let Some((_, rgba)) = outputs.next() {
                self.render.picture = Some(crate::render_ui::Picture {
                    width: l.width,
                    height: l.height,
                    rgba,
                    slot: Some(l.id),
                });
                self.render.outputs = outputs.collect();
            }
        }
        self.begin_render(ppp, true);
    }

    /// IV-05, left to right: Output, Pass, Stereo, Display, Zoom; then the
    /// Options (IV-35, IV-35a) and the panel switches.
    fn viewer_header(&mut self, ui: &mut egui::Ui) {
        let ppp = ui.ctx().pixels_per_point();
        let fit_hint = self.viewer_hint(FIT);
        let strip_hint = self.viewer_hint(STRIP);
        let stats_hint = self.viewer_hint(TAB_STATISTICS);
        let names = self.shown_output_names();
        let size = self.shown_pixels().map(|(w, h, _)| [w, h]);
        ui.horizontal_wrapped(|ui| {
            // Output (render outputs v1 §1: each output is a layer).
            let mut shown = self.render.shown;
            let only_final = "Output: this picture has only Final Color";
            ui.gated_ui(
                crate::unavailable::unless(!names.is_empty(), only_final),
                |ui| {
                    egui::ComboBox::from_id_salt("render-output-shown")
                        .selected_text(
                            shown
                                .and_then(|k| names.get(k))
                                .map_or(FINAL_COLOR, |n| n.as_str()),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut shown, None, FINAL_COLOR);
                            for (k, n) in names.iter().enumerate() {
                                ui.selectable_value(&mut shown, Some(k), n);
                            }
                        })
                        .response
                        .on_hover_text("Output: which Render Output the picture shows");
                },
            );
            if shown != self.render.shown {
                self.render.shown = shown;
                self.render.texture_stale = true;
            }
            // Pass and Stereo come in later stages: shown greyed.
            ui.gated_ui(
                Some("Pass: picks a render pass (comes in a later stage)"),
                |ui| {
                    egui::ComboBox::from_id_salt("iv-pass")
                        .selected_text("Pass: All")
                        .show_ui(ui, |_| {})
                },
            );
            ui.gated_ui(
                Some("Stereo: picks an eye (comes in a later stage)"),
                |ui| {
                    egui::ComboBox::from_id_salt("iv-stereo")
                        .selected_text("Stereo: Off")
                        .show_ui(ui, |_| {})
                },
            );
            let v = &mut self.render.viewer;
            let mut view = v.view();
            egui::ComboBox::from_id_salt("iv-display")
                .selected_text(view.label())
                .show_ui(ui, |ui| {
                    for t in ViewTransform::ALL {
                        ui.selectable_value(&mut view, t, t.label());
                    }
                })
                .response
                .on_hover_text(
                    "Display: how the floats show on screen. It never changes the \
                     stored picture or saved floats.",
                );
            if Some(view) != v.transform {
                v.transform = Some(view);
                self.render.texture_stale = true;
            }
            // Zoom in percent of screen pixels per picture pixel.
            let fit = size.map_or(1.0, |s| fit_zoom(self.render_picture_area(), s, ppp));
            let v = &mut self.render.viewer;
            let mut percent = v.zoom.unwrap_or(fit) * 100.;
            let r = ui
                .add(
                    crate::numeric_tab::DragValue::new(&mut percent)
                        .range(1.0..=3200.0)
                        .suffix("%")
                        .speed(1.0)
                        .max_decimals(0),
                )
                .on_hover_text(format!(
                    "Zoom (mouse wheel, Cmd+Alt-drag); fit to the window{fit_hint}"
                ));
            if r.changed() {
                v.zoom = Some((percent / 100.).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end()));
            }
            ui.separator();
            let mut channels = v.saved.channels;
            egui::ComboBox::from_id_salt("iv-channels")
                .selected_text(channels.label())
                .show_ui(ui, |ui| {
                    for c in Channels::ALL {
                        ui.selectable_value(&mut channels, c, c.label());
                    }
                })
                .response
                .on_hover_text("Display Channels: one channel shows as grey");
            if channels != v.saved.channels {
                v.saved.channels = channels;
                v.mark();
                self.render.texture_stale = true;
            }
            let v = &mut self.render.viewer;
            if ui
                .checkbox(&mut v.saved.show_clipping, "Show Clipping")
                .on_hover_text("Stripe the pixels where a colour channel is over 1.0")
                .changed()
            {
                v.mark();
                self.render.texture_stale = true;
            }
            let v = &mut self.render.viewer;
            ui.separator();
            if ui
                .selectable_label(v.saved.show_strip, "Strip")
                .on_hover_text(format!("Show or hide the history strip{strip_hint}"))
                .clicked()
            {
                v.saved.show_strip = !v.saved.show_strip;
                v.mark();
            }
            if ui
                .selectable_label(v.saved.show_panel, "Panel")
                .on_hover_text(format!(
                    "Show or hide the side panel (Statistics{stats_hint})"
                ))
                .clicked()
            {
                v.saved.show_panel = !v.saved.show_panel;
                v.mark();
            }
        });
    }

    /// The picture's area as of the last frame, for the Zoom field's fit
    /// value.
    fn render_picture_area(&self) -> Rect {
        self.render.viewer_area
    }

    /// The history strip: Save Image... and Load Image... above the
    /// thumbnails, newest first.
    fn history_strip(&mut self, ui: &mut egui::Ui) {
        let save_hint = self.viewer_hint(SAVE);
        ui.add_space(4.);
        ui.horizontal_wrapped(|ui| {
            // UIC-03: these ask for more, so their labels end with "...".
            if ui
                .gated(
                    crate::unavailable::unless(
                        self.shown_pixels().is_some(),
                        "Render or pick a slot first",
                    ),
                    egui::Button::new("Save Image..."),
                )
                .on_hover_text(format!("Save the shown picture to a file{save_hint}"))
                .clicked()
            {
                self.open_save_dialog(None);
            }
            if ui
                .button("Load Image...")
                .on_hover_text("Add a picture file to the strip")
                .clicked()
            {
                self.ask_load_path();
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("iv-strip-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.render.busy() {
                    self.live_tile(ui);
                }
                if self.render.history.slots.is_empty() && !self.render.busy() {
                    ui.weak("Renders appear here");
                }
                let ids: Vec<u64> = self.render.history.slots.iter().map(|s| s.id).collect();
                for (i, id) in ids.into_iter().enumerate() {
                    self.slot_tile(ui, i + 1, id);
                }
            });
    }

    fn tile_size(ui: &egui::Ui) -> Vec2 {
        let w = ui.available_width().max(40.);
        Vec2::new(w, (w * 0.66).round())
    }

    /// The render in progress, above the slots.
    fn live_tile(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(Self::tile_size(ui), Sense::click());
        let selected = self.render.viewer.primary.is_none();
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2., theme::color(ColorId::Field));
        let progress = self
            .render
            .control()
            .map(|c| c.progress())
            .filter(|p| p.samples_total > 0)
            .map_or(String::new(), |p| {
                format!("\n{} / {}", p.samples_done, p.samples_total)
            });
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("Rendering…{progress}"),
            egui::FontId::proportional(12.),
            ui.visuals().text_color(),
        );
        if selected {
            painter.rect_stroke(
                rect.shrink(1.),
                2.,
                Stroke::new(2_f32, theme::color(ColorId::Accent)),
                egui::StrokeKind::Inside,
            );
        }
        if response.on_hover_text("The render in progress").clicked() {
            self.render.viewer.show_live();
            self.render.texture_stale = true;
        }
        ui.add_space(4.);
    }

    fn thumb(&mut self, ctx: &egui::Context, slot: &Slot) -> Option<TextureHandle> {
        let folder = self.render.history.folder.clone();
        self.render
            .viewer
            .thumbs
            .entry(slot.id)
            .or_insert_with(|| {
                let image = image::open(slot.thumb_file(&folder)).ok()?.to_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                Some(ctx.load_texture(
                    format!("iv-thumb-{}", slot.id),
                    egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                    TextureOptions::LINEAR,
                ))
            })
            .clone()
    }

    /// One thumbnail: its strip number, the accent outline when selected
    /// (UIC-01, UIC-33), the x and lock on hover, and its menu.
    fn slot_tile(&mut self, ui: &mut egui::Ui, place: usize, id: u64) {
        let Some(slot) = self.render.history.get(id).cloned() else {
            return;
        };
        let (rect, response) = ui.allocate_exact_size(Self::tile_size(ui), Sense::click());
        let texture = self.thumb(ui.ctx(), &slot);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2., theme::color(ColorId::Field));
        if let Some(t) = &texture {
            let fit = crate::render_ui::fit_picture(
                rect.shrink(2.),
                [t.size()[0] as u32, t.size()[1] as u32],
            );
            // Thumbnails are small: fill the tile rather than 1:1.
            let k = (rect.shrink(2.).width() / fit.width().max(1.))
                .min(rect.shrink(2.).height() / fit.height().max(1.));
            let shown = Rect::from_center_size(rect.center(), fit.size() * k);
            painter.image(
                t.id(),
                shown,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                Color32::WHITE,
            );
        }
        // The number keys 1–9 and 0 pick (IV-10).
        let badge = Rect::from_min_size(rect.min + Vec2::splat(3.), Vec2::new(18., 14.));
        painter.rect_filled(badge, 3., Color32::from_black_alpha(170));
        painter.text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            place.to_string(),
            egui::FontId::proportional(10.),
            Color32::WHITE,
        );
        let selected =
            self.render.viewer.selected.contains(&id) || self.render.viewer.primary == Some(id);
        if selected {
            let width: f32 = if self.render.viewer.primary == Some(id) {
                2.
            } else {
                1.
            };
            painter.rect_stroke(
                rect.shrink(1.),
                2.,
                Stroke::new(width, theme::color(ColorId::Accent)),
                egui::StrokeKind::Inside,
            );
        }
        let hovered = response.hovered()
            || ui
                .ctx()
                .pointer_hover_pos()
                .is_some_and(|p| rect.contains(p));
        // Hover: the x (remove) at the top right and the lock beside it;
        // a locked slot always shows its lock.
        let x_rect = Rect::from_min_size(
            Pos2::new(rect.right() - 19., rect.top() + 3.),
            Vec2::splat(16.),
        );
        let lock_rect = x_rect.translate(Vec2::new(-19., 0.));
        let mut acted = false;
        if hovered {
            let x = ui
                .interact(x_rect, ui.id().with(("iv-x", id)), Sense::click())
                .on_hover_text(if slot.locked {
                    "Locked: unlock it to remove it"
                } else {
                    "Remove this slot and its cached file (no undo)"
                });
            paint_x(&painter, x_rect, !slot.locked, x.hovered());
            if x.clicked() {
                self.remove_slot(id);
                acted = true;
            }
        }
        if hovered || slot.locked {
            let lock = ui
                .interact(lock_rect, ui.id().with(("iv-lock", id)), Sense::click())
                .on_hover_text(if slot.locked {
                    "Unlock: Remove and Max Render Slots can take it again"
                } else {
                    "Lock: keep it from Remove and from Max Render Slots"
                });
            paint_lock(&painter, lock_rect, slot.locked, lock.hovered());
            if lock.clicked() {
                self.toggle_slot_lock(id);
                acted = true;
            }
        }
        let tip = {
            let what = match slot.kind {
                SlotKind::Render => format!("Render {} · {}", slot.number, slot.scene),
                SlotKind::Loaded => format!("Loaded · {}", slot.scene),
            };
            format!(
                "{what}\n{}\n{} × {} · key {}",
                image_viewer::time_label(slot.time),
                slot.width,
                slot.height,
                if place == 10 {
                    "0".into()
                } else if place < 10 {
                    place.to_string()
                } else {
                    "none".into()
                }
            )
        };
        let response = response.on_hover_text(tip);
        if response.clicked() && !acted {
            // UIC-32: Ctrl+click adds to the selection; on the Mac Cmd too.
            let mods = ui.input(|i| i.modifiers);
            self.click_slot(id, mods.ctrl || mods.command || mods.mac_cmd);
        }
        response.context_menu(|ui| self.slot_menu(ui, &slot));
        ui.add_space(4.);
    }

    /// The thumbnail's right-click menu (UIC-07: never empty).
    fn slot_menu(&mut self, ui: &mut egui::Ui, slot: &Slot) {
        if ui.button("Save Image...").clicked() {
            self.open_save_dialog(Some(slot.id));
            ui.close();
        }
        let lock = if slot.locked { "Unlock" } else { "Lock" };
        if ui.button(lock).clicked() {
            self.toggle_slot_lock(slot.id);
            ui.close();
        }
        if ui
            .gated(
                crate::unavailable::unless(!slot.locked, "Locked: unlock it to remove it"),
                egui::Button::new("Remove"),
            )
            .clicked()
        {
            self.remove_slot(slot.id);
            ui.close();
        }
        let file = slot
            .saved
            .clone()
            .filter(|p| p.exists())
            .unwrap_or_else(|| slot.output_file(&self.render.history.folder, 0));
        if ui
            .button(reveal_label())
            .on_hover_text(file.display().to_string())
            .clicked()
        {
            reveal(&file);
            ui.close();
        }
    }

    /// The side panel's tabs; Compare isn't built yet. Tabs
    /// change only when the user picks one (UIC-21).
    fn viewer_side_panel(&mut self, ui: &mut egui::Ui) {
        let hints = [
            self.viewer_hint(TAB_PROCESSING),
            self.viewer_hint(TAB_COMPARE),
            self.viewer_hint(TAB_STATISTICS),
        ];
        ui.add_space(4.);
        ui.horizontal_wrapped(|ui| {
            let v = &mut self.render.viewer;
            for (tab, hint) in Tab::ALL.into_iter().zip(&hints) {
                if ui
                    .selectable_label(v.saved.tab == tab, tab.label())
                    .on_hover_text(format!("{}{hint}", tab.label()))
                    .clicked()
                {
                    v.saved.tab = tab;
                    v.mark();
                }
            }
        });
        ui.separator();
        match self.render.viewer.saved.tab {
            Tab::ImageProcessing => self.image_processing_tab(ui),
            Tab::Compare => {
                ui.weak("Compare (two slots side by side or wiped) comes in a later stage.");
            }
            Tab::Statistics => {
                egui::ScrollArea::vertical()
                    .id_salt("iv-stats")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.statistics(ui));
            }
        }
    }

    /// Statistics (IV-35b): live while it renders, else the shown slot's.
    fn statistics(&mut self, ui: &mut egui::Ui) {
        let mut render: Vec<(String, String)> = Vec::new();
        let mut picture: Vec<(String, String)> = Vec::new();
        let mut device: Vec<(String, String)> = Vec::new();
        let live = self.render.viewer.primary.is_none() && self.render.busy();
        if let (true, Some(job)) = (live, &self.render.job) {
            let p = job.control.progress();
            let elapsed = job.elapsed().as_secs_f64();
            render.push(("Elapsed".into(), format!("{elapsed:.1} s")));
            let remaining = if p.samples_done > 0 && p.samples_total > p.samples_done {
                let per = elapsed / f64::from(p.samples_done);
                format!("{:.1} s", per * f64::from(p.samples_total - p.samples_done))
            } else {
                "—".into()
            };
            render.push(("Estimated Remaining".into(), remaining));
            render.push((
                "Samples".into(),
                format!(
                    "{} of {}",
                    p.samples_done,
                    p.samples_total.max(job.samples_total)
                ),
            ));
            if let Some(pic) = &self.render.picture {
                picture.push((
                    "Resolution".into(),
                    format!("{} × {}", pic.width, pic.height),
                ));
            }
            picture.push(("Camera".into(), job.camera.clone()));
            picture.push((
                "Polygons (after subdivision)".into(),
                format!("{} triangles", job.triangles),
            ));
        } else {
            let slot = self
                .render
                .viewer
                .primary
                .or(self.render.picture.as_ref().and_then(|p| p.slot))
                .and_then(|id| self.render.history.get(id));
            let Some(slot) = slot else {
                ui.weak("No render to describe yet");
                return;
            };
            let s = &slot.stats;
            match slot.kind {
                SlotKind::Render => {
                    let mut total = format!("{:.1} s", s.seconds);
                    if s.region {
                        total.push_str(" (region)");
                    }
                    render.push(("Total Time".into(), total));
                    let mut samples = format!("{} of {}", s.samples, s.samples_total);
                    if s.finished_early {
                        samples.push_str(" (finished early)");
                    }
                    render.push(("Samples".into(), samples));
                    picture.push(("Camera".into(), s.camera.clone()));
                    picture.push((
                        "Polygons (after subdivision)".into(),
                        format!("{} triangles", s.triangles),
                    ));
                    let kind = if s.device.contains("CPU") {
                        "CPU"
                    } else {
                        "GPU"
                    };
                    device.push(("Device".into(), format!("{kind} · {}", s.device)));
                    device.push((
                        "Denoised".into(),
                        if s.denoised { "Yes" } else { "No" }.into(),
                    ));
                    device.push((
                        "Peak Memory".into(),
                        s.peak_memory
                            .map_or("Not reported by the renderer".into(), |b| {
                                format!("{:.1} MB", b as f64 / 1_048_576.)
                            }),
                    ));
                }
                SlotKind::Loaded => {
                    render.push((
                        "Loaded From".into(),
                        slot.saved
                            .as_ref()
                            .map_or(slot.scene.clone(), |p| p.display().to_string()),
                    ));
                }
            }
            picture.insert(
                0,
                (
                    "Resolution".into(),
                    format!("{} × {}", slot.width, slot.height),
                ),
            );
            render.insert(0, ("Scene".into(), slot.scene.clone()));
        }
        let sections = [("Render", render), ("Picture", picture), ("Device", device)];
        for (k, (title, rows)) in sections.into_iter().enumerate() {
            if rows.is_empty() {
                continue;
            }
            let open = self.render.viewer.sections[k];
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.), Sense::click());
            icons::disclosure(
                ui,
                Rect::from_min_size(rect.min + Vec2::new(0., 1.), Vec2::splat(16.)),
                if open { 1. } else { 0. },
            );
            ui.painter().text(
                rect.min + Vec2::new(18., 9.),
                egui::Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(13.),
                ui.visuals().strong_text_color(),
            );
            if response
                .on_hover_text("Shift+click opens or closes every section")
                .clicked()
            {
                // UIC-37: Shift+click sets every section like this one.
                if ui.input(|i| i.modifiers.shift) {
                    self.render.viewer.sections = [!open; 3];
                } else {
                    self.render.viewer.sections[k] = !open;
                }
            }
            if self.render.viewer.sections[k] {
                egui::Grid::new(("iv-stats-grid", k))
                    .num_columns(2)
                    .striped(false)
                    .show(ui, |ui| {
                        for (name, value) in rows {
                            ui.weak(name);
                            ui.label(value);
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(4.);
        }
    }

    /// The status line under the picture (IV-07).
    fn pixel_readout(&mut self, ui: &mut egui::Ui) {
        let text = match self.render.viewer.hover {
            Some(([x, y], [r, g, b, a])) => {
                format!("X {x}  Y {y}    R {r:.4}  G {g:.4}  B {b:.4}  A {a:.4}")
            }
            None => match self.shown_pixels() {
                Some((w, h, _)) => format!("{w} × {h} · point at the picture to read a pixel"),
                None => "No picture".into(),
            },
        };
        ui.add(egui::Label::new(egui::RichText::new(text).monospace()).truncate());
    }

    /// The picture: zoom, pan, the region and the readout (IV-06, IV-07).
    fn viewer_picture(&mut self, ui: &mut egui::Ui) {
        let ppp = ui.ctx().pixels_per_point();
        let area = ui.max_rect();
        self.render.viewer_area = area;
        let response = ui.allocate_rect(area, Sense::click_and_drag());
        self.render.viewer.hover = None;
        let Some((w, h, _)) = self.shown_pixels() else {
            ui.painter().text(
                area.center(),
                egui::Align2::CENTER_CENTER,
                "No picture yet",
                egui::FontId::proportional(14.),
                ui.visuals().weak_text_color(),
            );
            return;
        };
        let size = [w, h];
        let fit = fit_zoom(area, size, ppp);
        let v = &mut self.render.viewer;
        let mut zoom = v.zoom.unwrap_or(fit);
        // Mouse wheel zooms about the pointer.
        if let Some(pos) = response.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            let pinch = ui.input(|i| i.zoom_delta());
            let factor = (scroll * 0.0025).exp() * pinch;
            if (factor - 1.).abs() > 1e-4 {
                let to = (zoom * factor).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                v.pan = zoom_about(area, v.pan, pos, zoom, to);
                zoom = to;
                v.zoom = Some(zoom);
            }
        }
        if v.zoom.is_none() {
            v.pan = Vec2::ZERO;
        }
        let mods = ui.input(|i| i.modifiers);
        // Cmd+Alt-drag zooms (Ctrl+Alt off the Mac).
        let zoom_mods = mods.alt
            && if cfg!(target_os = "macos") {
                mods.mac_cmd
            } else {
                mods.ctrl
            };
        if response.drag_started() {
            let primary = response.dragged_by(PointerButton::Primary);
            v.drag = if response.dragged_by(PointerButton::Middle) {
                Some(Drag::Pan)
            } else if primary && zoom_mods {
                response.interact_pointer_pos().map(|anchor| Drag::Zoom {
                    anchor,
                    zoom,
                    pan: v.pan,
                })
            } else {
                let shown = picture_rect(area, size, zoom, ppp, v.pan);
                let bigger =
                    shown.width() > area.width() + 0.5 || shown.height() > area.height() + 0.5;
                // Plain drag pans only when Region is off (or while it
                // renders, when no region can be drawn).
                (primary && bigger && (!v.saved.region_mode || self.render.job.is_some()))
                    .then_some(Drag::Pan)
            };
        }
        let v = &mut self.render.viewer;
        match v.drag {
            Some(Drag::Pan) if response.dragged() => {
                v.pan += response.drag_delta();
                if v.zoom.is_none() {
                    v.zoom = Some(zoom);
                }
            }
            Some(Drag::Zoom {
                anchor,
                zoom: z0,
                pan: p0,
            }) if response.dragged() => {
                if let Some(now) = response.interact_pointer_pos() {
                    let to = (z0 * ((now.x - anchor.x) * 0.01).exp())
                        .clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
                    v.pan = zoom_about(area, p0, anchor, z0, to);
                    zoom = to;
                    v.zoom = Some(zoom);
                }
            }
            _ => {}
        }
        if response.drag_stopped() {
            v.drag = None;
        }
        let shown = picture_rect(area, size, zoom, ppp, v.pan);
        let navigating = v.drag.is_some();
        // The texture: rebuilt when the picture or a display choice changed.
        let options = if zoom > 1.01 {
            TextureOptions::NEAREST
        } else {
            TextureOptions::LINEAR
        };
        if self.render.texture_stale
            || self.render.texture.is_none()
            || options != self.render.viewer.texture_options
        {
            let saved = &self.render.viewer.saved;
            // Group C: a linear output shows its raw numbers, a forced
            // sRGB one through sRGB; Default beauty outputs follow the
            // window's view switch.
            let (channels, clipping, view) = (
                saved.channels,
                saved.show_clipping,
                self.shown_meta().view(self.render.viewer.view()),
            );
            if let Some((w, h, rgba)) = self.shown_pixels() {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &image_viewer::display_rgba8(rgba, w, view, channels, clipping),
                );
                match &mut self.render.texture {
                    Some(t) if t.size() == [w as usize, h as usize] => t.set(image, options),
                    _ => {
                        self.render.texture =
                            Some(ui.ctx().load_texture("operandi-render", image, options))
                    }
                }
            }
            self.render.viewer.texture_options = options;
            self.render.texture_stale = false;
        }
        if let Some(t) = &self.render.texture {
            ui.painter_at(area).image(
                t.id(),
                shown,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                Color32::WHITE,
            );
        }
        // The pixel readout (IV-07).
        if let Some(pos) = response.hover_pos().filter(|p| shown.contains(*p))
            && let Some((w, _, rgba)) = self.shown_pixels()
        {
            let [x, y] = crate::render_ui::pixel_at(shown, size, pos);
            let i = ((y * w + x) * 4) as usize;
            if let Some(p) = rgba.get(i..i + 4) {
                self.render.viewer.hover = Some(([x, y], [p[0], p[1], p[2], p[3]]));
            }
        }
        // Drawing the region (not while rendering, so the picture under it
        // stays the one being rendered into).
        let region_drag = self.render.viewer.saved.region_mode && !navigating;
        if !self.render.busy() && region_drag {
            if response.drag_started_by(PointerButton::Primary)
                && !zoom_mods
                && let Some(pos) = response.interact_pointer_pos()
                && shown.contains(pos)
            {
                self.render.drag_from = Some(crate::render_ui::pixel_at(shown, size, pos));
            }
            if let (Some(from), Some(pos)) =
                (self.render.drag_from, response.interact_pointer_pos())
                && response.dragged_by(PointerButton::Primary)
            {
                self.render.region = crate::render_ui::region_between(
                    from,
                    crate::render_ui::pixel_at(shown, size, pos),
                );
            }
            if response.clicked() {
                self.render.region = None;
            }
        }
        if response.drag_stopped() {
            self.render.drag_from = None;
        }
        let outline = self
            .render
            .job
            .as_ref()
            .and_then(|j| j.region)
            .or(self.render.region);
        if let Some(r) = outline {
            let rect = crate::render_ui::region_rect(shown, size, r);
            ui.painter_at(area).rect_stroke(
                rect,
                0.,
                Stroke::new(1.5_f32, theme::orange()),
                egui::StrokeKind::Middle,
            );
        }
    }

    /// Save Image... (IV-36): the slot's output and the format, then the
    /// file panel for name and folder.
    fn save_image_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.render.viewer.save_dialog else {
            return;
        };
        let names: Vec<String> = match d.slot.and_then(|id| self.render.history.get(id)) {
            Some(s) => s.outputs.iter().skip(1).cloned().collect(),
            None => self.render.outputs.iter().map(|(n, _)| n.clone()).collect(),
        };
        // UIC-80..99: the shared form dialog. Its form lives in egui's
        // memory while open; the output and format come back on OK.
        let id = egui::Id::new("iv-save-image-form");
        let frame = ctx.cumulative_frame_nr();
        let mut form = ctx
            .data_mut(|m| m.get_temp::<(crate::form::Dialog, u64)>(id))
            .filter(|(_, shown)| shown + 1 >= frame)
            .map(|(form, _)| form)
            .unwrap_or_else(|| {
                use crate::form::{Row, When};
                let outputs: Vec<&str> = std::iter::once(FINAL_COLOR)
                    .chain(names.iter().map(String::as_str))
                    .collect();
                let formats: Vec<&str> = SaveFormat::ALL.iter().map(|f| f.label()).collect();
                let format = SaveFormat::ALL
                    .iter()
                    .position(|f| *f == d.format)
                    .unwrap_or(0);
                // OpenEXR Multilayer saves every output (RO-Q17).
                let single: Vec<usize> = (0..SaveFormat::ALL.len())
                    .filter(|&i| SaveFormat::ALL[i] != SaveFormat::ExrMultilayer)
                    .collect();
                let rows = vec![
                    Row::choice("output", "Output", &outputs, d.output.map_or(0, |k| k + 1))
                        .when(if names.is_empty() {
                            When::Never
                        } else {
                            When::ChoiceIn("format", single)
                        })
                        .tip(
                            "The render output to save (OpenEXR Multilayer saves every \
                             output in one file)",
                        ),
                    Row::choice("format", "Format", &formats, format).tip(if d.format.is_float() {
                        "Float formats keep the linear floats as rendered. OK then asks for \
                         the file name and folder."
                            .to_string()
                    } else {
                        format!(
                            "Saved as shown with the {} display transform (float formats keep \
                             the linear floats). OK then asks for the file name and folder.",
                            self.render.viewer.view().label()
                        )
                    }),
                ];
                crate::form::Dialog::with_defaults("Save Image", rows)
            });
        let outcome = form.show(ctx);
        match outcome {
            crate::form::Outcome::Open => {
                ctx.data_mut(|m| m.insert_temp(id, (form, frame)));
                self.render.viewer.save_dialog = Some(d);
            }
            crate::form::Outcome::Ok => {
                ctx.data_mut(|m| m.remove::<(crate::form::Dialog, u64)>(id));
                let v = &form.values;
                d.output = v.choice("output").checked_sub(1);
                d.format = SaveFormat::ALL[v.choice("format").min(SaveFormat::ALL.len() - 1)];
                // UIC-95/96: the format is kept for next time on OK only.
                if d.format != self.render.viewer.saved.save_format {
                    self.render.viewer.saved.save_format = d.format;
                    self.render.viewer.mark();
                }
                self.render.viewer.save_dialog = None;
                self.ask_save_path(d);
            }
            _ => {
                ctx.data_mut(|m| m.remove::<(crate::form::Dialog, u64)>(id));
                self.render.viewer.save_dialog = None;
            }
        }
    }
}

use crate::icons;

/// The x on a hovered thumbnail.
fn paint_x(painter: &egui::Painter, rect: Rect, enabled: bool, hot: bool) {
    painter.rect_filled(
        rect,
        3.,
        Color32::from_black_alpha(if hot { 220 } else { 160 }),
    );
    let color = if enabled {
        Color32::WHITE
    } else {
        Color32::from_gray(110)
    };
    let r = rect.shrink(4.5);
    let stroke = Stroke::new(1.5_f32, color);
    painter.line_segment([r.left_top(), r.right_bottom()], stroke);
    painter.line_segment([r.right_top(), r.left_bottom()], stroke);
}

/// A small padlock: closed and accent-coloured when locked. Drawn with
/// strokes as a stand-in until the icon generator has a lock recipe.
fn paint_lock(painter: &egui::Painter, rect: Rect, locked: bool, hot: bool) {
    painter.rect_filled(
        rect,
        3.,
        Color32::from_black_alpha(if hot { 220 } else { 160 }),
    );
    let color = if locked {
        theme::color(ColorId::Accent)
    } else {
        Color32::from_gray(200)
    };
    let body = Rect::from_min_max(
        Pos2::new(rect.left() + 4., rect.center().y - 0.5),
        Pos2::new(rect.right() - 4., rect.bottom() - 3.),
    );
    painter.rect_filled(body, 1., color);
    let (l, r) = (body.left() + 1.5, body.right() - 1.5);
    let top = rect.top() + 3.5;
    let stroke = Stroke::new(1.4_f32, color);
    // The shackle; open (one side lifted) when unlocked.
    let right_bottom = if locked { body.top() } else { top + 3. };
    painter.line_segment([Pos2::new(l, body.top()), Pos2::new(l, top + 2.)], stroke);
    painter.line_segment(
        [Pos2::new(l, top + 2.), Pos2::new((l + r) / 2., top)],
        stroke,
    );
    painter.line_segment(
        [Pos2::new((l + r) / 2., top), Pos2::new(r, top + 2.)],
        stroke,
    );
    painter.line_segment([Pos2::new(r, top + 2.), Pos2::new(r, right_bottom)], stroke);
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Modifiers;
    use std::collections::BTreeMap;

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn number_keys_pick_slots_and_bound_keys_reach_the_viewer() {
        let bindings: BTreeMap<String, String> = crate::input::default_bindings();
        let cmd = if cfg!(target_os = "macos") {
            Modifiers::MAC_CMD.plus(Modifiers::COMMAND)
        } else {
            Modifiers::CTRL.plus(Modifiers::COMMAND)
        };
        let events = [
            key(Key::Num3, Modifiers::NONE),
            key(Key::Num0, Modifiers::NONE),
            key(Key::Space, Modifiers::NONE),
            key(Key::T, Modifiers::NONE),
            key(Key::P, Modifiers::NONE),
            key(Key::I, Modifiers::NONE),
            key(Key::C, Modifiers::NONE),
            key(Key::S, Modifiers::NONE),
            key(Key::A, cmd),
            key(Key::S, cmd),
            key(Key::F9, Modifiers::ALT.plus(Modifiers::SHIFT)),
            key(Key::F9, Modifiers::NONE),
            // Shift+3 is not a slot key.
            key(Key::Num3, Modifiers::SHIFT),
        ];
        assert_eq!(
            viewer_keys(&events, &bindings),
            vec![
                ViewerKey::Slot(3),
                ViewerKey::Slot(10),
                ViewerKey::HideAll,
                ViewerKey::Action(STRIP),
                ViewerKey::Action(PAUSE),
                ViewerKey::Action(TAB_PROCESSING),
                ViewerKey::Action(TAB_COMPARE),
                ViewerKey::Action(TAB_STATISTICS),
                ViewerKey::Action(FIT),
                ViewerKey::Action(SAVE),
                ViewerKey::Action(OPEN),
                ViewerKey::Action("Render"),
            ]
        );
    }

    #[test]
    fn viewer_keys_share_chords_with_main_window_actions_only_across_windows() {
        // T is Element Move in the main window and the strip in the viewer.
        assert!(!crate::input::contexts_overlap(STRIP, "Element Move"));
        assert!(!crate::input::contexts_overlap("Make", PAUSE));
        assert!(crate::input::contexts_overlap(STRIP, FIT));
        assert!(crate::input::bindings_valid(
            &crate::input::default_bindings()
        ));
        assert_eq!(
            crate::input_map_editor::context(SAVE),
            "Render window focused"
        );
        assert_eq!(
            crate::input::default_bindings()[OPEN],
            "Alt+Shift+F9",
            "IV-02"
        );
    }

    #[test]
    fn open_render_window_is_a_command_and_a_menu_entry_that_never_renders() {
        let mut a = crate::Operandi::empty();
        a.execute("Open Render Window");
        assert!(a.render.open && !a.render.busy());
        a.render.open = false;
        a.menu_action(
            &egui::Context::default(),
            crate::app_menu::line_action("Open Render Window"),
        );
        assert!(a.render.open && !a.render.busy());
    }

    #[test]
    fn zoom_keeps_the_point_under_the_pointer_still() {
        let area = Rect::from_min_size(Pos2::new(0., 0.), Vec2::new(800., 600.));
        let size = [1600, 1200];
        let fit = fit_zoom(area, size, 1.0);
        assert_eq!(fit, 0.5);
        let at = Pos2::new(600., 200.);
        let before = picture_rect(area, size, fit, 1.0, Vec2::ZERO);
        let pan = zoom_about(area, Vec2::ZERO, at, fit, 2.0);
        let after = picture_rect(area, size, 2.0, 1.0, pan);
        // The same picture pixel sits under the pointer.
        let u = |r: Rect| ((at.x - r.left()) / r.width(), (at.y - r.top()) / r.height());
        let (a, b) = (u(before), u(after));
        assert!((a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5);
        // Small pictures fit 1:1, never enlarged; Retina halves the points.
        assert_eq!(fit_zoom(area, [100, 100], 2.0), 1.0);
        assert_eq!(
            picture_rect(area, [100, 100], 1.0, 2.0, Vec2::ZERO).width(),
            50.
        );
    }

    #[test]
    fn ctrl_click_toggles_the_selection_and_keeps_one_shown() {
        let mut v = ViewerState::default();
        v.select_only(1);
        v.toggle_selected(2);
        assert_eq!((v.selected.clone(), v.primary), (vec![1, 2], Some(2)));
        v.toggle_selected(2);
        assert_eq!((v.selected.clone(), v.primary), (vec![1], Some(1)));
        // The last one can't be toggled away: something stays shown.
        v.toggle_selected(1);
        assert_eq!((v.selected.clone(), v.primary), (vec![1], Some(1)));
        v.forget(1);
        assert_eq!((v.selected.clone(), v.primary), (vec![], None));
    }
}
