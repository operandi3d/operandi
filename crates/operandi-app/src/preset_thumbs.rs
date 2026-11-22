//! The preset browser's thumbnails (PB-106, PB-113, PB-114, CD-24).
//!
//! - Only entries the panel asks for (the visible ones) are decoded, on
//!   two background threads, newest request first, so scrolling a folder
//!   of 2,000 pictures never waits on a decode.
//! - Decoded thumbnails are kept in memory up to [`MEMORY_CAP`] (least
//!   recently shown dropped first) and on disk in the data folder's
//!   `Cache/Thumbnails`, keyed by file path + modified time + size, so a
//!   folder opens instantly the second time. Nothing is written next to
//!   the files themselves (CD-T5).
//! - Presets and texture sets without a picture of their own are rendered
//!   one at a time on a third thread: a material on the cutaway ball, an
//!   item framed, an environment on a mirror-ish ball (core
//!   `preset::ball`), small and with few samples. Until the render lands,
//!   a texture set shows its preview or base colour map (CD-24). Builds
//!   without Cycles show the picture or the kind's placeholder only.
//! - Renders wait while a scene render or preview is running.
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use operandi_core::preset::library::{EntryKind, ResolutionChoice, TextureSet};
use operandi_core::preset::{Preset, PresetKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};

/// Thumbnails are decoded to at most this many pixels across.
pub(crate) const PIXELS: u32 = 192;
/// Rendered thumbnails are this many pixels square.
pub(crate) const RENDER_PIXELS: u32 = 160;
/// Decoded thumbnails kept in memory (~150 KB each at most).
pub(crate) const MEMORY_CAP: usize = 600;
/// Waiting decode requests kept; older ones are dropped (and asked for
/// again if they scroll back into view).
const QUEUE_CAP: usize = 400;
/// Samples per pixel of a thumbnail render.
const RENDER_SAMPLES: u32 = 24;

/// One thumbnail's file stamp: what the disk cache is keyed by.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Stamp {
    pub path: PathBuf,
    pub modified_ns: u128,
    pub size: u64,
}

impl Stamp {
    pub(crate) fn of(path: &Path) -> Self {
        let meta = std::fs::metadata(path).ok();
        Self {
            path: path.to_path_buf(),
            modified_ns: meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos()),
            size: meta.map_or(0, |m| m.len()),
        }
    }
    /// The cache file name stem.
    pub(crate) fn key(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = operandi_core::fxhash::FxHasher::default();
        self.path.to_string_lossy().hash(&mut h);
        self.modified_ns.hash(&mut h);
        self.size.hash(&mut h);
        // HDR pictures changed tone mapping; a new key retires the old
        // washed-out thumbnails on disk without touching other entries.
        if is_hdr_path(&self.path) {
            HDR_THUMB_VERSION.hash(&mut h);
        }
        // A second, differently seeded pass makes clashes vanishingly rare.
        let first = h.finish();
        first.hash(&mut h);
        format!("{first:016x}{:016x}", h.finish())
    }
}

/// What to make a thumbnail of.
#[derive(Clone, Debug)]
pub(crate) struct Job {
    /// The entry's path: the memory key.
    pub key: PathBuf,
    pub kind: EntryKind,
    pub set: Option<TextureSet>,
    /// Write the render into the preset file's thumbnail slot (a preset
    /// just saved into the browser, PB-T15).
    pub embed: bool,
}

/// A finished picture for the main thread.
struct Done {
    key: PathBuf,
    picture: Option<ColorImage>,
    /// A rendered ball (or framing), not a decoded file picture.
    rendered: bool,
}

#[derive(Clone)]
enum Slot {
    Waiting,
    Ready {
        texture: TextureHandle,
        rendered: bool,
    },
    Failed,
}

type Queue<T> = Arc<(Mutex<Vec<T>>, Condvar)>;

pub(crate) struct Thumbs {
    slots: HashMap<PathBuf, (Slot, u64)>,
    tick: u64,
    decode: Queue<Job>,
    render: Queue<Job>,
    done_rx: Receiver<Done>,
    done_tx: Sender<Done>,
    /// Off while a scene render or preview runs (renders wait).
    pub render_allowed: Arc<AtomicBool>,
    pub resolution: Arc<Mutex<ResolutionChoice>>,
    cache: PathBuf,
    started: bool,
}

impl Thumbs {
    pub(crate) fn new(cache: PathBuf) -> Self {
        let (done_tx, done_rx) = channel();
        Self {
            slots: HashMap::new(),
            tick: 0,
            decode: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
            render: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
            done_rx,
            done_tx,
            render_allowed: Arc::new(AtomicBool::new(true)),
            resolution: Arc::new(Mutex::new(ResolutionChoice::default())),
            cache,
            started: false,
        }
    }

