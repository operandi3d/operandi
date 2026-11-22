//! The Matcap display style's matcaps: a per-view choice (Viewport
//! Properties > Advanced > Matcap) between the matcaps built into the
//! app (Blender's, CC0; assets/studiolights) and any PNG/EXR image the user
//! loads. An Operandi extra (Jason, 2026-11-10).
//!
//! The choice is `Copy`, like the rest of a view's display settings: a
//! custom image is held by an id into a process-wide list of the paths
//! loaded so far, and is saved by its path (MC-04, MC-08).
//!
//! The preset browser's matcaps (MATCAPS MC-01..11) are custom images
//! too. The graphics card keeps `MAX_CUSTOM` of them at once in slots
//! handed out least-recently-used first, so browsing a big library never
//! runs out. A missing file draws Clay Studio (MC-08).
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// The scene group binding of the matcap texture array, after the image
/// layers' (material_gpu.rs).
pub const BINDING: u32 =
    crate::material_gpu::FIRST_IMAGE + operandi_core::shader_wgsl::MAX_IMAGES as u32;
/// Pixels per side of every matcap layer (built-in and loaded).
pub const SIZE: u32 = 256;
/// How far out the lookup reaches (a touch inside the disc's edge, where
/// the image blends into its background).
pub const REACH: f32 = 0.97;

/// The built-in matcaps: saved key, menu label and the PNG (sRGB).
pub const BUILT_IN: [(&str, &str, &[u8]); 8] = [
    (
        "clay_studio",
        "Clay Studio",
        include_bytes!("../assets/studiolights/clay_studio.png"),
    ),
    (
        "basic_grey",
        "Basic Grey",
        include_bytes!("../assets/studiolights/basic_grey.png"),
    ),
    (
        "pearl",
        "Pearl",
        include_bytes!("../assets/studiolights/pearl.png"),
    ),
    (
        "resin",
        "Resin",
        include_bytes!("../assets/studiolights/resin.png"),
    ),
    (
        "ceramic_lightbulb",
        "Ceramic Lightbulb",
        include_bytes!("../assets/studiolights/ceramic_lightbulb.png"),
    ),
    (
        "metal_carpaint",
        "Metal Carpaint",
        include_bytes!("../assets/studiolights/metal_carpaint.png"),
    ),
    (
        "fullmetal",
        "Full Metal",
        include_bytes!("../assets/studiolights/fullmetal.png"),
    ),
    (
        "check_reflection_horizontal",
        "Check Reflection",
        include_bytes!("../assets/studiolights/check_reflection_horizontal.png"),
    ),
];
/// Custom images the GPU keeps at once (each its own layer). More than
/// this in use at once reuses the slot used longest ago.
pub const MAX_CUSTOM: usize = 16;
/// What a missing custom image draws with (MC-08).
pub const MISSING: &str = "clay_studio";
/// Layers in the GPU's matcap array: the built-ins, then the custom slots.
pub const LAYERS: u32 = (BUILT_IN.len() + MAX_CUSTOM) as u32;

/// Which matcap a view's Matcap style uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choice {
    /// Index into `BUILT_IN`.
    BuiltIn(u8),
    /// An image the user loaded: an id into the loaded-path list.
    Custom(u32),
}
/// The matcap new views start with, by key (Jason picked the metal one,
/// 2026-11-10). Views saved before matcaps existed load with it too.
pub const DEFAULT: &str = "fullmetal";

impl Default for Choice {
    fn default() -> Self {
        let i = BUILT_IN
            .iter()
            .position(|(k, ..)| *k == DEFAULT)
            .unwrap_or(0);
        Choice::BuiltIn(i as u8)
    }
}

/// The paths of the custom images chosen so far this run, by id.
fn paths() -> &'static Mutex<Vec<PathBuf>> {
    static PATHS: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    PATHS.get_or_init(|| Mutex::new(Vec::new()))
}

