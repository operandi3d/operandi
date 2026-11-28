//! Image Viewer, stage 1 (IV-*): the Render window's history of renders and
//! loaded images, kept as full-precision files in a cache folder, and the
//! picture conversions the window shows and saves. Nothing here needs a
//! window or a GPU; `image_viewer_ui.rs` draws it.
//!
//! - Slots (IV-08..13b): newest first. Renders count against Max Render
//!   Slots; loaded images don't. A new render over the cap removes the
//!   oldest unlocked render, or the oldest render when all are locked.
//! - Cache (IV-12, IV-13): every output of a slot is a plain float RGBA
//!   OpenEXR file, `slot-<id>-<k>.exr`, beside an 8-bit `slot-<id>-thumb.png`
//!   and one `index.json` holding order, locks, numbers and statistics.
//! - Display (IV-14, IV-35, IV-35a): the view transform, the channel shown
//!   and the clipping stripes change only what is drawn, never the stored
//!   floats.
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The index file inside the cache folder.
const INDEX: &str = "index.json";
/// Longest side of a strip thumbnail, in pixels.
pub const THUMB_SIDE: u32 = 160;
/// Max Render Slots range; 10 is the spec's default (IV-13).
pub const MAX_SLOTS_RANGE: std::ops::RangeInclusive<u32> = 1..=100;

/// Where a slot's picture came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotKind {
    #[default]
    Render,
    /// Load Image... (IV-38): never counts against Max Render Slots.
    Loaded,
}

/// What the Statistics tab shows for a slot (IV-35b), kept with it so old
/// renders keep their numbers.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    /// Render time, leaving out time paused.
    pub seconds: f64,
    pub samples: u32,
    pub samples_total: u32,
    /// Peak memory in bytes, when the renderer reports it (it doesn't yet).
    pub peak_memory: Option<u64>,
    /// Triangles sent to the renderer: the polygon count after subdivision.
    pub triangles: u64,
    pub camera: String,
    pub device: String,
    pub denoised: bool,
    /// A Render Region pass over an earlier picture.
    pub region: bool,
    /// Finished early with Finish.
    pub finished_early: bool,
}

/// One picture in the history strip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slot {
    pub id: u64,
    /// Render order, counting up across sessions (0 for a loaded image).
    pub number: u32,
    pub kind: SlotKind,
    pub locked: bool,
    /// The scene's name when it rendered, or the loaded file's name.
    pub scene: String,
    /// Seconds since 1970 when it finished or was loaded.
    pub time: u64,
    pub width: u32,
    pub height: u32,
    /// The outputs' names; output 0 is the picture (Final Color).
    pub outputs: Vec<String>,
    /// Where it was also saved outside the cache (Pictures, an MCP path).
    pub saved: Option<PathBuf>,
    pub stats: Stats,
    /// Per output (as `outputs`): what it is and its colour space, for the
    /// window's view and for saving. Empty for older slots and loaded
    /// images: the window's view then decides, as before.
    pub meta: Vec<OutputMeta>,
}

/// An output's kind and colour space (render outputs group C).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputMeta {
    /// `None`: not a Render Output (a light group).
    pub effect: Option<operandi_core::shader_tree::OutputEffect>,
    pub colorspace: operandi_core::render_outputs::OutputColorspace,
}
impl OutputMeta {
    pub fn of(output: &operandi_core::render_outputs::RenderOutput) -> Self {
        Self {
            effect: Some(output.effect),
            colorspace: output.settings.colorspace,
        }
    }
    /// How the window shows it, given the window's own view switch.
    pub fn view(self, window: ViewTransform) -> ViewTransform {
        use operandi_core::render_outputs::{OutputView, output_view};
        let Some(effect) = self.effect else {
            return window;
        };
        match output_view(effect, self.colorspace) {
            OutputView::Window => window,
            OutputView::Raw => ViewTransform::Linear,
            OutputView::Srgb => ViewTransform::Srgb,
        }
    }
    /// Whether its numbers are sRGB-encoded in a file of `format`.
    pub fn writes_srgb(self, format: SaveFormat, window: ViewTransform) -> bool {
        match self.effect {
            Some(effect) => operandi_core::render_outputs::writes_srgb(
                effect,
                self.colorspace,
                format.is_float(),
            ),
            // Light groups and loaded pictures: as shown (IV-36).
            None => !format.is_float() && window == ViewTransform::Srgb,
        }
    }
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            id: 0,
            number: 0,
            kind: SlotKind::Render,
            locked: false,
            scene: String::new(),
            time: 0,
            width: 0,
            height: 0,
            outputs: vec![FINAL_COLOR.into()],
            saved: None,
            stats: Stats::default(),
            meta: Vec::new(),
        }
    }
}

/// The picture's own output name.
pub const FINAL_COLOR: &str = "Final Color";