    /// Threads start on the first request, so tests and apps that never
    /// open the browser don't spawn any.
    fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        for n in 0..2 {
            let (queue, render, done, cache) = (
                self.decode.clone(),
                self.render.clone(),
                self.done_tx.clone(),
                self.cache.clone(),
            );
            let _ = std::thread::Builder::new()
                .name(format!("preset-thumbs-{n}"))
                .spawn(move || decode_worker(queue, render, done, cache));
        }
        if operandi_cycles::available() {
            let (queue, done, cache, allowed, resolution) = (
                self.render.clone(),
                self.done_tx.clone(),
                self.cache.clone(),
                self.render_allowed.clone(),
                self.resolution.clone(),
            );
            let _ = std::thread::Builder::new()
                .name("preset-thumb-render".into())
                .spawn(move || render_worker(queue, done, cache, allowed, resolution));
        }
    }

    /// The thumbnail for `job.key` if ready; otherwise it is asked for and
    /// `None` comes back (the panel shows the kind's placeholder).
    pub(crate) fn get(&mut self, job: &Job) -> Option<TextureHandle> {
        self.tick += 1;
        let tick = self.tick;
        if let Some((slot, seen)) = self.slots.get_mut(&job.key) {
            *seen = tick;
            return match slot {
                Slot::Ready { texture, .. } => Some(texture.clone()),
                _ => None,
            };
        }
        if !job.kind.has_picture() && !modo_file(&job.key) {
            return None;
        }
        self.start();
        self.slots.insert(job.key.clone(), (Slot::Waiting, tick));
        let (lock, wake) = &*self.decode;
        let mut queue = lock.lock().expect("thumbnail queue");
        queue.push(job.clone());
        if queue.len() > QUEUE_CAP {
            let dropped = queue.remove(0);
            self.slots.remove(&dropped.key);
        }
        wake.notify_one();
        None
    }

    /// Forget `key` (Refresh thumbnail, a replaced or renamed file): the
    /// next look decodes it again. `drop_disk` also clears its cached
    /// render so it is rendered afresh.
    pub(crate) fn forget(&mut self, key: &Path, drop_disk: bool) {
        self.slots.remove(key);
        if drop_disk {
            let stamp = Stamp::of(key).key();
            let _ = std::fs::remove_file(self.cache.join(format!("r-{stamp}.png")));
            let _ = std::fs::remove_file(self.cache.join(format!("d-{stamp}.png")));
            let _ = std::fs::remove_file(self.cache.join(format!("m-{stamp}.png")));
        }
    }

    /// Store `png` as `key`'s rendered thumbnail (Replace thumbnail with
    /// image file / last render, for files that aren't presets).
    pub(crate) fn store_rendered(&mut self, key: &Path, png: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.cache)?;
        let stamp = Stamp::of(key).key();
        std::fs::write(self.cache.join(format!("r-{stamp}.png")), png)?;
        self.slots.remove(key);
        Ok(())
    }

    /// Render a preset just saved into the browser and store the picture
    /// in the file (PB-T15, PB-52). Builds without Cycles skip it.
    pub(crate) fn render_into_file(&mut self, path: &Path, kind: PresetKind) {
        if !operandi_cycles::available() || kind == PresetKind::RenderSettings {
            return;
        }
        self.start();
        push(
            &self.render,
            Job {
                key: path.to_path_buf(),
                kind: EntryKind::Preset(kind),
                set: None,
                embed: true,
            },
        );
    }

    /// Drop every queued request (the folder changed).
    pub(crate) fn clear_queue(&mut self) {
        let (lock, _) = &*self.decode;
        for job in lock.lock().expect("thumbnail queue").drain(..) {
            self.slots.remove(&job.key);
        }
    }

    /// Take finished pictures onto the graphics card; returns whether any
    /// arrived (the panel repaints).
    pub(crate) fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut any = false;
        while let Ok(done) = self.done_rx.try_recv() {
            any = true;
            let tick = self.tick;
            match done.picture {
                Some(picture) => {
                    // A decoded picture never replaces a rendered one.
                    if !done.rendered
                        && matches!(
                            self.slots.get(&done.key),
                            Some((Slot::Ready { rendered: true, .. }, _))
                        )
                    {
                        continue;
                    }
                    let texture = ctx.load_texture(
                        format!("preset-thumb:{}", done.key.display()),
                        picture,
                        TextureOptions::LINEAR,
                    );
                    self.slots.insert(
                        done.key,
                        (
                            Slot::Ready {
                                texture,
                                rendered: done.rendered,
                            },
                            tick,
                        ),
                    );
                }
                None => {
                    if !matches!(self.slots.get(&done.key), Some((Slot::Ready { .. }, _))) {
                        self.slots.insert(done.key, (Slot::Failed, tick));
                    }
                }
            }
        }
        // The memory cap: forget the longest unseen.
        let ready = self
            .slots
            .values()
            .filter(|(s, _)| matches!(s, Slot::Ready { .. }))
            .count();
        if ready > MEMORY_CAP {
            let mut ticks: Vec<(u64, PathBuf)> = self
                .slots
                .iter()
                .filter(|(_, (s, _))| matches!(s, Slot::Ready { .. }))
                .map(|(k, (_, t))| (*t, k.clone()))
                .collect();
            ticks.sort();
            for (_, key) in ticks.into_iter().take(ready - MEMORY_CAP) {
                self.slots.remove(&key);
            }
        }
        any
    }

    /// Whether any work is outstanding (the panel shows its busy mark).
    pub(crate) fn busy(&self) -> bool {
        self.slots.values().any(|(s, _)| matches!(s, Slot::Waiting))
    }
}

fn pop<T>(queue: &Queue<T>) -> T {
    let (lock, wake) = &**queue;
    let mut q = lock.lock().expect("thumbnail queue");
    loop {
        if let Some(job) = q.pop() {
            return job;
        }
        q = wake.wait(q).expect("thumbnail queue");
    }
}

fn push<T>(queue: &Queue<T>, job: T) {
    let (lock, wake) = &**queue;
    lock.lock().expect("thumbnail queue").push(job);
    wake.notify_one();
}