impl Choice {
    /// The custom image at `path` (the same id for the same path).
    pub fn custom(path: &Path) -> Self {
        let mut list = paths().lock().unwrap_or_else(|e| e.into_inner());
        let id = match list.iter().position(|p| p == path) {
            Some(i) => i,
            None => {
                list.push(path.to_path_buf());
                list.len() - 1
            }
        };
        Choice::Custom(id as u32)
    }
    /// A custom image's path.
    pub fn path(self) -> Option<PathBuf> {
        match self {
            Choice::Custom(id) => paths()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(id as usize)
                .cloned(),
            Choice::BuiltIn(_) => None,
        }
    }
    /// The Matcap menu's index (MC-05): a built-in, then the library's
    /// matcaps, then Custom Image (any other file).
    pub fn menu_index(self) -> usize {
        match self {
            Choice::BuiltIn(i) => usize::from(i).min(BUILT_IN.len() - 1),
            Choice::Custom(_) => {
                let list = library();
                let path = self.path();
                BUILT_IN.len()
                    + list
                        .iter()
                        .position(|m| Some(&m.path) == path.as_ref())
                        .unwrap_or(list.len())
            }
        }
    }
    /// The GPU array layer drawn for this choice: a built-in's own, a
    /// readable custom image's slot, or Clay Studio's for a missing one
    /// (MC-08).
    pub fn layer(self) -> u32 {
        match self {
            Choice::BuiltIn(i) => u32::from(i).min(BUILT_IN.len() as u32 - 1),
            Choice::Custom(id) if image(self).is_some() => (BUILT_IN.len() + slot(id)) as u32,
            Choice::Custom(_) => missing_index() as u32,
        }
    }
    /// Whether this is a custom image that can't be read (MC-08).
    pub fn missing(self) -> bool {
        matches!(self, Choice::Custom(_)) && image(self).is_none()
    }
    /// The name people see: a built-in's label, or the file's name
    /// without its extension.
    pub fn label(self) -> String {
        match self {
            Choice::BuiltIn(_) => BUILT_IN[self.menu_index()].1.to_owned(),
            Choice::Custom(_) => self
                .path()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_default(),
        }
    }
    /// How it is saved: a built-in's key, or `file:` and the path.
    fn key(self) -> String {
        match self {
            Choice::BuiltIn(i) => BUILT_IN[self.menu_index().min(usize::from(i))].0.into(),
            Choice::Custom(_) => format!("file:{}", self.path().unwrap_or_default().display()),
        }
    }
    fn from_key(key: &str) -> Self {
        if let Some(path) = key.strip_prefix("file:") {
            return Choice::custom(Path::new(path));
        }
        BUILT_IN
            .iter()
            .position(|(k, ..)| *k == key)
            .map_or_else(Choice::default, |i| Choice::BuiltIn(i as u8))
    }
}
impl Serialize for Choice {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.key())
    }
}
impl<'de> Deserialize<'de> for Choice {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let key = String::deserialize(d)?;
        Ok(Choice::from_key(&key))
    }
}

/// The matcaps found under the preset browser's roots (MC-05), for the
/// view's Matcap menu. The browser fills it in the background.
pub type Library = Vec<operandi_core::preset::library::FoundMatcap>;

fn library_cell() -> &'static Mutex<Arc<Library>> {
    static LIBRARY: OnceLock<Mutex<Arc<Library>>> = OnceLock::new();
    LIBRARY.get_or_init(|| Mutex::new(Arc::new(Vec::new())))
}

/// The library's matcaps as last found.
pub fn library() -> Arc<Library> {
    library_cell()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Replace the library's matcaps (the browser's scan).
pub fn set_library(list: Library) {
    *library_cell().lock().unwrap_or_else(|e| e.into_inner()) = Arc::new(list);
}

/// The index in `BUILT_IN` of what a missing image draws with.
fn missing_index() -> usize {
    BUILT_IN
        .iter()
        .position(|(k, ..)| *k == MISSING)
        .unwrap_or(0)
}

/// The GPU slots of the custom images: which id each holds and when it
/// was last asked for.
struct Slots {
    owner: [Option<u32>; MAX_CUSTOM],
    used: [u64; MAX_CUSTOM],
    tick: u64,
}