impl Slot {
    /// The cached file holding output `k`.
    pub fn output_file(&self, folder: &Path, k: usize) -> PathBuf {
        folder.join(format!("slot-{}-{k}.exr", self.id))
    }
    pub fn thumb_file(&self, folder: &Path) -> PathBuf {
        folder.join(format!("slot-{}-thumb.png", self.id))
    }
    /// The render's raw Cryptomatte layers, when it had any (an OpenEXR
    /// multilayer file of only those, `exr_multilayer`).
    pub fn crypto_file(&self, folder: &Path) -> PathBuf {
        folder.join(format!("slot-{}-crypto.exr", self.id))
    }
    /// Output `k`'s kind and colour space (none known: the window decides).
    pub fn meta(&self, k: usize) -> OutputMeta {
        self.meta.get(k).copied().unwrap_or_default()
    }
    /// The name a saved file starts from: "Render 12", or the loaded file's.
    pub fn base_name(&self) -> String {
        match self.kind {
            SlotKind::Render => format!("Render {}", self.number),
            SlotKind::Loaded => Path::new(&self.scene)
                .file_stem()
                .map_or("Image".into(), |s| s.to_string_lossy().into_owned()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Index {
    version: u8,
    next_id: u64,
    next_number: u32,
    slots: Vec<Slot>,
}

/// The slots, newest first, and the folder their files live in. App-wide,
/// not per scene (IV-08).
#[derive(Clone, Debug, PartialEq)]
pub struct History {
    pub folder: PathBuf,
    pub slots: Vec<Slot>,
    next_id: u64,
    next_number: u32,
}

impl History {
    /// An empty history kept in `folder` (nothing is written yet).
    pub fn empty(folder: PathBuf) -> Self {
        Self {
            folder,
            slots: Vec::new(),
            next_id: 1,
            next_number: 1,
        }
    }

    /// The history saved in `folder`. Slots whose picture file is gone are
    /// dropped; a missing or unreadable index is an empty history.
    pub fn load(folder: PathBuf) -> Self {
        let index: Index = fs::read(folder.join(INDEX))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .filter(|i: &Index| i.version == 1)
            .unwrap_or_default();
        let mut history = Self::empty(folder);
        history.next_id = index.next_id.max(1);
        history.next_number = index.next_number.max(1);
        history.slots = index
            .slots
            .into_iter()
            .filter(|s| s.output_file(&history.folder, 0).is_file() && !s.outputs.is_empty())
            .collect();
        for s in &history.slots {
            history.next_id = history.next_id.max(s.id + 1);
            history.next_number = history.next_number.max(s.number + 1);
        }
        history
    }

    pub fn save_index(&self) -> io::Result<()> {
        fs::create_dir_all(&self.folder)?;
        let index = Index {
            version: 1,
            next_id: self.next_id,
            next_number: self.next_number,
            slots: self.slots.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&index).map_err(io::Error::other)?;
        let path = self.folder.join(INDEX);
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        let _ = fs::remove_file(&temp);
        crate::resource_write::write_bytes(&path, &temp, &bytes)
    }

    /// A fresh slot id, for files written before the slot joins the strip.
    pub fn reserve_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// The next render's number.
    pub fn take_number(&mut self) -> u32 {
        let n = self.next_number;
        self.next_number += 1;
        n
    }

    pub fn get(&self, id: u64) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.id == id)
    }

    /// The slot keys 1–9 and 0 (`n` = 10) select.
    pub fn at_position(&self, n: usize) -> Option<u64> {
        n.checked_sub(1)
            .and_then(|i| self.slots.get(i))
            .map(|s| s.id)
    }

    /// Adds a slot at the front, then trims renders to `max`. Returns the
    /// slots trimmed (their files are already deleted).
    pub fn add(&mut self, slot: Slot, max: u32) -> Vec<Slot> {
        let keep = slot.id;
        self.slots.insert(0, slot);
        self.trim_keeping(max, Some(keep))
    }

    /// Remove renders over `max`: the oldest unlocked first; when every
    /// render left is locked, the oldest regardless (IV-13). Loaded images
    /// are never trimmed.
    pub fn trim(&mut self, max: u32) -> Vec<Slot> {
        self.trim_keeping(max, None)
    }

    /// `trim`, never removing slot `keep` (the render that just arrived:
    /// with every older render locked, the oldest of those goes instead).
    fn trim_keeping(&mut self, max: u32, keep: Option<u64>) -> Vec<Slot> {
        let max = max.max(1) as usize;
        let mut removed = Vec::new();
        loop {
            let renders = self
                .slots
                .iter()
                .filter(|s| s.kind == SlotKind::Render)
                .count();
            if renders <= max {
                break;
            }
            let candidate = |s: &Slot| s.kind == SlotKind::Render && Some(s.id) != keep;
            let oldest_unlocked = self.slots.iter().rposition(|s| candidate(s) && !s.locked);
            let oldest = self.slots.iter().rposition(candidate);
            let Some(i) = oldest_unlocked.or(oldest) else {
                break;
            };
            let slot = self.slots.remove(i);
            delete_files(&self.folder, &slot);
            removed.push(slot);
        }
        removed
    }

    /// The x on a thumbnail or Remove: deletes the slot and its files with
    /// no prompt and no undo (IV-11). A locked slot stays; returns whether
    /// it went.
    pub fn remove(&mut self, id: u64) -> bool {
        let Some(i) = self.slots.iter().position(|s| s.id == id) else {
            return false;
        };
        if self.slots[i].locked {
            return false;
        }
        let slot = self.slots.remove(i);
        delete_files(&self.folder, &slot);
        true
    }
}

fn delete_files(folder: &Path, slot: &Slot) {
    for k in 0..slot.outputs.len() {
        let _ = fs::remove_file(slot.output_file(folder, k));
    }
    let _ = fs::remove_file(slot.thumb_file(folder));
    let _ = fs::remove_file(slot.crypto_file(folder));
}

/// Seconds since 1970, now.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// "2026-11-07 14:03" (UTC) for a tooltip.
pub fn time_label(seconds: u64) -> String {
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let days = (seconds / 86_400) as i64;
    let rem = seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        (rem % 3600) / 60
    )
}

// ---------------------------------------------------------------------------
// Cache files.

/// Write a slot's outputs (linear RGBA floats, `width` × `height`) as plain
/// OpenEXR files and its thumbnail, in `folder`. Runs on the render thread.
pub fn write_slot_files(
    folder: &Path,
    slot: &Slot,
    outputs: &[&[f32]],
    transform: ViewTransform,
) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    for (k, rgba) in outputs.iter().enumerate() {
        write_exr(&slot.output_file(folder, k), rgba, slot.width, slot.height)?;
    }
    if let Some(first) = outputs.first() {
        let (tw, th, thumb) = thumbnail(first, slot.width, slot.height, transform);
        image::RgbaImage::from_raw(tw, th, thumb)
            .ok_or("thumbnail size")?
            .save_with_format(slot.thumb_file(folder), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Read output `k` of a slot back from the cache.
pub fn read_output(folder: &Path, slot: &Slot, k: usize) -> Result<Vec<f32>, String> {
    let path = slot.output_file(folder, k);
    let image = crate::image_decode::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if (image.width(), image.height()) != (slot.width, slot.height) {
        return Err(format!("{} has the wrong size", path.display()));
    }
    Ok(image.into_rgba32f().into_raw())
}

fn write_exr(path: &Path, rgba: &[f32], width: u32, height: u32) -> Result<(), String> {
    image::Rgba32FImage::from_raw(width, height, rgba.to_vec())
        .ok_or("picture size")?
        .save_with_format(path, image::ImageFormat::OpenExr)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// A box-filtered, display-transformed 8-bit copy at most `THUMB_SIDE`
/// pixels on its longest side.
pub fn thumbnail(
    rgba: &[f32],
    width: u32,
    height: u32,
    transform: ViewTransform,
) -> (u32, u32, Vec<u8>) {
    let (w, h) = (width.max(1), height.max(1));
    let k = (f64::from(THUMB_SIDE) / f64::from(w.max(h))).min(1.0);
    let tw = ((f64::from(w) * k).round() as u32).max(1);
    let th = ((f64::from(h) * k).round() as u32).max(1);
    let mut out = Vec::with_capacity((tw * th * 4) as usize);
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let mut sum = [0f32; 4];
            let mut n = 0f32;
            for y in y0..y1.min(h) {
                for x in x0..x1.min(w) {
                    let i = ((y * w + x) * 4) as usize;
                    if let Some(p) = rgba.get(i..i + 4) {
                        for c in 0..4 {
                            sum[c] += p[c];
                        }
                        n += 1.0;
                    }
                }
            }
            let n = n.max(1.0);
            let px = [sum[0] / n, sum[1] / n, sum[2] / n, sum[3] / n];
            for v in &px[..3] {
                out.push(to_u8(transform.encode(*v)));
            }
            out.push(to_u8(px[3]));
        }
    }
    (tw, th, out)
}

// ---------------------------------------------------------------------------
// Display.

/// How floats become screen colours (IV-14). Display only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewTransform {
    #[default]
    Srgb,
    /// The raw values, clamped.
    Linear,
}
impl ViewTransform {
    pub const ALL: [Self; 2] = [Self::Srgb, Self::Linear];
    pub fn label(self) -> &'static str {
        match self {
            Self::Srgb => "sRGB",
            Self::Linear => "Linear (raw)",
        }
    }
    /// A linear value to the 0–1 display value.
    pub fn encode(self, v: f32) -> f32 {
        let v = if v.is_finite() {
            v.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match self {
            Self::Srgb => linear_to_srgb(v),
            Self::Linear => v,
        }
    }
}

pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn to_u16(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 65_535.0 + 0.5) as u16
}