fn decode_worker(queue: Queue<Job>, render: Queue<Job>, done: Sender<Done>, cache: PathBuf) {
    loop {
        let job = pop(&queue);
        let stamp = Stamp::of(&job.key).key();
        let rendered_file = cache.join(format!("r{RENDER_LOOK}-{stamp}.png"));
        if let Some(picture) = read_png_file(&rendered_file) {
            let _ = done.send(Done {
                key: job.key,
                picture: Some(picture),
                rendered: true,
            });
            continue;
        }
        // Matcaps are cached round (MC-01), apart from the same file's
        // square picture.
        let prefix = if job.kind == EntryKind::Matcap {
            "m"
        } else {
            "d"
        };
        let decoded_file = cache.join(format!("{prefix}-{stamp}.png"));
        let picture = read_png_file(&decoded_file).or_else(|| {
            let picture = decode(&job)?;
            if !matches!(job.kind, EntryKind::Preset(_)) {
                let _ = std::fs::create_dir_all(&cache);
                let _ = write_png(&decoded_file, &picture);
            }
            Some(picture)
        });
        let wants_render = match job.kind {
            EntryKind::Preset(PresetKind::RenderSettings) => false,
            // A preset with its own thumbnail keeps it.
            EntryKind::Preset(_) => picture.is_none(),
            EntryKind::TextureSet | EntryKind::MaterialX => true,
            // PG-05: scenes and models render (clay, three-quarter view).
            EntryKind::Scene | EntryKind::Model => picture.is_none(),
            // A mesh preset without a PRVW picture renders like a model.
            EntryKind::ModoMeshPreset => picture.is_none(),
            _ => false,
        };
        let failed = picture.is_none();
        let _ = done.send(Done {
            key: job.key.clone(),
            picture,
            rendered: false,
        });
        if wants_render && operandi_cycles::available() {
            push(&render, job);
        } else if failed {
            // Already reported as failed.
        }
    }
}

/// The entry's own picture: an image file, an HDR (tone mapped), a
/// texture set's preview or base colour (CD-24), or a preset's stored
/// thumbnail (PB-52).
fn decode(job: &Job) -> Option<ColorImage> {
    match job.kind {
        EntryKind::Preset(_) => {
            let header = operandi_core::persistence::read_preset_header(&job.key).ok()?;
            let png = operandi_core::persistence::preset_thumbnail(&header)?;
            read_png(&png)
        }
        EntryKind::Image | EntryKind::Environment => picture_file(&job.key),
        EntryKind::Matcap => matcap_picture(&job.key),
        EntryKind::TextureSet => picture_file(job.set.as_ref()?.representative()),
        // A MaterialX file shows its texture set's picture until rendered.
        EntryKind::MaterialX => picture_file(job.set.as_ref()?.representative()),
        // MC-30: a Modo scene or preset shows the PNG in its PRVW chunk.
        _ if modo_file(&job.key) => read_png(&operandi_core::preset::modo::preview_png(&job.key)?),
        _ => None,
    }
}

/// A Modo scene or preset, whose own preview the browser shows (MC-30).
fn modo_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["lxo", "lxl", "lxp", "lxe"]
            .iter()
            .any(|m| e.eq_ignore_ascii_case(m))
    })
}

/// Bumped when HDR thumbnails are drawn differently, so cached ones redo.
const HDR_THUMB_VERSION: u32 = 3;

fn is_hdr_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("hdr") || e.eq_ignore_ascii_case("exr"))
}

/// Linear HDR RGBA (f32) to display sRGB RGBA8 for a thumbnail.
///
/// Auto exposure puts the picture's log-average luminance at 0.09 (a
/// stop under mid grey; at 0.18 skies and studios still read washed
/// out), so dim interiors and bright skies both land in range; the ACES
/// filmic fit (Narkowicz 2015) then rolls highlights off with contrast,
/// and the sRGB transfer curve encodes it. A fixed exposure with plain
/// Reinhard and a 2.2 gamma left these grey and flat.
pub(crate) fn tone_map_hdr(rgba: &[f32]) -> Vec<u8> {
    let pixels = rgba.as_chunks::<4>().0;
    let luminance = |p: &[f32; 4]| (0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]).max(0.);
    let (sum, count) = pixels
        .iter()
        .map(luminance)
        .filter(|l| l.is_finite())
        .fold((0f64, 0usize), |(s, n), l| {
            (s + f64::from(l + 1e-4).ln(), n + 1)
        });
    let log_average = if count == 0 {
        0.18
    } else {
        (sum / count as f64).exp() as f32
    };
    let exposure = (0.09 / log_average.max(1e-6)).clamp(1e-4, 1e4);
    let aces = |x: f32| (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
    let encode = |c: f32| {
        let c = if c.is_finite() { c.max(0.) } else { 0. };
        let l = aces(c * exposure).clamp(0., 1.);
        let s = if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1. / 2.4) - 0.055
        };
        (s * 255. + 0.5) as u8
    };
    pixels
        .iter()
        .flat_map(|p| [encode(p[0]), encode(p[1]), encode(p[2]), 255])
        .collect()
}

/// Linear float RGBA shrunk to fit `side` (aspect kept, never enlarged)
/// by averaging each output pixel's block of source pixels.
fn shrink_linear(picture: &image::Rgba32FImage, side: u32) -> (u32, u32, Vec<f32>) {
    let (sw, sh) = picture.dimensions();
    let scale = (f64::from(side) / f64::from(sw.max(sh).max(1))).min(1.);
    let w = ((f64::from(sw) * scale).round() as u32).max(1);
    let h = ((f64::from(sh) * scale).round() as u32).max(1);
    let src = picture.as_raw();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let (y0, y1) = (y * sh / h, ((y + 1) * sh / h).max(y * sh / h + 1));
        for x in 0..w {
            let (x0, x1) = (x * sw / w, ((x + 1) * sw / w).max(x * sw / w + 1));
            let mut sum = [0f64; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = ((sy * sw + sx) * 4) as usize;
                    for (c, total) in sum.iter_mut().enumerate() {
                        let v = src[i + c];
                        // A stray NaN or infinity mustn't spoil the block.
                        *total += if v.is_finite() { f64::from(v) } else { 0. };
                    }
                }
            }
            let n = f64::from((x1 - x0) * (y1 - y0));
            out.extend(sum.map(|t| (t / n) as f32));
        }
    }
    (w, h, out)
}