/// The GPU slot (0..MAX_CUSTOM) holding custom image `id`, taking the
/// free slot or the one asked for longest ago when it has none. Every
/// frame asks again for the images it draws, so ones in use stay.
pub fn slot(id: u32) -> usize {
    static SLOTS: OnceLock<Mutex<Slots>> = OnceLock::new();
    let mut slots = SLOTS
        .get_or_init(|| {
            Mutex::new(Slots {
                owner: [None; MAX_CUSTOM],
                used: [0; MAX_CUSTOM],
                tick: 0,
            })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    slots.tick += 1;
    let tick = slots.tick;
    let s = match slots.owner.iter().position(|o| *o == Some(id)) {
        Some(s) => s,
        None => {
            let s = (0..MAX_CUSTOM)
                .min_by_key(|&s| (slots.owner[s].is_some(), slots.used[s]))
                .unwrap_or(0);
            slots.owner[s] = Some(id);
            s
        }
    };
    slots.used[s] = tick;
    s
}

/// One matcap's pixels: `SIZE` × `SIZE` gamma-encoded RGBA, rows from the
/// top (normals pointing up).
pub struct Image {
    pub rgba: Vec<u8>,
}

fn encode(v: f32) -> u8 {
    let v = v.clamp(0., 1.);
    let s = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    };
    (s * 255. + 0.5) as u8
}

/// Decode a matcap image at `SIZE` × `SIZE`. PNG, JPEG and the like are
/// taken as gamma-encoded; EXR and HDR as linear light (encoded here).
/// A picture that isn't square uses its centred square (MC-07); any size
/// is brought down to `SIZE` (MC-07's 1024 cap is met by that), and the
/// file is never touched.
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
    let mut img = image::load_from_memory_with_format(bytes, format).map_err(|e| e.to_string())?;
    let (w, h) = (img.width(), img.height());
    if w != h {
        let side = w.min(h);
        img = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
    }
    let linear = matches!(
        format,
        image::ImageFormat::OpenExr | image::ImageFormat::Hdr
    );
    let img = img.resize_exact(SIZE, SIZE, image::imageops::FilterType::Triangle);
    let rgba = if linear {
        img.to_rgba32f()
            .pixels()
            .flat_map(|p| [encode(p.0[0]), encode(p.0[1]), encode(p.0[2]), 255])
            .collect()
    } else {
        img.to_rgba8().into_raw()
    };
    Ok(Image { rgba })
}

/// The pixels of a choice: a built-in, or the custom image read from disk
/// (`None` when unreadable). Cached per choice.
pub fn image(choice: Choice) -> Option<Arc<Image>> {
    type Cache = Mutex<Vec<(Choice, Option<Arc<Image>>)>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    if let Some((_, img)) = cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|(c, _)| *c == choice)
    {
        return img.clone();
    }
    let img = match choice {
        Choice::BuiltIn(_) => decode(BUILT_IN[choice.menu_index()].2).ok(),
        Choice::Custom(_) => choice
            .path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| decode(&b).ok()),
    }
    .map(Arc::new);
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push((choice, img.clone()));
    img
}

/// The pixels a choice draws with: its own, else Clay Studio's (MC-08).
pub fn image_or_default(choice: Choice) -> Arc<Image> {
    image(choice)
        .or_else(|| image(Choice::BuiltIn(missing_index() as u8)))
        .unwrap_or_else(|| {
            Arc::new(Image {
                rgba: vec![153; (SIZE * SIZE * 4) as usize],
            })
        })
}