/// Display Channels (IV-35): one channel shows as grey.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channels {
    #[default]
    Rgb,
    Red,
    Green,
    Blue,
    Alpha,
}
impl Channels {
    pub const ALL: [Self; 5] = [Self::Rgb, Self::Red, Self::Green, Self::Blue, Self::Alpha];
    pub fn label(self) -> &'static str {
        match self {
            Self::Rgb => "RGB",
            Self::Red => "R",
            Self::Green => "G",
            Self::Blue => "B",
            Self::Alpha => "Alpha",
        }
    }
}

/// A pixel is clipped when a colour channel is over 1.0 (IV-35a).
pub fn clipped(p: &[f32]) -> bool {
    p.iter().take(3).any(|v| *v > 1.0)
}

/// Period of the clipping stripes, in picture pixels.
const STRIPE: u32 = 8;

/// The 8-bit RGBA the window draws for `rgba`.
pub fn display_rgba8(
    rgba: &[f32],
    width: u32,
    transform: ViewTransform,
    channels: Channels,
    show_clipping: bool,
) -> Vec<u8> {
    let width = width.max(1);
    let mut out = Vec::with_capacity(rgba.len());
    for (i, p) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let (x, y) = (i as u32 % width, i as u32 / width);
        if show_clipping && clipped(p) {
            // Diagonal black and white stripes.
            let v = if ((x + y) / (STRIPE / 2)).is_multiple_of(2) {
                0
            } else {
                255
            };
            out.extend_from_slice(&[v, v, v, 255]);
            continue;
        }
        match channels {
            Channels::Rgb => {
                for v in &p[..3] {
                    out.push(to_u8(transform.encode(*v)));
                }
                out.push(to_u8(p[3]));
            }
            Channels::Alpha => {
                // Alpha is coverage, not colour: shown raw.
                let v = to_u8(p[3]);
                out.extend_from_slice(&[v, v, v, 255]);
            }
            single => {
                let c = match single {
                    Channels::Red => 0,
                    Channels::Green => 1,
                    _ => 2,
                };
                let v = to_u8(transform.encode(p[c]));
                out.extend_from_slice(&[v, v, v, 255]);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Save Image... and Load Image... (IV-36, IV-38).

/// A Save Image format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveFormat {
    #[default]
    Png8,
    Png16,
    Jpeg,
    Tiff8,
    Tiff16,
    /// Float RGBA, linear.
    Exr,
    /// Every output in one float file, Cryptomatte included
    /// (`exr_multilayer`); a single picture saves as plain OpenEXR.
    ExrMultilayer,
    /// Radiance RGBE, linear (no alpha).
    Hdr,
    Tga,
    Bmp,
}
impl SaveFormat {
    pub const ALL: [Self; 10] = [
        Self::Png8,
        Self::Png16,
        Self::Jpeg,
        Self::Tiff8,
        Self::Tiff16,
        Self::Exr,
        Self::ExrMultilayer,
        Self::Hdr,
        Self::Tga,
        Self::Bmp,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Png8 => "PNG (8-bit)",
            Self::Png16 => "PNG (16-bit)",
            Self::Jpeg => "JPEG",
            Self::Tiff8 => "TIFF (8-bit)",
            Self::Tiff16 => "TIFF (16-bit)",
            Self::Exr => "OpenEXR (float)",
            Self::ExrMultilayer => "OpenEXR Multilayer",
            Self::Hdr => "Radiance HDR",
            Self::Tga => "Targa",
            Self::Bmp => "BMP",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png8 | Self::Png16 => "png",
            Self::Jpeg => "jpg",
            Self::Tiff8 | Self::Tiff16 => "tif",
            Self::Exr | Self::ExrMultilayer => "exr",
            Self::Hdr => "hdr",
            Self::Tga => "tga",
            Self::Bmp => "bmp",
        }
    }
    /// Keeps the linear floats rather than the display-transformed picture.
    pub fn is_float(self) -> bool {
        matches!(self, Self::Exr | Self::ExrMultilayer | Self::Hdr)
    }
    /// The formats one output can save in: every one but Multilayer, which
    /// is the whole render (an output's Format list, RO-Q17).
    pub fn per_output() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(|f| *f != Self::ExrMultilayer)
    }
}