/// A picture file shrunk to [`PIXELS`]; HDR and EXR are tone mapped
/// ([`tone_map_hdr`]) so bright skies don't burn out.
pub(crate) fn picture_file(path: &Path) -> Option<ColorImage> {
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let hdr = matches!(
        reader.format(),
        Some(image::ImageFormat::OpenExr | image::ImageFormat::Hdr)
    );
    let picture = reader.decode().ok()?;
    let rgba = if hdr {
        // Not `thumbnail`: its box average rounds as if pixels were
        // integers and adds 0.5 to every float value, which lifts an
        // HDR's blacks to a grey haze no tone mapping can undo.
        let (w, h, small) = shrink_linear(&picture.into_rgba32f(), PIXELS);
        image::RgbaImage::from_raw(w, h, tone_map_hdr(&small))?
    } else {
        picture.thumbnail(PIXELS, PIXELS).into_rgba8()
    };
    let (w, h) = rgba.dimensions();
    Some(ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        rgba.as_raw(),
    ))
}

/// A matcap's tile (MC-01): its centred square as the Matcap style reads
/// it (MC-07), cut to its circle so the panel shows round, with a
/// one-pixel soft edge.
pub(crate) fn matcap_picture(path: &Path) -> Option<ColorImage> {
    let bytes = std::fs::read(path).ok()?;
    let mut rgba = crate::matcap::decode(&bytes).ok()?.rgba;
    let n = crate::matcap::SIZE as usize;
    let r = n as f32 / 2.;
    for y in 0..n {
        for x in 0..n {
            let d = ((x as f32 + 0.5 - r).powi(2) + (y as f32 + 0.5 - r).powi(2)).sqrt();
            let alpha = (r - d).clamp(0., 1.);
            rgba[(y * n + x) * 4 + 3] = (alpha * 255. + 0.5) as u8;
        }
    }
    Some(ColorImage::from_rgba_unmultiplied([n, n], &rgba))
}

fn read_png(bytes: &[u8]) -> Option<ColorImage> {
    let picture = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = picture.dimensions();
    Some(ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        picture.as_raw(),
    ))
}

fn read_png_file(path: &Path) -> Option<ColorImage> {
    read_png(&std::fs::read(path).ok()?)
}