/// The matcap's colour (gamma-encoded 0–1) for a view-space normal's X and
/// Y, bilinear between pixels; the shader's `matcap` does the same.
pub fn sample(choice: Choice, x: f32, y: f32) -> [f32; 3] {
    let img = image_or_default(choice);
    let mut p = glam::Vec2::new(x, y);
    let r = p.length();
    if r > REACH {
        p *= REACH / r;
    }
    let n = SIZE as f32;
    let fx = (p.x * 0.5 + 0.5) * n - 0.5;
    let fy = (0.5 - p.y * 0.5) * n - 0.5;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let texel = |x: f32, y: f32| {
        let x = (x as i32).clamp(0, SIZE as i32 - 1) as usize;
        let y = (y as i32).clamp(0, SIZE as i32 - 1) as usize;
        let o = (y * SIZE as usize + x) * 4;
        glam::Vec3::new(
            f32::from(img.rgba[o]),
            f32::from(img.rgba[o + 1]),
            f32::from(img.rgba[o + 2]),
        ) / 255.
    };
    let top = texel(x0, y0).lerp(texel(x0 + 1., y0), tx);
    let bottom = texel(x0, y0 + 1.).lerp(texel(x0 + 1., y0 + 1.), tx);
    top.lerp(bottom, ty).to_array()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// New views (and views saved before matcaps existed) start with Full
    /// Metal.
    #[test]
    fn the_default_is_full_metal() {
        let d = Choice::default();
        assert_eq!(BUILT_IN[d.menu_index()].1, "Full Metal");
        assert_eq!(serde_json::to_string(&d).unwrap(), "\"fullmetal\"");
        assert_eq!(crate::view_display::Drawing::default().matcap, d);
    }

    #[test]
    fn every_built_in_decodes_at_full_size() {
        for (i, (key, ..)) in BUILT_IN.iter().enumerate() {
            let img = image(Choice::BuiltIn(i as u8)).unwrap_or_else(|| panic!("{key}"));
            assert_eq!(img.rgba.len(), (SIZE * SIZE * 4) as usize, "{key}");
        }
    }

    /// Saved by key or path; unknown keys fall back to the default; a
    /// custom image keeps its id and layer.
    #[test]
    fn choices_save_by_key_and_path() {
        let pearl = Choice::BuiltIn(2);
        assert_eq!(serde_json::to_string(&pearl).unwrap(), "\"pearl\"");
        assert_eq!(serde_json::from_str::<Choice>("\"pearl\"").unwrap(), pearl);
        assert_eq!(
            serde_json::from_str::<Choice>("\"no-such\"").unwrap(),
            Choice::default()
        );
        let path = Path::new("/tmp/operandi-matcap-test.png");
        let custom = Choice::custom(path);
        assert_eq!(Choice::custom(path), custom);
        assert_eq!(custom.path().as_deref(), Some(path));
        let saved = serde_json::to_string(&custom).unwrap();
        assert_eq!(saved, "\"file:/tmp/operandi-matcap-test.png\"");
        assert_eq!(serde_json::from_str::<Choice>(&saved).unwrap(), custom);
        assert_eq!(custom.menu_index(), BUILT_IN.len() + library().len());
        // Unreadable: drawn with Clay Studio (MC-08).
        assert!(image(custom).is_none());
        assert!(custom.missing());
        let clay = Choice::BuiltIn(missing_index() as u8);
        assert_eq!(BUILT_IN[clay.menu_index()].1, "Clay Studio");
        assert_eq!(image_or_default(custom).rgba, image(clay).unwrap().rgba);
        assert_eq!(custom.layer(), clay.layer());
        assert_eq!(custom.label(), "operandi-matcap-test");
    }

    /// The shiny matcaps are more contrasty than the clay one, and a loaded
    /// PNG reads back.
    #[test]
    fn samples_and_custom_png() {
        let spread = |c: Choice| {
            let v: Vec<f32> = (0..20)
                .flat_map(|j| (0..20).map(move |i| (i as f32 / 10. - 0.95, j as f32 / 10. - 0.95)))
                .filter(|(x, y)| x * x + y * y < 0.8)
                .map(|(x, y)| sample(c, x, y)[1])
                .collect();
            v.iter().cloned().fold(0., f32::max) - v.iter().cloned().fold(1., f32::min)
        };
        assert!(spread(Choice::BuiltIn(6)) > spread(Choice::BuiltIn(0)));
        let dir = std::env::temp_dir().join(format!("operandi-matcap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("red.png");
        image::RgbImage::from_pixel(16, 16, image::Rgb([200, 10, 10]))
            .save(&file)
            .unwrap();
        let c = Choice::custom(&file);
        let s = sample(c, 0., 0.);
        assert!((s[0] - 200. / 255.).abs() < 0.01 && s[1] < 0.1, "{s:?}");
        std::fs::remove_dir_all(dir).ok();
    }

    /// MC-07: a picture that isn't square uses its centred square.
    #[test]
    fn a_wide_picture_uses_its_centred_square() {
        // 30×10: red, green middle third, blue.
        let img = image::RgbImage::from_fn(30, 10, |x, _| match x {
            0..10 => image::Rgb([255, 0, 0]),
            10..20 => image::Rgb([0, 255, 0]),
            _ => image::Rgb([0, 0, 255]),
        });
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        let m = decode(&bytes).unwrap();
        assert_eq!(m.rgba.len(), (SIZE * SIZE * 4) as usize);
        // Every corner is green: the red and blue sides are cut away.
        for o in [0, (SIZE as usize - 1) * 4, m.rgba.len() - 4] {
            assert_eq!(&m.rgba[o..o + 3], &[0, 255, 0], "at {o}");
        }
    }

    /// More custom images than slots: each one asked for gets a slot,
    /// the ones in use keep theirs, and the oldest is reused.
    #[test]
    fn custom_slots_are_reused_oldest_first() {
        let base = 1_000_000;
        let first: Vec<usize> = (0..MAX_CUSTOM as u32).map(|i| slot(base + i)).collect();
        let mut sorted = first.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), MAX_CUSTOM);
        // Keep the last one in use, then ask for a new one: it takes the
        // slot asked for longest ago, never the one in use.
        let kept = slot(base + MAX_CUSTOM as u32 - 1);
        let new = slot(base + 999);
        assert_ne!(new, kept);
        assert_eq!(slot(base + MAX_CUSTOM as u32 - 1), kept);
        assert!(new < MAX_CUSTOM);
    }
}