/// Saves one output `rgba` (linear floats) in `format`, encoded as its
/// colour space says (`OutputMeta::writes_srgb`; `window` is the window's
/// view, used for outputs whose colour space doesn't decide).
#[allow(clippy::too_many_arguments)]
pub fn save_output(
    path: &Path,
    rgba: &[f32],
    width: u32,
    height: u32,
    format: SaveFormat,
    meta: OutputMeta,
    window: ViewTransform,
    options: crate::file_io_preferences::ImageOptions,
) -> Result<(), String> {
    let srgb = meta.writes_srgb(format, window);
    if format.is_float() {
        if srgb {
            let encoded = srgb_floats(rgba);
            return save_image(path, &encoded, width, height, format, window, options);
        }
        return save_image(path, rgba, width, height, format, window, options);
    }
    let transform = if srgb {
        ViewTransform::Srgb
    } else {
        ViewTransform::Linear
    };
    save_image(path, rgba, width, height, format, transform, options)
}

/// sRGB-encoded floats (colour channels; alpha as it is). Values over 1
/// follow the same curve.
pub fn srgb_floats(rgba: &[f32]) -> Vec<f32> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let e = |v: f32| {
                if v <= 1.0 {
                    linear_to_srgb(v.max(0.0))
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                }
            };
            [e(p[0]), e(p[1]), e(p[2]), p[3]]
        })
        .collect()
}

/// Every extension Load Image... reads.
pub const LOAD_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "tif", "tiff", "exr", "hdr", "tga", "bmp",
];