/// PNG bytes of a picture.
pub(crate) fn png_bytes(picture: &ColorImage) -> Option<Vec<u8>> {
    let [w, h] = picture.size;
    let rgba: Vec<u8> = picture
        .pixels
        .iter()
        .flat_map(|c| c.to_srgba_unmultiplied())
        .collect();
    let image = image::RgbaImage::from_raw(w as u32, h as u32, rgba)?;
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

fn write_png(path: &Path, picture: &ColorImage) -> Option<()> {
    std::fs::write(path, png_bytes(picture)?).ok()
}

fn render_worker(
    queue: Queue<Job>,
    done: Sender<Done>,
    cache: PathBuf,
    allowed: Arc<AtomicBool>,
    resolution: Arc<Mutex<ResolutionChoice>>,
) {
    loop {
        let job = pop(&queue);
        while !allowed.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        let choice = *resolution.lock().expect("resolution");
        let picture = preview_preset(&job, choice).and_then(|p| render_preview(&p));
        let Some(picture) = picture else { continue };
        if job.embed
            && let Some(png) = png_bytes(&picture)
        {
            // The file changes, so its stamp does: the decode path finds
            // the picture in the file from now on.
            let _ = operandi_core::persistence::set_preset_thumbnail(&job.key, Some(&png));
        }
        let stamp = Stamp::of(&job.key).key();
        let _ = std::fs::create_dir_all(&cache);
        let _ = write_png(&cache.join(format!("r{RENDER_LOOK}-{stamp}.png")), &picture);
        let _ = done.send(Done {
            key: job.key,
            picture: Some(picture),
            rendered: true,
        });
    }
}

/// The preset a thumbnail renders: the file, or a texture set's material.
fn preview_preset(job: &Job, choice: ResolutionChoice) -> Option<Preset> {
    match job.kind {
        EntryKind::Preset(_) => operandi_core::persistence::load_preset(&job.key).ok(),
        EntryKind::TextureSet => {
            // Thumbnails use the lowest resolution: same look, less to read.
            let _ = choice;
            job.set
                .as_ref()?
                .material_preset(ResolutionChoice::Lowest, &|p| {
                    image::image_dimensions(p).ok()
                })
                .ok()
                .map(|(p, _)| p)
        }
        EntryKind::MaterialX => {
            let size = |p: &std::path::Path| image::image_dimensions(p).ok();
            let (mut preset, _) =
                operandi_core::materialx::material_preset(&job.key, None, &size).ok()?;
            if let Some(set) = &job.set {
                set.add_ambient_occlusion(&mut preset, ResolutionChoice::Lowest, &size)
                    .ok()?;
            }
            Some(preset)
        }
        // PG-05: the file's meshes in the grey clay look (no materials,
        // lights or cameras of its own), framed like an item preset.
        EntryKind::Scene | EntryKind::Model | EntryKind::ModoMeshPreset => {
            const MAX_BYTES: u64 = 256 << 20;
            if std::fs::metadata(&job.key).ok()?.len() > MAX_BYTES {
                return None;
            }
            let mut scene = if job.kind == EntryKind::ModoMeshPreset {
                let mut go = |_: &str| true;
                let mut control = operandi_core::lxo::Control::new(&mut go);
                operandi_core::lxo::load_mesh_preset(&job.key, &mut control)
                    .ok()?
                    .scene
            } else {
                crate::import_into::read_scene_file(&job.key).ok()?.0
            };
            crate::import_into::filter_import(
                &mut scene,
                crate::import_into::ImportKinds {
                    meshes: true,
                    shaders: false,
                    lights: false,
                    environments: false,
                    cameras: false,
                },
            );
            Some(Preset {
                header: operandi_core::preset::PresetHeader {
                    kind: PresetKind::Item,
                    thumbnail: None,
                    source: None,
                },
                scene,
            })
        }
        _ => None,
    }
}

/// Bumped when the preview look changes, so cached renders from an
/// older look are made again (the disk cache's `r<N>-` file prefix).
/// 2: the slanted small-hole ball over a gradient (PB-114, 2026-11-11).
pub(crate) const RENDER_LOOK: u32 = 2;

/// The backdrop's linear grey at the top and bottom of the picture: a
/// soft studio sweep, lighter above (sRGB about 0.80 and 0.56).
const BACKDROP_TOP: f32 = 0.60;
const BACKDROP_BOTTOM: f32 = 0.27;

/// A soft square area light at `position` shining at `target`.
fn soft_light(
    position: glam::DVec3,
    target: glam::DVec3,
    size: f32,
    watts: f32,
) -> operandi_cycles::SceneLight {
    use glam::DVec3;
    let direction = (target - position).normalize();
    let right = direction.cross(DVec3::Y).normalize();
    let up = right.cross(direction);
    operandi_cycles::SceneLight {
        shape: operandi_cycles::LightShape::Area {
            size: [size, size],
            ellipse: false,
        },
        position,
        right,
        up,
        direction,
        strength: [watts; 3],
        cast_shadow: true,
        visible_to_camera: false,
        affects_diffuse: true,
        affects_specular: true,
        // The key's square shows as the gloss highlight.
        visible_to_reflection: true,
        visible_to_refraction: true,
        ies: None,
        lightgroup: None,
        layers: Vec::new(),
        gobo_tan_half: 0.,
        light_membership: None,
        shadow_membership: None,
        caustics_only: false,
    }
}

/// Render a preset's preview scene (PB-113/114): the subject over a clear
/// background (the floor only catches its contact shadow), lit by a big
/// soft key from the upper left and a dimmer fill from the right, then
/// laid over a soft grey vertical gradient with no floor line.
/// Environment presets light the ball themselves.
pub(crate) fn render_preview(preset: &Preset) -> Option<ColorImage> {
    use glam::DVec3;
    let (scene, framing) = operandi_core::preset::ball::preview_scene(preset).ok()?;
    let forward = (framing.target - framing.eye).normalize();
    let right = forward.cross(DVec3::Y).normalize();
    let up = right.cross(forward);
    let view = operandi_cycles::View::perspective(
        framing.eye,
        right,
        up,
        forward,
        framing.half_height_tan,
    );
    let mut settings = operandi_cycles::Settings::studio(RENDER_PIXELS, RENDER_PIXELS, view);
    settings.samples = RENDER_SAMPLES;
    settings.transparent = true;
    let reach = (framing.eye - framing.target).length();
    let key_at =
        framing.target + (-right * 0.9 + up * 1.0 + -forward * 0.8).normalize() * reach * 0.7;
    let fill_at = framing.target + (right * 1.0 + up * 0.15 + -forward * 0.6).normalize() * reach;
    settings.lights = Some(vec![
        soft_light(key_at, framing.target, (reach * 0.35) as f32, KEY_WATTS),
        soft_light(fill_at, framing.target, (reach * 0.5) as f32, FILL_WATTS),
    ]);
    settings.world = vec![operandi_cycles::EnvironmentLayer::flat(
        [0.45, 0.45, 0.46],
        WORLD_STRENGTH,
    )];
    if preset.header.kind == PresetKind::Environment {
        settings.lights = None;
        settings = settings.with_scene_world(&scene).with_scene_lights(&scene);
    }
    let meshes = operandi_cycles::scene_meshes(&scene).ok()?;
    let mut image = operandi_cycles::render(&meshes, &settings).ok()?;
    over_backdrop(&mut image.rgba, image.width as usize, image.height as usize);
    let bytes = image.to_srgb8();
    Some(ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &bytes,
    ))
}

const KEY_WATTS: f32 = 80.;
const FILL_WATTS: f32 = 22.;
const WORLD_STRENGTH: f32 = 0.3;

/// Lays premultiplied linear RGBA over the backdrop gradient, leaving the
/// picture opaque.
fn over_backdrop(rgba: &mut [f32], width: usize, height: usize) {
    for (i, p) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let row = i / width.max(1);
        let t = row as f32 / (height.max(2) - 1) as f32;
        let back = BACKDROP_TOP + (BACKDROP_BOTTOM - BACKDROP_TOP) * t;
        let a = p[3].clamp(0., 1.);
        for c in &mut p[..3] {
            *c += back * (1. - a);
        }
        p[3] = 1.;
    }
}

/// What a bulk re-render did.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Rerendered {
    /// Material presets whose embedded picture was replaced.
    pub presets: usize,
    /// `<Name>.png` files next to them that were replaced too.
    pub pngs: usize,
    /// Files that couldn't be read, rendered or written, with why.
    pub failed: Vec<(PathBuf, String)>,
}

/// Re-renders every material preset under `root` (all subfolders) with the
/// current preview look: the picture stored inside each `.oppreset`, and a
/// `<Name>.png` beside it when there is one. Other preset kinds are left
/// alone. `render` makes the picture (the app passes [`render_preview`];
/// tests pass a stand-in). `progress` hears each file as it's done.
pub(crate) fn rerender_material_thumbnails(
    root: &Path,
    render: &dyn Fn(&Preset) -> Option<ColorImage>,
    progress: &mut dyn FnMut(&Path),
) -> std::io::Result<Rerendered> {
    let mut files = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(operandi_core::preset::is_preset_extension)
            {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut done = Rerendered::default();
    for path in files {
        let fail = |why: String| (path.clone(), why);
        let preset = match operandi_core::persistence::load_preset(&path) {
            Ok(p) => p,
            Err(e) => {
                done.failed.push(fail(e.to_string()));
                continue;
            }
        };
        if preset.header.kind != PresetKind::Material {
            continue;
        }
        let Some(png) = render(&preset).and_then(|p| png_bytes(&p)) else {
            done.failed.push(fail("render failed".into()));
            continue;
        };
        if let Err(e) = operandi_core::persistence::set_preset_thumbnail(&path, Some(&png)) {
            done.failed.push(fail(e.to_string()));
            continue;
        }
        done.presets += 1;
        let beside = path.with_extension("png");
        if beside.is_file() {
            match std::fs::write(&beside, &png) {
                Ok(()) => done.pngs += 1,
                Err(e) => done.failed.push((beside, e.to_string())),
            }
        }
        progress(&path);
    }
    Ok(done)
}

/// `operandi-app --rerender-thumbnails <folder>`: [`rerender_material_thumbnails`]
/// with Cycles, from the command line. Returns the process exit code.
pub(crate) fn rerender_command(args: &[String]) -> i32 {
    let Some(folder) = args.first() else {
        eprintln!("usage: operandi-app --rerender-thumbnails <folder>");
        return 2;
    };
    if !operandi_cycles::available() {
        eprintln!("This build has no Cycles renderer (build with --features cycles)");
        return 1;
    }
    let root = PathBuf::from(folder);
    let mut count = 0;
    let result = rerender_material_thumbnails(&root, &render_preview, &mut |p| {
        count += 1;
        println!("{count:>5}  {}", p.display());
    });
    match result {
        Ok(done) => {
            for (path, why) in &done.failed {
                eprintln!("failed: {}: {why}", path.display());
            }
            println!(
                "Re-rendered {} material presets and {} PNGs; {} failed",
                done.presets,
                done.pngs,
                done.failed.len()
            );
            i32::from(!done.failed.is_empty())
        }
        Err(e) => {
            eprintln!("Couldn't read {}: {e}", root.display());
            1
        }
    }
}

/// Writes every starter mesh (core `preset::starter_meshes`) below
/// `folder` as `<Category>/<Name>.oppreset`, each with its thumbnail from
/// `render` stored inside (the app passes [`render_preview`]: the item on
/// the browser's preview stage). Returns the files written.
pub(crate) fn write_starter_meshes(
    folder: &Path,
    render: &dyn Fn(&Preset) -> Option<ColorImage>,
) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for shape in &operandi_core::preset::starter_meshes::ALL {
        let preset = shape.preset()?;
        let picture = render(&preset).ok_or_else(|| format!("{}: render failed", shape.name))?;
        let png = png_bytes(&picture).ok_or_else(|| format!("{}: PNG failed", shape.name))?;
        let path = folder.join(shape.relative_path());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        operandi_core::persistence::save_preset(&path, &preset)?;
        operandi_core::persistence::set_preset_thumbnail(&path, Some(&png))?;
        out.push(path);
    }
    Ok(out)
}