/// `path` with the format's extension unless it already has one of its own.
pub fn with_extension(path: &Path, format: SaveFormat) -> PathBuf {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let fits = match (format, ext.as_deref()) {
        (SaveFormat::Jpeg, Some("jpg" | "jpeg")) => true,
        (SaveFormat::Tiff8 | SaveFormat::Tiff16, Some("tif" | "tiff")) => true,
        (f, Some(e)) => f.extension() == e,
        (_, None) => false,
    };
    if fits {
        path.to_path_buf()
    } else {
        let mut name = path.as_os_str().to_owned();
        name.push(".");
        name.push(format.extension());
        PathBuf::from(name)
    }
}

/// Write `rgba` (linear floats) to `path` in `format`. Integer formats get
/// the display-transformed picture; EXR and HDR keep the floats (IV-36).
pub fn save_image(
    path: &Path,
    rgba: &[f32],
    width: u32,
    height: u32,
    format: SaveFormat,
    transform: ViewTransform,
    options: crate::file_io_preferences::ImageOptions,
) -> Result<(), String> {
    if rgba.len() != width as usize * height as usize * 4 {
        return Err("The picture's size doesn't match its pixels".into());
    }
    let err = |e: image::ImageError| format!("Can't save {}: {e}", path.display());
    let eight = || -> Vec<u8> {
        rgba.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                [
                    to_u8(transform.encode(p[0])),
                    to_u8(transform.encode(p[1])),
                    to_u8(transform.encode(p[2])),
                    to_u8(p[3]),
                ]
            })
            .collect()
    };
    let sixteen = || -> Vec<u16> {
        rgba.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                [
                    to_u16(transform.encode(p[0])),
                    to_u16(transform.encode(p[1])),
                    to_u16(transform.encode(p[2])),
                    to_u16(p[3]),
                ]
            })
            .collect()
    };
    let size_err = || "picture size".to_string();
    match format {
        SaveFormat::Png8 | SaveFormat::Jpeg => {
            crate::file_io_preferences::save_rgba(path, &eight(), width, height, options)
        }
        SaveFormat::Png16 | SaveFormat::Tiff16 => {
            let kind = if format == SaveFormat::Png16 {
                image::ImageFormat::Png
            } else {
                image::ImageFormat::Tiff
            };
            image::ImageBuffer::<image::Rgba<u16>, _>::from_raw(width, height, sixteen())
                .ok_or_else(size_err)?
                .save_with_format(path, kind)
                .map_err(err)
        }
        SaveFormat::Tiff8 | SaveFormat::Tga | SaveFormat::Bmp => {
            let kind = match format {
                SaveFormat::Tiff8 => image::ImageFormat::Tiff,
                SaveFormat::Tga => image::ImageFormat::Tga,
                _ => image::ImageFormat::Bmp,
            };
            image::RgbaImage::from_raw(width, height, eight())
                .ok_or_else(size_err)?
                .save_with_format(path, kind)
                .map_err(err)
        }
        SaveFormat::Exr | SaveFormat::ExrMultilayer => write_exr(path, rgba, width, height),
        SaveFormat::Hdr => {
            let rgb: Vec<f32> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0].max(0.0), p[1].max(0.0), p[2].max(0.0)])
                .collect();
            image::Rgb32FImage::from_raw(width, height, rgb)
                .ok_or_else(size_err)?
                .save_with_format(path, image::ImageFormat::Hdr)
                .map_err(err)
        }
    }
}

/// A picture file as linear RGBA floats. EXR and HDR are read as they are;
/// the other formats are taken as sRGB and linearised, so the viewer's
/// sRGB view shows them as they were saved.
pub fn load_image(path: &Path) -> Result<(u32, u32, Vec<f32>), String> {
    let name = path.display();
    let reader =
        crate::image_decode::reader(path).map_err(|e| format!("Can't read {name}: {e}"))?;
    let linear = matches!(
        reader.format(),
        Some(image::ImageFormat::OpenExr | image::ImageFormat::Hdr)
    );
    let image = reader
        .decode()
        .map_err(|e| format!("Can't read {name} as a picture: {e}"))?;
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 {
        return Err(format!("{name} has no pixels"));
    }
    let mut rgba = image.into_rgba32f().into_raw();
    if !linear {
        for p in rgba.as_chunks_mut::<4>().0 {
            for c in &mut p[..3] {
                *c = srgb_to_linear(*c);
            }
        }
    }
    Ok((w, h, rgba))
}

// ---------------------------------------------------------------------------
// Preferences and the window's place.

/// Preferences > Rendering > Render Window (IV-12, IV-13, IV-14 and the
/// Pictures choice Jason made). Saved per user.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewerPreferences {
    version: u8,
    /// Recent Frames Folder; the app's own data folder when `None`.
    pub recent_frames_folder: Option<PathBuf>,
    /// Max Render Slots: renders kept (loaded images don't count).
    pub max_render_slots: u32,
    /// Also save every render to ~/Pictures/Operandi Renders. Off: renders
    /// stay in the cache and are saved with Save Image... (Jason).
    pub save_to_pictures: bool,
    /// Default View Colourspace: the transform a new window starts with.
    pub default_view: ViewTransform,
}
impl Default for ViewerPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            recent_frames_folder: None,
            max_render_slots: 10,
            save_to_pictures: false,
            default_view: ViewTransform::Srgb,
        }
    }
}
impl ViewerPreferences {
    fn valid(&self) -> bool {
        self.version == 1 && MAX_SLOTS_RANGE.contains(&self.max_render_slots)
    }
    pub fn load(path: &Path) -> Self {
        read_small(path)
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid render window preference",
            ));
        }
        write_json(path, self)
    }
    /// The cache folder: the preference, else `default`.
    pub fn folder(&self, default: &Path) -> PathBuf {
        self.recent_frames_folder
            .clone()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| default.to_path_buf())
    }
}

/// The side panel's tabs (IV-35b; Compare comes later).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    ImageProcessing,
    Compare,
    #[default]
    Statistics,
}
impl Tab {
    pub const ALL: [Self; 3] = [Self::ImageProcessing, Self::Compare, Self::Statistics];
    pub fn label(self) -> &'static str {
        match self {
            Self::ImageProcessing => "Image Processing",
            Self::Compare => "Compare",
            Self::Statistics => "Statistics",
        }
    }
}

/// The Render window's place, panel sizes, visible tab and hidden panels
/// (UIC-73), and its display choices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSaved {
    version: u8,
    /// Inner size in points.
    pub size: [f32; 2],
    /// Outer position in points, once known.
    pub position: Option<[f32; 2]>,
    pub strip_width: f32,
    pub panel_width: f32,
    pub tab: Tab,
    pub show_strip: bool,
    pub show_panel: bool,
    /// Plain drag draws a render region (else it pans a zoomed picture).
    pub region_mode: bool,
    pub channels: Channels,
    pub show_clipping: bool,
    /// The last Save Image format.
    pub save_format: SaveFormat,
}
impl Default for WindowSaved {
    fn default() -> Self {
        Self {
            version: 1,
            size: [1000., 640.],
            position: None,
            strip_width: 132.,
            panel_width: 240.,
            tab: Tab::Statistics,
            show_strip: true,
            show_panel: true,
            region_mode: true,
            channels: Channels::Rgb,
            show_clipping: false,
            save_format: SaveFormat::Png8,
        }
    }
}
impl WindowSaved {
    fn valid(&self) -> bool {
        let finite = |v: &f32| v.is_finite();
        self.version == 1
            && self
                .size
                .iter()
                .all(|v| finite(v) && (100.0..=20_000.0).contains(v))
            && self.position.is_none_or(|p| p.iter().all(finite))
            && (40.0..=2000.0).contains(&self.strip_width)
            && (80.0..=2000.0).contains(&self.panel_width)
    }
    pub fn load(path: &Path) -> Self {
        read_small(path)
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        write_json(path, self)
    }
}