/// `operandi-app --write-starter-meshes <folder> [<previews folder>]`:
/// [`write_starter_meshes`] with Cycles; with a second folder, each
/// thumbnail is also saved there as `<Name>.png` (for a contact sheet; the
/// pack doesn't ship them). Returns the process exit code.
pub(crate) fn write_starter_meshes_command(args: &[String]) -> i32 {
    let Some(folder) = args.first() else {
        eprintln!("usage: operandi-app --write-starter-meshes <folder> [<previews folder>]");
        return 2;
    };
    if !operandi_cycles::available() {
        eprintln!("This build has no Cycles renderer (build with --features cycles)");
        return 1;
    }
    match write_starter_meshes(Path::new(folder), &render_preview) {
        Ok(paths) => {
            for p in &paths {
                println!("{}", p.display());
                if let Some(previews) = args.get(1) {
                    let png = operandi_core::persistence::load_preset(p)
                        .ok()
                        .and_then(|preset| {
                            operandi_core::persistence::preset_thumbnail(&preset.header)
                        });
                    let out = Path::new(previews)
                        .join(p.with_extension("png").file_name().unwrap_or_default());
                    if let Some(png) = png
                        && std::fs::create_dir_all(previews).is_ok()
                        && std::fs::write(&out, png).is_ok()
                    {
                        continue;
                    }
                    eprintln!("couldn't save the preview {}", out.display());
                    return 1;
                }
            }
            println!("Wrote {} starter meshes", paths.len());
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pack's mesh presets: one file per shape, its thumbnail inside.
    #[test]
    fn starter_meshes_are_written_with_their_thumbnails() {
        let dir =
            std::env::temp_dir().join(format!("operandi-starter-meshes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let grey = |_: &Preset| Some(ColorImage::new([4, 4], vec![egui::Color32::GRAY; 16]));
        let paths = write_starter_meshes(&dir, &grey).unwrap();
        assert_eq!(
            paths.len(),
            operandi_core::preset::starter_meshes::ALL.len()
        );
        for path in &paths {
            let preset = operandi_core::persistence::load_preset(path).unwrap();
            assert_eq!(preset.header.kind, PresetKind::Item);
            assert!(preset.header.thumbnail.is_some(), "{}", path.display());
        }
        assert!(dir.join("Furniture/Table.oppreset").is_file());
        std::fs::remove_dir_all(dir).ok();
    }

    /// PG-05: a model file previews as its meshes alone, as an item
    /// preset (rendered in the clay look).
    #[test]
    fn pg_05_a_model_previews_as_its_meshes() {
        let dir = std::env::temp_dir().join(format!("operandi-thumb-model-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let obj = dir.join("tri.obj");
        std::fs::write(&obj, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let job = Job {
            key: obj.clone(),
            kind: EntryKind::Model,
            set: None,
            embed: false,
        };
        let preset = preview_preset(&job, ResolutionChoice::Lowest).unwrap();
        assert_eq!(preset.header.kind, PresetKind::Item);
        assert!(
            preset
                .scene
                .items
                .iter()
                .all(|i| i.light.is_none() && i.camera.is_none())
        );
        assert_eq!(preset.scene.items.iter().filter(|i| i.is_mesh()).count(), 1);
        assert!(operandi_core::preset::ball::item_scene(&preset).is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_cache_key_follows_path_time_and_size() {
        let a = Stamp {
            path: "/a.png".into(),
            modified_ns: 1,
            size: 2,
        };
        let mut b = a.clone();
        assert_eq!(a.key(), b.key());
        b.size = 3;
        assert_ne!(a.key(), b.key());
        b = a.clone();
        b.modified_ns = 9;
        assert_ne!(a.key(), b.key());
    }

    #[test]
    fn pictures_decode_small_in_the_background_and_hit_the_disk_cache() {
        let dir = std::env::temp_dir().join(format!("operandi-thumbs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let picture = dir.join("big.png");
        image::RgbaImage::from_pixel(600, 300, image::Rgba([200, 10, 10, 255]))
            .save(&picture)
            .unwrap();
        let cache = dir.join("cache");
        let ctx = egui::Context::default();
        let mut thumbs = Thumbs::new(cache.clone());
        let job = Job {
            key: picture.clone(),
            kind: EntryKind::Image,
            set: None,
            embed: false,
        };
        assert!(thumbs.get(&job).is_none(), "asked for, not ready yet");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let texture = loop {
            thumbs.poll(&ctx);
            if let Some(t) = thumbs.get(&job) {
                break t;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "thumbnail never arrived"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(texture.size(), [192, 96]);
        // The decoded copy is on disk, keyed by the file's stamp, and the
        // content folder holds nothing new (CD-T5).
        let key = Stamp::of(&picture).key();
        assert!(cache.join(format!("d-{key}.png")).is_file());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The bulk re-render replaces material presets' embedded pictures and
    /// their `<Name>.png`, walks subfolders, and leaves other kinds alone.
    #[test]
    fn rerender_replaces_material_pictures_and_pngs_beside_them() {
        use operandi_core::preset::PresetSource;
        use operandi_core::shader_tree::NewLayer;
        let dir = std::env::temp_dir().join(format!("operandi-rerender-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Metal/Gold")).unwrap();
        let mut e = operandi_core::Editor::default();
        let group = e.scene.shader_tree.add(NewLayer::Group, None).unwrap();
        e.scene
            .shader_tree
            .add(NewLayer::Material, Some(group))
            .unwrap();
        let mut material = PresetSource::Material(group)
            .build(&e.scene)
            .unwrap()
            .preset;
        // A map beside the preset, named relative to it as packs do.
        material.scene.images = vec![operandi_core::backdrop::SceneImage {
            id: 1,
            name: "Gold_Color".into(),
            path: "Gold_Color.jpg".into(),
            width: 4,
            height: 4,
        }];
        let gold = dir.join("Metal/Gold/Gold.oppreset");
        let plain = dir.join("Plain.oppreset");
        operandi_core::persistence::save_preset(&gold, &material).unwrap();
        operandi_core::persistence::save_preset(&plain, &material).unwrap();
        std::fs::write(dir.join("Metal/Gold/Gold.png"), b"old").unwrap();
        let red = ColorImage::new([4, 4], vec![egui::Color32::RED; 16]);
        let mut seen = Vec::new();
        let done = rerender_material_thumbnails(&dir, &|_| Some(red.clone()), &mut |p| {
            seen.push(p.to_path_buf())
        })
        .unwrap();
        assert_eq!((done.presets, done.pngs, done.failed.len()), (2, 1, 0));
        assert_eq!(seen.len(), 2);
        let png = png_bytes(&red).unwrap();
        assert_eq!(std::fs::read(dir.join("Metal/Gold/Gold.png")).unwrap(), png);
        let header = operandi_core::persistence::read_preset_header(&gold).unwrap();
        assert_eq!(
            operandi_core::persistence::preset_thumbnail(&header),
            Some(png)
        );
        let text = std::fs::read_to_string(&gold).unwrap();
        assert!(
            text.contains("\"path\": \"Gold_Color.jpg\"")
                && !text.contains(&*dir.to_string_lossy()),
            "the re-render keeps the map relative"
        );
        assert!(
            !dir.join("Plain.png").exists(),
            "no PNG made where none was"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Opt-in look at the cutaway ball (PB-114), with Cycles:
    /// `OPERANDI_BALL_OUT=/tmp/ball.png cargo test --features cycles -p
    /// operandi-app cutaway_ball_render -- --ignored`. Writes the red
    /// glossy ball there, and `-metal` and `-rough` beside it.
    #[test]
    #[ignore]
    fn cutaway_ball_render() {
        use operandi_core::shader_tree::{NewLayer, NodeKind};
        let out =
            std::env::var("OPERANDI_BALL_OUT").unwrap_or_else(|_| "/tmp/operandi-ball.png".into());
        let out = std::path::PathBuf::from(out);
        let stem = out.with_extension("");
        for (suffix, color, roughness, metallic) in [
            ("", [0.75, 0.06, 0.04], 0.12, 0.),
            ("-metal", [0.9, 0.78, 0.55], 0.2, 1.),
            ("-rough", [0.35, 0.5, 0.65], 0.85, 0.),
        ] {
            let mut e = operandi_core::Editor::default();
            let group = e.scene.shader_tree.add(NewLayer::Group, None).unwrap();
            let material = e
                .scene
                .shader_tree
                .add(NewLayer::Material, Some(group))
                .unwrap();
            let mut node = e.scene.shader_tree.get(material).unwrap().clone();
            node.parent = Some(group);
            if let NodeKind::Material(m) = &mut node.kind {
                m.diffuse_color = color;
                m.roughness = roughness;
                m.metallic = metallic;
            }
            e.scene.shader_tree.replace(node).unwrap();
            let preset = operandi_core::preset::PresetSource::Material(group)
                .build(&e.scene)
                .unwrap()
                .preset;
            let picture = render_preview(&preset).expect("rendered");
            let path = format!("{}{suffix}.png", stem.display());
            std::fs::write(&path, png_bytes(&picture).unwrap()).unwrap();
        }
    }

    /// HDR thumbnails expose themselves: the same sky at any brightness
    /// gives the same picture, mid grey lands mid-tone, and a bright
    /// value keeps its saturation instead of washing to grey.
    #[test]
    fn hdr_thumbnails_auto_expose_and_keep_contrast() {
        let sky: Vec<f32> = [
            [0.4, 0.6, 1.0, 1.0],
            [0.05, 0.04, 0.03, 1.0],
            [8.0, 7.5, 7.0, 1.0],
        ]
        .iter()
        .cycle()
        .take(30)
        .flatten()
        .copied()
        .collect();
        let bright: Vec<f32> = sky.iter().map(|c| c * 100.).collect();
        let a = tone_map_hdr(&sky);
        let b = tone_map_hdr(&bright);
        for (x, y) in a.iter().zip(&b) {
            assert!(x.abs_diff(*y) <= 1, "exposure should not depend on scale");
        }
        // Dark ground stays dark, the sun stays near white, the sky stays blue.
        assert!(a[4] < 80 && a[8] > 230, "{:?}", &a[..12]);
        assert!(a[2] > a[0] + 40, "sky keeps its blue: {:?}", &a[..4]);

        let grey = tone_map_hdr(&[0.18, 0.18, 0.18, 1.0]);
        assert!((80..=110).contains(&grey[0]), "mid grey {grey:?}");
        assert_eq!(tone_map_hdr(&[f32::NAN, f32::INFINITY, -1., 1.])[3], 255);
    }

    /// Regression: HDR thumbnails were shrunk with `image`'s
    /// `thumbnail`, which adds 0.5 to every float pixel, so dark rooms and
    /// studios came out washed out even after tone mapping. A real .hdr
    /// file, mostly black with a bright window, must keep its blacks.
    #[test]
    fn hdr_file_thumbnails_keep_their_blacks() {
        let (w, h) = (400u32, 200u32);
        let pixels: Vec<image::Rgb<f32>> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                if (180..220).contains(&x) && (80..120).contains(&y) {
                    image::Rgb([40., 38., 35.])
                } else if x < 200 {
                    image::Rgb([0.002, 0.002, 0.002])
                } else {
                    image::Rgb([0.05, 0.04, 0.03])
                }
            })
            .collect();
        let dir = std::env::temp_dir().join(format!("operandi-hdr-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("room.hdr");
        image::codecs::hdr::HdrEncoder::new(std::fs::File::create(&path).unwrap())
            .encode(&pixels, w as usize, h as usize)
            .unwrap();
        let thumb = picture_file(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(thumb.size, [PIXELS as usize, PIXELS as usize / 2]);
        let darkest = thumb.pixels.iter().map(|c| c.r()).min().unwrap();
        let brightest = thumb.pixels.iter().map(|c| c.r()).max().unwrap();
        // The old shrink put this near 100.
        assert!(darkest < 40, "blacks lifted to {darkest}");
        assert!(
            brightest > 230,
            "window should stay near white: {brightest}"
        );
    }

    #[test]
    fn shrink_linear_averages_blocks_without_bias() {
        let picture =
            image::Rgba32FImage::from_fn(4, 2, |x, _| image::Rgba([x as f32, 0., 0.001, 1.]));
        let (w, h, small) = shrink_linear(&picture, 2);
        assert_eq!((w, h), (2, 1));
        assert_eq!(&small[..4], &[0.5, 0., 0.001, 1.]);
        assert_eq!(&small[4..], &[2.5, 0., 0.001, 1.]);
    }

    #[test]
    fn hdr_paths_get_their_own_cache_version() {
        assert!(is_hdr_path(Path::new("a/Sky.HDR")) && is_hdr_path(Path::new("b.exr")));
        assert!(!is_hdr_path(Path::new("c.png")));
    }
}