fn read_small(path: &Path) -> Option<Vec<u8>> {
    fs::metadata(path)
        .ok()
        .filter(|m| m.len() <= 64 * 1024)
        .and_then(|_| fs::read(path).ok())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    let _ = fs::remove_file(&temp);
    crate::resource_write::write_bytes(path, &temp, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "operandi-image-viewer-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A 4 × 2 picture with a value over 1 and a half-covered pixel.
    fn picture() -> (u32, u32, Vec<f32>) {
        let mut rgba = Vec::new();
        for i in 0..8 {
            let v = i as f32 / 8.0;
            rgba.extend_from_slice(&[v, 0.5 * v, 1.0 - v, 1.0]);
        }
        rgba[0] = 2.5; // over 1: clipped
        rgba[7] = 0.5; // half alpha
        (4, 2, rgba)
    }

    /// Adds a render slot with real files, as a finished render does.
    fn add_render(h: &mut History, max: u32) -> (u64, Vec<Slot>) {
        let (w, ht, rgba) = picture();
        let id = h.reserve_id();
        let slot = Slot {
            id,
            number: h.take_number(),
            width: w,
            height: ht,
            scene: "Scene".into(),
            time: now(),
            ..Default::default()
        };
        write_slot_files(&h.folder, &slot, &[&rgba], ViewTransform::Srgb).unwrap();
        (id, h.add(slot, max))
    }

    #[test]
    fn slots_number_in_render_order_and_keys_pick_strip_places() {
        let dir = temp("numbering");
        let mut h = History::empty(dir.clone());
        let ids: Vec<u64> = (0..12).map(|_| add_render(&mut h, 100).0).collect();
        // Newest first: the last render is place 1 and has number 12.
        assert_eq!(h.slots[0].id, ids[11]);
        assert_eq!(h.slots[0].number, 12);
        assert_eq!(h.at_position(1), Some(ids[11]));
        // Key 0 is the tenth place.
        assert_eq!(h.at_position(10), Some(ids[2]));
        assert_eq!(h.at_position(0), None);
        assert_eq!(h.at_position(13), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn trim_removes_the_oldest_unlocked_render_and_ignores_locks_when_all_locked() {
        let dir = temp("trim");
        let mut h = History::empty(dir.clone());
        let (a, _) = add_render(&mut h, 3);
        let (b, _) = add_render(&mut h, 3);
        let (c, _) = add_render(&mut h, 3);
        // A loaded image doesn't count against the cap.
        let loaded = h.reserve_id();
        let slot = Slot {
            id: loaded,
            kind: SlotKind::Loaded,
            width: 4,
            height: 2,
            ..Default::default()
        };
        write_slot_files(&dir, &slot, &[&picture().2], ViewTransform::Srgb).unwrap();
        assert!(h.add(slot, 3).is_empty());
        h.get_mut(a).unwrap().locked = true;
        // Over the cap: the oldest unlocked render (b) goes, files and all.
        let b_file = h.get(b).unwrap().output_file(&dir, 0);
        assert!(b_file.exists());
        let (d, removed) = add_render(&mut h, 3);
        assert_eq!(removed.iter().map(|s| s.id).collect::<Vec<_>>(), vec![b]);
        assert!(!b_file.exists());
        // Every render locked: the oldest goes anyway.
        for id in [a, c, d] {
            h.get_mut(id).unwrap().locked = true;
        }
        let (e, removed) = add_render(&mut h, 3);
        assert_eq!(removed.iter().map(|s| s.id).collect::<Vec<_>>(), vec![a]);
        assert!(h.get(e).is_some(), "the new render stays");
        assert!(h.get(loaded).is_some(), "loaded images are never trimmed");
        // The x leaves locked slots alone and removes unlocked ones.
        assert!(!h.remove(c));
        h.get_mut(c).unwrap().locked = false;
        let c_thumb = h.get(c).unwrap().thumb_file(&dir);
        assert!(h.remove(c));
        assert!(!c_thumb.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn the_history_survives_a_restart() {
        let dir = temp("persist");
        let mut h = History::empty(dir.clone());
        let (a, _) = add_render(&mut h, 10);
        let (b, _) = add_render(&mut h, 10);
        {
            let s = h.get_mut(a).unwrap();
            s.locked = true;
            s.stats = Stats {
                seconds: 3.5,
                samples: 64,
                samples_total: 128,
                triangles: 1234,
                camera: "Camera".into(),
                device: "CPU".into(),
                finished_early: true,
                ..Default::default()
            };
        }
        h.save_index().unwrap();
        let back = History::load(dir.clone());
        assert_eq!(back.slots, h.slots);
        // Numbering carries on after the restart.
        let mut back = back;
        assert_eq!(back.take_number(), 3);
        assert!(back.reserve_id() > b);
        // A slot whose picture went missing is dropped.
        fs::remove_file(h.get(b).unwrap().output_file(&dir, 0)).unwrap();
        let back = History::load(dir.clone());
        assert_eq!(back.slots.len(), 1);
        assert_eq!(back.slots[0].id, a);
        // The cached floats come back exactly.
        let pixels = read_output(&dir, &back.slots[0], 0).unwrap();
        assert_eq!(pixels, picture().2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clipping_shows_stripes_only_over_one() {
        let (w, _, rgba) = picture();
        assert!(clipped(&rgba[0..4]));
        assert!(!clipped(&rgba[4..8]));
        assert!(!clipped(&[1.0, 1.0, 1.0, 5.0]), "alpha doesn't clip");
        let on = display_rgba8(&rgba, w, ViewTransform::Srgb, Channels::Rgb, true);
        let off = display_rgba8(&rgba, w, ViewTransform::Srgb, Channels::Rgb, false);
        // Pixel 0 is a stripe (black at the origin), the rest unchanged.
        assert_eq!(&on[0..4], &[0, 0, 0, 255]);
        assert_eq!(&off[0..4], &[255, 0, 255, 255]);
        assert_eq!(&on[4..], &off[4..]);
        // Stripes alternate along the diagonal.
        let big = vec![2.0; 16 * 16 * 4];
        let s = display_rgba8(&big, 16, ViewTransform::Srgb, Channels::Rgb, true);
        let at = |x: usize, y: usize| s[(y * 16 + x) * 4];
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(4, 0), 255);
        assert_eq!(at(0, 4), 255);
        assert_eq!(at(4, 4), 0);
    }

    #[test]
    fn the_view_transform_and_channels_change_only_the_display() {
        let rgba = vec![0.5, 0.25, 0.0, 0.5];
        let before = rgba.clone();
        let srgb = display_rgba8(&rgba, 1, ViewTransform::Srgb, Channels::Rgb, false);
        let raw = display_rgba8(&rgba, 1, ViewTransform::Linear, Channels::Rgb, false);
        assert_eq!(srgb, vec![188, 137, 0, 128]);
        assert_eq!(raw, vec![128, 64, 0, 128]);
        assert_eq!(rgba, before, "the floats are untouched");
        let red = display_rgba8(&rgba, 1, ViewTransform::Linear, Channels::Red, false);
        assert_eq!(red, vec![128, 128, 128, 255]);
        let green = display_rgba8(&rgba, 1, ViewTransform::Linear, Channels::Green, false);
        assert_eq!(green, vec![64, 64, 64, 255]);
        let alpha = display_rgba8(&rgba, 1, ViewTransform::Srgb, Channels::Alpha, false);
        assert_eq!(alpha, vec![128, 128, 128, 255], "alpha is shown raw");
        // The two curves undo each other.
        for v in [0.0, 0.002, 0.18, 0.5, 1.0] {
            assert!((srgb_to_linear(linear_to_srgb(v)) - v).abs() < 1e-5);
        }
    }

    #[test]
    fn each_save_format_round_trips_through_load_image() {
        let dir = temp("formats");
        // Values inside 0–1 so integer formats can hold them, plus one
        // over 1 the float formats keep.
        let (w, h) = (4u32, 2u32);
        let mut rgba = Vec::new();
        for i in 0..8 {
            let v = (i as f32 + 0.5) / 8.0;
            rgba.extend_from_slice(&[v, v * 0.5, 1.0 - v, 1.0]);
        }
        let mut hdr = rgba.clone();
        hdr[0] = 3.0;
        let options = crate::file_io_preferences::ImageOptions::default();
        for format in SaveFormat::ALL {
            let source = if format.is_float() { &hdr } else { &rgba };
            let path = with_extension(&dir.join("picture"), format);
            assert_eq!(
                path.extension().unwrap().to_str().unwrap(),
                format.extension()
            );
            save_image(&path, source, w, h, format, ViewTransform::Srgb, options)
                .unwrap_or_else(|e| panic!("{format:?}: {e}"));
            let (bw, bh, back) = load_image(&path).unwrap_or_else(|e| panic!("{format:?}: {e}"));
            assert_eq!((bw, bh), (w, h), "{format:?}");
            // How close each format brings the floats back.
            let tolerance = match format {
                SaveFormat::Exr | SaveFormat::ExrMultilayer => 0.0,
                SaveFormat::Hdr => 0.02,
                SaveFormat::Png16 | SaveFormat::Tiff16 => 1e-4,
                SaveFormat::Jpeg => 0.08,
                _ => 0.01,
            };
            for (i, (a, b)) in source.iter().zip(&back).enumerate() {
                let tol = if format == SaveFormat::Hdr {
                    tolerance * a.abs().max(1.0)
                } else {
                    tolerance
                };
                assert!(
                    (a - b).abs() <= tol,
                    "{format:?} value {i}: saved {a}, read {b}"
                );
            }
            if format.is_float() {
                assert!(back[0] > 2.9, "{format:?} keeps values over 1");
            }
        }
        // A file with the format's own extension keeps it.
        assert_eq!(
            with_extension(Path::new("a.JPEG"), SaveFormat::Jpeg),
            PathBuf::from("a.JPEG")
        );
        assert_eq!(
            with_extension(Path::new("a.png"), SaveFormat::Exr),
            PathBuf::from("a.png.exr")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn preferences_and_window_place_round_trip() {
        let dir = temp("prefs");
        let d = ViewerPreferences::default();
        assert_eq!(d.max_render_slots, 10);
        assert!(!d.save_to_pictures, "renders stay in the cache (Jason)");
        assert_eq!(d.default_view, ViewTransform::Srgb);
        let p = ViewerPreferences {
            recent_frames_folder: Some(dir.join("frames")),
            max_render_slots: 4,
            save_to_pictures: true,
            default_view: ViewTransform::Linear,
            ..d.clone()
        };
        let path = dir.join("viewer.json");
        p.save(&path).unwrap();
        assert_eq!(ViewerPreferences::load(&path), p);
        assert_eq!(p.folder(Path::new("/x")), dir.join("frames"));
        assert_eq!(d.folder(Path::new("/x")), PathBuf::from("/x"));
        assert!(
            ViewerPreferences {
                max_render_slots: 0,
                ..p
            }
            .save(&path)
            .is_err()
        );
        let w = WindowSaved {
            size: [800., 500.],
            position: Some([10., 20.]),
            strip_width: 150.,
            panel_width: 300.,
            tab: Tab::Compare,
            show_strip: false,
            show_panel: false,
            region_mode: false,
            channels: Channels::Alpha,
            show_clipping: true,
            save_format: SaveFormat::Exr,
            ..Default::default()
        };
        let path = dir.join("window.json");
        w.save(&path).unwrap();
        assert_eq!(WindowSaved::load(&path), w);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn thumbnails_fit_the_strip_and_time_reads_as_a_date() {
        let rgba = vec![0.5; 400 * 200 * 4];
        let (w, h, px) = thumbnail(&rgba, 400, 200, ViewTransform::Linear);
        assert_eq!((w, h), (THUMB_SIDE, THUMB_SIDE / 2));
        assert_eq!(px.len(), (w * h * 4) as usize);
        assert_eq!(px[0], 128);
        let (w, h, _) = thumbnail(&rgba[..16], 2, 2, ViewTransform::Linear);
        assert_eq!((w, h), (2, 2), "never enlarged");
        assert_eq!(time_label(0), "1970-01-01 00:00 UTC");
        assert_eq!(time_label(1_794_000_000), "2026-11-06 21:20 UTC");
    }
}
